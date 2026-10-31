//! The top-level forms (Semantics ch. 3, "The top-level forms, designed as a set";
//! ch. 6, "The declaration forms"; Appendix A.6, `file` and `decl`).

use super::*;

impl Parser {
    /// A file: the header (`module`, then `emit`, then `use`), then declarations.
    /// With `fragment`, statements are accepted among the declarations.
    pub(crate) fn module(&mut self, fragment: bool) -> Module {
        let start = self.span();
        let mut module = Module { name: None, emit: None, uses: Vec::new(), decls: Vec::new(), template: None, span: start };
        if let TokenKind::Template(t) = self.kind().clone() {
            let span = self.advance().span;
            if let Ok(t) = self.template(*t, span) {
                module.template = Some(t);
            }
        }
        // 0: module allowed; 1: emit allowed; 2: use allowed; 3: declarations.
        let mut header_stage = 0u8;
        while !self.at_eof() {
            if self.at_layout(Layout::Sep) {
                self.advance();
                continue;
            }
            let item_start = self.span();
            let r: PResult<()> = (|| {
                match self.kind() {
                    TokenKind::Kw(Kw::Module, s) if s.is_empty() => {
                        self.advance();
                        let name = self.qual_name()?;
                        if header_stage > 0 || module.name.is_some() {
                            self.note_error(item_start, "the `module` line comes first in a file");
                        }
                        module.name = Some(name);
                        header_stage = 1;
                    }
                    TokenKind::Kw(Kw::Emit, s) if s.is_empty() => {
                        self.advance();
                        let row = self.row()?;
                        if header_stage > 1 || module.emit.is_some() {
                            self.note_error(item_start, "`emit` comes after `module` and above `use`, once (Semantics ch. 6)");
                        }
                        module.emit = Some(row);
                        header_stage = 2;
                    }
                    TokenKind::Kw(Kw::Use, s) if s.is_empty() => {
                        self.advance();
                        let u = self.use_line(item_start)?;
                        if header_stage > 2 {
                            self.note_error(item_start, "`use` lines belong to the header, above the declarations");
                        }
                        module.uses.push(u);
                        header_stage = 2;
                    }
                    _ => {
                        header_stage = 3;
                        let d = self.decl(fragment)?;
                        module.decls.push(d);
                    }
                }
                Ok(())
            })();
            if r.is_err() {
                self.recover_to_stmt_end();
            }
            if self.at_layout(Layout::Close) {
                // A stray close at the top level only follows an error.
                self.advance();
                continue;
            }
            if !self.at_eof() {
                self.expect_stmt_end("declaration");
            }
        }
        module.span = start.to(self.prev_span());
        module
    }

    fn use_line(&mut self, start: Span) -> PResult<Use> {
        match self.kind().clone() {
            TokenKind::FamilyStar(path) => {
                let span = self.advance().span;
                Ok(Use { module: QualName { path, span }, star: true, span: start.to(span) })
            }
            TokenKind::Name(n) if n.sigils.is_empty() && n.case == NameCase::Upper => {
                let span = self.advance().span;
                Ok(Use { module: QualName { path: n.path, span }, star: false, span: start.to(span) })
            }
            _ => self.error_here("expected a module name after `use`"),
        }
    }

    pub(crate) fn decl(&mut self, fragment: bool) -> PResult<Decl> {
        let start = self.span();
        let kind = match self.kind().clone() {
            TokenKind::Kw(Kw::Type, _) => DeclKind::Type(self.type_decl()?),
            TokenKind::Kw(Kw::Tag, _) => DeclKind::Tag(self.tag_decl()?),
            TokenKind::Kw(Kw::Derive, _) => {
                self.advance();
                let mut traits = vec![self.trait_ref()?];
                while self.at_comma() {
                    self.advance();
                    traits.push(self.trait_ref()?);
                }
                DeclKind::Derive(traits)
            }
            TokenKind::Kw(Kw::Trait, _) => DeclKind::Trait(self.trait_decl()?),
            TokenKind::Kw(Kw::Impl, _) => DeclKind::Impl(self.impl_decl()?),
            TokenKind::Kw(Kw::Check, _) => DeclKind::Check(self.check_decl()?),
            TokenKind::Kw(Kw::Test, _) => DeclKind::Test(self.test_decl()?),
            TokenKind::Kw(Kw::Fn, _) => DeclKind::Fn(self.fn_clause()?),
            TokenKind::Kw(Kw::Let, _) => {
                self.advance();
                let pattern = self.pattern(Ctx::ARG)?;
                let ty = if self.at_colon() {
                    self.advance();
                    Some(self.ty()?)
                } else {
                    None
                };
                self.expect_op(Op::Eq)?;
                let value = self.rhs_expr()?;
                DeclKind::Let(LetDecl { pattern, ty, value, span: start.to(self.prev_span()) })
            }
            TokenKind::Name(_) if matches!(self.kind_at(1), TokenKind::Colon) => {
                if matches!(self.kind_at(2), TokenKind::Layout(Layout::Open)) {
                    let name = self.sname()?;
                    let body = self.block_after_colon()?;
                    let span = start.to(body.span);
                    DeclKind::Widget(WidgetDecl { name, body, span })
                } else {
                    DeclKind::Signature(self.signature()?)
                }
            }
            _ if fragment => DeclKind::Stmt(self.stmt()?),
            _ => {
                // A statement at the top: name what it is, since "Nothing at top level is
                // built" (Semantics ch. 6).
                let st = self.speculate(|p| p.stmt());
                return match st {
                    Some(Stmt { kind: StmtKind::Bind { .. }, .. }) => self.error_at(start, "a bind or hold is a statement of a widget block: nothing at top level is built (Semantics ch. 6)"),
                    _ => self.error_here("expected a declaration (`fn`, `let`, `type`, `tag`, `derive`, `trait`, `impl`, `test`, `check`, or a signature `Name: type`)"),
                };
            }
        };
        Ok(Decl { kind, span: start.to(self.prev_span()) })
    }

    /// `Name: type (where bounds)?`, or in a trait, `+: fn a, a -> a`.
    pub(crate) fn signature(&mut self) -> PResult<Signature> {
        let start = self.span();
        let name = self.def_name()?;
        self.expect_colon()?;
        let ty = self.ty()?;
        let bounds = if self.eat_kw(Kw::Where) { self.bounds()? } else { Vec::new() };
        Ok(Signature { name, ty, bounds, span: start.to(self.prev_span()) })
    }

    fn def_name(&mut self) -> PResult<DefName> {
        match self.kind().clone() {
            TokenKind::Name(n) => {
                let name = self.sname()?;
                if n.sigils.first() == Some(&Sigil::Amp) && name.is_upper() {
                    self.out_of_scope(name.span, "named lenses `&Home` and `Over`", format!("the named lens `{}` is out of this compiler", name.text()));
                }
                Ok(DefName::Name(name))
            }
            TokenKind::Op { op, .. } if self.in_trait => {
                let span = self.advance().span;
                Ok(DefName::Op(op.as_str().to_string(), span))
            }
            _ => self.error_here("expected a name"),
        }
    }

    /// `fn Name p, q = e` or `fn Name p, q:` and a block: "each the keyword `fn`, the name,
    /// its parameters, and `=` for an expression body or `:` for a block".
    pub(crate) fn fn_clause(&mut self) -> PResult<FnClause> {
        let start = self.expect_kw(Kw::Fn)?;
        let name = self.def_name()?;
        let mut params = Vec::new();
        if !self.at_op(Op::Eq) && !self.at_colon() {
            params.push(self.pattern(Ctx::ARG)?);
            while self.at_comma() {
                self.advance();
                params.push(self.pattern(Ctx::ARG)?);
            }
        }
        let body = if self.eat_op(Op::Eq) {
            FnBody::Expr(self.rhs_expr()?)
        } else if self.at_colon() {
            FnBody::Block(self.block_after_colon()?)
        } else {
            return self.error_here("expected `=` and an expression, or `:` and a block, after the parameters");
        };
        Ok(FnClause { name, params, body, span: start.to(self.prev_span()) })
    }

    /// `type Name params (= body)?`.
    fn type_decl(&mut self) -> PResult<TypeDecl> {
        let start = self.advance().span;
        let name = self.qual_name()?;
        let mut params = Vec::new();
        while let TokenKind::Name(n) = self.kind() {
            if n.case != NameCase::Lower || !n.sigils.is_empty() {
                break;
            }
            params.push(self.ident()?);
        }
        let body = if self.eat_op(Op::Eq) {
            Some(self.type_body(&name, &params)?)
        } else {
            None
        };
        Ok(TypeDecl { name, params, body, span: start.to(self.prev_span()) })
    }

    fn type_body(&mut self, name: &QualName, params: &[Ident]) -> PResult<TypeBody> {
        if self.eat_kw(Kw::Record) {
            let fields = self.field_block()?;
            return Ok(TypeBody::Record(fields));
        }
        if self.eat_kw(Kw::Choice) {
            self.expect_colon()?;
            if !self.at_layout(Layout::Open) {
                return self.error_here("a choice's cases stand in an indented block after `choice:`");
            }
            self.advance();
            let mut cases = Vec::new();
            self.layout_items(|p| {
                cases.push(p.case()?);
                Ok(())
            });
            return Ok(TypeBody::Choice(cases));
        }
        if self.eat_kw(Kw::Row) {
            return Ok(TypeBody::Row);
        }
        match self.kind().clone() {
            // "A newtype is a choice of one alternative on one line, `type Meters = Meters
            // Float`."
            TokenKind::Name(n) if n.sigils.is_empty() && n.path.last() == name.path.last() && n.path.len() == 1 => {
                Ok(TypeBody::Newtype(self.case()?))
            }
            // "A unit type has one value, lowercase, `type Click = click`."
            TokenKind::Name(n)
                if n.sigils.is_empty() && n.case == NameCase::Lower && n.path.len() == 1 && !params.iter().any(|p| p.name == n.path[0]) && matches!(self.kind_at(1), TokenKind::Layout(_) | TokenKind::Eof) =>
            {
                Ok(TypeBody::Unit(self.ident()?))
            }
            _ => Ok(TypeBody::Alias(self.ty()?)),
        }
    }

    /// Parses items of an indented block (after its `open`) up to its `close`, recovering
    /// at each item.
    pub(crate) fn layout_items(&mut self, mut f: impl FnMut(&mut Parser) -> PResult<()>) {
        loop {
            if f(self).is_err() {
                self.recover_to_stmt_end();
            }
            match self.kind() {
                TokenKind::Layout(Layout::Sep) => {
                    self.advance();
                }
                TokenKind::Layout(Layout::Close) => {
                    self.advance();
                    return;
                }
                TokenKind::Eof => return,
                _ => {
                    let _ = self.error_here::<()>("expected the end of the line");
                    self.recover_to_stmt_end();
                }
            }
        }
    }

    /// `: open field (sep field)* close`.
    fn field_block(&mut self) -> PResult<Vec<Field>> {
        self.expect_colon()?;
        if !self.at_layout(Layout::Open) {
            return self.error_here("fields stand in an indented block, one `name: Type` per line");
        }
        self.advance();
        let mut fields = Vec::new();
        self.layout_items(|p| {
            let start = p.span();
            let name = p.ident()?;
            p.expect_colon()?;
            let ty = p.ty()?;
            fields.push(Field { name, ty, span: start.to(p.prev_span()) });
            Ok(())
        });
        Ok(fields)
    }

    /// A choice's alternative: `Name`, `Name T, U`, or the indexed `Name: T -> R`.
    fn case(&mut self) -> PResult<Case> {
        let start = self.span();
        let name = self.sname()?;
        if !name.sigils.is_empty() {
            self.note_error(name.span, "a constructor takes no sigil");
        }
        if self.at_colon() {
            // `Lit: Int -> Expr Int`: an indexed choice (a GADT).
            self.advance();
            let mut args = Vec::new();
            if !self.at_op(Op::Arrow) {
                args.push(self.emit_type_pub()?);
                while self.at_comma() {
                    self.advance();
                    args.push(self.emit_type_pub()?);
                }
            }
            self.expect_op(Op::Arrow)?;
            let result = self.ty()?;
            self.out_of_scope(start, "indexed choices/GADTs for user types", format!("`{}` is an indexed alternative: indexed choices for user types are out of this compiler", name.text()));
            return Ok(Case { name, args, indexed_result: Some(result), span: start.to(self.prev_span()) });
        }
        let mut args = Vec::new();
        if !self.at_stmt_boundary() {
            args.push(self.ty()?);
            while self.at_comma() {
                self.advance();
                args.push(self.ty()?);
            }
        }
        Ok(Case { name, args, indexed_result: None, span: start.to(self.prev_span()) })
    }

    fn emit_type_pub(&mut self) -> PResult<Type> {
        self.emit_type()
    }

    /// `tag F.Name of K out (where bounds)? (: fields)?`.
    fn tag_decl(&mut self) -> PResult<TagDecl> {
        let start = self.advance().span;
        let name = self.qual_name()?;
        self.expect_kw(Kw::Of)?;
        let of = self.ty()?;
        let bounds = if self.eat_kw(Kw::Where) { self.bounds()? } else { Vec::new() };
        let fields = if self.at_colon() { self.field_block()? } else { Vec::new() };
        Ok(TagDecl { name, of, bounds, fields, span: start.to(self.prev_span()) })
    }

    /// `trait ^T a (where bounds)?:` and its items.
    fn trait_decl(&mut self) -> PResult<TraitDecl> {
        let start = self.advance().span;
        let name = self.trait_ref()?;
        let mut params = Vec::new();
        while matches!(self.kind(), TokenKind::Name(n) if n.case == NameCase::Lower && n.sigils.is_empty()) {
            params.push(self.ident()?);
        }
        let bounds = if self.eat_kw(Kw::Where) { self.bounds()? } else { Vec::new() };
        let items = self.trait_items()?;
        let decl = TraitDecl { name, params, bounds, items, span: start.to(self.prev_span()) };
        self.check_higher_kinded(&decl);
        Ok(decl)
    }

    /// `impl ^T Type (where bounds)?:` and its items.
    fn impl_decl(&mut self) -> PResult<ImplDecl> {
        let start = self.advance().span;
        let trait_ = self.trait_ref()?;
        let ty = self.ty()?;
        let bounds = if self.eat_kw(Kw::Where) { self.bounds()? } else { Vec::new() };
        let items = self.trait_items()?;
        Ok(ImplDecl { trait_, ty, bounds, items, span: start.to(self.prev_span()) })
    }

    fn trait_items(&mut self) -> PResult<Vec<TraitItem>> {
        self.expect_colon()?;
        if !self.at_layout(Layout::Open) {
            return self.error_here("a trait's or impl's items stand in an indented block");
        }
        self.advance();
        let saved = self.in_trait;
        self.in_trait = true;
        let mut items = Vec::new();
        self.layout_items(|p| {
            let start = p.span();
            let item = match p.kind().clone() {
                TokenKind::Kw(Kw::Fn, _) => TraitItem::Fn(p.fn_clause()?),
                TokenKind::Kw(Kw::Type, _) => {
                    p.advance();
                    let name = match p.kind().clone() {
                        TokenKind::Name(n) if n.case == NameCase::Upper && n.sigils.is_empty() && n.path.len() == 1 => {
                            let span = p.advance().span;
                            Ident { name: n.path[0].clone(), span }
                        }
                        _ => return p.error_here("expected the associated type's name"),
                    };
                    let mut params = Vec::new();
                    while matches!(p.kind(), TokenKind::Name(n) if n.case == NameCase::Lower && n.sigils.is_empty()) {
                        params.push(p.ident()?);
                    }
                    let value = if p.eat_op(Op::Eq) { Some(p.ty()?) } else { None };
                    TraitItem::AssocType { name, params, value, span: start.to(p.prev_span()) }
                }
                TokenKind::Kw(Kw::Row, _) => {
                    p.advance();
                    let name = p.sname()?;
                    TraitItem::AssocRow { name, span: start.to(p.prev_span()) }
                }
                _ => TraitItem::Signature(p.signature()?),
            };
            items.push(item);
            Ok(())
        });
        self.in_trait = saved;
        Ok(items)
    }

    /// "higher-kinded traits" are out: a trait whose parameter is applied to arguments in
    /// its items' types (`trait ^Timeline f:` with `f a`).
    fn check_higher_kinded(&mut self, t: &TraitDecl) {
        fn applies(ty: &Type, params: &[Ident]) -> bool {
            match &ty.kind {
                TypeKind::Name { name, args } => {
                    (!args.is_empty() && name.path.len() == 1 && params.iter().any(|p| p.name == name.path[0])) || args.iter().any(|a| applies(a, params))
                }
                TypeKind::Sigiled { inner, .. } | TypeKind::Group(inner) | TypeKind::NonEmpty(inner) => applies(inner, params),
                TypeKind::Array { elem, .. } => applies(elem, params),
                TypeKind::Tuple(v) | TypeKind::RowUnion(v) => v.iter().any(|a| applies(a, params)),
                TypeKind::Map { key, value } => applies(key, params) || applies(value, params),
                TypeKind::Fn { params: ps, result, .. } => ps.iter().any(|a| applies(a, params)) || applies(result, params),
                TypeKind::Emit { ty, row } => applies(ty, params) || applies(row, params),
                _ => false,
            }
        }
        let hk = t.items.iter().any(|i| match i {
            TraitItem::Signature(s) => applies(&s.ty, &t.params),
            _ => false,
        });
        if hk {
            let is_monad = t.name.path == ["Monad"];
            if !is_monad {
                self.out_of_scope(t.name.span, "higher-kinded traits", format!("`^{}` applies its parameter to arguments: higher-kinded traits are out of this compiler", t.name.path.join(".")));
            }
        }
    }
}
