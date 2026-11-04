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

## The rule (Claude's, at Jimmy's "make something up")

**An element's output variable that appears only inside one function-typed parameter is
quantified at that parameter.** In `@HostDocument: fn @a emit c, (fn Path, Element c o ->
@o) -> @((a, $(Path ==> Elem c)))`, `o` is the output of an `Element` and appears nowhere
outside the supplier parameter, so the supplier must work for every `o`: rank-2. Nothing
else changes rank.

- **Why only element outputs:** a tag arm is the one place where a variable's rank
  changes what a program can do (it is what lets an arm assume `o ~ !HttpResponse`). A
  general confinement rule would also make `fn (fn Nat -> b) -> unit` demand a
  polymorphic argument, rejecting `\|n| Show n`, which a rank-1 reading accepts and no
  body could tell apart.
- **Why no syntax:** it is what "the signature supplies it" (Type System ch. 2) already
  says. A grep of the prelude, prismc, the examples and the tests finds the rule applies
  to the three host forms and nothing else, so no existing program changes meaning.
- **Refused:**
  - an explicit quantifier in the parameter (`each o: fn ...`): new syntax for a rule
    a signature already implies;
  - a confinement rule for every variable (above);
  - special-casing the three host forms in the checker: a user's own host-like widget
    taking a supplier could not refine, and it makes hosting privileged.
- **Books:** add the rule, in one sentence, to Type System ch. 5, "Refinement at tag
  matches", where it now says "with `o` rigid (a supplier's, rank-2)".

## The fix

- **At a call,** when an argument is a lambda checked against a function-typed parameter
  holding a confined element output, skolemize that variable to a fresh rigid one for
  the lambda's body, and check the lambda against the parameter in checking mode, so
  each arm gets `?expected` and `InferArm` refines it.
- **Escape check:** the rigid variable must not leak into the call's result or other
  arguments (`refinement-escape`, as Type System ch. 5 names it).
- **A named supplier** keeps working as it does: its scheme instantiates to a fresh
  flexible `o`, which unifies with the parameter's rigid one. A named supplier that is
  not polymorphic in `o` (`fn Path, Element c Nat -> @Nat`) is refused there, which is
  right.
- **Tests first,** committed failing before the fix:
  - `ts-ch05-supplier-rank2`: the refused program above, answering two tags with
    different outputs inline; it checks, builds, and answers both places;
  - `ts-ch05-supplier-escape`: a lambda supplier that leaks `o` (the element's output
    stored in a hold outside the match), refused with `refinement-escape`;
  - a checker test that `fn (fn Nat -> b) -> unit` still accepts `\|n| Show n`.
