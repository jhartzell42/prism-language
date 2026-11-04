# Checker bug: a lambda supplier's `o` is not rank-2, so tag arms do not refine it

*Written by Claude on the claude-self-hosting branch, 4 November 2026; subject to change.*

## The scenario

A host answers two tags of its child with different outputs, in one inline supplier:

```prism
@host:
    ignore <- @HostDocument @child, \|p, e| match e:              # 0/[p]
        Fs.Contents(..): @ $ (~ Contents(text="A"))
        Fs.File(..): @ never
        otherwise: @Element e
```

prismc refuses it:

```
arms.prism:15: error[mismatch]: expected `$~Contents`, found `!t20`
```

The first arm fixed `o` to `$~Contents`, and the second arm's `!~Contents` is checked
against it. The same supplier written as a named function with a signature is accepted:

```prism
@Supply: fn Path, Element (Ui.* | Fs.*) o -> @o
fn @Supply p, e = match e:
    Fs.Contents(..): @ $ (~ Contents(text="A"))
    Fs.File(..): @ never
    otherwise: @Element e

@host:
    ignore <- @HostDocument @child, @Supply                       # 0/[p]
```

The two programs differ only in where the supplier is written.

## Is it a bug?

- **Yes, against the books.** Type System ch. 2: "The supplier is rank-2 in `o`, and the
  signature supplies it." Type System ch. 5, "Refinement at tag matches": refinement
  applies over `e : Element R o` "with `o` rigid (a supplier's, rank-2) or known".
  Semantics ch. 9's first example, `@Offline`, is an inline lambda answering `Net.Http`
  with `@ never` and `Net.Socket` with a pair, the refused case exactly.
- **Not a language limit.** The named-supplier form shows the checker's refinement
  works when `o` is rigid.

## Is hosting special-cased?

No, and that is the bug. Nothing in the checker knows that `@HostDocument`'s supplier is
rank-2:

- **Refinement needs a rigid `o`.** `InferMatch` (`prismc/check-expr.prism`, about line
  1188) refines an arm only when the scrutinee's output walks to a `TyRigid`, and
  `InferArm` substitutes the tag's output for it (`SubstRigid`).
- **Rigid variables come only from a declared signature.** Skolemizing a scheme in a body
  (`prismc/check-types.prism`, about line 875) is the only place a `TyRigid` is made.
- **A lambda argument gets a flexible `o`.** Calling `@HostDocument` instantiates its
  scheme, so the `o` in `(fn Path, Element c o -> @o)` is a fresh unification variable,
  quantified at `@HostDocument`'s level (rank 1). The lambda is inferred against it, and
  the first arm binds it.
- **There is no rank handling anywhere.** The checker has no notion of a variable
  quantified at a parameter; `grep -i rank prismc/check-*.prism` finds nothing.
- **Why the suite never caught it:** every hosting test and the window library answer at
  most one tag with a non-forwarding output per inline supplier, or answer every tag
  alike (`@Element e`). A single answering arm unifies `o` once and passes.

## The design question under the fix

The books say "the signature supplies it", but `fn @a emit c, (fn Path, Element c o ->
@o) -> ...` does not say which `o` is rank-2. A signature needs a rule. Options:

- **(a) A variable that appears only inside one function-typed parameter is quantified
  at that parameter.** `o` appears nowhere else in `@HostDocument`'s signature, so it is
  rank-2 there, and the rule needs no new syntax.
  - *Cost:* a reader must check where else a variable appears to know its rank.
  - *Risk:* an existing signature whose variable is confined to a parameter by accident
    changes meaning (none in the prelude besides the three host forms, as far as a grep
    shows).
- **(b) An explicit quantifier in the parameter,** e.g. `(any o: fn Path, Element c o ->
  @o)`, reusing the existential's `any` for "for every `o`" would be confusing; a new word
  (`each o:`) would be clearer.
  - *Cost:* new syntax.
  - *Buys:* the rank is visible where it matters.
- **(c) Special-case the three host forms** in the checker.
  - *Cost:* a user's own host-like widget (taking a supplier) could not refine.
  - *Refused by Claude:* it makes hosting privileged, against "no privileged World".

**Claude's recommendation:** (a). It matches "the signature supplies it" with no new
syntax, it covers user-written hosts, and the risk is checkable by a grep.

## The fix, under (a)

- **At a call,** when an argument is a lambda checked against a function-typed parameter
  whose variable is confined to it, skolemize that variable to a fresh rigid one for the
  lambda's body, and check the lambda against the parameter (checking mode, so each arm
  gets `?expected`).
- **Escape check:** the rigid variable must not leak into the call's result or other
  arguments (`refinement-escape`, as the books name it).
- **A named supplier** keeps working as it does: its own scheme instantiates to a fresh
  flexible `o`, which unifies with the parameter's rigid one.
- **Tests first:**
  - `ts-ch05-supplier-rank2`: the refused program above, answering two tags with
    different outputs inline; expected to check, build and answer both places;
  - a negative test where the lambda leaks `o` (e.g. stores the element's output in a
    hold outside), expecting `refinement-escape`.
