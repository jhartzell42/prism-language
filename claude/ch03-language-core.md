# Chapter 3 — The language: sigils, lexical structure, values and types, expressions

A Prism program is a set of equations between things that are true, things that happen and things that exist, and the sigils on their names say which is which before a type checker runs. This chapter is the part of the language that holds still: how a line is cut into tokens and lines into blocks, what the plain values under the sigils are, and how an expression over mixed sigils finds its meaning without anyone writing `map`. Every rule here is written so that two parsers built from this text agree on every program, and every rule is argued from the goals it serves, so that a reader who wants a different rule knows what it would cost.

## Sigils and where time lives

You have read a line like `total = price * quantity` in an interactive program and not known, without scrolling, whether it ran once or every time the quantity box changed. You found out in production, because the label showed the old total. In a Reflex program you knew, because the variables were called `dPrice` and `eQuantity`, and you also knew that the author of Reflex asked you not to name them that way, and that everybody did anyway, because a line of code with no hint about which operand is live is a line you cannot review.

Prism makes the hint mandatory, uniform, and checked. A name's first character says what the name is:

- `x` is a plain value. It is true, now and always. It cannot change, it cannot happen, it cannot be torn down.
- `!x` is an event. It happens at some occurrences, with a value each time, and between them there is nothing to look at.
- `$x` is a dynamic. It is true now, and it will be different later, at some occurrence.
- `@x` is a blueprint. It describes a widget: something that can be built from it, that then exists for a while, talks to the outside, and is torn down. The book says "widget" for the built thing, the way everyone does, and "blueprint" for the value of type `@a` whenever the difference matters.
- `?x` is a plain value that may be absent.

The sigil is part of the name, the compiler checks it against the type, and that is the whole idea. `$count + !delta` tells you, before you know what a count or a delta is, that one of them is live and one of them happens, and so it tells you that the expression is an event, firing when the delta does, with the count read as it stood before. No inference over functors is needed to know that; a lexer could classify it. This serves the goal that you can see what moves. The other goal it serves is less obvious: because the time structure of an expression is readable off the names, the compiler never has to guess it, so the lifting rule in the last section of this chapter is a lookup table rather than a search, and the two implementations cannot disagree about it.

### Stacking

The sigils stack, leftmost outermost, in the order you would say the thing out loud:

- `?$user` is an optional dynamic: maybe there is a live user, and if there is, it changes.
- `$?user` is a dynamic optional: there is always a value here, and sometimes it is "nobody".
- `!!rendered` is an event of events: what a `rebuild` returns, one inner event per build.
- `!$contents` is an event of dynamics: what a `rebuild` returns when its block returns state.
- `$$current` is a dynamic of dynamics: what you hold an `!$contents` into, before `$Join` flattens it.
- `@!click` is a blueprint whose result is an event: the type of a button.
- `@$text` is a blueprint whose result is a dynamic: the type of a text input.
- `$@a` is a dynamic of blueprints: what a `rebuild` consumes.
- `@@a` is a blueprint that returns a blueprint, unbuilt: a widget factory.

Two of these deserve a second look, because the difference between them is the kind of difference the sigils exist to make visible. `?$user` and `$?user` are not two spellings of one thing. The first says that the *existence* of the dynamic is in question and fixed: you match on it once, and either you have a dynamic forever or you never did. The second says the dynamic exists and its *value* is sometimes absent: a logged-in user who may log out. A chat client's current connection is the second; a widget's optional parent callback is the first. Writing one when you meant the other is a type error at the first use, which is where you want to find out.

### Nesting in plain aggregates

A sigil names a kind, not a place where the value is permitted. Functor values sit inside plain values freely:

- `[!Click]` is a plain array of events. The array does not change; its elements each happen.
- `((!a, $b))` is a plain tuple holding an event and a dynamic.
- A record with fields `!changed` and `$value` is a plain record and is the ordinary way a widget returns two things at once.
- `Path ==> Slot`, where a `Slot` holds an `!request`, is a plain map of records each holding an event.

The array and the record are plain in the sense that *they* do not move: their membership and their field set are fixed. That is why `!Leftmost [[!a, !b, !c]]` is an ordinary function from a plain array to an event, and why a widget's result bundle is a record and not some special "result" construct.

So the sigil's *position* says what changes, and the three ways to write a list of items mean three different things:

- `$[Item]` is a list that changes as a whole. When any item changes, there is a new list.
- `[$Item]` is a fixed list of items that each change on their own. The list never grows or shrinks.
- `$(Key ==> $Item)` is a map whose membership changes and whose entries also change on their own. This is what a `for` block sees: `$_` inside it is one entry's `$Item`, and a change to the entry does not rebuild the block.

These are different types with different operations. You cannot append to `[$Item]`, because the list is plain and plain values do not change; you can only build a new plain list, which is a new value, not a change to the old one. You cannot read one item of `$[Item]` as its own dynamic without a lifted index, `$items[3]`, which is `$?Item`. The compiler keeps them apart and the names keep them apart for the reader. In other languages this distinction exists just as much and is written nowhere: a `List<Observable<Item>>` and an `Observable<List<Item>>` are both "the items" to the person reading the call site.

### Where time lives

`!` and `$` are defined against *occurrences*: the discrete, atomic moments at which anything happens. An occurrence has a before and an after and no inside anyone can see. An event fires at an occurrence or does not; a dynamic has one value before an occurrence and one after. A widget's lifetime is measured in occurrences too: built in one, alive through some, torn down in one. The laws are in the chapter on the functors; this chapter needs only that the sigils name those three relations to time, and that plain values and optionals have no relation to time at all, which is what makes them plain.

That last point makes the aggregate rule above work. A plain array of events is plain because arrays have no relation to time: the array was built once and is what it is. The events inside have their own relation to time, each on its own. Nothing about "an array" needs a notion of "now". If it did, if arrays were somehow live, then every container would be a dynamic and every lifted operation would have to say which layer it lifted over. Instead there is one plain layer, which is timeless, and three time functors that sit on top of it or inside it, and lifting only ever looks at the outermost layer of each operand.

### Functions and return sigils

Functions and types are `UpperCamelCase`; values are `snake_case`. A function's name takes the sigil of what it *returns*: `@Button` returns a blueprint, `!Leftmost` returns an event, `$Join` returns a dynamic, `IsClick` returns a plain value. The sigil on a function is a promise about its result, not about its arguments, and it is checked against the annotation.

A function is a plain value when it is passed around. `$(fn Element -> @$State)` is a dynamic whose value is a function; the function itself is plain, it is the dynamic that changes. A function held in a record field, an array or a map is plain. There is no "function that moves": if you want a computation that changes over time, you have a dynamic of a function, and the sigil on the dynamic says so. This is the same rule as for every other value, and it means that `\|e| @Element e` passed as the supplier to `intercept` is a plain lambda, wrapped in `$` by the caller because the parameter wants a dynamic.

A function takes at least one argument. A function of nothing is spelled as taking `unit`, and a widget function of nothing is not a function: it is a blueprint. `@counter` is a value you bind, not something you call. The distinction matters for the reader more than for the checker: `@Button $"OK"` is an application and builds nothing until bound, and `@counter` is a description you bind as it is. Both are plain values until a `<-` builds them.

### What the sigils are not

They are not Hungarian notation. Hungarian notation is a convention a reviewer enforces; a sigil is a token a compiler enforces, and the difference is the difference between `dCount` being wrong in a code review and `count` bound to a dynamic being a compile error with a fix in the message. Suppose the sigils were optional, a style you could turn on. Then a library written without them would be used from a program written with them, and at the boundary a reader would be back to reading types to learn what moves. Suppose instead the sigils were inferred and shown by the editor. Then the program's text, the thing that goes in version control and in a diff, would say nothing, and a reviewer reading the diff would be where the Reflex reviewer was. The sigils are mandatory because the one place they have to be right is the one place that is not an editor: the diff.

They are also not types. `$count` says that `count` is a dynamic; its type, `$Int`, says of what. The sigil is the part of the type that governs lifting and the part a reader needs at every use; the rest is for the checker. A program that annotates every binding with its full type is allowed and not expected.

Consider the alternative, where a function's name said nothing about its result and `Button $"OK"` returned a blueprint by inference. Every call site would then need the type to be read before the line could be understood: does `Label text` draw a label or return the text of one? Because a name says what it returns, a call site can be read without the callee. It costs one character per function, and the character is one the reader was going to want anyway.

## Lexical structure and layout

Two parsers, one in Rust and one in Haskell, read every Prism program, and the conformance suite fails the day they disagree about one. That is the reason this section is written at the level of individual characters: every place where a reasonable lexer could go two ways gets one answer here, with the reasoning, so that the sentence can be changed instead of one of the lexers. The goals in play are the two about reading: you can see what moves, which is why sigils are tokens and not a convention; and the spec is the thing, which is why the offside rule is spelled out below with its ugly cases instead of being described as "like Python".

### Source and comments

Source is UTF-8. A comment runs from `#` to the end of the line and is whitespace for every purpose below, except inside a text literal, where `#` is a character. There are no block comments; a program that wants to comment out a block indents the `#` to taste, since comment-only lines never take part in layout.

Indentation is counted in spaces from column 0. A tab in leading whitespace is an error, with the message naming the line and column; a tab after the first non-space character of a line is ordinary whitespace. The alternative, giving a tab a width, means two editors can show one program with two layouts and a reader cannot tell which the lexer saw; the error costs nothing anyone wants.

### Identifiers and sigils

An identifier is an optional sigil prefix followed by a name. There are two name conventions and they are not interchangeable:

- Lowercase names are `snake_case` and denote values: `count`, `next_id`, `is_done`. The pronoun `_` is a lowercase name.
- Uppercase names are `UpperCamelCase` and denote functions, constructors and types: `Button`, `IsClick`, `TodoItem`.

The first character alone decides which kind a name is, so a name that strays from its convention is still understood. A lowercase name with a capital in it, `isClick`, is a value, and the compiler reports a lint warning suggesting `is_click`; the program compiles and runs. An uppercase name with an underscore in it, `Todo_item`, is an error, because a capital first letter promises a function, constructor or type, and `Todo_item` follows neither convention. The first case is a warning because nothing downstream depends on the spelling: the lexer, the checker and the reader all classify `isClick` correctly from its first letter, and refusing to compile a program over a style the compiler sees through would cost a build for a rename. Digits are allowed after the first character in both forms: `item2`, `Vec2`.

A qualified name is an uppercase name followed by `.` and another uppercase name, with no whitespace anywhere in it, repeated as needed: `Ui.Button`, `Fs.File`, `Net.Socket`. It is one token. The namespace part is how extensible variants (next section) spread their tags across modules. A `.` followed by a lowercase name is never part of a qualified name; it is field access, and `Ui.Button.label` would be field access on the constructor, which is a type error, not a lexing question.

The sigils are `!` `$` `@` `?`. (`&` is reserved for references, later in this chapter, and is rejected with a message saying so; `%` does not exist.) The meaning of a sigil depends on what it is attached to:

- On a value's name, it states the value's kind: `!delta` is an event.
- On a function's name, it states the kind of the function's result: `@Button` returns a blueprint.
- On a type, it is the type constructor: `!Text`, `$[Item]`, `@!Click`; sigils stack leftmost-outermost here as elsewhere.

**Attachment.** The lexer has to decide whether a sigil is part of the next token or a prefix operator standing on its own. The rule: a sigil is *attached*, and part of the token, when the character after it is a name character (letter, digit), `_`, `"`, or another sigil. Otherwise it is *free*: a prefix operator that applies to the whole expression after it, to the end of the enclosing call.

```prism
!delta                       # attached: the name of an event
$"Count: {$count}"           # attached to a text literal: a constant-or-lifted dynamic
!$contents                   # attached twice: an event of dynamics
$_                           # attached to the pronoun
@ unit                       # free: the widget returning unit
@ ChecklistUpdate ix, !checked   # free: lifts the whole constructed record into a widget
$ supply                     # free: the constant dynamic of the function supply
@ ((!a, $b))                 # free, before a bracket: a widget returning a pair
@((!a, $b))                  # same tokens: `((` is not a name character, so the sigil is free
$ Unavailable                # free: the constant dynamic of a unit value
```

The attached and free forms mean the same thing when both are possible: `$"Increment"` and `$ "Increment"` are the same expression. The attachment rule exists for the token stream, not for the semantics, and it is chosen so that the common spellings are the attached ones and the free form reads as an operator with a space after it. A sigil followed by whitespace is always free. A sigil followed by `(`, `[`, `{` or a double bracket is free and applies to the bracketed expression, which is the same thing as if it were attached, so `@((!a, $b))` and `@ ((!a, $b))` have identical token streams: a free sigil then a bracket.

Suppose instead the sigil were always attached to whatever token followed it. Then `@ ChecklistUpdate ix, !checked` would be `(@ChecklistUpdate) ix, !checked`: a call to a function named `@ChecklistUpdate`, which does not exist. The free form lets a sigil act as `return` on a whole expression, and the space says so. The alternative of a separate keyword for lifting, `pure x` or `return x`, would spell the same thing with a word that does not say which functor is meant; `@ x` and `$ x` say it.

### Keywords

`let` `type` `tag` `of` `fn` `match` `if` `else` `for` `rebuild` `intercept` `widget` `record` `choice` `map` `array` `unit` `test` `ignore` `on` `then` `not` `and` `or` `key` `ix` `jx` `kx` `empty`.

Reserved for the `update` blocks later in this chapter and rejected with a message saying so: `read` `update` and the sigil `&`.

Not keywords, though they look like them: `never`, `started`, `main`, `Switch`, `Leftmost`, `Join`, `HoldFirst`, `Element`, `Some`, `Nil`, `True`, `False`, and the optional default `Or`. These are ordinary names in the prelude, written with their sigils where they have one (`!never`, `@started`, `@main`, `!Switch`, `!Leftmost`, `$Join`, `@Element`, `@HoldFirst`). A program may shadow them, which is a bad idea the compiler does not stop. `unit` is a keyword because it is both a type and the only value of that type, and letting it be shadowed would let `@ unit` mean something else.

`key` is a keyword rather than a name because the shorthand `for` binds it without it appearing in the loop's syntax, and `ix`, `jx` and `kx` are keywords because the loops of an `update` block bind them without their appearing in its syntax (the first for the innermost loop, `jx` and `kx` for the next two levels of nesting); a keyword cannot be shadowed by an outer binding by accident. They may be used as record field names (`ChecklistUpdate ix, !checked` is a record whose first field is named `ix`; `Entry(key=k, value=v)`) because a field name is never an expression on its own. Keywords are reserved everywhere; no word is a keyword in one position and a name in another.

### Literals

**Numbers.** A decimal digit string is an integer-spelled literal; a digit string with one `.` and digits on both sides is a decimal-spelled literal. `1_000_000` is allowed, with underscores ignored. `0xFF`, `0o17` and `0b1010` are integer-spelled literals in hexadecimal, octal and binary; `1.5e3` and `2e-3` are decimal-spelled, since an exponent makes a number a `Float` the way a point does. No name starts with a digit, so none of these collides with anything. Numeric literals are polymorphic over the numeric types and take their type from inference, as the next section describes: an integer-spelled literal is a `Nat` or an `Int`, a decimal-spelled one is a `Float`. `-1` is the negation of a literal and is therefore not a `Nat`. A literal is one token; the sign is a prefix operator, told apart from subtraction by the spacing rule below.

**Text.** `"..."` with the escapes `\\` `\"` `\n` `\t` `\{` `\}` and `\u{hex}`. A text literal may not contain a raw newline; a program that wants one writes `\n`. Inside a text literal, `{expression}` interpolates an expression, and `{name}` is the common case: `"hi {name}"`, `"count: {$count}"`, `"{$!line}"`. The braces nest like any brackets: the lexer lexes their contents as ordinary tokens, including nested text literals, until the matching `}`. A `$` in text is always a literal `$`, so `"cost: $5"` and `"$count"` are plain text; a literal brace is written `\{` or `\}`. The kind of the whole literal follows lifting: a literal that interpolates a dynamic is a dynamic, so `"Count: {$count}"` and `$"Count: {$count}"` are one expression, and `$"Increment"` is a constant dynamic because there is nothing to lift over.

The lexer produces a text literal as one token with a list of parts: plain text segments and interpolation token streams. Which parts there are is fixed at lex time, and a `{` that opens an interpolation is the only character in text that the lexer treats as more than itself.

**Char.** `'c'` with the same escapes. A char literal holds exactly one code point.

**Unit.** `unit`. **Empty collection.** `[[]]` or `empty` for an empty array; `{}` for an empty map. `empty` is a keyword so that an empty array can be written without brackets where the brackets would be noise: `Fold xs, empty, F`.

**Bool.** `True` and `False` are constructors of the prelude `choice`, not literals.

### Operators and punctuation

The operator tokens, grouped by what they are for:

- Statement: `<-` bind, `=` definition, `then` hold (a keyword used as an operator: `0 then $count + !delta`).
- Event: `|` merge, `on` sample (a keyword used as an operator).
- Boolean: `not` (prefix), `and`, `or`, keywords used as operators like `on`, over `Bool`. `and` binds tighter than `or`, `not` binds tighter than `and`, and comparison binds tighter than `not`; all of them are tighter than `on` and `|`.
- Comparison: `< <= > >=`, resolved through `Ord`, and `==` `!=`, resolved through `Eq`.
- Arithmetic: `+ - * /`, which resolve by inference, and the suffixed spellings `+~ -~ *~ /~` for `Int` and `+. -. *. /.` for `Float`, each one token, for when inference cannot decide. `/` and `/~` return `?` on the integers and `/.` returns a plain `Float`.
- Conversion: prefix `+?`, one token, converting an `Int` to a `?Nat`.
- Types: `->` function, `==>` map, `:` ascription.
- Structure: `,` `.` `..` `*` (in `Ns.*`), the brackets `( ) [ ] { }`, and the double brackets `((` `))` for tuples and `[[` `]]` for array literals, each pair written with nothing between its two characters.
- Lambda: `\` and the argument bars `|a, b|`.
- Layout: `:` as a block opener, and `;` between the statements of an inline block.

`;` lets an inline block hold more than one statement: `type Pair<a, b> = record: first: a; second: b`, `let s = if c: let t = x + 1; t * t else: 0`. The lexer emits a source `;` as the same statement separator the layout rules emit at a line's start, so the parser sees one kind of separator. A layout block never needs one, and a `;` at the end of a line is an error rather than a style, so that a program has one way to end a statement.

**Double brackets.** Brackets are matched one character at a time, as in any language, and the lexer marks each bracket with whether it touches the bracket before it. Two opens written with no space between them, whose matching closes are also written with no space between them, are one double bracket; the parser, which pairs brackets anyway, reads them so. Pairing goes outermost first.

```prism
((a, b))            # a tuple: the opens touch and so do their closes
((x))               # no comma inside: x itself, so redundant parentheses never change a program
((a + b) * c)       # a group inside a group: the inner ( closes after b, apart from the outer )
( (a, b) )          # spaced: single parens around `a, b`, which is not a call: an error
[[1, 2, 3]]         # an array literal
xs[ys[0]]           # two indexings: the opens are apart, though the closes touch
[[ys[0]]]           # an array literal holding one indexing
(((G b, c), d))     # a tuple; inside it, a single-paren call G(b, c)
```

Adjacency is the whole test, and it is the same test the spacing rule already applies: what touches what. One case costs something: an index whose index expression is itself a pronoun index, `xs[_[0]]`, cannot drop the `_` to `xs[[0]]`, since that is `xs` followed by an array literal. Writing the `_` is the fix, and the case is rare enough that the price is right.

`a < b` is less-than, `<=`, `>` and `>=` follow, and `a == b` and `a != b` are equality; every program compares things, and `if e.cursor == 0:` is what a reader expects to find there. The lexer tells `<`, `<-`, `<=`, `->` and `==>` apart by taking the longest token. Hold is the keyword `then`, so that `$count <= 3` means what it means in every other language and the line that defines a program's state reads as a timeline: `$count <- 0 then $count + !delta` is zero, then the sum whenever the event fires. Suppose instead hold were spelled `<=`, as the arrow suggests. Then `0 <= !e` would be a hold and `n <= !e` a lifted comparison, two different types from one spelling, told apart only by whether the left operand's type matches the event's payload; the one line a reader most needs to read aloud would be the one line that needed a type checker to read.

**Boolean operators.** You have written `!(a && b) || c` and counted parentheses, or written `or(and(a, b), c)` in a language whose booleans are functions and read the line from the inside out. `not`, `and` and `or` are words, as `<` and `on` are, so a condition reads as it is said: `if not done and $count > limit:`. They take `Bool` operands and lift like every other operation: `$a and $b` is a `$Bool`, and `not !flag` is an event. On plain operands, `and` and `or` short-circuit: when the left operand decides the answer, the right one is not evaluated. The reason is overflow, which aborts the occurrence. A guard like `a < 3_000_000 and a * a * a < limit` is only a guard if the cube is never computed for a large `a`; without short-circuit the left test would protect nothing, and the program would need a nested `if` to say what the operator says. Over dynamics and events, `and` and `or` are the same function lifted by the lifting rule below, so `$a and $b` is a dynamic that is recomputed when either changes and an `and` with one event fires when the event does; the short-circuit applies to each evaluation of the plain function inside. There are no prelude functions with these names, and the one prelude `Or` is the optional default (`Or xs[i], 0`), which takes a `?T` and a `T` and has nothing to do with `Bool`.

**The argument bars.** A `|` immediately after `\` opens an argument list, which closes at the next `|`. The lexer distinguishes the bars from the merge operator by position, not by spacing: `\|a, b| a | b` is a lambda of two arguments whose body merges them. Inside the bars only patterns appear, so there is no merge to confuse them with.

**The spacing rule.** Several tokens can be prefix or binary, and the lexer decides which from the whitespace around them: a token that can be either is *prefix* when it is preceded by whitespace (or by an open bracket, a comma, or the start of a line) and followed immediately by a non-space; it is *binary* otherwise.

```prism
F -1          # prefix: F applied to the Int -1
F - 1         # binary: F minus one (a type error unless F is a number)
F-1           # binary
\F .x         # prefix, with _ in scope: F applied to _.x
\F [0]        # prefix: F applied to _[0]
for xs: F .x  # prefix, in a for body: F applied to _.x
a.x           # binary: field x of a
xs[0]         # binary: index into xs
@ x           # free sigil (always prefix; a sigil is never binary)
Size $elems   # the sigil is attached to the name
```

The tokens the rule applies to are `+`, `-`, `.`, `[` and `+?`. A prefix `.` or `[` applies to the pronoun `_` wherever `_` is in scope: a lambda body, a `for` body, any construct that binds it (the lambda section below). Where no `_` is in scope, a prefix `.` or `[` is an error. A `.` that is first on a continuation line is a method-chain step, not a pronoun use; the line break tells the two apart (the paren-less call section). The sigils are not subject to it because they are never binary; `*` and `/` are never prefix. When `a -1` parses as a call and `a` turns out not to be a function, the type error says: "`a` is not a function; did you mean `a - 1`?" That diagnostic exists because the spacing rule is the one rule in the lexer that a newcomer will trip on, and the trip has to be a soft landing.

Why a spacing rule at all, rather than Haskell's approach (from memory), where `-` after an expression is always binary and a negative argument is written `F (-1)`? Because paren-less calls (below) make argument lists the most common thing in a program, and an argument list where a negative number needs parentheses while a positive one does not reads as a trap. The counter writes `-1 on !decrement | +1 on !increment`, and both literals look alike. The cost is that `F -1` and `F - 1` differ by a space, and the diagnostic above is the price paid for that cost. A second alternative, making spacing matter only for sigils and requiring parentheses around negative arguments, would make the counter's third line `(-1) on !decrement | (+1) on !increment`, which is the trap again with extra characters.

### Layout

You have read a widget block before you knew it was one. Here is the counter with braces, the way a C-family language would have it:

```
@counter {
    !inc <- @Button($"Increment");
    !dec <- @Button($"Decrement");
    $count <- 0 then $count + (-1 on !dec | +1 on !inc);
    @Label("Count: {$count}");
}
```

and here it is as Prism writes it:

```prism
@counter:
    !inc <- @Button $"Increment"
    !dec <- @Button $"Decrement"
    $count <- 0 then $count + (-1 on !dec | +1 on !inc)
    @Label "Count: {$count}"
```

Nothing is lost, and what is gone was never information. A widget block is a list of equations, one per line, and the thing that tells you where one equation ends and the next begins is the line; a `;` says it again, and a brace says a third time what the indentation already said. The habit of reading indentation as structure is one every programmer already has, because every program is indented whether or not its language reads the indentation, and the languages that do not read it have to lint for the case where the braces and the indentation disagree, which is the one case a reader is guaranteed to get wrong. A language in which the two cannot disagree has removed a whole class of bug from the reader rather than from the compiler, which is where the bug lived.

The real reason is the one about seeing what moves. The counter's block is four lines of definitions, and a reader is meant to take it in as a table: left column the names, right column what they are. Braces and semicolons put punctuation in the table; a `rebuild` or a `for` with a nested block puts a closing brace on a line by itself, which is a line the reader has to skip. The nested forms are where the difference is biggest: `rebuild $path:` followed by its block reads as "while the path is this, these," and the block is over when the indentation returns, which is exactly when the reader's attention does.

Rust made the same decision from the other side: an `if` body must have braces, with no brace-less form, so that there is one way to write a block and no dangling `else`. One channel, chosen in the other direction. Suppose instead the language took braces and ignored whitespace, as Rust does. The cost would be the separators on every line and the dangling closers, and the benefit would be that a formatter could reflow code and that a line could be split anywhere. Prism gets the second from rule 3 below (an indented continuation line continues the statement) and does not want the first, since a block whose lines are equations has no reason to reflow. The price of the offside rule is that it has to be specified exactly, with its ugly cases, so that two lexers agree; that specification is the rest of this section, and it is short.

Layout is the offside rule, with `:` as the block opener. Everything below is the contract. The intuition is Python's: a colon at the end of a line opens a block, the block is the lines indented under it, and the block ends when the indentation comes back. The difference from Python is in what a line *inside* a block means when it is indented more than its neighbours: it continues the statement above it, as in Haskell, rather than being an error.

1. The lexer keeps a stack of block columns, initially `[0]`. The top of the stack is the current block's column.

2. A `:` that is the last token on its line (comments ignored) opens a *layout block*. The next line that is not blank and not comment-only must be indented strictly more than the current block's column; its column is pushed and becomes the new block's column. Anything else, including end of file, is an error: "expected an indented block after `:`".

3. Within a block, a line at exactly the block's column begins a new statement; the lexer emits a statement separator before it. A line indented more than the block's column *continues* the current statement. A line indented less closes the block: the lexer emits a block close and pops, and the comparison repeats against the new top. After popping, the line's column must equal the top of the stack; an unindent that matches no open block is an error: "unindent does not match any open block".

4. A `:` with more tokens after it on the same line is an *inline block*: the rest of the line, up to an `else` keyword or the end of the line, is the block, and the block closes there. It holds one statement, or several separated by `;`. `True: 3` in a `match`, `"foo": 3` in a `map:` block, `if some_bool: 3` and `x: Float` in a `record:` block are all this form. The lexer emits the inline `:` as an ordinary colon token; the parser treats what follows as a one-statement block. If the line ends with another `:`, that one opens a layout block by rule 2, and the layout block belongs to the statement inside the inline block.

5. Inside `(`, `[`, `{`, the double brackets, and the `{...}` of a text interpolation, layout is suspended: line breaks and indentation are whitespace until the matching close bracket (the lexer still marks a `.` that begins a line, for method chains), and `:` inside brackets never opens a block of either kind; it is the key separator of a one-line map or a type ascription. A bracket opened on one line and closed on a later one leaves the stack untouched.

6. A block opened in the middle of a statement (`!events <- for $_:`, `$x <- match y:`) belongs to that statement; when the block closes, the statement ends. A following line indented more than the enclosing block's column but less than the closed block's is an error by rule 3; there is no continuation of a statement after its block.

7. `else` at the column of its `if` begins a statement that the parser attaches to the preceding `if` statement. An `else` after an inline `if` on the same line ends the inline block, by rule 4.

8. Top level is a block at column 0. A top-level definition is a statement of that block. A top-level line indented more than column 0 that is not a continuation is an error by rule 3, since nothing is open to match it.

9. Blank and comment-only lines never open, close or continue anything. A statement may contain blank lines between its continuation lines.

10. End of file closes every open block.

There is no explicit-brace form of a layout block. Programs that need one line use the inline form or brackets. The alternative, Haskell's `{ ; }` fallback (from memory), exists there so that generated code and one-liners can avoid layout; here generated code writes newlines, and one-liners have the inline form, and the saving is that no parser has to merge two block grammars.

**The ugly cases, each with its answer.**

*A statement continued by a more-indented line.* The continuation is any line indented more than the block column; it does not have to align with anything.

```prism
@form:
    $edit <- Edit initial, (Len initial)
        then Apply $edit, !key
    @Label (Render $edit)
```

The second line is indented more than the block's column (4), so it continues the hold statement. The third line is at column 4, so it begins a new statement.

*Two blocks closed by one dedent.*

```prism
@outer:
    !x <- rebuild $a:
        !y <- @Button $"B"
        @ !y
    @Label $"done"
```

After `@ !y` the next line is at column 4. The stack is `[0, 4, 8]`; 4 is less than 8, so the `rebuild` block closes and the stack is `[0, 4]`; 4 equals the top, so the line begins a statement of the outer block. Had the line been at column 0, two closes would have been emitted before the comparison matched. Had it been at column 6, the error in rule 3 fires: 6 is less than 8 and does not equal 4.

*An inline arm followed by a layout arm in one match.*

```prism
match shape:
    Circle r: Pi *. r *. r
    Square s:
        let side = s
        side *. side
```

Both arms are statements of the match block at column 4. The first is an inline block; the second opens a layout block at column 8 that belongs to the arm statement. Mixing is allowed because each arm decides for itself, and the lexer sees nothing special about a match.

*`:` inside brackets.*

```prism
let m = {"a": 1, "b": 2}
let n = Insert m, "c", (if c: 3 else: 4)
let t = Pair (x: Nat), y
```

None of these colons opens a block. The first two are map key separators; the `if` inside parentheses uses an inline block whose end is the `else`, not the line end, and the second inline block ends at the close paren, since brackets close everything inside them. The third is a type ascription on an expression.

*A trailing comment after the opening colon.*

```prism
@counter:    # the counter from chapter 2
    !increment <- @Button $"Increment"
```

Comments are whitespace, so the `:` is still the last token on its line and opens a layout block.

*A block that is the whole of a statement.* `@counter:` at top level, `type Vector = record:`, `test counter, @counter:`. In the last one the block holds one statement per turn, and each turn's `=>` lines are continuations of it by rule 3.

*A bracket spanning lines inside a block.*

```prism
@main:
    let names = [[
        "a",
        "b"
    ]]
    @Label $"ok"
```

Inside `[[` the lines are whitespace, so the indentation of `"a"` and of `]]` is irrelevant and the statement is one statement. The next line at column 4 begins a new statement. A close bracket at column 0 inside a block is still fine; the bracket's own position is never compared to the stack.

*A `for` with a block followed by a merge.* This one is an error, on purpose:

```prism
@bad:
    !x <- for $m:
        @ !click
      | !other               # error: unindent does not match any open block
```

Rule 6: the statement ended when the block closed, and column 6 matches nothing. The program wanted `!x <- (for $m: ...) | !other`, which cannot be written with layout, and is written by binding the `for` to a name on its own line and merging on the next. This refusal is deliberate: a continuation after a dedent would mean the reader has to find the end of a block by looking past it.

*Nested inline forms.*

```prism
if c: match x:
    Some v: v
    Nil: 0
else: 1
```

`if c:` opens an inline block; its statement is `match x:` whose line-final `:` opens a layout block belonging to the match; the arms are in it; the dedent to column 0 closes it and ends the match statement, which ends the inline block; `else` at the column of `if` attaches. Legal, defined, and not recommended.

*The lambda bars across a line.* `\|a, b|` must open and close on one line. A lambda's body may continue onto later lines by the continuation rule, or be a layout block if the body is `@widget:` or `match a:` or anything else ending in `:`.

*A text literal with an interpolation containing a colon.* `"{if c: "yes" else: "no"}"` is one token; the interpolation is lexed as bracket contents, where the inline `if` ends at `else` and the second inline block ends at the `}`. The nested text literals inside are complete literals of their own.

### Paren-less calls

You have read `render(layout(items, width), theme)` and counted parentheses to learn which call `theme` belongs to, and you have read `f x y + 1` in Haskell and stopped to remember that application binds tighter than `+`. Prism's call syntax is built so that neither question comes up: which call a comma belongs to is decided by the nearest bracket around it, and where an argument ends is decided by the next comma, never by a function's arity or a precedence you have to recall.

A function is applied by writing its name and then its arguments, separated by commas: `Insert items, id, todo`. The rules, all of them:

- **A paren-less call takes the rest of its expression.** Its arguments are whole expressions separated by commas, and the last one runs to the end of the expression the call sits in. A paren-less call inside an argument takes no commas, so commas belong to the outermost call: `F a, G b, c` is `F(a, G(b), c)`, and `F a + 1, b` is `F(a + 1, b)`.
- **A lambda takes the rest of its expression too, so the call heading its body takes the commas after it.** `Fold changes, items, \|acc, id, c| Step acc, id, c` gives `Step` three arguments and `Fold` three; a lambda that is not the last argument takes parentheses.
- **Single parentheses group, and a group whose head is a call is that call with all its commas.** `F a, (G b, c)` gives `G` two arguments; `(Insert $items, $next, todo) on !add` is one three-argument call, sampled at `!add`.
- **Double parentheses make a tuple.** `((a, b))` is a pair, and the commas inside belong to the tuple: `((G b, c))` is the pair of `G b` and `c`. A multi-argument call inside a tuple takes its own single parentheses: `(((G b, c), d))`.
- **Double brackets make an array literal.** `[[F a, G b]]` is a two-element array. Single brackets are always indexing: `xs[i]`.
- **Braces make a map**, and the commas inside separate entries: `{k: F a, j: b}`.

```prism
Insert items, id, (Todo label, False)            # Insert gets three arguments; Todo gets two
intercept @child, !down, $ supply                # three arguments; the free $ takes supply
intercept @_, (!Union !kid_ins, !fwd_ins)        # !Union gets two arguments
Concat [[(Prefix e.text, e.cursor), ToText c]]   # two array elements; the first is a two-argument call
((Len xs, ys))                                   # a pair: Len xs and ys
(Len xs, ys)                                     # a call: Len with two arguments
Describe $_, $starred                            # two arguments, lifted over the dynamics
@Label (Render $edit)                            # one argument, parenthesised for clarity, not necessity
Fold changes, items, \|acc, id, c| Step acc, id, c   # the trailing lambda's body takes the rest
```

A reader therefore never needs a function's arity to parse a line, and the parser never needs it either: every comma has a bracket or an outermost call to belong to, and which one is visible in the characters.

**The case to watch.** Because a paren-less call takes the rest of its expression, `Size $elems + 1` is `Size($elems + 1)`, and the program that wanted one more than the size writes `(Size $elems) + 1`. This is the one place the rule surprises, and it is kept on purpose. Arguments are whole expressions, so a reader looking at a call never weighs operator precedence against application; the only question is where the next comma is. Suppose instead application bound tighter than every operator, as in Haskell (from memory). Then `Size $elems + 1` would mean `(Size $elems) + 1`, the usual intent in arithmetic, and every argument that is itself an expression would need parentheses: `@Label ($"Count: {$count}")`, `Len ($name)`, `@Button ($"OK")` on the first line anyone writes, and `F (a + 1), b` where the rule above writes `F a + 1, b`. This language's programs are mostly calls with structured arguments and rarely arithmetic over a call's result, and the rule follows the programs. When arithmetic over a result is wanted, `let` it or parenthesise it; a lint flags a paren-less call on the left of an arithmetic operator, which catches the case cheaply.

**Why the doubled brackets.** Tuples and array literals get the doubled brackets so that the single ones mean one thing each: single parentheses group (and a group headed by a call is the call), single brackets index. Suppose tuples were written with single parentheses, as in Python and Rust. Then `(Len xs, ys)` is either the pair of `Len xs` and `ys` or `Len` called with two arguments, and only `Len`'s arity says which; a parser that wants to stay out of the type checker has to pick one meaning and give the other a second spelling anyway. Suppose arrays were written with single brackets. Then `xs[i]` and `xs [i]` differ by one space, between "index `xs`" and "call `xs` with a one-element array", and inside a lambda body `\F [0]` is either `F` applied to `[0]` or `F` applied to the pronoun's first element. With the doubled forms, the parse of every bracket is readable from the bracket alone, and the two literal forms follow one rule, so there is one thing to learn rather than two. The cost is two characters per literal. In these programs tuples are mostly bundle patterns and short pairs, and most collections are maps built by functions rather than written out, so the cost lands on the rarer forms and the common ones, calls, grouping and indexing, stay single.

**Method chains.** The method form `x.F y, z` is `F x, y, z`: the receiver becomes the first argument. There is no distinction between functions and methods; the form exists because `slots.Filter IsGui` reads better than `Filter slots, IsGui` when steps follow one another. Chains associate left, so `x.F.G` is `G (F x)`, and the receiver of a `.F` written against an expression is that expression up to the `.`: `a.b.F c` is `F (a.b), c`.

A step that takes arguments starts its own line. A `.Method` that begins a continuation line closes the call above it and takes it as its receiver:

```prism
let done = items.Filter IsDone
    .MapValues Label            # MapValues (Filter items, IsDone), Label
    .Values                     # Values (MapValues ..., Label)
```

The receiver runs back to the start of the statement's right-hand side, after its `=`, `<-` or `then`; inside brackets, where lines are whitespace, it runs back to the open bracket or the nearest comma at that bracket's depth. On one line, `x.F y` is method form and a spaced `.F` is not a chain step: `items.Filter IsDone .MapValues Label` is an error whose message says to put the step on its own line. Indentation decides because the alternative rules each give the parse to something a reader cannot see. Suppose a spaced `.F` on one line closed the call before it: then `F a .G b` and `F a.G b` would be different programs, distinguished by a space in the middle of a line, and the paren-less call would end somewhere other than at a comma or the end of the expression. A line break is the one boundary that is obvious from across the room, and one step per line is how a chain is written in every language that has them.

### Parse corpus

The programs on which the two parsers must produce identical trees, sketched the way the layout corpus sketches token streams. The sketch writes every application with explicit parentheses, `F(a, b)`; a tuple as `tuple(a, b)`; an array literal as `array(a, b)`; indexing as `index(xs, i)`; field access as `field(x, f)`; a lambda as `λ_. body`; and keeps operators infix with explicit grouping. Both implementations keep it as a golden test beside the token streams.

**1. Nested calls, with and without parentheses.**

```prism
let a = F x, G y, z
let b = F x, (G y, z)
let c = F G H x
```

```
a = F(x, G(y), z)
b = F(x, G(y, z))
c = F(G(H(x)))
```

`G y` in the first line takes no commas because it is not the outermost call. A chain of one-argument calls needs no parentheses at all.

**2. Tuple against call.**

```prism
let t = ((G y, z))
let u = (G y, z)
let v = (((G y, z), w))
```

```
t = tuple(G(y), z)
u = G(y, z)
v = tuple(G(y, z), w)
```

**3. Array literal against index.**

```prism
let xs = [[1, 2, 3]]
let x = xs[0]
let y = grid[r][c]
let z = [[xs[0], ys[i]]]
```

```
xs = array(1, 2, 3)
x = index(xs, 0)
y = index(index(grid, r), c)
z = array(index(xs, 0), index(ys, i))
```

**4. A method chain on continuation lines.**

```prism
let names = todos.Filter IsDone
    .MapValues Label
    .Values
```

```
names = Values(MapValues(Filter(todos, IsDone), Label))
```

**5. The case to watch.**

```prism
let a = Size $elems + 1
let b = (Size $elems) + 1
let c = 1 + Size $elems
```

```
a = Size($elems + 1)
b = (Size($elems)) + 1
c = 1 + Size($elems)
```

Nothing precedes a call's name that the call could absorb, so `1 + Size $elems` adds one to the size.

**6. A higher-order function type.**

```prism
let Map<a, b>: fn (fn a -> b), [a] -> [b]
let Curry<a, b, c>: fn (fn ((a, b)) -> c) -> (fn a -> (fn b -> c))
```

```
Map : fn(fn(a) -> b, [a]) -> [b]
Curry : fn(fn(tuple(a, b)) -> c) -> fn(a) -> fn(b) -> c
```

The parentheses around a function-typed argument or result are required, so the commas of an inner `fn` never join the outer one's argument list.

**7. A trailing lambda with the pronoun shorthand.**

```prism
let open = Keep $todos, \not .done
let firsts = for rows: Map _, \[0]
```

```
open = Keep($todos, λ_. not(field(_, done)))
firsts = for(rows, Map(_, λ_. index(_, 0)))
```

The lambda is the last argument and its body takes the rest of the line. Inside the `\` body a prefix `.done` is `_.done` and a prefix `[0]` is `_[0]`; the `_` of the `for` body is the argument `Map` receives, and the lambda's own `_` shadows it inside the lambda.

**8. A lambda body that is a call, and a bare projection.**

```prism
let total = Fold changes, 0, \|acc, c| Add acc, c.amount
let ids = Map todos, \.id
```

```
total = Fold(changes, 0, λ(acc, c). Add(acc, field(c, amount)))
ids = Map(todos, λ_. field(_, id))
```

`Add` takes both commas because the lambda body takes the rest of the expression; `Fold` still has three arguments.

**9. Redundant parentheses, a free sigil on a call, and an inline block.**

```prism
let a = ((x))
let b = @ F x, y
let c = F x + 1, y
type Pair<a, b> = record: first: a; second: b
```

```
a = x
b = @(F(x, y))
c = F(x + 1, y)
Pair<a, b> = record(first: a, second: b)
```

`((x))` has no comma and is `x`. The free `@` wraps the whole call, and the `;` separates the two statements of the inline `record` block.

**10. A spaced `.Method` on one line is an error.**

```prism
let bad = items.Filter IsDone .MapValues Label
```

```
ERROR(a .Method step must start its own line)
```

The program writes the step on its own line, as in case 4.

**11. Boolean operators against comparison, `on` and arithmetic.**

```prism
let a = x or y and z
let b = not done and $n > limit
let c = $done or $n > limit on !click
let d = not a == b
let e = not $n + 1 > limit
let f = not done and ready
```

```
a = x or (y and z)
b = (not done) and ($n > limit)
c = ($done or ($n > limit)) on !click
d = not (a == b)
e = not (($n + 1) > limit)
f = (not done) and ready
```

`and` binds tighter than `or`, both bind looser than comparison and tighter than `on`, and `not` sits between `and` and comparison, so its operand is a whole comparison: cases `d` and `e` negate the comparison, and case `f` negates `done` alone because `and` is looser than `not`.

**12. Guards and array patterns.**

```prism
let a = match items[id]:
    Some t if t.done: 1
    Some t: 2
    Nil: 3
let b = match xs:
    [[]]: 0
    [[head, ..rest]]: head
```

```
a = match(index(items, id);
      arm(Some(t) if field(t, done) => 1),
      arm(Some(t) => 2),
      arm(Nil => 3))
b = match(xs;
      arm(array() => 0),
      arm(array(head, ..rest) => head))
```

The guard is part of the arm and runs from `if` to the colon. `a` is exhaustive by its second and third arms alone; `b` is exhaustive by its two arms, the empty array and the non-empty one.

**13. `for` with named binders.**

```prism
let b = for k, item <- xs: F k, item
let c = for xs: F key, _
```

```
b = for(k, item <- xs, F(k, item))
c = for(key, _ <- xs, F(key, _))
```

`c` is the shorthand and the same tree as `for key, _ <- xs:`, so the two spellings are one program.

### What a conforming lexer must get right

The layout corpus at the end of this chapter lists the programs on which both implementations must produce identical token streams. The shortest statement of the contract is this: every decision in this section is made by the lexer or by the parser's treatment of colon tokens, `else`, `;` and bracket adjacency, never by the type checker; the token stream for a program is a pure function of its characters; and the only context the lexer keeps is the column stack, the bracket stack, and whether the previous character was whitespace. Two marks on tokens come from that context: each bracket says whether it touches the bracket before it, which is all the parser needs to pair double brackets, and a `.` that is the first token on its line is marked as a chain step, since the line break that makes it one is otherwise gone from the stream.

## Values and types

The value layer is meant to be boring, and that is a decision. Everything interesting about Prism is in the functors; the plain values under them are the ones any typed functional language has, so that nothing here competes with time for your attention. Two things in this section are not boring, and both are goals. Absence is in the type, `?` and the cases of every element's state, because you have shipped the `if (user)` guard that was missing. And there is no way to mutate a plain value, anywhere, because the only state is a hold and that is the promise the whole language rests on.

A plain value is immutable, eager, and cheap to copy or alias; the program cannot tell copying from aliasing and the implementation is free to do either. Functions are the one lazy thing. Every type in the language is either a plain value or a plain value under one or more of the functors.

### Primitives

- `Nat`: unsigned, 64-bit. Indexes arrays. Has no negation.
- `Int`: signed, 64-bit.
- `Float`: IEEE double.
- `Text`: Unicode text. Indexed by character position through prelude functions, never by byte.
- `Char`: one code point.
- `unit`: the type with one value, also spelled `unit`.
- `Bool`: not a primitive. `type Bool = choice True | False` in the prelude, and `if` is `match` on it.

Overflow in `Nat` or `Int` arithmetic is a runtime error that aborts the occurrence with the error kind `Overflow`; the error chapter says what that means, and the prelude's `Checked.*` functions (which return `?`) and `Wrap.*` functions (which wrap around) are the spellings for a program that wants a different answer. Division on `Nat` and `Int` returns an optional: `a / b` is `?Nat` for `Nat` operands and `?Int` for `Int` operands, `Nil` when `b` is zero. Division on `Float` follows IEEE and never fails: `1.0 / 0.0` is infinity, `0.0 / 0.0` is NaN, and `a / b` is a `Float`. The two rules differ because the two failures differ. An integer has no value to answer with when the divisor is zero, and an integer division that panics is the one that bites in a running program, so the `?` marks the one place a divide-by-zero can be and the type makes the program say what it does then. A `Float` has infinities and NaN to answer with, and they are the answers every numeric library and every GPU already gives; wrapping each `Float` division in a `?` would make a drawing program match on `Some` at every scale factor to guard against a case the representation already covers.

### Numbers by inference, and the three traits

Three things are overloaded in Prism and nothing else is: equality, ordering, and arithmetic. They are traits, in the Rust sense: a named set of operations that a type implements, with the operation resolved by inference from the operand types. The three are `Eq`, `Ord` and `Num`.

- `Eq`: every plain value whose type contains no function and no functor value. Structural: two records are equal when their fields are, two maps when their entries are, two values of an extensible variant when their tags and plain fields are (an element's `requests` event is excluded).
- `Ord`: primitives (`Float` included, by the total order described below), text, chars, tuples and arrays of ordered types (lexicographic), a `choice` whose payloads are ordered (declaration order of the alternatives, then payload; a closed `choice` of bare alternatives is ordered by declaration order alone, so `type Size = choice Small | Medium | Large` has `Small < Large`), a record whose fields are all ordered (lexicographic in field declaration order, like Rust's derived `Ord`), and an extensible variant whose plain fields are ordered (tag name, then plain fields in declaration order, `requests` excluded). Map keys must be `Ord`, so a record or an element can key a sorted map.
- `Num`: `Nat`, `Int` and `Float`, with `+ - * /`. Prefix `-` is `Int` and `Float` only. `/` is `?Nat` and `?Int` on the integers and plain `Float` on `Float`.

Values that hold a function, an event, a dynamic or a blueprint have none of these. Equality on events would have to mean something, and the only thing it could mean is identity, which the language does not give events; equality on functions is undecidable; equality on blueprints would be equality on descriptions, which is close to structural but would make two different closures compare unequal in a way the program cannot predict. Ruling them out keeps `Eq` meaning "the same value" and nothing subtler.

Arithmetic is written `+ - * /` and the type of each operation is found by inference from its operands:

```prism
let !delta = -1 on !decrement | +1 on !increment     # -1 is signed, so !delta : !Int
$count <- 0 then $count + !delta                        # Int addition; the 0 is an Int because the hold is
let area = w *. h                                     # Float, said directly
let third = total / 3                                 # ?Nat, if total is a Nat
let half = width / 2.0                                # Float, if width is a Float; never Nil
```

Numeric literals are polymorphic over `Nat`, `Int` and `Float` and take their type the same way. An integer-spelled literal is a `Nat` or an `Int`; a decimal-spelled literal is a `Float`; the negation of a literal is not a `Nat`. So `-1` is an `Int` by its spelling alone, `1.5` is a `Float`, and `0` is whatever the context says. When nothing constrains an integer-spelled literal, it is an `Int`: `$n <- 0 then $n + 1 on !tick` is an `Int` counter with no annotation anywhere. The suffixed spellings `+~ -~ *~ /~` (`Int`) and `+. -. *. /.` (`Float`) pin the type at the operator when a program wants to say it there: `0 then $count +~ !delta` says `Int` outright. `Nat` has no suffixed spelling; a program that needs to pin a `Nat` annotates it, `let n: Nat = 0`. Prefix `+` is the identity on any numeric type and exists so that `+1` and `-1` can sit next to each other in a merge. `+?` converts an `Int` to a `?Nat`, and `Abs` to a `Nat`.

Why inference rather than one operator per type, as OCaml does (from memory)? A program full of `+~` reads as a program about its own numeric representation, and the counter, the first program anyone writes, would have one on its most important line. Why `Int` when nothing decides? An integer literal that nothing pins is almost always a count, an offset or a step, and `Int` is the type under which all three behave: it goes below zero without an overflow abort, and it never turns `n / 2` into a fraction the way a `Float` default would. Suppose instead an unpinned literal were an error. Then the counter without its `!delta` line, `$n <- 0 then $n + 1 on !tick`, would be rejected, and the first program anyone writes would end in an annotation it does not need. The suffixed forms exist for the program that wants the type stated at the operator rather than in an annotation three lines away.

Equality and ordering in use look like this:

```prism
if e.cursor == 0: e else: Edit e.text, e.cursor - 1
let sorted = Sort names                    # Text is Ord
let by_id = Insert items, id, todo         # Nat is Ord, so Nat ==> Todo is a map
let same = slot_a.element == slot_b.element    # structural, on an extensible variant
let later = a.at > b.at                   # Ord, through the > operator
```

`Eq` on `Float` is IEEE equality, so a NaN is not equal to itself and `-0.0 == 0.0`. `Ord` on `Float` is the IEEE totalOrder: negative NaN, then negative infinity, the negative numbers, `-0.0`, `0.0`, the positive numbers, positive infinity, positive NaN. `< <= > >=`, `Sort` and map keys all use it, so a sort of floats terminates, a map with `Float` keys has a defined order and can hold a NaN key, and `-0.0 < 0.0` is `True` while `-0.0 == 0.0` is also `True`. The two relations disagree only on zeros and NaN, and a total order that agreed with `==` there would have to call a NaN equal to itself; a program that wants something else wraps the float in a newtype and compares through a function. Equality on `Text` is by code points, not by any normalisation; two texts that look the same and are encoded differently are different, and a program that cares normalises first. These are the dull answers, chosen because the alternatives are a choice of locale or a choice of normalisation form, and either one is a dependency on something outside the program.

Generic code may be bounded by a trait, `let Max<a: Ord>: fn a, a -> a`, and is still erased: the trait implementation is passed where the bound is, not looked up at runtime. It is not a general trait system. There are no user-defined traits, no associated types, no instances declared away from the type. The checker is written so that those can be added, and the chapter on the compiler says how; for this specification the three are the whole of it, and they are enough for sorting, map keys, element identity, and arithmetic that reads as arithmetic.

### Aggregates

- **Arrays** `[T]`: homogeneous, written `[[a, b, c]]`, `[[]]` or `empty`, or as an `array:` block with one element per statement. Indexed by `Nat` with `a[i]`, which returns `?T`. `Len`, `Append`, `Concat` and friends are prelude functions.
- **Maps** `K ==> V`: sorted by key, and the key type must be `Ord`. Written `{k: v, ...}` on one line, `{}` when empty, or as a `map:` block with one `k: v` per statement. `m[k]` returns `?V`. Iteration order is key order, always.
- **Tuples** `((A, B))`: heterogeneous, comma-separated, in double parentheses as values, types and patterns alike. A one-tuple is `((a,))`. The empty tuple is `unit`. Arrays are written in double brackets as values and as patterns alike (the `match` section gives the array patterns).

```prism
let some_map = map:
    "foo": 3
    "bar": 4
let other_map = {"foo": 3, "bar": 4}
let xs = array:
    First pair
    Second pair
```

This language prefers maps wherever identity matters. A map's key is an identity: in a `for`, it decides which block survives a change, which element path stays stable, and which hold keeps its state. An array has only positions, and a position is not an identity: insert at the front and every later item is "a different item" to anything keyed on position. The todo list in the worked examples allocates ids and keeps `Nat ==> Todo` for exactly this reason. Arrays are for positional data: the characters of a line, the rows of a matrix, the arguments of a program.

Maps are sorted rather than hashed because a sorted map has a defined iteration order and a hashed one has an implementation-defined one, and the element list a World reads is "in tree order" only because the paths that key it are ordered. A program whose output depended on hash order would be a program whose transcript changed with the runtime, which is what the determinism goal forbids.

Indexing returns `?T` rather than failing. You have written the bounds check, and you have forgotten it once. With `?` on the index the forgetting is a type error, and the cost, a `match` or an `Or` default at each index, is the cost of saying what happens when the index is bad, which the program had to decide anyway.

### Declared types

```prism
type Vector = record:
    x: Float
    y: Float

type Bool = choice True | False

type Shape = choice:
    Circle Float
    Square Float

type Meters = Meters Float

type Click = click

type TextInput = record:
    !changed: Text
    $value: Text
```

**Records.** A `record` names its fields and their types. The type name is also its constructor, callable three ways: positionally, `Vector 1.0, 2.0`; with named arguments in parentheses, `Vector(x=1.0, y=2.0)`, in any order, all of them given; or as a block of `field: value` statements:

```prism
let origin = Vector:
    x: 0.0
    y: 0.0
```

Fields are read with `.name`. A field holding a functor value is declared and read with its sigil: `.!changed`, `.$value`. The sigil is on the field name rather than only on the type so that `input.!changed` reads as an event at the point of use, the same as any other name; a field `changed: !Text` would be a name without a sigil denoting an event, which is the one thing the naming rule forbids.

Records are the one place a widget returns several things at once, and the `TextInput` above is the pattern: a plain record with sigiled fields is a bundle, and a widget `@TextInput: fn Text -> @TextInput` returns one. The parent binds it with a record pattern, `TextInput(!changed, $value) <- @TextInput ""`, or keeps the record and reads the fields.

**Choices.** A `choice` lists alternatives, inline with `|` or one per statement in a block. An alternative with a payload declares a newtype on the spot: after `type Shape = choice: Circle Float ...`, `Circle` is both a type and a one-argument constructor, and `Circle 2.0` is a `Shape`. An alternative without a payload is a unit type (below). Two choices may not declare the same alternative name in one scope; `Circle` means one thing.

**Newtypes.** `type Meters = Meters Float` declares a type and a one-argument constructor of the same name, unwrapped with `.value`. `Meters 3.0` is not a `Float` and `(Meters 3.0) +. 1.0` is a type error, which is what a newtype is for. Newtypes over tuples make lightweight records, `type Pair = Pair ((Nat, Nat))`, but records are preferred because fields have names.

**Unit types.** `type Click = click` declares a type with exactly one value, spelled in lowercase. A unit type is the type of a response that has no data: a click, a tick, an end-of-input. The lowercase value is a plain name and follows the value naming rule; the uppercase type follows the type rule. A `choice` alternative with no payload, `Stopped` in `choice: Stopped | Running Instant`, is the same construction, with the uppercase name serving as both the alternative and its value, because inside a choice the alternative name is the thing you match on.

**Generics.** Generic types and functions take parameters in angle brackets: `type Pair<a, b> = record: first: a; second: b`, `let Swap<a, b>: fn Pair<a, b> -> Pair<b, a>`. A lowercase name in a type is a type variable. In a `let` annotation, free type variables are quantified implicitly, `!Filter: fn !?a -> !a`, and the angle-bracket list is needed only to state a bound: `let Max<a: Ord>: fn a, a -> a`.

Generic code is erased. It runs on values whose types it does not know and never needs to, because nothing in the language inspects a value's type at runtime: there is no `typeof`, no downcast, no runtime type descriptor and no monomorphisation. A generic `Swap` is one function, run on every `Pair`. The one place where erasure needs a rule of its own is extensible variants, below, where the rule is that a tag's fields have concrete types so that a value built in generic code never needs a type it does not have.

### Optionals

`?T` is `Nil` or `Some x`. It is a plain-value type with a sigil, not a time functor: a `?Nat` has no relation to occurrences. It is a sigil rather than a type constructor spelled `Option` for the same reason the others are: so that a name says what it is. `?user` reads as "maybe a user" at the point of use, and a function that returns one says so in its name.

`?` lifts, at the plain layer. A plain operation applied to a `?T` where it wants a `T` gives a `?` result, `Nil` if any operand was `Nil` and `Some` of the operation otherwise, which is the applicative `Maybe` you know. So `a / b + 1` is `?Nat` when `a` and `b` are `Nat`, `Len ?name` is `?Nat`, and `?x == 3` is `?Bool`. `match` is the eliminator: the only way to get from `?T` back to `T` is a `match` with a `Nil` arm, or a prelude function that is one, `Or ?x, default`.

The worry about lifting `?` is the integer division case: `a / b + 1` quietly becomes optional, and the reader of the line might not notice. Look at where the result goes. It is bound to a name, and that name must be `?total`, because the sigil is part of the name and checked against the type; it is passed to a function wanting a `Nat`, and the function gets a `?Nat` instead, so that application lifts too and its result is optional, and so on until some binding is forced to show the `?` or some `match` eliminates it. The `?` cannot be lost; it can only move outward, and it stops at the first name. The alternative, no lifting, so that `a / b + 1` is a type error and every optional needs a `match` at the site of its first use, puts the `match` where the division is rather than where the decision about absence is made, and in the todo list that is the difference between one `match` in `Describe` and five in the arithmetic above it. Lifting `?` is the same decision as lifting `!` and `$`: the structure is in the names, so the operations can be plain.

Lifting over `?` happens at the plain layer, which means *inside* the time functors. `$a / $b` is `$?Nat`: a dynamic whose value is sometimes `Nil`. `?$x + 1` does not lift and is a type error: an optional dynamic is a value you match on, once, to find out whether there is a dynamic, and only then can you lift over the dynamic. The sigil order says which it is, and the lifting rule in the next section states the precedence between `?` and the time functors in one line.

### Extensible variants

```prism
type Element = choice ..

tag Ui.Button of Element:
    label: Text
    responds: Click

tag Fs.Contents of Element:
    path: Text
    state: Contents
```

An extensible variant is a `choice` whose alternatives are declared across modules rather than in one place: `type Element = choice ..` opens it, and each `tag` declaration adds an alternative. A tag's name is qualified, `Ui.Button`, and the namespace is the module's way of saying whose tag it is. A tag declaration states its fields with their types, and, for the one extensible variant the language relies on, up to three parts that say what the element does: `requests`, `responds` and `state`. The parts are specified in the elements chapter, where they mean something; here the variant is a type like any other.

The rules:

- **Field types are concrete.** A tag declaration may not mention a type variable. The reason is erasure: generic code never knows the type of a value it holds, and a tag whose field could be "whatever `a` is" would need a runtime type descriptor to match on. With concrete fields, every value of an extensible variant has fully known field types where it is built, and generic code cannot build one except at a type it knows. The cost: a generic library cannot wrap its own type parameter in an element. It takes elements already built, or is written at a concrete type. A generic function that tries, `let Wrap: fn a -> Element = \Test.Box(value=_)`, is rejected with a message naming the field and the type variable.
- **Construction** is the record forms: `Ui.Button(label="OK")` with named arguments, `Ui.Button "OK"` positionally, or a block. A tag with no fields is bare: `Clock`.
- **Matching** is by tag, with the fields bound by name or position as for a record: `Ui.Button(label=l)` binds `l : Text`. There is no matching on field *types*, because every field's type is fixed by its tag; what you match on is which tag, and the fields follow.
- **`Ns.*`** in a pattern matches any tag declared under that namespace: `Ui.*(..)` is every element of the GUI library. The `*` is the pattern's and appears nowhere else; a namespace alone, `Ui`, is not a pattern, so that adding a tag `Ui.Window` to a library cannot change which arm an existing `Ui` match takes. Nested namespaces match by prefix: `Ui.*` includes `Ui.Text.Input` if someone declares one.
- **The catch-all rule.** A `match` on an extensible variant must end in a catch-all arm, `ignore: ...` or a bare name, because the set of alternatives is open and no match can be exhaustive by listing. In an interceptor the catch-all is the forwarding case, and the rule makes forgetting to forward a compile error rather than a silently dropped element.
- **Equality** is structural, tag then plain fields, with the `requests` event excluded, and is `Eq` when every plain field is. **Ordering** is by tag name, then plain fields in declaration order, and is `Ord` when every plain field is, so elements can key a sorted map and a list of them sorts the same way under every implementation. State identity across the World boundary uses this equality, so two `Fs.Contents(path="/etc/motd")` values are one state wherever they are built.

Suppose instead that elements were ordinary records with a tag field, `Element(tag="Ui.Button", fields=...)`, and matching compared tag strings. Then a misspelled tag would match nothing and fall to the catch-all, silently, and a field's type would be whatever the record said that time. Tags as declared alternatives of a declared type get the checker involved: an undeclared tag is an error at the construction, a field of the wrong type is an error there too, and a `match` arm for a tag that does not exist is an error rather than dead code.

### Type annotations and inference

Types are inferred by Hindley–Milner over the value layer, with the functor layer read off the sigils. Inside a definition, nothing needs an annotation: `let !delta = -1 on !decrement | +1 on !increment` is checked from its parts. Top-level definitions take a full annotation:

```prism
let Magnitude: fn Vector -> Float = \Sqrt (_.x *. _.x +. _.y *. _.y)
@Button: fn $Text -> @!Click = \@widget:
    !!clicks <- rebuild $_:
        @Element Ui.Button(label=_)
    @ !Switch !!clicks
```

Function types are written `fn A, B -> C`, with the arguments comma-separated the way a call is; a function takes at least one argument, and a function of nothing takes `unit`. A function-typed argument or result is parenthesised, `fn (fn A -> B), [A] -> [B]` and `fn A -> (fn B -> C)`, so the commas and arrow of the inner type never run into the outer one's. A function of one pair is `fn ((A, B)) -> C`, written with the tuple's double parentheses, and differs from `fn A, B -> C` in the way `F ((a, b))` differs from `F a, b`. The `intercept` signature reads with all three rules at once: `fn @a, !(Path ==> Response), $(fn Element -> @$State) -> @((a, $(Path ==> Slot)))`, a function of three arguments, the third a dynamic of functions, returning a blueprint whose result is a pair. The annotation requirement at top level keeps every error local to one definition, since the checker never has to guess a signature from uses in another module, and it makes the typed form of a module readable at its boundary. It is kept rather than relaxed because the cost is one line per definition, and the line is the one a reader of the module wants first. If experience shows that small helper definitions make the cost real, the relaxation is to infer unannotated top-level `let`s that are used only within their module.

Local annotations are allowed anywhere a pattern is: `let n: Nat = 0`, `\|e: Element| ...`, `$x: $Int <- 0 then ...`. On a sigiled name the annotation gives the plain type under the sigil or the full type with it; both spellings are accepted and mean the same thing.

Inference here splits into a value layer and a functor layer. Hindley–Milner runs on plain types and stays principal: the type of `Insert items, id, todo` is found by unification over `k ==> v`, `k`, `v`, exactly as it would be in ML. The functor layer is not unified at all; it is *read*. `$items` is a dynamic because its name says so, and the application `Insert $items, $next, todo` is a dynamic because the operands say so and the lifting rule says what a plain function over dynamics is. No unification variable ever stands for "some functor"; the sigils have already answered the question. That is why a value's kind is readable from its spelling and why the lifting rule can be a table.

## Expressions

The sigils buy lifting. You have written `zipWith (+) counts deltas` and `fmap render model` and `combineLatest([a$, b$]).pipe(map(([a, b]) => a + b))` (RxJS, from memory), and in each case the interesting part was the `+`, which was buried under a word that said how to apply it. Because every operand in Prism says what kind it is, the compiler can apply a plain function across events and dynamics without being told, and the common case, `$count + !delta`, is written as arithmetic. The goal is that the program reads as what it computes and the time structure is inferred from the names, never the other way round: nothing in this section requires inference to decide whether something is an event. The second goal is one way to do each thing, so that two readers reach one parse; the precedence and application rules exist for that and nothing else.

### Lifting

Every operation and every function application is lifted over the time functors. The rule is read off the operands' outermost sigils, not inferred:

- **All operands plain:** the result is plain.
- **Some dynamics, no event:** the result is a dynamic, recomputed whenever any operand changes, in the occurrence of the change. It is prompt: there is no occurrence in which an operand has changed and the result has not.
- **Some dynamics and exactly one event:** the result is an event that fires when the event fires, with the value of the operation applied to the event's value and the dynamics' *old* values. Each dynamic is read through a tag, which is the one window onto "before".
- **Two or more events in one operation:** a compile error naming both and pointing at `|` and `!Leftmost`.
- **A blueprint operand:** not lifted by this rule. Widgets are built, not computed over. What happens to a widget-returning application with a dynamic argument is below.
- **An optional operand** where a plain one is wanted: lifted at the plain layer, giving `?` of the result, as the previous section describes. When an operand has both, `$?x`, the time functor is outermost and lifts first; `?$x` does not lift and must be matched.

Small examples, each the whole of what it says:

```prism
1 + x                     # plain
1 + $x                    # $Nat (or $Int): changes when $x does
$a + $b                   # a dynamic; changes when either does, once per occurrence
$count + !delta           # an event: fires with !delta, reads $count before the occurrence
!a + !b                   # error: two events in one operation
!a | !b                   # the merge: the right way to say "either"
Len $name                 # $Nat: a plain function over a dynamic
IsClick !in               # !Bool: a plain predicate over an event
Keep IsGui, $kids         # a two-argument plain function with one dynamic argument: a dynamic
(Insert $items, $next, todo) on !add    # an event; both dynamics read through tags
Describe $_, $starred     # a dynamic text, inside a for block
Append $log, !line        # an event of arrays
a / b + 1                 # ?Nat: the ? from division lifts through the +
$a / $b                   # $?Nat
?x == 3                   # ?Bool
$count > limit           # $Bool: a comparison, lifted like any operation
"Count: {$count}"         # $Text
"Got {!line}"            # !Text
$"Increment"              # a constant dynamic: nothing to lift over
v on !e                   # an event with v's value when !e fires
$v on !e                  # an event with $v's old value when !e fires
```

`on` has no semantics of its own. It is a binary operator that returns its left operand, and the rule above makes `v on !e` an event that fires with `e` and has the value `v`, and `$v on !e` an event whose value is the tag read of `$v`. Reflex spells that one operator plus the lifting rule as `tag`, `tagPromptlyDyn`, `attachWith`, and (from memory) `attach` and `<@`: the dynamics are always read as they stood before, so there is one `on` and no choice to make.

The two-events error is the rule most worth defending, because the alternative looks friendly. Suppose `!a + !b` meant "fire when either fires, using the other's most recent value". Then every event would need a most-recent value, which is a hold, which is state nobody declared, with an initial value nobody chose. Suppose instead it meant "fire when both fire in one occurrence". That is a real operation, and it is spelled by hand: `!a` and `!b` both fire in some occurrence exactly when both sides of a merge of them fire, and the program that wants that writes it with `|` and a tuple, or holds one and samples it at the other. The error exists because there are three things `!a + !b` could mean and the program has to say which.

The tag read in the third case is the whole of the language's mutation policy in one rule. `$count + !delta` reads the count *before* the click. That turns the counter's `$count <- 0 then $count + !delta` from an equation into a definition: the cycle passes through a read of the old value. It is also why there is no "which version of `$count` did the handler see": at an event there is one version, the one before. If you have debugged a Reflex program where `tagPromptlyDyn` and `tag . current` were both in use, you know which afternoon this prevents.

**Lifting never crosses a sigil boundary.** A function on `A` lifts over `$A` but not over the inner layer of `$$A`; `$$a + 1` is a type error, and `$Join $$a + 1` is what the program meant. Likewise a function on `!A` does not lift over `!!A`. Flattening is `!Switch` and `$Join`, in the switching chapter, and it has laws of its own that a lifted `+` could not keep.

**Lifting over `@`.** A blueprint operand is not computed over, but a widget-returning function applied to a dynamic where it wanted a plain value is a common thing to write, and it has one meaning. `@Element Ui.Button(label=$l)` applies `@Element` to a dynamic, and by the rule above the result is `$@!Click`: a dynamic of blueprints, one per value of `$l`. Binding a dynamic of blueprints with `<-` lifts to `rebuild`:

```prism
!click <- @Element Ui.Button(label=$l)
```

means

```prism
!!clicks <- rebuild $l:
    @Element Ui.Button(label=_)
let !click = !Switch !!clicks
```

The bound name is the `!Switch` of the `rebuild`'s event when the widget's result is an event; for a result that is a dynamic, it is `$Join` of `@HoldFirst` of the `rebuild`'s event, and for a plain result it is `@HoldFirst` of that event, so the bound name is a `$`; for a `unit` result, nothing is bound; for a bundle, each field is treated by its own kind. `@HoldFirst` is the prelude's answer to a hold with nothing to start from. A hold needs an initial value, and an event has none until it fires; `@HoldFirst: fn !a -> @$a` is a dynamic that holds its event's value, with the value of the first firing as its initial value, and the first firing of a `rebuild`'s event is in the occurrence of the first build, so the dynamic always has a value when anything reads it:

```prism
$label <- @HoldFirst !labels        # $label is the latest label, starting with the first one
```

The shorthand is used freely from here on and means exactly the expansion; a program that wants the `rebuild`'s event or a different initial value writes the expansion.

### Application

Application is juxtaposition with comma-separated arguments, under the rules of the paren-less call section in the lexical part of this chapter; this section places it among the operators. A paren-less call takes the rest of its expression, so in the precedence table it sits just above the statement forms: `F a | G b` is `F(a | G(b))`, and `1 + F a` is `1 + F(a)`, because nothing precedes a call's name that the call could absorb. Parentheses make any other grouping, and `Size $elems + 1` against `(Size $elems) + 1` is the case to watch, argued in that section.

```prism
@Label $"Count: {$count}"                     # one argument
Len $name                                     # one argument, lifted
!Filter (HitTest $slots, !pointer)            # the group is a two-argument call
$live on (!started | !refresh)                # on binds tighter than |, so the parentheses are needed
{!Filter (HitTest $slots, !pointer): Click}   # a one-entry map with a lifted key
```

Method form and chains follow the same section: `x.F y, z` is `F x, y, z`, `a.b.F c` is `F (a.b), c`, and a chain step that takes arguments starts its own line.

### Precedence

Loosest to tightest:

| | Operator | Reads as |
|---|---|---|
| 1 | `<-`, `let ... =` | statement level, not expressions |
| 2 | `then` | hold |
| 3 | paren-less application | to the right, to the end of the expression |
| 4 | `\|` | merge |
| 5 | `on` | sample |
| 6 | `or` | boolean or |
| 7 | `and` | boolean and |
| 8 | prefix `not` | boolean negation |
| 9 | `<` `<=` `>` `>=` `==` `!=` | comparison |
| 10 | `+` `-` and `+~` `-~` `+.` `-.` | additive |
| 11 | `*` `/` and `*~` `/~` `*.` `/.` | multiplicative |
| 12 | prefix `+` `-` `+?` and free sigils `!` `$` `@` `?` | prefix |
| 13 | `.field`, `[index]`, `.Method`, attached sigils | postfix |

Binary operators at one level associate to the left, except `then`, which does not associate: `a then b then c` is an error. The counter's lines are the test of the table: `$count <- 0 then $count + !delta` is `$count <- (0 then ($count + !delta))`; `-1 on !decrement | +1 on !increment` is `(-1 on !decrement) | (+1 on !increment)`; `$count + 1 on !click | 0 on !reset` is `(($count + 1) on !click) | (0 on !reset)`.

`on` tighter than `|` lets a merge of samples read without parentheses, which is the form of every "what happened" event in the worked examples. `on` looser than arithmetic makes `$count + 1 on !click` mean "the count plus one, at the click" rather than "the count, plus (one at the click)", which would be a two-event error waiting to happen. Under both choices the first program anyone writes needs no parentheses, and a rule that made the counter uglier would need a better reason than any of these. Comparison sits between `on` and arithmetic, so `$count + 1 > limit on !click` compares the sum and samples the answer at the click. The boolean operators sit between `on` and comparison, so a condition reads the way it is said: `$done or $count > limit on !click` is `($done or ($count > limit)) on !click`, `a < b and c < d` is `(a < b) and (c < d)`, and `x or y and z` is `x or (y and z)`. Sampling still wraps the whole condition without parentheses, which is what `on` being looser than every value operator buys. `not` sits between `and` and comparison, so it negates a whole comparison: `not a == b` is `not (a == b)`, `not $n + 1 > limit` is `not (($n + 1) > limit)`, and `not done and ready` is `(not done) and ready`, because `and` is looser. The operand of `not` is everything tighter than it, which is why `not $done` and `not a.ready` also read as expected. Suppose `not` bound tighter than comparison, as the unary minus does. Then `not a == b` would be `(not a) == b`, a type error in nearly every program that writes it, since the writer meant the negated comparison; the precedence that makes the line mean what it says is the one to choose. `not` is a prefix operator, so it cannot be the right operand of a tighter operator: `a == not b` is a parse error whose message asks for `a == (not b)`.

A free sigil at level 12 applies to everything to its right up to the end of the enclosing call or bracket, because it is prefix and nothing at level 12 or looser follows it: `@ ChecklistUpdate ix, !checked` is `@ (ChecklistUpdate ix, !checked)` since the call at level 3 is looser and extends under it. The free sigil and the paren-less call therefore nest the obvious way: the sigil wraps the call. `$ x + 1` is `$ (x + 1)` by the same reading.

A `.Method` that starts a continuation line stands outside the table. It closes everything open on the lines above, back to the statement's `=`, `<-` or `then`, and takes that whole expression as its receiver; level 13 covers only a `.` written against the expression before it.

### Statements

Three statement forms exist, and they are the only places where the sigil-level structure of a program is built rather than computed. They are statements, not expressions, because each one does something an expression cannot: a bind builds a widget, a hold owns state, and a `let` introduces a name that later statements in the same block can see.

```prism
!click <- @Button $"OK"                  # bind: build the widget, name its result
@Element Time.Every(period=100)          # bind with the result discarded
((!lines, !eof)) <- @Lines               # bind with a tuple pattern over a bundle
TextInput(!changed, $value) <- @TextInput ""   # bind with a record pattern
let !delta = -1 on !decrement | +1 on !increment   # let: name a value, build nothing
let @b = @Button $"OK"                   # let of a widget: a description, unbuilt
$count <- 0 then $count + !delta           # hold
$$current <- $ Unavailable then !$contents # hold of dynamics, for a $Join
```

A bind's left side is a pattern, and every name in it takes the sigil of what it is bound to. A `let` is the same, with no building: `let @b = @Button $"OK"` followed by `!x <- @b` and `!y <- @b` builds two buttons, because a blueprint is a description and each bind of it is a widget. A hold's left side is a single sigiled name, because a hold is one piece of state with one definition; a program that wants two holds writes two.

Outside a widget block, only `let` exists, and a sequence of `let`s ending in an expression is how a lambda body or a `for` body with intermediate names is written:

```prism
let Magnitude: fn Vector -> Float = \match _:
    Vector(x, y):
        let xx = x *. x
        let yy = y *. y
        Sqrt (xx +. yy)
```

Statements in a widget block are mutually recursive and may be written in any order; the resolver finds the order. Statements outside one are sequential: a `let` sees the names above it and not below, since plain values cannot be cyclic and there is nothing to be gained by pretending they can.

### Lambdas and the pronoun

`\body` is a one-argument function whose argument is `_`:

```prism
\IsGui _.element                      # a predicate on slots
\_.element                            # a projection
\not IsGui _.element                  # the complement; the call takes the rest of the body
\Test.Box(value=_)                    # a constructor
\Sqrt (_.x *. _.x +. _.y *. _.y)      # a magnitude
```

Wherever `_` is in scope, a `.` or `[` in prefix position (by the spacing rule) applies to it: `.x` means `_.x` and `[i]` means `_[i]`. This holds in a lambda body, in a `for` body and in any other construct that binds the pronoun. So `\.element` is the projection above, `\[0]` indexes the argument, and `for todos: .id` projects each todo. Only one `_` is in scope at a time; an inner lambda's `_` shadows the outer one, and a program that needs both names one of them. A `.Method` that starts a continuation line is a chain step on the call above it, not a pronoun use.

`\|a, b| body` names the arguments; one of them may still be the pronoun, `\|_, k| ...`, and any of them may be a pattern, `\|((x, y)), k| ...`, or `ignore`:

```prism
let Step: fn Watch, Instant, Action -> Watch = \|w, now, a| match a: ...
Fold changes, items, \|acc, id, c| Step acc, id, c
\|ignore, v| v
```

`\@widget:` opens a widget block as the body, which is how widget functions are written, and `\|$slots| @widget:` names a dynamic argument for one:

```prism
@Button: fn $Text -> @!Click = \@widget:
    !!clicks <- rebuild $_:
        @Element Ui.Button(label=_)
    @ !Switch !!clicks

@Forward: fn $(Path ==> Slot) -> @!(Path ==> Response) = \|$slots| @widget:
    for $slots:
        rebuild $_:
            ((!resp, ignore)) <- @Element _.element
            @ !resp
```

**The argument's kind.** A lambda's argument has a sigil like any name, and the rule is this: a body that mentions a sigiled pronoun, `$_`, `!_` or `@_`, takes an argument of that kind and is not lifted over it. `@Button`'s body says `rebuild $_:`, so it takes a dynamic; a lambda whose body says `intercept @_, ...` takes a blueprint. A named argument has the sigil it is written with. A body with no sigiled pronoun and no annotation takes a plain argument, and the lambda is a plain function.

A plain function is lifted afresh at each application: one definition serves a plain argument and a dynamic one, so `IsGui` written once over a plain `Element` is applied to `$kids` and to `!in` without a second definition. That is why the prelude's `Len` and `Insert` are written at plain types and used over everything, and the operators `not`, `and`, `or` and `+` lift the same way. The alternative, inferring a lambda's binder sigil from its uses, would mean a lambda applied to a dynamic takes `$_` and cannot also take a plain value, so the prelude would need two of everything.

When a lambda with no sigiled pronoun is written where the expected type says the argument is a dynamic, as in `rebuild` blocks and the `intercept` supplier, the argument takes the kind the expected type says; with no expected type it is plain. This is bidirectional checking in the one place it is needed. It is tentative in exactly one respect: the alternative, inferring functor variables alongside Hindley–Milner so that a lambda's kind is unified from its uses, would make the rule uniform at the cost of the property that a value's kind is readable from its spelling, and if that cost turns out to be lower than it looks, this is the rule that would change.

### `match` and `if`

```prism
match shape:
    Circle r: Pi *. r *. r
    Square s: s *. s

match todo:
    TodoItem(label, checked=True): Some label
    TodoItem(..): Nil

match items[id]:
    Some t: Insert items, id, (Todo t.label, not t.done)
    Nil: items

match element:
    Ui.Button(label=l): ...
    Ui.*(..): ...
    ignore: ...
```

A `match` has a scrutinee and a block of arms, each `pattern: expression` or `pattern:` followed by a layout block, and optionally a guard between the pattern and the colon (below). Arms are tried in order and the first that matches is taken. A `match` on a closed `choice` must be exhaustive; one on an extensible variant must end in a catch-all arm; one on `?T` is the eliminator for optionals and must have both a `Some` and a `Nil` arm (or a catch-all); one on an array must cover every length (the array patterns, below). A guarded arm does not count toward any of these.

**Patterns:**

- A literal: `0`, `"quit"`, `'x'`, `unit`.
- A lowercase name, which binds. `_` binds the pronoun.
- `ignore`, which matches anything and binds nothing.
- `((p, q))` for tuples; `((p,))` for a one-tuple.
- `Some p` and `Nil`.
- `Type p` for a newtype or a choice alternative with a payload: `Circle r`, `Running from`.
- `Type` alone for a unit alternative: `Stopped`, `Backspace`.
- `[[]]` for the empty array; `[[p, q]]` for an array of exactly two elements; `[[head, ..rest]]` for a non-empty array, with `head` the first element and `rest` the array of the others. The patterns mirror the literals: `[[p1, ..., pn]]` matches exactly `n` elements, and `[[p1, ..., pn, ..rest]]` matches `n` or more, binding the remainder (`..` alone leaves it unbound). The `..rest` comes last.
- `Type(p1, p2)` for a record, matched positionally; `Type(f=p, g=q)` by field name; `Type(f, g)` as shorthand for `Type(f=f, g=g)`, binding each field to a name of its own; `..` allowing the remaining fields to go unmatched. A record pattern that names every field needs no `..`.
- `Ns.Tag(...)` for a tag of an extensible variant, with the same record forms; `Ns.*(..)` for any tag under a namespace; a bare `Ns.Tag` for a tag with no fields.
- `p: Type` ascribes a type to a pattern.

**Array patterns.** You have walked an array by hand: check the length, index the front with `xs[0]`, get an `?T` back for an element you just proved was there, and match it anyway. An array pattern asks the question once. The empty case and the non-empty case are the two arms that cover every array, and the non-empty one hands over the first element and the rest:

```prism
let Summary: fn [Text] -> Text = \match _:
    [[]]: "nothing"
    [[only]]: only
    [[first, ..rest]]: "{first} and {Len rest} more"
```

The arms cover every length: none, exactly one, and one or more. The middle arm only picks out a case of the last one, and a program with just the first and the last is exhaustive too. A `match` on an array is exhaustive when its unguarded arms cover every length; `[[a, b]]` alone leaves out every other length, and the missing-case message names a length it does not cover.

**Guards.** You have written an arm that matches a pattern and then immediately checks a condition on what it bound, and had to repeat the fall-through case because the nested `if` cannot send control back to the next arm. A guard sits between the pattern and the colon, `pattern if condition:`, and the arm is taken only when the pattern matches and the condition, with the pattern's bindings in scope, is `True`. Otherwise the next arm is tried:

```prism
match items[id]:
    Some t if t.done: Insert items, id, (Todo t.label, False)
    Some t: items
    Nil: items
```

Without the guard, the second arm's work would be inside the first as an `if` and the `Some t: items` case would be written twice. The condition runs from `if` to the arm's colon, so an `if` expression inside a guard takes parentheses. A guarded arm does not count toward exhaustiveness: the compiler does not reason about conditions, so `Some t if t.done` covers nothing, and the `match` above is exhaustive only because the unguarded `Some t` and `Nil` follow. A guarded `ignore if c:` is not a catch-all for an extensible variant. A guard on a dynamic scrutinee is lifted with the rest of the arm, and dynamics it mentions are read as the arm body's are.

**`match` lifts like application.** With all arms plain, `match $x: ...` is a dynamic of the arms' results, changing when `$x` does, and `match !e: ...` is an event firing when `!e` does, with the taken arm's value. Dynamics mentioned in the arms are lifted by the same rule as any operation: in `match !e: Some v: v + $offset`, the `$offset` is read through a tag. When any arm is a blueprint, the scrutinee must be plain, because choosing a blueprint by a changing value is construction and is spelled `rebuild`; the error says so and points at it.

```prism
let $text = match $contents:
    Unavailable: "(unavailable)"
    Known t: t

let !line = match !ev:
    Received t: Some t
    Closed: Nil
```

`if c: a` then `else: b`, on one line or on two lines with `else` at the column of `if`, is `match c:` on `Bool`, with `True: a` and `False: b`. It lifts the same way: `if $done: "x" else: " "` is a dynamic text. An `if` without an `else` is an error, because every expression has a value and there is no `unit` to fall back on silently; a program that wants "do nothing" says `unit` in the `else`, and one that wants "nothing" says `Nil` in it: `if n == 1: Some unit else: Nil` is a `?unit`.

### Text interpolation

`"... {expr} ..."` builds text, lifted by the rule above. The braces hold an expression, usually a name, and the name keeps its sigil: `{$count}` interpolates a dynamic, `{!line}` an event, `{name}` a plain value. The interpolated value must be `Text` or a primitive with a canonical rendering (`Nat`, `Int`, `Float`, `Char`); anything else is a type error, not an implicit `ToText`. A record interpolated into a label would print as something, and the something would be a format nobody chose; the error costs a call to a rendering function the program had to write anyway.

```prism
$"Count: {$count}"                     # $Text over a $Int
"{Total $watch, $clock} ms"            # $Text over two dynamics, through a plain function
"{name} has {n} items"                 # plain, if both are
"Line: {!line}"                        # !Text
"Got {?maybe}"                         # ?Text: the ? lifts at the plain layer
"cost: $5"                             # plain text; the $ is only a character
"set \{x\}"                           # the characters {x}
```

Text has one interpolation marker, `{`, and an escape for it. A `$` in text is never special, so a price, a shell variable or a currency sign needs no escape, and `"cost: $5"` is the text it looks like. Suppose `$name` also interpolated. Then every dollar amount in a label would be a question about what follows the `$`, and a name written next to a digit or a space would silently turn into text or an error depending on the next character. With one marker the reader asks one question of any literal: is there a brace? The braces are also what a reader already knows from other languages' format strings (from memory: Python's f-strings and Rust's `format!` both use `{}`), and the cost is that a literal brace is written `\{` or `\}`.

### `for` as an expression

Outside a widget block, `for xs: body` maps `body` over a plain array or map. In the shorthand, `_` is the element and `key` is the key of the entry: the map key for a map, and the position, a `Nat`, for an array:

```prism
let squares = for [[1, 2, 3]]: _ * _
let ids = for todos: .id                       # prefix .id is _.id
let doubled = for prices: _ *. 2.0             # a map, keyed as the input
let labelled = for names: "{key}: {_}"         # key is the position in an array
```

You have nested two loops and needed the outer index inside the inner one, and found the inner loop had taken the name. The shorthand binds the same `_` and `key` at every level, so an inner loop hides the outer one. Named binders say which is which: `for k, item <- xs: body` binds `k` to the key and `item` to the element, and binds nothing else, so an enclosing loop's `_` and `key` stay visible:

```prism
let cells = for r, row <- grid:
    for c, cell <- row:
        "{r},{c}: {cell}"
```

The binder before `<-` is a pattern, so it takes the sigil of what it is bound to. Over a plain collection both are plain, `for k, item <- xs:`. Over a dynamic collection in a widget block the key is plain, since it is the identity of the block, and the element is the per-key dynamic, written `for k, $item <- $items:`; its body sees `$item` in the way the shorthand's body sees `$_`, and the key `k` where the shorthand sees `key`. `for $items:` is the shorthand for `for key, $_ <- $items:`. Writing `for k, item <- $items:` is an error that asks for `$item`, because the per-key value is a dynamic. There is no binder for a third thing: a loop that wants more than the key and the element takes them from the element.

It evaluates to an array when the input is an array and a map when the input is a map, in key order. The body is one expression or a layout block of `let`s ending in a value. Over a dynamic collection it is the construct of the switching chapter and is only allowed in a widget block, because it builds widgets; the error for `for $m:` outside a widget block says so.

## Diagnostics

A language whose syntax leans on spacing, sigils and inference is a language whose error messages are part of its design, because the mistakes a newcomer makes are predictable and each one has a right answer the compiler can name. Every node of the parse tree and of the typed tree has a source span from the first commit, since spans are the one thing a compiler cannot retrofit cheaply. The errors below are the ones the conformance suite pins with negative goldens: a rejected program and the exact message, spans included. Each is given with the form of its message; the wording is the implementation's, the content is not.

**Two events in one operation.**

```
error: two events in one operation
  --> counter.prism:4:17
   |
 4 |     let !delta = !increment + !decrement
   |                  ^^^^^^^^^^   ^^^^^^^^^^ both are events
   = an operation may lift over any number of dynamics and at most one event
   = to fire on either, merge them: `!increment | !decrement`
   = to fire on both in one occurrence, hold one and sample it at the other
```

Names both events, points at `|` and `!Leftmost`, and says what the two meanings of "both" would be spelled as.

**A cycle with no delayed edge.**

```
error: these bindings depend on each other in the same occurrence
  --> loop.prism:3:5
   |
 3 |     let $a = $b + 1
   |         ^^ depends on $b
 4 |     let $b = $a * 2
   |         ^^ depends on $a
   = a cycle in a widget block must pass through a hold's value, a tag, or a !Switch
   = a `rebuild` is not a delay: its event fires in the occurrence of the rebuild
```

Lists every binding in the cycle, in order, with spans. A cycle through `rebuild` alone gets the same message with the last line as its reason.

**A negative number where a subtraction was meant.**

```
error: `a` is not a function
  --> math.prism:7:12
   |
 7 |     let n = a -1
   |             ^ ^^ applied to -1
   = did you mean `a - 1`?
```

**A suffixed operator against known types.** A program using `+~` on two values already known to be `Float`, or `+.` on `Int`s, gets "operator `+~` is Int addition, but both operands are Float; use `+.` or `+`".

**A name whose sigil does not match its type.**

```
error: `count` is bound to a dynamic, but its name has no sigil
  --> counter.prism:5:5
   |
 5 |     let count = $total + 1
   |         ^^^^^ a dynamic must be named with `$`
   = write `$count`
```

The reverse, `let $x = 3` with a plain right side, says "a plain value must be named without a sigil, or lifted: `$ 3`". A name with the wrong sigil, `!x` bound to a dynamic, says which it is and which it should be.

**A name that strays from its convention.**

```
warning: `isClick` is a value name that is not snake_case
  --> gui.prism:2:5
   = values are snake_case (`is_click`); functions and types are UpperCamelCase (`IsClick`)
```

The lint is a warning. `Todo_item`, an uppercase name with an underscore, is an error with the same two lines of explanation.

**A match on an extensible variant without a catch-all.**

```
error: this match on `Element` has no catch-all arm
  --> fwd.prism:9:13
   |
 9 |             match _.element:
   |             ^^^^^ `Element` is an extensible variant; tags may be declared elsewhere
   = add a final arm: `ignore: ...`
```

**A non-exhaustive match on a closed choice.**

```
error: match does not cover `Backspace`
  --> edit.prism:12:5
   = `Key` has alternatives Char, Left, Right, Backspace; add an arm or `ignore:`
```

**A match on a dynamic with a blueprint arm.**

```
error: choosing a widget by a changing value
  --> view.prism:4:5
   |
 4 |     match $mode:
   |           ^^^^^ the scrutinee is a dynamic and an arm is a widget
   = building depends on the value; write `rebuild $mode:` and match inside it
```

**A hold outside a widget block.**

```
error: a hold must be inside a widget block
  --> helper.prism:3:5
   |
 3 |     $n <- 0 then $n + !e
   |     ^^^^^^^^^^^^^^^^^^ holds are owned by a widget's node
```

**Lifting across a sigil boundary.**

```
error: `+` cannot lift into the inner layer of `$$a`
  --> join.prism:6:13
   = lifting covers the outermost sigil only; flatten first: `$Join $$a + 1`
```

**An optional where a plain value is required at a boundary.**

```
error: `?Nat` where `Nat` is required
  --> idx.prism:4:21
   |
 4 |     @Element Ui.Row(height=xs[i])
   |                             ^^^^^ indexing returns ?Nat
   = element fields are plain; match on it, or give a default: `Or xs[i], 0`
```

Lifting over `?` stops at any position whose type is fixed by a declaration: an element's field, an annotated binding, a tag's `requests` event. The message says where the `?` came from.

**A type variable in a tag's field.**

```
error: tag field `value` mentions the type variable `a`
  --> lib.prism:8:5
   |
 8 |     let Wrap: fn a -> Element = \Test.Box(value=_)
   |                                              ^ `a` is not a concrete type
   = a tag's fields have concrete types; build the element at a known type
```

**Interpolating something without a rendering.**

```
error: cannot interpolate a `Todo` into text
  --> list.prism:7:22
   = text interpolation takes Text, Nat, Int, Float and Char; render it: `{Describe t}`
```

**Layout.** "expected an indented block after `:`", with the span of the colon and the line that was found instead; "unindent does not match any open block", with the line's column and the columns currently open (`open blocks are at columns 0, 4, 8; this line is at 6`); "tab character in indentation", with position. The second message's list of open columns turns a layout error from a puzzle into a one-keystroke fix.

**Reserved words.** `read`, `update` and `&` each say "reserved for a future version of the language; not available", so that a reader who tries one learns that it is intended rather than misspelled. `%` is reported as an unknown character.

**`for` over a dynamic outside a widget block.** "`for` over a dynamic collection builds widgets and must be inside a widget block"; with a plain collection inside a widget block it is the plain mapping and needs no message.

**`ix` outside a `for`.** "`ix` is bound only inside a `for` in an `update` block"; likewise `jx` and `kx`, and `jx` inside a single loop says "`jx` is the index of the second-nested `for`; this loop's index is `ix`".

**`key` outside a shorthand `for`.** "`key` is bound only inside a `for` written without `<-`; name the key in `for k, x <- xs:`". The same message applies inside a loop with named binders, where `key` is not bound.

**An `if` without `else`.** "every `if` has an `else`; this one has no value when the condition is False".

**A guarded arm in an exhaustiveness check.** "match does not cover `Nil`; the arm `Some t if t.done` has a guard and does not count". The message names the guard so that a program that thought the guarded arm covered its case finds out why it did not.

**A match on an array that misses a length.** "match does not cover arrays of length 0; add `[[]]` or `ignore:`".

**Unknown path and other runtime errors** belong to the error chapter; the compiler's job is the ones above, and the test for each is that the message contains the fix.

## Layout corpus

The programs on which the two lexers must produce identical token streams. Each is given with its stream sketched in a compact notation: `⟨` opens a layout block, `;` is a statement separator, `⟩` closes a block; `:` is an inline colon token; names are written as they appear, with attached sigils part of the token; `$free` marks a free sigil; `TEXT(...)` is a text literal with its parts, a plain segment as a string and an interpolation as its tokens in braces; `-pre` and `-bin` mark the prefix and binary readings; `\|` opens lambda bars and `|)` closes them, as distinct from the merge `|`. The streams omit nothing.

**1. The counter.** A statement continued by nothing; every line a statement.

```prism
@counter:
    !increment <- @Button $"Increment"
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 then $count + !delta
    @Label $"Count: {$count}"
```

```
@counter ⟨ !increment <- @Button $TEXT("Increment")
; let !delta = -pre 1 on !decrement | +pre 1 on !increment
; $count <- 0 then $count +bin !delta
; @Label $TEXT("Count: ", { $count }) ⟩
```

Note `$"Increment"` is an attached sigil on the text token, and `$"Count: {$count}"` is likewise attached with an interpolation part inside. `-1` is the prefix reading because the `-` follows whitespace and precedes a non-space.

**2. A continued statement inside a block.**

```prism
@form:
    $edit <- Edit initial, (Len initial)
        then Apply $edit, !key
    @Label (Render $edit)
```

```
@form ⟨ $edit <- Edit initial , ( Len initial ) then Apply $edit , !key
; @Label ( Render $edit ) ⟩
```

The continuation line produces no separator and no block tokens.

**3. Two blocks closed by one dedent.**

```prism
@outer:
    !x <- rebuild $a:
        !y <- @Button $"B"
        @ !y
    @Label $"done"
```

```
@outer ⟨ !x <- rebuild $a ⟨ !y <- @Button $TEXT("B") ; @free !y ⟩
; @Label $TEXT("done") ⟩
```

**4. Dedent of two levels at once, to column 0.**

```prism
@a:
    !x <- rebuild $b:
        @ unit
@c:
    @ unit
```

```
@a ⟨ !x <- rebuild $b ⟨ @free unit ⟩ ⟩ ; @c ⟨ @free unit ⟩
```

Two closes, then the separator for the new top-level statement.

**5. Inline arm then layout arm.**

```prism
let Area: fn Shape -> Float = \match _:
    Circle r: Pi *. r *. r
    Square s:
        let side = s
        side *. side
```

```
let Area : fn Shape -> Float = \ match _ ⟨ Circle r : Pi *. r *. r
; Square s ⟨ let side = s ; side *. side ⟩ ⟩
```

The first `:` after `Area` is an inline colon, not a block opener, because tokens follow it on the line; the parser reads an annotation there because the statement began with `let`. The `:` after `_` is line-final and opens the layout block.

**6. `:` inside brackets, across lines.**

```prism
let m = {
    "a": 1,
    "b": if c: 2 else: 3
}
```

```
let m = { TEXT("a") : 1 , TEXT("b") : if c : 2 else : 3 }
```

No block tokens anywhere; the braces suspend layout and the colons are plain colon tokens.

**7. A trailing comment after the opening colon, and comment-only lines inside the block.**

```prism
@counter:    # the counter
    # the first button
    !increment <- @Button $"Increment"

    # a blank line above, a comment here
    @Label $"ok"
```

```
@counter ⟨ !increment <- @Button $TEXT("Increment") ; @Label $TEXT("ok") ⟩
```

**8. `F -1` against `F - 1` against `F-1`.**

```prism
let a = F -1
let b = F - 1
let c = F-1
let d = F - -1
```

```
let a = F -pre 1 ; let b = F -bin 1 ; let c = F -bin 1 ; let d = F -bin -pre 1
```

**9. Merge against lambda bars.**

```prism
let m = \|a, b| a | b
let n = !x | !y
```

```
let m = \| a , b |) a | b ; let n = !x | !y
```

**10. A sigil before a bracket, both spellings.**

```prism
@ ((!a, $b))
@((!a, $b))
```

```
@free (( !a , $b )) ; @free (( !a , $b ))
```

Identical streams.

**11. A test definition: the `=>` continuation lines.**

```prism
test counter, @counter:
    build
        => expect Ui.Label(text="Count: 0")
    click Ui.Button(label="Increment")
        => expect Ui.Label(text="Count: 1")
```

```
test counter , @counter ⟨ build => expect Ui.Label ( text = TEXT("Count: 0") )
; click Ui.Button ( label = TEXT("Increment") ) => expect Ui.Label ( text = TEXT("Count: 1") ) ⟩
```

The line-final colon opens the block; the `=>` lines are indented more than the block's column, so by rule 3 they continue the turn line above them, and the `test` form reads each `=>` as one check of that turn. Two `=>` lines under one turn are two continuations of one statement.

**12. The `for`-then-merge error.**

```prism
@bad:
    !x <- for $m:
        @ !click
      | !other
```

```
@bad ⟨ !x <- for $m ⟨ @free !click ⟩ ERROR(unindent to column 6 matches no open block: 0, 4)
```

Both lexers must report the error at the `|` line with those columns.

**13. A method chain with the spacing rule.**

```prism
let a = items.Filter IsDone
    .MapValues Label
let b = Filter items, \.done
```

```
let a = items .bin Filter IsDone .chain MapValues Label ; let b = Filter items , \ .pre done
```

`.bin` after `items` is method form; `.chain` marks a `.` that is the first token of a continuation line, which the parser reads as a step whose receiver is everything back to the `=`; `.pre` after `\` is field access on `_`. The one-line spelling `items.Filter IsDone .MapValues Label` is an error from the parser, since its `.` is neither first on its line nor attached.

**14. Interpolation containing text and a colon.**

```prism
let s = "{if c: "yes" else: "no"}!"
```

```
let s = TEXT( { if c : TEXT("yes") else : TEXT("no") } , "!" )
```

One token, with one interpolation part holding seven tokens and one text part.

**15. A record block as an argument.**

```prism
let origin = Vector:
    x: 0.0
    y: 0.0
```

```
let origin = Vector ⟨ x : 0.0 ; y : 0.0 ⟩
```

**16. `else` at the column of `if`, with layout blocks in both branches.**

```prism
let v = if c:
    let t = 1
    t + 1
else:
    0
```

```
let v = if c ⟨ let t = 1 ; t +bin 1 ⟩ ; else ⟨ 0 ⟩
```

The separator before `else` is emitted by the lexer, since `else` is at the block column; the parser joins the two statements. The whole thing is one `let`.

**17. A tab in leading whitespace.**

```
@a:
⇥   @ unit
```

```
@a ERROR(tab in indentation at 2:1)
```

**18. Nested namespaces and the wildcard.**

```prism
match e:
    Ui.Text.Input(..): 1
    Ui.*(..): 2
    ignore: 3
```

```
match e ⟨ Ui.Text.Input ( .. ) : 1 ; Ui . * ( .. ) : 2 ; ignore : 3 ⟩
```

`Ui.Text.Input` is one qualified-name token. `Ui.*` lexes as the name `Ui`, a binary `.`, and `*`; the parser recognises the three as the wildcard pattern.

**19. A dollar sign and an escaped brace in text.**

```prism
let s = "cost: $5, {n} items, \{literal\}, $count"
```

```
let s = TEXT("cost: $5, ", { n }, " items, {literal}, $count")
```

One token with three parts: a plain segment, an interpolation holding one token, and a plain segment. Every `$` in text is a character, and the escapes `\{` and `\}` are decoded by the lexer into the plain segment, so the parser never sees a brace that is not an interpolation.

Both implementations keep this corpus as a golden test: the source on one side, the stream on the other, diffed. A new ugly case found in the field is added here first and to the lexers second.

## Negative space of the language core

The language's refusals are listed with the whole design in the first part of the book; these are the ones that belong to the core, each with what its exclusion buys.

**No assignment.** `count = count + 1` cannot be written. Not in a handler, not in a helper, not in a `let`. The only state is a hold, and a hold moves only when its event fires. What this buys the core: every binding is an equation, so a widget block can be read in any order and the resolver can build it in dependency order; `let` is a definition and never an update, so two readers of a block agree on what a name means without tracing control flow; and because every write is a hold's commit, an occurrence is a transaction for free.

**No sampling.** There is no `.current`, `.value` or `sample` on a dynamic. The only reads are a tag (in a lifted operation with an event), a derived dynamic (prompt), and `rebuild` (rebuild). What this buys: there is no "which version" question, since each of the three reads has one defined answer, and the implementation is free to propagate in any dependency-respecting order because nothing can observe the difference.

**No custom monads, no effect system.** One block type per functor, and the set is fixed: widget blocks, `rebuild`, `for`, `match`. There is no `do`, no transformer stack, no way to define a fourth functor with its own bind. What this buys: the lifting rule is a table over four sigils; a name's kind is readable from its spelling because the set of kinds is closed; and the compiler's elaboration is a lookup, not a search through instances. A program that wants a `Reader` passes an argument; one that wants a `with` has a hold; one that wants `IO` has an element.

**No overloading beyond the three traits.** `Eq`, `Ord` and `Num` are resolved by inference; nothing else is. There are no user-defined traits, no instances, no operator definitions, no implicit coercions through a class. What this buys: a call site names one function, and the reader can find it; inference stays Hindley–Milner on the value layer with three well-understood exceptions, and the checker's error messages stay local. The three are the ones without which sorting, map keys, element identity and arithmetic cannot be written; everything else has a name.

**No implicit conversions.** `Nat` does not become `Int`, `Int` does not become `Float`, `Text` does not become anything, a newtype does not become its payload and a `?T` does not become a `T`. Conversions are functions (`+?`, `Abs`, `ToText`, `.value`) and the one lifting over `?` is visible in the result's sigil. What this buys: the type of every expression is the type of its parts, and a count never becomes a float because it was divided; a line of arithmetic has one numeric type and the reader can find it at the nearest annotation or suffixed operator.

**No reified identifiers, no introspection.** There is no way to name a binding as data, reach into a widget, list a block's bindings, or ask a value for its type. A test drives a program through its elements, the way a World does, and reads its element list and its result. What this buys: a test that passes under the Test World passes under any World, since it used only the boundary; a name can be renamed without breaking anything but the program; and generic code can be erased, because nothing can ask a value what it is.

**No symbol with two meanings.** `<=` is less-or-equal and nothing else, `then` is hold and nothing else, and `<-` is bind and nothing else. What this buys: the line that defines a program's state never looks like a comparison, and a comparison never looks like state.

**No explicit-brace layout, no block comments, no tabs, no `else`-less `if`.** Each of these removes a second way to write something, and each second way is one more case on which two parsers can disagree. The spec is the thing, and the thing is shorter for it.

**No `read` and `update` blocks, no `&` references, no lenses.** These are described at the end of this chapter, and no transcript depends on them. Plain values are built, never modified; a function that "updates" a record builds a new one. What this buys now: the value layer has one operation on a record, construction, and the question of how mutation inside a plain value interacts with sharing does not arise. If they come, they come as sugar over construction, and nothing in this chapter changes meaning.

## `&` references, `read` and `update` blocks

You have a record with a record in it with a map in it, and you need to add one to a counter three levels down. In an imperative language it is a line. In a pure language it is a `let` per level, each one rebuilding the enclosing record with one field replaced, and the line that does the work is at the bottom of a staircase of plumbing. This is the one place a pure language is worse to write than an imperative one, and it is the one `update` blocks exist to fix. An `update` block sits beside the book's statements that the language has no assignment without contradicting the language the rest of the book specifies: it is a separate block form inside plain code, its value is an updated copy of a value, nothing outside the block sees a write, and nothing else in the book depends on it. Nothing in the other chapters needs them, and the semantics of references under erasure and copy-on-write are the part that has to be designed before the Rust implementation can commit to a representation.

### What the feature is

Three pieces, each small on its own.

A `read` block gives the pronoun over a value. `\read _:` opens a block in which `_` is the lambda's argument, `.x` reads a field of it, and `[k]` indexes it, as in any lambda or `for` body; `read v:` gives the same prefix `.x` and `[k]` over any value `v`, and an `update` block gives them over the value being updated, and `read p <- v:` binds a pattern. Only `let` statements and one result statement are allowed. The point is not saving characters. A function over a large value should read like a description of the value:

```prism
type Vector = record:
    x: Float
    y: Float
    z: Float

let Magnitude: fn Vector -> Float = \read _:
    let x = .x
    let y = .y
    let z = .z
    Sqrt (x * x + y * y + z * z)
```

An `update` block is an expression that evaluates to an updated copy of a value. Inside it, `&_` is a reference to the value being updated and `&_.x`, `&_[k]` are references into it. A reference can be set with `<=`, or combined with an operator: `+=`, `-=`, and so on for every operator the type has:

```prism
let Add: fn Vector, Vector -> Vector = \|a, b| update a:
    &_.x += b.x
    &_.y <= .y + b.y      # the same thing, spelled out
    .z += b.z             # the pronoun is implicit on the left too
```

The original `a` is unchanged. The block's value is the updated copy. Statements run in sequence, and a later statement reads the value as earlier statements left it, which is what makes the block worth having: `&_.x += 1` followed by `let t = .x` sees the new `x`.

References, `&T`, exist only inside an `update` block. A function may take a reference and return one, `fn &Vector -> &Float`, and that is a lens: it says where inside a `Vector` a `Float` lives, and applied inside an `update` it gives a reference you can set. A function may not take two references, return a reference it did not derive from its argument, or store one anywhere; a reference is a path into the value being updated, and a path only means something while the update is happening. `*r` dereferences for reading and gives the current value at the path.

Two conveniences fall out. A function whose result type is its first argument's type, `fn Vector, Vector -> Vector`, is an *update function*, and inside an `update` block it may be used as a statement with its first argument implied: `Add b` inside `update a:` means `&_ <= Add _, b`. And `for` inside an `update` block over a reference to an array or map yields a reference per element:

```prism
let AddByElement: fn [Float], [Float] -> [Float] = \|a, b| update a:
    for &_:               # `&_` inside the loop is a reference to a[ix]
        &_ += b[ix]
```

`ix` is the index of the current iteration; nested loops get `jx` and `kx`, and past that a helper function is better than another keyword.

### Why it is wanted

The use cases are the ones where a program keeps a large value in a hold and folds small changes into it. A text editor's buffer, a spreadsheet's cell map, a drawing's list of strokes, a game's entity table, a form with forty fields. Each one is a hold whose event is a merge of small edits, and the fold is a function from the old value and an edit to the new value. Today that function is written with functional update at every level:

```prism
type Doc = record:
    lines: Nat ==> Text
    cursor: Cursor
    dirty: Bool

let Insert: fn Doc, Edit -> Doc = \|doc, e|
    Doc(
        lines = Set doc.lines, e.line, (Splice doc.lines[e.line], e.col, e.text),
        cursor = Cursor(line = e.line, col = e.col + Len e.text),
        dirty = True)
```

Every field has to be named even when two of them are unchanged, the nested `Splice` is hidden inside the `Set`, and adding a field to `Doc` touches every function that builds one. The same function as an update:

```prism
let Insert: fn Doc, Edit -> Doc = \|doc, e| update doc:
    &_.lines[e.line] <= Splice .lines[e.line], e.col, e.text
    &_.cursor <= Cursor(line = e.line, col = e.col + Len e.text)
    &_.dirty <= True
```

The goal this serves is the one about seeing what moves. The hold's definition still lists every event that can change the document; nothing about that changes. What changes is that the function the hold applies says what it changes and nothing else, so reading it is reading the diff, which is what you wanted to read.

The second reason is performance, and it is the same reason reuse is in the optimisations list above. A functional update that rebuilds a record of forty fields to change one copies forty words; a map insertion copies the map unless the map is persistent. An `update` block that knows which path it changed can, when the value's count is 1, change exactly those words in place. The feature and the optimisation are one design seen from two sides.

### The semantics as far as they go

Everything about references can be said without mutation, and should be, because that is what makes them safe to add to a language with no assignment. An `update` block is a function from a value to a value. A reference is a lens, a pair of a getter and a setter over the root value. A statement `&p <= v` is `root <- set_p root v`. A statement `&p += v` is `root <- set_p root (get_p root + v)`. A `let` inside the block reads the root as it is after the statements before it. The block's value is the root after the last statement. Under this reading an `update` block is a fold of lens applications over the root, every value is immutable, and the block could be elaborated into exactly the staircase of functional updates it replaces. That is the reference semantics, and the Haskell implementation runs it directly.

Lenses nest. `&_.lines[e.line]` is the lens for `lines` followed by the lens for key `e.line`. A user function `fn &Doc -> &Text` is a lens written by hand, and inside an `update` it is applied like any lens: `&(CurrentLine _) <= "..."`. The map indexing lens has the insertion rule from the aggregate types: setting `&m[k]` on a map inserts if the key is absent, and reading `*&m[k]` gives a `?V`. The array indexing lens is a plain index, and setting it out of range is the same error as reading out of range would be, a `?` on the read side and an abort on the write side, which is the checked-arithmetic discipline applied to indexing.

Then the borrowing rules, which exist for one reason: so that an implementation may mutate in place. If two live references could alias, an implementation that wrote through one would have to know whether the other would see it, and the only safe answer would be to copy. So, inside one block: a reference is live from its creation until its last use; two live references must be disjoint, and the compiler must be able to see that they are. Two different fields of a record are disjoint. Two different literal keys or indices are disjoint; two keys that are variables are not known to be, and the second reference is an error while the first is live. A reference obtained through a user lens function borrows the lens's whole argument for its life, because the compiler cannot see inside the function. These are Rust's rules for `&mut` reduced to a block with no loops but `for`, no conditionals that keep a reference across arms, and no escape, which is a small enough fragment that the check is a pass over the block and not a borrow checker.

Copy-on-write is the implementation side, and the thing the Rust implementation has to get right. When the block starts, the root value has some reference count. If it is 1, which it is when the caller passed the block its last reference, the block owns it and every `<=` through a path with count 1 at each level writes in place. If any level's count is greater than 1, that level is copied once, the copy is written, and the parent is pointed at the copy; later writes through the same level find a count of 1. This is `Arc::make_mut` applied along a path. The observable behaviour is identical to the functional reading because no one else can see the write, and the borrowing rules are what guarantee that "no one else" includes the block itself.

The hold case is the one the reuse entry above describes: one copy per occurrence per level touched, which is already far better than a functional update's copy per field, and the runtime hook that lets a hold hand over its value when its only reader this occurrence is its own update removes the last copy. Nothing about `update` depends on that hook existing.

### What it must not change

An `update` block is an expression with a value. No reference escapes it; a reference is not a value and has no type outside a block. Nothing about occurrences is involved: an `update` block that appears inside a lifted expression runs when that expression's node runs, once per occurrence, over the value it was given, and the block cannot see or touch a hold, an event or a dynamic. Widget blocks do not get mutation; the only state in a program is a hold and the only way to change one is its event. A program that compiles today compiles unchanged, because `&`, `read` and `update` are new syntax and no old program has them.

The erasure question is the one that is not yet answered and is why the item is research: a lens through a type variable, `fn &[a] -> &a`, needs to write into a value whose layout generic code does not know. Under the one-word representation, where generic code clones and drops by tag bits and a record points at its layout, the setter can write a word into a field slot without knowing its type, so erasure and `update` are compatible, but the proof is a working implementation and not this paragraph.

### Why it belongs in the Reflex implementation first

The Haskell implementation interprets the typed AST, and a lens in Haskell is a first-class value with exactly the getter-setter semantics above: two lenses nest the way two functions do, and reading through one, writing through one and mapping through one are the three operations an `update` block needs (the `lens` library is the implementation choice; the form of its API is from memory). Adding `update` to the Haskell evaluator is adding an elaboration case that turns a block into a fold of lens applications, and the library does the rest. That makes the experiment cheap: the syntax, the pronoun rules, the typing of `&`, the borrowing check and the `for` form can all be tried on real programs and tested through the conformance suite before the Rust implementation commits to copy-on-write along paths and the compiler to a borrow pass. If the semantics change during the experiment, nothing in Rust has to change back. Once the suite has `update` programs with golden transcripts, the Rust implementation has an oracle, and the differential test says when its in-place version is right.

### The fact it waits on

A program in the suite, or a library, whose fold functions are more plumbing than logic. The text editor is the likeliest first case, because its buffer is a map of lines and every keystroke is a nested update. Before that, functional update is adequate and the feature would be a guess about ergonomics.

### What the check can refuse

The borrowing check has a question of its own. The sketch rejects two references whose keys are variables, a reference held across the arms of an `if`, and a function taking two references. All three have a meaning in the functional reading of the block, since swapping `a[i]` and `a[j]` is a well-defined fold of two lens applications. The rules are there so that the in-place implementation is safe and so that the check stays one pass over the block. A static check for the cases it can prove, with a runtime comparison of keys for the cases it cannot, gives programs the meaning the functional reading gives them: when the two keys are equal, the statements run in sequence on a copy at that level, or the occurrence aborts the way an out-of-range write does. Which of the two is right depends on whether a program that swaps an element with itself should be a no-op or a mistake, and the text editor's buffer is the first place the question will come up.

## User-defined traits and modules

Two pieces of ordinary language work, with no bearing on the semantics of time, deferred for exactly that reason: they can be added when a program needs them without any law changing, and a design that lands them early is a design that spends its attention on the wrong thing while the laws are still being tested. What follows is what each would look like, so that when the need arrives the work is known.

### Traits

You have a `Money` type, or a `Vector`, and every sum in the program is spelled `Add a, b` where you would write `a + b`, because the numeric trait reaches only the built-in number types. Comparison is already there: a record is ordered by its fields in declaration order, so `a < b` on two `Money` values works today, with the derived order rather than one the type's author chose. The language has three built-in traits, equality, ordering and numeric, which the checker resolves at each use to the implementation for the operand type and passes as a hidden argument where a generic bound needs one. That mechanism is a trait system with no syntax for declaring a trait or adding a type to one. User-defined traits are the syntax.

```prism
trait Show a:
    Show: fn a -> Text

impl Show Vector:
    Show = \read _: "({.x}, {.y}, {.z})"

impl <a: Show> Show [a]:
    Show = \|xs| Concat [["[", (JoinText ", ", (for xs: Show _)), "]"]]
```

The elaborator treats a trait as a record of functions, an `impl` as a value of that record, and a bound `<A: Show>` on a generic function as a hidden record argument, resolved at each call site by the operand's type. That is dictionary passing, the same thing the three built-ins do, and it is why generic code stays erased under the one-word representation: a trait method on a type variable is a call through the dictionary, which is a plain record of closures, and the value itself needs no descriptor.

What traits do not do, and the exclusion is the design: no traits over the functors. There is no `Functor` or `Monad` trait, no user-defined block kind, no instance that says how `!` lifts. The sigils fix the set of functors and the lifting rule is the language's; a trait that could change how `$a + !b` elaborates would make the sentence "you can see what moves" false, because what moves would depend on an instance in scope. Traits are over plain types only, and a sigiled type (`!Int`) may not be an instance. Coherence is the simple rule: one `impl` per trait per type in a program, and overlap is an error at the point both are in scope, which the module system below makes a concrete place.

What traits buy, in order of how soon it will be wanted: `Show` for debugging and labels, `Default` for a record with many fields, a `Paint` trait in the GUI library so that a container can take a list of any types with a `Paint` instance, without a closed choice of what can be drawn, and the user-defined numeric types (a `Money`, a `Px`) that make `+` mean something for them without a suffix. The last is also the one place the deferral has a visible cost today: a program with a `Vector` type writes `Add a, b` where it would write `a + b`, which is where this section began.

### Modules

One file plus the prelude is the whole of the current module story. Anything larger than one file needs a way to split, and anything meant to be reused by another program needs a way to say what it exports.

```prism
module Ui.Layout

import Draw (Rect, Color)
import Ui.Focus as Focus

export @HBox, @VBox, Extent, Pos
```

A file is a module and names it on its first line. Imports bring in a module's exports, optionally renamed; exports list what is visible outside. Tags are already namespaced, `Ui.Button`, and a module's tag declarations live in its namespace, so `tag Button of Element` inside `module Ui` declares `Ui.Button`, which makes the tag namespaces and the module namespaces one thing. Instances are exported with their trait or their type, whichever the module defines, and the overlap check runs at import.

What the typed AST needs: qualified names. Every binding in the text form is already named; a module system makes the name `Gui.Layout.HBox` instead of `HBox`, and nothing else in the form changes. That is the test for whether modules are ordinary language work: if the typed-AST text form had to gain a construct, the deferral would be hiding a semantic question. It does not.

The thing deliberately left out: no module-level state and no module initialisation. A module is a set of definitions, a program is `@main`, and the first thing that happens is occurrence zero building it. A module that wanted to "run something at load" would be a widget being built by nothing, which the language does not have, and this is the same rule as "nothing happens because an element appears" applied to source structure.

### The fact it waits on

A program that does not fit in one file, or a library (the GUI library is the candidate) that a second program wants to import. Both will arrive during the conformance work, because the GUI library in Prism is a module the twelve programs share, and the moment it is shared it needs a name. Traits come when the second user-defined numeric type appears.

## A package system

A second repository wants the GUI library. Someone copies the directory. Three weeks later the two copies disagree on `Ui.Button`'s fields, a program that imports one of them is handed elements by a widget written against the other, and the first sign is a `Build` error in a tag the author never touched. A module system says how one program is split into files. A package system says how code written by someone else gets into a program, with a version, and how a program says which versions it is compatible with, so that the disagreement is a version mismatch the loader reports and not an afternoon. It comes after modules and is the last item here because it is the one with the least to do with the language and the most to do with everything around it.

### What a package is

A named set of modules with a version, a manifest, and its own conformance scripts:

```
name    = "gui"
version = "0.3.0"
prism   = ">=1.0"           # the specification version the package is written against
depends = { "draw" = "^0.2" }
```

Modules in a package live under its namespace, so a package `gui` provides `Gui.Layout`, `Gui.Focus`, and so on, and its tag declarations are in the `Gui` namespace. That makes the tag namespaces, the module namespaces and the package namespaces one thing, and it makes "who owns `Gui.Button`" a question with one answer. A World's components are packages too: the CLI World's tags are the `cli` package's declarations, and a program that imports `Cli` is declaring which World components it needs, which a World can check at load.

Two things a package may not contain, and the loader is what enforces both. No native code: a package is Prism source and the manifest and nothing else, so that the Rust and Haskell implementations read the same package and the differential test covers it. Native code lives in Worlds, which are host-language programs, and a package that needs something native declares a tag and a World provides it. And no build scripts: a package is read, not run, which is the "nothing happens because something exists" rule applied once more. Each implementation's loader rejects a package containing any file that is not Prism source or the manifest, before any of it is parsed, and both loaders run the same check from the same rule, so a package that one implementation accepts and the other refuses is a differential-test failure like any other. The package author is bound by the rule; the loader checks it; nothing downstream of the loader needs to.

### Versions and the conformance suite

A package's version is a claim about its exported signatures and its behaviour, and the second half of that claim is testable. A package ships its own test scripts, written in the conformance suite's script language against the Test World, and a dependent program's build runs them under the implementation it uses. A package that passes its own scripts under both implementations is a package the differential test has checked; that is a stronger guarantee than most ecosystems offer, and it costs nothing beyond what the suite already is.

Resolution is the ordinary thing: a lockfile with exact versions, semantic versioning on the manifest, one version of a package per program (no diamond with two versions, because tag declarations would collide and because the one-impl-per-trait-per-type rule has to hold program-wide). The compatibility question that is Prism's own is the `prism` field: the specification version. The typed-AST text form has a version, and a package compiled against one specification version has a dump that a later frontend can read or reject, which is the same mechanism incremental compilation's cache uses.

### What it must not change

The language, the typed AST beyond qualified names, the laws. A program with packages is a program with more modules.

### The fact it waits on

The second program that wants the GUI library, from a different repository. Before that, a module system and a path on disk are enough.
