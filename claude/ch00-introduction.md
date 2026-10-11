# Introduction

This book is a working reference for Prism, a functional reactive language, and for the programs that implement it. It is an internal document. Its readers are its author and the collaborators the author hands it to, and its job is memory: the design is too big to hold in one head, and every question of the form "what does the language do when…" should have an answer written down in one place, with the reason next to it. It is also the source the real specification and documentation will be written from later, by hand. That later writing will be a separate piece of work in a different voice, and nothing here is meant to be lifted out and published as it stands.

The book therefore favors completeness over polish. A chapter may repeat a rule that another chapter states, because the reader who arrives through a search should not have to follow a chain of references. A chapter may argue a decision at length, because the argument is what lets a later reader tell a choice from an accident. And where the design has taken a choice on thin evidence, the chapter says what the evidence is and what would change it, on the merits, in the section the choice lives in.

## The three parts

The first part is the language. Chapter 1 says why it exists and what it refuses to have. Chapter 2 shows programs working, occurrence by occurrence, before anything is defined. Chapters 3 to 5 define the language: the lexical structure, values and expressions; events, dynamics and time; widgets and switching. Chapters 6 and 7 define the boundary: elements and interception, and the Worlds that stand outside every program. Chapter 8 works larger use cases through the whole machine, and chapter 9 says what goes wrong and how it is reported.

The second part is the implementations. Chapter 10 is the conformance suite, which is the contract between them. Chapters 11 to 13 are the Rust implementation: the runtime, the frontend, and the Worlds. Chapter 14 is the Haskell implementation, which reads the same chapters independently and runs the typed AST on Reflex, so that a disagreement between the two points at a sentence of the book.

The third part is the plan and what lies beyond it. Chapter 15 cuts the work into milestones that a model or a person can be handed one at a time. Chapter 16 holds the parts of the design that have had less effort. Chapter 17 collects the reference tables: the lexical summary, the prelude, the tag declarations.

## How to read it

A first read goes in order through chapters 1 to 7. A reader who wants only the model can stop after chapter 5 and one World. A reader implementing something starts from the chapter for that piece, reads chapter 3 or 4 as needed for the sentences it implements, and treats chapter 10's goldens as the arbiter. Chapter 17 is for lookup. The summaries at the end of the book give each chapter in a page, and are the quickest way to find where an idea lives.

## Conventions

Code in Prism uses the sigils the language is built on: `!` for events, `$` for dynamics, `@` for widgets and `?` for optionals. Chapter 3 defines them all; chapter 2 uses them before it does, deliberately, so that each one is first met doing work.

Some sections are marked as under-planned. That means they have had less effort, and nothing more: the design in them is as committed as anywhere else, the questions in them are listed with their merits, and a reader is more likely to find a gap there than elsewhere. Chapter 16 collects the largest of these. Where a mechanism's spelling is tentative, the section says why on the merits, for instance that the interaction of a comparison operator with the hold arrow has not been worked through, and says what would settle it.

The book does not track its own history. A chapter describes the design as it is. The one exception is the implementation chapters, which describe the current code where the plan changes it.

## Summaries

What follows are short summaries of each chapter, as an index to the argument.

### 1. A language whose only subject is time

The chapter opens on an afternoon every reader has lived: a "show archived" checkbox that unticks itself one time in five, because a setter notified its observers one line before it wrote its field, and a re-render read the model back half-updated. The thesis follows from that bug. The many bugs of interactive programs are one bug, a value whose set of writers is not a fact about its type. Functional reactive programming makes the writers a fact with two types, a value that happens and a value that holds, tied by one law: a held value changes only through the events named in its definition, and only when the moment ends. Prism is a language that keeps that law and no other, and the list of what it refuses is the evidence.

The chapter first names the four faces of the bug (uncontrolled mutation, action at a distance, "who can change this?", order within a moment) and shows one cause under all four. It then asks for a bigger version of `const`: a policy on who may write and when. A policy removes sentences from a language, and a library can only remove what its host lets it name, so the policy has to be a language. Each paradigm gets a picture, the constraint it adds, and the place its own documentation shows the problem returning: callbacks, objects and the Observer pattern's "make sure Subject is self-consistent", React's tearing and Suspense, signals as the behavior half of FRP found independently, Rx as the event half without behaviors as a law, Elm as the friendly proof that a language-level mutation policy works, Haskell's `dFoo`/`eFoo` naming and open door to `IO`, and the synchronous languages as allies built for fixed structure.

### 2. Worked examples

The chapter opens with the claim that the programs are the language and the walkthroughs are the laws applied to them. It offers fourteen programs, from a counter to a shell that hosts other programs, each followed by an occurrence-by-occurrence account of what the World sends, what fires in dependency order, what commits, and what the element list looks like afterwards. A reader who has read the specification can predict each walkthrough and check; a reader coming cold meets every later rule here first.

The conventions fix the vocabulary for the book: occurrences numbered from zero with zero as the build, inputs addressed by path and simultaneous, the only promised order being dependency order, and a small shared set of declarations under `Gui`, `Ui`, `Fs`, `Time` and `Process`, each declared before first use. The first programs build up the core: the counter, a text input with a cursor, a todo list driven by `for` over a keyed map, a button that relabels itself (a dynamic field lifted into `dyn` with a fresh slot per change), a file viewer over a state element, and a stopwatch reading `Clock` as state.

### 3. The language: sigils, lexical structure, values and types, expressions

The chapter opens on `total = price * quantity`, a line where you cannot tell without scrolling whether it ran once or on every keystroke, and on the Reflex habit of naming variables `dPrice` and `eQuantity` against the library author's advice. The thesis: a Prism program is a set of equations between things that are true, things that happen and things that exist, and the sigils on the names say which before a type checker runs. The chapter covers the part of the language that holds still, written so two parsers built from the text agree on every program.

Sigils come first. `!` is an event, `$` a dynamic, `@` a widget, `?` an optional, and bare is plain; they stack leftmost outermost, functor values nest inside plain aggregates, and a function's name takes the sigil of what it returns. The section argues against the alternatives: optional sigils would leave the boundary between styled and unstyled code unreadable, and inferred sigils shown by an editor would make review depend on tooling. The lexical section then goes down to the character, because the conformance suite fails the day the Rust and Haskell lexers disagree: comments, identifiers, keywords, literals (integer-spelled literals are `Nat | Int` by inference defaulting to `Int`, decimal-spelled are `Float`, `-1` is an `Int`), operators, the offside rule with `:` as block opener and more-indented lines continuing a statement, and paren-less calls where commas belong to the outermost call on the line.

### 4. Events, dynamics, and time

The chapter opens by saying that everything a Prism program does happens in an occurrence, and an occurrence is a transaction with a before, an after and no visible inside. Every interactive program asks three questions over and over: what is true now, what just happened, and what exists. Most languages answer with a variable, a callback and an object that disagree about time; Prism answers with three kinds of value whose sigils name their relation to one clock. The claim is that a small set of laws suffices, and the tag law alone turns a whole class of cyclic programs into legal ones.

Events come first, introduced by the handler that ran at the wrong time. An event is a thing that occurred, not an object with `.preventDefault()`. Simultaneity is defined, not raced: a program never observes two events in order inside an occurrence. `!never` is the identity for merge, and there is no `!always`. Merge is leftmost, with the reasoning for why the left wins. `on` gives an event a dynamic's value from before the occurrence. Lifting over one event fires exactly when that event fires, and `!Filter` is the one way to drop firings. `!Switch` gives a parent one event across every build of a `dyn`. Events are values and need no subscription or disposal.

### 5. Widgets and switching

The chapter opens on the subtree that was removed while its listener lived on, and on the component remounted by a parent's render that fetched twice and let an old response overwrite a new one. Both bugs treat something with a lifetime as if it had none. The thesis: a widget is the one thing in Prism with a lifetime, so state, IO, identity, cycles and subscriptions are all owned by one, and three constructs (`dyn`, `for`, and lifting over `@`) let a program change its own structure while two primitives, `!Switch` and `$Join`, bring results across the boundary.

The first part separates blueprint from widget: a blueprint is a value, a widget is what one bind builds from it, with identity, a path, holds, placeholders, slots and children, and a widget is never a value. Building is the only doing; the World performs everything. The button is the archetype because it has every property at once, and the word stops meaning GUI once a socket is seen to have them too. Widget blocks are where "one way to change anything, written next to the thing" becomes syntax: holds appear only there, bindings are mutually recursive through delayed edges, every binding is marked build-time or occurrence-time, results can be bundles, widgets can be arguments, `@main` takes nothing (arguments and environment are state elements), and `@started` is a pure `dyn` over a constant that fires once at build.

### 6. Elements and interception

The chapter opens on the component that fetched on mount, fetched again on re-render, and showed a stale avatar after a race the test suite could not reach. The thesis: an element is the only thing a program can say to the outside and the only thing the outside can address in reply. The chapter defines that channel, the rule that nothing it asks for is answered in the occurrence it asked, the two identities of a slot, the element list as an ordinary dynamic, and `intercept`, the primitive that lets a widget stand where the World stands so that toolkits, sandboxes, caches and mocks are Prism code.

Tags declare elements: a name, plain fields, and any subset of `requests`, `responds` and `state`. A dozen declarations across GUI, files, network, time and process show why each part is what it is, and four rules follow from elements being plain data. `@Element` makes a slot and returns what the tag declares. The creation rule is quoted in full and argued: an `Fs.Load` that read on creation would save a line and cost the three things `@started` exists for. Its complement is that state has a value in its creation occurrence, an observation and not a firing. A slot has two identities: a path for conversations (bind positions, `dyn` generations, `for` keys, never reused) and a value for facts (equal state elements share one dynamic).

### 7. Worlds

The chapter opens on the context object handed over at startup, stashed in a global, read after teardown, and eventually discovered to be the render loop, the clock and the filesystem at once. The thesis: a program cannot write a byte, read a clock or hand anyone a callback, so something outside must do all of that, and the design of the boundary lies in how little that thing has to know. A World is the outermost interceptor, written in the host language, the owner of the loop and the only starter of occurrences, driving the runtime through one synchronous call per occurrence with plain data both ways.

The run call is given whole. In: responses by path (at most one per path per occurrence), pushes by element value, and an oracle answering state from the snapshot as the occurrence opened. Out: the element list, request firings per slot, the result and errors. The list is a description the World reconciles to, never a trigger. Nothing else crosses, and occurrence zero builds `@main` with the oracle live. The snapshot discipline comes next, opened by the Binance and Kubernetes snapshot-plus-changes procedure: the World keeps the last value supplied for every state element, the oracle cannot fail because unavailability is a case of the state type, and the World stitches native reads and watches before anything reaches the snapshot. A directory listing is traced through the race, and events-only and dynamics-across designs are argued against.

### 8. Use cases in depth

The chapter opens on eight hard moments: the file that changes on disk as the user presses undo, the fill that lands with the book update that would have re-quoted it, the emergency stop that arrives with the tick, the member who leaves a room as someone speaks to it, the source that changes as its compile finishes, the window that quits under the click that raised it, the jump pressed on a frame boundary, the time range that changes as the old range's data arrives. The thesis: these programs are not toys, and each one, taken far enough, reaches a moment where two things happen in one instant and the answer is a line of the program.

Each deep dive follows one frame: the element vocabulary, the structure of the program, the hard moment walked occurrence by occurrence, what the goals buy, and where the language is awkward or wrong. The text editor has exactly the states a framework claims to manage (text, cursor, undo, dirty flag, search) and the outside events it does not, and giving each state one defining line decides which write wins. The market-data consumer shows what "act on exactly the state you saw" means and treats latency as an implementation question. The control loop gets a synchronous-language reading and admits where it is wrong.

### 9. Errors

The chapter opens on the stack trace that ends in a map lookup: a key present a moment ago, `null`, a dereference, a guard added forever. The thesis: a program's own failures are values, and the failures the language cannot make into values do the least damage the transaction allows. The first half is a discipline the type system enforces; the second covers logic errors and broken contracts, such as a cycle that exists only once two widgets are wired together at run time.

The first content section sorts what can fail: missing files, closed sockets, division by zero and out-of-range indexes are cases of a type, and leaving their arm out of a `match` is a compile error. The occurrence then becomes the unit of failure, opened by the half-updated model every reactive system has shown and by Spider's single exception for causality loops. A cycle, overflow or protocol error aborts the occurrence, and the transaction drops its buffered writes. Containment at a construction boundary is the one smaller unit: a `dyn` whose new generation fails to build does not fire, keeps the tree consistent, and reports through `errors`. Containment to an owning subtree, quarantine, answers the dashboard with one broken panel among twelve: the World passes back the paths it wants quarantined on the next run call.

### 10. Conformance and testing

The chapter opens on the white-box test that read a private field, broke on a rename, and was fixed by exporting the field. The thesis: a specification only one implementation can check describes that implementation, so the language is tested from the one place two implementations must look identical, the World's side of the boundary. Every test sends plain data to elements and reads plain data back, never naming a node, hold or runtime object.

The Test World is a World with no effects, written once per host language on the world core, that accepts every tag and answers to a script. Everything it sees is printed turn by turn in one canonical transcript form that the harness compares and the differential test diffs. The script language follows: tests as top-level statements, element patterns (named fields with `..`, `Ns.*(..)`, `ignore`), and every step (`expect`, `expect_top`, `expect_none`, `expect_top_none`, `expect_count`, `send`, `click`, `with`, `push`, `returns`, `fired`, `quiet`, `requested`, `unrequested`, `errors`), each with a script. Depth inside containers uses the GUI library's `children` field and `ChildInput` convention, and turns define when checks run: always between turns, against committed state.

### 11. The Rust runtime

The chapter opens on the list every runtime author knows: a scheduler, dirty tracking, re-entrancy guards, deferred effects, an unsubscribe story with its leaks. The thesis: the Prism runtime needs none of them. It holds events, behaviors and dynamics as nodes with heights, runs one occurrence per call by popping heights in order, commits every hold at the end, and owns the tree of widget nodes that is the program's whole state. A height-ordered action map, a commit list, weak subscriber lists and one synchronous call are the whole engine.

The first half describes `prism_runtime` as it stands on branch `on-fire-repros`, with 38 tests passing and no boundary yet. It covers values (`AnyValue` as a 24-byte enum behind `Value<T>`), the runtime core loop, lazy events that disconnect when unheld, behaviors as the pull half with the only dirty tracking, dynamics where a hold's change event is its input event, widgets with `PolymorphicWidget` so a `dyn` can rebuild into another kind, and the test backend that reads behaviors directly. The 38 tests are mapped to the laws they pin, and two occurrences are traced through the code.

### 12. The Rust language implementation

The chapter opens on the compiler for a language that runs, whose back half was a fetch-execute loop, and on the reactive framework built over it that became a second runtime fighting the first for control of time. The thesis: a Prism compiler does not emit code that computes. It emits code that, run once, calls the runtime's constructors in order with the right captures, and the resulting graph does the computing. The frontend decides what every line means and writes it into one tree, the typed AST; a lint checks the tree after every pass; a closure compiler walks it once.

The pipeline is laid out and then each stage in order. The lexer and layout pass produce tokens with spans, implement the offside rule, treat inline `:` as a plain token, and keep only a column stack, bracket depth and a whitespace flag. The parser builds a parse tree from a grammar sketch, records spans, and leaves meaning undecided. The resolver handles scopes, qualified names and namespaces, intrinsics, dependency analysis into groups, and placement of placeholders for recursive groups.

### 13. Rust Worlds and the world core

The chapter opens on the year the framework's context object became a door everything walked through, until a test could not run without a browser. The thesis: a World is the only part of a Prism system with effects, written in Rust against one synchronous call, and everything a World must get right that is not specific to a screen, terminal or socket is written once in a crate, `prism_world`, so a World is that crate plus a handful of components.

The chapter first lists what the crate refuses to contain, each with its reason, then gives the crate as code: the types, the six-method `Component` trait with each method traced to an obligation, registration and the `World` struct, the turn loop (pushes applied after the first occurrence commits, re-queued on failure, `Defer` occurrences with no new pushes), `DeferTable` keyed by path with generations stripped, snapshot and pushes, slot tracking with discarded late results, `Unsupported`, error reporting, and a harness that drives a runtime from any event loop.

### 14. The Haskell implementation

The chapter opens on handing a specification to a second person who built something that passed every test and still disagreed on the first untested program, because both had read one sentence differently. The thesis: a specification with one implementation describes that implementation, so the Haskell implementation is a second, independent reading of the language chapters, in the language whose FRP library supplied most of those chapters' semantics. It shares nothing with the Rust side but the typed-AST text form and the transcript format.

The first decision is to interpret, not compile: the typed AST is interpreted at runtime against Reflex, never translated to Haskell source for GHC, so there is no second compiler to be wrong in. The frontend is a separate lexer, parser and elaborator written from the book and the goldens, whose output must match the Rust frontend's text form character for character, with `diff` as its test.

### 15. Plan

The chapter opens on the plan that was a list of features, where the first week went to choosing a starting point because every feature needed a piece of every other. The thesis: cut milestones along the lines the design already has (the run call, the typed AST, the transcript) so each milestone can be handed to a model with the book and the previous code and come back with work judged by named tests and matching transcripts. Eleven milestones run in two tracks that share no code and meet when the compiled counter's transcript matches the hand-written one byte for byte.

Each milestone follows one frame: what it waits on, the work in steps, a done criterion, what it proves, and its risks. Milestone 1 makes the runtime single-threaded and transactional, with `Rc`/`RefCell` first and alone, a re-entry flag reporting `Cycle` instead of a double borrow, and `Dynamic::join`. Milestone 2 adds the boundary: element values, tags, slots, paths as values in tree order and never reused, and the run call, proven by a harness printing the counter's transcript. Milestone 3 adds `InterceptWidget`, state supply and `for`, with the risk that a supplied widget built mid-grandchild must sit under the right parent.

### 16. Future work and research

The chapter opens with its discipline: nothing here is part of the specification, and nothing here may change a transcript. Every item is either a representation or scheduling choice behind fixed semantics, or an extension that adds programs without changing what existing ones mean. Each item gives the afternoon that makes someone want it, its design as far as it goes, what it must leave alone, and the fact that would make it worth starting, and the test every item must pass first is that the differential test still comes out empty.

Values representation is the longest item, opened by a profile dominated by clones of a 24-byte enum. The design is one machine word with three tag bits, full-width integers kept, heap objects with a count and a static descriptor, records storing floats raw, an immediate float on 64-bit, erasure made safe by forbidding type variables in tag fields, two-word debug values, and allocation-free optionals. An urgent subset can land any time; the full design waits for a benchmark, and an eight-step plan uses the enum as a test oracle. `&` references with `read` and `update` blocks follow, opened by the staircase of nested functional updates, defined as lenses with no mutation and no escaping references, and suited to the Haskell implementation first.

### 17. Appendices

The chapter opens as four reference tables for a reader who knows the design and wants one page to check against. Appendix A sets Prism beside Reflex construct by construct, opened by the afternoon a `Dynamic` read through `current` in one place and `updated` in another disagreed by a frame. The thesis is that almost every choice in the language is a choice to keep something of Reflex's or remove it, and the notes show both spellings side by side and point to the chapter that owns each argument.

The Reflex notes cover the hidden behavior half (no `current` except inside the Haskell evaluator's tag node), lifting in place of `fmap` and `zipDynWith`, one non-prompt switch in place of four variants, no `PostBuild` with `@started` in its place, elements instead of `Requester` and `MonadQuery`, a plain-data boundary in place of an improvised host, and `listWithKey` as `for` with prompt consolidation.

