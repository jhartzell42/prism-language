//! Blocks (Semantics ch. 4): widget blocks, expression blocks, `given:`.
//!
//! **A block of equations is one binding group; a block of steps is a sequence**
//! (Semantics ch. 4, "Binding groups and cycles"). A widget block and an expression
//! block's `let`s are groups: any line may use any name the block binds. `given:` is a
//! sequence: a line sees only the lines above.
//!
//! A group's lines are unordered: what orders them is prompt dependence (Semantics ch. 7,
//! "There are no cycles"), with a sample as a delayed edge. **The order of a group's lines
//! decides positions and nothing else**, and `prism_rt` hands out positions in the order
//! statements run, so the group runs in three passes:
//!
//! - **Its lines, in page order.** Every statement that can take a position (an element,
//!   a blueprint, `rebuild`, `for`, `@started`, a host document) runs at its line, so
//!   positions follow the page (Semantics ch. 3, "What takes a position"). A `let`, a
//!   hold's initial value and a local function take no position, so each is evaluated
//!   when a line first needs it, wherever it stands: a name is a [`Def`] until then.
//! - **The deferred event `let`s.** A `let` of events needed before a line it depends on
//!   has run gets one forward event per binder (`Build::forward_event`), and is evaluated
//!   and resolved once every line has run. This is how `@Window`'s supplier lambda, at
//!   line 0, reads `!route`, which depends on lines below it and on line 0's own output.
//! - **The holds' `then`s.** A hold exists from its first use (`Build::hold`), and its
//!   `then` is wired after every line, so it may read any name in the block: `@labels`
//!   reads `$tags` above the hold that defines it (Reflex's `rec`, with no annotation).
//!
//! A line that reads an event bound by a later bind gets a forward event, resolved when
//! that line runs. A line that reads a dynamic or plain value bound by a later bind runs
//! that bind first, out of page order, when the bind takes no position (a `@Switch`, a
//! `@Flatten`, a widget application that emits nothing), and the position counter is
//! checked after it. A later bind that takes a position cannot run early, and `prism_rt`
//! has no forward dynamic to stand in for its output, so reading one of those above its
//! line is refused with a diagnostic (README.md, "What fails"). A definition that needs
//! itself while it is being evaluated is a cycle: a hold's initial value naming itself, a
//! cycle among plain values, or a bind whose right side reads its own binder (a prompt
//! cycle), each refused naming the name (Semantics ch. 4; ch. 7). The prompt-cycle check
//! is only this run-time one: cycles that close through forward events are not detected
//! (README.md, PC).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use prism_rt::{Build, OutputKind, ResultKind, Value};
use prism_syntax::ast::*;

use crate::interp::{BlockKind, Bx, Code, Frame, Interp, Loc, Scopes};
use crate::load::pattern_binders;
use crate::ops::R;

/// A name a group defines, not yet evaluated.
#[derive(Clone, Copy, Debug)]
pub enum Def {
    /// `let p = e`, statement index.
    Let(usize),
    /// `$h <- init then e`.
    Hold(usize),
    /// A bind that takes a position: `x <- e` with no `then`.
    Positional(usize),
    /// `fn Name ...` clauses in the block.
    LocalFn,
    /// `@name:` and a block, inside the block.
    InnerWidget(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DefState {
    InProgress,
    Done,
}

/// A binding group: one block's definitions and their state.
pub struct Group {
    pub block: &'static Block,
    defs: HashMap<Rc<str>, Def>,
    state: RefCell<HashMap<Rc<str>, DefState>>,
    forwards: RefCell<HashMap<Rc<str>, Value>>,
    /// The event `let`s needed before a line they depend on had run: statement indices,
    /// evaluated after the lines, and the forward events standing in for their binders.
    deferred: RefCell<Vec<usize>>,
    let_forwards: RefCell<HashMap<Rc<str>, Value>>,
    /// The binds already run out of page order, which the page-order pass skips.
    ran: RefCell<HashSet<usize>>,
    lines_done: Cell<bool>,
    /// The address of the group's own `Build` while its lines run, so that a bind is run
    /// early only into the widget it belongs to.
    builder: Cell<usize>,
    /// The holds made so far, with their statement, to wire their `then`s.
    holds: RefCell<Vec<(usize, Value)>>,
    local_fns: HashMap<Rc<str>, (Option<&'static Signature>, Vec<&'static FnClause>)>,
}

impl Group {
    fn new(block: &'static Block) -> Group {
        let mut defs = HashMap::new();
        let mut sigs: HashMap<Rc<str>, &'static Signature> = HashMap::new();
        let mut local_fns: HashMap<Rc<str>, (Option<&'static Signature>, Vec<&'static FnClause>)> = HashMap::new();
        for (i, s) in block.stmts.iter().enumerate() {
            match &s.kind {
                StmtKind::Let { pattern, .. } => {
                    for n in pattern_binders(pattern) {
                        defs.insert(Rc::from(n), Def::Let(i));
                    }
                }
                StmtKind::Bind { pattern, then: Some(_), .. } => {
                    for n in pattern_binders(pattern) {
                        defs.insert(Rc::from(n), Def::Hold(i));
                    }
                }
                StmtKind::Bind { pattern, then: None, .. } => {
                    for n in pattern_binders(pattern) {
                        defs.insert(Rc::from(n), Def::Positional(i));
                    }
                }
                StmtKind::Fn(c) => {
                    let n: Rc<str> = Rc::from(match &c.name {
                        DefName::Name(n) => n.text(),
                        DefName::Op(o, _) => o.clone(),
                    });
                    defs.insert(n.clone(), Def::LocalFn);
                    local_fns.entry(n).or_insert((None, vec![])).1.push(c);
                }
                StmtKind::Signature(sig) => {
                    if let DefName::Name(n) = &sig.name {
                        sigs.insert(Rc::from(n.text()), sig);
                    }
                }
                StmtKind::Widget(w) => {
                    defs.insert(Rc::from(w.name.text()), Def::InnerWidget(i));
                }
                _ => {}
            }
        }
        for (n, s) in sigs {
            if let Some(f) = local_fns.get_mut(&n) {
                f.0 = Some(s);
            }
        }
        Group {
            block,
            defs,
            state: RefCell::new(HashMap::new()),
            forwards: RefCell::new(HashMap::new()),
            deferred: RefCell::new(Vec::new()),
            let_forwards: RefCell::new(HashMap::new()),
            ran: RefCell::new(HashSet::new()),
            lines_done: Cell::new(false),
            builder: Cell::new(0),
            holds: RefCell::new(Vec::new()),
            local_fns,
        }
    }

    /// Whether the group defines `name` and has not evaluated it yet.
    pub fn defines(&self, name: &str) -> bool {
        self.defs.contains_key(name) && self.state.borrow().get(name) != Some(&DefState::Done)
    }

    fn mark(&self, name: &str, s: DefState) {
        self.state.borrow_mut().insert(Rc::from(name), s);
    }

    fn done(&self, name: &str) -> bool {
        self.state.borrow().get(name) == Some(&DefState::Done)
    }
}

/// The address of a `Build`, to tell a group's own builder from an inner widget's.
fn addr(b: &Build) -> usize {
    b as *const Build as *const () as usize
}

impl Interp {
    /// Evaluate a group's definition of `name`, in the scopes up to frame `k`.
    pub fn force(&self, sc: &mut Scopes, k: usize, g: Rc<Group>, name: &str, b: Bx, loc: Loc) -> R<Value> {
        let inner = sc.frames.split_off(k + 1);
        // A definition evaluated in the middle of a `rebuild` or `for` line must not take
        // the result kind that line's binder set.
        let kind = sc.bind_kind.take();
        let r = self.force_in(sc, &g, name, b, loc);
        sc.bind_kind = kind;
        sc.frames.extend(inner);
        r
    }

    /// The first name bound by a bind of `g` that has not finished running, and is not an
    /// event (an event has a forward node), which the definition of `name` reads, directly
    /// or through the group's other definitions. A hold's `then` is not followed: it is
    /// wired after every line.
    fn blocker(&self, g: &Group, name: &str, seen: &mut HashSet<Rc<str>>) -> Option<Rc<str>> {
        let def = *g.defs.get(name)?;
        if g.done(name) || !seen.insert(Rc::from(name)) {
            return None;
        }
        let stmt = |i: usize| &g.block.stmts[i];
        let deps: Vec<Rc<str>> = match def {
            Def::Positional(_) if name.starts_with('!') => return None,
            Def::Positional(_) => return Some(Rc::from(name)),
            Def::Let(i) => match &stmt(i).kind {
                StmtKind::Let { value, .. } => (*self.free_names(value, crate::eval::walk_expr)).clone(),
                _ => vec![],
            },
            Def::Hold(i) => match &stmt(i).kind {
                StmtKind::Bind { value, .. } => (*self.free_names(value, crate::eval::walk_expr)).clone(),
                _ => vec![],
            },
            Def::LocalFn => {
                let mut out = Vec::new();
                for c in &g.local_fns[name].1 {
                    out.extend(self.free_names(*c, |c, o| match &c.body {
                        FnBody::Expr(e) => crate::eval::walk_expr(e, o),
                        FnBody::Block(bl) => crate::eval::walk_block(bl, o),
                    }).iter().cloned());
                }
                out
            }
            Def::InnerWidget(i) => match &stmt(i).kind {
                StmtKind::Widget(w) => (*self.free_names(&w.body, crate::eval::walk_block)).clone(),
                _ => vec![],
            },
        };
        deps.iter().find_map(|n| self.blocker(g, n, seen))
    }

    fn force_in(&self, sc: &mut Scopes, g: &Rc<Group>, name: &str, mut b: Bx, loc: Loc) -> R<Value> {
        let def = g.defs[name];
        if g.state.borrow().get(name) == Some(&DefState::InProgress) {
            return Err(self.fault(
                loc,
                match def {
                    Def::Hold(_) => format!("{name}: a hold whose initial value names itself (Semantics ch. 4)"),
                    Def::Positional(_) => format!("{name} is read by the line that binds it: a cycle with no `then` in it is a prompt cycle (Semantics ch. 7)"),
                    _ => format!("{name} is defined in terms of itself: a cycle among plain values (Semantics ch. 4)"),
                },
            ));
        }
        let stmt = |i: usize| &g.block.stmts[i];
        match def {
            // An event `let` that reads a bind not run yet: forward events now, the `let`
            // after the lines.
            Def::Let(i)
                if !g.lines_done.get()
                    && b.is_some()
                    && matches!(&stmt(i).kind, StmtKind::Let { pattern, .. }
                        if pattern_binders(pattern).iter().all(|n| n.starts_with('!')))
                    && self.blocker(g, name, &mut HashSet::new()).is_some() =>
            {
                let StmtKind::Let { pattern, .. } = &stmt(i).kind else { unreachable!() };
                let b = b.expect("checked");
                for n in pattern_binders(pattern) {
                    let fwd = b.forward_event();
                    g.let_forwards.borrow_mut().insert(Rc::from(n.as_str()), fwd.clone());
                    g.mark(&n, DefState::Done);
                    sc.bind(Rc::from(n.as_str()), fwd);
                }
                g.deferred.borrow_mut().push(i);
            }
            Def::Let(i) => {
                let StmtKind::Let { pattern, value, .. } = &stmt(i).kind else { unreachable!() };
                let binders = pattern_binders(pattern);
                for n in &binders {
                    g.mark(n, DefState::InProgress);
                }
                let v = self.eval(value, sc, b);
                for n in &binders {
                    g.state.borrow_mut().remove(n.as_str());
                }
                let v = v?;
                let mut binds = Vec::new();
                if !crate::pattern::matches(self, sc.module, pattern, &v, &mut binds)? {
                    return Err(self.fault(Loc { module: sc.module, span: pattern.span }, "the `let` pattern does not match its value"));
                }
                for (n, bv) in binds {
                    g.mark(&n, DefState::Done);
                    sc.bind(n, bv);
                }
            }
            Def::Hold(i) => {
                let StmtKind::Bind { value, .. } = &stmt(i).kind else { unreachable!() };
                g.mark(name, DefState::InProgress);
                let init = self.eval(value, sc, b.as_deref_mut());
                g.state.borrow_mut().remove(name);
                let init = init?;
                let b = b.ok_or_else(|| self.fault(loc, "a hold stands in a widget block"))?;
                // A dynamic initial value is read as the construction reads it (R7).
                let init = b.read(&init)?;
                let h = b.hold(init);
                g.holds.borrow_mut().push((i, h.clone()));
                g.mark(name, DefState::Done);
                sc.bind(Rc::from(name), h);
            }
            Def::Positional(i) => {
                if name.starts_with('!') {
                    let b = b.ok_or_else(|| self.fault(loc, "an event stands in a widget block"))?;
                    let fwd = b.forward_event();
                    g.forwards.borrow_mut().insert(Rc::from(name), fwd.clone());
                    // Bound now for the lines above; the line rebinds it to the event itself.
                    sc.bind(Rc::from(name), fwd);
                } else {
                    // A dynamic or a plain value: run its bind now, if it takes no position.
                    let line = self.l.modules[sc.module].at(stmt(i).span);
                    let refuse = |why: &str| {
                        self.fault(
                            loc,
                            format!("{name} is bound at {line}, below this line, by a bind that {why}; reading it here needs a forward dynamic, which prism_rt does not have (README.md, \"What fails\")"),
                        )
                    };
                    let StmtKind::Bind { value, .. } = &stmt(i).kind else { unreachable!() };
                    if takes_a_position(self, value) {
                        return Err(refuse("takes a position, so it runs at its line"));
                    }
                    let Some(b) = b.filter(|b| addr(b) == g.builder.get()) else {
                        return Err(refuse("belongs to the enclosing widget, not the one reading it"));
                    };
                    let before = b.next_path();
                    let span = stmt(i).span;
                    self.run_bind(i, g, sc, b, Loc { module: sc.module, span })?;
                    g.ran.borrow_mut().insert(i);
                    if b.next_path() != before {
                        return Err(refuse("took a position when it ran"));
                    }
                }
            }
            Def::LocalFn => {
                let (sig, clauses) = g.local_fns[name].clone();
                let module = sc.module;
                let key = format!("localfn:{}", clauses[0] as *const FnClause as usize);
                let id = self.code_keyed(key, || {
                    let id = self.fn_code(module, name, sig, clauses.clone(), true);
                    match Rc::try_unwrap(self.code(id)) {
                        Ok(c) => c,
                        Err(rc) => clone_fn_code(&rc),
                    }
                });
                g.mark(name, DefState::InProgress);
                let mut free = Vec::new();
                for c in &clauses {
                    match &c.body {
                        FnBody::Expr(e) => crate::eval::walk_expr(e, &mut free),
                        FnBody::Block(bl) => crate::eval::walk_block(bl, &mut free),
                    }
                }
                free.retain(|n| &**n != name);
                let cap = self.capture(sc, &free, vec![], b, loc);
                g.state.borrow_mut().remove(name);
                let v = self.closure(id, name, cap?);
                g.mark(name, DefState::Done);
                sc.bind(Rc::from(name), v);
            }
            Def::InnerWidget(i) => {
                let StmtKind::Widget(w) = &stmt(i).kind else { unreachable!() };
                let module = sc.module;
                let id = self.code_at(&w.body, 40, || Code::Block { block: &w.body, kind: BlockKind::Widget, module, set_row: false });
                g.mark(name, DefState::InProgress);
                let free = self.free_names(&w.body, crate::eval::walk_block);
                let cap = self.capture(sc, &free, vec![], b, loc);
                g.state.borrow_mut().remove(name);
                let v = self.blueprint(id, name, cap?);
                g.mark(name, DefState::Done);
                sc.bind(Rc::from(name), v);
            }
        }
        Ok(sc.frames.last().and_then(|f| f.vars.get(name)).cloned().unwrap_or(Value::Unit))
    }

    /// Run a widget block (Semantics ch. 4, "The widget block"): its statements build the
    /// widget, and its result is its last line.
    pub fn run_widget_block(&self, block: &'static Block, sc: &mut Scopes, b: &mut Build) -> R<Value> {
        let g = Rc::new(Group::new(block));
        sc.frames.push(Frame { vars: HashMap::new(), group: Some(g.clone()) });
        let r = self.widget_lines(block, &g, sc, b);
        sc.frames.pop();
        r
    }

    fn widget_lines(&self, block: &'static Block, g: &Rc<Group>, sc: &mut Scopes, b: &mut Build) -> R<Value> {
        let n = block.stmts.len();
        let mut result = Value::Unit;
        g.builder.set(addr(b));
        for (i, stmt) in block.stmts.iter().enumerate() {
            let last = i + 1 == n;
            let loc = Loc { module: sc.module, span: stmt.span };
            if g.ran.borrow().contains(&i) {
                continue;
            }
            match &stmt.kind {
                StmtKind::Emit(row) => {
                    if i != 0 {
                        return Err(self.fault(loc, "`emit` stands first in its block (Semantics ch. 3)"));
                    }
                    b.set_emit(crate::load::row_of(row));
                }
                StmtKind::Let { pattern, .. } => {
                    for name in pattern_binders(pattern) {
                        self.lookup_local(sc, &name, Some(&mut *b), loc)?;
                    }
                }
                StmtKind::Signature(_) | StmtKind::Fn(_) | StmtKind::Widget(_) => {}
                StmtKind::Bind { pattern, then: Some(_), .. } => {
                    for name in pattern_binders(pattern) {
                        self.lookup_local(sc, &name, Some(&mut *b), loc)?;
                    }
                }
                StmtKind::Bind { then: None, .. } => {
                    self.run_bind(i, g, sc, b, loc)?;
                    if last {
                        result = Value::Unit;
                    }
                }
                StmtKind::Ignore(e) => {
                    self.statement_value(Some(ResultKind::Unit), e, sc, b, loc)?;
                }
                StmtKind::Expr(e) => {
                    let kind = if last { None } else { Some(ResultKind::Unit) };
                    let v = self.statement_value(kind, e, sc, b, loc)?;
                    if last {
                        result = v;
                    }
                }
                StmtKind::Require(_) | StmtKind::Forbid(_) => {
                    return Err(self.fault(loc, "`require` and `forbid` stand in a `given:` block (Semantics ch. 4)"));
                }
                StmtKind::While { .. } | StmtKind::Set { .. } | StmtKind::Accum { .. } => {
                    return Err(self.fault(loc, "this statement stands in an `update` block (Semantics ch. 5)"));
                }
                StmtKind::Return(_) => return Err(self.out_of_scope(loc, "`T monad:` blocks and `^Monad`")),
            }
        }
        g.lines_done.set(true);
        // The deferred event `let`s, once every line has run. Each binder's forward event
        // is taken out of scope while its `let` is evaluated, so a `let` that reads itself
        // is a cycle, as in the first pass.
        loop {
            let todo: Vec<usize> = std::mem::take(&mut *g.deferred.borrow_mut());
            if todo.is_empty() {
                break;
            }
            for i in todo {
                let StmtKind::Let { pattern, value, .. } = &block.stmts[i].kind else { unreachable!() };
                let loc = Loc { module: sc.module, span: block.stmts[i].span };
                let binders = pattern_binders(pattern);
                for n in &binders {
                    sc.frames.last_mut().expect("the group's frame").vars.remove(n.as_str());
                    g.mark(n, DefState::InProgress);
                }
                let v = self.eval(value, sc, Some(&mut *b));
                for n in &binders {
                    g.state.borrow_mut().remove(n.as_str());
                }
                let v = v?;
                let mut binds = Vec::new();
                if !crate::pattern::matches(self, sc.module, pattern, &v, &mut binds)? {
                    return Err(self.fault(Loc { module: sc.module, span: pattern.span }, "the `let` pattern does not match its value"));
                }
                for (name, bv) in binds {
                    let fwd = g.let_forwards.borrow_mut().remove(&name);
                    if let Some(fwd) = fwd {
                        if !matches!(bv, Value::Event(_)) {
                            return Err(self.fault(loc, format!("{name} is an event, and its `let` gives {}", crate::ops::show_plain(&bv))));
                        }
                        b.resolve_event(&fwd, &bv).map_err(|f| self.wrap_fault(loc, f))?;
                    }
                    g.mark(&name, DefState::Done);
                    sc.bind(name, bv);
                }
            }
        }
        // The holds' `then`s, once every line has run.
        let holds: Vec<(usize, Value)> = g.holds.borrow().clone();
        for (i, h) in holds {
            let StmtKind::Bind { then: Some(t), .. } = &block.stmts[i].kind else { unreachable!() };
            let loc = Loc { module: sc.module, span: t.span };
            let ev = self.eval(t, sc, Some(&mut *b))?;
            match ev {
                Value::Event(_) => b.hold_on(&h, &ev).map_err(|f| self.wrap_fault(loc, f))?,
                other => {
                    return Err(self.fault(
                        loc,
                        format!("`then` takes an event: the hold changes when it fires; this is {}", crate::ops::show_plain(&other)),
                    ));
                }
            }
        }
        let mut leftover: Vec<Rc<str>> = g.forwards.borrow().keys().cloned().collect();
        leftover.extend(g.let_forwards.borrow().keys().cloned());
        if let Some(n) = leftover.first() {
            return Err(self.fault(Loc { module: sc.module, span: block.span }, format!("{n} was read but never bound")));
        }
        Ok(result)
    }

    /// Run the bind at statement `i` (`x <- e`, no `then`): at its line, or earlier when a
    /// line above needs its dynamic and it takes no position.
    fn run_bind(&self, i: usize, g: &Rc<Group>, sc: &mut Scopes, b: &mut Build, loc: Loc) -> R<()> {
        let StmtKind::Bind { pattern, value, then: None } = &g.block.stmts[i].kind else { unreachable!() };
        let binders = pattern_binders(pattern);
        for n in &binders {
            g.mark(n, DefState::InProgress);
        }
        let kind = self.bind_kind(pattern, value);
        let v = self.statement_value(kind, value, sc, b, loc);
        for n in &binders {
            g.state.borrow_mut().remove(n.as_str());
        }
        let v = v?;
        let mut binds = Vec::new();
        if !crate::pattern::matches(self, sc.module, pattern, &v, &mut binds)? {
            return Err(self.fault(loc, format!("the bind's pattern does not match its output, {}", crate::ops::show_plain(&v))));
        }
        for (name, bv) in binds {
            let fwd = g.forwards.borrow_mut().remove(&name);
            if let Some(fwd) = fwd {
                b.resolve_event(&fwd, &bv).map_err(|f| self.wrap_fault(loc, f))?;
            }
            g.mark(&name, DefState::Done);
            sc.bind(name, bv);
        }
        Ok(())
    }

    /// The value a statement's right side gives: a `match` or `if` over a plain
    /// scrutinee runs its arm as a building block; an element is emitted and a blueprint
    /// built ([`Interp::bind_build`]).
    fn statement_value(&self, kind: Option<ResultKind>, e: &'static Expr, sc: &mut Scopes, b: &mut Build, loc: Loc) -> R<Value> {
        let inner = match &e.kind {
            ExprKind::Group(x) => &**x,
            _ => e,
        };
        if matches!(inner.kind, ExprKind::Match { .. } | ExprKind::If { .. }) {
            let v = self.eval_control(inner, sc, Some(&mut *b), true)?;
            return self.bind_build(v, b, loc);
        }
        if matches!(inner.kind, ExprKind::Rebuild { .. } | ExprKind::For { .. }) {
            // A builder on the block's last line has no binder to say what it hands
            // back: its rows' last line says.
            let guessed = match &inner.kind {
                ExprKind::For { body, .. } => self.row_kind(body),
                _ => ResultKind::Plain,
            };
            sc.bind_kind = Some(kind.unwrap_or(guessed));
        }
        let v = self.eval(e, sc, Some(&mut *b));
        sc.bind_kind = None;
        self.bind_build(v?, b, loc)
    }

    /// An expression block: its `let`s and local functions are a group, and its last
    /// line is its value.
    pub fn run_expr_block(&self, block: &'static Block, sc: &mut Scopes, mut b: Bx) -> R<Value> {
        let g = Rc::new(Group::new(block));
        sc.frames.push(Frame { vars: HashMap::new(), group: Some(g) });
        let n = block.stmts.len();
        let mut result = Ok(Value::Unit);
        for (i, stmt) in block.stmts.iter().enumerate() {
            let loc = Loc { module: sc.module, span: stmt.span };
            let r = match &stmt.kind {
                StmtKind::Let { pattern, .. } => {
                    let mut out = Ok(Value::Unit);
                    for name in pattern_binders(pattern) {
                        if let Err(e) = self.lookup_local(sc, &name, b.as_deref_mut(), loc) {
                            out = Err(e);
                            break;
                        }
                    }
                    out
                }
                StmtKind::Signature(_) | StmtKind::Fn(_) | StmtKind::Widget(_) => Ok(Value::Unit),
                StmtKind::Expr(e) => self.eval(e, sc, b.as_deref_mut()),
                StmtKind::Bind { .. } | StmtKind::Ignore(_) | StmtKind::Emit(_) => {
                    Err(self.fault(loc, "a bind builds, and stands in a widget block or a `given:` block"))
                }
                StmtKind::Require(_) | StmtKind::Forbid(_) => {
                    Err(self.fault(loc, "`require` and `forbid` stand in a `given:` block (Semantics ch. 4)"))
                }
                StmtKind::While { .. } | StmtKind::Set { .. } | StmtKind::Accum { .. } => {
                    Err(self.fault(loc, "this statement stands in an `update` block (Semantics ch. 5)"))
                }
                StmtKind::Return(_) => Err(self.out_of_scope(loc, "`T monad:` blocks and `^Monad`")),
            };
            match r {
                Err(e) => {
                    result = Err(e);
                    break;
                }
                Ok(v) if i + 1 == n => result = Ok(v),
                Ok(_) => {}
            }
        }
        sc.frames.pop();
        result
    }

    /// A `given:` block (Semantics ch. 4, `given:`; ch. 10): binds through `?` or `~`
    /// with `<-`, guards with `require` and `forbid`, and ends on its answer. A failed
    /// bind gives up into what it bound (`nil` or the `Error`); a failed guard gives up
    /// into `nil` under a `?` name and, under a `~` name, into `Prelude.Unmet` with the
    /// guard's source text as its message (decisions.md D11).
    pub fn run_given(&self, block: &'static Block, sc: &mut Scopes, mut b: Bx) -> R<Value> {
        sc.push();
        let r = self.given_steps(block, sc, b.as_deref_mut());
        sc.pop();
        r
    }

    fn given_steps(&self, block: &'static Block, sc: &mut Scopes, mut b: Bx) -> R<Value> {
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            let loc = Loc { module: sc.module, span: stmt.span };
            match &stmt.kind {
                StmtKind::Bind { pattern, value, then: None } => {
                    let v = self.eval(value, sc, b.as_deref_mut())?;
                    let target = match &v {
                        Value::Opt(None) | Value::Fallible(Err(_)) => return Ok(v),
                        Value::Opt(Some(x)) | Value::Fallible(Ok(x)) => (**x).clone(),
                        other => other.clone(),
                    };
                    let mut binds = Vec::new();
                    if !crate::pattern::matches(self, sc.module, pattern, &target, &mut binds)? {
                        return Ok(self.give_up(sc, stmt));
                    }
                    for (n, bv) in binds {
                        sc.bind(n, bv);
                    }
                }
                StmtKind::Let { pattern, value, .. } => {
                    let v = self.eval(value, sc, b.as_deref_mut())?;
                    let mut binds = Vec::new();
                    if !crate::pattern::matches(self, sc.module, pattern, &v, &mut binds)? {
                        return Err(self.fault(loc, "the `let` pattern does not match its value"));
                    }
                    for (n, bv) in binds {
                        sc.bind(n, bv);
                    }
                }
                StmtKind::Require(c) | StmtKind::Forbid(c) => {
                    let want = matches!(stmt.kind, StmtKind::Require(_));
                    match self.eval(c, sc, b.as_deref_mut())? {
                        Value::Bool(x) if x == want => {}
                        Value::Bool(_) => return Ok(self.give_up(sc, stmt)),
                        other => {
                            return Err(self.fault(loc, format!("a guard is a Bool, and this one is {}", crate::ops::show_plain(&other))));
                        }
                    }
                }
                StmtKind::Fn(c) => {
                    let name = match &c.name {
                        DefName::Name(n) => n.text(),
                        DefName::Op(o, _) => o.clone(),
                    };
                    let id = self.fn_code(sc.module, &name, None, vec![c], true);
                    let mut free = Vec::new();
                    match &c.body {
                        FnBody::Expr(e) => crate::eval::walk_expr(e, &mut free),
                        FnBody::Block(bl) => crate::eval::walk_block(bl, &mut free),
                    }
                    let cap = self.capture(sc, &free, vec![], b.as_deref_mut(), loc)?;
                    let v = self.closure(id, &name, cap);
                    sc.bind(Rc::from(name.as_str()), v);
                }
                StmtKind::Signature(_) => {}
                StmtKind::Expr(e) => {
                    let v = self.eval(e, sc, b.as_deref_mut())?;
                    if i + 1 == n {
                        // The last line is the answer: `? v`, `~ v`, or a `?T` passing through.
                        return Ok(match (&v, sc.mark) {
                            (Value::Opt(_) | Value::Fallible(_), _) => v,
                            (_, Some(Sigil::Tilde)) => Value::ok(v),
                            _ => Value::some(v),
                        });
                    }
                }
                _ => return Err(self.fault(loc, "this statement does not stand in a `given:` block")),
            }
        }
        Ok(Value::Unit)
    }

    fn give_up(&self, sc: &Scopes, stmt: &Stmt) -> Value {
        match sc.mark {
            Some(Sigil::Tilde) => Value::Fallible(Err(Rc::new(Value::record(
                "Error",
                vec![
                    ("kind", Value::ctor("Prelude.Unmet", vec![])),
                    ("message", Value::text(self.l.modules[sc.module].text(stmt.span).trim())),
                ],
            )))),
            _ => Value::nil(),
        }
    }
}

fn clone_fn_code(c: &Code) -> Code {
    match c {
        Code::Fn(f) => Code::Fn(crate::interp::FnCode {
            name: f.name.clone(),
            sig: f.sig,
            clauses: f.clauses.clone(),
            module: f.module,
            mark: f.mark,
            unbounded: f.unbounded,
            layers: f.layers.clone(),
            nat: f.nat.clone(),
            local: f.local,
        }),
        _ => unreachable!(),
    }
}

/// Whether a bind's right side certainly takes a position in its block (Semantics ch. 3,
/// "What takes a position"): an element, `rebuild`, `for`, `@started`. A widget
/// application may or may not; it is run, and the position counter checked.
fn takes_a_position(interp: &Interp, e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Group(x) => takes_a_position(interp, x),
        ExprKind::Rebuild { .. } | ExprKind::For { .. } => true,
        ExprKind::Name(n) => n.path.len() == 1 && n.text() == "@started",
        _ => interp.tag_called(e).is_some(),
    }
}

impl Interp {
    /// The tag an element expression names, `Gui.Metrics(text=t)` or a bare
    /// `Ui.Label`, if it names one (not a fault kind).
    fn tag_called(&self, e: &Expr) -> Option<&crate::load::TagInfo> {
        let n = match &e.kind {
            ExprKind::Call { callee, .. } => match &callee.kind {
                ExprKind::Name(n) => n,
                _ => return None,
            },
            ExprKind::Name(n) => n,
            _ => return None,
        };
        if n.path.len() < 2 {
            return None;
        }
        self.tag(&n.path.join(".")).filter(|t| !t.fault)
    }

    /// The result kind of a `rebuild` or `for` bound by `pattern`, read off the binder's
    /// sigils (prism_rt API.md, `ResultKind`; Semantics ch. 4's two tables). `None` when
    /// the right side is not a builder.
    fn bind_kind(&self, pattern: &Pattern, value: &Expr) -> Option<ResultKind> {
        let inner = match &value.kind {
            ExprKind::Group(e) => &**e,
            _ => value,
        };
        let sigils: Vec<Sigil> = match &pattern.kind {
            PatternKind::Name(n) => n.sigils.clone(),
            PatternKind::Ignore => return Some(ResultKind::Unit),
            _ => vec![],
        };
        match &inner.kind {
            // A `rebuild` hands back `$!a`, `$$a` or `$a`; prism_rt gives each the same node.
            ExprKind::Rebuild { .. } => Some(match sigils.as_slice() {
                [Sigil::Dollar, Sigil::Bang, ..] => ResultKind::Event,
                [Sigil::Dollar, Sigil::Dollar, ..] => ResultKind::Dynamic,
                [] => ResultKind::Unit,
                _ => ResultKind::Plain,
            }),
            // A `for` hands back `!(K ==> T)` or `$(K ==> T)`; for `$`, what the rows end
            // on says whether they are dynamics to gather.
            ExprKind::For { body, .. } => Some(match sigils.first() {
                Some(Sigil::Bang) => ResultKind::Event,
                Some(Sigil::Dollar) if self.row_kind(body) == ResultKind::Dynamic => ResultKind::Dynamic,
                _ => ResultKind::Plain,
            }),
            _ => None,
        }
    }

    /// What a `for`'s rows end on: an event, a dynamic, `unit` or plain. An element on the
    /// last line answers what its tag declares (the families' `tag .. of Element T`
    /// lines); otherwise the last line is read off the page, where a checker reads its
    /// type (D3): `@ $x`, `$ v`, a `$"..."` literal, a call over a dynamic are dynamics,
    /// `!e`, `x on !e` and a merge are events.
    fn row_kind(&self, body: &Block) -> ResultKind {
        let Some(StmtKind::Expr(e)) = body.stmts.last().map(|s| &s.kind) else {
            return ResultKind::Plain;
        };
        self.expr_kind(e)
    }

    fn expr_kind(&self, e: &Expr) -> ResultKind {
        fn event(e: &Expr) -> bool {
            match &e.kind {
                ExprKind::Name(n) => n.sigils.first() == Some(&Sigil::Bang),
                ExprKind::Binary { op: BinOp::On | BinOp::Merge, .. } => true,
                ExprKind::FreeSigil { sigil: Sigil::At, operand } => event(operand),
                ExprKind::Group(e) => event(e),
                _ => false,
            }
        }
        fn dynamic(e: &Expr) -> bool {
            match &e.kind {
                ExprKind::Name(n) => n.sigils.first() == Some(&Sigil::Dollar),
                ExprKind::Lit(l) => l.marks.first() == Some(&Sigil::Dollar),
                ExprKind::FreeSigil { sigil: Sigil::Dollar, .. } => true,
                ExprKind::FreeSigil { sigil: Sigil::At, operand } => dynamic(operand),
                ExprKind::Group(e) => dynamic(e),
                ExprKind::Binary { lhs, rhs, op } if *op != BinOp::On => dynamic(lhs) || dynamic(rhs),
                ExprKind::Call { args, .. } => args.iter().any(|a| dynamic(&a.value)),
                ExprKind::Field { recv, .. } => dynamic(recv),
                _ => false,
            }
        }
        match &e.kind {
            ExprKind::Group(x) | ExprKind::FreeSigil { sigil: Sigil::At, operand: x } if self.tag_called(x).is_some() => {
                return self.expr_kind(x);
            }
            _ => {}
        }
        if let Some(t) = self.tag_called(e) {
            return match &t.decl.output {
                OutputKind::Event => ResultKind::Event,
                OutputKind::Dynamic => ResultKind::Dynamic,
                OutputKind::Unit => ResultKind::Unit,
                OutputKind::Plain | OutputKind::Bundle(_) => ResultKind::Plain,
            };
        }
        if event(e) {
            ResultKind::Event
        } else if dynamic(e) {
            ResultKind::Dynamic
        } else {
            ResultKind::Plain
        }
    }
}
