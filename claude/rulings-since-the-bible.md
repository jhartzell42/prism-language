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
- **NL** Named lenses (`&Acc: fn &Vm -> &Int`, Semantics chapter 5) are in scope: a named
  lens is a function from one reference to a reference. Jimmy: "It's just a function from
  references to references". `Over`, `Get`, `Set`, the `^Lens` trait and a lens as a
  value (`&.score.home` outside an `update` block, Type System chapter 3) are out of this
  compiler's scope: the checker's `unsupported` error stands for them. Jimmy: "yes that's
  out of scope".
- **ME** A named lens may not end at a map key, nor step through one: a checker error
  (`lens-map-key`). Inside one `update`
  statement the checker knows by the container's type that a path ends at a key (its
  part is `?V`, and setting `nil` removes the key); behind a named lens's signature
  `&?V` would be either a real location or an entry. Jimmy: "A & can be a real location
  or a map entry ... I think that that's redonkulous actually, and that entries should be
  Entry<K,V> and have a separate operator to set them". The entry type and its operator
  are a change to the language, sent to the Bible; this compiler keeps the `update`
  block's own map index paths as Semantics chapter 5 has them, and a named lens may end at
  an array's index.

## 12 November 2026

- **OU** An element has an output: the prelude's `Answer c` is `Output c`, and the
  constructor ruling AN generates for each tag ends in `Output`: `Ui.Button.Output click`,
  `Fs.Contents.Output (~ c)` (AN's form, the last segment renamed). Jimmy: "An element has
  an output, a request has a response, and a query has an answer. Rename the prelude's
  `Answer c` and each tag's `Ui.Button.Answer click` to `Output c` and
  `Ui.Button.Output click`. That frees `Answer` for a document's `type Answer r`, which the
  hosting code spells `Told` until the rename lands." The emitted crate's enum is
  `crate::Output`, and prism_rtl's `AnswerOf` is `OutputOf`. Refused: keeping `Answer`
  for an element, which leaves a document's query without its word.
- **OR** A map's values have one type, and no closed `emit` row widens to another: a
  widget meant to stand beside others of other rows declares an open row, `emit r | ...`,
  as Type System chapter 2's `@Offline` does. Chess's `@BoardPlayer` and `@LinePlayer`,
  held in one map of seats, say `emit r | Pane.* | Gui.* | Seat.* | Play.*` and
  `emit r | Console.* | Seat.* | Play.*`. Jimmy: "I'm sold on your recommendation".
  Refused: row subtyping where a value meets an expected type; signatures open by default.
- **EN** (replaces ME above) A map's entry is a type of its own, `Entry K V`, with an
  operator of its own. Jimmy, 11 November: "A & can be a real location or a map entry.
  That's a runtime distinction, isn't it? I think that that's redonkulous actually, and
  that entries should be Entry<K,V> and have a separate operator to set them"; 12
  November: the operator is `<:` (`<-` binds), and `<: nil` removes the key.
  - On a map, `&m[k]` is an `Entry K V`, a prelude type; on an array, `&xs[i]` stays an
    ordinary reference.
  - `e <: v` takes a `?V`: `&_.board[sq] <: ? piece` puts the value,
    `&_.board[from] <: nil` removes the key. A read of an entry is the `?V` that `m[k]`
    is.
  - `<=` on an entry is the checker's `set-entry`, whose fix writes the statement with
    `<:`; `<:` on anything else a reference names is `put-location`, whose fix writes it
    with `<=`.
  - An entry has no fields and no index: `&_.board[sq].color` is an ordinary type error
    ("an `Entry` has no field `color`") in a named lens, a function and an `update`
    alike. A field of a stored value is changed by reading the value, changing it, and
    putting it back.
  - Named lenses are not special: a lens may take or give back an `Entry`
    (`&Acc: fn &(Text ==> Int) -> Entry Text Int`). ME's `lens-map-key` and its test are
    gone.
  - The coordinator's ruling on the lexer, 12 November: `<:` is one token by longest
    match, and in a trait's member position the parser reads it as the method `<` and its
    signature's colon (`<: fn a, a -> Bool` in `^Ord`). Refused: a lexer that splits `<:`
    by context, which no other operator needs.
  - (overrules the bullet above) Jimmy, 12 November 18:12: "make someone put a space".
    A trait declares its method `<` as `< :`, with a space (`< : fn a, a -> Bool` in the
    prelude's `^Ord`), and an unspaced `<:` in a trait's member position is a parse error
    whose message says to write `< :`. The parser has no special case for it. Refused:
    reading `<:` there as `<` and its colon.
- **HD** Holding braces and lifting constructors: every application lifts a sigiled
  argument, a constructor as a function does, and `{ !e }` is the only way a sigiled
  value fills a type variable unlifted (`Requested {!asked}` is the bundle `Heard !Asked`;
  `Push events, {!e}` pushes the event). Constructors lift over `$` and `!` only; `?`,
  `~` and `@` go in whole. Tuple and array literals keep their parts whole; a declared
  sigiled parameter or field takes the live argument as it is; braces that hold nothing
  are `hold-meaningless`; patterns take no braces. Jimmy: "There should be an operator
  which disables lifting ON THE SIGILED ITEM for both that just lets it be in a type
  variable... It's mandatory for both constructors and functions." Replaces Type System
  ch. 2's "A type-parameter field takes its argument whole" and LC's second half.
- **DB** Every literal doubles its bracket: a map is `{{k: v}}`, the empty map `{{}}`;
  single braces hold. A code template's placeholder is `<<expr>>` and its brace block
  runs from `<<` to `>>` alone on lines, with `\<` and `\\` as escapes; text keeps
  `"{x}"`. The old forms are `single-brace-map` and `brace-placeholder`. The HTML-like
  template notation, out of this compiler, switches the same way (for the Bible).
- **CM** A name's case says what it holds (Language ch. 10): an uppercase name is a
  function, constructor, type or trait; a lowercase one a value; `case-mark` otherwise.
  Jimmy: "why would we allow an uppercase let name unless it was a function?"
- **NC** No C++: the C++ target is dropped (emitter, runtime, worlds, seed, the target
  layer and `--target`); its shared lowering fixes are kept. Jimmy: "thoroughly drop C++
  everywhere, I think it's served its purpose".
- **LB** The standard library's directory is `library/`; `Prelude` names only the module
  every file uses without an import. Jimmy: "Prelude strictly speaking means things you
  don't have to import."
- **GF** Type-system features are generic, never shaped to the case that motivated them.
  Jimmy, of a rule for a trait's own role: "far too ad hoc. It makes Role special, and we
  don't want special." Rulings PK to VO below follow it.
- **PK** Promoted constructors: a choice whose cases carry no payload is also a kind,
  its cases types of that kind; a parameter that takes a case declares its kind, `(r:
  Access)` or `(r: Role d)` (an associated type is a kind like any type), and a parameter
  with no kind is a type, so a case there is `kind`; a choice with a payload is no kind.
  Jimmy: "good"; and, 13 November, of whether the kind may be left to inference: "it must
  declare its kind".
- **AP** Associated types take parameters of any kind, given by equations on the
  parameters' constructors, exhaustive over a closed kind, abstract under a variable.
  Jimmy: "good".
- **FA** `forall` is the explicit universal: it scopes a signature's variables over its
  body (for `<<a>>`), gives variables kinds, and later allows higher rank. `any` stays
  the existential of Type System ch. 3, now over any type variable.
- **VO** A promoted case stays an ordinary value of its choice; a type index is passed
  explicitly (`Keyed<<Writer>> 3`) and must match the signature, and the standard
  library's `ValueOf: forall (r: k). k` gives a promoted case's value
  (`ValueOf<<Writer>>`). Refused: a singleton type `Is r` whose literal takes its type
  from the expected type, which makes every case polymorphic. Jimmy: "this is nicer
  really". Its replacement for the singleton, which this entry had lost: `Fixed r`, whose
  value is written with its type, `Fixed<<Writer>>`. Jimmy, 13 November: "we came up with
  a replacement, Fixed r, with a value Fixed<<WhateverChoice>>". A clause or arm that
  matches `Fixed<<Raiser>>` knows `r` is `Raiser` in its body. Jimmy: "indeed!".
- **IN** No injectivity for `State`: a game's state may belong to several documents (a
  file, the network). An ambiguous call names its type, `Step<<d>>`. Jimmy: "Chess app
  should allow saving games in a file and doing them over a network, actually".
- **AF** All the design form's features are built, generically: indexed tag families
  with `_` over a closed kind, `any` over type variables, row parameters on aliases and
  associated types, trait bounds on a trait's parameter, `where` heads of any type,
  associated rows, the no-override rule, `derive ... via` as an opted-in generic impl,
  and partial application. Jimmy: "Do all the features including ones you tried to
  backpedal".
- **TC2** This conversation outranks the books where they differ; the Bible hears of
  every change by channel note. Jimmy: "This conversation beats the books."
- **SD** A sub-trait may give a default for a method of a trait above it. An impl gets its
  own method, else the default of the lowest rung it implements that gives one; two rungs
  that do not extend each other both giving one is `ambiguous` at the impl, which writes
  the method (the diamond: Claude's first ruling). Jimmy: "A subtrait can define a default
  impl of a method above it, why not?" Replaces the "no-override rule" Claude had filed
  under AF, which was never Jimmy's.
- **RS** `^Document`'s `@Request` and `@Query` take the state they answer against, `State
  d`, which `^Document` declares with the default `unit` (a database has none);
  `^StateDocument` gives them defaults lifted from `Step` and `Read`. Jimmy: "b seems
  right".
- **IF** The file implementation is `@InFile`. Jimmy: "it's 'in file' buckaroo".
- **EI** An impl with nothing to write is its header alone, `impl ^Named Pen`, with no
  colon and no block (Claude's call, 13 November, for an impl that takes every default).
- **OS** A file's own declaration shadows a name it reaches through `use X.*`; two
  modules reached by `use` that both declare a name still collide at the use, and the
  prelude against a file's own still collides (Semantics ch. 6, neg/collision). Jimmy, 13
  November, on chess's `Move` against `^PerfectInformationGame`'s: "a and c both just do
  both". So also **CR**: chess names its own move `ChessMove` (Jimmy: "good"). The prelude
  is covered too: a file's own `Len` shadows `Prelude.Len` ("prelude covered too"), so
  the file document's request keeps the book's name, `Replace [U8]`.
- **UA** `use` renames what it reaches with `as`: `use Fish.Count as FishCount`. Jimmy,
  13 November: "Use can support as".
- **RR** A parameter used only after `|` in an `emit` row is a row and declares no kind:
  `type ChessSeats rest = record: $player: (Color ==> $@(@unit emit Seat.* ChessGame
  Player | rest))`. That position holds nothing else, unlike a case. Jimmy, 13 November:
  "sure let's do a".
- **OP** A place has one role: the seat and the element at a place are one existential,
  `type Guest d = any (r: Role d): ((Fixed r, SeatId d r, Elem (Seat.* d r)))`, and the
  supplier is given `any (r: Role d): ((Fixed r, SeatId d r, Element (Seat.* d r) o))`, so
  opening it tells the arm the role of all three. Jimmy, 13 November: "a indeed".
  Claude's calls under it: a `Fixed s` matched against `Fixed<<r>>`, `r` a variable of
  the function whose `Fixed r` parameter is in scope, compares the two and narrows `s` to
  `r` (VO for a variable; library `?ResponseTo`); a tag arm narrows the matched element
  to its family's row (library `@HostRole`); `@Serve` takes the connection's role from its
  caller.
- **RT** A row may hold several variables, each decided by an argument: `@HostDocument ...
  emit lower | rest` (documents book ch. 5). Jimmy, 13 November: "a".
- **DE** A derive may ask equalities of associated types, `derive ^StateDocument g via
  ^PerfectInformationGame g where State g = g, Request g Player = Move g, ...` (Claude's
  call, 13 November, from the chess agent's report that the derive's `Step` asked nothing
  of the game's `impl ^Document`).
