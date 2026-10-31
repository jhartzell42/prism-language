//! Module loading: a program is a set of files, one module each (Semantics ch. 6, "A file
//! is a module", "The root, and what a set of files means").
//!
//! What [`load`] does, in order:
//!
//! 1. Parse every file with `prism_syntax`. A lexical or parse error stops the load; an
//!    out-of-scope diagnostic (decisions.md, Features in and out) is kept, and the
//!    construct stops with that diagnostic only if evaluation reaches it.
//! 2. Add the modules the implementation supplies: the prelude (`prelude/prelude.prism`,
//!    Appendix B) and the bundled libraries whose families the Worlds answer (`Ui`, `Fs`,
//!    `Process`, `Time`, `Test`, `Gui`). A file among the program's with the same module
//!    name replaces a bundled library.
//! 3. Name every module by its `module` line. Two files naming one module, a library
//!    whose module is the prelude's, or (for `run`) anything but exactly one root file is
//!    an error naming the files.
//! 4. Gather each module's declarations into its table, by the name with its sigils
//!    (`@Spotter`, `?Score`, `Call`): sigils are part of names (Type System ch. 1).
//! 5. Resolve `use` lines. A `use` of a module not among the files is an error at the
//!    `use`. `use M.*` brings every bare name of `M` in except tags; the prelude is used by
//!    every file with its `*` and without a line. **Two bare names that collide are an
//!    error at the use, both candidates named, and a file's own declarations do not
//!    shadow what a `use M.*` brought in either** (ch. 6).
//! 6. Register tags (`tag F.T of K o:`), constructors (records, choices, newtypes, unit
//!    types), traits and impls in the program-wide tables. Every tag of a family is
//!    declared in one module: a family declared by two modules is an error.
//!
//! The AST of every module is leaked to `'static`: compiler zero runs once per process
//! and holds its program for the whole run, and every code object can then point into the
//! tree without reference counting (see `interp`).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use prism_rt::{FieldKind, OutputKind, Row};
use prism_syntax::ast::*;
use prism_syntax::{LineIndex, Span, Stage};

/// An index into [`Loaded::modules`].
pub type ModId = usize;

/// The modules the implementation supplies, beside the program's files: the prelude, and
/// the libraries of the families the Worlds on this branch answer.
pub const BUNDLED: &[(&str, &str)] = &[
    ("prelude/prelude.prism", include_str!("../../prelude/prelude.prism")),
    ("prelude/ui.prism", include_str!("../../prelude/ui.prism")),
    ("prelude/fs.prism", include_str!("../../prelude/fs.prism")),
    ("prelude/process.prism", include_str!("../../prelude/process.prism")),
    ("prelude/time.prism", include_str!("../../prelude/time.prism")),
    ("prelude/test.prism", include_str!("../../prelude/test.prism")),
    ("prelude/gui.prism", include_str!("../../prelude/gui.prism")),
];

/// A program-wide declaration a name resolves to.
#[derive(Clone)]
pub enum Global {
    /// A function: its signature (required at the top, Semantics ch. 6) and its clauses.
    Fn { sig: Option<&'static Signature>, clauses: Vec<&'static FnClause> },
    /// `let pattern = expr`: the name is one of the pattern's binders.
    Let(&'static LetDecl),
    /// `@counter:` and its block: a parameterless widget, a value.
    Widget { sig: Option<&'static Signature>, decl: &'static WidgetDecl },
    /// A record's or a choice's constructor.
    Ctor(Rc<CtorInfo>),
    /// `type Click = click`: the unit type's one value.
    Unit(Rc<str>),
    /// A signature line with no clauses, in the prelude: a Rust builtin.
    Builtin { sig: &'static Signature },
    /// A trait method: dispatched on the runtime type of one argument.
    Method(Rc<MethodInfo>),
}

/// A constructor (Type System ch. 2, Records and Choices).
pub struct CtorInfo {
    pub name: Rc<str>,
    pub type_name: Rc<str>,
    pub record: bool,
    /// Field names (a choice's payload is numbered) and their declared types.
    pub fields: Vec<(Rc<str>, &'static Type)>,
    /// The alternative's place in its choice, for `^Ord` by declaration order.
    pub index: usize,
    /// The type's parameters: a field whose type is one of them takes its argument whole
    /// (Type System ch. 4, "Where the table looks").
    pub params: Vec<String>,
    pub module: ModId,
}

/// A trait method's declaration.
pub struct MethodInfo {
    pub trait_name: Rc<str>,
    pub name: Rc<str>,
    pub sig: Option<&'static Signature>,
    /// The default body, if the trait gives one.
    pub default: Vec<&'static FnClause>,
    /// Which argument's runtime type picks the impl: the first whose declared type is
    /// the trait's parameter (or an array or optional of it).
    pub dispatch: usize,
    pub module: ModId,
}

/// A tag (Semantics ch. 8, "Elements and the element statement").
pub struct TagInfo {
    pub decl: Rc<prism_rt::TagDecl>,
    /// `tag Fs.NotFound of Fault`: a fault kind, whose value is the tag itself.
    pub fault: bool,
    pub module: ModId,
    pub ast: &'static TagDecl,
}

/// An `impl ^T Type:`.
pub struct ImplInfo {
    pub methods: HashMap<Rc<str>, Vec<&'static FnClause>>,
    pub module: ModId,
}

/// One module: one file.
pub struct ModInfo {
    pub name: Option<String>,
    pub file: String,
    pub src: &'static str,
    pub ast: &'static Module,
    pub bundled: bool,
    /// Its own declarations, by name with sigils.
    pub decls: HashMap<Rc<str>, Global>,
    /// Bare names brought in by `use M.*` and the prelude: the owning module.
    pub scope: HashMap<Rc<str>, ModId>,
    /// Modules reachable by qualified name (`use M`, `use M.*`, the prelude).
    pub qualified: HashSet<ModId>,
    /// The row of its `emit` line: what its widgets may put in the document.
    pub row: Row,
    /// Its out-of-scope diagnostics, reported if evaluation reaches the construct.
    pub out_of_scope: Vec<(Span, &'static str, String)>,
    /// A test script's expression, evaluated in the module this names: its names
    /// resolve there.
    pub alias: Option<ModId>,
}

impl ModInfo {
    pub fn is_root(&self) -> bool {
        self.name.is_none()
    }
    /// `file:line:col` for a span of this module.
    pub fn at(&self, span: Span) -> String {
        let lc = LineIndex::new(self.src).line_col(span.start);
        format!("{}:{}:{}", self.file, lc.line, lc.col)
    }
    /// The source text of a span.
    pub fn text(&self, span: Span) -> &'static str {
        let (s, e) = (span.start as usize, (span.end as usize).min(self.src.len()));
        &self.src[s.min(e)..e]
    }
}

/// A loaded program.
pub struct Loaded {
    pub modules: Vec<ModInfo>,
    pub prelude: ModId,
    /// The root file, if exactly one file has no `module` line.
    pub root: Option<ModId>,
    pub tags: HashMap<String, TagInfo>,
    /// Constructors by bare name, program-wide: what a runtime value's constructor
    /// names, for trait dispatch and `^Ord` by declaration order.
    pub ctors: HashMap<Rc<str>, Rc<CtorInfo>>,
    /// Impls by trait name (without `^`) and type head.
    pub impls: HashMap<(String, String), ImplInfo>,
    /// Trait names, for `check`.
    pub traits: HashSet<String>,
}

/// What a load hands back when it fails: every diagnostic, rendered `file:line:col:
/// message`.
pub type LoadErrors = Vec<String>;

/// Load a program from its files (`(name, text)` pairs).
pub fn load(files: &[(String, String)]) -> Result<Loaded, LoadErrors> {
    let mut errors = Vec::new();
    let mut parsed: Vec<(String, &'static str, &'static Module, bool, Vec<prism_syntax::Diagnostic>)> = Vec::new();
    let mut user_names = HashSet::new();
    let mut user_families: HashSet<String> = HashSet::new();
    for (file, text) in files {
        let src: &'static str = Box::leak(text.clone().into_boxed_str());
        let p = prism_syntax::parse_module(src);
        if let Some(n) = &p.tree.name {
            user_names.insert(n.path.join("."));
        }
        for d in &p.tree.decls {
            if let DeclKind::Tag(t) = &d.kind {
                user_families.insert(t.name.path[..t.name.path.len().saturating_sub(1)].join("."));
            }
        }
        let m: &'static Module = Box::leak(Box::new(p.tree));
        parsed.push((file.clone(), src, m, false, p.diagnostics));
    }
    for (file, text) in BUNDLED {
        let p = prism_syntax::parse_module(text);
        let name = p.tree.name.as_ref().map(|n| n.path.join(".")).unwrap_or_default();
        // The program supplies its own library of that name, or of a family it declares
        // (the chess window library declares `Ui` and `Gui`).
        let declares_theirs = p.tree.decls.iter().any(|d| match &d.kind {
            DeclKind::Tag(t) => user_families.contains(&t.name.path[..t.name.path.len().saturating_sub(1)].join(".")),
            _ => false,
        });
        if user_names.contains(&name) || declares_theirs {
            continue;
        }
        let m: &'static Module = Box::leak(Box::new(p.tree));
        parsed.push((file.to_string(), text, m, true, p.diagnostics));
    }

    let mut modules = Vec::new();
    for (file, src, ast, bundled, diags) in parsed {
        let mut out_of_scope = Vec::new();
        let index = LineIndex::new(src);
        for d in &diags {
            match d.stage {
                Stage::OutOfScope => out_of_scope.push((d.span, d.feature.unwrap_or("?"), d.message.clone())),
                _ => {
                    let lc = index.line_col(d.span.start);
                    errors.push(format!("{file}:{}:{}: {}", lc.line, lc.col, d.message));
                }
            }
        }
        modules.push(ModInfo {
            name: ast.name.as_ref().map(|n| n.path.join(".")),
            file,
            src,
            ast,
            bundled,
            decls: HashMap::new(),
            scope: HashMap::new(),
            qualified: HashSet::new(),
            row: Row::Any,
            out_of_scope,
            alias: None,
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // 3. Module names.
    let mut by_name: HashMap<String, ModId> = HashMap::new();
    for (id, m) in modules.iter().enumerate() {
        if let Some(n) = &m.name {
            if let Some(other) = by_name.get(n) {
                errors.push(format!(
                    "{}: module {n} is also declared by {}; no two files name the same module",
                    m.file, modules[*other].file
                ));
            } else {
                by_name.insert(n.clone(), id);
            }
        }
    }
    let prelude = *by_name.get("Prelude").ok_or_else(|| vec!["the prelude is missing".to_string()])?;
    let roots: Vec<ModId> = (0..modules.len()).filter(|i| modules[*i].is_root()).collect();
    let root = if roots.len() == 1 { Some(roots[0]) } else { None };

    let mut loaded = Loaded {
        modules,
        prelude,
        root,
        tags: HashMap::new(),
        ctors: HashMap::new(),
        impls: HashMap::new(),
        traits: HashSet::new(),
    };

    // 4 and 6. Declarations and the program-wide tables.
    let mut families: HashMap<String, ModId> = HashMap::new();
    for id in 0..loaded.modules.len() {
        gather(&mut loaded, id, &mut families, &mut errors);
    }

    // 5. `use` lines and bare-name scopes.
    for id in 0..loaded.modules.len() {
        resolve_uses(&mut loaded, id, &by_name, &mut errors);
    }

    // The rows of the `emit` lines.
    for id in 0..loaded.modules.len() {
        let row = loaded.modules[id].ast.emit.as_ref().map(row_of).unwrap_or(Row::Any);
        loaded.modules[id].row = row;
    }

    if errors.is_empty() { Ok(loaded) } else { Err(errors) }
}

/// The runtime row of an `emit` line: its families, or anything when the row has a
/// variable or names a whole row kind (`Element`).
pub fn row_of(row: &Row_) -> Row {
    let mut fams = BTreeSet::new();
    for t in &row.terms {
        match t {
            RowTerm::Family(q) => {
                fams.insert(q.path.join("."));
            }
            RowTerm::Var(_) => return Row::Any,
            RowTerm::Type(ty) => match &ty.kind {
                // `Cards.Element`: the family's element tags.
                TypeKind::Name { name, .. } if name.path.len() > 1 && name.base() == "Element" => {
                    fams.insert(name.path[..name.path.len() - 1].join("."));
                }
                _ => return Row::Any,
            },
        }
    }
    Row::Families(fams.into_iter().collect())
}

/// `prism_syntax`'s row type, renamed so it does not collide with `prism_rt::Row`.
pub type Row_ = prism_syntax::ast::Row;

fn decl_name(n: &DefName) -> Rc<str> {
    match n {
        DefName::Name(s) => Rc::from(s.text()),
        DefName::Op(o, _) => Rc::from(o.as_str()),
    }
}

/// Gather one module's declarations.
fn gather(l: &mut Loaded, id: ModId, families: &mut HashMap<String, ModId>, errors: &mut Vec<String>) {
    let m = &l.modules[id];
    let ast = m.ast;
    let mut sigs: HashMap<Rc<str>, &'static Signature> = HashMap::new();
    let mut clauses: Vec<(Rc<str>, &'static FnClause)> = Vec::new();
    let mut decls: HashMap<Rc<str>, Global> = HashMap::new();
    let dup = |decls: &HashMap<Rc<str>, Global>, name: &Rc<str>, span: Span, errors: &mut Vec<String>| {
        if decls.contains_key(name) {
            errors.push(format!("{}: {name} is declared twice in this module", l.modules[id].at(span)));
            true
        } else {
            false
        }
    };
    let mut last_type: Option<Rc<str>> = None;
    for d in &ast.decls {
        match &d.kind {
            DeclKind::Signature(s) => {
                sigs.insert(decl_name(&s.name), s);
            }
            DeclKind::Fn(c) => clauses.push((decl_name(&c.name), c)),
            DeclKind::Let(ld) => {
                for b in pattern_binders(&ld.pattern) {
                    let b: Rc<str> = Rc::from(b);
                    if !dup(&decls, &b, ld.span, errors) {
                        decls.insert(b, Global::Let(ld));
                    }
                }
            }
            DeclKind::Widget(w) => {
                let n: Rc<str> = Rc::from(w.name.text());
                if !dup(&decls, &n, w.span, errors) {
                    decls.insert(n, Global::Widget { sig: None, decl: w });
                }
            }
            DeclKind::Type(t) => {
                let tname: Rc<str> = Rc::from(t.name.path.join("."));
                last_type = Some(tname.clone());
                let params: Vec<String> = t.params.iter().map(|p| p.name.clone()).collect();
                let mut add_ctor = |decls: &mut HashMap<Rc<str>, Global>, info: CtorInfo, span: Span| {
                    let info = Rc::new(info);
                    let n = info.name.clone();
                    if decls.contains_key(&n) {
                        errors.push(format!("{}: {n} is declared twice in this module", l.modules[id].at(span)));
                    }
                    l.ctors.insert(n.clone(), info.clone());
                    decls.insert(n, Global::Ctor(info));
                };
                match &t.body {
                    Some(TypeBody::Record(fields)) => {
                        let fs = fields.iter().map(|f| (Rc::from(f.name.name.as_str()), &f.ty)).collect();
                        add_ctor(
                            &mut decls,
                            CtorInfo { name: tname.clone(), type_name: tname.clone(), record: true, fields: fs, index: 0, params, module: id },
                            t.span,
                        );
                    }
                    Some(TypeBody::Choice(cases)) => {
                        for (i, c) in cases.iter().enumerate() {
                            let fs = c.args.iter().enumerate().map(|(k, ty)| (Rc::from(k.to_string()), ty)).collect();
                            add_ctor(
                                &mut decls,
                                CtorInfo {
                                    name: Rc::from(c.name.text()),
                                    type_name: tname.clone(),
                                    record: false,
                                    fields: fs,
                                    index: i,
                                    params: params.clone(),
                                    module: id,
                                },
                                c.span,
                            );
                        }
                    }
                    Some(TypeBody::Newtype(c)) => {
                        let fs = c.args.iter().enumerate().map(|(k, ty)| (Rc::from(k.to_string()), ty)).collect();
                        add_ctor(
                            &mut decls,
                            CtorInfo { name: Rc::from(c.name.text()), type_name: tname.clone(), record: false, fields: fs, index: 0, params, module: id },
                            c.span,
                        );
                    }
                    Some(TypeBody::Unit(v)) => {
                        let n: Rc<str> = Rc::from(v.name.as_str());
                        // A unit value's type, for dispatch: recorded as a constructor with no fields.
                        l.ctors.insert(
                            n.clone(),
                            Rc::new(CtorInfo { name: n.clone(), type_name: tname.clone(), record: false, fields: vec![], index: 0, params: vec![], module: id }),
                        );
                        decls.insert(n.clone(), Global::Unit(n));
                    }
                    _ => {}
                }
            }
            DeclKind::Tag(t) => {
                let name = t.name.path.join(".");
                let family = t.name.path[..t.name.path.len().saturating_sub(1)].join(".");
                if let Some(own) = families.get(&family) {
                    if *own != id {
                        errors.push(format!(
                            "{}: the family {family} is declared by {}; every tag of a family is declared in one module (Semantics ch. 6)",
                            l.modules[id].at(t.span),
                            l.modules[*own].file
                        ));
                    }
                } else {
                    families.insert(family.clone(), id);
                }
                let (fault, output) = tag_kind(&t.of);
                let fields: Vec<(&str, FieldKind)> =
                    t.fields.iter().map(|f| (f.name.name.as_str(), field_kind(&f.ty))).collect();
                let decl = prism_rt::TagDecl::new(&name, &fields, output);
                l.tags.insert(name, TagInfo { decl, fault, module: id, ast: t });
            }
            DeclKind::Derive(_) => {
                // Derived impls are structural and need no table (Type System ch. 3,
                // "Derive, out loud"); the line belongs to the type above.
                let _ = &last_type;
            }
            DeclKind::Trait(t) => {
                let tname: Rc<str> = Rc::from(t.name.path.join("."));
                l.traits.insert(tname.to_string());
                let var = t.params.first().map(|p| p.name.clone()).unwrap_or_default();
                let mut defaults: HashMap<Rc<str>, Vec<&'static FnClause>> = HashMap::new();
                for item in &t.items {
                    if let TraitItem::Fn(c) = item {
                        defaults.entry(decl_name(&c.name)).or_default().push(c);
                    }
                }
                for item in &t.items {
                    if let TraitItem::Signature(s) = item {
                        let n = decl_name(&s.name);
                        let dispatch = dispatch_index(&s.ty, &var);
                        let info = MethodInfo {
                            trait_name: tname.clone(),
                            name: n.clone(),
                            sig: Some(s),
                            default: defaults.get(&n).cloned().unwrap_or_default(),
                            dispatch,
                            module: id,
                        };
                        decls.insert(n, Global::Method(Rc::new(info)));
                    }
                }
            }
            DeclKind::Impl(i) => {
                let tname = i.trait_.path.join(".");
                let head = type_head(&i.ty);
                let mut methods: HashMap<Rc<str>, Vec<&'static FnClause>> = HashMap::new();
                for item in &i.items {
                    if let TraitItem::Fn(c) = item {
                        methods.entry(decl_name(&c.name)).or_default().push(c);
                    }
                }
                let key = (tname.clone(), head.clone());
                if let Some(other) = l.impls.get(&key) {
                    errors.push(format!(
                        "{}: ^{tname} already has an impl for {head}, in {}; one impl per trait per type (Type System ch. 3, Coherence)",
                        l.modules[id].at(i.span),
                        l.modules[other.module].file
                    ));
                }
                l.impls.insert(key, ImplInfo { methods, module: id });
            }
            DeclKind::Check(_) | DeclKind::Test(_) | DeclKind::Stmt(_) => {}
        }
    }
    // Functions: signatures with their clauses; a prelude signature with no clauses is a
    // builtin.
    let mut fns: Vec<Rc<str>> = Vec::new();
    let mut grouped: HashMap<Rc<str>, Vec<&'static FnClause>> = HashMap::new();
    for (n, c) in clauses {
        if !grouped.contains_key(&n) {
            fns.push(n.clone());
        }
        grouped.entry(n).or_default().push(c);
    }
    for n in fns {
        let cs = grouped.remove(&n).unwrap();
        if let Some(Global::Method(_)) = decls.get(&n) {
            errors.push(format!("{}: {n} is a trait method; give it in an impl", l.modules[id].at(cs[0].span)));
            continue;
        }
        if decls.contains_key(&n) {
            errors.push(format!("{}: {n} is declared twice in this module", l.modules[id].at(cs[0].span)));
            continue;
        }
        let sig = sigs.remove(&n);
        decls.insert(n, Global::Fn { sig, clauses: cs });
    }
    for (n, s) in sigs {
        match decls.get_mut(&n) {
            Some(Global::Widget { sig, .. }) => *sig = Some(s),
            Some(Global::Let(_)) => {}
            Some(_) => {}
            None if l.modules[id].name.as_deref() == Some("Prelude") => {
                decls.insert(n, Global::Builtin { sig: s });
            }
            None => errors.push(format!("{}: {n} has a signature and no definition", l.modules[id].at(s.span))),
        }
    }
    l.modules[id].decls = decls;
}

/// What a tag's `of` clause says: a fault kind, or an element and its output's kind.
fn tag_kind(of: &Type) -> (bool, OutputKind) {
    match &of.kind {
        TypeKind::Name { name, args } if name.base() == "Fault" => {
            let _ = args;
            (true, OutputKind::Unit)
        }
        TypeKind::Name { args, .. } => match args.first() {
            None => (false, OutputKind::Unit),
            Some(t) => (false, output_kind(t)),
        },
        _ => (false, OutputKind::Unit),
    }
}

/// The outermost sigil of a type, if it has one.
pub fn outer_sigil(t: &Type) -> Option<Sigil> {
    match &t.kind {
        TypeKind::Name { name, .. } => name.sigils.first().copied(),
        TypeKind::Sigiled { sigils, .. } => sigils.first().copied(),
        TypeKind::Unit(s) => s.first().copied(),
        TypeKind::Hole(n) => n.sigils.first().copied(),
        TypeKind::Group(t) => outer_sigil(t),
        TypeKind::Emit { ty, .. } => outer_sigil(ty),
        _ => None,
    }
}

fn output_kind(t: &Type) -> OutputKind {
    match (outer_sigil(t), &t.kind) {
        (Some(Sigil::Bang), _) => OutputKind::Event,
        (Some(Sigil::Dollar), _) => OutputKind::Dynamic,
        (None, TypeKind::Unit(_)) => OutputKind::Unit,
        // A bundle, `((WindowId, !WindowInput))`: each component by its own sigil.
        (None, TypeKind::Tuple(parts)) if parts.iter().any(|p| outer_sigil(p).is_some()) => {
            OutputKind::Bundle(parts.iter().map(output_kind).collect())
        }
        _ => OutputKind::Plain,
    }
}

fn field_kind(t: &Type) -> FieldKind {
    match outer_sigil(t) {
        Some(Sigil::Dollar) => FieldKind::Live,
        Some(Sigil::Bang) => FieldKind::Request,
        _ => FieldKind::Identity,
    }
}

/// The head of an impl's type, as runtime values name their types: `Sq`, `[]` for
/// arrays, `==>` for maps, `(())` for tuples.
pub fn type_head(t: &Type) -> String {
    match &t.kind {
        TypeKind::Name { name, .. } => name.path.join("."),
        TypeKind::Array { .. } => "[]".into(),
        TypeKind::Map { .. } => "==>".into(),
        TypeKind::Tuple(_) => "(())".into(),
        TypeKind::Group(t) => type_head(t),
        TypeKind::Unit(_) => "unit".into(),
        _ => "?".into(),
    }
}

/// Which parameter of a method's signature carries the trait's type: the first whose
/// type is the variable, or an array, map or optional of it. Defaults to the first.
fn dispatch_index(t: &Type, var: &str) -> usize {
    fn mentions(t: &Type, var: &str) -> bool {
        match &t.kind {
            TypeKind::Name { name, args } => {
                (name.path.len() == 1 && name.base() == var) || args.iter().any(|a| mentions(a, var))
            }
            TypeKind::Array { elem, .. } => mentions(elem, var),
            TypeKind::Group(t) => mentions(t, var),
            TypeKind::Sigiled { inner, .. } => mentions(inner, var),
            TypeKind::Map { key, value } => mentions(key, var) || mentions(value, var),
            _ => false,
        }
    }
    if let TypeKind::Fn { params, .. } = &t.kind {
        for (i, p) in params.iter().enumerate() {
            if mentions(p, var) {
                return i;
            }
        }
    }
    0
}

/// The names a pattern binds, with their sigils.
pub fn pattern_binders(p: &Pattern) -> Vec<String> {
    let mut out = Vec::new();
    fn go(p: &Pattern, out: &mut Vec<String>) {
        match &p.kind {
            // An uppercase name is a binder too where it is no constructor (a function-valued
            // parameter, `F`); a caller that knows the constructors skips those.
            PatternKind::Name(n) if n.path.len() == 1 && !matches!(n.base(), "nil" | "True" | "False") => out.push(n.text()),
            PatternKind::Ctor { args, .. } => args.iter().for_each(|a| go(a, out)),
            PatternKind::Record { fields, .. } => fields.iter().for_each(|f| go(&f.pattern, out)),
            PatternKind::Sigil { inner, .. } => go(inner, out),
            PatternKind::Tuple(ps) | PatternKind::Array(ps) => ps.iter().for_each(|a| go(a, out)),
            PatternKind::Group(p) => go(p, out),
            _ => {}
        }
    }
    go(p, &mut out);
    out
}

/// Resolve one module's `use` lines and build its bare-name scope.
fn resolve_uses(l: &mut Loaded, id: ModId, by_name: &HashMap<String, ModId>, errors: &mut Vec<String>) {
    let prelude = l.prelude;
    let mut imports: Vec<(ModId, bool, Span)> = Vec::new();
    if id != prelude {
        imports.push((prelude, true, Span::default()));
    }
    for u in &l.modules[id].ast.uses {
        let name = u.module.path.join(".");
        match by_name.get(&name) {
            Some(&m) => imports.push((m, u.star, u.span)),
            None => errors.push(format!(
                "{}: use of {name}, a module not among the files (Semantics ch. 6)",
                l.modules[id].at(u.span)
            )),
        }
    }
    let mut scope: HashMap<Rc<str>, ModId> = HashMap::new();
    let mut qualified = HashSet::new();
    qualified.insert(id);
    for (m, star, span) in imports {
        qualified.insert(m);
        if !star || m == id {
            continue;
        }
        let names: Vec<Rc<str>> = l.modules[m].decls.keys().cloned().collect();
        for n in names {
            if l.modules[id].decls.contains_key(&n) {
                errors.push(format!(
                    "{}: {n} is declared in this file and brought in by `use {}.*`; a file's own declarations do not shadow a use (Semantics ch. 6)",
                    if span == Span::default() && span.start == 0 && span.end == 0 {
                        l.modules[id].file.clone()
                    } else {
                        l.modules[id].at(span)
                    },
                    l.modules[m].name.as_deref().unwrap_or("?")
                ));
                continue;
            }
            if let Some(&other) = scope.get(&n) {
                if other != m {
                    errors.push(format!(
                        "{}: {n} is brought in by both {} and {}; two bare names that collide are an error at the use (Semantics ch. 6)",
                        l.modules[id].at(span),
                        l.modules[other].name.as_deref().unwrap_or("?"),
                        l.modules[m].name.as_deref().unwrap_or("?")
                    ));
                }
                continue;
            }
            scope.insert(n, m);
        }
    }
    l.modules[id].scope = scope;
    l.modules[id].qualified = qualified;
}
