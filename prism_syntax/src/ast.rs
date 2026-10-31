//! The abstract syntax tree of Prism.
//!
//! The tree follows Semantics Appendix A.6 form by form, with Templates chapter 2's
//! templates and Conformance Appendix A's tests. Every node has a [`Span`]. Sigils stay
//! where the program wrote them: a name keeps its attached stack ([`SName`]), a literal
//! its marks, a type its stack, and a free sigil is its own node, because "Sigils are
//! never implied" (Semantics ch. 3, Principles) and the checker reads them (D12).
//!
//! Equality ignores spans (see [`crate::span`]), so a printed tree that parses back gives
//! an equal tree.

use crate::span::Span;
pub use crate::token::{BracketKind, Delims, Sigil, TemplateKw};

// ----- names ---------------------------------------------------------------------------

/// A name as written: attached sigils, outermost first, then a possibly qualified name.
/// `@$?draft`, `Ui.Button`, `@Birds.Spotter`, `$_`, `&fuel`.
#[derive(Clone, Debug, PartialEq)]
pub struct SName {
    pub sigils: Vec<Sigil>,
    pub path: Vec<String>,
    pub span: Span,
}

impl SName {
    /// The last segment.
    pub fn base(&self) -> &str {
        self.path.last().map(|s| s.as_str()).unwrap_or("")
    }
    pub fn is_upper(&self) -> bool {
        self.base().starts_with(|c: char| c.is_ascii_uppercase())
    }
    pub fn is_topic(&self) -> bool {
        self.path.len() == 1 && self.path[0] == "_"
    }
    pub fn is_reserved_ctor(&self) -> bool {
        crate::lexer::is_reserved_ctor(self.base())
    }
    /// Applied later: an uppercase name or a reserved lowercase constructor.
    pub fn is_callee(&self) -> bool {
        self.is_upper() || self.is_reserved_ctor()
    }
    pub fn text(&self) -> String {
        format!("{}{}", crate::token::sigils_str(&self.sigils), self.path.join("."))
    }
}

/// A plain lowercase identifier: a field, a named argument, a type variable.
#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// A trait reference, `^Ord` (the `^` mark attached to an uppercase name).
#[derive(Clone, Debug, PartialEq)]
pub struct TraitRef {
    pub path: Vec<String>,
    pub span: Span,
}

// ----- the file -------------------------------------------------------------------------

/// A file, which is one module (Semantics ch. 6, "A file is a module").
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    /// `module Birds`; absent in a root file.
    pub name: Option<QualName>,
    /// The module's `emit` row; absent in a root file that keeps the implied `emit Element`.
    pub emit: Option<Row>,
    pub uses: Vec<Use>,
    pub decls: Vec<Decl>,
    /// A file whose first token is a language tag is a template (Templates ch. 1).
    pub template: Option<Template>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualName {
    pub path: Vec<String>,
    pub span: Span,
}

/// `use Birds` or `use Birds.*`.
#[derive(Clone, Debug, PartialEq)]
pub struct Use {
    pub module: QualName,
    pub star: bool,
    pub span: Span,
}

/// A row: families joined by `|` (Semantics ch. 3, `emit`; Type System ch. 2).
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub terms: Vec<RowTerm>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RowTerm {
    /// `Ui.*`: a family's tags of the row kind in force.
    Family(QualName),
    /// `Element`, `Cards.Element`, `Lang t`: a named row or a POD type's row.
    Type(Type),
    /// `r`: a row variable.
    Var(Ident),
}

// ----- declarations -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Decl {
    pub kind: DeclKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclKind {
    /// `Name: type`, a signature line above its definition.
    Signature(Signature),
    /// `fn Name params = expr` or `fn Name params:` and a block: one clause.
    Fn(FnClause),
    /// `let pattern = expr`.
    Let(LetDecl),
    /// `@counter:` and a block: a parameterless widget, a value with no keyword.
    Widget(WidgetDecl),
    Type(TypeDecl),
    Tag(TagDecl),
    /// `derive ^Eq, ^Show`: free impls for the declaration above.
    Derive(Vec<TraitRef>),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Check(CheckDecl),
    Test(TestDecl),
    /// A statement where a declaration was expected; only [`crate::parse_fragment`]
    /// produces these, for the books' snippets.
    Stmt(Stmt),
}

/// The name a signature or definition is for: a name with its sigils, or, in a trait, an
/// operator (`+: fn a, a -> a`, Type System ch. 3).
#[derive(Clone, Debug, PartialEq)]
pub enum DefName {
    Name(SName),
    Op(String, Span),
}

impl DefName {
    pub fn span(&self) -> Span {
        match self {
            DefName::Name(n) => n.span,
            DefName::Op(_, s) => *s,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub name: DefName,
    pub ty: Type,
    pub bounds: Vec<Bound>,
    pub span: Span,
}

/// `where a: ^Ord`: a type variable and the traits it needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Bound {
    pub var: Ident,
    pub traits: Vec<TraitRef>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnClause {
    pub name: DefName,
    pub params: Vec<Pattern>,
    pub body: FnBody,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FnBody {
    /// `= expr`
    Expr(Expr),
    /// `:` and a block; its kind follows the mark on the name (Semantics ch. 3).
    Block(Block),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LetDecl {
    pub pattern: Pattern,
    /// `let byte: U8 = 255` (Semantics ch. 2, Numbers).
    pub ty: Option<Type>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetDecl {
    pub name: SName,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    /// `Element` or, in the prelude's style, `Cards.Element`.
    pub name: QualName,
    pub params: Vec<Ident>,
    /// Absent for an opaque type, `type Name`.
    pub body: Option<TypeBody>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeBody {
    /// `record:` and its fields.
    Record(Vec<Field>),
    /// `choice:` and its cases.
    Choice(Vec<Case>),
    /// `row`: a row kind (`type Element o = row`).
    Row,
    /// `type Meters = Meters Float`: a choice of one alternative on one line
    /// (Type System ch. 2): the right side's head is the declaration's own name.
    Newtype(Case),
    /// `type Click = click`: a unit type, whose one value is lowercase.
    Unit(Ident),
    /// Anything else on the right: an alias.
    Alias(Type),
}

/// `name: Type` in a record or a tag.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}

/// A choice's alternative: `Leaf Nat`, `Node Tree, Tree`, `Dot`, `templatetext Text`.
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub name: SName,
    pub args: Vec<Type>,
    /// `Lit: Int -> Expr Int`: an indexed alternative (a GADT), out of this compiler.
    pub indexed_result: Option<Type>,
    pub span: Span,
}

/// `tag Ui.Button of Element !Click:` and its fields.
#[derive(Clone, Debug, PartialEq)]
pub struct TagDecl {
    pub name: QualName,
    pub of: Type,
    pub bounds: Vec<Bound>,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraitDecl {
    pub name: TraitRef,
    pub params: Vec<Ident>,
    pub bounds: Vec<Bound>,
    pub items: Vec<TraitItem>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TraitItem {
    Signature(Signature),
    /// A default method.
    Fn(FnClause),
    /// `type Quotient` in a trait, `type Quotient = ?Int` in an impl: an associated type.
    AssocType { name: Ident, params: Vec<Ident>, value: Option<Type>, span: Span },
    /// `row Lang` in a trait: an associated row (`^Pod`, Type System ch. 2).
    AssocRow { name: SName, span: Span },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImplDecl {
    pub trait_: TraitRef,
    pub ty: Type,
    pub bounds: Vec<Bound>,
    pub items: Vec<TraitItem>,
    pub span: Span,
}

// ----- tests (Conformance ch. 1 and Appendix A) ---------------------------------------------

/// `check name:` and lines of `expression => pattern`.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckDecl {
    pub name: Ident,
    pub lines: Vec<CheckLine>,
    pub span: Span,
}

/// `expr => pattern`, or a bare expression, which is short for `=> True` (Conformance
/// Appendix A, doubt CK).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckLine {
    pub expr: Expr,
    pub pattern: Option<Pattern>,
    pub span: Span,
}

/// `test name, @blueprint:` and steps; `, seed 7` names a seed.
#[derive(Clone, Debug, PartialEq)]
pub struct TestDecl {
    pub name: Ident,
    pub blueprint: Expr,
    pub seed: Option<u64>,
    pub steps: Vec<Step>,
    pub span: Span,
}

/// One step line, which is one tick, and the checks under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// `with entry = v` before the step's action.
    pub withs: Vec<Entry>,
    pub action: StepAction,
    pub checks: Vec<TestCheck>,
    pub span: Span,
}

/// `entry = value`, for `with` and `push`.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub element: Expr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StepAction {
    Build,
    Tick,
    /// `compute`, and `compute until idle` (Conformance ch. 1, Steps; D6).
    Compute { until_idle: bool },
    Push(Entry),
    /// One or more deliveries, simultaneous, in no order.
    Deliver(Vec<Delivery>),
    /// A line of `with` entries alone.
    WithOnly,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Delivery {
    pub kind: DeliveryKind,
    pub target: Target,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeliveryKind {
    /// `send p, v`
    Send(Expr),
    /// `click p`, which is `send p, click`
    Click,
    /// `answer p`
    Answer,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Pattern(Pattern),
    /// `at 1/#0/0`
    Path(Path),
    /// `click element 0/[e2]/0`: the element at a path of a canvas program's layout (D9).
    Element(Path),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSeg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathSeg {
    /// A position.
    Num(u64),
    /// `#g`, a generation.
    Gen(u64),
    /// `[k]`, a key.
    Key(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TestCheck {
    pub kind: TestCheckKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TestCheckKind {
    Expect(Pattern),
    ExpectNone(Pattern),
    ExpectCount(Pattern, u64),
    ExpectTop(Pattern),
    ExpectTopNone(Pattern),
    Requested(Pattern, Pattern),
    Unrequested(Pattern),
    Returns(Pattern),
    Fired(Pattern),
    Quiet,
    Settles,
    Aborted,
    Snapshot(String),
}

// ----- types ---------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    /// A named type and its arguments by juxtaposition: `Nat`, `Element !Click`,
    /// `Expr Int`, `f a`. Sigils attached to the head name (`$Elem c`) stay on it, and
    /// read outside the application ("application by juxtaposition" is tighter than
    /// "sigil stacks", Appendix A.5).
    Name { name: SName, args: Vec<Type> },
    /// Free sigils before a type: `$(Path ==> Elem c)`, `@[a]`, `!(K ==> T)`.
    Sigiled { sigils: Vec<Sigil>, inner: Box<Type> },
    /// `[a]` (or `[[a]]`, which the books also write): an array.
    Array { elem: Box<Type>, doubled: bool },
    /// `((a, b))`, and `((a))` the 1-tuple.
    Tuple(Vec<Type>),
    /// `(t)`: a group.
    Group(Box<Type>),
    /// `K ==> V`: a map.
    Map { key: Box<Type>, value: Box<Type> },
    /// `fn A, B -> R`, with its color: `unbounded fn`, or a color variable `u fn`
    /// (Semantics ch. 7, doubts UB and CV).
    Fn { color: Color, params: Vec<Type>, result: Box<Type> },
    /// `T emit R`: a blueprint type's row.
    Emit { ty: Box<Type>, row: Box<Type> },
    /// `a | b`: row terms joined.
    RowUnion(Vec<Type>),
    /// `Ui.*` in a type position (a row term, or an explicit row argument).
    Family(QualName),
    /// `any ^Show`: an existential.
    Any(TraitRef),
    /// `unit`, with its attached sigils (`@unit`, `!unit`).
    Unit(Vec<Sigil>),
    /// `_` and `_1` in an `impl` header, with sigils: `!_`.
    Hole(SName),
    /// `-` in a sparse tuple type (out of this compiler).
    Absent,
    /// `[a]+`: the non-empty mark (out of this compiler).
    NonEmpty(Box<Type>),
}

/// A function type's color (Semantics ch. 7, "Compute ticks"; D6, D8).
#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    /// No word: bounded.
    Bounded,
    /// `unbounded fn`.
    Unbounded,
    /// `u fn`: a color variable, resolved at each call.
    Var(Ident),
}

// ----- patterns ------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternKind {
    /// A bare name: a binder (`x`, `$count`, `!e`, `K`) or a constructor with no
    /// arguments (`Up`, `Dot`). Which is the checker's to decide.
    Name(SName),
    /// `ignore`: matches anything, binds nothing.
    Ignore,
    Lit(Literal),
    /// `-40` in a pattern is a literal (ch. 2, Numbers).
    Neg(Literal),
    Unit,
    Nil,
    /// A constructor applied by commas: `Node l, r`, `Letter s`, `Lit n`.
    Ctor { name: SName, args: Vec<Pattern> },
    /// A constructor with named fields in touching parentheses:
    /// `Error(kind=Fs.NotFound, ..)`, `Ui.Button(label="OK", ..)`.
    Record { name: SName, fields: Vec<FieldPat>, rest: bool },
    /// `? p`, `~ p`: the free forms in a pattern take one apart.
    Sigil { sigil: Sigil, inner: Box<Pattern> },
    Tuple(Vec<Pattern>),
    Array(Vec<Pattern>),
    Group(Box<Pattern>),
    /// `Ui.*`: every tag of a family.
    Family(QualName),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldPat {
    /// Absent for a positional argument inside the parentheses.
    pub name: Option<Ident>,
    pub pattern: Pattern,
    pub span: Span,
}

// ----- literals ------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Literal {
    /// Marks on the literal: `$"Go"`, `$0`, `!"Got {!line}"` (ch. 2, "Marks on literals").
    pub marks: Vec<Sigil>,
    pub kind: LitKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LitKind {
    /// Kept as written, `1_000_000`, `0xFF`; inference picks the type.
    Int(String),
    /// Decimal-spelled: a `Float` (out of this compiler).
    Float(String),
    /// `250ms`: digits and a unit.
    Duration { digits: String, unit: String },
    Char(char),
    Text(Vec<TextSeg>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextSeg {
    Plain(String),
    Placeholder(Expr),
}

// ----- expressions --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// Binary operators, by the precedence table of Semantics ch. 2 / Appendix A.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    /// `<|`, level 4.
    Merge,
    /// `on`, level 5.
    On,
    /// `or`, level 6.
    Or,
    /// `and`, level 7.
    And,
    /// Level 9.
    Eq, NotEq, Lt, Le, Gt, Ge,
    /// Level 10.
    Add, Sub, AddQ, SubQ,
    /// Level 11.
    Mul, Div, MulQ, Mod,
}

impl BinOp {
    /// The level in the precedence table, loosest 4 to tightest 11.
    pub fn level(self) -> u8 {
        use BinOp::*;
        match self {
            Merge => 4,
            On => 5,
            Or => 6,
            And => 7,
            Eq | NotEq | Lt | Le | Gt | Ge => 9,
            Add | Sub | AddQ | SubQ => 10,
            Mul | Div | MulQ | Mod => 11,
        }
    }

    pub fn as_str(self) -> &'static str {
        use BinOp::*;
        match self {
            Merge => "<|", On => "on", Or => "or", And => "and", Eq => "==", NotEq => "!=",
            Lt => "<", Le => "<=", Gt => ">", Ge => ">=", Add => "+", Sub => "-",
            AddQ => "+?", SubQ => "-?", Mul => "*", Div => "/", MulQ => "*?", Mod => "mod",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    /// `not`, level 8.
    Not,
    /// Prefix `-`, level 13.
    Neg,
    /// Prefix `+`, level 13.
    Pos,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Name(SName),
    Lit(Literal),
    Unit(Vec<Sigil>),
    Nil(Vec<Sigil>),
    /// `key`, `ix`, `jx`, `kx`: the names the language binds.
    Bound(BoundName),
    /// `((a, b))`, `((x))`.
    Tuple(Vec<Expr>),
    /// `[[a, b]]`, `[[]]`.
    Array(Vec<Expr>),
    /// `{k: v}`, `{}`.
    Map(Vec<(Expr, Expr)>),
    /// `(e)`: kept, so a printed program keeps its groups.
    Group(Box<Expr>),
    /// A call. `parens` is true for `F(a, b)` with the parenthesis touching the callee,
    /// false for the paren-less `F a, b`.
    Call { callee: Box<Expr>, args: Vec<Arg>, parens: bool, rest: bool },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Unary { op: UnOp, operand: Box<Expr> },
    /// An operator at a type, `a >Int b` (out of this compiler).
    Suffixed { op: BinOp, ty: QualName, lhs: Box<Expr>, rhs: Box<Expr> },
    /// `@ v`, `$ v`, `? v`, `~ v`: a free sigil over the rest of its expression.
    FreeSigil { sigil: Sigil, operand: Box<Expr> },
    /// `a.f`
    Field { recv: Box<Expr>, name: Ident },
    /// `xs[i]`
    Index { recv: Box<Expr>, index: Box<Expr> },
    /// `.f` with no receiver: `_.f` (the spacing rule's prefix `.`).
    TopicField(Ident),
    /// `[i]` with no receiver: `_[i]`.
    TopicIndex(Box<Expr>),
    /// `recv.Method args`: the method's first argument is the receiver (layout rule 9).
    Method { recv: Box<Expr>, method: SName, args: Vec<Arg> },
    /// `Name<<T, U>>`: explicit type arguments.
    TypeArgs { target: Box<Expr>, types: Vec<Type> },
    /// `e as T`; `checked` for `as T?` (and `as [T]+?`).
    Cast { expr: Box<Expr>, ty: Type, checked: bool },
    /// `[[x]]+`, the non-empty mark on a literal (out of this compiler).
    NonEmpty(Box<Expr>),
    /// `-` as a sparse tuple's absent component (out of this compiler).
    Absent,
    /// `..` among a call's arguments.
    Rest,
    /// `\|a, b| body` or `\body` on the topic.
    Lambda { params: Option<Vec<Pattern>>, body: Box<LambdaBody> },
    /// `if c: a else: b`, or `if p <- e:` in an `update` block.
    If { cond: Box<Cond>, then: Block, else_: Option<Block> },
    Match { scrutinee: Box<Expr>, arms: Vec<Arm> },
    /// `rebuild $x:` and its block.
    Rebuild { scrutinee: Box<Expr>, body: Block },
    /// `for k, v <- e:`, `for $m:`, `for &s <- &_.xs:`.
    For { binders: Option<ForBinders>, source: Box<Expr>, body: Block },
    /// `given:` and its steps.
    Given(Block),
    /// `update v:` and its statements.
    Update { target: Box<Expr>, body: Block },
    /// `@widget:` and its block.
    WidgetBlock(Block),
    /// `T monad:` ended by `return` (out of this compiler).
    Monad { monad: Box<Expr>, body: Block },
    Template(Box<Template>),
    /// `any v`: an existential's construction.
    Any(Box<Expr>),
    /// `&.score.home`: a lens value.
    Lens(Vec<Ident>),
    /// A node the parser put where an error was.
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundName {
    Key,
    Ix,
    Jx,
    Kx,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    /// `label=` in `Ui.Button(label=$"Go")`.
    pub name: Option<Ident>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LambdaBody {
    Expr(Expr),
    /// A closing bar last on its line opens a block (layout rule 3): `let`s, then the body.
    Block(Block),
}

/// An `if` or `while` condition.
#[derive(Clone, Debug, PartialEq)]
pub enum Cond {
    Expr(Expr),
    /// `pattern <- e`, with an optional guard: `while ? p <- &parent[&r] if p != &r:`.
    Bind { pattern: Pattern, value: Expr, guard: Option<Expr> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForBinders {
    pub first: Pattern,
    /// The value binder of a map loop, `for id, q <- questions`.
    pub second: Option<Pattern>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    /// Absent for `otherwise:`.
    pub pattern: Option<Pattern>,
    pub guard: Option<Expr>,
    pub body: Block,
    pub span: Span,
}

// ----- blocks and statements ---------------------------------------------------------------

/// A block: an indented one after a `:` that ends its line, or an inline one, one
/// statement or several separated by `;` (layout rules 2 and 5).
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub inline: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

/// The accumulating statements of an `update` block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccumOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl AccumOp {
    pub fn as_str(self) -> &'static str {
        match self {
            AccumOp::Add => "+=",
            AccumOp::Sub => "-=",
            AccumOp::Mul => "*=",
            AccumOp::Div => "/=",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    /// `emit row`, first in a block.
    Emit(Row),
    /// `pattern <- e`: emit, build, or bind through `?`/`~` in `given:`; with `then`,
    /// a hold.
    Bind { pattern: Pattern, value: Expr, then: Option<Expr> },
    /// `ignore <- e`: the result is discarded, out loud.
    Ignore(Expr),
    /// `let pattern = e`; `let &x = e` makes a local in an `update` block;
    /// `let x: T = e` annotates.
    Let { pattern: Pattern, ty: Option<Type>, value: Expr },
    Signature(Signature),
    Fn(FnClause),
    /// `@name:` and a block, inside a block.
    Widget(WidgetDecl),
    /// `&r <= e` in an `update` block.
    Set { target: Expr, value: Expr },
    /// `&r += e` and its kin.
    Accum { target: Expr, op: AccumOp, value: Expr },
    /// `require c`, and `require c, e`: Semantics ch. 10's doubt GV2 gives a guard in a
    /// `~` block its `Error` as a second argument; decisions.md D11 refuses the second
    /// argument for `Prelude.Unmet`. The tree holds either; the checker rules.
    Require(Expr, Option<Expr>),
    /// `forbid c`, and `forbid c, e` (as `Require`).
    Forbid(Expr, Option<Expr>),
    /// `while c:` or `while p <- e if g:` in an `update` block.
    While { cond: Cond, body: Block },
    /// `return v`, ending a monad block (out of this compiler).
    Return(Expr),
    /// An expression: a bare element or blueprint, the last line, `@ v`, a building
    /// `match`, `for` or `if`.
    Expr(Expr),
}

// ----- templates (Templates ch. 2) ----------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub keyword: TemplateKw,
    pub lang: String,
    pub delims: Delims,
    pub parts: Vec<TemplatePart>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TemplatePart {
    /// A run: static text, the whitespace rules already applied.
    Text(String),
    /// `{{expr}}`; `column` when alone on its line, its expansion indented there.
    Placeholder { expr: Expr, column: Option<u32> },
    /// A brace block's statements.
    Block(Block),
    /// `<@Name attr={{v}}/>` or `<@Name ...>body</@Name>`.
    Call { name: SName, attrs: Vec<Arg>, body: Option<Vec<TemplatePart>>, span: Span },
}
