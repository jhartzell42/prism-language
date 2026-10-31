//! A compact S-expression dump of the tree, without spans: what the snapshot tests check
//! in, and what `cargo run -p prism_syntax --example dump FILE` prints.
//!
//! Every node prints as `(kind child ...)`; names print with their sigils as written,
//! literals as their source spelling. Blocks print their statements one per line,
//! indented, so a dump reads like the program's outline.

use crate::ast::*;
use crate::token::sigils_str;

pub fn module(m: &Module) -> String {
    let mut d = Dump::default();
    d.module(m);
    d.out
}

pub fn expr(e: &Expr) -> String {
    let mut d = Dump::default();
    d.expr(e);
    d.out
}

#[derive(Default)]
struct Dump {
    out: String,
    depth: usize,
}

impl Dump {
    fn w(&mut self, s: &str) {
        self.out.push_str(s);
    }
    fn nl(&mut self) {
        self.out.push('\n');
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }
    fn open(&mut self, tag: &str) {
        self.w("(");
        self.w(tag);
    }
    fn close(&mut self) {
        self.w(")");
    }
    fn sp(&mut self) {
        self.w(" ");
    }

    fn module(&mut self, m: &Module) {
        self.open("module");
        self.depth += 1;
        if let Some(n) = &m.name {
            self.nl();
            self.w(&format!("(name {})", n.path.join(".")));
        }
        if let Some(r) = &m.emit {
            self.nl();
            self.open("emit");
            self.sp();
            self.row(r);
            self.close();
        }
        for u in &m.uses {
            self.nl();
            self.w(&format!("(use {}{})", u.module.path.join("."), if u.star { ".*" } else { "" }));
        }
        if let Some(t) = &m.template {
            self.nl();
            self.template(t);
        }
        for d in &m.decls {
            self.nl();
            self.decl(d);
        }
        self.depth -= 1;
        self.close();
        self.w("\n");
    }

    fn row(&mut self, r: &Row) {
        self.open("row");
        for t in &r.terms {
            self.sp();
            match t {
                RowTerm::Family(q) => self.w(&format!("{}.*", q.path.join("."))),
                RowTerm::Var(v) => self.w(&format!("(var {})", v.name)),
                RowTerm::Type(t) => self.ty(t),
            }
        }
        self.close();
    }

    fn decl(&mut self, d: &Decl) {
        match &d.kind {
            DeclKind::Signature(s) => self.signature(s),
            DeclKind::Fn(f) => self.fn_clause(f),
            DeclKind::Let(l) => {
                self.open("let");
                self.sp();
                self.pattern(&l.pattern);
                if let Some(t) = &l.ty {
                    self.sp();
                    self.ty(t);
                }
                self.sp();
                self.expr(&l.value);
                self.close();
            }
            DeclKind::Widget(w) => {
                self.open("widget");
                self.w(&format!(" {}", w.name.text()));
                self.block(&w.body);
                self.close();
            }
            DeclKind::Type(t) => {
                self.open("type");
                self.w(&format!(" {}", t.name.path.join(".")));
                for p in &t.params {
                    self.w(&format!(" {}", p.name));
                }
                match &t.body {
                    None => self.w(" opaque"),
                    Some(TypeBody::Record(fs)) => {
                        self.w(" record");
                        self.fields(fs);
                    }
                    Some(TypeBody::Choice(cs)) => {
                        self.w(" choice");
                        self.depth += 1;
                        for c in cs {
                            self.nl();
                            self.case(c);
                        }
                        self.depth -= 1;
                    }
                    Some(TypeBody::Row) => self.w(" row"),
                    Some(TypeBody::Newtype(c)) => {
                        self.w(" newtype ");
                        self.case(c);
                    }
                    Some(TypeBody::Unit(i)) => self.w(&format!(" unit {}", i.name)),
                    Some(TypeBody::Alias(t)) => {
                        self.w(" alias ");
                        self.ty(t);
                    }
                }
                self.close();
            }
            DeclKind::Tag(t) => {
                self.open("tag");
                self.w(&format!(" {} ", t.name.path.join(".")));
                self.ty(&t.of);
                self.bounds(&t.bounds);
                self.fields(&t.fields);
                self.close();
            }
            DeclKind::Derive(ts) => {
                self.open("derive");
                for t in ts {
                    self.w(&format!(" ^{}", t.path.join(".")));
                }
                self.close();
            }
            DeclKind::Trait(t) => {
                self.open("trait");
                self.w(&format!(" ^{}", t.name.path.join(".")));
                for p in &t.params {
                    self.w(&format!(" {}", p.name));
                }
                self.bounds(&t.bounds);
                self.items(&t.items);
                self.close();
            }
            DeclKind::Impl(i) => {
                self.open("impl");
                self.w(&format!(" ^{} ", i.trait_.path.join(".")));
                self.ty(&i.ty);
                self.bounds(&i.bounds);
                self.items(&i.items);
                self.close();
            }
            DeclKind::Check(c) => {
                self.open("check");
                self.w(&format!(" {}", c.name.name));
                self.depth += 1;
                for l in &c.lines {
                    self.nl();
                    self.open("line");
                    self.sp();
                    self.expr(&l.expr);
                    if let Some(p) = &l.pattern {
                        self.w(" => ");
                        self.pattern(p);
                    }
                    self.close();
                }
                self.depth -= 1;
                self.close();
            }
            DeclKind::Test(t) => self.test(t),
            DeclKind::Stmt(s) => self.stmt(s),
        }
    }

    fn signature(&mut self, s: &Signature) {
        self.open("sig");
        self.sp();
        self.def_name(&s.name);
        self.sp();
        self.ty(&s.ty);
        self.bounds(&s.bounds);
        self.close();
    }

    fn def_name(&mut self, n: &DefName) {
        match n {
            DefName::Name(n) => self.w(&n.text()),
            DefName::Op(o, _) => self.w(&format!("(op {o})")),
        }
    }

    fn bounds(&mut self, bs: &[Bound]) {
        for b in bs {
            self.w(&format!(" (where {}", b.var.name));
            for t in &b.traits {
                self.w(&format!(" ^{}", t.path.join(".")));
            }
            self.w(")");
        }
    }

    fn fn_clause(&mut self, f: &FnClause) {
        self.open("fn");
        self.sp();
        self.def_name(&f.name);
        self.w(" (params");
        for p in &f.params {
            self.sp();
            self.pattern(p);
        }
        self.w(")");
        match &f.body {
            FnBody::Expr(e) => {
                self.depth += 1;
                self.nl();
                self.expr(e);
                self.depth -= 1;
            }
            FnBody::Block(b) => self.block(b),
        }
        self.close();
    }

    fn fields(&mut self, fs: &[Field]) {
        self.depth += 1;
        for f in fs {
            self.nl();
            self.w(&format!("(field {} ", f.name.name));
            self.ty(&f.ty);
            self.w(")");
        }
        self.depth -= 1;
    }

    fn case(&mut self, c: &Case) {
        self.open("case");
        self.w(&format!(" {}", c.name.text()));
        for a in &c.args {
            self.sp();
            self.ty(a);
        }
        if let Some(r) = &c.indexed_result {
            self.w(" -> ");
            self.ty(r);
        }
        self.close();
    }

    fn items(&mut self, items: &[TraitItem]) {
        self.depth += 1;
        for it in items {
            self.nl();
            match it {
                TraitItem::Signature(s) => self.signature(s),
                TraitItem::Fn(f) => self.fn_clause(f),
                TraitItem::AssocType { name, params, value, .. } => {
                    self.w(&format!("(assoc-type {}", name.name));
                    for p in params {
                        self.w(&format!(" {}", p.name));
                    }
                    if let Some(v) = value {
                        self.sp();
                        self.ty(v);
                    }
                    self.w(")");
                }
                TraitItem::AssocRow { name, .. } => self.w(&format!("(assoc-row {})", name.text())),
            }
        }
        self.depth -= 1;
    }

    fn test(&mut self, t: &TestDecl) {
        self.open("test");
        self.w(&format!(" {} ", t.name.name));
        self.expr(&t.blueprint);
        if let Some(s) = t.seed {
            self.w(&format!(" (seed {s})"));
        }
        self.depth += 1;
        for s in &t.steps {
            self.nl();
            self.open("step");
            for w in &s.withs {
                self.w(" (with ");
                self.expr(&w.element);
                self.sp();
                self.expr(&w.value);
                self.w(")");
            }
            self.sp();
            match &s.action {
                StepAction::Build => self.w("build"),
                StepAction::Tick => self.w("tick"),
                StepAction::Compute { until_idle } => self.w(if *until_idle { "compute-until-idle" } else { "compute" }),
                StepAction::WithOnly => self.w("with-only"),
                StepAction::Push(e) => {
                    self.w("(push ");
                    self.expr(&e.element);
                    self.sp();
                    self.expr(&e.value);
                    self.w(")");
                }
                StepAction::Deliver(ds) => {
                    for (i, d) in ds.iter().enumerate() {
                        if i > 0 {
                            self.sp();
                        }
                        let k = match d.kind {
                            DeliveryKind::Send(_) => "send",
                            DeliveryKind::Click => "click",
                            DeliveryKind::Answer => "answer",
                        };
                        self.open(k);
                        self.sp();
                        match &d.target {
                            Target::Pattern(p) => self.pattern(p),
                            Target::Path(p) => self.path("at", p),
                            Target::Element(p) => self.path("element", p),
                        }
                        if let DeliveryKind::Send(v) = &d.kind {
                            self.sp();
                            self.expr(v);
                        }
                        self.close();
                    }
                }
            }
            self.depth += 1;
            for c in &s.checks {
                self.nl();
                self.check(c);
            }
            self.depth -= 1;
            self.close();
        }
        self.depth -= 1;
        self.close();
    }

    fn path(&mut self, word: &str, p: &Path) {
        self.w(&format!("({word}"));
        for s in &p.segments {
            match s {
                PathSeg::Num(n) => self.w(&format!(" {n}")),
                PathSeg::Gen(n) => self.w(&format!(" #{n}")),
                PathSeg::Key(e) => {
                    self.w(" [");
                    self.expr(e);
                    self.w("]");
                }
            }
        }
        self.w(")");
    }

    fn check(&mut self, c: &TestCheck) {
        let (word, pats, extra): (&str, Vec<&Pattern>, Option<String>) = match &c.kind {
            TestCheckKind::Expect(p) => ("expect", vec![p], None),
            TestCheckKind::ExpectNone(p) => ("expect_none", vec![p], None),
            TestCheckKind::ExpectCount(p, n) => ("expect_count", vec![p], Some(n.to_string())),
            TestCheckKind::ExpectTop(p) => ("expect_top", vec![p], None),
            TestCheckKind::ExpectTopNone(p) => ("expect_top_none", vec![p], None),
            TestCheckKind::Requested(p, v) => ("requested", vec![p, v], None),
            TestCheckKind::Unrequested(p) => ("unrequested", vec![p], None),
            TestCheckKind::Returns(p) => ("returns", vec![p], None),
            TestCheckKind::Fired(p) => ("fired", vec![p], None),
            TestCheckKind::Quiet => ("quiet", vec![], None),
            TestCheckKind::Settles => ("settles", vec![], None),
            TestCheckKind::Aborted => ("aborted", vec![], None),
            TestCheckKind::Snapshot(s) => ("snapshot", vec![], Some(format!("{s:?}"))),
        };
        self.open(&format!("=> {word}"));
        for p in pats {
            self.sp();
            self.pattern(p);
        }
        if let Some(e) = extra {
            self.sp();
            self.w(&e);
        }
        self.close();
    }

    fn block(&mut self, b: &Block) {
        if b.inline {
            self.w(" (inline");
            for s in &b.stmts {
                self.sp();
                self.stmt(s);
            }
            self.w(")");
            return;
        }
        self.depth += 1;
        for s in &b.stmts {
            self.nl();
            self.stmt(s);
        }
        self.depth -= 1;
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Emit(r) => {
                self.open("emit");
                self.sp();
                self.row(r);
                self.close();
            }
            StmtKind::Bind { pattern, value, then } => {
                self.open(if then.is_some() { "hold" } else { "bind" });
                self.sp();
                self.pattern(pattern);
                self.sp();
                self.expr(value);
                if let Some(t) = then {
                    self.w(" then ");
                    self.expr(t);
                }
                self.close();
            }
            StmtKind::Ignore(e) => {
                self.open("ignore");
                self.sp();
                self.expr(e);
                self.close();
            }
            StmtKind::Let { pattern, ty, value } => {
                self.open("let");
                self.sp();
                self.pattern(pattern);
                if let Some(t) = ty {
                    self.sp();
                    self.ty(t);
                }
                self.sp();
                self.expr(value);
                self.close();
            }
            StmtKind::Signature(sg) => self.signature(sg),
            StmtKind::Fn(f) => self.fn_clause(f),
            StmtKind::Widget(w) => {
                self.open("widget");
                self.w(&format!(" {}", w.name.text()));
                self.block(&w.body);
                self.close();
            }
            StmtKind::Set { target, value } => {
                self.open("set");
                self.sp();
                self.expr(target);
                self.sp();
                self.expr(value);
                self.close();
            }
            StmtKind::Accum { target, op, value } => {
                self.open(op.as_str());
                self.sp();
                self.expr(target);
                self.sp();
                self.expr(value);
                self.close();
            }
            StmtKind::Require(c, e) | StmtKind::Forbid(c, e) => {
                self.open(if matches!(s.kind, StmtKind::Require(..)) { "require" } else { "forbid" });
                self.sp();
                self.expr(c);
                if let Some(e) = e {
                    self.sp();
                    self.expr(e);
                }
                self.close();
            }
            StmtKind::While { cond, body } => {
                self.open("while");
                self.sp();
                self.cond(cond);
                self.block(body);
                self.close();
            }
            StmtKind::Return(e) => {
                self.open("return");
                self.sp();
                self.expr(e);
                self.close();
            }
            StmtKind::Expr(e) => self.expr(e),
        }
    }

    fn cond(&mut self, c: &Cond) {
        match c {
            Cond::Expr(e) => self.expr(e),
            Cond::Bind { pattern, value, guard } => {
                self.open("<-");
                self.sp();
                self.pattern(pattern);
                self.sp();
                self.expr(value);
                if let Some(g) = guard {
                    self.w(" (if ");
                    self.expr(g);
                    self.w(")");
                }
                self.close();
            }
        }
    }

    fn literal(&mut self, l: &Literal) {
        let m = sigils_str(&l.marks);
        match &l.kind {
            LitKind::Int(s) | LitKind::Float(s) => self.w(&format!("{m}{s}")),
            LitKind::Duration { digits, unit } => self.w(&format!("{m}{digits}{unit}")),
            LitKind::Char(c) => self.w(&format!("{m}{c:?}")),
            LitKind::Text(segs) => {
                if segs.iter().all(|s| matches!(s, TextSeg::Plain(_))) {
                    let s: String = segs.iter().map(|s| if let TextSeg::Plain(p) = s { p.as_str() } else { "" }).collect();
                    self.w(&format!("{m}{s:?}"));
                } else {
                    self.w(&format!("({m}text"));
                    for s in segs {
                        self.sp();
                        match s {
                            TextSeg::Plain(p) => self.w(&format!("{p:?}")),
                            TextSeg::Placeholder(e) => {
                                self.w("{");
                                self.expr(e);
                                self.w("}");
                            }
                        }
                    }
                    self.w(")");
                }
            }
        }
    }

    fn args(&mut self, args: &[Arg]) {
        for a in args {
            self.sp();
            if let Some(n) = &a.name {
                self.w(&format!("{}=", n.name));
            }
            self.expr(&a.value);
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(n) => self.w(&n.text()),
            ExprKind::Lit(l) => self.literal(l),
            ExprKind::Unit(s) => self.w(&format!("{}unit", sigils_str(s))),
            ExprKind::Nil(s) => self.w(&format!("{}nil", sigils_str(s))),
            ExprKind::Bound(b) => self.w(&format!("{b:?}").to_lowercase()),
            ExprKind::Tuple(es) => {
                self.open("tuple");
                for x in es {
                    self.sp();
                    self.expr(x);
                }
                self.close();
            }
            ExprKind::Array(es) => {
                self.open("array");
                for x in es {
                    self.sp();
                    self.expr(x);
                }
                self.close();
            }
            ExprKind::Map(es) => {
                self.open("map");
                for (k, v) in es {
                    self.w(" (");
                    self.expr(k);
                    self.w(": ");
                    self.expr(v);
                    self.w(")");
                }
                self.close();
            }
            ExprKind::Group(x) => {
                self.open("group");
                self.sp();
                self.expr(x);
                self.close();
            }
            ExprKind::Call { callee, args, parens, rest } => {
                self.open(if *parens { "call()" } else { "call" });
                self.sp();
                self.expr(callee);
                self.args(args);
                if *rest {
                    self.w(" ..");
                }
                self.close();
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.open(op.as_str());
                self.sp();
                self.expr(lhs);
                self.sp();
                self.expr(rhs);
                self.close();
            }
            ExprKind::Suffixed { op, ty, lhs, rhs } => {
                self.open(&format!("{}{}", op.as_str(), ty.path.join(".")));
                self.sp();
                self.expr(lhs);
                self.sp();
                self.expr(rhs);
                self.close();
            }
            ExprKind::Unary { op, operand } => {
                self.open(match op {
                    UnOp::Not => "not",
                    UnOp::Neg => "neg",
                    UnOp::Pos => "pos",
                });
                self.sp();
                self.expr(operand);
                self.close();
            }
            ExprKind::FreeSigil { sigil, operand } => {
                self.open(&format!("free{}", sigil.as_char()));
                self.sp();
                self.expr(operand);
                self.close();
            }
            ExprKind::Field { recv, name } => {
                self.open(".");
                self.sp();
                self.expr(recv);
                self.w(&format!(" {}", name.name));
                self.close();
            }
            ExprKind::Index { recv, index } => {
                self.open("index");
                self.sp();
                self.expr(recv);
                self.sp();
                self.expr(index);
                self.close();
            }
            ExprKind::TopicField(n) => self.w(&format!("(. _ {})", n.name)),
            ExprKind::TopicIndex(i) => {
                self.w("(index _ ");
                self.expr(i);
                self.w(")");
            }
            ExprKind::Method { recv, method, args } => {
                self.open(&format!(".{}", method.text()));
                self.sp();
                self.expr(recv);
                self.args(args);
                self.close();
            }
            ExprKind::TypeArgs { target, types } => {
                self.open("<<>>");
                self.sp();
                self.expr(target);
                for t in types {
                    self.sp();
                    self.ty(t);
                }
                self.close();
            }
            ExprKind::Cast { expr, ty, checked } => {
                self.open(if *checked { "as?" } else { "as" });
                self.sp();
                self.expr(expr);
                self.sp();
                self.ty(ty);
                self.close();
            }
            ExprKind::NonEmpty(x) => {
                self.open("nonempty");
                self.sp();
                self.expr(x);
                self.close();
            }
            ExprKind::Absent => self.w("absent"),
            ExprKind::Rest => self.w(".."),
            ExprKind::Lambda { params, body } => {
                self.open("lambda");
                if let Some(ps) = params {
                    self.w(" (params");
                    for p in ps {
                        self.sp();
                        self.pattern(p);
                    }
                    self.w(")");
                } else {
                    self.w(" topic");
                }
                match &**body {
                    LambdaBody::Expr(e) => {
                        self.sp();
                        self.expr(e);
                    }
                    LambdaBody::Block(b) => self.block(b),
                }
                self.close();
            }
            ExprKind::If { cond, then, else_ } => {
                self.open("if");
                self.sp();
                self.cond(cond);
                self.block(then);
                if let Some(e) = else_ {
                    if !e.inline {
                        self.nl();
                    }
                    self.w(" else");
                    self.block(e);
                }
                self.close();
            }
            ExprKind::Match { scrutinee, arms } => {
                self.open("match");
                self.sp();
                self.expr(scrutinee);
                self.depth += 1;
                for a in arms {
                    self.nl();
                    self.open("arm");
                    self.sp();
                    match &a.pattern {
                        Some(p) => self.pattern(p),
                        None => self.w("otherwise"),
                    }
                    if let Some(g) = &a.guard {
                        self.w(" (if ");
                        self.expr(g);
                        self.w(")");
                    }
                    self.block(&a.body);
                    self.close();
                }
                self.depth -= 1;
                self.close();
            }
            ExprKind::Rebuild { scrutinee, body } => {
                self.open("rebuild");
                self.sp();
                self.expr(scrutinee);
                self.block(body);
                self.close();
            }
            ExprKind::For { binders, source, body } => {
                self.open("for");
                if let Some(b) = binders {
                    self.sp();
                    self.pattern(&b.first);
                    if let Some(s) = &b.second {
                        self.w(", ");
                        self.pattern(s);
                    }
                    self.w(" <-");
                }
                self.sp();
                self.expr(source);
                self.block(body);
                self.close();
            }
            ExprKind::Given(b) => {
                self.open("given");
                self.block(b);
                self.close();
            }
            ExprKind::Update { target, body } => {
                self.open("update");
                self.sp();
                self.expr(target);
                self.block(body);
                self.close();
            }
            ExprKind::WidgetBlock(b) => {
                self.open("@widget");
                self.block(b);
                self.close();
            }
            ExprKind::Monad { monad, body } => {
                self.open("monad");
                self.sp();
                self.expr(monad);
                self.block(body);
                self.close();
            }
            ExprKind::Template(t) => self.template(t),
            ExprKind::Any(x) => {
                self.open("any");
                self.sp();
                self.expr(x);
                self.close();
            }
            ExprKind::Lens(p) => {
                let s: Vec<&str> = p.iter().map(|i| i.name.as_str()).collect();
                self.w(&format!("(lens {})", s.join(".")));
            }
            ExprKind::Error => self.w("<error>"),
        }
    }

    fn template(&mut self, t: &Template) {
        self.w(&format!("(template {:?} <{}>", t.keyword, t.lang));
        self.depth += 1;
        self.parts(&t.parts);
        self.depth -= 1;
        self.w(")");
    }

    fn parts(&mut self, parts: &[TemplatePart]) {
        for p in parts {
            self.nl();
            match p {
                TemplatePart::Text(s) => self.w(&format!("{s:?}")),
                TemplatePart::Placeholder { expr, column } => {
                    self.w("{{");
                    self.expr(expr);
                    self.w("}}");
                    if let Some(c) = column {
                        self.w(&format!(" @col {c}"));
                    }
                }
                TemplatePart::Block(b) => {
                    self.w("(brace-block");
                    self.block(b);
                    self.w(")");
                }
                TemplatePart::Call { name, attrs, body, .. } => {
                    self.w(&format!("(call {}", name.text()));
                    self.args(attrs);
                    if let Some(b) = body {
                        self.depth += 1;
                        self.parts(b);
                        self.depth -= 1;
                    }
                    self.w(")");
                }
            }
        }
    }

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
                self.w(&format!("(ctor {}", name.text()));
                for a in args {
                    self.sp();
                    self.pattern(a);
                }
                self.w(")");
            }
            PatternKind::Record { name, fields, rest } => {
                self.w(&format!("(record {}", name.text()));
                for f in fields {
                    self.sp();
                    if let Some(n) = &f.name {
                        self.w(&format!("{}=", n.name));
                    }
                    self.pattern(&f.pattern);
                }
                if *rest {
                    self.w(" ..");
                }
                self.w(")");
            }
            PatternKind::Sigil { sigil, inner } => {
                self.w(&format!("(free{} ", sigil.as_char()));
                self.pattern(inner);
                self.w(")");
            }
            PatternKind::Tuple(ps) => {
                self.w("(tuple");
                for x in ps {
                    self.sp();
                    self.pattern(x);
                }
                self.w(")");
            }
            PatternKind::Array(ps) => {
                self.w("(array");
                for x in ps {
                    self.sp();
                    self.pattern(x);
                }
                self.w(")");
            }
            PatternKind::Group(x) => {
                self.w("(group ");
                self.pattern(x);
                self.w(")");
            }
            PatternKind::Family(q) => self.w(&format!("{}.*", q.path.join("."))),
        }
    }

    fn ty(&mut self, t: &Type) {
        match &t.kind {
            TypeKind::Name { name, args } => {
                if args.is_empty() {
                    self.w(&name.text());
                } else {
                    self.w(&format!("({}", name.text()));
                    for a in args {
                        self.sp();
                        self.ty(a);
                    }
                    self.w(")");
                }
            }
            TypeKind::Sigiled { sigils, inner } => {
                self.w(&format!("(free{} ", sigils_str(sigils)));
                self.ty(inner);
                self.w(")");
            }
            TypeKind::Array { elem, doubled } => {
                self.w(if *doubled { "[[" } else { "[" });
                self.ty(elem);
                self.w(if *doubled { "]]" } else { "]" });
            }
            TypeKind::Tuple(ts) => {
                self.w("(tuple");
                for x in ts {
                    self.sp();
                    self.ty(x);
                }
                self.w(")");
            }
            TypeKind::Group(x) => {
                self.w("(group ");
                self.ty(x);
                self.w(")");
            }
            TypeKind::Map { key, value } => {
                self.w("(==> ");
                self.ty(key);
                self.sp();
                self.ty(value);
                self.w(")");
            }
            TypeKind::Fn { color, params, result } => {
                let c = match color {
                    Color::Bounded => String::new(),
                    Color::Unbounded => "unbounded ".into(),
                    Color::Var(v) => format!("{} ", v.name),
                };
                self.w(&format!("({c}fn"));
                for p in params {
                    self.sp();
                    self.ty(p);
                }
                self.w(" -> ");
                self.ty(result);
                self.w(")");
            }
            TypeKind::Emit { ty, row } => {
                self.w("(emit ");
                self.ty(ty);
                self.sp();
                self.ty(row);
                self.w(")");
            }
            TypeKind::RowUnion(ts) => {
                self.w("(|");
                for x in ts {
                    self.sp();
                    self.ty(x);
                }
                self.w(")");
            }
            TypeKind::Family(q) => self.w(&format!("{}.*", q.path.join("."))),
            TypeKind::Any(tr) => self.w(&format!("(any ^{})", tr.path.join("."))),
            TypeKind::Unit(s) => self.w(&format!("{}unit", sigils_str(s))),
            TypeKind::Hole(n) => self.w(&n.text()),
            TypeKind::Absent => self.w("absent"),
            TypeKind::NonEmpty(x) => {
                self.w("(nonempty ");
                self.ty(x);
                self.w(")");
            }
        }
    }
}
