# Rulings made during the prismc run, after the books moved

*Written by Claude (the prismc coordinator), 10 November 2026. Subject to change;
implementers use discretion.*

The Semantics and Type System books moved into the Prism Bible (prism-notes, `claude/`),
which another session writes. Rulings Jimmy makes during the prismc run after that move
are kept here, with his words, until they are woven into the Bible. Nothing is removed.

## 10 November 2026

- **AN** The answer constructor that ruling AQ generates for a tag is written as the
  tag's name with a last segment `Answer`: `Ui.Button.Answer click`,
  `Fs.Contents.Answer (~ c)`. Jimmy: "AN seems good!" Refused: a suffix on the tag's own
  name, `Ui.ButtonAnswer`, which collides with a tag named `FooAnswer`. Where: Semantics
  chapter 9 and Appendix B.
- **RM** A README gets no edits from Claude except deleting what went stale. Jimmy: "stop
  modifying `README.md`. If they're stale entries, delete them though. Delete stale
  things." Whoever lowers a construct deletes its "Not lowered yet" entry in the same
  commit.
- **EL** The finished branch lowers everything the compiler knows it refuses. Jimmy: "I'd
  like the main Claude branch to contain proper lowering when you're done of everything
  you know about and not to have lies in the `README.md`."
- **IB** An inner block may rebind a name written outside it: a lambda's parameter, a
  clause's parameter, an arm's pattern, a `let` or a bind in an inner block each hide the
  outer name inside that block. Jimmy: "Inner block rebinding is fine, I don't know why
  it's forbidden". This overrules Semantics chapter 4's "an inner block does not rebind a
  written name from outside" and its negative-space line "no rebinding of a written name".
  A name bound twice in one block stays an error (`collision`), since a block's lines are
  an unordered group. Refused: enforcing the book's sentence, which breaks 14 places (10
  in prismc, 4 in two conformance programs).
- **TC** Places are counted as they are built. After a building `match` or `if`, or a
  widget of several clauses, the block's later places take the next numbers after the
  places the chosen arm or clause built, so their positions differ between instances
  that took different arms, and no position is left unborn. Jimmy: "count things as they
  are built then!" This settles the question TP kept alive (Semantics chapter 8, "takes
  the next number"). Refused: prismc's pick CS, the widest arm's positions reserved,
  which has no answer for a blueprint parameter or for a clause widget that calls itself
  in a static position (prismc overflowed its stack on one).
- **CL** `match x as [a]+?:` reads without parentheses: the lexer is fixed so that the
  checked cast's `+?` before a block's `:` is the non-empty mark and the option. Jimmy:
  "fix the lexer". Refused: leaving the book's lexing rule and writing
  `match (x as [a]+?):`. Where: Semantics chapter 2; Type System chapter 2, "Collections".
- **AD** A trait's default for an associated type is read: an impl that does not state
  that associated type takes the default, and `impl-lacks` is reported only for one with
  no default. Jimmy: "fix it". Where: Type System chapter 3.
- **NS** The conversion from a non-empty collection to its plain type is symmetric at an
  operator: `[Nat]+ == [Nat]` checks as `[Nat] == [Nat]+` does. Jimmy: "change it".
  Refused: the conversion only at the right operand, the old rule as written. Where: Type
  System chapter 2, "Collections".
- **RA** Every lowering refusal is fixed by its class. A legal program the back end does
  not lower is lowered ("fix these all"); a program that is wrong gets a checker error; a
  guard no checked program reaches becomes an internal error. Jimmy: "apply appropriate
  fixes based on category".
- **BT** Work in bigger batches, one build per batch, and C++ last. Jimmy: "fix bugs in
  bigger batches rather than doing full builds after each one and save fixing C++ until
  the end when you make one big commit in the branch." During the run a fix is verified
  on Rust; the C++ fixes land as one commit after one C++ pass at the end.
- **TF** Tests first for everything, then the fixes, then the run. Jimmy: "Make failing
  tests for all the bugs, then try to fix all of them, then run the tests." A lowering
  gap is called a bug.
- **EP** An event field in an element pattern binds the event the description was made
  with, a `!T`, as a `$` field binds a `$T`. Claude's first ruling; Jimmy: "your rulings
  seem right". Where: Type System chapters 2 and 4.
- **FM** `for &v` over a map is legal and visits each value by reference in key order.
  Claude's first ruling; Jimmy: "your rulings seem right". Where: Semantics chapter 5.
- **SC** A supplier written by clauses refines its output at a clause's tag pattern as a
  `match` arm does: `fn @Supply p, (Pad.Num(..)) = @ 7` checks. The checker refusing it is
  a bug. Claude's first ruling; Jimmy: "your rulings seem right". Where: Type System
  chapter 5, "Refinement at tag matches".
- **IE** An `if` needs its `else`, a building `if` included: `if loud:` and a block with
  no `else` is a checker error. Jimmy: "no helll no if needs else". Refused: an else-less
  building `if` that builds its arm or nothing.
- **ZD** A second block for a widget that takes no arguments is a name defined twice:
  `collision`, in both spellings (two bare `@Twice:` blocks, and `fn @Twice:` twice).
  Jimmy: "yeah this is a name defined twice". Refused: `unreachable`, whose fix line
  ("put it above the clause that covers it") makes no sense with no arguments to tell
  clauses apart.
- **HA** A hold above a building `match` may begin at the `match`'s plain result: a
  block's bindings are a group and may be cyclic, and line order decides positions only.
  Jimmy: "why wouldn't this be legal?" and "bindings are cyclic man".
- **LP** A pattern cannot test a dynamic's value: `Ui.Label($text=$"x")` as a pattern is a
  checker error. Binding the field, `$text=$t`, stays legal. Jimmy: "this is crazy wrong
  you can't match on a dynamic like this", and "correct" to the checker-error reading.
- **BP** A `for` or a `rebuild` as a plain function's value is a checker mismatch, as a
  building `match` there is; an interpolation lifts over `?` as any application does.
  Claude's first rulings, shown to Jimmy with their programs and not overruled.
- **IE2** (overrules IE above) A builder's `if` and a statement's `if` may stand without
  an `else`: a building `if` in a widget block builds its arm or nothing, and an `update`
  block's statement `if` changes something or does not. Only an `if` that is an
  expression needs its `else`. Jimmy: "Fine, builders and statements can have raw ifs.
  Damn. I didn't know widget blocks could have ifs guarding bindings but I guess they
  can. I was leaning too heavily on Haskell semantics where you have to do `when check $
  do`". With ruling TC, the places after an else-less building `if` count from what was
  built.

## 11 November 2026

- **LS** A local function that calls itself needs a signature line, as a top-level
  function does. Jimmy: "Self calling local function needs a signature why not?" Refused:
  proving that an unsigned one shrinks at the types it is used at. An unsigned local
  function that does not call itself is generalized as before.
- **QN** A qualified name needs no `use`: `Ui.Label` is reached by its module's name, and
  `use` is for writing a name bare. Jimmy: "I tend to agree—follow Rust". This overrules
  Semantics chapter 6's "only through `use`". Refused: an error for a qualified name with
  no `use`, with a `use` line added to every program that writes `Ui.Label`.
- **KY** An `update` block's `for` over a map binds `key`, as a widget's `for` does; that
  it does not today is a bug. Jimmy: "yes a bug".
- **EX** A method that needs one dictionary for a collection of the trait's type, or has
  no value of the type to read one from, cannot be called on `any ^Trait`: a checker
  error. Each packed value keeps its own dictionary. Claude's ruling (the fixer proposed
  the dictionary of an array's first element, an empty array a fault); told to Jimmy and
  not overruled.
- **NI** A trait asked at `[a]+` is solved by an impl for `[a]+`, never by the impl for
  `[a]`. With only `impl ^Sized [a]`, `Count ([[1, 2]] as [Nat]+)` is a `no-impl` error
  whose fix line says to write `impl ^Sized [a]+` or to pass the collection with
  `as [a]`. `^Eq`, `^Ord` and `^Show` at `[a]+` go by the collection's parts. A non-empty
  value still converts where a plain `[a]` is wanted. Jimmy: "I agree." Refused: falling
  back to the `[a]` impl for traits whose type is never a result. `[a]+` and
  `(K ==> V)+` have a target type of their own on both targets.
- **PG** A function that calls itself at a type that grows without end
  (`fn Depth value, n: 1 + Depth [value], (n -? 1)`) is a checker error. Jimmy: "error
  yes". Recursion at another type that reaches a finite set of types stays legal with a
  signature (Type System chapter 2). Refused: a second lowering with the types passed at
  run time.
- **NL** Named lenses (`&Acc: fn &Vm -> &Int`, Semantics chapter 5) and `Over` are out of
  this compiler's scope: the checker's `unsupported` error stands. Jimmy: "Out of scope I
  think". Lens paths inside an `update` block (`&_.regs.acc <= e`, `for &v <- &_`) are in
  scope and lowered.
