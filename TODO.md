# Plan

* Make widgets dyn-compatible
    * Why were they not to begin with?
        * If there's a good reason, address it! Certainly document it!
        * `AnyWidget` should then probably also be a thing!
        * `Widget`s should be storable in values...
    * This will make everything easier later on
        * Perf is not a priority right now!
            * Perf will be addressed once the damn thing is working at all!
* Write backend plan
    * Do a bunch of research into `egui`
    * Write up a proper backend plan document
        * Give a few alternatives to how backend plans might actually look
    * A composable backend?
        * A delegate that looks up what component we're looking for
            * A few different ways to register components
            * Which run loops is a given component compatible with?
                * Capabilities
                    * Async
                    * `egui`
                        * But maybe this is introduced by context
                        * And if you start an `egui` window component
                            * Then it activates new components within it?
                    * `ratatui`
                        * Maybe this is similar?
                * Maybe a generic way of writing async IO components
                    * This should be compatible with basically any run loop
                * But components can also be implemented for specific run loops(?)
                    * Same component for multiple run loops = form of polymorphism
                    * This can be exposed higher up
                    * But maybe components can contain their own run loop
                        * Which runs in its own thread?
        * Run loop ideas
            * Just `async`
            * `egui`
            * `ratatui`
* Write button/label sample app
    * With `test_backend` test
    * Start determining how we communicate with backend
        * Way of loading backend components
        * GUI + async?
        * How do we do this?
* List combinator for widgets!
* Do hygiene refactors
* GUI backend
    * `egui`?
    * Features:
        * Horizontal/vertical alignment
        * Image with background color
        * Textbox
        * Button
        * Label
* Write sample apps
    * Ideas
        * Mortgage calculator
        * Chess
        * Tabbed root app chooser
    * Test
        * Using test backend
        * Using GUI backend
* Define bytecode/AST for PL
    * In `prism_ir`
        * SSA or bytecode
    * Implement event/dynamic transformers
    * Implement widgets
    * Rewrite sample apps in IR
* Implement compiler to IR
    * In `prism_frontend`
    * Rewrite sample apps in programming language

# Hygiene refactors:

* Figure out how to put events in values without `Arc`
    * `EventImpl` has associated type for output
    * `EventHarness` implements the actual `EventInterface` trait
        * For `AnyValue` and for `T`
            * So they share the same underlying impl
        * `EventInterface` has `as_any_event()` casting to `Event<AnyValue>`
        * `EventInterface` has `as_specific_event()`
            * For casting from `Event<AnyValue>` to `Event<T>`
            * Giving you a `&dyn Any`
                * Which is an `Event<T>`
* Add documentation to public methods
* All combinators that take closures
    * Take custom traits
    * Easy conversions for closures?
* Better API for backends subscribing