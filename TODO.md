# Plan

* Make it so that `Event`/`Dynamic` have implicit `Arc`
    * So you don't have to use `Arc` with them
    * Later, we can refine the implementation
* Write button/label sample app
* List combinator for widgets
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