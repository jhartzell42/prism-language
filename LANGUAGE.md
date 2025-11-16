# Prism Programming Language

The Prism Programming Language is a strongly typed programming language for
writing applications in a Functional Reactive paradigm. It is integrated with
the Prism Runtime, a polyglot framework for writing FRP components.

This document is intended for an audience familiar with Haskell, FRP, and who
are conversant in programming language design. It is a pitch for a programming
language, and not a tutorial for one. Tutorials are for future work.

## Status

None of the programming language itself is implemented yet. Major parts of the
runtime are.

## Motivation and Vision

The first question any designer of a programming language is asked is always the
same: Why? Or the slightly expanded version: Don't we already have enough
programming languages? To which my blanket answer is: No. Different programming
situations benefit from different programming languages, and the existing
ones are frankly not good enough to build the tech world of the present,
let alone the future.

But I want my programming language to succeed, and if and when it has fans I
want them to have talking points, so in this section I'll pitch the "why." This
will also help people stay on track, and avoid having them add features that
will ruin everything.

I have had an excellent experience with the Reflex FRP framework in Haskell,
both professionally as an employee at Obsidian and in the one small hobby
project I've done in it. I truly believe that FRP is the way of the future for
writing GUIs, and GUIs should be written in such a framework. Haskell's features
are deeply built into the framework, and it's hard to imagine FRP semantics
working in a programming language without those features.

That said, there are certain things I've found frustrating about FRP in Haskell.
I've found myself writing `dFoo`, `eFoo`, `deFoo` to disambiguate variables that
concern themselves with the same thing, which is sort of a Haskell version of
Hungarian Notation. Ryan officially recommends against doing it, but not doing
it is worse, because all of these monads are recursive, which means you can't
use shadowing consistently as an alternative.

This is just the most superficial frustration (and the most visible in Prism).
There are many smaller issues where it seemed like the framework's semantics
of FRP were working at cross-purposes with the general semantics of Haskell.

Additionally, being tied to Haskell means that the success of FRP is tied to the
success of Haskell. While I enjoy Haskell deeply, especially at a conceptual
level, the community is commiting to "avoid\[ing\] success at all costs." While
I understand that this is meant as a tongue-in-cheek inside joke, I think
they've succeeded at the more salient interpretation of this goal.

So, for FRP to succeed, it needs a different programming language than Haskell,
one that seeks success at some cost. But why its own programming language?

Well, FRP's semantics were designed for pure functional semantics. Pure
functional programming is always pitched as avoiding mutation and side effects.
In practice, Haskell is a language for creating custom silos with
different models of mutation and side effects, whether `IO` or
`State`/`MonadState`---or Reflex FRP/`MonadWidget` along with its many other
functors and monads. FRP is one such custom silo---it is a policy on how
to manage mutation.

Only Haskell has this level of flexibility, this notion of user control of
mutation semantics. Arguably, that's the entire point of Haskell. Other
programming languages choose a mutation policy and stick to it. Most just are
imperative and freely allow mutation, perhaps with constraints like `const` in
C++. Rust goes a different route, and forbids mutable aliasing while allowing
both mutation and aliasing.

If how to handle mutation and IO is a policy that we set at the programming
language level---which it seems that it is---then what does this mean for FRP,
which is a policy for mutation and IO that doesn't play well with others? Well,
outside of the one programming language that made flexible and programmable
mutation policy one of its core features, all of this implies that FRP will be
fundamentally at odds with every other existing programming language.

For these reasons, I have concluded the only way to have FRP outside of Haskell
(or a Haskell derivative or strict dialect) would be to design a new programming
language where FRP is that programming language's mutation (and IO) policy.

And that is where the Prism Language comes in. It is not a systems programming
language. It is an applications programming language. I anticipate that
low-level or performance-sensitive components for the Prism Runtime will have to
be written in other programming languages---for which programmers will pay the
price of having a programming language ill-matched to the FRP mutation semantics.
Given that those low-level components will likely involve translating from
FRP mutation and IO semantics to other semantics, this is a cost I am more than
willing to pay.

Since we are trying to use FRP semantics as our sole mechanism for mutation and
IO, the semantics of Prism will be tightly constrained. The syntax I'm designing
has a misleading scripting-language vibe: significant whitespace, comments with
`#`, and the `$` sigil for the most common type of mutable state. It also will
support convenient automatic coercions based on what I judge that people will
want to avoid extra typing and thinking. But that doesn't mean it's going to be
a Perl or Javascript-style anarchy.

I imagine that Rust and Haskell will be the other preferred languages for
interacting with the Prism Runtime---and probably just Rust, due to Haskell's
professed aversion to success. Maybe we can get Swift in there too, if the Swift
people want.

I know that learning a new programming language takes work. My goal is to spread
FRP semantics, and learning FRP semantics will take work in any case. Whether
embedded in another programming language or not, FRP is a different programming
paradigm than any other, distinct from the procedural paradigm, imperative OOP,
the pure functional paradigm, and Rust's hybrid paradigm blending systems and
functional programming.  I would argue that learning a new programming paradigm
is far more work than learning one individual programming language within that
paradigm. As evidence to that, see all the complaints about Rust not supporting
inheritance from people who know several OOP programming languages.

Given that, I think it's better that people learn it a new programming language.
They will be less tempted to use other paradigms for their mutation---which, if
Prism were embedded in another language, they would be able to do.  Learners
would work around FRP, create their own worse versions of other GUI frameworks,
and then blame FRP for its awkwardness and their terrible results.

Humans have to learn. I truly believe that FRP will make it worth it for them.
I also truly believe, in the case of FRP, that well-designed, finely tailored
syntax will make it easier for them. I believe that with rare exceptions, anyone
can learn any skill to proficiency, given enough time and appropriate motivation.

But I will also try to make it as easy for folks as possible. I believe in success
at some cost. But also, I believe that success sometimes comes from imitating
other programming languages superficially, and sometimes comes from just making
a system intuitive but distinctive. Prism focuses on the latter philosophy.

## Feature Selection

I believe strongly that a programming language's usefulness and longevity
depends as much on what features it rejects as what features it accepts.
The primary thing I'm trying to avoid in Prism is *cheating*, creating
mutation and IO outside of the FRP domain.

I am not an acolyte of the OOP religion. I'm also not a fundamentalist in
opposition to it. Some of these semantics will feel like OOP to some
people---and if that's what it takes to get you to "get" it, by all means think
of them that way!

But that doesn't mean I'll add your favorite OOP feature. I won't reject it out
of hand just because it comes from OOP. Even if you pitch me a feature called
"inheritance," I'll consider the actual concrete proposal on its merits.

## Syntax-Forward Tour

I'm going to tour based on the syntax, while explaining the semantics of said
syntax. The syntax takes its inspiration from Perl, Python, and (obviously)
Haskell. From each of these languages, it takes some of the most polarizing
and controversial features: sigils from Perl, significant whitespace
from Python, significant capitalization from Haskell.

This is because I genuinely believe in these syntax features. I know
Perl gets a lot of flack---and deservedly so---for being a write-only
programming language. I don't think that's because of the syntactic
elements that I'm borrowing from it. I think that's because it went
too hard in the TMTOWTDI direction, and made the syntax too heavy.

I don't think that's what I'm doing with Prism. The syntax is doing a lot of
lifting, but that's because the computation system we're trying to model has a
lot of nuance.

The syntax is designed to emphasize and reify the semantics. Distinctions
in how a value mutates over time or interacts with the outside world
should be visible in how we write the value. Distinctions in what
an operation is doing should be visible in how we write the operation.

### Identifiers

A la Perl, identifiers use sigils to identify their functor (i.e. widget, event,
dynamic, etc):

* **`!`** for **events** which represent values that "fire" or not at any
given instant in time (known as an occurrence). Occurrences are created
as-needed to propagate events, based on when, in the outside world,
some event occurs---thus the name.
* **`@`** for **widgets** which represent stable components of logic, tying
together events and dynamics with each other and the outside world. The
archetypical use case of widgets is to implement a component of a GUI, but they
can also be used for network connections, file read/write, and other IO. All IO
is mediated through widgets.
* **`$`** for **dynamics**, which represent a value that changes over time.
Dynamics transition from one value to another during an occurrence. Both
the old value and the new value are in play during an occurrence, depending
on the semantics of the operator.
* **`?`** for **optional** values. If an optional value is absent, it is `Nil`, which
is a type. Non-optional values are always present when you expect them.
* Bare for **pure values**. Pure values can contain all these other types
as fields or elements of arrays or other such, but they don't have
one functor wrapping all the values in any way. It's kind of hard
to rigorously define their mutation semantics, especially given that
we have an update monad. I think a rigorous definition will evolve over
time.
* `&` for **references**, which may only actually exist within the scope of an
`update` context. They represent lenses applied to the currently updated value.
Functions can also exist outside an update context that map from reference to
reference, but each such function must have only 1 reference in the parameters.
There's no way to convey references outside of an `update` block (though you can
always have reference functions aka lenses) If you use the `*` sigil instead,
you dereference it for reading only, and get the current regular value.
* `UpperCamelCase` for **functions** and **types**. This isn't technically a
sigil, but it serves as one. It combines with other sigils based on what the
function *returns*, such that `@Button` is a function that returns a widget. If
a function is instead *contained*, it gets demoted to a pure value "function
reference." There aren't many use cases for a function that takes no arguments,
so technically those aren't allowed, and just take a unit value instead.
If you're writing a widget function that takes no arguments, you almost certainly
just want to write a widget.

Rejected sigils:
* For **arrays**/**collections** 

They can compose, leftmost outermost. So, `?$foo` means what we would write in
Rust as `foo: Option<Dynamic<T>>`.

Functions identifiers have an uppercase letter, and then the sigil indicates the
functor of their return value. Types also start with an uppercase letter.
Function values inside functors (a `Dynamic<Fn>`) are treated as pure values.

Lowercase identifiers are expected to be in `snake_case` and uppercase
identifiers in `UpperCamelCase`. To do otherwise is an error.

I considered trying to get people to call them "funny characters" in an homage
to Perl, but I don't think I can make it a thing. If anyone disagrees, though,
let me know, and if there's enough interest I will go along with it. I imagine
that rather than discussing what functor a value is in, people might say what
sigil it's in.

On a more earnest note, behaviors aren't exposed at a Prism Language level.
Like in Reflex, a `$dynamic` consists internally of a behavior and an event.
Unlike in Reflex, there's no way to access the behavior independently. Any
tagging or switching primitives implicitly access the behavior---if you want
promptness, implement it yourself, at the risk of cycles. This is to say,
during event propagation, dynamics consistently are considered to have
their *old* values. During dynamic construction, of course, old and
new values must be considered.

### Blocks

There are a number of different kinds of blocks for different kinds of flow
control. Layout is Python-style: blocks are introduced with `:`. Statements
are newline separated and indented. These are based on Haskell's monads,
especially monads in a Reflex context, but unlike in Haskell, we will not
be allowing custom polymorphic monad operators (at least for now). Some
built-in operations may support multiple monads, but that doesn't extend
into a full Haskell-style effects system.

#### Widget Blocks

Can define widgets (a user interface or IO component element with no inputs or configuration), or
functions that return widgets (a component with inputs or configuration). Here's an example
of defining a widget at the top level:

```prism
@counter:
    !increment <- @Button $"Increment"
    !decrement <- @Button $"Decrement"
    let !delta = -1 on !decrement || +1 on !increment 
    $count <- 0 <= $count + !delta
    @Label $"Count: $count"
```

Note: `@Button` is a widget function. It takes a dynamic label and returns a
click event.  `@CustomButton` would take a bundle of dynamic configuration and
returns a bundle of possible events as a pure value.

To reiterate: `@Button` is a function because it has arguments, and it takes
those arguments and returns a widget. A widget without arguments is just a
value.

`@counter:` here makes a top-level widget definition. We could have also
written:

```prism
let @counter = @widget:
```

In general, `@widget` introduces an anonymous widget block. Any other identifier
`@foo:` is equivalent to `let @foo = @widget:`.

##### Statement Types

Each block type has its own statement types. In Widget blocks:

* A widget is a statement on its own. Binds the widget into the block,
ignoring its return value.
* `let foo = expression` in any sigil is a computation that doesn't bind, for
any sigils, including widget. Widgets can be bound multiple times.  The
resulting widget nodes do not alias.
* `foo <- @expression` binds and puts the outputs of the widget in `foo`
* `$foo <- initial <= !expression` holds an event into a dynamic. This can
only be done in a widget context. Someone has to own the darn thing.
* Recursive bindings, as you can see, are just allowed. Cycles in
pure values are not allowed. Prism is eager in its computational model, at least
in theory, with the obvious exception of functions.  (For a finite set of
possible arguments, you could make functions entirely eager, but no one does.
You can also memoize, which some folks do.)
* The last line of a widget block computes the return value. If it is
a widget, the return value of the parent widget is the return value of
the widget it invokes. If you want to return a pure value, use the
`@` operator, which as an operator lifts a pure value to a widget---this
is the equivalent of Haskell's `return` or `lift`, and every functor
supports it where it makes sense.

#### Reader Blocks (from Haskell `MonadReader`)

For complicated functions.

```prism
type Vector = record:
    x: Float
    y: Float
    z: Float

let Magnitude: fn Vector -> Float = \read _: # This `_` is for the lambda
    let x = .x
    let y = _.y # This `_` is for the `read` block.
    let z = .z
    Sqrt(x * x + y * y + z * z)
```

The `read` block allows us to introduce `_`, and its friends, `$_` ,`@_`, and
`!_` for when it has those sigils. This is our pronoun in Prism. It means "the
thing we're currently talking about." This concept is borrowed from Perl, and it's
used in other contexts as well.

A `read` block lets you examine a value, introduced with the keyword `read` (`\`
is for the lambda).  Within the read block, `_` represents the argument. Also,
for single-argument lambdas, `_` represents the argument by default, but `read`
lets you create a block.

In contexts where `_` is defined, you can also do field-accesses without
specifying which value you're accessing a field from, and you do this by `.foo`
to access the field `foo`. Similarly, indexing an array or a map can be done
with just `[index]`.

In a `read` block, only `let`s are allowed except for one statement with a
value, which computes the return value, to which the `read` expression
evaluates.

The Rust/Haskell meaning of `_` is covered by the `ignore` keyword. You can
also specify a variable name, by writing `read foo <- v` or similar.

#### Update Blocks (from Haskell `MonadState`/Rust mutable references)

Update blocks allow mutation.

```prism
let Update: fn (Vector, Vector) -> Vector = \|a, b| update a:
    &_.x += b.x      # Update addition syntax
    &_.y <= .y + b.y # Alternate syntax
    .z += b.z       # Still have implicit `_`
```

Within an update block, you can do mutations on `&` values. Mutations happen in
sequence. A statement can be a `let`, or it can be the result of a mutation
operator. Mutation operators include `<=` for setting values, as well as `+=`
and friends.

An `update` block is an expression that evaluates to the updated value. The
original value isn't modified; values have copy-on-write semantics.

Functions that return the same type as their first argument are automatically
updates, with one less argument when used in an update context.  Updates can be
used as an imperative statement within an update context.  Functions from
references to references (lenses) can also be applied within an update context.

References have to follow Rust's borrowing rules for mutable references.
If you have multiple fields, you can have multiple sub-references, or
multiple array indexes. Any generically-written functions mutably borrow
their entire argument for the duration of the returned reference.

#### Loop Blocks (vaguely like Haskell list monad)

For looping, we have `for`. `for` enacts the statements inside the blocks
multiple times in the context of the block it is included in, for each element
of the array it's investigating. `for` outside a block context creates a `read`
block. The return value/last line of `for` is applied to a loop. If there is no
return value, it just "does" the statements in the block.

```prism
let AddByElement<A: Add>: fn (Array A, Array A) -> Array A = \|a, b| update a <- a:
    for 0..a.len:
        &a[_] += b[_]
```

In reader contexts, `for` can read the array. In mutation contexts, if the array is
part of the thing we're mutating, `for` can mutate the array. `ix` is a keyword
that gives you the current count. If you nest loops, the inner indexes are automatically
`jx` and `kx`. If you nest a loop more than that, you're on your own for indexes. Write
a helper function.

```prism
let AddByElement<A: Add>: fn (Array A, Array A) -> Array A = \|a, b| update a:
    for &_:                 # `_` means `a`
        &_ += b[ix]      # `_` means `a[index]`
```

`for` can loop over dynamics, giving dynamic values of individual dynamics.  In
a widget context, this is useful to add a widget per element. When the dynamic
changes, widget nodes are torn down and built up, but shifts and changes of
individual elements preserve the existing widget node without rebuilding,
changing what dynamic is offered:

```prism
# This would be defined by the checkbox type, maybe scoped.
type IsChecked = choice Checked | UnChecked

type ChecklistItem = record:
    label: String
    is_checked: IsChecked

type ChecklistUpdate = record:
    ix: Nat               # ix is allowed as a field name for this reason
    checked: IsChecked

@ChecklistList: fn $[ChecklistItem] -> @![ChecklistUpdate] = \@widget:
    !events <- for $_:       # `$_` is the single function argument
        !checked <- checkbox $.checked
        @ ChecklistUpdate ix, !checked # Records automatically create functions
    @ !events
```

When `for` is in a widget context, it creates a widget that returns an array of
all the outputs.

For all contexts, `for` will consolidate return values. For arrays of events, it
then converts into an event that has an array of all the actual events that are
firing in a given occurrence. For widgets, it consolidates them into a single
widget. For dynamics, it converts to a dynamic of arrays rather than an array of
dynamics. This is done recursively to shove the array as deeply into the value
as is possible with these automatic conversions.

#### Dyn Blocks

This is for within widget blocks. It takes a dynamic, and rerenders the widget
whenever the dynamic changes. It outputs an event with the output of that
widget, which fires when rendering is complete, including the first time.

Imagine we have a widget function `@Button` that takes static
text as its label. We want to convert it into a button that takes
dynamic text, and passes the text through when someone clicks on it.
We can do that with `dyn`:

```prism
@DynLabelButton: fn $Text -> !Text = \@widget:
    !!rendered <- dyn $_:
        !click <- @Button _
        @ _ on !click
    @ !Switch !!rendered
```

Here, every time our input text changes, we recreate the button.
The widget in which we recreate the button annotates the button's
output event with the same text. Within the `dyn` block, it's a pure
value, as within the context of that widget, it will never change.
When it does change, a new widget will be made.

You can still access external `$` values from outside of the `dyn`
block. They stay dynamic because changes to them don't trigger
re-rendering. Only ones listed in the top of the `dyn` block
trigger re-rendering.

`!Switch` takes `!!` and makes it `!`. It starts out as subscribed to nothing.
When the outer event fires, it subscribes to the inner event. Importantly for
implementation, it isn't prompt. That subscription only takes effect on the next
occurrence. Fortunately, that's sufficient for basically all use cases.

You may also write: `dyn pattern <- $_`.

### Expressions

#### Lifting

All operations are automatically lifted. `1 + !event` returns a
new event where the value inside is one more than the previous
event. Similarly, `1 + $dynamic` returns a new dynamic.
Only one event is supported per operation.

Operations can include dynamics and a single event.

#### Function Calls

No `()` are required for function calls. They just list their arguments,
separated by commas. If this causes precedence problems, you may use
parentheses, either for the arguments or around the function call.
It might be easier to use `let`-bindings though. Short statements are
encouraged.

Function calls resolve to the output of the function. If it's a widget,
you can bind it in a widget, or use `let =` to store the widget and not
actually insert the component into your application yet.

You can also write the first argument preceded by `.` to use method-call
like syntax. This is purely a syntactic option---there is no distinction
between functions and methods.

#### Lambdas

Lambdas are introduced with `\`. By default they take one argument, `_` (or `$_`
or other sigils depending on type, as can be detected by type inference or the
presence of `$_` in the body; there can only be one active `_` at a time).

We use `||` to name arguments or supply multiple arguments. One of them can
still be `_` or `$_`, just named.

If we want to ignore an argument, we can use the `ignore` keyword, which
is a pattern. Patterns can deconstruct tuples and other types.

#### Operators

##### Arithmetic Operators

Standard arithmetic operations are supported. The numeric types must match up
exactly. They are implemented polymorphically through traits. The `+=` operators
are implemented based on the `+` operators, for all the `X=`.

We have a `Nat` type for unsigned and an `Int` type for signed. `Nat` can be
converted to `Int` via unary `+` (or `-` if you want a negative integer).  `Int`
can be converted to `?Nat` via unary `+?`, or to `Nat` via `Abs`.

Division results in a `?`, whether integer or floating-point.

##### Event Operators

Technically, `on` just returns its left-hand side. Since it's used
with a pure value or dynamic on the left, automatic lifting gives it
its semantics.

`|` fires whenever either of its inputs fire. If both fire in the same
occurrence, it returns the left side. For many, you can use `!Leftmost`, which
takes an array.

`!Switch` flattens from `!!` to `!`, as discussed above.

`!never` is all types and never fires. There's just one. There is no "always."
If the occurrence has nothing to do with you, you don't get to know about it.

##### Dynamic Operators

Hold is the most important dynamic operator. It's easy to implement `fold` by hand
using hold, by referencing the output dynamic in its expression, as shown
in the counter example, repeated here for a refresher:

```prism
$count <- 0 <= $count + !delta
```

`<=` with a dynamic on the right and a pure value of matching type on the left
simply returns a widget which enacts the hold, and which can be bound.  The `<-`
is then just our old-fashioned bind operator. You could also store the widget
and bind it multiple times, but I'm not sure why you would.

`$` as a unary operator lifts values into constant dynamics.

##### Widgets

`@` lifts to widget context. `<-` binds within a widget block, which
is a statement structure not an expression operator.

### Types

All of these types either express a policy with regard to values
changing over time (events and dynamics) or represent values that
once created, do not change outside of the context of an `update:`
block.

Even within an `update:` block, they have copy-on-write implementation.  The
upshot is, the input to the `update` block stays the same, and the `update`
block outputs the updated values. Any linear resources are encapsulated behind
event sources, and all events, dynamics, and primitive values are cheaply
aliasable or cloneable---a distinction that is invisible to the user, for
whom they are all just "values."

All types are eager except for function types.

#### Primitives

* **`Nat`** for naturals. These can index arrays. `usize` in Rust. Overflow is panic.
* **`Int`** for integers. They can be negative. `isize` in Rust. Overflow or underflow is panic.
* **`Float`** for floating point. `f64` in Rust. Smaller floats might be provided if needed.
* **`Text`** for string types. `String` in Rust.
* **`Char`** for individual characters as unicode codepoints. `char` in Rust.

#### Sigils

Event, dynamic, and widget all form functors, and create a new type
when applied to a type. Just like `@foo` is a widget variable, `@Foo` is
a widget type (or a function returning a widget)

#### Aggregates

Prism has two built-in aggregate types: arrays and tuples.

**Arrays** are homogeneous, and contain all elements of the same type.
They are written `[Int]` or `[!@Nat]` or similar. The Rust equivalent is
`Arc<[T]>` or similar. They are immutable, though in an `update` context
mutations are implemented as efficiently as feasible. The values
are comma-separated in `[]`, and an empty list is `[]` or `empty`. You
can also have values in an `array:` block which just has expressions in it
that merge to form an array.
**Maps** as a type are written as e.g. `Text==>Int` for a map from strings
to integers. The syntax is introduced with `map:` for multiple lines, or
`{}` for one line. They index by entry. `some_map[x]` means the actual
value in a normal context, or a setter/inserter in an lvalue context
like an `update:` block---that is, it converts a `Text==>Int` to an `Int`, or
a `&Text==>Int` to an `&Int` that if/when you set it, causes an insert to
take place.

```prism
let some_map = map:
    "foo": 3
    "bar": 4
let other_map = {"foo": 3, "bar": 4}
```

**Tuples** are heterogeneous, and contain multiple values of different types,
with comma separation. The types are also written with commas, a la Rust.
A tuple with one value is written with a trailing comma. A tuple with no
values is a warning unless there's a polymorphic reason, and it's equivalent
to the built-in unit type `unit`.

#### References

References are written with `&`. They can only be used as the LHS of an `update`
operation. They are mutable references. They turn into the values they refer
to in all context besides the LHS of an `update` operation. They all borrow
from a value that's been the main value being updated.

This is the context that needs the most work. I'm still figuring out
what these things mean.

#### Units

Units are custom types that only have one possible value.

You can write them as:

```prism
type Unit = unit
type True = true
type False = false
```

The value (lowercase) must match the type name (uppercase). The value
is a global of the uppercase type, and represents the only possible value.

#### Newtypes

If you write:

```prism
type Foo = Foo Int
```

Then `Foo` is both a type and a function that converts `Int` to that type.  To
convert back, you can call `.value`. Newtypes plus tuples can be used to make
lightweight record types, but you should prefer records.

#### Records

Records look like this:

```prism
type CheckboxItem = record:
    label: Text
    checked: Bool
```

`CheckedBoxItem` is now also a function that constructs it. You can invoke it
either like a normal function, or as a block with labeled values.
Accessing the fields is done through `.`. If a field has a sigil, you
might have to write `.!`, because fields with sigils must have names that
start with sigils.

#### Choices

The equivalent to `enum`s in Rust is choices. You list the types the
`enum` might contain:

```prism
type Bool = choice True | False
```

The constructors are named after the types. You can also use
whitespace-significant syntax:

```prism
type Bool = choice:
    True
    False
```

If you want to have multiple options with the same inner type,
you can declare new types on the spot.

```prism
type DifferentStrings = choice:
    Foo String
    Bar String
```

So, `Foo` and `Bar` are also declared as newtypes in this syntax.

### Patterns

`match` chooses between the options of a choice, resolving to the content
of each, in the monad the `match` is embedded in, or in a pure state only
containing one expression.

```prism
match some_bool
    True: 3
    False: 4
```

`if` is a specialized match for `Bool` values:

```prism
if some_bool: 3 # This can be a block in a context that allows them.
else: 4
```

Patterns can also handle tuples in the obvious way. Newtypes need both the type
name and the value to match, even if not in a `choice` context. In a `choice`
that doesn't use a newtype, you can use the type name or not to distinguish
which variant is being used, as long as it's unambiguous.

For records, you use the type name and match the fields:

```prism
type TodoItem = record:
    label: Text
    checked: Bool

let ?Text = match todo_item:
    TodoItem(label, checked=True): Some label
    TodoItem(label=ignore, ..): Nil # Can use either syntax to ignore
```

As we discussed before, `ignore` is a special keyword pattern that matches
anything. `..` can be used to avoid specifying all the fields for a record.

## Semantics and Implementation

When an application runs, before any actual IO happens, it constructs
a tree of widget nodes. Widget nodes are the output of a widget, and
binding a widget makes it construct a node owned by its parent node.

An application, at the top level, manifests as a widget node. This is owned by
the backend in the current runtime design, which owns the main loop and runs
occurrences for external events. If these events have subscriptions in
the app, then they'll propagate, as the events propagate through the
event graph, update the behavior component of dynamics as they go,

Only widget nodes support event cycles, dynamic cycles, and holds.
This is why these things can't be done outside of a widget block,
because it needs to populate the node with the data structures that
support these operations.

## Testing

There's a test function and an additional sigil for testing: `%`. `%` represents
an identifier reified as a string, or a path of identifiers. It represents local
variables within a live widget, or within within a live widget, etc.

The `Test` function
takes a widget, and runs it in a test runtime. The test runtime doesn't actually
act---it's stubbed out. Instead, you give it a list of events to trigger and
validations to make. You can declare tests with `test`, where the body is a call
to `Test`, and they will be detected by Prism's test harness. You can also
call `Test` on your own, or make your own functions that return a `TestResult`.

```
type TestResult = TestPassed | TestFailed

@counter:
    !increment <- @Button $"Increment"
    !decrement <- @Button $"Decrement"
    let !delta = -1 on !decrement || +1 on !increment 
    $count <- 0 <= $count + !delta
    @Label <- $"Count: $count"

test counter: Test @counter: array:
    Inject %!increment
    Inject %!increment
    Assert %$count, 2
    Inject %!decrement
    Inject %!decrement
    Assert %$count, 0
```