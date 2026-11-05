# A field read through a `?` is checked but not lowered

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to
change. Found while writing the regular-expression conformance programs; not fixed.*

## The scenario

Type System ch. 2's own example reads a field through an optional:

```prism
?Key: fn Text -> ?Text
fn ?Key line = match ?Find %/^(?<key>[a-z_]+)\s*=/, line:
    ? m: m.named["key"].text
    nil: nil
```

`m.named["key"]` is a `?Group`, and the book says the field read lifts through the `?`
(Type System ch. 4), giving a `?Text`. The same shape with a plain record,
`fn ?F k = ps[k].a` over `ps: Text ==> P`, checks clean and then fails in cargo:

```
error[E0609]: no field `a` on type `Option<P>`
    get(&ps(), k).a.clone()
```

## The cause

The checker lifts the field read (`Check.Expr.Lifting`); lowering's `FieldValue`
(`prismc/lower/expr.prism`) reads the field off the value as if it were the record, so
the printers write `.a` on an `Option` (C++: on a `std::optional`). Unsupported is a hard
error, so until the fix the lowering should refuse the lifted read with a message rather
than emit code the C++ or Rust compiler refuses.

## The fix

`FieldValue` on a `?` (and `~`) receiver maps the read over it: Rust `.as_ref().map(|r|
r.a.clone())`, C++ a conditional on `has_value()`, with a conformance program reading
the book's example. tests/conformance/sem-appB-regex-groups writes `?Key` with a nested
`match` meanwhile.

## Also seen

A user function `Field` lowers to Rust `field`, which collides with the runtime's POD
helper `field` that the emitted `FromPod` impls call (E0061 in the same crate); its own
report when it is traced.
