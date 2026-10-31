//! Name resolution without running anything, for `prism_zero check` (D2: parse and
//! resolve only; the type checker is `prismc`'s, D3).
//!
//! Every unqualified name written in an expression must be bound where it stands: by a
//! pattern, by its block (a binding group binds every name of the block, Semantics ch. 4),
//! by the language (`_`, `$_`, `key`, `ix`, `jx`, `kx`), or by the module's own
//! declarations and what its `use` lines and the prelude bring in. A qualified name must
//! be a tag, a builtin reached by its qualified name, or a declaration of a module the
//! file uses. A constructor in a pattern must resolve. Out-of-scope features the parser
//! flagged are reported too, since `prismc` will refuse them.

use std::collections::HashSet;

use prism_syntax::ast::*;

use crate::load::{pattern_binders, Loaded, ModId};

struct Ctx<'a> {
    l: &'a Loaded,
    module: ModId,
    scopes: Vec<HashSet<String>>,
    errors: Vec<String>,
}

/// Every resolution error of the program's own files (not the bundled ones).
pub fn check(l: &Loaded) -> Vec<String> {
    let mut errors = Vec::new();
    for (id, m) in l.modules.iter().enumerate() {
        if m.bundled {
            continue;
        }
        for (span, feature, msg) in &m.out_of_scope {
            errors.push(format!("{}: out of scope ({feature}): {msg}", m.at(*span)));
        }
        let mut cx = Ctx { l, module: id, scopes: vec![HashSet::new()], errors: Vec::new() };
        for d in &m.ast.decls {
            cx.decl(d);
        }
        errors.extend(cx.errors);
    }
    errors
}

impl Ctx<'_> {
    fn bound(&self, n: &str) -> bool {
        if self.scopes.iter().any(|s| s.contains(n)) {
            return true;
        }
        let m = &self.l.modules[self.module];
        m.decls.contains_key(n)
            || m.scope.contains_key(n)
            || matches!(n, "True" | "False" | "never" | "!never" | "_" | "$_" | "key" | "ix" | "jx" | "kx" | "&_")
    }

    fn err(&mut self, span: prism_syntax::Span, msg: String) {
        let at = self.l.modules[self.module].at(span);
        self.errors.push(format!("{at}: {msg}"));
    }

    fn with<T>(&mut self, names: impl IntoIterator<Item = String>, f: impl FnOnce(&mut Self) -> T) -> T {
        self.scopes.push(names.into_iter().collect());
        let r = f(self);
        self.scopes.pop();
        r
    }

    fn decl(&mut self, d: &Decl) {
        match &d.kind {
            DeclKind::Fn(c) => self.clause(c),
            DeclKind::Let(ld) => {
                self.pattern(&ld.pattern);
                self.expr(&ld.value);
            }
            DeclKind::Widget(w) => self.block(&w.body),
            DeclKind::Trait(t) => {
                for i in &t.items {
                    if let TraitItem::Fn(c) = i {
                        self.clause(c);
                    }
                }
            }
            DeclKind::Impl(i) => {
                for it in &i.items {
                    if let TraitItem::Fn(c) = it {
                        self.clause(c);
                    }
                }
            }
            DeclKind::Check(c) => {
                for line in &c.lines {
                    self.expr(&line.expr);
                }
            }
            DeclKind::Test(t) => self.expr(&t.blueprint),
            DeclKind::Stmt(s) => self.stmt(s),
            _ => {}
        }
    }

    fn clause(&mut self, c: &FnClause) {
        let names: Vec<String> = c.params.iter().flat_map(pattern_binders).collect();
        for p in &c.params {
            self.pattern(p);
        }
        self.with(names, |cx| match &c.body {
            FnBody::Expr(e) => cx.expr(e),
            FnBody::Block(b) => cx.block(b),
        });
    }

    fn block(&mut self, b: &Block) {
        let mut names = Vec::new();
        for s in &b.stmts {
            match &s.kind {
                StmtKind::Bind { pattern, .. } | StmtKind::Let { pattern, .. } => names.extend(pattern_binders(pattern)),
                StmtKind::Fn(c) => {
                    if let DefName::Name(n) = &c.name {
                        names.push(n.text());
                    }
                }
                StmtKind::Widget(w) => names.push(w.name.text()),
                _ => {}
            }
        }
        self.with(names, |cx| {
            for s in &b.stmts {
                cx.stmt(s);
            }
        });
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Bind { pattern, value, then } => {
                self.pattern(pattern);
                self.expr(value);
                if let Some(t) = then {
                    self.expr(t);
                }
            }
            StmtKind::Let { pattern, value, .. } => {
                self.pattern(pattern);
                self.expr(value);
            }
            StmtKind::Ignore(e) | StmtKind::Expr(e) | StmtKind::Return(e) | StmtKind::Require(e) | StmtKind::Forbid(e) => self.expr(e),
            StmtKind::Fn(c) => self.clause(c),
            StmtKind::Widget(w) => self.block(&w.body),
            StmtKind::Set { target, value } | StmtKind::Accum { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::While { cond, body } => {
                let names = self.cond(cond);
                self.with(names, |cx| cx.block(body));
            }
            StmtKind::Emit(_) | StmtKind::Signature(_) => {}
        }
    }

    fn cond(&mut self, c: &Cond) -> Vec<String> {
        match c {
            Cond::Expr(e) => {
                self.expr(e);
                vec![]
            }
            Cond::Bind { pattern, value, guard } => {
                self.expr(value);
                self.pattern(pattern);
                let names = pattern_binders(pattern);
                if let Some(g) = guard {
                    self.with(names.clone(), |cx| cx.expr(g));
                }
                names
            }
        }
    }

    fn name(&mut self, n: &SName) {
        if n.path.len() == 1 {
            let t = n.text();
            if !self.bound(&t) {
                self.err(n.span, format!("unknown name {t}"));
            }
            return;
        }
        let full = n.path.join(".");
        if self.l.tags.contains_key(&full) || crate::builtins::is_qualified_builtin(&full) {
            return;
        }
        let module = n.path[..n.path.len() - 1].join(".");
        let base = format!("{}{}", prism_syntax::token::sigils_str(&n.sigils), n.base());
        match self.l.modules.iter().position(|m| m.name.as_deref() == Some(module.as_str())) {
            Some(m) if self.l.modules[self.module].qualified.contains(&m) => {
                if !self.l.modules[m].decls.contains_key(base.as_str()) {
                    self.err(n.span, format!("module {module} declares no {base}"));
                }
            }
            Some(_) => self.err(n.span, format!("{full}: module {module} is not used here; write `use {module}`")),
            None => self.err(n.span, format!("unknown name {full}: no tag, and no module {module} among the files")),
        }
    }

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Name(n) if n.path.len() > 1 => {
                if !self.l.tags.contains_key(&n.path.join(".")) {
                    self.name(n);
                }
            }
            // A bare uppercase name that is no constructor binds a function value.
            PatternKind::Ctor { name, args } => {
                if name.path.len() == 1 && !self.bound(&name.text()) {
                    self.err(name.span, format!("unknown constructor {}", name.text()));
                }
                args.iter().for_each(|a| self.pattern(a));
            }
            PatternKind::Record { name, fields, .. } => {
                let full = name.path.join(".");
                if name.path.len() == 1 && !self.bound(&full) {
                    self.err(name.span, format!("unknown constructor {full}"));
                } else if name.path.len() > 1 && !self.l.tags.contains_key(&full) {
                    self.err(name.span, format!("unknown tag {full}"));
                }
                fields.iter().for_each(|f| self.pattern(&f.pattern));
            }
            PatternKind::Sigil { inner, .. } | PatternKind::Group(inner) => self.pattern(inner),
            PatternKind::Tuple(ps) | PatternKind::Array(ps) => ps.iter().for_each(|a| self.pattern(a)),
            _ => {}
        }
    }

    fn arms_block(&mut self, names: Vec<String>, b: &Block) {
        self.with(names, |cx| cx.block(b));
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(n) => self.name(n),
            ExprKind::Lit(l) => {
                if let LitKind::Text(segs) = &l.kind {
                    for s in segs {
                        if let TextSeg::Placeholder(x) = s {
                            self.expr(x);
                        }
                    }
                }
            }
            ExprKind::Tuple(xs) | ExprKind::Array(xs) => xs.iter().for_each(|x| self.expr(x)),
            ExprKind::Map(es) => es.iter().for_each(|(k, v)| {
                self.expr(k);
                self.expr(v);
            }),
            ExprKind::Group(x) | ExprKind::NonEmpty(x) | ExprKind::Any(x) => self.expr(x),
            ExprKind::Call { callee, args, .. } => {
                self.expr(callee);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Binary { lhs, rhs, .. } | ExprKind::Suffixed { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Unary { operand, .. } | ExprKind::FreeSigil { operand, .. } => self.expr(operand),
            ExprKind::Field { recv, .. } => self.expr(recv),
            ExprKind::Index { recv, index } => {
                self.expr(recv);
                self.expr(index);
            }
            ExprKind::TopicIndex(i) => self.expr(i),
            ExprKind::Method { recv, method, args } => {
                self.expr(recv);
                self.name(method);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::TypeArgs { target, .. } => self.expr(target),
            ExprKind::Cast { expr, .. } => self.expr(expr),
            ExprKind::Lambda { params, body } => {
                let names: Vec<String> = match params {
                    Some(ps) => ps.iter().flat_map(pattern_binders).collect(),
                    None => vec!["_".into()],
                };
                if let Some(ps) = params {
                    ps.iter().for_each(|p| self.pattern(p));
                }
                self.with(names, |cx| match &**body {
                    LambdaBody::Expr(x) => cx.expr(x),
                    LambdaBody::Block(b) => cx.block(b),
                });
            }
            ExprKind::If { cond, then, else_ } => {
                let names = self.cond(cond);
                self.arms_block(names, then);
                if let Some(e) = else_ {
                    self.block(e);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                for a in arms {
                    let names = a.pattern.as_ref().map(pattern_binders).unwrap_or_default();
                    if let Some(p) = &a.pattern {
                        self.pattern(p);
                    }
                    self.with(names, |cx| {
                        if let Some(g) = &a.guard {
                            cx.expr(g);
                        }
                        cx.block(&a.body);
                    });
                }
            }
            ExprKind::Rebuild { scrutinee, body } => {
                self.expr(scrutinee);
                self.block(body);
            }
            ExprKind::For { binders, source, body } => {
                self.expr(source);
                let mut names = Vec::new();
                if let Some(b) = binders {
                    names.extend(pattern_binders(&b.first));
                    if let Some(s) = &b.second {
                        names.extend(pattern_binders(s));
                    }
                }
                self.with(names, |cx| cx.block(body));
            }
            ExprKind::Given(b) | ExprKind::WidgetBlock(b) | ExprKind::Monad { body: b, .. } => self.block(b),
            ExprKind::Update { target, body } => {
                self.expr(target);
                // An `&` block's locals are bound by its `let &x` lines.
                self.block(body);
            }
            ExprKind::Template(_) | ExprKind::TopicField(_) | ExprKind::Unit(_) | ExprKind::Nil(_) | ExprKind::Bound(_) => {}
            ExprKind::Absent | ExprKind::Rest | ExprKind::Lens(_) | ExprKind::Error => {}
        }
    }
}
