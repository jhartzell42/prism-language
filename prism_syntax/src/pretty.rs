//! The printer: a tree back to Prism source.
//!
//! The contract is the round-trip test: `parse(print(t)) == t` for every tree `t` the
//! parser produces (spans aside). The tree keeps the program's groups (`ExprKind::Group`),
//! so the printer adds no parentheses of its own; what it must get right is everything the
//! lexer reads from spacing and adjacency:
//!
//! - binary operators get a space on both sides and signs none, so the spacing rule reads
//!   each as it was;
//! - a free sigil gets a space after it unless a bracket follows, an attached one none;
//! - a group whose contents begin with a bracket gets a space after its `(`, so the
//!   pairing rule does not read the two opens as a double bracket;
//! - indented blocks print at four spaces a level, and a block opened inside a statement
//!   ends that statement (layout rule 7), which is where the tree has it;
//! - a `.Method` whose receiver took the rest of its expression prints on a continuation
//!   line (rule 9);
//! - a template prints its text at column 0, so no closing-tag column is stripped, and
//!   each placeholder with a column alone on its line at that column.

use crate::ast::*;
use crate::token::sigils_str;

/// Prints a module.
pub fn module(m: &Module) -> String {
    let mut p = Printer::default();
    p.module(m);
    p.out
}

/// Prints one expression on one line where it can.
pub fn expr(e: &Expr) -> String {
    let mut p = Printer::default();
    p.expr(e);
    p.out
}

pub fn ty(t: &Type) -> String {
    let mut p = Printer::default();
    p.ty(t);
    p.out
}

pub fn pattern(t: &Pattern) -> String {
    let mut p = Printer::default();
    p.pattern(t);
    p.out
}

#[derive(Default)]
struct Printer {
    out: String,
    indent: usize,
}

impl Printer {
    fn w(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn line(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push(' ');
        }
    }

    fn sep_list<T>(&mut self, items: &[T], sep: &str, mut f: impl FnMut(&mut Printer, &T)) {
        for (i, it) in items.iter().enumerate() {
            if i > 0 {
                self.w(sep);
            }
            f(self, it);
        }
    }

    // ----- the file ---------------------------------------------------------------------------

    fn module(&mut self, m: &Module) {
        let mut first = true;
        let nl = |p: &mut Printer, first: &mut bool| {
            if !*first {
                p.w("\n");
            }
            *first = false;
        };
        if let Some(t) = &m.template {
            self.template(t);
            first = false;
        }
        if let Some(n) = &m.name {
            nl(self, &mut first);
            self.w("module ");
            self.w(&n.path.join("."));
        }
        if let Some(r) = &m.emit {
            nl(self, &mut first);
            self.w("emit ");
            self.row(r);
        }
        for u in &m.uses {
            nl(self, &mut first);
            self.w("use ");
            self.w(&u.module.path.join("."));
            if u.star {
                self.w(".*");
            }
        }
        let mut prev_kind: Option<&DeclKind> = None;
        for d in &m.decls {
            if !first {
                self.w("\n");
                let blank = match (&d.kind, prev_kind) {
                    (DeclKind::Fn(_) | DeclKind::Derive(_), Some(DeclKind::Fn(_) | DeclKind::Signature(_) | DeclKind::Type(_))) => false,
                    (DeclKind::Widget(_), Some(DeclKind::Signature(_))) => false,
                    (DeclKind::Stmt(_), Some(DeclKind::Stmt(_))) => false,
                    _ => true,
                };
                if blank {
                    self.w("\n");
                }
            }
            first = false;
            self.decl(d);
            prev_kind = Some(&d.kind);
        }
        self.w("\n");
    }

    fn row(&mut self, r: &Row) {
        self.sep_list(&r.terms, " | ", |p, t| match t {
            RowTerm::Family(q) => {
                p.w(&q.path.join("."));
                p.w(".*");
            }
            RowTerm::Type(t) => p.ty(t),
            RowTerm::Var(v) => p.w(&v.name),
        });
    }

    fn decl(&mut self, d: &Decl) {
        match &d.kind {
            DeclKind::Signature(s) => self.signature(s),
            DeclKind::Fn(f) => self.fn_clause(f),
            DeclKind::Let(l) => {
                self.w("let ");
                self.pattern(&l.pattern);
                if let Some(t) = &l.ty {
                    self.w(": ");
                    self.ty(t);
                }
                self.w(" = ");
                self.rhs(&l.value);
            }
            DeclKind::Widget(w) => {
                self.w(&w.name.text());
                self.block_after_colon(&w.body);
            }
            DeclKind::Type(t) => self.type_decl(t),
            DeclKind::Tag(t) => {
                self.w("tag ");
                self.w(&t.name.path.join("."));
                self.w(" of ");
                self.ty(&t.of);
                self.bounds(&t.bounds);
                if !t.fields.is_empty() {
                    self.w(":");
                    self.fields(&t.fields);
                }
            }
            DeclKind::Derive(ts) => {
                self.w("derive ");
                self.sep_list(ts, ", ", |p, t| p.trait_ref(t));
            }
            DeclKind::Trait(t) => {
                self.w("trait ");
                self.trait_ref(&t.name);
                for prm in &t.params {
                    self.w(" ");
                    self.w(&prm.name);
                }
                self.bounds(&t.bounds);
                self.w(":");
                self.trait_items(&t.items);
            }
            DeclKind::Impl(i) => {
                self.w("impl ");
                self.trait_ref(&i.trait_);
                self.w(" ");
                self.ty(&i.ty);
                self.bounds(&i.bounds);
                self.w(":");
                self.trait_items(&i.items);
            }
            DeclKind::Check(c) => {
                self.w("check ");
                self.w(&c.name.name);
                self.w(":");
                self.indent += 4;
                for l in &c.lines {
                    self.line();
                    self.rhs(&l.expr);
                    if let Some(p) = &l.pattern {
                        self.w(" => ");
                        self.pattern(p);
                    }
                }
                self.indent -= 4;
            }
            DeclKind::Test(t) => self.test(t),
            DeclKind::Stmt(s) => self.stmt(s),
        }
    }

    fn signature(&mut self, s: &Signature) {
        self.def_name(&s.name);
        self.w(": ");
        self.ty(&s.ty);
        self.bounds(&s.bounds);
    }

    fn def_name(&mut self, n: &DefName) {
        match n {
            DefName::Name(n) => self.w(&n.text()),
            DefName::Op(o, _) => self.w(o),
        }
    }

    fn bounds(&mut self, bs: &[Bound]) {
        if bs.is_empty() {
            return;
        }
        self.w(" where ");
        self.sep_list(bs, ", ", |p, b| {
            p.w(&b.var.name);
            p.w(": ");
            p.sep_list(&b.traits, ", ", |p, t| p.trait_ref(t));
        });
    }

    fn trait_ref(&mut self, t: &TraitRef) {
        self.w("^");
        self.w(&t.path.join("."));
    }

    fn fn_clause(&mut self, f: &FnClause) {
        self.w("fn ");
        self.def_name(&f.name);
        if !f.params.is_empty() {
            self.w(" ");
            self.sep_list(&f.params, ", ", |p, x| p.pattern(x));
        }
        match &f.body {
            FnBody::Expr(e) => {
                self.w(" = ");
                self.rhs(e);
            }
            FnBody::Block(b) => self.block_after_colon(b),
        }
    }

    fn type_decl(&mut self, t: &TypeDecl) {
        self.w("type ");
        self.w(&t.name.path.join("."));
        for prm in &t.params {
            self.w(" ");
            self.w(&prm.name);
        }
        let Some(body) = &t.body else { return };
        self.w(" = ");
        match body {
            TypeBody::Record(fs) => {
                self.w("record:");
                self.fields(fs);
            }
            TypeBody::Choice(cs) => {
                self.w("choice:");
                self.indent += 4;
                for c in cs {
                    self.line();
                    self.case(c);
                }
                self.indent -= 4;
            }
            TypeBody::Row => self.w("row"),
            TypeBody::Newtype(c) => self.case(c),
            TypeBody::Unit(i) => self.w(&i.name),
            TypeBody::Alias(t) => self.ty(t),
        }
    }

    fn case(&mut self, c: &Case) {
        self.w(&c.name.text());
        if let Some(r) = &c.indexed_result {
            self.w(": ");
            if !c.args.is_empty() {
                self.sep_list(&c.args, ", ", |p, t| p.ty(t));
                self.w(" ");
            }
            self.w("-> ");
            self.ty(r);
            return;
        }
        if !c.args.is_empty() {
            self.w(" ");
            self.sep_list(&c.args, ", ", |p, t| p.ty(t));
        }
    }

    fn fields(&mut self, fs: &[Field]) {
        self.indent += 4;
        for f in fs {
            self.line();
            self.w(&f.name.name);
            self.w(": ");
            self.ty(&f.ty);
        }
        self.indent -= 4;
    }

    fn trait_items(&mut self, items: &[TraitItem]) {
        self.indent += 4;
        for it in items {
            self.line();
            match it {
                TraitItem::Signature(s) => self.signature(s),
                TraitItem::Fn(f) => self.fn_clause(f),
                TraitItem::AssocType { name, params, value, .. } => {
                    self.w("type ");
                    self.w(&name.name);
                    for prm in params {
                        self.w(" ");
                        self.w(&prm.name);
                    }
                    if let Some(v) = value {
                        self.w(" = ");
                        self.ty(v);
                    }
                }
                TraitItem::AssocRow { name, .. } => {
                    self.w("row ");
                    self.w(&name.text());
                }
            }
        }
        self.indent -= 4;
    }

    // ----- tests --------------------------------------------------------------------------------

    fn test(&mut self, t: &TestDecl) {
        self.w("test ");
        self.w(&t.name.name);
        self.w(", ");
        self.expr(&t.blueprint);
        if let Some(s) = t.seed {
            self.w(&format!(", seed {s}"));
        }
        self.w(":");
        self.indent += 4;
        for s in &t.steps {
            self.line();
            for w in &s.withs {
                self.w("with ");
                self.entry(w);
                if !matches!(s.action, StepAction::WithOnly) {
                    self.w(", ");
                }
            }
            match &s.action {
                StepAction::Build => self.w("build"),
                StepAction::Tick => self.w("tick"),
                StepAction::Compute { until_idle } => self.w(if *until_idle { "compute until idle" } else { "compute" }),
                StepAction::Push(e) => {
                    self.w("push ");
                    self.entry(e);
                }
                StepAction::WithOnly => {}
                StepAction::Deliver(ds) => {
                    self.sep_list(ds, ", ", |p, d| {
                        p.w(match d.kind {
                            DeliveryKind::Send(_) => "send ",
                            DeliveryKind::Click => "click ",
                            DeliveryKind::Answer => "answer ",
                        });
                        match &d.target {
                            Target::Pattern(pt) => p.pattern(pt),
                            Target::Path(path) => {
                                p.w("at ");
                                p.path(path);
                            }
                            Target::Element(path) => {
                                p.w("element ");
                                p.path(path);
                            }
                        }
                        if let DeliveryKind::Send(v) = &d.kind {
                            p.w(", ");
                            p.expr(v);
                        }
                    });
                }
            }
            self.indent += 4;
            for c in &s.checks {
                self.line();
                self.w("=> ");
                self.test_check(c);
            }
            self.indent -= 4;
        }
        self.indent -= 4;
    }

    fn entry(&mut self, e: &Entry) {
        self.expr(&e.element);
        self.w(" = ");
        self.expr(&e.value);
    }

    fn path(&mut self, p: &Path) {
        self.sep_list(&p.segments, "/", |pr, s| match s {
            PathSeg::Num(n) => pr.w(&n.to_string()),
            PathSeg::Gen(n) => pr.w(&format!("#{n}")),
            PathSeg::Key(e) => {
                pr.w("[");
                pr.expr(e);
                pr.w("]");
            }
        });
    }

    fn test_check(&mut self, c: &TestCheck) {
        match &c.kind {
            TestCheckKind::Expect(p) => {
                self.w("expect ");
                self.pattern(p);
            }
            TestCheckKind::ExpectNone(p) => {
                self.w("expect_none ");
                self.pattern(p);
            }
            TestCheckKind::ExpectCount(p, n) => {
                self.w("expect_count ");
                self.pattern(p);
                self.w(&format!(", {n}"));
            }
            TestCheckKind::ExpectTop(p) => {
                self.w("expect_top ");
                self.pattern(p);
            }
            TestCheckKind::ExpectTopNone(p) => {
                self.w("expect_top_none ");
                self.pattern(p);
            }
            TestCheckKind::Requested(p, v) => {
                self.w("requested ");
                self.pattern(p);
                self.w(", ");
                self.pattern(v);
            }
            TestCheckKind::Unrequested(p) => {
                self.w("unrequested ");
                self.pattern(p);
            }
            TestCheckKind::Returns(p) => {
                self.w("returns ");
                self.pattern(p);
            }
            TestCheckKind::Fired(p) => {
                self.w("fired ");
                self.pattern(p);
            }
            TestCheckKind::Quiet => self.w("quiet"),
            TestCheckKind::Settles => self.w("settles"),
            TestCheckKind::Aborted => self.w("aborted"),
            TestCheckKind::Snapshot(s) => {
                self.w("snapshot ");
                self.w(&text_literal_plain(s));
            }
        }
    }

    // ----- blocks and statements ----------------------------------------------------------------

    fn block_after_colon(&mut self, b: &Block) {
        self.w(":");
        self.block(b);
    }

    fn block(&mut self, b: &Block) {
        if b.inline {
            self.w(" ");
            self.sep_list(&b.stmts, "; ", |p, s| p.stmt(s));
        } else {
            self.indent += 4;
            for s in &b.stmts {
                self.line();
                self.stmt(s);
            }
            self.indent -= 4;
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Emit(r) => {
                self.w("emit ");
                self.row(r);
            }
            StmtKind::Bind { pattern, value, then } => {
                self.pattern(pattern);
                self.w(" <- ");
                self.rhs(value);
                if let Some(t) = then {
                    self.w(" then ");
                    self.rhs(t);
                }
            }
            StmtKind::Ignore(e) => {
                self.w("ignore <- ");
                self.rhs(e);
            }
            StmtKind::Let { pattern, ty, value } => {
                self.w("let ");
                self.pattern(pattern);
                if let Some(t) = ty {
                    self.w(": ");
                    self.ty(t);
                }
                self.w(" = ");
                self.rhs(value);
            }
            StmtKind::Signature(sg) => self.signature(sg),
            StmtKind::Fn(f) => self.fn_clause(f),
            StmtKind::Widget(w) => {
                self.w(&w.name.text());
                self.block_after_colon(&w.body);
            }
            StmtKind::Set { target, value } => {
                self.expr(target);
                self.w(" <= ");
                self.rhs(value);
            }
            StmtKind::Accum { target, op, value } => {
                self.expr(target);
                self.w(" ");
                self.w(op.as_str());
                self.w(" ");
                self.rhs(value);
            }
            StmtKind::Require(c, e) | StmtKind::Forbid(c, e) => {
                self.w(if matches!(s.kind, StmtKind::Require(..)) { "require " } else { "forbid " });
                self.expr(c);
                if let Some(e) = e {
                    self.w(", ");
                    self.expr(e);
                }
            }
            StmtKind::While { cond, body } => {
                self.w("while ");
                self.cond(cond);
                self.block_after_colon(body);
            }
            StmtKind::Return(e) => {
                self.w("return ");
                self.rhs(e);
            }
            StmtKind::Expr(e) => self.rhs(e),
        }
    }

    fn cond(&mut self, c: &Cond) {
        match c {
            Cond::Expr(e) => self.expr(e),
            Cond::Bind { pattern, value, guard } => {
                self.pattern(pattern);
                self.w(" <- ");
                self.expr(value);
                if let Some(g) = guard {
                    self.w(" if ");
                    self.expr(g);
                }
            }
        }
    }

    /// An expression on the right of `=`, `<-` or `then`, where a `.Method` may take a
    /// continuation line.
    fn rhs(&mut self, e: &Expr) {
        self.expr(e);
    }

    // ----- expressions --------------------------------------------------------------------------

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(n) => self.w(&n.text()),
            ExprKind::Lit(l) => self.literal(l),
            ExprKind::Unit(s) => {
                self.w(&sigils_str(s));
                self.w("unit");
            }
            ExprKind::Nil(s) => {
                self.w(&sigils_str(s));
                self.w("nil");
            }
            ExprKind::Bound(b) => self.w(match b {
                BoundName::Key => "key",
                BoundName::Ix => "ix",
                BoundName::Jx => "jx",
                BoundName::Kx => "kx",
            }),
            ExprKind::Tuple(es) => {
                self.w("((");
                self.sep_list(es, ", ", |p, x| p.expr(x));
                self.w("))");
            }
            ExprKind::Array(es) => {
                self.w("[[");
                self.sep_list(es, ", ", |p, x| p.expr(x));
                self.w("]]");
            }
            ExprKind::Map(entries) => {
                self.w("{");
                self.sep_list(entries, ", ", |p, (k, v)| {
                    p.expr(k);
                    p.w(": ");
                    p.expr(v);
                });
                self.w("}");
            }
            ExprKind::Group(inner) => {
                let s = sub_expr(inner, self.indent);
                self.w("(");
                if s.starts_with('(') || s.starts_with('[') {
                    self.w(" ");
                }
                self.expr(inner);
                self.w(")");
            }
            ExprKind::Call { callee, args, parens, rest } => {
                self.expr(callee);
                if *parens {
                    self.w("(");
                    self.args(args);
                    if *rest {
                        if !args.is_empty() {
                            self.w(", ");
                        }
                        self.w("..");
                    }
                    self.w(")");
                } else {
                    self.w(" ");
                    self.args(args);
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.expr(lhs);
                self.w(" ");
                self.w(op.as_str());
                self.w(" ");
                self.expr(rhs);
            }
            ExprKind::Suffixed { op, ty, lhs, rhs } => {
                self.expr(lhs);
                self.w(" ");
                self.w(op.as_str());
                self.w(&ty.path.join("."));
                self.w(" ");
                self.expr(rhs);
            }
            ExprKind::Unary { op, operand } => {
                match op {
                    UnOp::Not => self.w("not "),
                    UnOp::Neg => self.w("-"),
                    UnOp::Pos => self.w("+"),
                }
                self.expr(operand);
            }
            ExprKind::FreeSigil { sigil, operand } => {
                self.w(&sigil.as_char().to_string());
                self.w(" ");
                self.expr(operand);
            }
            ExprKind::Field { recv, name } => {
                self.expr(recv);
                self.w(".");
                self.w(&name.name);
            }
            ExprKind::Index { recv, index } => {
                self.expr(recv);
                self.w("[");
                self.expr(index);
                self.w("]");
            }
            ExprKind::TopicField(n) => {
                self.w(".");
                self.w(&n.name);
            }
            ExprKind::TopicIndex(i) => {
                self.w("[");
                self.expr(i);
                self.w("]");
            }
            ExprKind::Method { recv, method, args } => {
                self.expr(recv);
                if postfix_safe(recv) {
                    self.w(".");
                } else {
                    // Rule 9: a `.Method` beginning a continuation line.
                    self.indent += 4;
                    self.line();
                    self.indent -= 4;
                    self.w(".");
                }
                self.w(&method.text());
                if !args.is_empty() {
                    self.w(" ");
                    self.args(args);
                }
            }
            ExprKind::TypeArgs { target, types } => {
                self.expr(target);
                self.w("<<");
                self.sep_list(types, ", ", |p, t| p.ty(t));
                self.w(">>");
            }
            ExprKind::Cast { expr, ty, checked } => {
                self.expr(expr);
                self.w(" as ");
                self.ty(ty);
                if *checked {
                    self.w("?");
                }
            }
            ExprKind::NonEmpty(inner) => {
                self.expr(inner);
                self.w("+");
            }
            ExprKind::Absent => self.w("-"),
            ExprKind::Rest => self.w(".."),
            ExprKind::Lambda { params, body } => {
                self.w("\\");
                if let Some(ps) = params {
                    self.w("|");
                    self.sep_list(ps, ", ", |p, x| p.pattern(x));
                    self.w("|");
                    if matches!(**body, LambdaBody::Expr(_)) {
                        self.w(" ");
                    }
                }
                match &**body {
                    LambdaBody::Expr(e) => self.expr(e),
                    LambdaBody::Block(b) => self.block(b),
                }
            }
            ExprKind::If { cond, then, else_ } => {
                self.w("if ");
                self.cond(cond);
                self.block_after_colon(then);
                if let Some(eb) = else_ {
                    if then.inline {
                        self.w(" ");
                    } else {
                        // Rule 8: `else` at the column of its `if`'s statement.
                        self.line();
                    }
                    self.w("else");
                    self.block_after_colon(eb);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.w("match ");
                self.expr(scrutinee);
                self.w(":");
                self.indent += 4;
                for a in arms {
                    self.line();
                    match &a.pattern {
                        None => self.w("otherwise"),
                        Some(p) => self.pattern(p),
                    }
                    if let Some(g) = &a.guard {
                        self.w(" if ");
                        self.expr(g);
                    }
                    self.block_after_colon(&a.body);
                }
                self.indent -= 4;
            }
            ExprKind::Rebuild { scrutinee, body } => {
                self.w("rebuild ");
                self.expr(scrutinee);
                self.block_after_colon(body);
            }
            ExprKind::For { binders, source, body } => {
                self.w("for ");
                if let Some(b) = binders {
                    self.pattern(&b.first);
                    if let Some(s) = &b.second {
                        self.w(", ");
                        self.pattern(s);
                    }
                    self.w(" <- ");
                }
                self.expr(source);
                self.block_after_colon(body);
            }
            ExprKind::Given(b) => {
                self.w("given");
                self.block_after_colon(b);
            }
            ExprKind::Update { target, body } => {
                self.w("update ");
                self.expr(target);
                self.block_after_colon(body);
            }
            ExprKind::WidgetBlock(b) => {
                self.w("@widget");
                self.block_after_colon(b);
            }
            ExprKind::Monad { monad, body } => {
                self.expr(monad);
                self.w(" monad");
                self.block_after_colon(body);
            }
            ExprKind::Template(t) => self.template(t),
            ExprKind::Any(e) => {
                self.w("any ");
                self.expr(e);
            }
            ExprKind::Lens(path) => {
                self.w("&.");
                self.sep_list(path, ".", |p, f| p.w(&f.name));
            }
            ExprKind::Error => self.w("<error>"),
        }
    }

    fn args(&mut self, args: &[Arg]) {
        self.sep_list(args, ", ", |p, a| {
            if let Some(n) = &a.name {
                p.w(&n.name);
                p.w("=");
            }
            p.expr(&a.value);
        });
    }

    fn literal(&mut self, l: &Literal) {
        self.w(&sigils_str(&l.marks));
        match &l.kind {
            LitKind::Int(s) | LitKind::Float(s) => self.w(s),
            LitKind::Duration { digits, unit } => {
                self.w(digits);
                self.w(unit);
            }
            LitKind::Char(c) => {
                self.w("'");
                self.w(&escape_char(*c, '\''));
                self.w("'");
            }
            LitKind::Text(segs) => {
                self.w("\"");
                for s in segs {
                    match s {
                        TextSeg::Plain(t) => {
                            for c in t.chars() {
                                self.w(&escape_char(c, '"'));
                            }
                        }
                        TextSeg::Placeholder(e) => {
                            self.w("{");
                            self.expr(e);
                            self.w("}");
                        }
                    }
                }
                self.w("\"");
            }
        }
    }

    // ----- templates ------------------------------------------------------------------------------

    fn template(&mut self, t: &Template) {
        match t.keyword {
            TemplateKw::Output => self.w("output "),
            TemplateKw::InlineText => self.w("inline_text "),
            TemplateKw::Raw => self.w("raw "),
            TemplateKw::File => {}
        }
        self.w(&format!("<{}>", t.lang));
        let saved = self.indent;
        self.indent = 0;
        self.template_parts(&t.parts, t.delims, t.keyword == TemplateKw::Raw);
        self.indent = saved;
        self.w(&format!("</{}>", t.lang));
    }

    /// Prints parts right after an opening tag (or a call's `>`).
    fn template_parts(&mut self, parts: &[TemplatePart], delims: Delims, raw: bool) {
        // If the parts begin at the start of a line, the tag must end its line (whose
        // newline is then not text); otherwise the text follows the tag directly.
        let begins_line = match parts.first() {
            Some(TemplatePart::Text(s)) => s.starts_with('\n'),
            Some(TemplatePart::Placeholder { column: Some(_), .. }) | Some(TemplatePart::Block(_)) => true,
            _ => false,
        };
        if begins_line {
            self.w("\n");
        }
        let (open, close) = if delims == Delims::Double { ("{{", "}}") } else { ("{", "}") };
        for p in parts {
            match p {
                TemplatePart::Text(s) => {
                    if raw {
                        self.w(s);
                    } else if delims == Delims::Double {
                        self.w(&s.replace("{{", "\\{{"));
                    } else {
                        self.w(&s.replace('{', "\\{"));
                    }
                }
                TemplatePart::Placeholder { expr, column } => {
                    if let Some(c) = column {
                        if !self.out.ends_with('\n') {
                            self.w("\n");
                        }
                        self.w(&" ".repeat(*c as usize));
                        self.w(open);
                        self.expr(expr);
                        self.w(close);
                        self.w("\n");
                    } else {
                        self.w(open);
                        self.expr(expr);
                        self.w(close);
                    }
                }
                TemplatePart::Block(b) => {
                    if !self.out.ends_with('\n') {
                        self.w("\n");
                    }
                    self.w(open);
                    for s in &b.stmts {
                        self.line();
                        self.stmt(s);
                    }
                    self.w("\n");
                    self.w(close);
                    self.w("\n");
                }
                TemplatePart::Call { name, attrs, body, .. } => {
                    self.w("<");
                    self.w(&name.text());
                    for a in attrs {
                        self.w(" ");
                        if let Some(n) = &a.name {
                            self.w(&n.name);
                        }
                        self.w("=");
                        self.w(open);
                        self.expr(&a.value);
                        self.w(close);
                    }
                    match body {
                        None => self.w("/>"),
                        Some(b) => {
                            self.w(">");
                            self.template_parts(b, delims, false);
                            self.w(&format!("</{}>", name.text()));
                        }
                    }
                }
            }
        }
    }

    // ----- patterns -----------------------------------------------------------------------------

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Name(n) => self.w(&n.text()),
            PatternKind::Ignore => self.w("ignore"),
            PatternKind::Lit(l) => self.literal(l),
            PatternKind::Neg(l) => {
                self.w("-");
                self.literal(l);
            }
            PatternKind::Unit => self.w("unit"),
            PatternKind::Nil => self.w("nil"),
            PatternKind::Ctor { name, args } => {
                self.w(&name.text());
                self.w(" ");
                self.sep_list(args, ", ", |p, a| p.pattern(a));
            }
            PatternKind::Record { name, fields, rest } => {
                self.w(&name.text());
                self.w("(");
                self.sep_list(fields, ", ", |p, f| {
                    if let Some(n) = &f.name {
                        p.w(&n.name);
                        p.w("=");
                    }
                    p.pattern(&f.pattern);
                });
                if *rest {
                    if !fields.is_empty() {
                        self.w(", ");
                    }
                    self.w("..");
                }
                self.w(")");
            }
            PatternKind::Sigil { sigil, inner } => {
                self.w(&sigil.as_char().to_string());
                self.w(" ");
                self.pattern(inner);
            }
            PatternKind::Tuple(ps) => {
                self.w("((");
                self.sep_list(ps, ", ", |p, x| p.pattern(x));
                self.w("))");
            }
            PatternKind::Array(ps) => {
                self.w("[[");
                self.sep_list(ps, ", ", |p, x| p.pattern(x));
                self.w("]]");
            }
            PatternKind::Group(inner) => {
                let s = pattern(inner);
                self.w("(");
                if s.starts_with('(') || s.starts_with('[') {
                    self.w(" ");
                }
                self.pattern(inner);
                self.w(")");
            }
            PatternKind::Family(q) => {
                self.w(&q.path.join("."));
                self.w(".*");
            }
        }
    }

    // ----- types --------------------------------------------------------------------------------

    fn ty(&mut self, t: &Type) {
        match &t.kind {
            TypeKind::Name { name, args } => {
                self.w(&name.text());
                for a in args {
                    self.w(" ");
                    self.ty(a);
                }
            }
            TypeKind::Sigiled { sigils, inner } => {
                self.w(&sigils_str(sigils));
                let s = ty(inner);
                if !(s.starts_with('(') || s.starts_with('[')) {
                    self.w(" ");
                }
                self.ty(inner);
            }
            TypeKind::Array { elem, doubled } => {
                self.w(if *doubled { "[[" } else { "[" });
                if !*doubled && ty(elem).starts_with('[') {
                    self.w(" ");
                }
                self.ty(elem);
                self.w(if *doubled { "]]" } else { "]" });
            }
            TypeKind::Tuple(ts) => {
                self.w("((");
                self.sep_list(ts, ", ", |p, x| p.ty(x));
                self.w("))");
            }
            TypeKind::Group(inner) => {
                let s = ty(inner);
                self.w("(");
                if s.starts_with('(') || s.starts_with('[') {
                    self.w(" ");
                }
                self.ty(inner);
                self.w(")");
            }
            TypeKind::Map { key, value } => {
                self.ty(key);
                self.w(" ==> ");
                self.ty(value);
            }
            TypeKind::Fn { color, params, result } => {
                match color {
                    Color::Bounded => {}
                    Color::Unbounded => self.w("unbounded "),
                    Color::Var(v) => {
                        self.w(&v.name);
                        self.w(" ");
                    }
                }
                self.w("fn ");
                self.sep_list(params, ", ", |p, x| p.ty(x));
                if !params.is_empty() {
                    self.w(" ");
                }
                self.w("-> ");
                self.ty(result);
            }
            TypeKind::Emit { ty, row } => {
                self.ty(ty);
                self.w(" emit ");
                self.ty(row);
            }
            TypeKind::RowUnion(ts) => self.sep_list(ts, " | ", |p, x| p.ty(x)),
            TypeKind::Family(q) => {
                self.w(&q.path.join("."));
                self.w(".*");
            }
            TypeKind::Any(tr) => {
                self.w("any ");
                self.trait_ref(tr);
            }
            TypeKind::Unit(s) => {
                self.w(&sigils_str(s));
                self.w("unit");
            }
            TypeKind::Hole(n) => self.w(&n.text()),
            TypeKind::Absent => self.w("-"),
            TypeKind::NonEmpty(inner) => {
                self.ty(inner);
                self.w("+");
            }
        }
    }
}

/// Whether a `.Method` can touch this receiver on its line: the receiver is a postfix
/// operand that did not take the rest of its expression, and is not an uppercase name
/// (which would lex with the method as one qualified name).
fn postfix_safe(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Name(n) => !n.is_upper(),
        ExprKind::Lit(_) | ExprKind::Group(_) | ExprKind::Tuple(_) | ExprKind::Array(_) | ExprKind::Map(_) | ExprKind::Field { .. } | ExprKind::Index { .. } | ExprKind::TypeArgs { .. } | ExprKind::TopicField(_) | ExprKind::TopicIndex(_) | ExprKind::Unit(_) | ExprKind::Nil(_) | ExprKind::Bound(_) => true,
        ExprKind::Call { parens, .. } => *parens,
        ExprKind::Method { args, recv, .. } => args.is_empty() && postfix_safe(recv),
        _ => false,
    }
}

fn sub_expr(e: &Expr, _indent: usize) -> String {
    expr(e)
}

fn escape_char(c: char, quote: char) -> String {
    match c {
        '\\' => "\\\\".into(),
        '\n' => "\\n".into(),
        '\t' => "\\t".into(),
        '{' => "\\{".into(),
        '}' => "\\}".into(),
        c if c == quote => format!("\\{c}"),
        c if c.is_control() => format!("\\u{{{:x}}}", c as u32),
        c => c.to_string(),
    }
}

fn text_literal_plain(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        out.push_str(&escape_char(c, '"'));
    }
    out.push('"');
    out
}
