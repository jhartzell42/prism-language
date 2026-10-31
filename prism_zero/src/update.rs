//! `&` blocks (Semantics ch. 5): `update v:` and its statements.
//!
//! **An `&` block means the pure function it translates to** (round 47 AB): the
//! translation threads a state record (the updated value and every live local) through
//! the statements. prism_zero runs that state directly: the block's state is a frame of
//! locals (`&_` and each `let &x`), each statement replaces one part of one local, and
//! the block's value is `&_` at the end. The values are persistent (`im_rc` and `Rc`), and
//! a part is replaced with `Rc::make_mut`, so a part nobody else holds is edited in place
//! and a shared part is copied first: K5's "edited in place when unique, else copy",
//! with K1's one meaning either way. `Buffer a` is an array and `Table k v` a record of
//! its keys in insertion order and a map (K6), as the translation section says.
//!
//! The statements: `&r <= e` and `&r += e` (and `-=`, `*=`, `/=`) through a lens path of
//! field and index steps (setting `nil` at an index removes the key); `let &x = e`;
//! `if`, `if p <- e`, `match`; `while c:` and `while p <- e if g:`, its pattern's names
//! bound for the turn (doubt WB); `for &s <- &p:` with a reference per element, the bare
//! `for &p:` rebinding `&_`, and `for x <- xs:` over a plain value. The borrow rule is the
//! checker's (D3); prism_zero does not refuse overlapping references.
//!
//! A `while` inside an unbounded function counts one cycle point per turn: its
//! translation is a recursive `Loop`, and each turn is a call of it from itself.

use std::collections::HashMap;
use std::rc::Rc;

use prism_rt::Value;
use prism_syntax::ast::*;

use crate::interp::{Bx, Interp, Loc, OpKind, Scopes, CYCLE_BUDGET, UNBOUNDED};
use crate::ops::R;

/// One step of a lens path.
#[derive(Clone, Debug)]
enum Step {
    Field(Rc<str>),
    Index(Value),
}

/// A reference: a local and a lens path into it.
#[derive(Clone, Debug)]
struct Lens {
    root: Rc<str>,
    steps: Vec<Step>,
}

/// The aliases `for &s <- &p:` makes: a name standing for a path.
#[derive(Default)]
struct Refs {
    aliases: HashMap<Rc<str>, Lens>,
    hidden: usize,
}

impl Interp {
    /// Run `update target:` and its statements; the value is the updated copy.
    pub fn run_update(&self, target: Value, body: &'static Block, sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Value> {
        let _ = loc;
        sc.push();
        sc.bind(Rc::from("&_"), target);
        let mut refs = Refs::default();
        let r = self.update_stmts(body, sc, b.as_deref_mut(), &mut refs);
        let out = sc.frames.last().and_then(|f| f.vars.get("&_")).cloned().unwrap_or(Value::Unit);
        sc.pop();
        r.map(|_| out)
    }

    fn update_stmts(&self, block: &'static Block, sc: &mut Scopes, mut b: Bx, refs: &mut Refs) -> R<()> {
        for stmt in &block.stmts {
            self.update_stmt(stmt, sc, b.as_deref_mut(), refs)?;
        }
        Ok(())
    }

    fn update_stmt(&self, stmt: &'static Stmt, sc: &mut Scopes, mut b: Bx, refs: &mut Refs) -> R<()> {
        let loc = Loc { module: sc.module, span: stmt.span };
        match &stmt.kind {
            StmtKind::Set { target, value } => {
                let v = self.eval(value, sc, b.as_deref_mut())?;
                let lens = self.lens(target, sc, b.as_deref_mut(), refs)?;
                self.write(sc, refs, &lens, v, loc)
            }
            StmtKind::Accum { target, op, value } => {
                let v = self.eval(value, sc, b.as_deref_mut())?;
                let lens = self.lens(target, sc, b.as_deref_mut(), refs)?;
                let cur = self.read(sc, &lens, loc)?;
                let bin = match op {
                    AccumOp::Add => BinOp::Add,
                    AccumOp::Sub => BinOp::Sub,
                    AccumOp::Mul => BinOp::Mul,
                    AccumOp::Div => BinOp::Div,
                };
                let new = self.apply(&self.op_value(OpKind::Bin(bin)), vec![cur, v], None, loc)?;
                self.write(sc, refs, &lens, new, loc)
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
                Ok(())
            }
            StmtKind::While { cond, body } => {
                loop {
                    if UNBOUNDED.with(|u| u.depth.get() > 0) {
                        let over = UNBOUNDED.with(|u| {
                            let p = u.points.get() + 1;
                            u.points.set(p);
                            matches!(u.budget.get(), Some(budget) if p > budget)
                        });
                        if over {
                            return Err(prism_rt::Fault::new(CYCLE_BUDGET, "the cycle budget ran out"));
                        }
                    }
                    let binds = match self.update_cond(cond, sc, b.as_deref_mut())? {
                        Some(binds) => binds,
                        None => break,
                    };
                    sc.push();
                    for (n, v) in binds {
                        sc.bind(n, v);
                    }
                    let r = self.update_stmts(body, sc, b.as_deref_mut(), refs);
                    self.pop_keeping_locals(sc);
                    r?;
                }
                Ok(())
            }
            StmtKind::Expr(e) => self.update_expr(e, sc, b, refs),
            StmtKind::Fn(_) | StmtKind::Signature(_) => Ok(()),
            _ => Err(self.fault(loc, "this statement does not stand in an `update` block (Semantics ch. 5)")),
        }
    }

    /// Leave a nested frame, carrying out the locals (`&x`) it changed that live outside it.
    fn pop_keeping_locals(&self, sc: &mut Scopes) {
        let inner = sc.frames.pop().map(|f| f.vars).unwrap_or_default();
        for (n, v) in inner {
            if n.starts_with('&') {
                for f in sc.frames.iter_mut().rev() {
                    if f.vars.contains_key(&n) {
                        f.vars.insert(n.clone(), v.clone());
                        break;
                    }
                }
            }
        }
    }

    /// A `while`'s or `if`'s condition: `Some` bindings to run the arm with, or `None`.
    fn update_cond(&self, cond: &'static Cond, sc: &mut Scopes, mut b: Bx) -> R<Option<Vec<(Rc<str>, Value)>>> {
        match cond {
            Cond::Expr(c) => match self.eval(c, sc, b)? {
                Value::Bool(true) => Ok(Some(vec![])),
                Value::Bool(false) => Ok(None),
                other => Err(self.fault(Loc { module: sc.module, span: c.span }, format!("a condition is a Bool, and this one is {}", crate::ops::show_plain(&other)))),
            },
            Cond::Bind { pattern, value, guard } => {
                let v = self.eval(value, sc, b.as_deref_mut())?;
                let mut binds = Vec::new();
                if !crate::pattern::matches(self, sc.module, pattern, &v, &mut binds)? {
                    return Ok(None);
                }
                if let Some(g) = guard {
                    sc.push();
                    for (n, v) in binds.clone() {
                        sc.bind(n, v);
                    }
                    let r = self.eval(g, sc, b);
                    sc.pop();
                    if !matches!(r?, Value::Bool(true)) {
                        return Ok(None);
                    }
                }
                Ok(Some(binds))
            }
        }
    }

    /// An expression statement: `if`, `match`, `for`, or a value thrown away.
    fn update_expr(&self, e: &'static Expr, sc: &mut Scopes, mut b: Bx, refs: &mut Refs) -> R<()> {
        let loc = Loc { module: sc.module, span: e.span };
        match &e.kind {
            ExprKind::If { cond, then, else_ } => {
                let arm = match self.update_cond(cond, sc, b.as_deref_mut())? {
                    Some(binds) => Some((then, binds)),
                    None => else_.as_ref().map(|e| (e, vec![])),
                };
                if let Some((block, binds)) = arm {
                    sc.push();
                    for (n, v) in binds {
                        sc.bind(n, v);
                    }
                    let r = self.update_stmts(block, sc, b, refs);
                    self.pop_keeping_locals(sc);
                    r?;
                }
                Ok(())
            }
            ExprKind::Match { scrutinee, arms } => {
                let s = self.eval(scrutinee, sc, b.as_deref_mut())?;
                for arm in arms {
                    let mut binds = Vec::new();
                    if let Some(p) = &arm.pattern {
                        if !crate::pattern::matches(self, sc.module, p, &s, &mut binds)? {
                            continue;
                        }
                    }
                    sc.push();
                    for (n, v) in binds {
                        sc.bind(n, v);
                    }
                    if let Some(g) = &arm.guard {
                        match self.eval(g, sc, b.as_deref_mut()) {
                            Ok(Value::Bool(true)) => {}
                            Ok(_) => {
                                sc.pop();
                                continue;
                            }
                            Err(err) => {
                                sc.pop();
                                return Err(err);
                            }
                        }
                    }
                    let r = self.update_stmts(&arm.body, sc, b, refs);
                    self.pop_keeping_locals(sc);
                    return r;
                }
                Err(self.fault(loc, format!("no arm matches {}", crate::ops::show_plain(&s))))
            }
            ExprKind::For { binders, source, body } => self.update_for(binders.as_ref(), source, body, sc, b, refs, loc),
            _ => {
                self.eval(e, sc, b)?;
                Ok(())
            }
        }
    }

    fn update_for(
        &self,
        binders: Option<&'static ForBinders>,
        source: &'static Expr,
        body: &'static Block,
        sc: &mut Scopes,
        mut b: Bx,
        refs: &mut Refs,
        loc: Loc,
    ) -> R<()> {
        let by_ref = is_reference(source);
        let outer_ix = self.lookup_local(sc, "ix", None, loc)?;
        let outer_jx = self.lookup_local(sc, "jx", None, loc)?;
        if !by_ref {
            // `for x <- xs:` over a plain value yields values.
            let coll = self.eval(source, sc, b.as_deref_mut())?;
            let items: Vec<(Value, Value)> = entries(&coll);
            for (ix, (k, v)) in items.into_iter().enumerate() {
                sc.push();
                bind_loop_names(sc, ix, &k, outer_ix.clone(), outer_jx.clone());
                let mut binds = Vec::new();
                let ok = match binders {
                    Some(fb) => match &fb.second {
                        Some(second) => {
                            crate::pattern::matches(self, sc.module, &fb.first, &k, &mut binds)?
                                && crate::pattern::matches(self, sc.module, second, &v, &mut binds)?
                        }
                        None => crate::pattern::matches(self, sc.module, &fb.first, &v, &mut binds)?,
                    },
                    None => {
                        binds.push((Rc::from("_"), v.clone()));
                        true
                    }
                };
                if !ok {
                    sc.pop();
                    return Err(self.fault(loc, "a `for` binder does not match its element"));
                }
                for (n, bv) in binds {
                    sc.bind(n, bv);
                }
                let r = self.update_stmts(body, sc, b.as_deref_mut(), refs);
                self.pop_keeping_locals(sc);
                r?;
            }
            return Ok(());
        }
        // `for &s <- &p:` yields a reference per element; `for &p:` rebinds `&_`.
        let lens = self.lens(source, sc, b.as_deref_mut(), refs)?;
        let coll = self.read(sc, &lens, loc)?;
        let keys: Vec<Value> = entries(&coll).into_iter().map(|(k, _)| k).collect();
        let name: Rc<str> = match binders.map(|fb| &fb.first.kind) {
            Some(PatternKind::Name(n)) => Rc::from(n.text()),
            None => Rc::from("&_"),
            _ => return Err(self.fault(loc, "a `for` over a reference binds a reference, `&s`")),
        };
        // The bare form hides the block's own `&_` for the loop's length.
        let hidden: Option<Rc<str>> = if &*name == "&_" {
            refs.hidden += 1;
            let h: Rc<str> = Rc::from(format!("&_#{}", refs.hidden));
            let root = self.lookup_local(sc, "&_", None, loc)?.unwrap_or(Value::Unit);
            self.set_local(sc, "&_", Value::Unit);
            sc.frames.last_mut().unwrap().vars.insert(h.clone(), root);
            Some(h)
        } else {
            None
        };
        let base = match &hidden {
            Some(h) => {
                let mut l = lens.clone();
                if &*l.root == "&_" {
                    l.root = h.clone();
                }
                l
            }
            None => lens.clone(),
        };
        let mut result = Ok(());
        for (ix, k) in keys.iter().enumerate() {
            let mut el = base.clone();
            el.steps.push(Step::Index(k.clone()));
            refs.aliases.insert(name.clone(), el.clone());
            let cur = self.read(sc, &el, loc)?;
            let cur = match cur {
                Value::Opt(Some(x)) => (*x).clone(),
                other => other,
            };
            sc.push();
            bind_loop_names(sc, ix, k, outer_ix.clone(), outer_jx.clone());
            sc.bind(name.clone(), cur);
            let r = self.update_stmts(body, sc, b.as_deref_mut(), refs);
            sc.pop();
            if let Err(e) = r {
                result = Err(e);
                break;
            }
        }
        refs.aliases.remove(&name);
        if let Some(h) = hidden {
            let root = self.lookup_local(sc, &h, None, loc)?.unwrap_or(Value::Unit);
            self.set_local(sc, "&_", root);
        }
        result
    }

    /// The lens a reference expression names: `&x`, `&_.regs.acc`, `&parent[&ra]`.
    fn lens(&self, e: &'static Expr, sc: &mut Scopes, mut b: Bx, refs: &Refs) -> R<Lens> {
        let loc = Loc { module: sc.module, span: e.span };
        match &e.kind {
            ExprKind::Name(n) if n.sigils.first() == Some(&Sigil::Amp) => {
                let name: Rc<str> = Rc::from(n.text());
                Ok(refs.aliases.get(&name).cloned().unwrap_or(Lens { root: name, steps: vec![] }))
            }
            ExprKind::Field { recv, name, .. } => {
                let mut l = self.lens(recv, sc, b, refs)?;
                l.steps.push(Step::Field(Rc::from(name.name.as_str())));
                Ok(l)
            }
            ExprKind::Index { recv, index } => {
                let k = self.eval(index, sc, b.as_deref_mut())?;
                let mut l = self.lens(recv, sc, b, refs)?;
                l.steps.push(Step::Index(k));
                Ok(l)
            }
            ExprKind::Group(inner) => self.lens(inner, sc, b, refs),
            ExprKind::Call { .. } => Err(self.out_of_scope(loc, "named lenses `&Home` and `Over`")),
            _ => Err(self.fault(loc, "a reference is `&_`, a local `&x`, or a lens path of `.field` and `[index]` steps from one")),
        }
    }

    fn read(&self, sc: &mut Scopes, lens: &Lens, loc: Loc) -> R<Value> {
        let mut v = self
            .lookup_local(sc, &lens.root, None, loc)?
            .ok_or_else(|| self.fault(loc, format!("{} is not a local of this block", lens.root)))?;
        for s in &lens.steps {
            v = match s {
                Step::Field(f) => self.field(&v, f, loc)?,
                Step::Index(k) => self.index(&v, k, loc)?,
            };
        }
        Ok(v)
    }

    fn write(&self, sc: &mut Scopes, refs: &Refs, lens: &Lens, v: Value, loc: Loc) -> R<()> {
        let root = self
            .lookup_local(sc, &lens.root, None, loc)?
            .ok_or_else(|| self.fault(loc, format!("{} is not a local of this block", lens.root)))?;
        let new = self.set_path(root, &lens.steps, v, loc)?;
        self.set_local(sc, &lens.root, new);
        // A reference a `for` lent keeps showing its element.
        let aliases: Vec<(Rc<str>, Lens)> = refs.aliases.iter().map(|(n, l)| (n.clone(), l.clone())).collect();
        for (n, l) in aliases {
            if l.root == lens.root {
                if let Ok(cur) = self.read(sc, &l, loc) {
                    let cur = match cur {
                        Value::Opt(Some(x)) => (*x).clone(),
                        other => other,
                    };
                    self.set_local(sc, &n, cur);
                }
            }
        }
        Ok(())
    }

    /// Replace a local's value where it lives.
    fn set_local(&self, sc: &mut Scopes, name: &str, v: Value) {
        for f in sc.frames.iter_mut().rev() {
            if f.vars.contains_key(name) {
                f.vars.insert(Rc::from(name), v);
                return;
            }
        }
        sc.bind(Rc::from(name), v);
    }

    /// Set the part at `steps` inside `whole`, in place where nobody else holds it.
    fn set_path(&self, whole: Value, steps: &[Step], v: Value, loc: Loc) -> R<Value> {
        // A part keeps its type: an `Int` literal set into a `Nat` part is a `Nat`, as a
        // checker would have typed the literal (`&fuel <= 0`, then `&fuel -? 1`).
        let Some((first, rest)) = steps.split_first() else { return Ok(keep_type(&whole, v)) };
        match (whole, first) {
            (Value::Record(mut r), Step::Field(f)) => {
                let rec = Rc::make_mut(&mut r);
                let slot = rec
                    .fields
                    .iter_mut()
                    .find(|(n, _)| **n == **f)
                    .ok_or_else(|| self.fault(loc, format!("{} has no field {f}", rec.name)))?;
                let part = std::mem::take(&mut slot.1);
                slot.1 = self.set_path(part, rest, v, loc)?;
                Ok(Value::Record(r))
            }
            (t @ Value::Record(_), Step::Index(k)) => {
                let cur = crate::builtins::table_get(&t, k)?;
                let new = self.set_index_part(cur, rest, v, loc)?;
                crate::builtins::table_set(t, k.clone(), new)
            }
            (Value::Map(mut m), Step::Index(k)) => {
                let cur = m.get(k).cloned().map(Value::some).unwrap_or(Value::nil());
                match self.set_index_part(cur, rest, v, loc)? {
                    Value::Opt(Some(x)) => {
                        m.insert(k.clone(), (*x).clone());
                    }
                    Value::Opt(None) => {
                        m.remove(k);
                    }
                    other => {
                        m.insert(k.clone(), other);
                    }
                }
                Ok(Value::Map(m))
            }
            (Value::Array(mut a), Step::Index(k)) => {
                let i = crate::ops::index_of(k).filter(|i| *i < a.len()).ok_or_else(|| {
                    self.fault(loc, format!("index {} is past the end of an array of {}", crate::ops::show_plain(k), a.len()))
                })?;
                let cur = Value::some(a[i].clone());
                match self.set_index_part(cur, rest, v, loc)? {
                    Value::Opt(Some(x)) => a.set(i, (*x).clone()),
                    Value::Opt(None) => return Err(self.fault(loc, "an array's element cannot be removed by setting `nil`")),
                    other => a.set(i, other),
                };
                Ok(Value::Array(a))
            }
            (Value::Opt(Some(x)), _) => Ok(Value::some(self.set_path((*x).clone(), steps, v, loc)?)),
            (other, _) => Err(self.fault(loc, format!("{} has no part here", crate::ops::show_plain(&other)))),
        }
    }

    /// An index path's part is `?V` (Semantics ch. 5): a deeper step goes through the `?`.
    fn set_index_part(&self, cur: Value, rest: &[Step], v: Value, loc: Loc) -> R<Value> {
        if rest.is_empty() {
            return Ok(v);
        }
        match cur {
            Value::Opt(Some(x)) => Ok(Value::some(self.set_path((*x).clone(), rest, v, loc)?)),
            _ => Err(self.fault(loc, "a path through a missing key")),
        }
    }
}

fn is_reference(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Name(n) => n.sigils.first() == Some(&Sigil::Amp),
        ExprKind::Field { recv, .. } | ExprKind::Index { recv, .. } => is_reference(recv),
        ExprKind::Group(e) => is_reference(e),
        _ => false,
    }
}

/// The keys and values of a collection, in order: an array's indices, a map's keys, a
/// table's keys in insertion order.
fn entries(v: &Value) -> Vec<(Value, Value)> {
    match v {
        Value::Array(a) => a.iter().enumerate().map(|(i, x)| (Value::nat(i as u64), x.clone())).collect(),
        Value::Map(m) => m.iter().map(|(k, x)| (k.clone(), x.clone())).collect(),
        Value::Record(r) if &*r.name == "Table" => crate::builtins::table_entries(v),
        _ => vec![],
    }
}

fn bind_loop_names(sc: &mut Scopes, ix: usize, key: &Value, outer_ix: Option<Value>, outer_jx: Option<Value>) {
    sc.bind(Rc::from("ix"), Value::nat(ix as u64));
    sc.bind(Rc::from("key"), key.clone());
    if let Some(j) = outer_ix {
        sc.bind(Rc::from("jx"), j);
    }
    if let Some(k) = outer_jx {
        sc.bind(Rc::from("kx"), k);
    }
}

/// The value set into a part, with the part's integer type kept.
fn keep_type(old: &Value, v: Value) -> Value {
    match (old, &v) {
        (Value::Nat(_), Value::Int(_)) => crate::interp::to_nat(v),
        (Value::Opt(Some(o)), Value::Opt(Some(x))) if matches!((&**o, &**x), (Value::Nat(_), Value::Int(_))) => {
            Value::some(crate::interp::to_nat((**x).clone()))
        }
        _ => v,
    }
}
