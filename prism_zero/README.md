# `prism_zero`: compiler zero

*Written by Claude, 31 October 2026, on the `claude-self-hosting` branch. Scaffolding
(docs/architecture.md; D2): it runs `prismc` once, to compile `prismc`, and is deleted when
stage 3 lands. Subject to change: Jimmy rewrites it by hand, and an implementer reading it
uses their own discretion. The books in `prism-notes/claude/prism/` are the specification;
where this crate and a book disagree, the book wins, unless the disagreement is listed
below.*

A dynamic interpreter of Prism over `prism_rt`, with no type checker (D3): lifting and
trait dispatch are decided at each application from runtime values and the sigils on
names and signatures. `src/lib.rs` describes the modules; each cites the book rules it
keeps.

## Usage

```
prism_zero run <files.prism...> [-- args...]       # @main under the CLI World (Fs, Process, Time)
prism_zero test <files.prism...> [scripts.prismtest...]
prism_zero check <files.prism...>                  # parse and resolve only
```

- `run`: exactly one file has no `module` line; its `@main` (any stack starting `@`) is
  the root. Exit codes: `Process.Exit`'s; 0 when nothing can cause another tick; 1 for a
  poisoned root or a failed tick; 2 when the program fails to load, resolve its root, or
  pass the root row check.
- `test`: each script runs under the Test World and its transcript goes to standard
  output. With no script named, each `X.prism` runs the `X.prismtest` beside it, or else
  its own `test` and `check` declarations. A script's tests are declarations of the module
  in the `.prism` of its name, else of the root file. An `X.expected` beside a script is
  compared byte for byte (its leading `# ` comment lines ignored). Exit: 0 pass, 1 a check
  failed or the transcript differs, 2 the script or program did not load.
- `check`: load and resolve; 0, or 2 with `file:line:col: message` diagnostics.

`cargo test -p prism_zero` runs `tests/conformance/` (one Rust test walks the directory),
the examples' scripts, and the command tests.

## The prelude

`prelude/prelude.prism` is Semantics Appendix B. A signature with clauses is written in
Prism (`Each`, `Filter`, `Sum`, `Max`, `Min`, `Or`, `~ParseNat`, `@Once`, `@AfterFirst`,
`@Numbered`); a signature with none is a Rust builtin in `src/builtins.rs`, registered by
that name and lifted by that signature. `prelude/{ui,fs,process,time,test,gui}.prism` are
the libraries whose families the Worlds answer, supplied beside the prelude; a program's
own file of that module name, or declaring a tag of that family, replaces one (the chess
window library declares `Ui` and `Gui`).

## Picks

The places the books leave to an implementation, or that a checker would decide, and
what compiler zero does. Each is Claude's, for Jimmy to overturn.

- **LA Layers.** A parameter's declared type says what it takes: a sigiled type, a tag's
  field, a widget function's parameter, and a constructor's type-parameter field take the
  argument whole; a bare type variable lifts over time and keeps a `?`; a concrete plain
  type lifts over time and peels `?`/`~` (L5, wrapping the result once). With no
  signature, a parameter pattern's sigil decides.
- **ML Map literals lift** over a `$` or `!` among their entries (`{$next: Added} on
  !add`, Semantics ch. 4's `@labels`); tuples and arrays take components whole.
- **LM A lifted `match`/`if`** is one node over the scrutinee and every handle its arms
  read (doubt EL's first ruling); an arm that is `never` fires nothing. Arms reading two
  or more events (EL's gates) are refused with a diagnostic.
- **RK What a `rebuild` or `for` hands back** is read off the binder's sigils (`$!`,
  `$$`, `!`, `$`, `ignore`); for a `$` binder or a builder on a block's last line, off the
  rows' last line (`@ x on !e` an event, `@ $x` a dynamic).
- **PF A plain `for` gathers its rows** as Semantics ch. 4's table says (events into
  `!(K ==> T)`, dynamics into `$(K ==> T)`); `prism_rt`'s plain `for` returns the raw map.
- **BG Binding groups.** Positional statements run in line order (positions follow the
  page); `let`s, holds' initial values and local functions are evaluated on first use;
  holds' `then`s are wired after every line. An event read above its binding line is a
  forward event; a dynamic read above its line is refused (prism_rt has no forward
  dynamic; no book program does it).
- **CP Cycle points without continuations.** `@Run` runs the unbounded function to its
  end once, counting cycle points (each call of an unbounded function from an unbounded
  function, each `while` turn inside one), and reports `More` for that many cycles, then
  `Done`. Same transcript as the canonical rule, cost front-loaded; past a budget the run
  is cut and resumed with twice the budget next cycle, so a job that never ends keeps
  `pending` set and every tick ends.
- **LT Literals** are `Int` until a signature, a record field, or an `&` part that held a
  `Nat` says `Nat` (converted at the call, at construction, or at the set).
- **NQ `-?` on two non-negative integers is `Nat`'s**, `nil` below zero, since an `Int`
  that is not negative is nearly always a literal a checker would have typed `Nat`.
- **MD `mod`** gives a plain number (floor modulo); `x mod 0` is `x`. `/` gives `?` on
  integers, IEEE on `Float`.
- **FL `Float` (D14)** is a payload-free choice whose constructor is the float's
  transcript spelling (`280.0`, `nan`): `prism_rt::Value` has no float, and prism_zero
  may not change prism_rt. `pod::print` then prints it as Type System ch. 2 spells it;
  arithmetic, equality and order decode it; an integer literal meeting a float is a float.
- **ER `Error(..)` is the failed `~T`**, one value for both (as `prism_rt::pod` reads it
  back); a field of a failed `~` passes the `Error` through unless the `Error` has that
  field.
- **TG Tags need no `use`.** `Ui.Button` resolves wherever written; book programs name
  `Ui` and `Fs` tags with only an `emit` line. Qualified declarations (`Birds.Call`) need
  the `use`.
- **CO Collisions are strict, the prelude included**: a file's own declaration of a
  prelude name is an error at the (implicit) use. `?Step` in the `Normalize` test is
  therefore `?Reduce`.
- **UB Uppercase binders.** A bare uppercase pattern name is a constructor only if it
  resolves to one; otherwise it binds a function value (`fn Kept xs, P`).
- **DV Derived traits are always available** structurally; nothing checks that the type
  wrote `derive` (a checker's job).
- **GU Guards (D11, D15)**: a failed guard under `~` gives `Error(kind=Prelude.Unmet,
  message=<the guard's source text>)`.
- **UW `Prelude.Unwrap`** is answered by the CLI World (`src/world.rs`): `~ x` with `x`
  a tick later; the first `Error` poisons the innermost generation or row (or the root)
  and reports an `Exception` to the nearest `Prelude.Exceptions` place outside it, or
  prints and exits 1 at the root. The Test World (prism_rt's) does not answer it.
- **RC The root row check** reads the root's `emit` line, or for the implied `emit
  Element` the families of the tags the reachable modules name; host documents'
  subtraction is not done.
- **PC Prompt cycles** are caught only at run time, as a definition needed while it is
  evaluated; a cycle through events is not found statically.
- **CT Compute ticks' transcript.** `prism_zero test` rewrites `prism_rt`'s `compute until
  idle: k` and its empty ticks into Conformance ch. 1's `compute until idle: k ticks, n
  to m` with the empty ticks dropped.
- **TP Code templates** evaluate to text (runs, placeholders, brace blocks, calls); the
  POD form of a templated language (`templatetext`, `templateplaceholder`) is not built.

## Gaps in prism_rt that prism_zero meets

- No `Float` in `Value` or in `pod::read` (scripts cannot write `280.0`); no float in
  the Test World's `Press` (it sends `Press(x=Int, y=Int)`).
- The Test World has no `click element path` step (it has `click at path`, which falls
  back to the scene), no `answer` policy (the seeded deck), and prints `compute until
  idle: k` with every tick (fixed up here, CT).
- `Level` does not project elements nested inside a live field (`Ui.Grid(children=..)`
  prints raw handles), and paths and the transcript ignore a program's `impl ^Show`.
- Outputs that are bundles (`Gui.Window of Element ((WindowId, !WindowInput))`) are not
  supported (`OutputKind` has no bundle).

## Doubts

- The heuristics RK and LM read the page where a checker reads types; a program that
  defeats them gets a wrong result kind rather than an error.
- NQ changes the meaning of `-?` on an `Int` that a checker typed `Int`; no book program
  relies on `Int`'s `-?`.
