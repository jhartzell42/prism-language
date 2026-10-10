# Lint: ch02-language-core.md

Line numbers refer to /home/claude/book/ch/ch02-language-core.md as read.

## Part 1: rule violations

### Rule 1 — history / superseded designs (3)

- L161: "A spelling like `.<` for less-than was considered and rejected because a program with `<=` meaning hold and `<` meaning less-than is a program whose most important line looks like a typo." — reports a past decision process. Fix: "Suppose less-than were spelled `.<`. Then a program with `<=` meaning hold and `.<` meaning less-than is a program whose most important line looks like a typo."
- L600: "a rule that made the counter uglier would need a better reason than any that has turned up." — "has turned up" refers to the history of the design. Fix: "a rule that made the counter uglier would need a better reason than any of these."
- L920: "so that a reader of an older description who tries one learns that it is intended rather than misspelled." — refers to earlier documents. Fix: "so that a reader who tries one learns that it is intended rather than misspelled."

### Rule 2 — status tracking (0)

None. L136 ("no hexadecimal ... forms yet; the day a program needs `0xFF`, this is where it goes"), L494 and L674 say why something is tentative without tracking status, which is the allowed form.

### Rule 3 — quoting or attributing Jimmy (0)

None.

### Rule 4 — banned words (1)

- L19: "### Composition" — "composition" as the vague noun for sigil stacking. Fix: "### Stacking" (the body already says "The sigils stack").

`shape`/`Shape` at L235, L410, L435, L679, L1003 are a variable and a type name in examples: fine.

### Rule 5 — clefts and non-default word order (17)

- L17: "The goal served is the one that says you can see what moves." Fix: "This serves the goal that you can see what moves."
- L44: "What they hold is whatever it is." Fix: cut (the previous sentence already says it), or "They hold whatever they hold."
- L56: "what this chapter needs is only that the sigils are the names of those three relations to time" Fix: "this chapter needs only that the sigils name those three relations to time".
- L58: "That last point is the one that makes the aggregate rule above work." Fix: "That last point makes the aggregate rule above work."
- L74: "The rule that a name says what it returns is the rule that a call site can be read without the callee." Fix: "Because a name says what it returns, a call site can be read without the callee."
- L122: "The free form is what lets a sigil act as `return` on a whole expression, and the space is what says so." Fix: "The free form lets a sigil act as `return` on a whole expression, and the space says so."
- L319: "The outermost rule is the one under which the common case, ..., needs no parentheses, and the uncommon case, ..., needs exactly one pair." Fix: "Under the outermost rule the common case, a call whose arguments are values and one-argument calls, needs no parentheses, and the uncommon case, a nested multi-argument call, needs exactly one pair."
- L377: "What this is not: it is not a general trait system." Fix: "It is not a general trait system."
- L477: "the rule is what makes forgetting to forward a compile error rather than a silently dropped element." Fix: "the rule makes forgetting to forward a compile error rather than a silently dropped element."
- L498: "The split between the value layer and the functor layer is the thing to understand about inference here." Fix: "Inference here splits into a value layer and a functor layer."
- L502: "Lifting is what the sigils buy." Fix: "The sigils buy lifting."
- L540: "That one operator and the lifting rule together are what Reflex spells as `tag`, ..." Fix: "Reflex spells that one operator plus the lifting rule as `tag`, ..."
- L544: "That is what turns the counter's `$count <- 0 <= $count + !delta` from an equation into a definition" Fix: "That turns the counter's ... from an equation into a definition".
- L566: "The thing to know about application in the precedence order is where it sits:" Fix: "Application sits at one place in the precedence order:"
- L578: "It is also what makes `@Label $"Count: $count"` and `intercept @_, (!Union !kid_ins, !fwd_ins)` read the way they look." Fix: "It also makes ... read the way they look."
- L600: "`on` tighter than `|` is the choice that makes a merge of samples read without parentheses" and "`on` looser than arithmetic is the choice that makes `$count + 1 on !click` mean ..." and "Both choices are the ones under which the first program anyone writes needs no parentheses". Fix: "`on` tighter than `|` lets a merge of samples read without parentheses ... `on` looser than arithmetic makes `$count + 1 on !click` mean ... Under both choices the first program anyone writes needs no parentheses".
- L918: "The second message's list of open columns is what turns a layout error from a puzzle into a one-keystroke fix." Fix: "The second message's list of open columns turns a layout error from a puzzle into a one-keystroke fix."

### Rule 6 — lecturing tone (0)

None. The numbered list at L185–203 is a procedure the lexer executes.

### Rule 7 — dry definition before motivation (4)

- L126 "### Keywords" opens with a bare keyword list. Lead with the one case that explains why the list has the odd entries it has: "Suppose `unit` were an ordinary prelude name. Then a program could shadow it and `@ unit` would mean something else." Then the list, then the `ix`/`key` reasoning already at L132.
- L345 "### Numbers by inference, and the three traits" opens with "Three things are overloaded in Prism and nothing else is ... They are traits, in the Rust sense" and a bullet list; the motivation (the counter's most important line reading `+~`) sits at L364. Open with `$count <- 0 <= $count + !delta` and the question of what `+` is there.
- L381 "### Aggregates" opens with three bullet definitions; the point ("This language prefers maps wherever identity matters", the insert-at-the-front example) comes at L395. Open with the insert-at-the-front example, then the definitions.
- L484 "### Type annotations and inference" opens with "Types are inferred by Hindley–Milner over the value layer". Open with the error you get from a module whose helper has no annotation and whose type was guessed from a use three files away, then the rule.

### Rule 8 — presence or drop as a trigger (0)

None.

### Rule 9 — conflicts with the BIBLE Decisions (7)

- `Gui.Button`, `Gui.Label`, `Gui.Row`, `Gui.Text.Input` used as element tags with `label`/`text`/`height` fields: L460–462 (`tag Gui.Button of Element: label: Text; responds: Click`), L490, L551, L558, L660, L692 (`Gui.Button(label=l)`), L693 (`Gui.*(..)`), L845, L892 (`Gui.Row(height=xs[i])`), L1088–1089 (`Expect Gui.Label(text="Count: 0")`, `Send Gui.Button(label="Increment"), Click`), L1182 (`Gui.Text.Input`). Decisions: `Gui.*` is only `Gui.Window`, `Gui.Scene`, `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames`; buttons and labels are `Ui.Button`, `Ui.Label`, handled by the library. Fix: `Ui.Button`, `Ui.Label`, `Ui.Row`, `Ui.Text.Input`, `Ui.*(..)` throughout; L476's hypothetical "adding a tag `Gui.Window` to a library" becomes "adding a tag `Ui.Window`" (`Gui.Window` is a real primitive).
- L333–334: "`Nat`: unsigned, machine width." / "`Int`: signed, machine width." Decisions: `Int` is 64-bit signed and `Nat` 64-bit unsigned. Fix: "`Nat`: unsigned, 64-bit." / "`Int`: signed, 64-bit."
- L348: "`Ord`: primitives, text, chars, tuples and arrays of ordered types, and declared types whose parts are all ordered (a `choice` in declaration order, then by payload)." Decisions: records have equality when all fields do and no ordering. Fix: "`Ord`: primitives, text, chars, tuples and arrays of ordered types, and a `choice` whose payloads are ordered (declaration order, then payload). Records have no ordering."
- L478: "**Equality** is structural, tag then fields, and is `Eq` when every field is; `Ord` is by tag name then fields." Decisions: element equality is tag plus plain fields, excluding any request event; records (and so elements) have no ordering. Fix: "**Equality** is structural, tag then plain fields, with the `requests` event excluded, and is `Eq` when every plain field is. There is no `Ord` on an extensible variant."
- L1089: "`Send Gui.Button(label="Increment"), Click`" — the value of a unit type is lowercase (`type Click = click`, value `click`), as L416 and L439 of this chapter say. Fix: "`Send Ui.Button(label="Increment"), click`" (and in the corpus stream at L1094).
- L130: "`!never`, `@started`, `@main`, `!Switch`, ... These are ordinary names in the prelude". `@main` is the program's entry widget, defined by the program, not a prelude name. Fix: drop `main`/`@main` from the list, or say "`@main` is the name the program gives its root widget".
- L666–667: "`!resp <- @Element _.element` / `@ resp`" — the catch-all form in the Decisions is `(!resp, ignore) <- @Element _.element` (the generic `@Element` returns `@(!Response, $State)`). Fix: "`(!resp, ignore) <- @Element _.element` / `@ !resp`".

### Rule 10 — unverified specifics (4)

- L540: "what Reflex spells as `tag`, `tagPromptlyDyn`, `attach`, `attachWith` and `<@`" — `tagPromptlyDyn` and `tag (current d)` are in research-notes; `attachWith` appears in /tmp/claude-0/rx/Query.hs; `attach` and `<@` are in neither. Verify or mark from memory or cut the two.
- L502: "`combineLatest([a$, b$]).pipe(map(([a, b]) => a + b))`" — RxJS `combineLatest`/`pipe`/`map` are not in research-notes. Verify or mark from memory.
- L122: "Suppose instead the sigil were always attached to whatever token followed it, as Perl does." — not in the notes, and Perl accepts whitespace between a sigil and its name, so the attribution is doubtful. Verify or cut "as Perl does".
- L179 ("Haskell's approach, where `-` after an expression is always binary and a negative argument is written `F (-1)`"), L205 ("Haskell's `{ ; }` fallback"), L364 ("one operator per type, as OCaml does", "Haskell's `Integer` default"), L578 ("application binding tighter than every operator as in Haskell"): language-behaviour claims not in the notes. Mark from memory (they are standard, but the rule is the rule).

### Rule 11 — code that cannot be Prism (11)

- L306: "An inner call with more than one argument is parenthesised: `F a, (G b, c)`." and three sentences later "`(F a, b)` is a tuple of `F a` and `b`, not a call of two arguments." These contradict each other: by the second sentence `(G b, c)` is a tuple. Fix: pick one. The simplest consistent rule: inside parentheses commas belong to the call whose name opens the parenthesis, so `(G b, c)` is a two-argument call and a tuple of a call and a value is written `((G b), c)`; then state the tuple rule as "a parenthesised list whose first element is not a paren-less call is a tuple".
- L312: "`Concat [(Prefix e.text, e.cursor), ToText c]   # two array elements; the first is a two-argument call`" — by L306's tuple sentence the first element is a tuple. Fix follows the L306 decision.
- L527: "`Insert ($items, $next, todo) on !add`" — by L306 `Insert` is applied to one 3-tuple; `Insert` takes three arguments (L309). Fix: "`(Insert $items, $next, todo) on !add`".
- L573, L575: "`!Filter (HitTest ($slots, !pointer))`" — `HitTest` is applied to a tuple by L306; the comment says "nested one-argument calls", which is only true if `HitTest` takes a tuple. Fix: "`!Filter (HitTest $slots, !pointer)`" and comment "a two-argument call inside a one-argument call".
- L437: "`Meters 3.0 +. 1.0` is a type error, which is what a newtype is for." By L566 a paren-less call extends to the end of the expression, so this is `Meters (3.0 +. 1.0)`, a well-typed `Meters`. Fix: "`(Meters 3.0) +. 1.0` is a type error".
- L441: "`type Pair<a, b> = record: first: a; second: b`" — `;` is not a token (L150–159) and L205 says there is no explicit-separator form of a block. Fix: write it as a layout block, or as a one-line map-style literal if records admit one (they do not, per L423).
- L377, L441, L650: "`let Max<a: Ord>: fn (a, a) -> a`", "`let Step: fn (Watch, Instant, Action) -> Watch = \|w, now, a| ...`" — L494 says function types are `fn A, B -> C`, comma-separated like a call; `fn (A, B, C) -> D` is a function of one tuple, but the lambda at L650 takes three named arguments. Fix: "`fn a, a -> a`", "`fn Watch, Instant, Action -> Watch`".
- L666–667: "`!resp <- @Element _.element` / `@ resp`" — the bound name is `!resp`, the returned name is `resp` with no sigil, which L820 says is an error. Fix: "`@ !resp`" (and the catch-all pattern per rule 9).
- L750: "`let doubled = for prices: key, _ *. 2.0        # a map, keyed as the input`" — a bare `a, b` is not a tuple (tuples are parenthesised, L383) and commas belong to calls; and if the result is "keyed as the input" the body should produce only the value. Fix: "`let doubled = for prices: _ *. 2.0`".
- L1118/L1122/L1125: "`let b = Filter items, \.done`" with stream "`\ .pre done`" and "`.pre` after `\` is field access on `_`". By the spacing rule at L165 a `.` is prefix only after whitespace, an open bracket, a comma or line start; after `\` it is binary. Fix: add `\` to the list at L165 ("preceded by whitespace, an open bracket, a comma, `\`, or the start of a line").
- L645: "`.x` means `_.x` and `[i]` means `_[i]`, by the spacing rule: a `.` or `[` in prefix position applies to the pronoun." `[` is not among the tokens the spacing rule covers (L177: `+`, `-`, `.`, `+?`), and under this reading `F [1, 2]` would be `F` applied to `_[1, 2]` rather than `F` applied to an array literal. Fix: either add `[` to the spacing rule with an explicit statement that `F [1, 2]` is an array argument (prefix `[` applies to the pronoun only directly after `\`), or drop `\[0]` and require `\_[0]`.

### Rule 12 — filler (2)

- L103: "Sigils stack leftmost-outermost in all three positions, as the first section describes." — restates L21. Fix: cut, or fold into the previous sentence ("On a type, it is the type constructor: `!Text`, `$[Item]`, `@!Click`; sigils stack leftmost-outermost here as elsewhere.").
- L562: "The shorthand is used freely from here on and means exactly the expansion. The expansion is the real thing: anything the shorthand does, the hand version does with the same primitives, and the hand version is what a program writes when it wants the `dyn`'s event or a different initial value." — the second sentence restates the first. Fix: keep "The shorthand is used freely from here on and means exactly the expansion; a program that wants the `dyn`'s event or a different initial value writes the expansion."

## Part 2: structural notes

1. **The paren-less call grammar is not one rule.** L306 gives two incompatible readings of `(F a, b)`, and the chapter's own examples split: L309 and L688 parenthesise multi-argument inner calls as calls, while L527, L573 and L312 write what the tuple sentence says are tuples (the integration tests in /home/claude/prism-tests use `Insert ($names, "c", "c")` too). Same tension in function types: L494 says `fn A, B -> C`, L377/L441/L650 write `fn (A, B) -> C`. A reader implementing the parser in months needs one sentence that decides parenthesised calls, tuples, one-tuples `(a,)` and `fn` argument lists together, plus a corpus entry for each (see 8).

2. **Method form and the spacing rule do not agree.** L317 says `x .F` is "`x` applied to `_.F`", then reads `items.Filter IsDone .MapValues Label` as `MapValues (Filter items, IsDone), Label`, which under the call-extends-to-the-end rule and the "`.F` applies to the pronoun" rule it is not (`.MapValues Label` would be an argument of `Filter`). L1125 adds a third story ("which the parser resolves as the chain's left operand"). State one rule for a chain: e.g. a prefix `.Method` following a complete call closes that call and takes it as receiver, which is a parser rule, and say that `_` is not involved. Then the `\.element` / `\[0]` pronoun forms (L645) need their own sentence (see rule 11, L645).

3. **Top-level definition forms are not stated.** L484 says top-level definitions take a full annotation and shows `let Magnitude: ...`; L488, L658, L663 write `@Button: fn $Text -> @!Click = \@widget:` with no `let`; L212, L937 write `@counter:` (a widget block, no annotation, no `let`). Corpus item 5 (L1015) says the parser reads an annotation "because the statement began with `let`", which does not cover `@Button: fn ...`. Say which forms exist at top level (`let name: T = e`, `@name:` block, `@Name: T = e`?), whether `let` is optional for sigiled names, and how `@counter:` (line-final colon, block) and `@Button: fn ...` (inline colon, annotation) are told apart. Also missing: that the last statement of a widget block is its result, and what a bare non-final expression statement means (L610 shows `@Element Time.Every(period=100)` as "bind with the result discarded", but L606 lists only bind, let and hold).

4. **Decisions not yet in the chapter.** The Decisions say field access and indexing lift like application (`!!panes.!inbox`), a trailing lambda as the last argument absorbs the rest of the line (L651 relies on it), `Wrap.*` and `Checked.*` prelude functions exist for arithmetic, `!Updated` is a plain function giving a dynamic's change event, and `$Distinct` is the only dedupe. The first two are grammar/lifting rules that belong in Application/Lifting; the third belongs next to "Overflow ... is a runtime error" at L341; the last two belong in "No sampling" (L1201 lists three reads of a dynamic and omits `!Updated`) and next to `Eq` at L347.

5. **`Count2Label` (L93).** Listed as an error that "mixes conventions", but the same paragraph allows digits after the first character (`Vec2`), and `Count2Label` is UpperCamelCase with a digit. Either it is legal, or the rule is "no uppercase letter after a digit" and should say so.

6. **`.value` twice (L437 vs L1201).** Newtypes are "unwrapped with `.value`"; "No sampling" says "There is no `.current`, `.value` or `sample` on a dynamic." Not a contradiction, but the negative-space entry reads as if `.value` were a thing the language does not have. Either rename the newtype accessor or add "on a dynamic; `.value` on a newtype is a field" to L1201.

7. **Generics are not lexable as written.** `Pair<a, b>`, `Max<a: Ord>` (L441, L377) use `<`, `>` and a `:` inside angle brackets. The operator list (L150–159) has no `<` or `>` token, L161 says `<=` is the only thing starting with `<`, and layout rule 5 (L193) suspends layout only inside `(`, `[`, `{`, so the `:` in `<a: Ord>` would be an inline colon. Add the tokens, add `<...>` to rule 5 (or spell bounds another way), and add a corpus entry.

8. **The corpus is a lexer corpus; precedence and calls have none.** L323 says two parsers must agree on every program and the only test is the counter's three lines at L598. The contradictions in items 1 and 2 are parse-level and would not be caught by token-stream goldens. Add a parse corpus (ten entries: nested calls with and without parentheses, tuple vs call, method chain, trailing lambda, `on` under `|`, `Size $elems + 1`, free sigil over a call, `match` arm with `let`s, `if` inside an argument, a generic annotation) with the parse sketched the way the token streams are.

9. **Invariants without an enforcer.** L329: "the program cannot tell copying from aliasing and the implementation is free to do either" — who is bound (the implementation; plain values have no identity) and what enforces it (no `Eq` on functions/functors, no reified nodes) should be said in place, since L1209 says the same thing from the other side. L753: "Over a dynamic collection it is the construct of the switching chapter and is only allowed in a widget block" — say the checker enforces it (the diagnostic at L922 does; link them). L562's "the first build happens in the occurrence the `dyn` is built" is a runtime law stated as a fact; name the law and the chapter it lives in.

10. **Duplication with other chapters.** The diagnostics "A cycle with no delayed edge" (L776–790) and "A hold outside a widget block" (L868–876) are about the functor laws and the error chapter's `Cycle` kind; keep the message forms here (this is where goldens are listed) but move the reasoning ("a `dyn` is not a delay") to the functors chapter and point at it. The "Lifting over `@`" expansion (L548–562) is the one place the BIBLE says to state it, fine; make sure the switching chapter does not restate it. The negative-space section says it defers to Part I; the "No sampling" and "No assignment" entries are long enough that one of the two places should be a pointer.
