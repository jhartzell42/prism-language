# Chapter 14: The Haskell implementation

A specification with one implementation is a description of that implementation. The Haskell implementation is the second reading of the language chapters, done independently, in a language whose FRP library is where most of those chapters' semantics came from. It has its own frontend, written from the book and sharing nothing with the Rust one but the typed-AST text form; its runtime is Reflex's Spider under a thin evaluator that walks the typed AST and builds the Reflex network each widget block describes; its Worlds are a Test World over the same transcript format as the Rust one and a reflex-dom World that puts Prism programs on a screen before the Rust GUI World exists. It proves that the chapters can be read twice and mean the same thing, that Prism's semantics are Reflex's where they overlap, and that the boundary (slots, paths, the run call, state supply, `Defer`) can be put on an engine that was not built for it without changing the engine. It does not prove anything about the Rust runtime's memory, its closure compiler, or its speed, and running it in production is outside this plan, though nothing in its design rules that out. It is kept passing for the same reason a second witness is kept: so that the specification keeps being one.

## What the second implementation is for

You have written a specification, handed it to a second person, and watched them build something that passed every test you had and still disagreed with your implementation on the first program you had not thought to test. The disagreement was never in the tests; it was in a sentence both of you had read and each of you had read differently. That afternoon is the reason this implementation exists, and it decides everything about how it is built.

The Haskell implementation is a reader, not a port. A port of the Rust runtime would reproduce the Rust runtime's reading of every sentence, bugs included, and the differential test between the two would be a test of the port's fidelity rather than of the specification's precision. So the Haskell implementation does not call the Rust frontend, does not load the Rust runtime, does not read the Rust implementation's internal formats, and the person or model writing it is handed the book and the goldens and not the Rust source. It shares two interchange formats: the typed-AST text form, so that the two frontends can be compared on the same program, and the element-list transcript, so that the two whole implementations can be compared on the same test. Everything between those two formats is written twice.

What the second reading checks, concretely:

- That the language chapters are a specification. Every program in the suite is dumped by both frontends and the dumps are diffed; every divergence is a bug in one frontend or a sentence two careful readers read differently, and the second kind is the find that matters, because it goes back into the book as a rewritten sentence and a new law test.
- That the time chapters are Reflex's semantics where the two overlap. The tag law, the hold law, non-prompt switching, `dyn` firing in the rebuild occurrence, the mid-occurrence construction rule: each of these was taken from Reflex, and running Prism on Spider is a check that nothing was lost in the taking. If a Prism law cannot be implemented on Spider by calling the combinator the law names, either the law is not what Spider does, which is worth knowing, or the law is Prism's own and the evaluator has to add it, which is also worth knowing.
- That the boundary is separable from the engine. Slots, paths, routing against the opened list, state supply through a supplier, the run call as plain data, `Defer`: these are Prism's additions to Reflex, and the claim that they are additions rather than changes is tested by putting them on Reflex without patching Reflex.
- That two runtimes with different internals produce the same transcripts. The Rust runtime is a height-ordered action map with holds committing at the top height; Spider is a subscriber graph with hold assignments at the end of the frame. Both pass the same scripts and print the same transcripts, which is the strongest available evidence that the transcripts are determined by the chapters and not by a runtime.

What it does not check matters as much. It has no ownership tree, no closure compiler, no lint and no values representation, so it says nothing about any of them; and the plan builds it only two Worlds, so it says nothing about the others either.

## Interpret, do not compile

The Haskell implementation interprets the typed AST at runtime. It does not translate Prism to Haskell source and hand the result to GHC. The decision follows from what the implementation is for: an interpreter checks the specification a second time with no second compiler to be wrong in.

Consider what the alternative involves, because it is attractive and the attraction is real. Prism's constructs line up with Reflex's one to one. The counter,

```prism
@counter: @unit = @widget:
    !increment <- @Button $"Increment"
    !decrement <- @Button $"Decrement"
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 then $count + !delta
    @Label "Count: {$count}"
```

printed as Haskell is the program a Reflex programmer would type:

```haskell
counter = mdo
  increment <- button (constDyn "Increment")
  decrement <- button (constDyn "Decrement")
  let delta = leftmost [(-1) <$ decrement, 1 <$ increment]
  count <- holdDyn 0 (attachWith addInt (current count) delta)
  label (("Count: " <>) . showInt <$> count)
```

Every line of the typed AST has a line here. The cyclic group is `mdo`. The tag is `attachWith` over `current count`. The hold is `holdDyn`. Nothing is closure-converted, because Haskell has closures; nothing is boxed at a crossing, because every Haskell value already has one representation; GHC type-checks the output, so a printer that emits a `Dynamic` where an `Event` was meant is caught by a compiler that has been catching that for decades. The output is readable by anyone who has written Reflex, which makes it a golden format a person can review. And `Strict` on the pure modules gives Prism's eager evaluation for free. All of that is true, and it is why compiling to Haskell is the right second thought and the wrong design.

The first problem is in the sentence "GHC type-checks the output". A printer that emits Haskell has to emit Haskell *types*, and Prism's types are not Haskell's. Polymorphic literals resolved by three built-in traits, tag-indexed slot fields inside a `match` arm, extensible variants with a catch-all, erasure of type variables in tag fields, records ordered by field declaration order, elements ordered by tag name: each of these has a Haskell spelling, and each spelling is a design decision about how Prism's typing maps onto GHC's, which the elaborator has already made once in Prism's own terms. Make it twice and there are two elaborators, and when GHC rejects the output nobody can say without an afternoon's work whether the Prism elaborator was wrong, the printer was wrong, or GHC's inference reached a different conclusion from a program the printer left underdetermined. The second reading now has a third reader in it, and the third reader is the one whose reasoning is hardest to inspect.

The second problem is GHC in the loop. Compiling to Haskell makes GHC a build dependency of every Prism program the Haskell implementation runs: the conformance suite is a few hundred programs, the differential test runs on every change to either implementation and on every change to the book, and each run is a GHC invocation per program. GHCi takes some of the cost away, since the Prism runtime library compiles once and the generated modules are small, but the cost that remains is the one that matters for a test oracle: the time from "the sentence changed" to "both implementations have been checked against it" has a compiler in it. An interpreter loads a typed AST and runs. The whole suite under the Haskell implementation is one process.

The third problem settles it. A compiler from Prism to Haskell is a compiler, with every place a compiler can be wrong: a lift printed as the wrong combinator, a group placed in the wrong `mdo`, a capture that names the wrong binding after a rename. The Rust implementation already has a compiler, with a lint pass and goldens to keep it honest. The point of the second implementation is to have a reading of the chapters that does *not* share the compiler's structure, so that when the two disagree the disagreement is about the chapters. An evaluator that walks a tree and calls `holdDyn` where the tree says `hold` has one place to be wrong per node kind, and that place is a `case` arm a reviewer can read against the sentence it implements. A compiler has a place to be wrong per node kind times every context the node can appear in.

The typed-AST text form is the bridge that makes this cheap. The Haskell frontend prints it; the Haskell evaluator parses it; and the two are separable, so that a typed AST written by hand, or dumped by the Rust frontend, runs under the Haskell evaluator with no Haskell frontend at all. In the order the plan runs, this matters: the evaluator can be built against hand-written typed ASTs and the Rust frontend's dumps before the Haskell frontend passes the layout corpus, and a Rust frontend bug that produces a wrong dump shows up as a wrong transcript under both evaluators, which localises it to the frontend without anyone reading the frontend.

The interpreter gives up GHC checking the output, and speed, and needs neither: the lint pass on the Rust side and the differential test on both do the first job, and the Haskell implementation runs test scripts, where per-occurrence cost is not a problem. The evaluator itself, the world core, the printer and the Worlds are ordinary typed Haskell, and GHC checks those.

## The frontend

The Haskell frontend is a second reading of the chapters on lexical structure, values and types, expressions, and the language implementation, written from those chapters and from the goldens. Its output is the typed AST in the text form the Rust frontend prints, character for character, and the test of the frontend is `diff`.

A cabal project `prism-hs` with these modules:

- `Prism.Lex`: a hand-written lexer over the token set, with the spacing rule, the suffixed operators (`+~`, `+.`), the comparison operators `< <= > >=` and the keyword operators `then`, `not`, `and`, `or` beside `==` and `!=`, text literals with `{expr}` interpolation (a `{` inside a text literal opens an expression that the lexer scans to its matching `}`, `\{` and `\}` are literal braces, and `$` is always literal text), `;` as a statement separator, the lambda bars distinct from merge, `Gui.*` as a name, a dot and a star, and a span on every token. Each paren and bracket token records whether it touches its neighbour, so that the parser can tell `((` from `( (` and `]]` from `] ]` without the lexer deciding what a double bracket means.
- `Prism.Layout`: the offside rule as the lexical chapter states it, producing indent and dedent tokens, with inline `:` as a plain token the parser interprets, `;` separating statements inside an inline block (`type Pair<a, b> = record: first: a; second: b`), and `else` terminating an inline block. A continuation line that starts with `.Method` closes the call on the line above and takes it as receiver, so `items.Filter IsDone` followed by an indented `.MapValues Label` is `MapValues (Filter items, IsDone), Label`; on one line a spaced `.F` is not a chain step. The layout corpus is the first test it must pass, before the parser exists.
- `Prism.Parse`: a parser (hand-written recursive descent, or megaparsec; the precedence table is small and the decision does not show in any output). The operator table, loosest to tightest: `on` and `|`, `or`, `and`, `not`, comparison, arithmetic, so `not a == b` parses as `not (a == b)` and `a < b and c < d` needs no parentheses. A `match` arm is `pattern if cond:` or `pattern:`, and the guard goes on the arm node. `for k, $item <- $items:` binds the key and the per-key dynamic by name, and `for $items:` is the shorthand that binds `key` and `$_`; the parser fills in those two names, so no later stage sees the shorthand. Array patterns (`[[]]`, `[[head, ..rest]]`, `[[a, b]]`) are parsed with the same double-bracket rule as array literals. `{expr}` pieces of a text literal are parsed as expressions, and a literal that interpolates a dynamic is a dynamic. Paren-less calls taking the rest of their expression (`F a + 1, b` is `F(a + 1, b)`), the comma rule, a single-paren group whose head is a call taking all its commas (`(G b, c)` is `G(b, c)`), tuples, tuple types and tuple patterns in double parens (`((a, b))`, `((A, B))`, `((x, y)) <- ...`), array literals in double brackets (`[[1, 2, 3]]`, `[[]]`) with single `[i]` always indexing and array types staying `[A]`, function types as `fn A, B -> C` with parens only around a function-typed argument or result, method form (`x.F y` is `F x, y`), a trailing lambda absorbing the rest of the line, sigils, blocks, patterns, tag declarations, `intercept`, and the `test` form with its steps.
- `Prism.Resolve`: names and scopes, the pronoun and its sigiled forms, the pronoun shorthand wherever `_` is in scope (prefix `.x` is `_.x` and prefix `[i]` is `_[i]`, so `\.done` and `\[0]` resolve, as does `F .x` inside a `for` body; with no `_` in scope it is an unbound-pronoun error), tag namespaces, and binding groups: a dependency analysis over each widget block into strongly connected groups in dependency order, with each edge labelled prompt or delayed.
- `Prism.Elab`: Hindley–Milner on the value layer with the three traits, the functor layer read off the sigils by the lifting rule, numeric types by spelling, `for` consolidation by result kind, tag field types from declarations, the build-time or occurrence-time mark on every binding, the cycle check over the labelled groups, and exhaustiveness (a guarded arm does not count toward it; array patterns do, so `[[]]` with `[[head, ..rest]]` is exhaustive). Its output is `Prism.Typed`'s tree.
- `Prism.Typed`: the typed AST as a Haskell data type, one constructor per keyword in chapter 12's grammar table: `dmap`, `tag … [$count]`, `hold_first`, `element … responds`, the trait-resolved operators (`Num.Add[Int]`, `ToText[Int]`), `switch`, `join`, `dyn`, `for` with its can-fire-at-birth flag, and the rest of that table.
- `Prism.Typed.Text`: the printer and the parser for the text form, from one definition, so that `parse . print` is the identity on every tree and `print . parse` is the identity on every golden. The type-variable naming rule (by first appearance, left to right, top to bottom) and the let-generalisation rule (at `let` of a lambda only) are implemented here and in `Prism.Elab` respectively, and both are stated in the language implementation chapter because the text form exposes them.
- An executable `prism-hs` with `dump`, `test` and later `run` subcommands.

The frontend's tests are the layout corpus; every golden from the Rust frontend, as a diff between `prism dump` and `prism-hs dump`; and the negative goldens, compared on the error kind and the spans, with the message prose allowed to differ since a message is not a meaning.

The dump is where the second reading pays. Here is what the Haskell frontend prints for the counter in chapter 12's file, which must be byte-identical to what the Rust frontend prints (the spans are that file's, where `@Button` and `@Label` are defined above the counter):

```
@counter: @unit                                                                       @11:1
  !increment: !Click [build] g1 = bind @Button(const "Increment")                     @12:5
  !decrement: !Click [build] g2 = bind @Button(const "Decrement")                     @13:5
  !delta: !Int [occ] g3 = leftmost [map !decrement \|ignore| -1:Int, map !increment \|ignore| +1:Int]   @14:9
  $count: $Int [build] g4 = placeholder                                               @15:5
  $count.hold: $Int [build] g4 = hold 0:Int (tag !delta [$count] \|d, c| Num.Add[Int] c, d)   @15:5
  close $count $count.hold                                                            @15:5
  _: unit [build] g5 = bind @Label(dmap [$count] \|c| cat ["Count: ", ToText[Int] c])   @16:5
  @ _
```

Every decision the two frontends could make differently is on the page. If the Haskell elaborator read the lifting rule as "a dynamic and an event give a tag" but placed the tag on the other operand, the `$count.hold` line in `g4` differs. If it generalised the `let` in `g3` (it must not; `!delta` is not a lambda), the type differs. If its resolver put `$count` in a group with `!delta`, the group numbers differ. If its inference named a type variable by a different rule, a polymorphic golden differs. None of these are runtime questions, and all of them would otherwise surface as a transcript divergence three layers down, which is why the dumps are diffed first.

The procedure for a dump divergence is fixed and is the same as for a transcript divergence. Either one frontend has a bug, in which case the golden says which and the frontend is fixed; or the sentence the two frontends read is ambiguous, in which case the sentence is rewritten to admit one reading, a golden is added that pins it, and both frontends are brought to it. The Haskell frontend is never brought to the Rust frontend's behaviour without a sentence to point at. Hindley–Milner is where this will happen most: where generalisation occurs, whether a monomorphic group's variables are generalised after the group, how a literal's type is pinned by an operand three lines away. Each of these is a sentence in the language implementation chapter, and a divergence that traces to a choice that chapter does not state is a missing sentence before it is a bug.

## The evaluator

The evaluator is a function from a typed AST and a widget name to a Reflex network. It is the whole of the Haskell runtime. Everything else is Reflex, and the sections below say which Reflex, node kind by node kind.

### Values

The elaborator has already checked every type, so the evaluator does not ask Haskell to check them again. One Haskell type holds any Prism value:

```haskell
data Val t
  = VUnit
  | VBool   !Bool
  | VNat    !Word64
  | VInt    !Int64
  | VFloat  !Double
  | VText   !Text
  | VOpt    !(Maybe (Val t))
  | VTuple  ![Val t]
  | VArray  !(Seq (Val t))
  | VRecord ![(Name, Val t)]                  -- fields in declaration order
  | VCon    !Name !Int ![Val t]               -- closed choices; the Int is the declaration index
  | VMap    !(Map (Val t) (Val t))            -- Ord on Val is hand-written (below)
  | VElem   !ElementValue                      -- tag plus plain fields; Ord by tag name, then fields
  | VFun    ![Val t] -> Val t                  -- pure functions are Haskell closures
  | VEv     !(Event t (Val t))
  | VDyn    !(Dynamic t (Val t))
  | VWidget !(PrismM t (Val t))               -- runs at build, in the widget monad
```

This is the Rust runtime's one-word value seen from the other side: there, a `Plain` plus the static type at each downcast; here, a sum whose projections (`asEv`, `asDyn`, `asInt`) fail with "elaborator bug" when the tree lied. A typed alternative would turn the typed AST into a GADT with each expression's type hidden in an existential and recovered at every binding with `eqTypeRep`. That re-proves what the elaborator already proved, in a type system that is not Prism's. The untyped `Val` keeps the evaluator one `case` per node kind and keeps the type questions in the frontend, where the goldens are.

`Int` is `Int64` and `Nat` is `Word64`, so that overflow happens at the same number under both implementations; the bare arithmetic nodes are checked, and overflow raises the occurrence's abort with kind `Overflow` (below); `Checked.*` and `Wrap.*` are prelude functions the evaluator binds to `Int64`/`Word64` operations that return `Maybe` and wrap respectively. `Float` is `Double`, and `/` on it is IEEE division that never fails, while `/` on `Nat` and `Int` returns `VOpt Nothing` for a zero divisor. `Ord (Val t)` is written by hand so that a `VMap` can be keyed by any ordered plain value (text, numbers, tuples, arrays, records, elements), which is what `for` over a `Text ==> Item` needs. Two of the derived instances Haskell would give are wrong for Prism and are not used. Derived `Ord` on `Double` is the partial IEEE comparison, under which a NaN is neither less nor greater than anything and a `Map` keyed by it corrupts itself; `Float` is ordered by IEEE totalOrder instead (negative NaN, negative infinity, the negative numbers, `-0.0`, `0.0`, the positive numbers, positive infinity, positive NaN), implemented on the bit pattern of the `Double`, while `Eq` stays IEEE equality. And derived `Ord` on a constructor name would order a closed `choice` alphabetically; a closed choice is ordered by the declaration order of its alternatives, then by payload, so `VCon` holds each alternative's declaration index and the instance compares the index first. Records compare structurally. A record keeps its fields as a list in declaration order rather than a `Map Name`, because a `Map` would order a record by field *name*; two records of one type have the same names in the same positions, so the derived `Ord` on the list is lexicographic in declaration order, which is the language's record ordering (Rust's derived `Ord`), and the elaborator has already rejected a record key with an unordered field. Equality on elements is tag plus plain fields, with any request event excluded, which is the identity rule for state; `Ord ElementValue` is written by hand to match, comparing tag names first and then the plain fields in declaration order, so that elements can key sorted maps. Events, dynamics, blueprints and functions have no `Eq`, and the elaborator guarantees they are never compared; if a `VFun` reaches `==`, the projection fails as above.

### The monad stack

A widget block runs in `PrismM t`, which is what `@A` corresponds to:

```haskell
type PrismM t = ReaderT (Env t) (PrismBase t)

type PrismBase t =
  DynamicWriterT t (Map Path (Slot t))          -- the element list
    (EventWriterT t (Seq (Path, Val t))         -- request firings this occurrence
      (PerformEventT t (SpiderHost Global)))    -- the host frame; Adjustable
```

Each layer is there for one law.

The `DynamicWriterT` is the element list. Reflex's `MonadDynamicWriter` documentation says (from memory) it "efficiently collects 'Dynamic' values using 'tellDyn' and combines them monoidally to provide a 'Dynamic' result", and a `Map Path Slot` under union is the monoid, with paths unique so union never has to choose. A slot created in occurrence n is told in n and is in the list after n; a `dyn` that replaces its block replaces the block's told dynamics, through the writer's `Adjustable` instance, so the old block's slots leave the list in the rebuild occurrence. The list is an ordinary `Dynamic`, which is the sentence "the element list of a widget is an ordinary dynamic" implemented by choosing the transformer whose output is one.

The `EventWriterT` is request collection. A slot whose tag has a `requests` field tells its request event at creation, keyed by its path; the run call reads the writer's event in the read phase and groups it by path. The writer's own merging handles the case the mid-occurrence construction rule makes awkward: a block built in n whose request fires in n (`Read on !started` in a newborn row) is told in n, and `EventWriterT`'s `runWithReplace` instance includes a newborn's tells in the frame of its birth (from memory). How it does that is the subject of the `for` section, because `for` needs the same thing and borrows the same mechanism.

The `PerformEventT` is in the stack for one reason: its `Adjustable` instance. `runWithReplace`, which `dyn` is, and `traverseDMapWithKeyWithAdjust`, which `for` is, are methods of `Adjustable`, and the host frame does not have an instance; `PerformEventT` over the host frame does. The evaluator never calls `performEvent`. A Prism program performs nothing; the World does, after the occurrence commits, from the request firings in the run call's output. The review checklist has a line for this: no `performEvent` anywhere in `Prism.Eval`.

`SpiderHost Global` is the bottom for the host's sake: trigger creation, `fireEventsAndRead`, the read phase. The evaluator is written against `Reflex t` and the transformer classes rather than against `Spider` directly, so that rollback (below) can run a program on a fresh local timeline.

The absences from the stack are as telling. No `PostBuildT`, because `@started` does not use post-build. No `RequesterT` or `QueryT`, because elements are not requests in a transformer and state is not a query; both are slots in the writer. No `StateT` for paths, generation counters or memo tables; those are `IORef`s in the reader, because they are per-node facts that `mfix` must be able to tie through, and a state monad under `mfix` is a knot that bites. And no `IO` reachable from a widget block except through those refs, which is the "there is no `liftIO`" row of the correspondence table made structural.

### The environment

Every slot needs a path, every response needs a fan to select from, every state slot needs a supplier to ask. The environment is where a widget block finds those:

```haskell
data Env t = Env
  { envVars     :: Map Name (Val t)          -- lexical scope
  , envPath     :: Path                      -- prefix for slots made here
  , envBindIx   :: IORef Int                 -- next bind position in this block
  , envFan      :: EventSelector t (Const2 Path (Val t))  -- responses, by path
  , envOpened   :: Behavior t (Map Path (Slot t))         -- the enclosing list, as opened
  , envSupply   :: Supply t                  -- where a state slot gets its dynamic
  , envPushes   :: Map ElementValue (Val t)  -- this occurrence's pushes (root only)
  , envTags     :: Map Name TagInfo          -- the tag table from the typed AST
  }
```

`envVars` is the scope. Pure functions close over it as Haskell closures; recursive groups extend it through `mfix`. `envPath` and `envBindIx` give every slot its path: a `bind` statement at position i under prefix p makes slots at `p/i`; a `dyn` adds `#g` for its generation g; a `for` adds `[k]`; a supplied widget runs under `s/n`. The spelling is the transcript's and is the one fact about paths the implementation does not get to choose. `envFan` is the responses event of the nearest enclosing interceptor (the run call's trigger at the root), fanned by path with `fanMap`, so that a slot at path p selects `select envFan (Const2 p)` and nothing is scanned. `envOpened` is the behavior of that interceptor's child list, read only for the unknown-path check. `envSupply` is the nearest supplier context, the heart of state supply. `envPushes` exists only at the root and only during an occurrence, for a cell created mid-occurrence whose value was pushed in that same occurrence.

### Pure expressions

A plain expression evaluates without the monad:

```haskell
evalE :: Reflex t => Env t -> Expr -> Val t
evalE env = \case
  Lit l          -> litVal l
  Var x          -> envVars env ! x
  Lam xs body    -> VFun $ \vs -> evalE (bindAll xs vs env) body
  App f as       -> apply (evalE env f) (map (evalE env) as)
  Op op a b      -> primOp op (evalE env a) (evalE env b)   -- Num.Add[Int] and the other trait operators
  Field e name   -> project name (evalE env e)
  Match s arms   -> matchPlain (evalE env s) arms env
  If c a b       -> if asBool (evalE env c) then evalE env a else evalE env b
  And a b        -> if asBool (evalE env a) then evalE env b else VBool False
  Or a b         -> if asBool (evalE env a) then VBool True else evalE env b
  Text parts     -> VText (foldMap (render . evalE env) parts)
  ...
```

Three absences matter here. It does not type anything; `primOp Num.Add[Int]` is `Int64` addition that aborts with kind `Overflow`, because the elaborator chose `Num.Add[Int]`. It does not decide lifting; `map` and `tag` are explicit nodes, so a `Var` is always a plain value and an `App` a plain application. It is strict: `litVal`, `apply` and `primOp` force their arguments, which is Prism's evaluation order (eager except for functions) as a property of three functions. The exceptions are `If`, `And` and `Or`, which are their own cases so that the operand not needed is never evaluated, and an overflow in it does not abort. `not` is an ordinary strict operator node; its precedence is settled in the parser and the evaluator never sees it.

`matchPlain` tries arms in order. An arm matches when its pattern does and, if it has a guard, the guard evaluates to `True` in the environment the pattern extended; a failed guard falls through to the next arm. Array patterns match on the `Seq`: `[[]]` on empty, `[[a, b]]` on length two, and `[[head, ..rest]]` on a non-empty array, with `rest` the tail. `If c a b` evaluates the branch the condition selects (both branches are always present); the lifted forms over a dynamic condition are `map` nodes over that function like any other. The one laziness the evaluator relies on is the knot of a recursive group, where a placeholder's value is demanded only inside an occurrence-time closure.

### Events and dynamics

The typed AST has every lift explicit, so the evaluator does not lift; it maps the node to the combinator:

```haskell
  MapE f e       -> VEv  (apply1 (evalE env f) <$> asEv  (evalE env e))
  MapD f d       -> VDyn (apply1 (evalE env f) <$> asDyn (evalE env d))
  Map2D f a b    -> VDyn (zipDynWith (apply2 (evalE env f)) (asDyn a') (asDyn b'))
  Const v        -> VDyn (constDyn (evalE env v))
  Leftmost es    -> VEv  (leftmost (map (asEv . evalE env) es))
  Never          -> VEv  never
  Tag d e f      -> VEv  (attachWith (apply2 (evalE env f)) (current (asDyn d')) (asEv e'))
  Updated d      -> VEv  (updated (asDyn (evalE env d)))
  FilterE f e    -> VEv  (fmapMaybe (asOpt . apply1 (evalE env f)) (asEv (evalE env e)))
```

Four rows deserve a sentence.

`Tag` is the only place `current` appears in the evaluator's expression cases. Reflex's `hold` documentation states the law the tag node implements: "if the occurrence that sets the 'Behavior' (or one that is simultaneous with it) is used to sample the 'Behavior', it will see the __old__ value of the 'Behavior', not the new one." `attachWith f (current d) e` reads the old value, and that is the tag law. A reviewer greps the evaluator for `current` and expects to find it here and in two other named places (below), and nowhere else.

`Map2D` is the change law. `zipDynWith` produces a dynamic that changes in any occurrence either operand changes, with both new values, and changes once, which is glitch freedom as Reflex provides it. Three or more dynamics are nested `zipDynWith`s, which the elaborator has already arranged since the typed AST has binary nodes.

`Updated` is `updated`, the change event of a dynamic, firing with the new value in every occurrence the dynamic changes and never at build. Nothing more is needed: `!Updated` is a plain prelude function in Prism and a plain projection in Reflex, and the evaluator does not wrap it in a widget.

`Distinct` is `holdUniqDyn`, which is monadic because it needs a hold, so it is a build-time node and appears in the statement cases rather than here. Its definition in Reflex is (from memory) `attachWithMaybe` over `current d` and `updated d`, holding the result, so it fires only when the new value differs from the old; that is exactly `$Distinct`'s law and the `current` inside it is Reflex's, not the evaluator's.

Lifting over `?` is at the plain layer and never reaches the monad: an `App` on optionals is an applicative `Maybe` inside `apply`, and `match` on `?` is a `matchPlain` arm.

### Holds and recursive groups

A widget block is a list of statements in group order, each extending the environment, and the block's result is an expression over the final environment. Groups with a placeholder are knots:

```haskell
evalBlock :: Env t -> Block -> PrismM t (Val t)
evalBlock env0 (Block groups result) = do
  env <- foldM evalGroup env0 groups
  pure (evalE env result)

evalGroup :: Env t -> Group -> PrismM t (Env t)
evalGroup env (Group stmts placeholders) = mdo
  env' <- foldM (evalStmt (bindAll placeholders (lookupAll placeholders env') env)) env stmts
  pure env'
```

The placeholders are the names the resolver found used before their binding, which the cycle check has already passed through a delayed edge. They are bound in the environment to their own eventual values through `mdo`, and nothing else in the block is in the knot. The `close` line in the text form is where the knot's value is demanded; everything between the `placeholder` and the `close` sees the name through the knot.

This works because of where the delayed edges are. `$count` reaches its own `holdDyn` only inside the closure `attachWith` runs on each occurrence, and Spider's `hold` does not look at its event during build; the source says so in a comment above `hold`: "hold cannot examine its event until after the phase is over", and the implementation defers the subscription with `defer $ SomeHoldInit h`. A `!Switch` reaches its held inner event through a behavior read at frame end. A tag reads a behavior inside a push closure. So every legal cycle passes through something Spider resolves lazily, and `mfix` ties the rest. Reflex's own `foldDyn` is the same knot: a `rec` block whose push closure samples the hold it feeds.

If the elaborator were wrong, and a placeholder were read promptly during build, the knot would be forced early and GHC would report `<<loop>>`, which is the counterpart of the Rust runtime's "cyclic dynamic never closed". If the cycle check were wrong, and a prompt cycle were built, Spider throws an `EventLoopException` whose `Show` instance begins "causality loop detected", which is the counterpart of the Rust runtime's re-entry panic. Both are reported as a `Cycle` error with the group's spans, which the typed AST has.

The statements:

```haskell
evalStmt env acc = \case
  Let x e          -> pure (insert x (evalE env e) acc)
  Bind x w         -> do v <- runWidget env (evalE env w); pure (insert x v acc)
  Hold x v e       -> do d <- holdDyn (evalE env v) (asEv (evalE env e))
                         pure (insert x (VDyn d) acc)
  Distinct x d     -> do d' <- holdUniqDyn (asDyn (evalE env d))
                         pure (insert x (VDyn d') acc)
  Dyn x d f        -> evalDyn env x d f acc
  Switch x ee      -> do e <- switchHold never (asEv <$> asEv (evalE env ee))
                         pure (insert x (VEv e) acc)
  Join x dd        -> pure (insert x (VDyn (join (asDyn <$> asDyn (evalE env dd)))) acc)
  For x m f kind   -> evalFor env x m f kind acc
  Slot x el        -> evalSlot env x el acc
  Intercept x c r s -> evalIntercept env x c r s acc
```

`Hold` is `holdDyn v e`: the initial value is the plain value, the change event is the input event itself, so the dynamic's change is prompt in the occurrence its event fires and its value side commits at the frame's end. A hold built in n whose source fired in n adopts that value at commit, because `subscribeAndRead` hands the hold its event's occurrence in n (Spider's `getHoldEventSubscription` reads the `occ` it is given and schedules the assignment). That is the sentence "a hold built in n whose source fired in n adopts that value at commit" implemented by calling `holdDyn` and nothing else.

`Bind` runs a widget. `runWidget` increments the bind position, extends the path, and runs the `VWidget` under the extended environment; the value it returns is whatever the widget's block returned. Binding a widget application whose result is plain gives a `$` holding the first build's value, and binding a dynamic of blueprints is a `dyn`, but both of those are already in the typed AST as explicit nodes by the time the evaluator sees them.

### `dyn` and `@started`

`dyn $x: block` is Reflex's `networkView`, which reflex-dom exports as `dyn` (from memory):

```haskell
networkView :: (NotReady t m, Adjustable t m, MonadHold t m) => Dynamic t (m a) -> m (Event t a)
networkView child = do
  postBuild <- now
  let newChild = leftmost [updated child, tagCheap (current child) postBuild]
  snd <$> runWithReplace notReady newChild
```

Read it against the `dyn` law. The block is built when `newChild` fires. `now` is the event that fires in the frame it is created, so at the `dyn`'s own build `newChild` fires at once; if the dynamic also changed in this frame (a `dyn` built mid-occurrence over a dynamic that changed in that occurrence), `leftmost` prefers `updated child`, so the block is built once, with the new value, and not with the old value and then again. On every later change `updated child` fires and `runWithReplace` replaces the block in that frame. The returned event is the block's result, firing in the frame of every build including the first, delivered to the caller and invisible to the block. Every clause of the `dyn` law is a line of this function, which is why the evaluator calls it rather than writing its own:

```haskell
evalDyn env x d f acc = do
  gen <- liftIO (newIORef 0)
  let build v = do
        g <- liftIO (atomicModifyIORef' gen (\n -> (n + 1, n)))
        runWidget (env { envPath = envPath env </> Gen g }) (apply1 (evalE env f) v)
  e <- networkView (build <$> asDyn (evalE env d))
  pure (insert x (VEv e) acc)
```

The generation counter gives rebuilt slots their fresh `#g` paths. Holds inside the block are new holds, because the block is a new `m a` action run afresh; the old block's told dynamics leave the writer when `runWithReplace` replaces them; and the old block's nodes keep propagating until the frame ends, because Spider does not tear anything down mid-frame, which is the sentence about the old node being kept alive until commit.

`@started` is a `dyn` over a constant, and the evaluator spells it that way:

```haskell
started :: PrismM t (Val t)
started = VEv <$> networkView (constDyn (pure VUnit))
```

That is one build, in the frame the widget is built, firing once with `unit`, and then never, because a constant never updates. The reason to write it as the `dyn` it is defined as, rather than as the `now` it reduces to, is that the `@started` law and the `dyn` law are then one code path and one law test covers the frame question for both.

The frame question is why `getPostBuild` is not used. The `PostBuild` class documentation says: "'PostBuild' represents an action that is notified via an 'Event' when it has finished executing. Note that the specific definition of "finished" is determined by the instance of 'PostBuild', but the intent is to allow 'Behavior's and 'Dynamic's to be safely sampled, regardless of where they were created, when the post-build 'Event' fires." Two things in that sentence disqualify it. "Determined by the instance" means the frame in which it fires depends on which transformer answers it, and at the root in a typical host it is a frame of its own, after construction, while inside a `runWithReplace` it is the rebuild frame; a widget using it would behave differently depending on where it is placed. And its stated purpose is to make sampling safe, which is a purpose Prism has no use for, since sampling at build does not exist. `now` has neither problem: it is the current frame, wherever it is asked, and Reflex's own `networkView` uses it for exactly this. In Spider, `now` is an `IORef` holding `Just ()` with a clear scheduled for the frame's end, so a subscriber that arrives mid-frame reads the occurrence through `subscribeAndRead` like any other late subscriber.

### `!Switch` and `$Join`

`!Switch` is `switchHold never` and nothing else. The documentation of `switchHold` reads: "Switches to the new event whenever it receives one. Only the old event is considered the moment a new one is switched in; the output event will fire at that moment only if the old event does." That is the non-prompt law word for word: at occurrence n the inner event current before n is the one heard, and an inner delivered at n is heard from n+1. The initial `never` is "it starts subscribed to nothing". The consequence the specification draws, that whatever a newly built block fires in its creation occurrence is swallowed at the switch, follows without further code: the new inner is delivered at n, and only the old one is considered at n.

`$Join` is `join` on `Dynamic`, which exists because `Monad (Dynamic t)` is a superclass of `Reflex` (from memory). Its behavior half is the inner's behavior read through the outer's; its event half fires when the outer changes (with the new inner's value after the frame) or the current inner changes. That is the prompt law of `$Join`, and the asymmetry with `!Switch` is Reflex's, kept.

The cache idiom, a hold of `!$B` joined with `$Join`, is two statements in the typed AST and two lines here; Reflex's `widgetHold` and `networkHold` are that idiom with `holdDyn` inside, and there is no primitive for it.

### `for`

`for $m: block` over a `$(K ==> V)` is Reflex's `listWithKey` for the building and the evaluator's own merging for the consolidation. `listWithKey :: (Ord k, Adjustable t m, MonadFix m, MonadHold t m, Eq v) => Dynamic t (Map k v) -> (k -> Dynamic t v -> m a) -> m (Dynamic t (Map k a))` (signature from memory) is `for` almost exactly: one block per key, the value as a dynamic inside the block, a map of results out. The evaluator builds with the lower-level pieces `listWithKey` is made of, because it needs the patch event and not only the held map:

```haskell
evalFor env x m f kind acc = do
  let perKey k dv = runWidget (env { envPath = envPath env </> Key k })
                              (apply2 (evalE env f) k (VDyn dv))   -- f's two parameters are the binders
  (results0, patches) <- listHoldWithKeyAdjust (asDynMap (evalE env m)) perKey
  held <- holdIncremental results0 patches
  pure (insert x (consolidate kind held patches) acc)
```

The block is a two-parameter function because `for k, $item <- $items:` names its binders; the shorthand `for $items:` gets the names `key` and `$_` from the parser, so the evaluator has one case.

`listHoldWithKeyAdjust` is the evaluator's helper over `traverseDMapWithKeyWithAdjust`, the call under `listWithKey`, giving the initial results and an event of patches that fires in the frame a key appears or disappears, with the new keys' results. A key appearing in n is built in n under the mid-occurrence rule; a key disappearing in n has its block replaced at the patch, and its nodes propagate until the frame ends. The per-key dynamic is the map's entry projected with `uniqDyn`-style equality on the value (from memory), so a change under a stable key updates `$_` and rebuilds nothing.

Consolidation is by the result kind the typed AST states:

- plain and `$` results: `joinDynThroughMap (incrementalToDynamic held)`, a `$(K ==> A)` that changes promptly whenever a key appears, disappears or an entry's dynamic changes;
- `!` results: an event of a map with exactly the entries that fired, which is where care is needed.

The event case has two terms. The first is the merge over the members as the occurrence opened: `mergeIncremental` over `held`, whose membership patches Spider applies at the end of the frame. The source is explicit about this: a merge's change subscriber does `defer $ updateMerge subscribed m updateFunc a`, and `runFrame` runs the deferred `_someMergeUpdate_update`s after `clearOccurrences`. So a block born in n joins this merge from n+1, and a block torn down in n is still in it for n, which is right for the torn-down block (it keeps propagating until commit) and wrong for the newborn, because `for` consolidation is prompt: a key that appears in n is built in n and anything its block fires in n is in the consolidated event in n. Only pure consequences of construction can fire then, `@started` and inner `dyn` events and lifted operations over captured events that fired in n, and the parent must hear them.

The second term supplies the newborn's birth firing:

```haskell
consolidate ConsolidateEvent held patches =
  VEv $ mergeWith Map.union
    [ Map.fromList . DMap.toList' <$> mergeIncremental (asEv <$> held)
    , coincidence (mergeMap . newborns <$> patches)          -- the birth term
    ]
```

`coincidence` subscribes to the inner event in the frame the outer fires and delivers the inner's occurrence in that same frame; its documentation says it will "occur whenever the input event is occurring -- and its occurrence value, another 'Event', is also occurring." Here the outer is the patch event, firing in n with the newborn blocks' result events, and the inner is the merge of those events, so the term fires in n with exactly the entries the newborns fired at birth. A key cannot be both an existing member and a newborn in one frame, so the union has nothing to decide. The new block does not see itself, because nothing inside the block is downstream of the block's result.

This is the one place `coincidence` appears in the evaluator, and here is why it is right here and nowhere else. The warning on the prompt combinators is about cyclic timing: `switchHoldPromptly`'s documentation says `switchHold`, "by always forwarding the old event the moment it is switched out, avoids many potential cyclic dependency problems / metastability problems", and a prompt reader placed where a program can make a cycle through it brings the program down. Prism's cycle check removes that possibility for this term: the elaborator rejects a `for` whose block's result depends promptly on the collection's change (`@ Delete on !started` as a row result is a compile error), so the only prompt edges into the birth term are from `@started`, inner `dyn` events and captured events, none of which can be downstream of the consolidation. The term is prompt because the law is prompt, and the law is safe because the checker made it so. Reflex's own `EventWriterT` does the same thing (from memory) in its `runWithReplace` instance, telling `coincidence` over the replacement event so that a newborn child's tells are heard in its birth frame, with a comment that the coincidence case must come first so that it has precedence; the evaluator's request collection inherits that, which is why a newborn row's `Read on !started` is in the run call's output for the occurrence of its birth without any code in the evaluator.

The elaborator's finding, whether a given `for`'s block can fire at birth, is on the `for` node in the typed AST next to the consolidation kind, and the evaluator omits the birth term when the block cannot fire at birth. That is an optimisation with a correctness side: a `for` whose block returns only element responses (the todo list, the interceptor's routing loop) has no birth firings, and leaving the `coincidence` out means Spider never subscribes mid-frame to an inner event that is structurally downstream of the fan, which keeps the interceptor loop's heights static. The law test for a new key's result being seen by the parent in its creation occurrence exercises the term; the law test for the forwarding interceptor exercises its absence.

Arrays are `for` keyed by index and go through the same path with `Nat` keys, which is why an insertion at the front rebuilds every later block, as it does under `simpleList` (from memory), and why the chapter on switching calls the array form second class.

### Elements and the list

`@Element e` makes a slot:

```haskell
evalSlot env x el acc = do
  path <- nextPath env
  let ev   = asElem (evalE env (elemValue el))
      info = envTags env ! elemTag ev
  resp  <- if hasResponds info
             then pure (select (envFan env) (Const2 path))          -- by path
             else pure never
  state <- if hasState info then supplyState (envSupply env) ev else pure Nothing
  let req = fmap (asEv . evalE env) (elemRequests el)
  forM_ req $ \r -> tellEvent ((\v -> Seq.singleton (path, v)) <$> r)
  tellDyn (constDyn (Map.singleton path (Slot ev req)))
  pure (insert x (slotResult info resp state) acc)
```

Four things happen and they are the four parts of a slot. The slot's path is allocated from the environment's prefix and bind position. Its response event, if the tag responds, is a selection from the enclosing interceptor's fan by that path, so that routing is a map lookup per firing and not a scan. Its state dynamic, if the tag has state, comes from the supplier context. Its request event, if the tag requests and the program supplied one, is told to the request writer under its path. And the slot itself is told to the list writer, as a constant dynamic of a one-entry map, because an element never changes after creation: a dynamic where a field goes is a `dyn` in the typed AST and a fresh slot per change.

The creation rule holds here by construction. A response enters only through the fan, the fan is over an event the World or an interceptor fires, and nothing fires in the frame a slot is created except what was already firing when it was created; a World cannot answer in the request's occurrence because it does not see the request until the read phase. A `state` element has a value in its creation occurrence because `supplyState` returns a `Dynamic` with a value, synchronously, during the build.

Routing against the opened list is in the fan's source, not the slot. The interceptor (and the root) builds its fan from its responses event *checked against the child list as the occurrence opened*:

```haskell
routed = attachWith checkPaths (current childList) responses
  where checkPaths opened rs
          | all (`Map.member` opened) (Map.keys rs) = rs
          | otherwise = throw (UnknownPath (head (Map.keys rs \\ Map.keys opened)))
```

`current childList` is the second sanctioned `current` in the evaluator: a read of the list before the occurrence, which is the routing law ("delivered to the slot at that path in the list as n opened") and a delayed edge, which is what makes the interceptor's loop legal. A response addressed to a slot created in this occurrence is an unknown-path error; a response addressed to a slot torn down in this occurrence is delivered, since the slot was in the opened list and its nodes propagate until the frame ends.

Element equality, needed for state identity, is `Eq ElementValue`: tag and plain fields, requests excluded; the memo and cell maps are keyed by `ElementValue` through the matching `Ord`. `Kind: fn Element -> Kind` is a lookup in `envTags`.

### `intercept`

`intercept @child, !responses, $supplier` builds the child under a fresh writer, so that its list is captured rather than told onward:

```haskell
evalIntercept env x child responses supplier acc = do
  outer <- ask
  memo  <- liftIO (newIORef Map.empty)
  mdo
    let routed   = attachWith checkPaths (current childList) (asEv (evalE env responses))
        childEnv = env { envFan    = fanMap routed
                       , envOpened = current childList
                       , envSupply = Supply (asDyn (evalE env supplier)) memo outer childList
                       , envPath   = envPath env </> Bind ix }
    (result, told) <- lift (runDynamicWriterT (runReaderT (runWidget' child) childEnv))
    let (supplied, childList) = splitDyn (Map.partitionWithKey (isSupplied (envPath env))) told
    tellDyn supplied
    pure (insert x (VTuple [result, VDyn (fmap slotsOf childList)]) acc)
```

The child runs under `runDynamicWriterT`, which peels one writer layer: the child's slots go into a fresh list that comes back as `told`, and nothing the child creates reaches the outer writer unless the interceptor tells it. The child's environment has a fan built from the interceptor's responses event, checked against the child's own list as opened; a supply context holding the supplier dynamic, a memo, the outer environment and the child's list; and the interceptor's path as prefix. The `mdo` is the cycle in the specification's own example: `routed` reads `current childList`, and `childList` is what building the child under `routed` produces. It is legal for the same reason it is legal in Prism, the read is delayed, and Spider resolves it the same way it resolves a hold.

The interceptor returns the child's result and the child's list as a `$(Path ==> Slot)`, which the interceptor's own code loops over with `for` and `dyn` to re-emit, rewrite or hold back. Forwarding is manual: the evaluator has no table of tags to forward and no mode. `\|e| @Element e` is a Prism lambda the evaluator applies like any other.

### State supply

A `state` element's dynamic has to exist, with a value, in the occurrence the slot is created, and it has to come from outside: the interceptor's supplier, or the World's oracle at the root. The specification's mechanism is that the interceptor's supplier is a dynamic of a function from an element to a blueprint returning the state's dynamic; at slot creation the runtime reads the function, applies it to the element, builds the widget then and there as a child of the interceptor's node, and the widget's result dynamic is the slot's. Once per distinct element value per interceptor; alive as long as some child slot with that value lives; a slot's policy fixed at creation.

```haskell
data Supply t
  = SupplyRoot   { oracle :: ElementValue -> Val t, cells :: IORef (Map ElementValue (Cell t)) }
  | SupplyWidget { supplierDyn :: Dynamic t (Val t)
                 , memo        :: IORef (Map ElementValue (Dynamic t (Val t), Dynamic t (Map Path (Slot t))))
                 , outerEnv    :: Env t
                 , childList   :: Dynamic t (Map Path (Slot t)) }

supplyState (SupplyWidget sup memo outer childList) ev = do
  live <- liftIO (readIORef memo)
  opened <- sample (current childList)
  case Map.lookup ev live of
    Just (d, l) | ev `suppliedIn` opened || builtThisFrame ev -> do tellDyn l; pure (Just d)
    _ -> do
      f <- sample (current sup)                                   -- the supplier as opened
      n <- nextSupplied outer
      let suppliedEnv = outer { envPath = envPath outer </> Supplied n }
      (d, l) <- lift (runDynamicWriterT (runReaderT (asWidget (apply1 f (VElem ev))) suppliedEnv))
      liftIO (modifyIORef' memo (Map.insert ev (asDyn d, l)))
      tellDyn l
      pure (Just (asDyn d))
```

Read the miss case first. The supplier function is read with `sample (current sup)`, which is the third and last sanctioned `sample` in the evaluator: the supplier's value as the occurrence opened, which is what "a slot's policy is fixed at creation" means when the supplier changes in the same occurrence. The function is applied to the element and the resulting blueprint is run *in the outer environment*: the interceptor's fan, the interceptor's supply context (so the supplied widget's own state slots ask the next context out, which is how `@Element e` forwards one level), the interceptor's path with an `s/n` segment. It is run under its own fresh writer, so its list comes back as the dynamic `l`, and its result `d` is the slot's dynamic from this instant on. It is run synchronously, during the child's build, in the same frame, which is "then and there, in the same build pass".

Then the two lifetime facts. The supplied widget's list `l` is told into the *child's* writer by every child slot that uses it, under the interceptor's `s/` prefix; the interceptor's partition after `runDynamicWriterT` separates those entries from the child's own and tells them onward to the outer writer, so the supplied widget's elements are in the interceptor's list and not in the child's `$slots`. Because every using slot tells the same dynamic under the same paths, the union has one copy, and the copy persists exactly as long as some telling slot's scope persists: when a `dyn` in the child replaces the last slot with that value, the writer's `Adjustable` instance drops its tells, the supplied elements leave the interceptor's list, and the supplied widget's nodes become garbage. That is "lives as long as some child slot with that value lives", with Reflex's writer doing the reference counting.

The hit case is memoisation by value with a liveness check. A memo entry is live if its paths are in the child's list as the occurrence opened (it was alive at open and nothing torn down in this occurrence has committed yet) or if it was built earlier in this frame (for a second newborn with the same value). A slot created for a value whose supplied widget died in an earlier occurrence gets a fresh build, so a cache's held value does not survive with no slot alive, which is what the Rust runtime's weak memo gives and what the law test for two child slots sharing one supplied widget distinguishes from a global cache. A new child slot whose value already has a live supplied widget joins it regardless of the current supplier dynamic, which the hit case does by not reading `sup` at all.

At the root the supply is the oracle:

```haskell
supplyState (SupplyRoot oracle cells) ev = do
  live <- liftIO (readIORef cells)
  case Map.lookup ev live of
    Just c  -> pure (Just (cellDyn c))
    Nothing -> do
      (pushE, pushRef) <- newEventWithTriggerRef
      pushedNow <- asks (Map.lookup ev . envPushes)
      n <- now
      d <- holdDyn (oracle ev) (leftmost [pushE, maybe never (<$ n) pushedNow])
      liftIO (modifyIORef' cells (Map.insert ev (Cell d pushRef)))
      pure (Just d)
```

A cell is a hold initialised from the oracle's answer, which is the value before this occurrence from the World's previous snapshot, and fed by a trigger the run call fires with pushes. A cell created mid-occurrence for a value this occurrence's pushes include cannot receive the trigger, since the triggers fired at the frame's start, so the `now` term delivers the push in the creation frame: the hold adopts the pushed value at commit and its change event fires in n, so a slot created in n ends n holding the new snapshot, exactly like a slot that existed before n. Two slots with equal values share one cell and therefore one dynamic, which is the equality law by construction. A state tag the World does not accept is an `UnsuppliedState` error raised here, at creation, because the oracle's type cannot express refusal and the world core checks the tag before the oracle is called.

### The run call

The root is an interceptor the host builds itself: its child is `@main`, its responses event is a trigger, its supply is the oracle, and its captured list is what the run call returns. A program is loaded once:

```haskell
loadProgram :: TypedAst -> Name -> SpiderHost Global (Runtime)
loadProgram ast mainName = do
  (respE, respRef) <- newEventWithTriggerRef
  cells <- newRef Map.empty
  ((listD, reqE, resultV), fire) <- hostPerformEventT $ do
    (((resultV, _), reqE), listD) <- runEventWriterT' (runDynamicWriterT' (runReaderT (rootBuild mainName) (rootEnv respE cells)))
    pure (listD, reqE, resultV)
  listH   <- subscribeEvent (updated listD)
  reqH    <- subscribeEvent reqE
  resultH <- subscribeEvent (resultEvent resultV)
  pure Runtime { .. }
```

`hostPerformEventT` (from memory, like `newEventWithTriggerRef`, `FireCommand` and `runFireCommand`) runs the stack's bottom and returns the `FireCommand` the run call uses; the evaluator never performs anything through it. Building `@main` inside the host frame is occurrence zero: a frame with no inputs in which every hold has an occurrence to have been built in and every `@started` fires. The three handles are the read side.

Each run call is one frame:

```haskell
run :: Runtime -> RunIn -> IO RunOut
run rt RunIn { responses, pushes, oracle } = runSpiderHost $ do
  setOccurrence rt pushes oracle                           -- envPushes, the oracle for new cells
  pushTriggers <- forM (Map.toList pushes) $ \(ev, v) -> cellTrigger rt ev v
  r <- try $ runFireCommand (fire rt)
         ((respRef rt :=> Identity responses) : pushTriggers) $ do
            list   <- readEvent (listH rt)   >>= sequence
            reqs   <- readEvent (reqH rt)    >>= sequence
            result <- readEvent (resultH rt) >>= sequence
            pure (list, reqs, result)
  pure (toRunOut r)
```

`fireEventsAndRead` (from memory) takes a list of triggers with their values and a read phase, runs one frame in which all the triggers fire together, and returns what the read phase read. That is the run call: responses by path and pushes by element value go in as trigger firings, all simultaneous; the element list, the request firings and the result come out. The read phase reads *events*, never behaviors: Spider's read phase runs inside the frame before the hold assignments, so `sample (current listD)` there would give the list before the occurrence, which is the tag law biting the host. `updated listD` fired in this frame if the list changed, and the host keeps the last list it was given when it did not; the request writer's event fired if any slot requested, grouped by path in tree order; the result's change event likewise. Pushes for values nobody has asked about are not fired, because there is no cell to fire into; the world core records them in its snapshot and they reach a slot created later through the oracle.

An occurrence is a Spider frame, opened by `fireEventsAndRead` and closed when it returns with holds assigned. Mid-occurrence construction is Spider's `subscribeAndRead`, which every late subscriber goes through and which hands it the event's occurrence in this frame if it has one, so that a `dyn` built in n sees n. The run call is `runSpiderHost` with `fireEventsAndRead` over trigger refs from `newEventWithTriggerRef`, and those four names appear in the Haskell implementation in this function and in the cell constructor and nowhere else; the Prism program cannot reach them, which is the row that says `newTriggerEvent` is removed.

### Errors and rollback by replay

An occurrence that aborts leaves no trace, in both implementations. The Rust runtime gets that from its transaction: holds commit at the top height, an abort clears the action map, and the committed state is the state before. Spider has no such transaction. Its hold assignments are collected in `eventEnvAssignments` and applied after propagation returns, so an exception thrown mid-frame does leave every hold at its old value; but the frame's occurrence caches, scheduled clears, deferred hold initialisations and merge updates are left half-done, and Spider is not designed to run another frame after that. A cycle is an `EventLoopException` thrown from `checkCycle`; an overflow, an unknown path or an unsupplied state is an exception the evaluator throws from inside a push closure or a build; all of them escape `fireEventsAndRead`, and after any of them the timeline is not to be trusted.

One failure is contained instead of aborting. A `dyn` whose block fails to build (kind `Build`) does not fire the `dyn`'s event and is reported through `errors`, the stream element the World offers. In the evaluator that is `evalDyn`'s `build` catching the exception around the block, running the block under its own fresh writer so that the half-built block's slots and holds are dropped, firing nothing for that generation, and telling a `Build` error into the error writer the root reads next to the request writer. The generation counter still advances, so the paths of later generations do not depend on whether an earlier one failed. A cycle, an overflow at occurrence time, an unknown path, a `DeferBackwards` and a `Component` error from the World abort the occurrence as above; the `ErrorKind` closed choice (`Cycle`, `Overflow`, `UnknownPath`, `UnsuppliedState`, `DeferBackwards`, `Component`, `Build`) is the same type in both implementations, and the transcript prints its constructor name.

The Haskell implementation's rollback is replay. A Prism program is deterministic: given the same sequence of input sets and the same snapshots, its sequence of lists and outputs is fully determined, and the determinism law is one of the sentences the suite pins. So the host keeps the log of every committed run call's inputs, which it has anyway since they are plain data, and on an abort it reports the error with the spans the exception names, discards the timeline, creates a fresh local one with `withSpiderTimeline`, rebuilds `@main` and replays the log. The program is then in the state before the aborted occurrence, with every hold, every `dyn` generation and every supplied widget as it was, and the next run call proceeds. The cost is the length of the history, paid only on the error path, and the benefit is that the transaction law is implemented by the determinism law rather than by surgery on Spider's internals. It is also a test of determinism that runs itself: if the replay produces a different list than the one the host had last received, the Haskell implementation has found a non-determinism, and it says so.

This is why the evaluator is written against `Reflex t` and not against `Global`: a replay needs a fresh `t`, and `SpiderTimeline Global` cannot be reset.

After an error the Haskell implementation does not quarantine. The world core accepts a `quarantine: [Path]` input and ignores it, and the transcript says so, so a script that exercises quarantine is marked as Rust-only in the suite. Everything up to and including the `error:` line of a transcript is compared.

### An occurrence, traced

Take the counter under the Test World, with a click on the increment button in turn 1, and follow it through Spider.

Occurrence zero is `loadProgram`. Inside the host frame the evaluator walks the counter's groups. `g0` and `g1` bind two `@Button`s, which are the GUI library's widgets over `Ui.Button` slots: each allocates a path (`0/#0/0`, `1/#0/0` once the library's own `dyn` over the label has added its generation), selects its response event from the root fan by that path, and tells its slot. `g2` is `leftmost` over two `fmap`s. `g3` is an `mdo`: `holdDyn (VInt 0) (attachWith add (current count) delta)`, with `count` bound through the knot; Spider defers the hold's subscription, so nothing is demanded early. `g4` binds the label, a `dyn` over the text dynamic, whose `networkView` fires its `now` in this frame and builds the `Ui.Label` slot at `2/#0/0`. The read phase reads `updated listD`, which fired in this frame with the three slots, and the result. The transcript's first turn is printed from that.

Turn 1 is one run call with `responses = {0/#0/0: click}`. The response trigger fires; `attachWith checkPaths (current childList)` reads the list as opened, finds the path, and passes the map to the fan; the fan selects by path, and the first button's response event fires with `click`. The library's widget maps it to `Click`; `g2`'s `leftmost` fires with `VInt 1`; the `attachWith` in `g3` reads `current count`, which Spider's hold still holds at `0` because assignments happen at frame end, and fires with `1`; the hold schedules its assignment. `updated count` fires with `1`, so the label's text dynamic changes, so the `networkView` under the label's `dyn` sees `updated child` and `runWithReplace`s the block: a new `Ui.Label` slot is told at `2/#1/0`, the old one's told dynamic is dropped. The writer's list dynamic changes in this frame with the new map. `runFrame` then runs the assignments: `count` becomes `1`. The read phase reads `updated listD`, and the transcript's second turn shows the new label path and text. Nothing was sampled, nothing was performed, and the whole of what the World learned is the list.

## What must not be used

Reflex offers, next to each combinator the evaluator needs, one that looks the same and means something Prism has removed. The evaluator is small enough that these are review items rather than architecture, and the review is a grep. Each line below has the evidence for it, so that whoever reviews the evaluator can see why the line is there.

**`current` and `sample`, outside three places.** `current` extracts a dynamic's behavior and `sample` reads it, and `MonadSample` is a superclass of nearly every monad a widget runs in, so a model writing the evaluator will reach for `sample (current d)` whenever it needs a value at build time. There is no such need: everything at build time is a build-time binding in the typed AST, and those are plain values; a build-time read of a dynamic does not exist in Prism, and a value sampled at build is a constant that looks like a dynamic, which is the afternoon the whole design is built to prevent. The three places `current` or `sample` legitimately appear are the `Tag` node (the tag law), the fan's `attachWith checkPaths (current childList)` (routing against the opened list) and `sample (current sup)` at supply (the supplier's policy as the occurrence opened). Each is a delayed edge the specification names. A fourth occurrence is a bug until a sentence in the chapters is found that it implements, and then it is a fourth named place.

**`switchHoldPromptly`, `switchPromptlyDyn`, `switchHoldPromptOnly`, `coincidence`, except the birth term.** The evidence is Reflex's own documentation. On `switchHold`: "Because the simultaneous firing case is irrelevant, this function imposes laxer "timing requirements" on the overall circuit, avoiding many potential cyclic dependency / metastability failures. It's also more performant. Use this rather than 'switchHoldPromptly' and 'switchHoldPromptOnly' unless you are absolutely sure you need to act on the new event in the coincidental case." On `switchPromptlyDyn`: "Prefer 'switchDyn' to this where possible. The timing requirements that switching before imposes are likely to bring down your app unless you are very careful." On `coincidence`, after the definition: "You maybe looking for '@switchHold@ @never@' instead." Prism took the advice as a rule, `!Switch` is `switchHold never`, and the evaluator may not spell `!Switch` any other way. The one `coincidence` is in `for`'s birth term, where the cycle check has already made the timing requirement hold, and it is marked with a comment naming the law it implements. A `coincidence` anywhere else, or a `switchHoldPromptly` anywhere, is a reviewer's finding.

**`tagPromptlyDyn` and `attachPromptlyDyn`.** The documentation of `tagPromptlyDyn` is the clearest statement of the tag law in the Reflex sources, written as the warning against its own function: "@/tagPromptlyDyn d e/@ differs from @/tag (current d) e/@ in the case that @/e/@ is firing at the same time that @/d/@ is changing. With @/tagPromptlyDyn d e/@, the __new__ value of @/d/@ will replace the value of @/e/@, whereas with @/tag (current d) e/@, the __old__ value will be used, since the 'Behavior' won't be updated until the end of the frame. Additionally, this means that the output 'Event' may not be used to directly change the input 'Dynamic', because that would mean its value depends on itself. __When creating__ __cyclic data flows, generally @/tag (current d) e/@ is preferred.__" Prism has only the old-value form. The evaluator's `Tag` node is `attachWith` over `current`, and the prompt forms do not appear.

**`getPostBuild` and `PostBuildT`.** Its frame is "determined by the instance", and its purpose is safe sampling, which Prism does not have. The stack has no `PostBuildT`, so `getPostBuild` does not type-check in `PrismM`, which is the cheapest kind of review.

**`performEvent`, `performEvent_`, `performEventAsync`.** The documentation says (from memory) "the output 'Event' will generally occur later than the input 'Event', since most 'Performable' actions cannot be performed during 'Event' propagation", which is the creation rule's spirit with the wrong party performing. In Prism the program performs nothing; the World performs request firings after the occurrence commits. `PerformEventT` is in the stack for `Adjustable`, and a `performEvent` call in the evaluator would be the program doing an effect, which no sentence in the chapters allows.

**`newTriggerEvent` and `TriggerEventT`.** A trigger is how a frame starts, and only the World starts one. The two triggers that exist are the root's responses trigger and the per-cell push triggers, both made with `newEventWithTriggerRef` in the host and fired only from `run`. A `newTriggerEvent` in a widget would let a program start an occurrence, and `Defer` is the only way a program has to cause a later occurrence, through the World.

**`Requester` and `MonadQuery`.** Elements replace both, and the correspondence appendix says what each exchange buys. `Incremental` is used inside `for` and the merges and never surfaces as a `Val`.

How review catches them: `Prism.Eval` is one module of a few hundred lines, the checklist is a comment at its top, and a CI step greps the module for each forbidden name and for `current`, `sample` and `coincidence`, counting the sanctioned sites; the step fails on any new one. That is not a proof, and it is the right amount of machinery for a module whose author is supposed to read the chapters first.

## The Test World and the transcript printer

The Haskell world core is the same few hundred lines as the Rust one, in `Prism.World`: the turn loop, `Defer`, snapshot and push bookkeeping, slot tracking and the discarding of late results, `Unsupported`, error reporting, and the transcript printer. It is written against the run call and nothing else, so it would work over any runtime that had one; there is one, and the Test World is the core plus a component that answers to a script.

The turn loop is the specification's. Gather the turn's inputs into one `RunIn`; call `run`; from the output, collect the `Defer` slots that fired; if there are any, run another occurrence delivering their responses, with no other input, and repeat; a `Defer` answered twice in one turn is a `DeferBackwards` error; when no `Defer` fired, the turn is over and the World acts on the last output. The snapshot is held still across a turn: pushes enter with the first occurrence only, and the oracle answers from the previous snapshot throughout. A failed turn drops its responses and re-queues its pushes. Request firings from a slot that the last list no longer contains are performed if the component can and their responses discarded. All of this is the world core chapter's content and is not different in Haskell; what differs is only that it is twenty lines shorter, because `Map` and `Either` are closer to hand.

The Test World's component accepts every tag. For a `requests` field it records what fired, for `requested`; for a `responds` field it holds nothing, since the script's `send` steps supply the responses; for a `state` field it answers the oracle from the script's `with` steps and turns a later `with` or `push` into a push. It knows no tag's meaning, which is what lets a program with a `Net.Listener` run under it. The script is parsed by `Prism.Parse` as part of the test file, because a `test` is a top-level statement whose steps are an array of plain values, and the harness is a function from a blueprint and an array of steps to a transcript and a verdict:

```haskell
runTest :: TypedAst -> Test -> IO (Transcript, Verdict)
runTest ast (Test widget turns) = do
  rt <- runSpiderHost (loadProgram ast widget)
  w  <- newTestWorld rt
  foldM step (turn0 w) steps
```

Each step is one of the suite's: `expect`, `expect_top`, `expect_none`, `expect_count`, `expect_top_none`, `send`, `click`, `with`, `push`, `returns`, `fired`, `quiet`, `requested`, `unrequested`, `errors`. `expect` reaches inside container elements by the GUI library's convention (`Ui.VBox`'s `children: Path ==> Element`), because the library is part of the suite. `send` finds exactly one matching slot in the last list, delivers the response by that slot's path, and runs a turn; `Together` delivers all of its `send`s in one occurrence and fails if two address one slot. `with` may precede the build, and must for any state tag the program creates at build, since there are no declared defaults. `returns` on an event result is a script error, as the suite says.

Only the transcript printer has to agree with the Rust one character for character, and the suite's format defines it, not either implementation:

```
turn 0
  list:
    0/#0/0: Ui.Button(label="Increment")
    1/#0/0: Ui.Button(label="Decrement")
    2/#0/0: Ui.Label(text="Count: 0")
  result: unit
turn 1  responses: {0/#0/0: click}
  list:
    0/#0/0: Ui.Button(label="Increment")
    1/#0/0: Ui.Button(label="Decrement")
    2/#1/0: Ui.Label(text="Count: 1")
  result: unit
```

A turn header names the turn and its inputs, `responses` by path and `pushes` by element value. The list is one slot per line, `path: element`, in tree order, which is the order the `Map Path` iterates in because `Ord Path` is defined to be tree order: bind positions compare numerically, generations numerically, keys by the key's own ordering, with a supplied segment `s/n` ordering after the binds of its node. A slot that fired its request gets a `requested` line after it with the values in firing order. An occurrence boundary inside a turn, which only `Defer` causes, is `occurrence 2` between the parts. The result is printed by its current value for a plain or dynamic result and as `fired: v` or `quiet` for an event result. An aborted occurrence prints `error: Error(kind=Cycle, spans=[[...]], occurrence=1)` and no list.

Element values print as their constructor with named fields in declaration order, which the tag table gives; plain values print as Prism literals, with `Text` escaped the way the lexer reads it, `Float` in the shortest round-tripping decimal, arrays as `[[a, b]]`, tuples as `((a, b))`, and maps as `{k: v, ..}`. Every spelling decision here is a decision both printers must make the same way, and a `diff` on the counter's transcript is the first test the Haskell printer runs, before any program with a `Defer` or a `for` is attempted.

Paths are where the printers most easily disagree. The Haskell `Path` is a list of segments allocated by the evaluator's environment and the Rust one is an `Rc<[Segment]>`; both print `0/#1/0`, `[k]` and `s/n` because the transcript format says so, and the first `for` test with a space in a key will find out whether both escaped it alike.

## The reflex-dom World

The reflex-dom World is the alternate GUI World and the only place the Haskell implementation does something the Rust implementation cannot yet: it puts a Prism program on a screen. GUI is not a World concept; there is one GUI vocabulary, `Gui.*`, that any World with pixels and a pointer can speak, and the Prism GUI library, with its layout, hit-testing, focus and text editing, runs on top of it unchanged. The reflex-dom World speaks the vocabulary to a browser.

The vocabulary, as the World sees it:

| Tag | Part | What the reflex-dom World does with it |
| --- | --- | --- |
| `Gui.Window` | `title`, `size`; responds `WindowEvent` (`Resized`, `CloseRequested`, `Pointer`, `Pressed`); one per window | the document for the first slot; a popup for further ones. Listeners on the window element turn pointer, key and resize DOM events into `WindowEvent` responses by the slot's path. |
| `Gui.Scene` | presence: `window` (the title it paints) and `draw: [Draw]`, the draw list as data | rendered to the DOM after each turn, one entry of its `draw` field per element: `Draw.Rect` to an absolutely positioned `div` with a background, `Draw.Text` to a `span` at its origin, `Draw.Image` to an `img`, `Draw.Clip` to a `div` with `overflow: hidden` and the clipped entries inside it. |
| `Gui.Focusable` | presence: `name` only, no responses; focus is a library concept | an invisible element with `tabindex` and `aria-label`, so that the browser shows its focus ring, has something to focus for the input method's candidate window and a screen reader has a name to read. Its key DOM events bubble to the window listener and become `Pressed` responses addressed to the `Gui.Window` slot's path, not the focusable's; the library routes them to the input whose name is the focus, and emits at most one focusable, the one it holds as focused. |
| `Gui.Metrics` | `text`, `font`; state `Extent`, answered synchronously | the oracle measures with a hidden `canvas`'s `measureText` for the element's text and font; the push fires when the fonts change, which in a browser is `document.fonts`' `loadingdone`. |
| `Gui.Frames` | responds `Frame(now: Instant)` per frame while a slot exists | `requestAnimationFrame` re-armed after each turn while a `Gui.Frames` slot is in the list; a tick is a turn delivering `Frame(now)` to the slot's path. |

The World is itself a small Reflex application, which is the natural way to write it in this language and a nice symmetry: the Prism runtime is a Reflex network on a local timeline, the renderer is a reflex-dom network on the global one, and the run call is the seam. The renderer's input is a `Dynamic t (Map Path Element)`, fed from the run call's output through a trigger after each turn; `listWithKey` over it draws one DOM subtree per slot, and `reflex-dom`'s own diffing does what a GUI World's draw-list diff does in Rust. DOM events become responses: a `click` on the window element becomes a `Pointer` response addressed to the `Gui.Window` slot's path, a `keydown` becomes a `Pressed` response addressed to the `Gui.Window` slot, a `resize` observer becomes a `Resized` response to the window, and each is one turn. The World does no geometry: hit-testing is the library's, from the draw list it emitted and the pointer position the window reported, which is why the window responds with pointer events and the draw entries, which are values and not elements, have no events and no paths.

A turn in this World, end to end: the user clicks. The window listener makes `{window_path: Pointer(Down(((x, y)), Left))}`, the world core runs the turn (the run call plus any `Defer` occurrences the library's focus handling needs), the output list has a new `Gui.Scene` with a pressed button drawn differently, the renderer's dynamic is updated and reflex-dom patches two `div`s. Nothing was drawn because an element appeared; the scene is a description, read after the turn.

The `Ui.*` tags never reach this World. `Ui.Button`, `Ui.Label` and `Ui.TextInput` are the library's, handled by its interceptors, which turn them into `Gui.Scene` entries and focusables before the list reaches the root. A native-toolkit World might accept them directly; a browser World could, by mapping `Ui.Button` to a `<button>`, and that is a different World with higher tags, not this one. This one proves the sentence "layout, hit-testing, focus and text editing are Prism interceptors" by running them in a place where the host has its own opinions about all four and is not asked for any of them.

The World runs under GHC's JavaScript or WebAssembly backend through jsaddle, or natively through WebKitGTK with the same code (from memory). The Prism program's typed AST is data the page loads, so there is no compiler on the page and no GHC per program, which is the interpreter decision paying again.

## Differential testing

Everything in the suite runs under both implementations, and the differential test is `diff`. Two commands produce the two interchange formats: `dump` prints the typed AST in the text form and `test --transcript` runs every test in a file under the Test World and prints the transcript of each. The harness runs both commands under `prism` and `prism-hs` over every file in the suite, the layout corpus's token files included, and reports every byte that differs.

An empty diff is a precise claim: that both implementations read the chapters the same way, for every program in the suite, at the level of what the compiler chose and at the level of what a World sees. It is not a claim that either is right. The conformance scripts make that claim, and the differential test adds the second reader.

A divergence is one of two things. One implementation has a bug, which the law test pinning the relevant sentence will usually have caught already and the diff localises to a construct; or two careful readers read a sentence differently, and the sentence is ambiguous. The second kind is the find the whole exercise exists for. Its procedure: rewrite the sentence so that it admits one reading, add a law test pinning that reading, bring both implementations to it. A divergence is never resolved by making one implementation copy the other without a sentence to point at; that turns the second reading back into the first.

Three kinds of divergence have a predictable home. A path printed differently is a reading of "its position in the widget tree" and belongs to the transcript format's definition, not to either runtime. A dump that differs in a lift or a numeric type is a reading of the lifting rule or the inference rule, and the goldens, which both frontends must match, answer it before the transcripts can. A transcript that differs in which occurrence of a turn a request fired is a reading of the `Defer` rule or the creation rule, and the world core tests answer it. Everything else is new, and new is what the test is for.

The Haskell implementation is a particularly good second reader on the sentences a height-ordered runtime finds hardest, because Spider resolves them differently. The mid-occurrence construction rule is `subscribeAndRead` in Spider and catch-up caching in Rust; the two agreeing on a nested `dyn` over a dynamic that changed in the same occurrence, building once with the new value, is two independent derivations of one transcript. `for`'s prompt consolidation is a `coincidence` term in Haskell and a merge with a prompt birth contribution in Rust. `$Join` is Reflex's `join` in Haskell and a hand-written `Dynamic::join` with three law tests in Rust, and the third test, an outer switching to an inner that does not change, is the kind of case a differential run finds before a person thinks to write it. When the Rust side fails a script, the first question is whether the runtime is wrong or the script is, and the Haskell transcript answers it before anyone reads the runtime.

Random programs are the natural extension once the suite agrees: a generator of well-typed Prism programs, run under both with random scripts, with any transcript difference a finding, the way Rustlantis runs random MIR on several backends (from memory). The harness takes a directory of files, so a generator can feed it without changes.

The differential test runs on every change to either implementation and on every change to the book, since a change to a sentence is a change to what the goldens and transcripts should say. When a sentence changes and the diff stays empty, both implementations have the same bug, and only a law test can find it; that is why there are both.

## What it cannot check, and what it alone can

Some limits are chosen by the design and buy something; others are only where the plan stops.

It cannot check the Rust runtime's ownership. Prism's garbage collection in Rust is the ownership tree: a node is dropped by its parent and its subscriptions die as weak pointers. Haskell has a collector, Spider's nodes are kept alive by whoever references them, and the evaluator never thinks about it. A leak in the Rust runtime, a `trace` that keeps a node alive, an `Rc` cycle through a closure, shows up in no transcript. The Rust runtime's invariant checker and its weak-count tests are the defence, and the Haskell implementation does not help. This buys an evaluator with no lifetime code in it at all, which is the kind of second reading that cannot share an ownership bug with the first.

It cannot check the closure compiler, the lint, or the values representation. It has none of them. A capture list that names the wrong variable, a placeholder closed twice, a `Plain` downcast at the wrong type: the Rust lint and goldens catch these. It does catch a wrong *typed AST*, because it executes the typed AST directly, so a Rust compiler bug that is in the dump is also in the Haskell transcript of that dump, which localises it to the frontend.

It cannot check performance, and does not try. The evaluator allocates a `Map` per environment extension and a `Val` per value, and an occurrence under it costs whatever Spider costs plus that. The suite's programs are small and this is fine; a benchmark is not a test the Haskell implementation runs.

It does not check any World but two, because the plan builds it no others. The CLI World, the software rasteriser, the X11 World and the tokio skeleton are Rust, tested in Rust against the Rust world core; a Haskell CLI World would speak the same plain-data run call and needs nothing the design lacks. The Haskell world core is tested against the same fake-component tests, which is the one place World testing is duplicated, and it is duplicated because the turn loop is the part of a World the chapters specify.

It cannot continue after an error the way a Rust World with quarantine can. Rollback by replay gives it the transaction law; quarantine is a World policy this plan does not give it.

The other side of the ledger has four entries, and nothing else in the plan provides any of them.

It is an executable statement of which Prism laws are Reflex's. Every row of the correspondence table that maps a construct to a Reflex combinator with no evaluator code around it is a law Reflex already had; every row with code around it (`for`'s birth term, state supply, the fan, the run call, replay) is a law Prism added. That list, as code that runs, is more exact than any appendix, and it is what a reader who wants to move a law can consult to see whether moving it means changing an engine or changing a hundred lines.

It is the cheapest place to try a semantic change. A change to what `!Switch` means is one `case` arm in the evaluator against a runtime rewrite and a compiler change on the Rust side; a change to `for`'s consolidation is one function; a change to the run call's inputs is one record. The law tests say what the change did, under both implementations, and a sentence can be tried before it is written.

It is the first Prism on a screen. The reflex-dom World is a few hundred lines over a renderer that already exists, and it runs the GUI library, which means the library's layout and focus and text editing can be seen and poked before the Rust rasteriser exists, and the library's bugs found with a mouse rather than a script.

And it is the oracle when the Rust runtime is wrong in a way no script anticipated. The hard bugs in a height-ordered runtime are the ones around construction in the middle of an occurrence: a nested `dyn` rebuilt in the occurrence its parent is rebuilt in, a late subscriber's view of a firing earlier in the same occurrence, a cycle that closes only after a switch adopts a new inner. Each of those is a question Spider has an answer to, and reading Spider's source to find the answer is slow and error-prone. With the Haskell implementation, the reading is a run: write the script, diff the transcripts, and the sentence the Rust runtime got wrong is the one the law test names.

## The smallest version that pays for itself

The whole chapter describes the finished implementation. The smallest one that earns a differential test is a fraction of it, and the order in which the rest arrives is set by which law tests it can run.

*Skip the frontend.* The Rust frontend already prints the typed AST in the text form, and `Prism.Typed.Text` has to parse that form anyway. So the first Haskell binary reads the text form and nothing else: no lexer, no layout, no parser, no elaborator. The differential test still runs, Rust frontend into both evaluators, and every law test exercises the Reflex evaluator from day one. The Haskell frontend is a second project with its own diff, added when the evaluator is believed.

*The evaluator for the counter.* Values are one `Value` sum type, as in the runtime chapter; `Event t Value`, `Dynamic t Value`, `Behavior t Value`. The counter needs five nodes: `hold` is `holdDyn`, `tag` is `attachWith` on `current`, `leftmost` is `leftmost`, lifted application over a dynamic is `fmap`, and `@Element` with a response is a `newTriggerEvent` keyed by path plus a `DynamicWriter` entry for the element list. A widget block is a `RecursiveDo` block, because the counter's hold refers to itself, and the resolver's binding groups say which bindings go in one `rec`. That is enough for the tag law, the leftmost law, the hold-in-build-occurrence law, and the glitch-freedom diamond.

*The Test World.* `runSpiderHost`, one trigger per response path and one per state element, a turn is `fireEventsAndRead` over the turn's triggers, and the transcript printer reads the element list `Dynamic` and the collected request firings after each turn. `Defer` is a loop: after a turn, read the `Defer` firings, fire their triggers, repeat until none. That is the whole World, and it is under two hundred lines.

*Then, in this order, because each unlocks a block of law tests.* `switchHold` for `!Switch` and `join` for `$Join`, which unlock the switching laws. `networkView` for `dyn`, with the rebuild's element list substituted in the parent's `DynamicWriter`, which unlocks the construction-in-the-middle laws. `listWithKey` over a `Dynamic (Map Key Value)` for `for`, with consolidation as `mergeMap` on the blocks' results, which unlocks the keyed-list laws. `intercept` last, as `runDynamicWriterT` on the child plus a `fanMap` of the responses by path, which unlocks the element laws. Errors and the replay-based rollback come after all of that, because until then a failing test is a bug in the evaluator and not a rollback question.

What this leaves for the day the evaluator is believed: the frontend, `reflex-dom`, and the replay log's memory. None of them is needed to find a disagreement between the two readings of chapters 3 through 7, which is the only reason the implementation exists.

## What the Haskell implementation leaves out

The negative space, in one place, with what each exclusion buys.

No compilation of Prism to Haskell source. Bought: no second compiler to be wrong in, no GHC per program, a typed AST that runs without a Haskell frontend.

No typed representation of the typed AST. `Val` is a sum and projections fail with "elaborator bug". Bought: one `case` arm per node kind, and every type question kept in the frontend where the goldens are.

No `PostBuildT`, `RequesterT`, `QueryT`, `TriggerEventT` in the stack. Bought: `getPostBuild`, `requesting`, `queryDyn` and `newTriggerEvent` do not type-check in a widget block, so the review for them is the compiler's.

No `performEvent`, no `liftIO` reachable from a block. Bought: a program under the Haskell implementation can do exactly what a program under the Rust one can, which is nothing, and the transcript is the whole of what it did.

No prompt switch, no prompt tag, no post-build. Bought: the laws of the switching chapter hold by the choice of combinator, and the warnings in Reflex's own documentation are the argument.

No transaction inside Spider. Bought: no patch to Reflex; rollback by replay, which is the determinism law doing the transaction's work and testing itself.

Real Worlds, quarantine, benchmarks and production use of the Haskell implementation are outside this plan. Nothing in its design rules them out: a Haskell World speaks the same plain-data run call as a Rust one, and the evaluator over Spider is a reasonable place to run real programs. The plan stops at the Test World, which the differential test needs, and the reflex-dom World, which puts the GUI library on a screen early; there is little to say about the rest until someone wants it.
