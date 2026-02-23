# Plan

* Universal erased value type
    * Get it working
        * Fix more types
            * Test backend
            * Widget
        * Run tests
        * Commit
    * Migrate erased events
        * Can treat as events of erased types
        * Can cast back with checks to events of specific types
    * Store erased in `PrismValue`
        * Making events valid `PrismValue` without double `Arc`
    * Work on more ergonomic API for dealing with this stuff
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

* Add named fields to types in `erased.rs`
* Add documentation to public methods
* All combinators that take closures
    * Take custom traits
    * Easy conversions for closures?
* Better API for backends subscribing