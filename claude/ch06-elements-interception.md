# Chapter 6 — Elements and interception

An element is the only thing a Prism program can say to the outside, and the only thing the outside can address when it speaks back. This chapter defines that one channel in full: what an element is and how its tag declares what it does; the rule that nothing an element asks for can be answered in the occurrence it asked; the two identities a slot has and why they are different; the element list as an ordinary dynamic that an interceptor or a World reads; and `intercept`, the one primitive that lets a Prism widget stand where the World stands, so that a GUI toolkit, a sandbox, a cache, a mock, a tracer and a program that hosts other programs are all written in the language rather than under it. The chapter is long because everything about the boundary is here, and because the boundary is where every other framework keeps a second set of rules.

## What an element is

You have shipped the component that fetched on mount. It was fine until the list it lived in re-rendered, and then it fetched again, and the second response landed after the third mount's, and the row showed a stale avatar for a user who had been scrolled away. You fixed it with a cancellation token in a ref, and the fix had a race in it that the test suite could not reach because the test suite needed a mock server before the first render just to get past `componentDidMount`. Somewhere in that afternoon you wrote the sentence "the network layer has its own lifecycle," and that sentence is the problem this chapter removes.

Three promises meet at the boundary and pull in different directions.

The first is that IO must be impossible from inside the language. A program may not write a byte, open a socket or read a clock; the promise that a hold's definition lists everything that can change it is only true if nothing can reach around it. So whatever a program says to the outside has to be data, with no function and no object in it, and whatever the outside says back has to arrive the way every other value arrives, as an event firing or a dynamic changing in an occurrence.

The second is that the toolkit, the sandbox and the mock must be writable in Prism. Layout, focus, text editing, confinement, caching, tracing: each is something a program wants between itself and the world, and if the language cannot express "stand between a child and the world", every one of them is native code, with native code's rules. So the boundary has to be a thing a Prism program can stand on either side of, and it has to look the same from below whether a World or a widget is above it.

The third is that a widget which asks for state must get a type that tells the truth. The file that has not been read is `Unavailable`, as a case of the type, and the `match` that draws it has an arm for that. But a dynamic has a value at every occurrence, so a widget that asks for the contents of a file as a dynamic must get a value in the occurrence it asks. That forces a question most systems never put to themselves: when does outside state exist for a program? The answer here, in the occurrence the slot is created, is the only one under which a widget built in the middle of an occurrence is consistent with everything else built then, and the second half of this chapter is the reasoning for it.

An element is what satisfies all three. It is a plain value of the extensible variant `Element`: a tag and some fields, `Ui.Button { label: "Increment" }`, `Fs.File { path: "/etc/motd" }`, nothing else. A widget puts one in its element list by calling `@Element`, which creates a *slot*: a position in the widget tree where that element sits for as long as the widget's node lives. Whoever is outside, a World in the host language or an interceptor in Prism, reads the list, does whatever the element means to it, and sends back responses addressed to the slot and state keyed by the element's value. That is the entire protocol. There is no handle, no callback, no promise, no context object; there is a list of plain data going out and a map of plain data coming in, once per occurrence.

The consequence that takes getting used to is that an element does nothing. A `Ui.Button` is not a button; it is a request to whoever is outside to show one and to say when it is clicked. An `Fs.File` is not an open file; it is a declaration that this widget may ask for reads and writes of that path and wants the answers. Whether anything happens is up to the outside, and the program's only way to find out is a response arriving. This is what makes the same program run under a test World with no screen and no disk: the program never did anything, it only ever said what it wanted, and the World that says "done" without doing is as conforming as the one that does.

## Tags

A tag is the declaration of one kind of element: its name, its fields, and which of three things it does. Every element in the language is built from a declared tag, and the declaration is where the types that cross the boundary are fixed.

```prism
type Element = choice ..

tag Ui.Label of Element:
    text: Text

tag Ui.Button of Element:
    label: Text
    responds: Click

tag Fs.File of Element:
    path: Text
    requests: !FsRequest
    responds: FsResponse

tag Fs.Contents of Element:
    path: Text
    state: Contents

tag Clock of Element:
    state: Instant
```

`type Element = choice ..` declares an extensible variant: a choice whose alternatives are added by `tag` declarations across modules rather than listed in one place. `Ui.Label` and `Fs.File` are constructors of it, namespaced, and a value built from one is a plain value with structural equality like any record. The constructor is called the way a record is, positionally or with named fields: `Ui.Button(label="OK")`, `Fs.File(path=p, requests=!reqs)`.

The three parts a tag may declare, in the order a reader meets them:

- **Fields** are plain data, fixed for the life of the slot. `label: Text`, `path: Text`, `period: Int`. A field's type is concrete; a tag may not mention a type variable, so generic code cannot build an element except at a type it knows, because there is no generic tag to build. Fields are what the outside reads to know what the element is.
- `requests: !R` declares an event field. The program supplies the event when it creates the element, and its firings go outward: after the occurrence in which the event fired, the outside sees the values fired, in order. A request event fires at most once per occurrence like every event, so a request type that wants to say two things at once is an array, `requests: ![ConnRequest]`.
- `responds: R` declares that the element returns an event `!R`. Its firings come from outside, addressed to this slot by path, in occurrences after the one in which the slot was created.
- `state: S` declares that the element returns a dynamic `$S`, supplied from outside, with a value in the occurrence the slot is created and a change whenever the outside pushes one. State is keyed by the element's value, not its path, which is the subject of the section on identity.

A tag declares any subset, including none. The subsets that occur are the kinds of element you have met under other names elsewhere, and they are combinations of three parts, not five different mechanisms:

| requests | responds | state | what it is | examples |
| --- | --- | --- | --- | --- |
| | | | presence: the element being in the list is the whole message | `Ui.Label`, `Gui.Scene`, `Gui.Focusable` |
| ✓ | | | sink: firings go out, nothing comes back | `Process.Exit`, `Log.Line` |
| | ✓ | | stream: things arrive for as long as the slot lives | `Cli.Stdin`, `Net.Listener`, `Time.Every`, `Ui.Button`, `Gui.Window` |
| ✓ | ✓ | | request: one firing in, some number of answers later | `Fs.File`, `Net.Http`, `Time.After`, `Cli.Stdout`, `Defer` |
| | | ✓ | state: a value that is simply true, pushed when it changes | `Clock`, `Process.Args`, `Fs.Contents` |
| | ✓ | ✓ | a watched thing that also talks | a feed that is up or down and delivers messages: `Net.Socket` without the send |
| ✓ | | ✓ | a thing you drive and observe | `Process.Child` without its output |
| ✓ | ✓ | ✓ | the full conversation | `Net.Socket`, `Process.Child` |

A button is a stream in this table, and that is right: a click is something that arrives, and the button asked for nothing. A request element is a stream with an input. A state element is neither; it is the outside's view of a fact, offered as a dynamic. Nothing in the language treats the rows differently except by what `@Element` returns.

The alternative worth a paragraph is one part per tag, so that a socket is three elements: a `Net.Send` sink, a `Net.Receive` stream and a `Net.Link` state, each with its own slot. It is cleaner in the declaration and it costs a path per part. A program then has three slots to keep together, a World has three slots to correlate into one connection, and an interceptor that wants to forward a socket has to forward three things that are only a socket if they travel together. A tag with three parts says in one place that these three things are one conversation, and the slot has one path for its responses and one value for its state. The cost is a declaration syntax with three optional lines in it, and that is cheap.

### A dozen declarations

Here are the tags the rest of this chapter uses, each with the one sentence that says why its parts are what they are. These are the GUI, filesystem, network, time and process vocabularies a World is assumed to accept; a World that accepts more declares more, and a World that accepts fewer answers `Unsupported` to the rest.

```prism
type Click = click
type Tick = tick
type Instant = Instant Int                   # milliseconds since the World's epoch

tag Ui.Label of Element:                     # presence: a draw command
    text: Text

tag Ui.Button of Element:                    # stream: clicks arrive
    label: Text
    responds: Click

tag Gui.Window of Element:                   # stream: pointer, keys and resizes arrive
    title: Text
    size: Extent                             # the size the program asks for
    responds: WindowInput

tag Gui.Scene of Element:                    # presence: the picture, as data
    window: Text
    draw: [Draw]

tag Gui.Focusable of Element:                # presence: the name of the region that has focus
    name: Text
```

A label asks for nothing and hears nothing; it is on the screen because it is in the list and off it because it is not. A button hears clicks. The three `Gui` tags are three of the five that a World drawing pixels accepts; the GUI section declares the other two, text measurement and frame ticks. A window is a stream: the pointer, the keys and the platform's resizes arrive as responses, and nothing in the window decides who gets them. A scene is a presence element like a label, the list of draw commands for that window, replaced whenever the picture changes. A focusable is a presence element too: it names the region the library considers focused, so that a World can show the platform's focus ring, place an input method's candidate window or tell a screen reader, and it hears nothing, because which region gets a key is decided in Prism by name. Everything between a program's buttons and those tags is Prism, and the long section on the GUI library builds it.

```prism
type FsRequest = choice Read | Write Text
type FsResponse = choice:
    Contents Text
    Missing
    Denied
type Contents = choice:
    Unavailable
    Known Text
type Listing = choice:
    Unlisted
    Entries [Text]

tag Fs.File of Element:                      # request: reads and writes, answered later
    path: Text
    requests: !FsRequest
    responds: FsResponse

tag Fs.Contents of Element:                  # state: the file as a value that is true now
    path: Text
    state: Contents

tag Fs.Dir of Element:                       # state: the listing as a value that is true now
    path: Text
    state: Listing
```

Two ways to get at a file because there are two things you might mean. `Fs.File` is a conversation: ask, and be answered some occurrences later, once per ask, with a response that can say the file is missing or the read was denied. `Fs.Contents` is a fact: the contents of the path as a dynamic, `Unavailable` when the World does not have them, `Known` when it does, pushed when they change. The request form is for writes and for one-shot reads where the program wants to know that *this* ask was answered; the state form is for anything a program wants to show.

```prism
type SocketEvent = choice:
    Received Text
    Closed
type Link = choice:
    Connecting
    Connected
    Disconnected Text
type Accepted = Accepted Nat
type HttpRequest = record:
    method: Text
    url: Text
    body: Text
type HttpResponse = record:
    request: HttpRequest
    status: Nat
    body: Text

tag Net.Socket of Element:                   # the full conversation
    address: Text
    requests: !Text
    responds: SocketEvent
    state: Link

tag Net.Listener of Element:                 # stream: accepted connections, with ids the World allocates
    port: Nat
    responds: Accepted

tag Net.Http of Element:                     # request: one firing, one answer, later
    requests: !HttpRequest
    responds: HttpResponse
```

A socket has all three parts because a connection is all three things: you send on it, you receive from it, and it is up or not. `Net.Http` has no fields at all: the URL is in the request, so one slot can fetch many things over its life, and the response says which request it answers so that two in flight can be told apart. A listener is the stream of accepted connections; each accepted connection is its own `Net.Conn` element keyed by the id the World allocated, built under a `for`, which is the server of the worked examples.

```prism
tag Time.After of Element:                   # request: a delay in milliseconds, echoed back once it has passed
    requests: !Nat
    responds: Nat

tag Time.Every of Element:                   # stream: a tick per period
    period: Int
    responds: Tick

tag Clock of Element:                        # state: the time, as the World last pushed it
    state: Instant
```

There is no `now()`. The clock is a fact the World pushes, at most once per turn, and a program that wants to be woken asks `Time.After` or keeps a `Time.Every` in its list. The split between the three is the split between asking once, subscribing, and reading, and it is the same split as between `Fs.File`, `Net.Listener` and `Fs.Contents`.

```prism
type ChildRequest = choice:
    Stdin Text
    Signal Nat
type ChildEvent = choice:
    Stdout Text
    Stderr Text
type Status = choice:
    Running
    Exited Int

tag Process.Child of Element:                # the full conversation again: a subprocess
    command: [Text]
    requests: !ChildRequest
    responds: ChildEvent
    state: Status

tag Process.Args of Element:                 # state: the arguments, which an interceptor may replace
    state: [Text]

tag Cli.Stdin of Element:                    # stream
    responds: StdinEvent

tag Cli.Stdout of Element:                   # request: a write that fails comes back
    requests: !Text
    responds: WriteFailed

tag Process.Exit of Element:                 # sink: the firing is the exit
    requests: !Code

tag Defer of Element:                        # request: answered in a later occurrence of the same turn
    requests: !unit
    responds: unit
```

A subprocess is a socket with a command instead of an address: the World starts it because the element is in the list, stops it when the element leaves, and its exit status is a fact. Arguments are state rather than parameters of `@main` so that an interceptor can give a subtree different ones and a test can set them. Exit is a sink whose firing is the instruction, never a presence whose appearance is, because nothing in this language is triggered by an element appearing. `Defer` is the one request element every World must provide, and it is what lets an interceptor answer a request at all; it is the subject of its own section below.

### The rules a tag obeys

Four rules, each a consequence of elements being plain data that crosses a boundary.

Field types are concrete. The boundary is between a typed program and an untyped outside, and the outside has to be able to read a field without knowing what type variable it was instantiated at. A tag that said `value: a` would be a tag no World could accept.

A `match` on `Element` ends in a catch-all arm, because the set of alternatives is open: a module this program has never seen may declare a tag, and an interceptor that forgot about it must still do something with it. In an interceptor the catch-all is the forwarding case, and leaving it out is the compile error that stops a sandbox from silently dropping an element it did not anticipate.

Matching is by tag, with fields bound by name as for a record, and `Ns.*(..)` matches any tag under a namespace. There is no matching on a field's type, because every field's type is fixed by its tag; `Fs.File(path=p)` binds `p : Text` and nothing else is possible.

Equality is structural, tag then fields. Two `Fs.Contents(path="/etc/motd")` values are equal wherever they were built, and that equality is what identifies state across the boundary: it is the key of the push map and the argument of the oracle. A field containing an event, which only a `requests` field does, is excluded from the comparison, because events have no equality; two elements that differ only in their request event are the same element to the outside, which is right, since the outside never sees the event, only its firings.

## `@Element`

`@Element e` is a widget. Binding it in a widget block creates a slot for `e` in the enclosing node and returns what `e`'s tag declares:

| tag declares | `@Element e` has type |
| --- | --- |
| nothing | `@unit` |
| `responds: R` | `@!R` |
| `state: S` | `@$S` |
| `responds: R` and `state: S` | `@((!R, $S))` |

The slot exists from the occurrence in which the widget is built until its node is torn down, and its element never changes: a field is read once, when the slot is created, and anything that looks like changing a field is creating another slot. `@Element` is therefore the only place where the program's text meets the outside, and a reader who wants to know everything a widget asks of the world greps for one word.

```prism
@Button: fn Text -> @!Click = \|label| @widget:
    !click <- @Element Ui.Button(label=label)
    @ !click

@viewer: fn Text -> @unit = \|path| @widget:
    !started <- @started
    !resp <- @Element Fs.File(path=path, requests=(Read on !started))
    $text <- "(reading...)" <= ShowResponse !resp
    @Label $text
    @ unit

((!rx, $link)) <- @Element Net.Socket(address="chat.example:7000", requests=!out)
```

The three statements show the three return forms. A button returns its click. A file read returns its response event, and the request is wired in at creation: `Read on !started` is an event that fires once, in the build occurrence, and that firing is the one read this widget will ever make unless it fires the event again. A socket returns a tuple, and the pattern on the left takes it apart.

When the tag of `e` is not statically known, as in a catch-all arm of a `match` inside an interceptor, `@Element e` has the generic type `@((!Response, $State))`, and the response and state types are checked at the slot at runtime, per tag. In a tag-specific arm the types are the tag's, and the checker knows them. This is the one place in the language where a type is refined by a `match` on a tag, and it is what lets an interceptor forward an element it has never heard of while still being typed when it handles one it has.

One thing `@Element` does not take is a dynamic where a field goes. `@Element Ui.Label(text=$text)` is not a label whose text changes; it is an application to a dynamic, which lifts, so it is `$(@Element Ui.Label(text=text))`, a dynamic of widgets. Binding a dynamic of widgets lifts to `dyn`: `@Element Ui.Label(text=$text)` as a statement means `dyn $text: @Element Ui.Label(text=_)`, a fresh slot per change of the text, and a bound name on the left is the `!Switch` of the `dyn`'s event when the element responds, or a hold of dynamics joined with `$Join` when it has state, with the first build's result as the hold's initial value. The chapter says this once here and uses the short form from now on; the section on dynamic fields says what it costs and why it is the right cost.

A program rarely spells this out for library widgets. Library widgets take dynamic arguments: `@Button` takes a `$Text` label, and a button whose label changes is written `@Button $label`. The fresh slot per change belongs to the library's interceptor, which builds a new `Ui.Button` slot when the label changes and answers to the program through the one `!Click` event it returns; a World never sees it, because a World sees only the `Gui.*` elements the interceptor emits. The rule that a field change is a fresh slot holds in full for every element that reaches a World, and the `dyn` forms above are that rule written out, not the way a program normally labels a button.

## The creation rule

> **Elements never fire in the occurrence they were created. Firing within the same occurrence implies they could have been pure functions. Instead, elements with request-like semantics take an input event and fire their output event in a different occurrence.**

Read the second sentence twice, because it is the argument and not just the rule. Suppose an element could be created and answered in one occurrence. Then from the program's side, `@Element Fs.File(...)` with a `Read` fired from `@started` would be a widget that returns the file's contents in its build occurrence, which is to say a function from a path to text. But it is not a function from a path to text: it is a conversation with something outside that may take time, may fail, may be denied, and may never answer. A rule that let it answer at once would let a program depend on an effect having happened in the moment it was asked for, and every ordering bug at a boundary is some version of that dependency. The rule says the dependency cannot be written.

The practical statement is in three parts, one per party.

**The program** may assume that nothing it asked for has happened yet in the occurrence it asked. A widget that fires a `Read` from `@started` knows that its response event is quiet in the build occurrence and that its hold is at the initial value then. A widget that wants something as soon as it exists fires its request from `@started`, which fires in the build occurrence and is pure; the request firing in that occurrence is allowed and is the whole point, and the response arrives in a later one.

**Worlds and interceptors** are bound not to answer in the occurrence of the request, and not to offer any element that responds merely because it was created. The first is a norm a World cannot break by construction, since request firings come out of the run call after the occurrence commits and the earliest an answer can go in is the next call. The second is a norm a World could break, by starting a download because an `Fs.File` slot appeared and delivering the contents next turn; a conforming World does not, and the next subsection is the reason.

**The runtime** enforces the half it can. Responses enter the program only at the start of an occurrence, through the run call's response map, or through an interceptor's response event, which is routed against the element list as the occurrence opened. A slot created in occurrence n is not in that list, so nothing can be delivered to it in n: an interceptor that fires a response at a path created this occurrence has sent to an unknown path, which is an error that aborts the occurrence. That one check is what turns the rule from a convention into a law. An interceptor cannot cheat it, because the only way down to a child's slot is the response argument of `intercept`, and that argument is routed.

### No request on creation

A World could offer an element that acts because it exists: an `Fs.Load(path)` that reads the file when the slot appears and responds with the contents next turn. It would save a line in every file viewer, and it is what most toolkits do, and it is not offered, for three reasons that are the same three reasons `@started` exists.

Building stops being pure. Today building a widget adds nodes and slots and nothing else; with act-on-creation, every rebuild of a `dyn` and every new key of a `for` is also an effect, at a moment the program's text does not show. The fetch-on-mount that fired twice was exactly this.

The trigger stops being a value. With `Read on !started` the program can merge the start with a refresh button, `Read on (!started | !refresh)`, gate it on a setting, or replace it with a click, without touching the element. A test sees the request fire as an ordinary event and can assert `Requested Fs.File(..), Read`. With act-on-creation the trigger is the slot's existence, which is not an event and cannot be merged with anything.

There would be two kinds of request element, the ones that act on build and the ones that act on events, and every interceptor between a widget and the World would have to handle both: a sandbox would deny some requests by answering their events and others by refusing to forward their elements, and a cache would memoise two protocols. With one kind, an interceptor answers requests and nothing else.

The objection is that `@started` is act-on-creation with an extra step. It is one step, and the step is the point: the program wrote down when it asks.

### The complement: state has a value

The rule has one complement, and it is not an exception. A `state` element has a value in the occurrence it is created. That is not a firing; it is the dynamic being a dynamic. The value it has in that occurrence is the value the outside held as the occurrence opened, from the World's snapshot or an interceptor's supplied dynamic, and if the outside pushes a change in that same occurrence the new slot sees the change like every slot with that value. A state element is an observation of something that already exists, the way reading a dynamic is, and creating the slot asks for nothing. The section on why state has to be there in its first occurrence argues that no other arrangement keeps a widget consistent with itself.

## Identity

A slot has two identities, and they answer different questions.

### By path

Every slot has a path: its position in the widget tree, made of the bind positions, `for` keys and `dyn` generations from the root down to it. The worked examples write a path as `0/#1/[7]/2`: the first bind in `@main`, generation one of the `dyn` there, key 7 of the `for` inside it, the third bind in that block. A path is unique for the life of its slot and is never reused: a `dyn` that rebuilds gives the new slot a new generation number, a `for` key that leaves and comes back gets a new path, and a World that keeps a stale path in a queue will find it refused rather than redirected. An implementation may represent paths as it likes provided the element list is in tree order and the outside can treat a path as an opaque key.

Responses are addressed by path. A click is for *this* button, not for every button with this label; the answer to a read is for the slot that asked, not for every slot reading that file. A path is the identity of a conversation.

### By value

A `state` element's dynamic is identified by the element's value, tag and fields compared structurally. Two slots whose elements are equal observe one and the same state: whoever supplies it does so once per distinct value, however many slots share it, and every push keyed by that value reaches every one of them, including a slot created in the occurrence of the push. A value is the identity of a fact.

Two examples show the distinction, and they are the two the worked examples walk through.

**The socket.** `Net.Socket(address="chat.example:7000")` has a request, a response and a state. Its sends and receives are its own: addressed by path, a conversation between this slot and the connection. Its link state is shared with every other socket element of the same address in the program, because there is one connection, and whether it is up is a fact about the world and not about any slot. Two chat panes to one server are two conversations on one link. If the user types the old address back while a second slot still has it, the connection stays up through the change, which is either a feature or a surprise depending on what the program wanted, and either way it is in the program's text. Whether the World keeps one OS socket per address or per slot is the World's business and is invisible so long as it keeps the equality law.

**The clock.** `Clock` has no fields, so every clock element in the program is one clock. A stopwatch, a status bar and a log stamp each bind `@Element Clock` without knowing about each other, and in every occurrence all three read the same instant, because there is one value and the World pushes it once. A test sets it once, `State Clock, Instant 5000`, and all three see it. Had state been identified by path, the three would be three subscriptions the World had to answer identically, and the equality law would be a promise the World made three times per occurrence instead of a fact about one key.

The two identities are not in tension. A slot with all three parts has a path for its responses and a value for its state, and the two never cross: a response never reaches a slot because its value matched, and a push never reaches a slot because its path matched. The alternative, state by path, is worth imagining for a paragraph. Each `Fs.Contents` slot would be its own watch; a hundred rows reading one file would be a hundred watches at the World; the World would be obliged to push the same value to all hundred in one occurrence and to answer the oracle identically for each; and an interceptor that wanted to share one outer query among many inner slots would need a table from value to supplied dynamic, which is the table the runtime keeps anyway under value identity. Identity by value puts the sharing where it is free.

## The element list

The element list of a widget is an ordinary dynamic, `$(Path ==> Slot)`: a map from path to slot, in tree order because paths are ordered that way. Its value before occurrence n is the list as n opened; its change event, if the list changed in n, fires with the list after n. A slot created in n is in the list after n; a slot torn down in n is in the list before n and not after. Interceptors read it through `intercept`, and at the root the World reads it out of the run call.

```prism
type Slot = record:
    element: Element
    !request: Request          # typed by the tag, once `element` has been matched
```

A slot is its element and, if the tag declares one, its request event. The request event is in the slot so that whoever reads the list can wire it into an element of their own; the response event and the state dynamic are not in the slot, because they are what the reader supplies, not what the reader sees.

Because the list is a dynamic, everything the language says about dynamics applies to it. It changes promptly in the occurrence a slot is created or torn down. Reading it through a tag gives the list as the occurrence opened. Deriving from it is a derived dynamic: `GuiOnly $slots` is a dynamic of the GUI elements, kept current for free. And a `for` over it builds one block per slot, with the slot's path as the key, which is how every interceptor in this chapter is written.

### Routing

An input addressed to a path in occurrence n is delivered to the slot at that path in the list *as n opened*. Clicks go to what was there before the click. A slot created in n is addressable from n+1. An input addressed to a path not in that list is an error that aborts the occurrence: the sender had the list and sent to something not in it.

This is the tag law at the boundary. The list as the occurrence opened is the list's value before the occurrence, and reading a dynamic at an event gives its value before the occurrence. There is no separate routing rule to learn; routing reads the list the way every other read at an event reads a dynamic, and the consequences follow from that. A button clicked in the occurrence it is replaced delivers its click to the old slot, under the old label, because the old slot is what the user saw and what the World's list named. A hit-test in a layout interceptor reads the child's list through a tag and so hit-tests against the list the user was looking at, with no extra rule. A World that queued a click for a path and then received a list without that path must drop the click, and the obligation is stated in the chapter on Worlds; an interceptor that does the same thing gets an error, because an interceptor is Prism code and Prism code that sends to an unknown path has a bug.

### The diode

Inputs cause changes to the element list; changes to the element list never cause inputs. A new occurrence comes only from outside, with inputs addressed to slots that already existed. Nothing in the language observes its own element list and reacts to it except an interceptor, and an interceptor reads it through a delayed edge: the `for` over the list builds blocks for new slots in the occurrence they appear, but nothing a new block fires can be delivered to the new slot in that occurrence, by routing.

The diode is why there is no feedback loop in a program whose label is a fresh slot per keystroke. If a slot appearing were an event, a text input whose label rebuilds on every edit would fire an event on every edit and the World would have to decide whether that event causes anything, and some World would decide yes. Here the list is a description, the World reads it after the occurrence, and the only things that start occurrences are the things the World chooses to deliver. A label appearing is not a thing that happened; it is a thing that is.

It is also why a World can be as simple as it is. The run call takes inputs and returns a list. The World never subscribes to anything, never registers interest, never receives a notification that the list changed; it reads the list when the call returns and diffs it against the last one if it wants to. Everything a GUI framework does with observers, dirty flags and invalidation queues is replaced by the World looking at a map.

## Two laws across the boundary

Everything in this chapter rests on two laws, and each binds a named party.

**Equality.** Two readings of one state element's dynamic in one occurrence, by any two slots, anywhere in the program, give the same value. The occurrence is the version. This binds whoever supplies state: a World, which must answer its oracle from one snapshot per occurrence and push one value per key; and an interceptor, which gets the law for free because the runtime builds one supplied widget per distinct value and hands its dynamic to every slot with that value. The law is what the language's own equality-of-readings law (two readings of one dynamic in one occurrence agree) becomes when the dynamic's source is outside, and it is stated separately because the source is outside: a World that answers two slots differently in one occurrence has broken the program, not the World, and the Test World checks it every turn.

**Monotone turns.** A response reaches a slot in an occurrence strictly after the request's, and the outside reads a request only after its occurrence commits. There is no round trip inside an occurrence. This binds Worlds and interceptors, and the runtime enforces it for interceptors by routing and for Worlds by the structure of the run call. Its corollary is the creation rule: a slot created in n fired at the earliest in n, so nothing can answer it before n+1.

Together they say what an occurrence is at the boundary: a snapshot in and a list out, with every value in agreeing with every other value in, and nothing out visible until the snapshot is closed. That is a transaction, and the rest of the chapter is what a Prism program can build on top of one.

## Dynamic fields

Fields are plain data. A label's text, a file's path, a socket's address: each is read once when the slot is created and never again. The question every reader asks next is what happens to the label whose text changes, and the answer is that the language already has a construct for building something afresh when a dynamic changes, and uses it.

An element with a dynamic where a field goes is an application to a dynamic, which lifts: `Ui.Label(text=$text)` is `$(Ui.Label(text=text))`, a dynamic of elements, and `@Element` of it is a dynamic of widgets. Binding a dynamic of widgets is `dyn`. So

```prism
@Element Ui.Label(text=$text)
```

means

```prism
dyn $text:
    @Element Ui.Label(text=_)
```

a fresh slot per change of `$text`, with the old slot torn down at the commit of the occurrence in which the text changed and the new one in the list from that occurrence. For an element that responds, the bound name is the `!Switch` over the generations:

```prism
!click <- @Element Ui.Button(label=$label)
# is
!!clicks <- dyn $label:
    @Element Ui.Button(label=_)
let !click = !Switch !!clicks
```

and for an element with state it is a hold of dynamics joined:

```prism
$contents <- @Element Fs.Contents(path=$path)
# is
!$each <- dyn $path:
    @Element Fs.Contents(path=_)
$$current <- $first <= !$each        # $first: the dynamic the first build returned
let $contents = $Join $$current
```

where the first build's result is available as the hold's initial value because the first build happens in the occurrence the `dyn` is built; the runtime has it in hand when it builds the hold, and a library spells the three lines as one word. An event where a field goes is a type error unless the field is the declared `requests` event, which is the one field whose type is an event.

The consequences, each of which some other part of the design depends on: the element list never contains a sigil, so a World and an interceptor read plain data; value identity never has to pick a moment, because a value is fixed for its slot's life; nothing about an element changes after creation, so "the same element with a new field" is not a case any reader has to handle; a response to the old slot is for the old slot, and the World drops it because the slot is gone; and the new slot's state is sampled in the occurrence of the change, under the rule for widgets built mid-occurrence, so a file viewer whose path changes shows the new file's state in the occurrence the path changed.

### Fresh slot per change, case by case

The worry about this design is that a slot is destroyed and recreated for every change of a field, and some things want to persist across the change. Take the cases one at a time.

| Element | Dynamic field | Fresh slot per change is | Why |
| --- | --- | --- | --- |
| `Ui.Label(text)` | the text | right | A draw command. The World diffs lists; nothing is in flight; a new label at the same position is what a redraw is. |
| `Ui.Button(label)` | the label | right | A click in the occurrence of the relabel goes to the old slot, under the old label, which is what the user clicked. From the next occurrence the new slot's clicks are heard. There is no moment with two buttons listened to or none. |
| `Fs.Contents(path)` | the path | right | A new path is a new file. The new slot samples the new file's state in the occurrence of the change; the old slot's watch is released when the World sees it gone. |
| `Fs.File(path, requests)` | the path | right | Responses to the old path's requests are for the old file. The World drops them because the slot is gone, and the program could not have told them apart from the new file's anyway. |
| `Net.Http` | none | n/a | The URL is in the request, so the element has no field to change and one slot serves many fetches. This is the design to reach for when a field would change per request. |
| `Net.Socket(address)` | the address | right | Reconnect. The old generation's send in the occurrence of the change goes on the old connection, which is the only connection that is up. |
| `Time.Every(period)` | the period | acceptable | The phase restarts. A World that wants phase continuity keys its timer on tree position, which the path exposes; nothing in the program can tell. |
| `Process.Child(command)` | the command | right | A different command is a different process. The old one is stopped when its element leaves the list. |
| `Gtk.Entry(value)` native | the value | the hard case | A native text box has cursor and selection state. Per keystroke, a fresh slot loses it. |
| `Gui.Window(title)` as a container | the title | a non-problem | Containment is by interception, not by fields. The window's children are widgets under an interceptor; the window element is a leaf with no children field, so its rebuild touches nothing else. |

The hard case is the native text input, and the answer is that in Prism the text input is not native. The GUI library is Prism, so the cursor is a hold in `@TextInput` outside any `dyn`; the keystrokes arrive at the window, and the library routes each one to the input whose name is the focus, a name that does not change when you type into it; and what the World sees is a scene of draw commands and, for the focused input, a `Gui.Focusable` with that name. The worked example with a cursor is four occurrences of exactly this, and no cursor is lost because there is no second copy of it anywhere. A World that wraps a native toolkit and wants continuity for a native widget keys it by tree position and tag, which the path exposes; that is a World's policy, and the language promises nothing about it.

### The declared-dynamic-field alternative

Suppose a tag could declare a field as `$Text`:

```prism
tag Gtk.TextBox of Element:
    value: $Text
```

meaning the slot persists across changes of the value and the outside sees the list's value change at the same path. This is the sample-plus-update form: the World reads the field at creation and is told each change. It would make the native text box a language-level non-problem, and it is the first thing a reader who has written a toolkit binding asks for.

What it costs. Elements would contain a sigil, so the list would no longer be plain data: a World would hold a dynamic, which is a runtime object, and the rule that no runtime object crosses the boundary would have an exception. Value identity would have to pick a moment, because two `Gtk.TextBox` elements with equal values now may differ next occurrence, and the oracle, the push map and the supplier would each need a rule for which moment. A response to a slot whose field changed would need a second rule for what it answers: the slot as it was when the request fired, or as it is now. And interceptors would have two kinds of element to forward, the ones whose fields are fixed and the ones whose fields move, and forwarding the second kind means forwarding a dynamic, which is a subscription, which is the thing the boundary was designed not to have.

The table above is the argument that the cost is not yet paid for. Every case but one is right or acceptable with fresh slots, and the one hard case has a better answer in the library than in the language. If a use case appears that the leaf-plus-interceptor pattern cannot express, a declared dynamic field is the escape hatch, and the costs above are what it would cost; until then the boundary never sees a sigil, and that single fact is what makes a World diff lists of plain data instead of subscribing to anything.

## Why state has to be there in its first occurrence

A dynamic has a value at every occurrence. A widget that binds `$contents <- @Element Fs.Contents(path="/etc/motd")` therefore has, in the occurrence it is built, a value for `$contents`, and the question is where it comes from. The answer is the oracle: a World answers synchronously from its snapshot as the occurrence opened, and an interceptor wires in a dynamic it already has. This section is the reasoning for why nothing less will do, written as the designs one would try first.

### Suppose state were a request

The first design to try builds state out of the request element the language already has. Ask for the current value with a request fired from `@started`; get it back next occurrence; subscribe to changes thereafter; hold the lot into a dynamic. The library widget would look like this:

```prism
tag Fs.Query of Element:                     # not the design
    path: Text
    requests: !unit                          # "tell me the current contents"
    responds: Contents                       # the answer, and every later change

@Contents: fn Text -> @$?Contents = \|path| @widget:
    !started <- @started
    !msg <- @Element Fs.Query(path=path, requests=(unit on !started))
    $contents <- Nil <= Some !msg
    @ $contents
```

The program holds a `?Contents` that is `Nil` for one occurrence. That is honest, in the sense that a type says the value may be absent, and it is cheap, in the sense that the World needs no oracle. The trouble is not the one occurrence of waiting. The trouble is that the wait is visible, and visible in a way no library can hide.

Take a widget built in occurrence n by a `dyn` over `$selected`, which changed in n, and which also reads a dynamic `$theme` captured from its enclosing scope. Under the rule for widgets built mid-occurrence, the new widget sees all of n: it sees `$selected`'s new value (that is why it was built) and it sees `$theme`'s value as of n, including a change to `$theme` in n if there was one. Now give it a state request. Its `$contents` is `Nil` in n and `Some` in n+1. So there is an occurrence, n, in which the widget has the new `$selected`, the new `$theme`, and no contents; and if `$theme` changes again in n+1, there is no occurrence at all in which the widget has the contents and the `$theme` it was built under. The widget was supposed to be a function of three dynamics and it has been shown a combination of their values that never existed.

### Suppose an interceptor hid the delay

The second design keeps state as a request but puts an interceptor around the child that shows it one occurrence for the runtime's two: the child's build occurrence n is held back, the request goes out, the answer comes in n+1, and the interceptor presents n+1 to the child as if it were n, with the answer already there. Everything downstream of the child is delayed by one occurrence to keep the story consistent.

The residue is the captured dynamic. `$theme` is not the child's; it is lexically in scope from outside the interceptor, and the child reads it directly, not through the interceptor's response argument. If `$theme` changed in n and again in n+1, the child sees both changes, and nothing the interceptor does to the child's own slots can hide that two occurrences passed. The interceptor could try to freeze every dynamic the child captures, but the child captures whatever its author wrote, and the interceptor does not know which names those are; an interceptor sees elements, not scope. The only arrangement under which a widget built in n is consistent in n is the one where its state is there in n, and the only party that can put it there is whoever holds the state, answering synchronously as the occurrence opens.

### Suppose a placeholder

The third design admits the wait and gives it a name: the state type gets a `NotYet` case, the World answers next occurrence, and every `match` on the state has a `NotYet` arm. It is the first design with a nicer type, and it fails the same way; its failure looks like a feature, which is why it gets its own paragraph. A `NotYet` arm in every consumer is the `if (data)` guard you have written a thousand times and forgotten once, except that here forgetting it is a compile error, so the design feels safe. What it hides is that `NotYet` is not a value the World ever held. `Unavailable` is: the World looked and the file was not there, or had not been read. `NotYet` is the runtime's constant before anyone looked, and a program that draws it is drawing something that was never true of the world. The language has a goal that "not yet" is a type, and this is the case that shows the goal needs the *World's* not-yet, not the runtime's.

### What the oracle is

So the World, as it opens an occurrence, answers a function from state-element value to state value, synchronously, from its snapshot; and an interceptor, when its child creates a state slot, builds a widget that returns a dynamic, then and there, in the same build pass, so the slot's value is the dynamic's value before the occurrence. Both give a widget built in n a state value in n that agrees with every other reading of that value in n, which is the equality law, and both let the widget see a push in n to that value, which is the rule for widgets built mid-occurrence. The cost is that the World answers a question in the middle of a run call rather than only at its start, and that cost is paid by Worlds, not programs. A World with a fact in memory answers it; a World that would have to go and look answers `Unavailable` and pushes `Known` when it has looked, which is what it would have had to do anyway, one occurrence later, with the honest difference that the program never shows a value the World did not hold.

### What this subsumes

Two protocols one might design for live state fall out of this one. A *query plus subscription*, where an element answers "what is it now" on request and reports changes as a stream, with a rule that an answer and a change in one occurrence agree, is what the World does internally to maintain its snapshot; the program never sees the two halves because the oracle and the push map are the snapshot's interface. A *versioned channel*, where every message is a complete state or a delta against a version number, is what a World keeps toward a remote source that does not share its clock; the program never sees the version because the occurrence is the version. In both cases the matching is the World's job, and it is the World's job because only the World sees the race between a read and a watch. The program gets a dynamic.

## `intercept`

You have written the code that wraps a third-party widget so that its network calls go through your proxy. In most frameworks that is a global: you replace `fetch` for the whole process, and the widget beside it that you did not want to proxy goes through the proxy too, and the test that ran before yours left `fetch` replaced. Or it is a context object the widget was kind enough to accept, in which case the widget you did not write ignores it. Interception here is per subtree, written in the program, and there is no channel a child can use that the interceptor around it does not see, because the language has no ambient anything.

An interceptor stands between a child and everything outside it. From the child's side it is indistinguishable from a World: the child's slots get their responses and their state from it, and the child cannot ask who is answering. It does exactly two things, and the two directions never mix.

*Downward*, it supplies what the child's slots return: responses for elements that respond, dynamics for elements with state.

*Upward*, it makes elements of its own, with `@Element` like any widget, or does not. Nothing the child creates goes further out by itself. Whether the interceptor re-emits a child's element unchanged, rewrites it, holds it back, merges it with another, answers it itself, or decides at runtime, is ordinary code.

That last sentence is the test the design has to pass. Every policy an interceptor could have must be writable by hand in the language, because if some policy needs a mode of the primitive, the semantics are incomplete and the mode is hiding it. `intercept` is a low-level call for framework writers; it is not meant to be convenient, and the library wraps it.

```prism
intercept: fn @a, !(Path ==> Response), $(fn Element -> @$State) -> @((a, $(Path ==> Slot)))

type Slot = record:
    element: Element
    !request: Request          # typed by the tag, once `element` has been matched
```

`intercept @child, !responses, $supplier` builds `@child` as a child widget of the interceptor's node and captures its element list. It returns the child's result and that list, keyed by path. The two other arguments are the two downward supplies: responses by path, and state by value through the supplier.

### Downward: responses by path

`!responses` is an event of maps from the child's paths to response values. When it fires in occurrence n, each entry is delivered to the slot at that path in the child's list *as n opened*, under the routing rule, and that slot's response event fires with the value. An entry whose path is not in that list is an error that aborts the occurrence. The interceptor makes this event however it likes; the usual form is a `for` over the list that does something per slot and lets consolidation build the map:

```prism
@Forward: fn $(Path ==> Slot) -> @!(Path ==> Response) = \|$slots| @widget:
    for $slots:                      # key is the child's path
        dyn $_:                      # a slot never changes, so this builds once
            ((!resp, ignore)) <- @Element _.element
            @ !resp

@Transparent: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- @Forward $slots
    @ result
```

Read `@Forward` slowly, because every interceptor in this chapter is a variation on it. `for $slots` builds one block per slot in the child's list, keyed by the child's path; a block is built in the occurrence its slot appears and torn down at the commit of the occurrence its slot leaves. Inside, `$_` is the slot as a dynamic, and since a slot never changes, `dyn $_:` builds exactly once and gives `_` as the plain slot. `@Element _.element` makes an element of the interceptor's own with the same value, in the interceptor's node; its tag is not statically known, so it returns the generic pair of a response event and a state dynamic, and the pattern keeps the event and discards the dynamic, which nothing needs because the child's state slots are answered by the supplier, not by this loop. The block returns the event, and `for` consolidates the blocks' events into `!(Path ==> Response)`, firing in any occurrence in which at least one outer response arrived, with exactly the entries that did. That map is `!down`, and `intercept` delivers each entry to the child's slot at that path.

The request side forwards itself. The child's request event is in the slot, `_.!request`, and `@Element _.element` makes an element whose `requests` field is that very event, so when the child's request fires, the interceptor's element's request fires in the same occurrence, and the outside sees it after the occurrence with no delay. A widget built in occurrence n sees all of n, so even a request fired from `@started` in a slot's creation occurrence is seen by the block built for that slot in the same occurrence and goes out that turn.

`!down` is used before it is defined, and its definition reads `$slots`, which the `intercept` that consumes `!down` produces. That cycle is legal because routing reads the list as the occurrence opened: the edge from `!down` into the child is through a tag on the list. At runtime the only way round the loop in one occurrence would be a block, built for a slot created in n, firing something into `!down` in n; by the creation rule only pure code could do that, and routing refuses it as an unknown path. An interceptor that writes `@ click on !started` in a block has written a bug, and the runtime names the path.

`@Transparent` is the identity. A child under it has the same list at the outside, one level deeper, with its responses coming back under its own paths and its state supplied by an element of the same value. The law the language promises is that an interceptor which re-emits everything and supplies by forwarding is observably not there; the conformance suite tests it by running every program under `@Transparent` and comparing transcripts.

### Downward: state by value, through the supplier

The third argument is a dynamic of a function from an element to *a widget returning the state's dynamic*. When the child creates a state slot for element `e`, and no supplied widget for a value equal to `e` is alive in this interceptor, the runtime reads the function (the supplier dynamic's current value at that instant), applies it to `e`, and builds the widget it returns, then and there, as a child of the interceptor's node, in the same build pass as the child's slot. The widget's result dynamic is the slot's dynamic from that instant on: its value before the occurrence is the slot's value in its creation occurrence, and its changes are the slot's changes. The slot therefore has its value in its creation occurrence, exactly as it would from a World's oracle.

The widget runs in the interceptor's context, so its own elements go into the interceptor's list. It can ask the outer context for the same element, a rewritten one, two of them, or nothing:

```prism
\|e| @Element e                              # transparent: ask outside for the same thing
\|e| @Element (Rewrite e)                    # ask outside for something else
\|e| @ $ Unavailable                         # sandbox: a constant, no element at all
\|e| @widget:                                # cache: a hold around a forwarded query
    $live <- @Element e
    ...
    @ $cached
```

There is no second mechanism and no table. Forwarding a state element *is* making an element, with the one call the language has for that, and supplying a constant is returning the constant dynamic, with the one call the language has for that.

The supplied widget is built once per distinct element value per interceptor, and it lives as long as some child slot with that value lives. A second child slot with an equal value joins the existing supplied widget and gets the same dynamic, whatever the supplier dynamic says at that moment; the supplier is consulted only for a value no live supplied widget serves. That is value identity holding inside an interceptor for free, and it is also the equality law: two child slots with equal values read one dynamic, so they cannot disagree. When the last child slot with that value is torn down, the supplied widget is torn down with it at the same commit, and its elements leave the interceptor's list, so the outer context sees the interest go.

The supplier is a dynamic so that policy can change over time. A slot's dynamic is fixed at creation; if the supplier changes, values first seen afterwards get the new policy and values already being served keep theirs. Most interceptors pass a constant, `$ \|e| @Element e`. The migration interceptor below is the one that does not.

The World works the same way, and the parallel is deliberate: its supplier is a synchronous function from element to value, the oracle, with pushes for the changes. An interceptor's supplier is that function with a widget around it, because an interceptor, unlike a World, has an outside to ask, and asking is building an element.

### Upward: elements, or nothing

An interceptor's own elements are made with `@Element`, and those are what the next interceptor out, or the World, sees. There is no third way. The child's list is a dynamic the interceptor holds; nothing in the runtime reads it on the interceptor's behalf and re-emits it. If the interceptor's code does not make an element for a child's slot, that slot is invisible outside, and the child waits for an answer that never comes unless the interceptor gives one. Holding back is therefore the default and forwarding is a choice, which is the right way round for a sandbox and the only way round under which "nothing reaches past an interceptor unless Prism code sends it" can be a law.

### Why forwarding is manual

Suppose `intercept` took a table instead: a list of tag patterns to forward unchanged and a map from tag to supplier, and did the forwarding itself.

```prism
# not the design
intercept @child, forward=[Gui.*, Time.*], supply={Fs.Contents: ..., Clock: ...}
```

It is convenient, and it covers the sandbox and the logger. Now a library wants to forward a file read with its path rewritten under a root; the table grows a `rewrite` entry with a function per tag. A cache wants to forward one query for many identical children and hold the answer; the table grows a `memo` mode with a policy for when the memo is stale. A tracing layer wants to forward and also log each request; the table grows an `observe` hook. A quota layer wants to forward until a count is reached and deny after; the table grows a condition, and the condition has to read a hold, so the table is now a dynamic of tables and the question of when it is re-read has to be answered. A migration layer wants to forward to one of two backends depending on a dynamic; that is a second condition with a different scope. Each is a new mode of the primitive, each mode has its own rules for when it runs and what it sees, and before long the table is a second language for things the first language already says, with its own ordering rules at the one place in the program where ordering rules matter most.

The alternative is a primitive that does nothing but expose the two directions, so that forwarding is the shortest program an interceptor can write, `@Forward` above, and every other policy is a longer program in the same language, with the same laws about tags, holds and occurrences that the rest of the program has. That is the goal that anything a convenience does, you can do by hand with the same primitives, applied where it matters most. It also keeps the invariant a sandbox is made of: with a table, the question "does anything the child makes reach the outside" is answered by the table's semantics; with manual forwarding, it is answered by reading the interceptor's `match`, and the catch-all arm is where the answer is.

### Typing

Inside a `match` arm on tag `T`, `_.!request` has `T`'s declared request type, `@Element` of an element built from `T` returns `T`'s declared types, and a widget the supplier returns for `T` must return `T`'s declared state type. In the catch-all arm, `_.!request` is the generic `Request`, `@Element _.element` is the generic `@((!Response, $State))`, and the runtime checks the per-slot types at the slot. The refinement is a rule of the checker and not a runtime structure: the extensible variant with per-tag payload types is what makes it natural, and the alternative, a runtime check everywhere, is what elements get in the generic arm anyway.

An interceptor that wants to be typed therefore writes one arm per tag it handles and a catch-all for the rest. An interceptor that handles a namespace, `Gui.*(..)`, is in the generic case inside that arm, which is right, since it does not know which tag it has.

### Answering a request

An interceptor that answers a request itself, a sandbox denying a read, a cache serving a hit, a mock returning canned data, does so the only way anything does: in a later occurrence. Prism cannot start an occurrence, so the interceptor fires a `Defer`, which every World provides, and answers when the `Defer` comes back:

```prism
Fs.File(path=p):
    !later <- @Element Defer(requests=(unit on _.!request))
    @ Denied on !later
```

`unit on _.!request` fires in the occurrence the child's request fires; the `Defer`'s request goes out with the turn; the World, after each occurrence, checks for fired `Defer` requests and, finding one, runs another occurrence of the same turn delivering `unit` to it; `!later` fires; `Denied on !later` fires; the block's result is consolidated into `!down`; the child's slot receives `Denied`. From the World's view it was one turn. From the child's view it asked and was answered later, exactly as it would have been by a World with a filesystem that refused.

Why not answer at once? The block for the child's slot sees the request fire, and could in principle compute `Denied on _.!request` and put that into `!down` in the same occurrence. Two things stop it. The creation rule is one: a response in the occurrence of the request means the element could have been a pure function, and the child is entitled to assume it was not. Routing is the other: if the child's slot was created in this occurrence, which is the common case for a request fired from `@started`, the path is not in the list as the occurrence opened, and the delivery is an unknown-path error. `Defer` is the one occurrence of delay that makes both go away, and it is why the rule can be a law for interceptors and not only for Worlds.

A `Defer` slot is answered at most once per turn. A slot that fires again after being answered in the same turn is an error, deferring backwards, and that is what bounds a turn: a child that retries at once on `Denied` makes the sandbox's `Defer` fire a second time in the same turn, and the World reports it rather than running forever. A retry belongs behind `Time.After`, where the World's clock separates the attempts. The error is a feature: it catches the tight retry loop in the occurrence it is written, with the path of the slot that caused it.

### What an interceptor may not do

The supplier's type says it returns a widget for every element. An interceptor that wants to deny a state element cannot return nothing; it must say what denial is, in the state's type, `@ $ Unavailable` for a file, `@ $ Disconnected "sandboxed"` for a socket. That is not an inconvenience. It is the rule that unavailability lives in the type, enforced at the one place it could be dodged: a child under a sandbox is shown a value the World could have shown it, and the child's `match` already has the arm.

A state tag no supplier and no World accepts is an error at the creation of the slot, because nothing later can give it a value. A request tag no World accepts gets `Unsupported` as its response in the next turn, so a `Ui.Button` under a CLI World is a program-visible fact rather than a silent nothing; an interceptor that forwards it unchanged forwards the `Unsupported` back down.

### Invariants

Stated once, with who is bound and what enforces each:

- *Nothing reaches past an interceptor unless Prism code sends it.* Bound: the runtime. Enforced by the structure of `intercept`: the child's list is returned as a value and read by nothing else.
- *Responses reach a child's slot only through the interceptor's response argument, routed against the opened list.* Bound: the interceptor. Enforced by the runtime's unknown-path check.
- *A state slot has its value in its creation occurrence, equal for every slot of that value under this interceptor.* Bound: the interceptor's supplier, which must return a widget for every element. Enforced by the runtime building one supplied widget per value and handing its dynamic to every slot.
- *No response in the occurrence of its request.* Bound: the interceptor. Enforced by routing for slots created this occurrence, and by convention, with `Defer` as the only tool, for older ones; the conformance suite's `mock_timing` test is the check.
- *An interceptor that forwards everything and supplies by forwarding is observably not there.* Bound: the runtime. Enforced by the differential test against `@Transparent`.
- *A supplied widget lives exactly as long as some child slot with its value.* Bound: the runtime. Enforced by ownership: the supplied widget is a child of the interceptor's node, keyed by value.

## Recapitulation and depth

An interceptor's own elements are what the next layer out sees, and a layer that wants to present several of its child's elements as one thing puts them inside a field of its own element, as data. A container's element then holds its children's elements; the outside addresses the container; and the container routes inward through its response argument. This is how depth works, and the convention a library adopts for it, a `children` field holding a `Path ==> Element` and a `ChildInput` response naming a path and a response for the slot there, is the library's, not the language's. The language guarantees only the two laws, routing against the opened list, and transparency of the forwarding interceptor; what a container looks like is up to whoever writes containers.

The convention has a consequence for tests. A Test World that wants `Expect Ui.Button(label="OK")` to find a button inside a box looks inside `children` fields because the library and the suite agree to, and `Send` to a nested slot wraps the input in one `ChildInput` per level, so that delivery takes the path a real World's input would. The suite knows the convention because the library is part of the suite, not because the language does. The next section builds that library.

## A GUI library as interceptors

Every toolkit you have used had layout, focus and hit-testing inside it, in the host language, next to the pixels, and the consequence was that a button was a native object: it had a position the toolkit assigned, a focus flag the toolkit set, and a click handler the toolkit called, and the three were visible to your program only through whatever accessors the toolkit chose to expose. The text box of the worked examples lost its cursor because the cursor was the toolkit's. This section builds the whole of a small toolkit, labels, buttons, text inputs, vertical and horizontal boxes, scrolling, focus and hit-testing, as Prism interceptors over five World-facing tags, so that there is nothing native between a program's button and the pixel except the drawing of the pixel. The library is the demonstration that the language is enough, and it is also the convention the conformance suite's Test World uses to look inside containers.

### The five tags

A World that draws accepts a window, a scene, a focusable, a text measurer and a frame clock. The first three are declared in the section on tags above. The other two, and the plain values all five use, are these:

```prism
type Point = record:
    x: Float
    y: Float
type Extent = record:
    w: Float
    h: Float
type Rect = record:
    x: Float
    y: Float
    w: Float
    h: Float
type Color = choice White | Grey | Blue | Black

type Draw = choice:
    Rect ((Rect, Color))
    Text ((Point, Text))
    Image ((Rect, Text))
    Clip ((Rect, [Draw]))

type Key = choice:
    Char Char
    Left
    Right
    Backspace
    Tab
    Enter

type WindowInput = choice:
    Pointer Point
    Pressed Key
    Wheel ((Point, Float))
    Resized Extent

type Frame = record:
    now: Instant

tag Gui.Metrics of Element:                  # state: the size of a text, answered synchronously
    text: Text
    font: Font                               # a plain value; Default unless the program names one
    state: Extent

tag Gui.Frames of Element:                   # stream: one Frame per displayed frame while a slot exists
    responds: Frame
```

The window has no dynamic field, so its slot is stable for the life of the program: the World has one path to send pointer, key, wheel and resize events to. Its `size` field is the size the program asks for at the start; the size the platform actually gives arrives as `Resized`, like any other input. The scene is a presence element whose `draw` field is the picture. A change to the picture is a fresh `Gui.Scene` slot, which the World draws, and nothing is in flight on a picture, so a fresh slot per change is right. Keeping the window and the scene apart is what lets the window have one path while a scene that changes sixty times a second is sixty presence elements.

Draw commands are values inside the scene, not elements. Suppose each rectangle and each run of text were a slot of its own: a window of two thousand glyph runs would be two thousand paths, each with an identity nobody addresses, and a redraw would be two thousand teardowns and builds. As one field, the scene is one slot, the World reads the list in order, which is paint order, and `Draw.Clip` nests a list inside a rectangle. A command has no response and no state, so nothing about it needs a slot.

The measurer is a state element for the reason the whole second half of this chapter gives. A layout needs the width of a text in the occurrence it lays out. As a request, the width would arrive a turn later, and the scene would be drawn once with the wrong width and once with the right one. As state, `Gui.Metrics(text="Ada", font=Default)` has its `Extent` in the occurrence the slot is created, and every slot asking about the same text and font shares one answer. `Font` is a plain value naming a family and size, declared with the other values in Appendix D, and `Default` is the font a program gets when it names none. The World owns the fonts and the library never sees them.

A World that offers these five can run every program in this section. A software rasteriser, a GTK-backed World that paints into a drawing area, a web World that turns `Draw` values into canvas calls, and a native Prism system that speaks X11 over a socket are all the same World as far as the library is concerned. A World that would rather offer native widgets accepts `Ui.*` tags or `Gtk.Button` and friends directly, and a different library targets it; the program above both libraries is the same program.

### The library's own tags

Between a program and the window, the library has elements of its own, declared under `Ui` to keep visible whose they are. `Ui.Label` and `Ui.Button` are declared above. The rest:

```prism
type ChildInput = record:
    path: Path
    response: Response

tag Ui.TextInput of Element:
    name: Text
    text: Text
    cursor: Nat
    responds: Key

tag Ui.VBox of Element:
    children: Path ==> Element
    responds: ChildInput

tag Ui.HBox of Element:
    children: Path ==> Element
    responds: ChildInput

tag Ui.Scroll of Element:
    name: Text
    height: Float
    children: Path ==> Element
    responds: ChildInput
```

The window interceptor claims every one of these. A World that accepts them natively could take them directly, and the Test World accepts every tag, which is how the suite looks inside the library.

Containers hold their children as data in a `children` field, keyed by the child's path, which is the *children convention*. A `Ui.VBox`'s `children` is the map of its children's elements, in tree order, and a container two levels up sees the whole subtree as one value. That is what the window needs to lay the tree out, and it is why the convention is a field and not an interceptor-to-interceptor protocol: the tree is plain data, and layout is a plain function over it.

Every container responds with `ChildInput`: a path in the child's list and a response for the slot there. `Response` is the generic type of `intercept`'s response argument, so a container delivers whatever it is given without knowing what the child is, and a value of any tag's response type is a `Response`, checked against the slot's tag when it arrives. A button's slot gets a `Click`, a text input's a `Key`, another container's a `ChildInput`. Suppose instead a container named the response types of its children: a union of every leaf's type, or one type for all leaves that each leaf filters. A container written before a new leaf tag exists could not hold it, and every leaf would see inputs meant for others. With the generic `Response`, routing is one line per container, and the leaf is the one place that knows what it accepts.

### Leaves

```prism
@Label: fn $Text -> @unit = \@widget:
    @Element Ui.Label(text=$_)
    @ unit

@Button: fn $Text -> @!Click = \@widget:
    !click <- @Element Ui.Button(label=$_)
    @ !click
```

A label is one element with a dynamic text, which is a `dyn` with a fresh slot per change. A button is the same with a response, which is a `!Switch` over the generations. There is nothing else in a leaf; everything a leaf does belongs to the layers above it.

### Containers

A container is an interceptor. It captures its child's list, puts the library elements into a `children` field of one element of its own, forwards everything else unchanged, and routes `ChildInput` down to the child at the named path:

```prism
let IsUi: fn Element -> Bool = \|e| match e:
    Ui.*(..): True
    ignore: False

let UiOnly: fn Path ==> Slot -> Path ==> Element = \|slots|
    MapValues (Filter slots, \IsUi _.element), \_.element

let NonUi: fn Path ==> Slot -> Path ==> Slot = \|slots|
    Filter slots, \not IsUi _.element

let Inward: fn ChildInput -> Path ==> Response = \|c| {c.path: c.response}

@VBox: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !in <- @Element Ui.VBox(children=(UiOnly $slots))
    !fwd <- @Forward (NonUi $slots)
    let !down = !Union (Inward !in), !fwd
    @ result

@Scroll: fn Text, Float, @a -> @a = \|name, height, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !in <- @Element Ui.Scroll(name=name, height=height, children=(UiOnly $slots))
    !fwd <- @Forward (NonUi $slots)
    let !down = !Union (Inward !in), !fwd
    @ result
```

`@HBox` is `@VBox` with `Ui.HBox`. `UiOnly $slots` is a derived dynamic of the child's library elements, kept current by the change law. `Ui.VBox(children=...)` with a dynamic field is a `dyn`, a fresh box per change of the children, and `!in` is the `!Switch` over its generations. When the window sends a `ChildInput` to the box, `Inward !in` is the one-entry map from the named path to the response, `!down` is that map merged with whatever the forwarded non-library elements received, and `intercept` delivers the response to the child's slot at that path, against the child's list as the occurrence opened. The child at that path is a leaf, which fires, or another container, which peels the next `ChildInput`.

The two maps never share a key, because `Inward !in` has library paths and `!fwd` has the others; `!Union` is there so that a click into the box and a file response to a forwarded `Fs.File` can land in one occurrence without one losing the other.

The supplier is `\|e| @Element e`: a box has no opinion about state and forwards a child's `Clock` or `Fs.Contents` by asking the outside for the same thing. A box that wanted to give its children a different clock would be a different interceptor with the same first line and a different third argument.

### Text input and focus

A text input is a leaf with a name, and the editing state sits in a hold outside the element:

```prism
@TextInput: fn Text, Text -> @$Text = \|name, initial| @widget:
    !key <- @Element Ui.TextInput(name=name, text=$edit.text, cursor=$edit.cursor)
    $edit <- (Edit initial, (Len initial)) <= Apply $edit, !key
    @ $edit.text
```

`Apply` is the plain function of the worked example with a cursor: it takes the old edit state and a key and returns the new state. The hold `$edit` is referenced by the element's fields before the line that defines it, which a widget block allows, and the cycle passes through the hold's value side.

Look at what changes per keystroke and what does not. The element's text and cursor change, so the element is a fresh slot; the hold `$edit`, with the cursor in it, is outside the `dyn`, so it persists. The slot being fresh per keystroke is why focus, in the window below, is keyed by the input's `name` and not by its path: the name is the one thing about a text input that does not change when you type into it. That is the same move as identifying state by value rather than by path, for the same reason. A path names a slot; a name names the thing the user is looking at.

Focus is a hold of an optional name in the window, and only there. Tab has to walk every input in tree order, and a click has to move focus from one input to another, so one owner sees all of them; an input that asked "am I focused" would need the window's answer anyway.

### Scrolling

A scroll region needs an offset, which wheel events move, and the layout needs the offset to place the children. The offset is state that no child reads, so the window holds it, in a map keyed by the region's `name` for the reason focus is: a `Ui.Scroll` is a fresh slot whenever its children change, and a list whose items come and go must not jump back to the top. `Ui.Scroll` is therefore a container like `Ui.VBox` with a name and a height. The next sections show where the offset is applied and where it is moved.

### Layout and hit-testing as plain functions

Layout is a function from an element tree, a rectangle and what the layout needs to know to a draw list and a list of hit regions. It needs the measured size of each text, the scroll offsets and the focus, bundled as `Env`:

```prism
type Env = record:
    sizes: Text ==> Extent       # one Gui.Metrics answer per distinct text
    offsets: Text ==> Float        # scroll offsets, by region name
    focus: ?Text

type HitKind = choice:
    Clickable
    Focus Text
    Scrollable ((Text, Float))     # region name, largest offset

type Hit = record:
    rect: Rect
    chain: [Path]                # from the window's child down to this element
    kind: HitKind

type Placed = record:
    draw: [Draw]
    hits: [Hit]

let TextExtent: fn Env, Text -> Extent = \|env, t|
    Or env.sizes[t], (Extent 0.0, 0.0)

let Measure: fn Env, Element -> Extent = \|env, e| match e:
    Ui.Label(text=t): Padded (TextExtent env, t), 4.0
    Ui.Button(label=l): Padded (TextExtent env, l), 8.0
    Ui.TextInput(text=t, ..): Padded (TextExtent env, t), 4.0
    Ui.VBox(children=cs): Fold cs, (Extent 0.0, 0.0), \|acc, ignore, c| Stacked acc, (Measure env, c)
    Ui.HBox(children=cs): Fold cs, (Extent 0.0, 0.0), \|acc, ignore, c| Beside acc, (Measure env, c)
    Ui.Scroll(children=cs, height=h, ..): Extent (Measure env, Ui.VBox(children=cs)).w, h
    ignore: Extent 0.0, 0.0

let Place: fn Env, Element, Rect, [Path] -> Placed = \|env, e, r, chain| match e:
    Ui.Label(text=t):
        Placed [[(Draw.Text (((Point r.x + 4.0, r.y + 4.0), t)))]], [[]]
    Ui.Button(label=l):
        Placed [[(Draw.Rect ((r, Grey))), (Draw.Text (((Point r.x + 8.0, r.y + 8.0), l)))]], [[(Hit r, chain, Clickable)]]
    Ui.TextInput(name=n, text=t, cursor=c):
        Placed (InputDraw env, r, n, t, c), [[(Hit r, chain, (Focus n))]]
    Ui.VBox(children=cs):
        PlaceStacked env, cs, r, chain, True
    Ui.HBox(children=cs):
        PlaceStacked env, cs, r, chain, False
    Ui.Scroll(name=n, children=cs, height=h):
        Clipped (PlaceStacked env, cs, (Up r, (Offset env, n)), chain, True), r, [[(Hit r, chain, (Scrollable ((n, (MaxOffset env, cs, h)))))]]
    ignore:
        Placed [[]], [[]]

# TextsIn: fn Path ==> Element -> Text ==> unit      every text the layout measures: labels, button labels, input texts, and each input's text up to its cursor
# PlaceStacked: fn Env, Path ==> Element, Rect, [Path], Bool -> Placed    children in key order, down or across
# InputDraw: fn Env, Rect, Text, Text, Nat -> [Draw]  a white box, the text, and a caret bar when the name is the focus
# Clipped: fn Placed, Rect, [Hit] -> Placed           a Draw.Clip around the draw list; hits cut to the rect, the region's own last
# Offset, MaxOffset, Up, Stacked, Beside, Padded: the arithmetic on offsets, sizes and rectangles their names say
```

`Or ?x, d`, as in `TextExtent`, is the optional default: `x` if there is one and `d` otherwise. Boolean or is the keyword `or`.

`PlaceStacked` walks the children in key order, which is tree order, which is top to bottom or left to right, allotting each its measured extent, placing it with its path appended to the chain, and concatenating the draw lists and hits with the children's hits *before* the container's own, so that the innermost region at a point is found first. `InputDraw` finds the caret's x by measuring the text up to the cursor, `TextExtent env, (Take t, c)`, which is why `TextsIn` lists that prefix: the library asks the World for the width of every text it will need, and `Gui.Metrics` answers all of them in the occurrence of the question. `Clipped` wraps the draw list in a `Draw.Clip` and discards hits outside the clip rect. `Stacked`, `Beside`, `Padded`, `Up` and the rest are the arithmetic their names say. None of this touches time: `Measure` and `Place` are functions on plain values, testable without a World, and when they are applied to a dynamic tree they lift to a dynamic picture.

Hit-testing is a lookup in the hit list, and turning a hit into an input is a fold over its chain:

```prism
let Contains: fn Rect, Point -> Bool = \|r, p|
    r.x le p.x and p.x lt r.x + r.w and r.y le p.y and p.y lt r.y + r.h

let HitAt: fn [Hit], Point -> ?Hit = \|hits, p|
    FirstWhere hits, \Contains _.rect, p

let Nest: fn [Path], Response -> Response = \|chain, r| match chain[0]:
    Some head: ChildInput(path=head, response=(Nest (Rest chain), r))     # Rest: the array without its first element
    Nil: r

let Deliver: fn [Path], Response -> Path ==> Response = \|chain, r| match chain[0]:
    Some head: {head: (Nest (Rest chain), r)}
    Nil: {}
```

`Deliver` turns a chain `[[box, row, button]]` and a `click` into `{box: ChildInput(path=row, response=ChildInput(path=button, response=click))}`: a response to the box, which the box peels and sends to the row, which the row peels and sends to the button. The map's one key is the first element of the chain, a path in the window's child's list, which is the key `!down` needs.

### The window

The window interceptor claims every library element, lays the tree out, draws it, hit-tests the pointer, keeps focus and scroll offsets, and routes keys and wheel events:

```prism
type Wheeled = record:
    name: Text
    most: Float
    by: Float

let PointerOf: fn WindowInput -> ?Point = \|w| match w:
    Pointer p: Some p
    ignore: Nil

let PressedOf: fn WindowInput -> ?Key = \|w| match w:
    Pressed Tab: Nil
    Pressed k: Some k
    ignore: Nil

let IsTab: fn WindowInput -> ?unit = \|w| match w:
    Pressed Tab: Some unit
    ignore: Nil

let WheelOf: fn WindowInput -> ?((Point, Float)) = \|w| match w:
    Wheel ((p, d)): Some ((p, d))
    ignore: Nil

let ResizeOf: fn WindowInput -> ?Extent = \|w| match w:
    Resized x: Some x
    ignore: Nil

let ClickFor: fn Hit -> ?(Path ==> Response) = \|h| match h.kind:
    Clickable: Some (Deliver h.chain, click)
    Focus ignore: Nil
    Scrollable ignore: Nil

let FocusOf: fn Hit -> ?Text = \|h| match h.kind:
    Focus n: Some n
    ignore: Nil

let NextFocus: fn [Hit], ?Text -> ?Text = \|hits, current|
    After (Collect hits, FocusOf), current   # the name after current in hit order, wrapping; Nil if none

let RouteKey: fn [Hit], ?Text, Key -> ?(Path ==> Response) = \|hits, focus, k|
    match focus:
        Nil: Nil
        Some n: match (FirstWhere hits, \(FocusOf _) == Some n):
            Some h: Some (Deliver h.chain, k)
            Nil: Nil

let WheelTarget: fn Hit, Point, Float -> ?Wheeled = \|h, p, d| match h.kind:
    Scrollable ((n, most)): if Contains h.rect, p: Some (Wheeled n, most, d) else: Nil
    ignore: Nil

let WheelAt: fn [Hit], ((Point, Float)) -> ?Wheeled = \|hits, ((p, d))|
    (Collect hits, \WheelTarget _, p, d)[0]  # hits are in tree order, children first: the innermost region under the pointer

let Clamp: fn Float, Float, Float -> Float = \|lo, hi, v|
    if v lt lo: lo else: if hi lt v: hi else: v

let Scrolled: fn Text ==> Float, Wheeled -> Text ==> Float = \|offsets, w|
    Insert offsets, w.name, (Clamp 0.0, w.most, (Or offsets[w.name], 0.0) + w.by)

@Window: fn Text, Extent, @a -> @a = \|title, size0, @app| @widget:
    ((result, $slots)) <- intercept @app, !down, $ \|e| @Element e
    !raw <- @Element Gui.Window(title=title, size=size0)
    let $kids = UiOnly $slots
    $sizes <- for (TextsIn $kids):               # a Gui.Metrics slot per distinct text
        $m <- @Element Gui.Metrics(text=key, font=Default)
        @ $m
    $size <- size0 <= !Filter (ResizeOf !raw)
    let $root = Ui.VBox(children=$kids)
    let $placed = Place (Env $sizes, $offsets, $focus), $root, (Rect 0.0, 0.0, $size.w, $size.h), [[]]
    @Element Gui.Scene(window=title, draw=$placed.draw)

    let !hit = !Filter (HitAt $placed.hits, !Filter (PointerOf !raw))
    $focus <- Nil <= (Some !Filter (FocusOf !hit)) | (NextFocus $placed.hits, $focus) on !Filter (IsTab !raw)
    $offsets <- {} <= Scrolled $offsets, !Filter (WheelAt $placed.hits, !Filter (WheelOf !raw))
    dyn $focus:
        match _:
            Some n: @Element Gui.Focusable(name=n)
            Nil: @ unit

    let !clicks = !Filter (ClickFor !hit)
    let !keys = !Filter (RouteKey $placed.hits, $focus, !Filter (PressedOf !raw))
    !fwd <- @Forward (NonUi $slots)
    let !down = !Union (!clicks | !keys), !fwd
    @ result
```

Walk the lines. `$kids` is the app's library elements, and `$root` is those elements as one tree, a derived dynamic. The `for` over the texts in the tree builds one `Gui.Metrics` slot per distinct text and consolidates their answers into `$sizes`; a text that appears in this occurrence has its slot built in this occurrence, and its answer is there too, so `$placed`, the layout of the tree at the window's current size with the current sizes, offsets and focus, is consistent in every occurrence in which anything changed. The scene element draws its list, a fresh slot per change. The window's raw input is split by plain filters. A pointer is hit-tested against `$placed.hits` *read through a tag*, the layout as the occurrence opened, which is the layout the user was looking at when they clicked. A hit on a button becomes a `Deliver` addressed to the app's slot at the head of the chain; a hit on an input moves focus and delivers nothing, since an input responds only to keys. Focus is a hold of an optional name, moved by a click on an input or by Tab, which steps through the input names in tree order. A key is routed to the input whose name matches the focus as the occurrence opened, again through tags. A wheel event moves the offset of the innermost scroll region under the pointer, clamped to the largest offset that region's content allows. The `Gui.Focusable` is the window telling the World which name has focus, rebuilt whenever the hold changes. The window delivers one `WindowInput` per occurrence, so `!clicks | !keys` loses nothing. Everything that is not a library element is forwarded, and `!down` is the union.

There is no second pass, no layout phase, no deferred focus. Hit-testing is a lifted operation over a dynamic and an event, so it runs in the input's occurrence, against the list the user saw, which is the routing rule. The scene is derived, so it changes promptly in any occurrence in which anything on the screen changed, and the World draws it after the turn. Focus moving to an input built in this occurrence cannot happen in this occurrence, because that input's name is not in `$placed.hits` as the occurrence opened; it happens the next time the user presses Tab or clicks, which is the only time focus moves anyway. A program that wants an input focused as soon as it appears puts the name into the window's focus through a request element, which the window answers with a `Defer`, and that is a few more lines in the same language.

### A form, occurrence by occurrence

Take a small form under the window:

```prism
@form:
    $name <- @TextInput "name", "Ada"
    !ok <- @HBox @widget:
        !ok <- @Button $ "OK"
        !cancel <- @Button $ "Cancel"
        @ !ok
    @Label "Hello, {$name}"
    @ unit

@main:
    @Window "Form", (Extent 400.0, 300.0), (@VBox @form)
```

Paths, from the window's child down. The `@VBox` is the window's child, and it holds everything back, so the app's list has one library element, the box's `Ui.VBox`, at `1/#0/0`. Inside the box, the form's elements: the text input at `0/#0/0`, the HBox's element at `1/#0/0`, the greeting label at `2/#0/0`. Inside the HBox, the two buttons at `0/#0/0` and `1/#0/0`. A path is only a key in the list of the interceptor that reads it, so the same string names different slots at different levels.

The World's text measurement is fixed-width in this walk: eight pixels per character and sixteen per line. A label pads its text by 4 on each side, a button by 8, an input by 4, which puts the input at the top left with width 32 and height 24, the two buttons below it from y = 24, `OK` from x = 0 to 32 and `Cancel` from x = 32 to 96, each 32 high, and the greeting at y = 56, 88 wide.

**Occurrence 0 (build).** Everything is built. The `Gui.Window` slot is created. `TextsIn` finds four distinct texts, `Ada` (the input's text and its text up to the cursor), `Cancel`, `Hello, Ada` and `OK`, and the `for` builds a `Gui.Metrics` slot for each; the oracle answers `Extent 24.0, 16.0` for `Ada` and so on, in this occurrence. `Place` lays the tree out with those sizes, and the scene slot is created with the draw list. `$focus` is `Nil`, so no focusable. The World sees:

```
1:               Gui.Window(title="Form", size=Extent(w=400.0, h=300.0))
2/[Ada]/0:       Gui.Metrics(text="Ada", font=Default)
2/[Cancel]/0:    Gui.Metrics(text="Cancel", font=Default)
2/[Hello, Ada]/0: Gui.Metrics(text="Hello, Ada", font=Default)
2/[OK]/0:        Gui.Metrics(text="OK", font=Default)
4/#0/0:          Gui.Scene(window="Form", draw=[[...]])
```

It draws the scene and waits.

**Occurrence 1.** The user clicks on the OK button. The World sends `Pointer (Point 12.0, 40.0)` to `1`. `PointerOf` passes the point; `HitAt` reads `$placed.hits` through a tag and finds the button's region first (leaves before containers), with chain `[[1/#0/0, 1/#0/0, 0/#0/0]]`: the box in the app's list, the HBox in the box's list, the OK button in the HBox's list. `ClickFor` turns it into `{1/#0/0: ChildInput(path=1/#0/0, response=ChildInput(path=0/#0/0, response=click))}`. `!down` fires with it; `intercept` delivers to the box's slot at `1/#0/0` in the app's list as opened. The box's `!in` fires; `Inward` turns the `ChildInput` into `{1/#0/0: ChildInput(path=0/#0/0, response=click)}`; the box's `!down` delivers it to the HBox's slot; the HBox peels again and delivers `{0/#0/0: click}`; the OK button's `!in` fires with `click`; `!ok` fires in the form. Nothing on the screen changes, so no scene is rebuilt. Three interceptors and one leaf, in one occurrence, each one line of routing.

**Occurrence 2.** The user clicks in the text field. `Pointer (Point 12.0, 10.0)` to `1`. `HitAt` finds the input's own region, with kind `Focus "name"`. `ClickFor` gives `Nil`, since an input responds only to keys, and `FocusOf` fires `Some "name"`; `$focus` changes to `Some "name"`. The layout depends on the focus, so `$placed` changes promptly and the scene rebuilds to `4/#1/0`, now with a caret bar after `Ada`; and the focus dyn builds `Gui.Focusable(name="name")` at `7/#1/0`. Commit: focus is the input, and the World can show the platform's focus for it.

**Occurrence 3.** The user types `!`. `Pressed (Char '!')` to `1`. `PressedOf` passes it; `RouteKey` reads `$placed.hits` and `$focus` through tags, finds the hit whose kind is `Focus "name"`, and `Deliver` addresses `Char '!'` down its chain `[[1/#0/0, 0/#0/0]]`. The box peels; the input's `!key` fires; `Apply $edit, !key` fires with `Edit "Ada!", 4`; `$edit` changes. The input element is a fresh slot at `0/#1/0`; the box's `children` changes and the box is a fresh slot at `1/#1/0`; `$kids` changes; the greeting label changes too, since `$name` is `$edit.text`. Now the measurer: `TextsIn` finds `Ada!`, `Cancel`, `Hello, Ada!` and `OK`. The `for` builds `Gui.Metrics` slots for `Ada!` and `Hello, Ada!` in this occurrence and the oracle answers them in this occurrence; `Ada` and `Hello, Ada` are torn down at commit. `$sizes` has the new widths before the occurrence ends, `$placed` is laid out with them, and the scene is rebuilt at `4/#2/0`. There is no occurrence in which the text has changed and its width has not. Commit: `$edit` is `Edit "Ada!", 4`, and `$focus` is still `Some "name"`, which names the new input slot as well as it named the old one.

**Occurrence 4.** The user presses Tab. `IsTab` passes; `NextFocus` reads the hit list and the focus through tags; there is one input, so the name after `"name"`, wrapping, is `"name"` again. A hold set to the value it already has still changes, so the scene and the focusable are rebuilt with equal contents; the World's diff finds nothing different, and `$Distinct` on `$focus` would suppress the rebuild. A form with two inputs would move to the second here, and Tab on the last would wrap to the first, which is what `After` does; a library that wants Shift-Tab adds a key and a `Before`.

**Occurrence 5.** The user resizes the window. The World sends `Resized (Extent 600.0, 300.0)` to `1`. `$size` changes; `$placed` changes promptly; the scene rebuilds with the new rectangles. Nothing in the form ran, and the next click is hit-tested against the new layout because the next occurrence opens with it.

This prevents the afternoon where the click landed on the button that had just moved, because layout ran after the event was dispatched and the hit-test used the new positions while the user had seen the old ones. Here the hit-test reads the layout the user saw, by the tag law, and no layout pass can get between a click and the thing it was aimed at.

### Tests that look inside

A `Ui.VBox` holds its children as data, so the Test World can look inside it. `Expect Ui.Button(label="OK")` matches a slot at any depth because the suite's matcher descends into `children` fields, and `Send Ui.Button(label="OK"), click` wraps `click` in one `ChildInput` per level and delivers it to the outermost slot on the chain, which is the same delivery a window would make after a hit-test. The Test World knows the convention because the library is part of the suite, not because the language does; a library that nested differently would ship a different matcher.

A library test of the form above, with no window at all:

```prism
test form_greets: Test (@VBox @form): array:
    Expect Ui.Label(text="Hello, Ada")
    Send Ui.TextInput(name="name", ..), Char '!'
    Expect Ui.Label(text="Hello, Ada!")
    Send Ui.Button(label="OK"), click
```

The test drives the library below the window, so it checks routing through the boxes and the input without any layout. A test of the window adds the measurer, which the Test World has no answers for. The window creates its `Gui.Metrics` slots at build, so the script gives each one a value before the build:

```prism
test window_focus: Test (@Window "Form", (Extent 400.0, 300.0), (@VBox @form)): array:
    State Gui.Metrics(text="Ada", font=Default), Extent 24.0, 16.0
    State Gui.Metrics(text="Cancel", font=Default), Extent 48.0, 16.0
    State Gui.Metrics(text="Hello, Ada", font=Default), Extent 80.0, 16.0
    State Gui.Metrics(text="OK", font=Default), Extent 16.0, 16.0
    Send Gui.Window(..), Pointer (Point 12.0, 10.0)
    Expect Gui.Focusable(name="name")
```

`Place` is a plain function, so most of the window's behaviour is a unit test of `Place` and `HitAt` against a tree and a map of sizes.

### What the library does not do, yet

Text longer than its box needs wrapping, which is line breaking over widths: the library asks `Gui.Metrics` for the prefixes it needs, as `InputDraw` does for the caret, and breaks where the widths say. A library with more than one font passes the font along: `Gui.Metrics` already has the `font` field, the draw commands gain one, and `$sizes` is keyed by text and font. Size feedback, where a child wants to be a different size because of the size it was given, is a cycle through the layout and needs a `Defer` to break it; the library above has no such child. An accessibility tree is a derived dynamic over the same element tree the layout uses, with the input names as node ids, emitted as an element of its own for a World that speaks to a screen reader. Animation is a `Gui.Frames` slot inside a `dyn` over whether anything is moving, so that a window at rest asks the World for no frames, and each `Frame` is an input to a hold. Each is more Prism over the same five tags, and none is a new primitive.

## Worked interceptors

Each of the following is a complete interceptor with a short walkthrough. They are presented in order of how much they do beyond forwarding, and every one of them is `@Forward` with a different `match`. The point of the sequence is that nothing in it needed a mode of `intercept`.

### Transparent forwarding

`@Transparent` from the section on `intercept` is the identity: every slot re-emitted with its request wired in, every response routed back under the child's path, every state element supplied by asking outside for the same value. A child under it has the same transcript it would have without it, one level deeper in the outer list. It is in the conformance suite because it is the law every other interceptor is measured against, and it is in the library because a wrapper that wants to observe without altering starts from it.

### A path-rewriting sandbox

```prism
let Confine: fn Text, Text -> ?Text = \|root, p|
    if (StartsWith p, "/home/"): Some (Concat [[root, p]])
    else: Nil

@Jail: fn Text, @a -> @a = \|root, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ supply
    !down <- for $slots:
        dyn $_:
            match _.element:
                Fs.File(path=p):
                    match Confine root, p:
                        Some q:
                            !resp <- @Element Fs.File(path=q, requests=_.!request)
                            @ !resp
                        Nil:
                            !later <- @Element Defer(requests=(unit on _.!request))
                            @ Denied on !later
                Fs.Dir(..):
                    @ !never                            # a state element: supplied below, not forwarded here
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ !resp
    let supply = \|e| match e:
        Fs.Contents(path=p): match Confine root, p:
            Some q: @Element Fs.Contents(path=q)
            Nil: @ $ Unavailable
        Fs.Dir(path=p): match Confine root, p:
            Some q: @Element Fs.Dir(path=q)
            Nil: @ $ Unlisted
        ignore: @Element e
    @ result
```

Two directions, two policies. Requests to files under `/home/` are forwarded with the path rewritten under the root, and the response comes back under the child's path; requests to anything else are denied, one occurrence later, through a `Defer`. State for files under `/home/` is forwarded with the same rewrite; state for anything else is a constant, `Unavailable` for contents and `Unlisted` for a listing, each a case of the type the child already matches on. The `Fs.Dir` arm in the response loop returns `!never` because a state element has no responses; it is there so that the catch-all does not put a second, unconfined `Fs.Dir` element in the outer list beside the one the supplier makes. Everything else goes through untouched.

The child cannot tell the difference between this and a World whose filesystem is rooted at `root` and refuses everything outside `/home/`. That is the test of the primitive. The worked example that runs a file viewer under a jail walks the build occurrence, a forwarded read's answer coming back, a confined path getting a constant, and a denial arriving in the second occurrence of a turn.

### A denial sandbox

A sandbox that denies a whole namespace needs no rewrite:

```prism
let Forbidden: fn ?HttpRequest -> ?HttpResponse = \|asked| match asked:
    Some r: Some (HttpResponse r, 403, "sandboxed")
    Nil: Nil

@NoNetwork: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ supply
    !down <- for $slots:
        dyn $_:
            match _.element:
                Net.Http(..):
                    !later <- @Element Defer(requests=(unit on _.!request))
                    $asked <- Nil <= Some _.!request
                    @ !Filter (Forbidden $asked on !later)
                Net.Socket(..):
                    @ !never
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ !resp
    let supply = \|e| match e:
        Net.Socket(..): @ $ Disconnected "sandboxed"
        ignore: @Element e
    @ result
```

Nothing under `Net` reaches the outside. An HTTP request is answered with a 403 in the next occurrence of the turn; the response has to name the request it answers, so the block remembers the request in a hold across the `Defer` and reads it back through a tag when the answer comes. A socket's state is `Disconnected "sandboxed"` from its first occurrence, which is what a socket to an unreachable host looks like; its sends go nowhere and it never receives, which is also what that socket looks like. The child's `match` on `Link` already has the arm.

The strong objection to this whole arrangement is that forwarding by default gives every widget the World's full access unless something above it says otherwise. That is true, and it is the same as `std::fs` in any language. What the language adds is confinement per subtree, in the program, without a process boundary, so a parent can confine a child it did not write; and the guarantee that there is nothing else, no global, no context, no ambient channel, that the confinement could miss. The standard library should ship a sandbox, and the guidance for third-party widgets should be to wrap them. As in any capability system, an interceptor above the sandbox must not turn the child's data into authority: a sandbox that forwards `Fs.File(path=p)` for any `p` the child chooses has confined nothing.

### A cache for state

A child that watches a file sees every change the World pushes. A cache shows it the value at start and at a refresh, and nothing in between:

```prism
let Freeze: fn a -> $a = \|v| $ v

@Cached: fn !unit, @a -> @a = \|!refresh, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ supply
    !down <- @Forward $slots
    let supply = \|e| match e:
        Fs.Contents(path=p): @widget:
            $live <- @Element Fs.Contents(path=p)
            !started <- @started
            $$shown <- $live <= (Freeze $live) on (!started | !refresh)
            @ $Join $$shown
        ignore: @Element e
    @ result
```

The supplier returns a widget, and the widget has state. For an `Fs.Contents` element it builds, in the cache's node, a live slot for the same path, asking the outer context; and a hold of dynamics `$$shown`, initially the live dynamic itself, replaced by a frozen constant of the live value at start and at each refresh. `(Freeze $live) on !started` fires in the supplied widget's build occurrence with the constant dynamic of `$live`'s value before that occurrence, which is the oracle's answer; the hold adopts it in that occurrence; `$Join $$shown` has, before the occurrence, `$live`'s value, and after it, the frozen constant with the same value. The child's slot sees one value in its creation occurrence and no hiccup. From then on pushes move `$live` and not `$$shown`, until a refresh freezes the live value again.

The cache is also a cache in the ordinary sense. The supplied widget is built once per distinct value, so two child slots for `/etc/app.conf`, in two widgets that have never heard of each other, read one `$Join $$shown` from one live slot, and the outer World is asked once. That is the equality law inside an interceptor and the sharing of interest in one mechanism.

The tag law shows up when a push and a refresh land in one occurrence: `(Freeze $live) on !refresh` reads `$live` before the occurrence, so the cache shows the value before the push and the next refresh shows the pushed one. A cache that wanted the pushed value in the same occurrence as the click cannot have it, because the only window onto a dynamic at an event is the value before, and that is the same rule everywhere else.

### A cache for requests

A request cache remembers answers and serves repeats without going outside:

```prism
let Lookup: fn Text ==> HttpResponse, HttpRequest -> ?HttpResponse = \|memo, r|
    if r.method == "GET": memo[r.url] else: Nil

let Missing: fn Text ==> HttpResponse, HttpRequest -> ?HttpRequest = \|memo, r|
    match Lookup memo, r:
        Some ignore: Nil
        Nil: Some r

let Remember: fn Text ==> HttpResponse, Path ==> HttpResponse -> Text ==> HttpResponse = \|memo, answers|
    Fold answers, memo, \|acc, ignore, a| Insert acc, a.request.url, a

type CacheOut = record:
    !resp: Response                              # what goes down to the child
    !learned: HttpResponse                       # what came back from outside

@HttpCache: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    out <- for $slots:
        dyn $_:
            match _.element:
                Net.Http(..):
                    let !hit = !Filter (Lookup $memo, _.!request)
                    let !miss = !Filter (Missing $memo, _.!request)
                    !fetched <- @Element Net.Http(requests=!miss)
                    !later <- @Element Defer(requests=(unit on !hit))
                    $pending <- Nil <= Some !hit
                    @ CacheOut (!fetched | !Filter ($pending on !later)), !fetched
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ CacheOut !resp, !never
    let !down = out.!resp
    $memo <- {} <= Remember $memo, out.!learned
    @ result
```

Each child `Net.Http` slot gets a block. When its request fires, the block reads `$memo` through a tag, the cache as the occurrence opened. A miss goes out on the block's own `Net.Http` element and comes back as `!fetched`, under the child's path. A hit is remembered in `$pending` and answered through a `Defer`, so that it arrives in a later occurrence like a real answer would. The block returns a record, which `for` consolidates field by field: `out.!resp` is the map of responses going down, and `out.!learned` the map of answers that came from outside, from which `$memo` is held.

`$memo` is read by the blocks whose answers define it, and that is a legal cycle because the read is a tag. Two identical GETs in one occurrence from two child slots both miss, because neither sees the other's answer before the occurrence, and both go out. A cache that wanted to coalesce them would key an outstanding-requests hold by URL and attach the second slot to the first's answer, which is a longer program in the same language.

The conformance suite's `cache_hit_no_request` test puts two identical GETs in two turns under this interceptor and checks that the second turn has `Unrequested Net.Http(..)` at the top and that the child still received an answer, in a later occurrence than its request.

### A mock for tests

```prism
@MockHttp: fn (fn HttpRequest -> HttpResponse), @a -> @a = \|answer, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- for $slots:
        dyn $_:
            match _.element:
                Net.Http(..):
                    !later <- @Element Defer(requests=(unit on _.!request))
                    $asked <- Nil <= Some _.!request
                    @ !Filter (answer $asked on !later)
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ !resp
    @ result
```

Every HTTP request in the subtree is answered from a pure function, through a `Defer`, and nothing in `Net` reaches the outside. `answer $asked on !later` lifts the plain function over a dynamic and an event: it fires when the `Defer` answers, with the function applied to the remembered request. The mock is for previews and for tests of a widget's request logic that should not depend on a Test World's fake network. Because the answer goes through a `Defer`, a widget under the mock sees the timing it would see in production: never in the occurrence it asked. The suite's `mock_timing` test checks exactly that; it catches the mocks that hide a timing bug the real World would have shown.

A mock for state is the supplier alone:

```prism
@MockFs: fn Text ==> Text, @a -> @a = \|files, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ supply
    !down <- @Forward $slots
    let supply = \|e| match e:
        Fs.Contents(path=p): match files[p]:
            Some t: @ $ Known t
            Nil: @ $ Unavailable
        ignore: @Element e
    @ result
```

A child under `@MockFs {"/etc/motd": "hi"}` reads `Known "hi"` in the occurrence it asks and `Unavailable` for anything else, with no World involved. A test that wants the file to change mid-test wraps the map in a dynamic and reads it in the supplied widget; the map's change is then a push, as far as the child can tell.

### A logger

```prism
tag Log.Line of Element:
    requests: !Text

@Traced: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- for $slots:
        dyn $_:
            ((!resp, ignore)) <- @Element _.element
            @Element Log.Line(requests=("{ShowPath key} asked {ToText _.!request}" | "{ShowPath key} got {ToText !resp}"))
            @ !resp
    @ result
```

Forward everything and, per slot, fire a log line for each request and each response. `key` is the child's path, so the line says which slot spoke; interpolating an event into a text literal lifts the literal to an event of text, so each side of the `|` fires when its event does. The two cannot coincide for one slot, since a response is never in the occurrence of its request, so `|` is exact. The log lines are `Log.Line` requests in the tracer's own list, which the World performs in list order after the occurrence, so a transcript reads in the order the program spoke. Nothing about the child's timing changes, because the tracer added no occurrences.

### A quota

```prism
let IfUnder: fn Nat, Nat, HttpRequest -> ?HttpRequest = \|used, limit, r|
    if used lt limit: Some r else: Nil

let IfOver: fn Nat, Nat, HttpRequest -> ?HttpRequest = \|used, limit, r|
    if used lt limit: Nil else: Some r

let Refusal: fn ?HttpRequest -> ?HttpResponse = \|asked| match asked:
    Some r: Some (HttpResponse r, 429, "quota")
    Nil: Nil

@Quota: fn Nat, @a -> @a = \|limit, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !asked <- for $slots:                        # every HTTP request fired this occurrence, by path
        dyn $_:
            match _.element:
                Net.Http(..): @ _.!request
                ignore: @ !never
    $used <- 0 <= $used + Size !asked           # Size: the prelude's count of a map's entries
    !down <- for $slots:
        dyn $_:
            match _.element:
                Net.Http(..):
                    let !allowed = !Filter (IfUnder $used, limit, _.!request)
                    let !refused = !Filter (IfOver $used, limit, _.!request)
                    !fetched <- @Element Net.Http(requests=!allowed)
                    !later <- @Element Defer(requests=(unit on !refused))
                    $held <- Nil <= Some !refused
                    @ !fetched | !Filter (Refusal $held on !later)
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ !resp
    @ result
```

Forward until a count is reached and refuse after. Two `for`s over the same list: the first collects every HTTP request fired in an occurrence into one map, whose size moves the count; the second does the routing, with each block reading the count through a tag. Two requests in one occurrence both see the count as it opened and both pass if it was under the limit, so the quota can overshoot by the number of simultaneous requests; a quota that must be exact folds over the consolidated map and lets the leftmost through, which is a change to the first `for` and not to the primitive. A refusal is a 429 through a `Defer`, so the child sees it in a later occurrence like any other answer.

### Migration between two outer contexts

An interceptor has one outer context, its own node's list, but that list can hold elements of two kinds, and a dynamic can choose between them:

```prism
type Backend = choice Local | Remote

tag Kv.Get of Element:
    requests: !Text
    responds: ?Text

tag Kv.Store of Element:
    key: Text
    state: ?Text

tag Remote.Get of Element:
    host: Text
    requests: !Text
    responds: ?Text

tag Remote.Store of Element:
    host: Text
    key: Text
    state: ?Text

let IfLocal: fn Backend, Text -> ?Text = \|b, r| if b == Local: Some r else: Nil
let IfRemote: fn Backend, Text -> ?Text = \|b, r| if b == Remote: Some r else: Nil

@Migrate: fn $Backend, Text, @a -> @a = \|$backend, host, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $supply
    !down <- for $slots:
        dyn $_:
            match _.element:
                Kv.Get(..):
                    let !local = !Filter (IfLocal $backend, _.!request)
                    let !remote = !Filter (IfRemote $backend, _.!request)
                    !a <- @Element Kv.Get(requests=!local)
                    !b <- @Element Remote.Get(host=host, requests=!remote)
                    @ !a | !b
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ !resp
    let $supply = Supplier $backend, host
    @ result

let Supplier: fn Backend, Text -> (fn Element -> @$State) = \|backend, host| \|e| match e:
    Kv.Store(key=k): match backend:
        Local: @Element Kv.Store(key=k)
        Remote: @Element Remote.Store(host=host, key=k)
    ignore: @Element e
```

Requests are routed per firing: each `Kv.Get` request reads `$backend` through a tag and goes out on one of two elements, both of which exist in the interceptor's list for the life of the child's slot, and the answer comes back from whichever was asked. State is routed per slot: the supplier is a *dynamic* of a function, `Supplier $backend, host` lifted over `$backend`, so a `Kv.Store` slot created while the backend is `Local` is supplied by a local store element and keeps it, while a slot created after the backend changes to `Remote` is supplied by a remote one. A value already being served keeps its supplied widget whatever the backend says, by the equality law, so two child slots for one key never disagree; and when the last slot for a key goes and a new one comes under the new backend, the key migrates. That is the behaviour a migration wants: existing readers see a consistent store, new readers see the new one, and the cutover is per key rather than per program.

This is the one interceptor in the chapter that passes a non-constant supplier, and it shows what the dynamic is for. Policy over time is a dynamic like any other; the runtime reads it at the instant a value first needs a supplier and never again for that value.

## A Prism-hosted World

A World can be written in Prism: a program whose children are other programs, each under its own interceptor, with the host deciding what each one sees and what reaches the World below. The shell of the worked examples shows two apps sharing one screen. Here is a host with a process table, so that the set of programs is a keyed collection that changes while the host runs:

```prism
type Launch = choice:
    Start Text
    Stop Text

let ApplyLaunch: fn Text ==> unit, Launch -> Text ==> unit = \|table, l| match l:
    Start name: Insert table, name, unit
    Stop name: Remove table, name

@Host: fn Text ==> @unit -> @unit = \|programs| @widget:
    $clock <- @Element Clock
    let supply = \|e| match e:
        Clock: @ $clock
        Process.Args(..): @ $ [[]]
        ignore: @Element e
    !launch <- @Launcher (Keys programs)              # a menu returning Start name or Stop name
    $table <- {} <= ApplyLaunch $table, !launch
    $lists <- for $table:                             # key is the program's name
        match programs[key]:
            Some @app:
                ((ignore, $slots)) <- intercept @app, !down, $ supply
                !down <- @Forward (NonUi $slots)
                @ $slots
            Nil:
                @ $ {}
    @Window "Host", (@Tiled $lists)
    @ unit
```

Each entry in the process table is a `for` block, keyed by the program's name, that builds the named program under an `intercept`, forwards everything that is not GUI, and returns the program's element list as a dynamic; the `for` consolidates those into `$lists : $(Text ==> (Path ==> Slot))`, one list per running program, which a tiling interceptor lays out side by side and routes clicks into by the same `Into` convention as a box. The `match` inside the block chooses between two widgets by a plain value, `programs[key]`, which is the one form a `match` with widget arms may take. The supplier gives every program the host's own clock, so there is one clock across the host and all its guests, and an empty argument list; everything else is forwarded. A `Stop` removes the key, the block is torn down, the program's slots leave the outer list, and the World below releases whatever it was doing for them.

From a guest's side, the host is its World: it sees buttons and labels answered, a clock with a value, files it asked for. From the World below, the host is one program with a window, a clock and some forwarded file reads. A guest that wants to escape has no way to: it cannot name the host, cannot reach another guest, cannot find a global, and every element it makes passes through `supply` or the `for` block's `match` before anything outside knows it exists. That is what a World is, and the only difference between this one and the Rust one below it is that this one has an outside to forward to.

The clock deserves a second look because it is the host's one piece of policy. `supply` returns `@ $clock` for every guest's `Clock`: not an element, the host's own dynamic, so the guests' clocks are not even forwarded, they are the host's clock read directly. A host that wanted to run a guest at half speed, or frozen for a debugger, would return a different dynamic for that guest, and the guest would have no way to know. That is the sandbox argument applied to time, and it is why there is no `now()`.

## Negative space

The boundary is defined as much by what it refuses as by what it offers. Each refusal below closes a hole, and each one also lets the implementation, and every World, leave out a mechanism.

**No dynamic fields in elements.** An element is born with its fields and keeps them; a label whose text changes is a new label at the same spot. What it buys: the element list is plain data, so a World diffs maps instead of subscribing to anything, value identity never has to pick a moment, a response is always to the slot that asked, and the runtime has no subscription object that crosses the boundary and no notification that a field moved. The cost is a fresh slot per change, and the table in the section on dynamic fields is the argument that the cost is right in every case but one, and that the one has a better answer in the library.

**No request on creation.** Creating an element never, by itself, makes anything happen. What it buys: building is pure, so a rebuild costs exactly what it asks for and a `for` key appearing is not an effect; the trigger is a value the program can merge, gate and mock; there is one kind of request element, so an interceptor answers requests and nothing else; and in the occurrence a widget is built nothing can depend on an effect having already happened, because none has. The World core never has to watch for slots appearing, only for request events firing.

**No implicit forwarding.** An interceptor's child's elements reach the outside only if the interceptor makes elements for them. What it buys: a sandbox is a `match` whose catch-all arm is readable, and the question "can this child reach the network" is answered by reading it; a container holds back what it lays out and forwards the rest with one line; and the primitive has no table, no modes, and no second set of rules for when the table is consulted. The cost is that the shortest interceptor is seven lines rather than one, and the library ships the seven.

**No reaching past an interceptor.** Nothing a child creates goes further out by itself; nothing further out can address a child's slot except through the interceptor's response argument; a child cannot name its interceptor, its siblings, or anything above it. What it buys: the interceptor is a capability boundary with no back door, the Test World can stand in for any World because a program cannot tell who is answering, and a program that hosts programs is an ordinary program.

**No ambient context.** No globals, no "the current World", no environment read from the middle of a function, no clock. A widget has its arguments, its scope, and its elements. What it buys: there is no channel an interceptor does not see, so confinement is complete; arguments, environment and time are state elements, so a sandbox can replace them and a test can set them; and there is no implicit parameter for the compiler to thread or the runtime to look up.

**No round trip inside an occurrence.** A response is in a later occurrence than its request, always, from a World and from an interceptor. What it buys: nothing a program does can depend on an effect having happened in the moment it was asked for; routing is always against a list the sender has seen; and `Defer` is the only form of "later" the language needs, which bounds a turn without a cap.

**No placeholder for state.** A state element has the value the outside held, from its first occurrence. What it buys: a widget built mid-occurrence is consistent with everything it captured; `NotYet` does not exist, so no `match` has an arm for a value the world never held; and the equality law is a fact about one key rather than a promise made per slot.

**No node values, no reified paths in the program.** A program cannot hold a path, compare two, or address a slot by one; only the outside addresses paths, and the program sees them only as the keys of an interceptor's list. What it buys: ownership is a tree, a path is never reused, and a program cannot keep a stale reference to a slot that its parent chose to remove.

The pattern repeats from the rest of the language: a feature that looks essential is removed, the removal closes a hole, and the same removal lets the World be a loop that reads a map and writes a map.

## Recapitulation

An element is plain data in a list. A tag says what it is and which of three things it does. `@Element` makes a slot and returns what the tag declares. Nothing the slot asks for is answered in the occurrence it asked, and that is a law bound on Worlds by the structure of the run call and on interceptors by routing against the opened list. A slot has a path for its conversations and a value for its facts. The list is a dynamic, read by whoever is outside, and the diode says the outside reads it and never reacts to it. State is there in its first occurrence because nothing else keeps a widget built mid-occurrence consistent with what it captured, and two readings of one value in one occurrence agree because there is one value. `intercept` exposes the two directions and nothing else, so that forwarding is the shortest program and every policy is a longer one, and a World written in Prism is a program with an outside to forward to.

Everything above the two World-facing GUI tags, and everything between a program's file read and the disk, is a `for` over a list of slots with a `match` inside it. The chapter on Worlds says what the loop at the bottom has to do, and it is short, because this chapter took everything else.
