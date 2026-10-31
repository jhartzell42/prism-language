//! Blocks (Semantics ch. 4): widget blocks, expression blocks, `given:`.
//!
//! **A block of equations is one binding group; a block of steps is a sequence**
//! (Semantics ch. 4, "Binding groups and cycles"). A widget block and an expression
//! block's `let`s are groups: any line may use any name the block binds. `given:` is a
//! sequence: a line sees only the lines above.
//!
//! A group runs in two passes, because **the order of a group's lines decides positions
//! and nothing else**:
//!
//! - **Its lines, in order.** Every statement that takes a position (an element, a
//!   blueprint, `rebuild`, `for`, `@started`, a host document) runs at its line, so
//!   positions follow the page (Semantics ch. 3, "What takes a position"). A `let`, a
//!   hold's initial value and a local function take no position, so each is evaluated
//!   when a line first needs it, wherever it stands: a name is a [`Def`] until then.
//! - **The holds' `then`s, last.** A hold exists from its first use (`Build::hold`), and
//!   its `then` is wired after every line has run, so it may read a name bound anywhere
//!   in the block. This is what lets `@labels` read `$tags` above the hold that defines
//!   it (Reflex's `rec`, which Prism needs no annotation for).
//!
//! A line that reads an event bound by a later line gets a forward event, resolved when
//! that line runs (`Build::forward_event`). A dynamic bound by a later line has no forward
//! node in `prism_rt`, so reading one above its line is refused with a diagnostic; no
//! book program does it. A definition that needs itself while it is being evaluated is
//! a cycle: a hold's initial value naming itself, a cycle among plain values, or a bind
//! whose right side reads its own binder (a prompt cycle), each refused naming the name
//! (Semantics ch. 4; ch. 7, "There are no cycles"). The prompt-cycle check is only this
//! run-time one: cycles through `let !a = ...` lines that close through forward events
//! are not detected (README.md, doubts).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use prism_rt::{Build, ResultKind, Value};
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
}

/// The result kind of a `rebuild` or `for` bound by `pattern`, read off the binder's
/// sigils (prism_rt API.md, `ResultKind`; Semantics ch. 4's two tables). `None` when the
/// right side is not a builder.
fn bind_kind(pattern: &Pattern, value: &Expr) -> Option<ResultKind> {
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
        // A `for` hands back `!(K ==> T)` or `$(K ==> T)`; for `$`, the rows' last line
        // says whether they end on dynamics.
        ExprKind::For { body, .. } => Some(match sigils.first() {
            Some(Sigil::Bang) => ResultKind::Event,
            Some(Sigil::Dollar) => {
                if ends_on_dynamic(body) {
                    ResultKind::Dynamic
                } else {
                    ResultKind::Plain
                }
            }
            None => ResultKind::Plain,
            _ => ResultKind::Plain,
        }),
        _ => None,
    }
}

/// Whether a row's last line looks like a dynamic: `@ $x`, `$ v`, a `$"..."` literal. A
/// checker reads the type; compiler zero reads the page (D3).
fn ends_on_dynamic(body: &Block) -> bool {
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
    match body.stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Expr(e)) => dynamic(e),
        _ => false,
    }
}

impl Interp {
    /// Evaluate a group's definition of `name`, in the scopes up to frame `k`.
    pub fn force(&self, sc: &mut Scopes, k: usize, g: Rc<Group>, name: &str, b: Bx, loc: Loc) -> R<Value> {
        let inner = sc.frames.split_off(k + 1);
        let r = self.force_in(sc, &g, name, b, loc);
        sc.frames.extend(inner);
        r
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
                    let line = self.l.modules[sc.module].at(stmt(i).span);
                    return Err(self.fault(
                        loc,
                        format!("{name} is bound at {line}, below this line; compiler zero reads only events above the line that binds them (prism_rt has no forward dynamic)"),
                    ));
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
        for (i, stmt) in block.stmts.iter().enumerate() {
            let last = i + 1 == n;
            let loc = Loc { module: sc.module, span: stmt.span };
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
                StmtKind::Bind { pattern, value, then: None } => {
                    let binders = pattern_binders(pattern);
                    for n in &binders {
                        g.mark(n, DefState::InProgress);
                    }
                    let v = self.statement_value(pattern_kind(pattern, value), value, sc, b, loc);
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
        let leftover: Vec<Rc<str>> = g.forwards.borrow().keys().cloned().collect();
        if let Some(n) = leftover.first() {
            return Err(self.fault(Loc { module: sc.module, span: block.span }, format!("{n} was read but never bound")));
        }
        Ok(result)
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
                ExprKind::For { body, .. } => row_kind(body),
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

fn pattern_kind(pattern: &Pattern, value: &Expr) -> Option<ResultKind> {
    bind_kind(pattern, value)
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

/// What a `for`'s rows end on, read off their last line: an event, a dynamic, or plain.
fn row_kind(body: &Block) -> ResultKind {
    fn event(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Name(n) => n.sigils.first() == Some(&Sigil::Bang),
            ExprKind::Binary { op: BinOp::On | BinOp::Merge, .. } => true,
            ExprKind::FreeSigil { sigil: Sigil::At, operand } => event(operand),
            ExprKind::Group(e) => event(e),
            _ => false,
        }
    }
    match body.stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Expr(e)) if event(e) => ResultKind::Event,
        _ if ends_on_dynamic(body) => ResultKind::Dynamic,
        _ => ResultKind::Plain,
    }
}
