# prism-hs: what is cut, what is missing, and the type checker

This is the list the next implementer needs before trusting a test written against this prototype.
Corners cut come first, because a cut corner makes a passing test conditional on the cut; missing
features come second; the type checker, which is most of the design work left, comes last.
Chapter references are to the book in `../ch*.md`.

## Corners cut (a passing test is conditional on each of these)

- **No type checker, so every lift is a guess.** The book's rule (ch03, lifting) is that the type
  checker decides from the expected type whether an expression is plain, `!`, `$` or `@`. The
  prototype decides from the runtime `Value`: an operator over a `VDyn` lifts, over a `VEvent`
  fires, otherwise computes. Where it guesses rather than observes: an array literal is always
  plain (`[[!a, !b]]` is an array of events, right; `[[$a, $b]]` as a `$[T]` is wrong); a map
  literal always lifts (`{$k: v}` becomes a `$` map, which the todos program needs, and a map of
  dynamics as a plain value is impossible); a `for`'s result kind is read off its last statement
  (`VEvent` → `!` consolidation, `VDyn` → `$`, else plain); a constructor over a `$` lifts even
  when a choice of dynamics was meant. Every test whose expectation depends on one of these is
  really a test of the guess.
- **No prompt-cycle check (ch04 "Cycles", P8).** A prompt cycle is not an error; it either hangs
  the scheduler or reads a stale cached firing. No chapter 9 cycle test can be written against
  this prototype, and a program that passes here may be rejected by a conforming implementation.
- **Holds pre-created before the block's other statements.** A hold can be used above its line,
  but its initial value is evaluated in the pre-pass and so cannot mention a later block-local
  `let` or bind. The book's binding groups (ch03) allow it. A test that puts a hold's initial
  value after a `let` it uses fails here for the wrong reason.
- **State answered from the script, not from a snapshot taken as the occurrence opened.** The
  Test World's `with`/`push` entries are the oracle. The book (ch06, ch10 "Every state element
  needs a `with` first") has the oracle answer from the snapshot *before* n and then deliver n's
  push; the prototype does both in one step, which agrees for every script in the book but not
  for a state element created in n whose entry changed in n by `push` and was read by a tag in
  the same occurrence.
- **Errors.** Only "no match arm" during a build is reported, as an aborted turn. Overflow does
  not abort (Haskell `Integer`, unbounded); division by zero, indexing out of range and a missing
  map key all return `Nil`, which is the book's `?` dynamically and would be a typed `?T` with a
  checker. No quarantine, no contained build
  failures, no `Error` values a program can hold (ch09).
- **Equality.** `Value`'s `Ord` compares functor values by id, so a map keyed by an event is
  legal here and not in the book. Equal-valued changes are not deduplicated, which matches the
  book (`$Distinct` is the opt-in), so every generation count in a test is right by the book's
  rule and would change if the rule changed.
- **One global namespace.** Top-level names from every loaded file share one environment, so
  example files run one at a time (`test.sh`). Chapter 3's modules and `Ns.*` imports do not
  exist; dotted names are just names.
- **Char literals** lex as one-character `Text`; there is no `Char` type.
- **Text interpolation** is parsed by matching braces inside the string at lex time, so a `}`
  inside a nested string literal inside `{...}` breaks it.
- **Paths of supplier-built slots** (`s/n`) do not exist because `intercept` does not.
- **`@started` fires through the pending queue**, which is right, but its event has no height of
  its own; a program that depends on `@started` ordering against a rebuild's event in the build
  occurrence is relying on queue order.

## Missing outright

- `intercept` and `@Forward` (ch06): no interceptors, no sandbox, mock, cache or container tests.
- Any World but the Test World (ch07): no CLI, no GUI, no clocks that advance by themselves.
- `update` blocks and `&` references (ch03).
- `Defer` and multi-occurrence turns (ch10 "occurrence 2" inside a turn).
- `expect_top`, `expect_top_none` depth semantics by the GUI library's container convention: the
  prototype has no containers, so "top" is the whole list.
- The typed-AST text form and goldens (ch10, ch12).
- `?`-lifting at the plain layer (`a / b + 1` as `?Nat`): `?` is `VSome`/`VNil` and operators do
  not lift over it.
- `match` on arrays with `..rest` patterns; record patterns with `..` work, array patterns do not.
- `Checked.*` and `Wrap.*`.
- `$Distinct`, `!Updated` exist; `!Leftmost` over an array of events exists; `Kind` does not.
- Module system, `Ns.*` imports, top-level type annotations are parsed and ignored.

## The type checker: what has to be decided before it is written

The checker is where most of the remaining design is, and these are the questions the prototype
could duck and a checker cannot. Each is phrased as the test file that would pin it.

1. **Lifting is directed by the expected type.** `let $y = F $x` lifts `F`; `let y = F $x` is an
   error, not a lift. Pin: a file of `let` lines with each sigil on the left and each on the
   right, expected to type or to fail with a named error. The prototype would accept all of them.
2. **Sigils on binders.** `\|$a| ...` takes a dynamic; `\|a| ...` takes a plain value and is lifted
   afresh at each application (ch03 "A plain function is lifted afresh"). Pin: one plain function
   applied to a plain, a `$` and a `!` argument, and a `$`-binder lambda applied to a plain
   argument, which must fail.
3. **Containers of functors.** `[!a]` is a plain array of events (ch03 says so); `$[T]` is a
   dynamic array; `[[$a, $b]]` must be one or the other by expected type. Maps likewise. Tuples
   are bundles and never lift. Pin: literals of each shape against each expected type.
4. **Widget results.** A widget block's result is its last statement; a `rebuild`'s bound name is
   `!Switch` of its event for an event result, `$Join` of `@HoldFirst` for a dynamic, `@HoldFirst`
   for a plain value, nothing for `unit`, and fieldwise for a bundle (ch03 "The bound name"). Pin:
   one `rebuild` per result kind, with the bound name's sigil checked by use.
5. **`for`'s consolidation kind** follows the block's result kind the same way. Pin: three `for`s.
6. **Element fields are plain; a `$` field lifts the element.** `@Element Ui.Label(text=$t)` is the
   shorthand; `@Element Ui.Label(text=!t)` is an error. A `requests` field is `!T` of the tag's
   declared request type; `responds` and `state` are results, not fields. Pin: one element per
   field kind, and one with a wrong sigil.
7. **Tag declarations type the Test World's steps.** `send p, v` is checked against the tag's
   `responds` type at compile time (ch10). Pin: a script that sends a `Tick` to a button, expected
   to fail to compile.
8. **`?` lifts at the plain layer only.** `a / b + 1` is `?Nat`; `$a / $b` is `$?Nat`; `?` never
   crosses a functor. Pin: arithmetic over each combination.
9. **Recursion through a hold is typed by the hold's declared sigil**, and recursion through a
   plain `let` in a block is an error unless through a function. Pin: `$count` on both sides
   (ok), `let x = x + 1` (error), a forward `!` reference (ok), a forward plain reference (error).
10. **Exhaustive `match`**, including on `?T` and on choices with payloads. Pin: a `match` missing
    the `Nil` arm, expected to fail.
11. **Top-level annotations are required** (ch03), and the checker never infers across
    definitions. Pin: an unannotated top-level `let`, expected to fail.
12. **What the checker prints.** Chapter 10's typed-AST goldens are the oracle for the checker's
    output; the prototype has no typed AST. Pin: the counter's golden.

The order to build it: (1) and (2) first, since every other rule is phrased in terms of expected
types and binder sigils; then (3) and (6), which fix what the prototype guesses today; then (4)
and (5); the rest are local checks. A `typecheck` test form, a file expected to fail with a named
error, is the one addition the script language needs for this, and it is not in chapter 10 yet.
