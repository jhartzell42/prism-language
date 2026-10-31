//! Types, rows and bounds.
//!
//! Appendix A.5's type order, tightest first: "application by juxtaposition; sigil
//! stacks; `[a]`, `K ==> V` and `((a, b))`; `|` between row terms; `emit` clauses on a
//! blueprint type; `->`, right associative; `where` bounds." So:
//!
//! ```text
//! type      ::= "unbounded"? "fn" params "->" type | emit_type
//! params    ::= emit_type ("," emit_type)*
//! emit_type ::= union ("emit" union)?
//! union     ::= map ("|" map)*
//! map       ::= sigiled ("==>" map)?
//! sigiled   ::= free-sigil* app
//! app       ::= atom atom*            (the head a name; arguments atoms)
//! atom      ::= name | "unit" | "any" trait | hole | Fam.* | "(" type ")"
//!             | "((" type ("," type)* "))" | "[" type "]" | "[[" type "]]" | free-sigil+ atom
//! ```
//!
//! A function type always begins with `fn`; the books write no bare `a -> b`.

use super::*;

impl Parser {
    pub(crate) fn ty(&mut self) -> PResult<Type> {
        let start = self.span();
        let mut color = Color::Bounded;
        if self.eat_kw(Kw::Unbounded) {
            color = Color::Unbounded;
        } else if let TokenKind::Name(n) = self.kind() {
            // A color variable "written before its `fn`s" (Semantics ch. 7, doubt CV).
            if n.sigils.is_empty() && n.case == NameCase::Lower && n.path.len() == 1 && matches!(self.kind_at(1), TokenKind::Kw(Kw::Fn, _)) {
                color = Color::Var(self.ident()?);
            }
        }
        let unbounded = !matches!(color, Color::Bounded);
        if self.at_kw(Kw::Fn) {
            self.advance();
            let mut params = Vec::new();
            if !self.at_op(Op::Arrow) {
                params.push(self.emit_type()?);
                while self.at_comma() {
                    self.advance();
                    params.push(self.emit_type()?);
                }
            }
            self.expect_op(Op::Arrow)?;
            let result = self.ty()?;
            let span = start.to(result.span);
            return Ok(Type { kind: TypeKind::Fn { color, params, result: Box::new(result) }, span });
        }
        if unbounded {
            return self.error_here("a color (`unbounded`, or a color variable) is written before a function type's `fn`");
        }
        let t = self.emit_type()?;
        if self.at_op(Op::Arrow) {
            let span = self.span();
            return self.error_at(span, "a function type begins with `fn`: `fn A -> B`");
        }
        Ok(t)
    }

    /// The type after `as`: no `emit` clause, no function type without parentheses.
    pub(crate) fn ty_cast(&mut self) -> PResult<Type> {
        self.map_type()
    }

    pub(crate) fn emit_type(&mut self) -> PResult<Type> {
        let start = self.span();
        let t = self.union_type()?;
        if self.at_kw(Kw::Emit) {
            self.advance();
            let row = self.union_type()?;
            let span = start.to(row.span);
            return Ok(Type { kind: TypeKind::Emit { ty: Box::new(t), row: Box::new(row) }, span });
        }
        Ok(t)
    }

    fn union_type(&mut self) -> PResult<Type> {
        let start = self.span();
        let first = self.map_type()?;
        if !matches!(self.kind(), TokenKind::Bar) {
            return Ok(first);
        }
        let mut terms = vec![first];
        while matches!(self.kind(), TokenKind::Bar) {
            self.advance();
            terms.push(self.map_type()?);
        }
        let span = start.to(self.prev_span());
        Ok(Type { kind: TypeKind::RowUnion(terms), span })
    }

    fn map_type(&mut self) -> PResult<Type> {
        let start = self.span();
        let key = self.sigiled_type()?;
        if self.at_op(Op::MapArrow) {
            self.advance();
            let value = self.map_type()?;
            let span = start.to(value.span);
            return Ok(Type { kind: TypeKind::Map { key: Box::new(key), value: Box::new(value) }, span });
        }
        Ok(key)
    }

    fn sigiled_type(&mut self) -> PResult<Type> {
        let start = self.span();
        let mut sigils = Vec::new();
        while let TokenKind::FreeSigil(s) = self.kind() {
            sigils.push(*s);
            self.advance();
        }
        let inner = self.app_type()?;
        if sigils.is_empty() {
            return Ok(inner);
        }
        let span = start.to(inner.span);
        Ok(Type { kind: TypeKind::Sigiled { sigils, inner: Box::new(inner) }, span })
    }

    fn app_type(&mut self) -> PResult<Type> {
        let start = self.span();
        let head = self.type_atom()?;
        if let TypeKind::Name { name, args } = &head.kind {
            if args.is_empty() && self.type_atom_starts_here() {
                let name = name.clone();
                let mut args = Vec::new();
                while self.type_atom_starts_here() {
                    args.push(self.type_atom()?);
                }
                let span = start.to(self.prev_span());
                return Ok(Type { kind: TypeKind::Name { name, args }, span });
            }
        }
        Ok(head)
    }

    fn type_atom_starts_here(&self) -> bool {
        match self.kind() {
            TokenKind::Name(_) | TokenKind::FamilyStar(_) | TokenKind::FreeSigil(_) | TokenKind::Open { .. } => true,
            TokenKind::Kw(Kw::Unit | Kw::Any, _) => true,
            _ => false,
        }
    }

    fn type_atom(&mut self) -> PResult<Type> {
        let start = self.span();
        let tok = self.peek().clone();
        let t = match tok.kind {
            TokenKind::Name(n) => {
                self.advance();
                let name = SName { sigils: n.sigils, path: n.path, span: tok.span };
                if matches!(n.case, NameCase::Topic | NameCase::Hole) {
                    // "`_` followed by digits is a hole, legal only in an `impl` header";
                    // `impl ^Timeline !_:` writes the topic there too.
                    Type { kind: TypeKind::Hole(name), span: tok.span }
                } else {
                    self.check_type_name_scope(&name);
                    Type { kind: TypeKind::Name { name, args: Vec::new() }, span: tok.span }
                }
            }
            TokenKind::Kw(Kw::Unit, sigils) => {
                self.advance();
                Type { kind: TypeKind::Unit(sigils), span: tok.span }
            }
            TokenKind::Kw(Kw::Any, _) => {
                self.advance();
                let tr = self.trait_ref()?;
                Type { kind: TypeKind::Any(tr), span: start.to(self.prev_span()) }
            }
            TokenKind::FamilyStar(path) => {
                self.advance();
                Type { kind: TypeKind::Family(QualName { path, span: tok.span }), span: tok.span }
            }
            TokenKind::FreeSigil(_) => return self.sigiled_type(),
            TokenKind::Absent => {
                self.advance();
                self.out_of_scope(tok.span, "sparse tuples", "a sparse tuple type is out of this compiler");
                Type { kind: TypeKind::Absent, span: tok.span }
            }
            TokenKind::Open { kind: BracketKind::Paren, .. } => {
                if self.at_double_open(BracketKind::Paren) {
                    self.open_double();
                    let mut elems = vec![self.ty()?];
                    while self.at_comma() {
                        self.advance();
                        elems.push(self.ty()?);
                    }
                    self.close_double(BracketKind::Paren)?;
                    Type { kind: TypeKind::Tuple(elems), span: start.to(self.prev_span()) }
                } else {
                    self.advance();
                    let inner = self.ty()?;
                    self.expect_close(BracketKind::Paren)?;
                    Type { kind: TypeKind::Group(Box::new(inner)), span: start.to(self.prev_span()) }
                }
            }
            TokenKind::Open { kind: BracketKind::Square, .. } => {
                let doubled = self.at_double_open(BracketKind::Square);
                if doubled {
                    self.open_double();
                } else {
                    self.advance();
                }
                let elem = self.ty()?;
                if doubled {
                    self.close_double(BracketKind::Square)?;
                } else {
                    self.expect_close(BracketKind::Square)?;
                }
                Type { kind: TypeKind::Array { elem: Box::new(elem), doubled }, span: start.to(self.prev_span()) }
            }
            _ => return self.error_here("expected a type"),
        };
        if matches!(self.kind(), TokenKind::NonEmpty) {
            let m = self.advance();
            self.out_of_scope(m.span, "non-empty arrays", "the non-empty array type `[a]+` is out of this compiler");
            let span = start.to(m.span);
            return Ok(Type { kind: TypeKind::NonEmpty(Box::new(t)), span });
        }
        Ok(t)
    }

    pub(crate) fn trait_ref(&mut self) -> PResult<TraitRef> {
        match self.kind().clone() {
            TokenKind::Trait(path) => {
                let span = self.advance().span;
                if path.len() == 1 && path[0] == "Monad" {
                    self.out_of_scope(span, "`T monad:` blocks and `^Monad`", "`^Monad` is out of this compiler");
                }
                Ok(TraitRef { path, span })
            }
            _ => self.error_here("expected a trait, `^Name`"),
        }
    }

    /// `where a: ^Ord, b: ^Show, ^Eq`: after a comma, a trait continues the same
    /// variable's list and `name:` begins the next.
    pub(crate) fn bounds(&mut self) -> PResult<Vec<Bound>> {
        let mut out = Vec::new();
        loop {
            let start = self.span();
            let var = self.ident()?;
            self.expect_colon()?;
            let mut traits = vec![self.trait_ref()?];
            while self.at_comma() && matches!(self.kind_at(1), TokenKind::Trait(_)) {
                self.advance();
                traits.push(self.trait_ref()?);
            }
            out.push(Bound { var, traits, span: start.to(self.prev_span()) });
            if self.at_comma() && matches!(self.kind_at(1), TokenKind::Name(_)) && matches!(self.kind_at(2), TokenKind::Colon) {
                self.advance();
                continue;
            }
            return Ok(out);
        }
    }

    pub(crate) fn ident(&mut self) -> PResult<Ident> {
        match self.kind().clone() {
            TokenKind::Name(n) if n.sigils.is_empty() && n.path.len() == 1 && n.case == NameCase::Lower => {
                let span = self.advance().span;
                Ok(Ident { name: n.path[0].clone(), span })
            }
            TokenKind::Kw(Kw::Key, s) if s.is_empty() => {
                let span = self.advance().span;
                Ok(Ident { name: "key".into(), span })
            }
            _ => self.error_here("expected a lowercase name"),
        }
    }

    pub(crate) fn qual_name(&mut self) -> PResult<QualName> {
        match self.kind().clone() {
            TokenKind::Name(n) if n.sigils.is_empty() && n.case == NameCase::Upper => {
                let span = self.advance().span;
                Ok(QualName { path: n.path, span })
            }
            _ => self.error_here("expected an uppercase name"),
        }
    }

    /// A row in an `emit` line or statement: `Ui.* | Net.*`, `r`, `r | Ui.*`, `Element`.
    pub(crate) fn row(&mut self) -> PResult<Row> {
        let start = self.span();
        let mut terms = vec![self.row_term()?];
        while matches!(self.kind(), TokenKind::Bar) {
            self.advance();
            terms.push(self.row_term()?);
        }
        Ok(Row { terms, span: start.to(self.prev_span()) })
    }

    fn row_term(&mut self) -> PResult<RowTerm> {
        match self.kind().clone() {
            TokenKind::FamilyStar(path) => {
                let span = self.advance().span;
                Ok(RowTerm::Family(QualName { path, span }))
            }
            TokenKind::Name(n) if n.sigils.is_empty() && n.case == NameCase::Lower && n.path.len() == 1 && !self.type_atom_after_name() => {
                let span = self.advance().span;
                Ok(RowTerm::Var(Ident { name: n.path[0].clone(), span }))
            }
            _ => Ok(RowTerm::Type(self.map_type()?)),
        }
    }

    fn type_atom_after_name(&self) -> bool {
        matches!(self.kind_at(1), TokenKind::Name(_) | TokenKind::Open { .. } | TokenKind::FamilyStar(_))
    }

    /// Type names out of this compiler: sized integers other than `U8`, `Float`, `Instant`.
    fn check_type_name_scope(&mut self, name: &SName) {
        if name.path.len() != 1 {
            return;
        }
        let b = name.base();
        let sized = ["U16", "U32", "U64", "U128", "I8", "I16", "I32", "I64", "I128"];
        if sized.contains(&b) {
            self.out_of_scope(name.span, "sized ints other than `U8`", format!("`{b}` is out of this compiler: the sized integer it has is `U8`"));
        } else if b == "Float" {
            self.out_of_scope(name.span, "Float", "`Float` is out of this compiler");
        } else if b == "Instant" {
            self.out_of_scope(name.span, "`Instant`", "`Instant` is out of this compiler");
        }
    }
}
