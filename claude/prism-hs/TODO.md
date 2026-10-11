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
- **Forward references by sigil, not by binding group.** A block is one `mdo` knot: a name bound
  later in the block is a placeholder whose constructor comes from its sigil (`!` an event, `$`
  a dynamic, anything else the bare knot). A later plain `let` used at build time above its line
  is reported as `Cycle` (the knot is forced), which the book's binding groups (ch03) would
  accept or reject by the group's shape, not by the sigil.
- **The `for` birth term is syntactic.** Chapter 14 leaves the prompt birth term out of a
  `for`'s consolidation when the typed AST says the block cannot fire at birth; here
  `canFireAtBirth` says so when the block mentions no `@started`, `rebuild`, `!Updated` or
  outer event. A block that can fire at birth and feeds the collection is a prompt cycle, and
  hangs (reported as `Cycle` by the stack bound below) instead of being rejected at compile time.
- **Cycle is a bounded stack, not a check.** A prompt cycle makes Spider recurse without bound
  while subscribing; the binary runs with `-K64m` and reports the overflow as `Cycle`. Chapter
  14's claim that Spider throws `EventLoopException` does not hold for a cycle through a
  `switch`. A legitimate program that needed more than 64 MB of stack would be reported as a
  cycle too.
- **State answered from the script, not from a snapshot taken as the occurrence opened.** The
  Test World's `with`/`push` entries are the oracle. The book (ch06, ch10 "Every state element
  needs a `with` first") has the oracle answer from the snapshot *before* n and then deliver n's
  push; the prototype does both in one step, which agrees for every script in the book but not
  for a state element created in n whose entry changed in n by `push` and was read by a tag in
  the same occurrence.
- **Errors.** An exception escaping the frame is the turn's error (`UnsuppliedState` at slot
  creation, `Build` for a bad match or operands, `Cycle` as above). An error inside a derived
  dynamic is lazy: it surfaces when the value is used (a `rebuild` forces the value it builds
  with, so most reach the turn), not necessarily in the occurrence that computed it. After an
  error the timeline is not trusted and the test ends; chapter 14's rollback by replay is not
  written. Overflow does not abort (Haskell `Integer`, unbounded); division by zero, indexing
  out of range and a missing map key all return `Nil`, which is the book's `?` dynamically and
  would be a typed `?T` with a checker. No quarantine, no contained build failures, no `Error`
  values a program can hold (ch09).
- **Equality.** `Value`'s `Ord` compares all events as equal and all dynamics as equal (Reflex
  gives them no identity), so a map keyed by events collapses to one entry; the book forbids the
  key. Equal-valued changes are not deduplicated, which matches the
  book (`$Distinct` is the opt-in), so every generation count in a test is right by the book's
  rule and would change if the rule changed.
- **One global namespace.** Top-level names from every loaded file share one environment, so
  example files run one at a time (`test.sh`). Chapter 3's modules and `Ns.*` imports do not
  exist; dotted names are just names.
- **Char literals** lex as one-character `Text`; there is no `Char` type.
- **Text interpolation** is parsed by matching braces inside the string at lex time, so a `}`
  inside a nested string literal inside `{...}` breaks it.
- **Paths of supplier-built slots** (`s/n`) do not exist because `intercept` does not.
- **`@started` is Reflex's `now`**, not chapter 14's `rebuild` over a constant; the two are one
  frame and one firing, but the chapter's reason for the spelling (one code path for both laws)
  is not taken.
- **`@HoldFirst`** is a hold from `Nil` that adopts the build occurrence's firing at commit; the
  book's "the event did not fire in the build occurrence" error is not raised.
- **A dynamic element's tag is read off the syntax** (`@Element Ui.Button(label=_)`); a dynamic
  element whose expression is not a constructor application gets no response event or state.

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
