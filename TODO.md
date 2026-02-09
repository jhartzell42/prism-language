# Plan

* Write and test `fold_dyn()` combinator
    * Requires builder/cyclic
    * Test cyclic dynamic values
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
* Universal erased copy/clone value type
    * Interface over implementation
        * Interface that allows replacing the implementation
    * Different debug impl vs release
        * Release impl doesn't check if types match
    * Anything you store in it has to implement `PrismValue`
        * Blanket implementation for `Copy`
            * "Runtime" queries size to determine whether to box
        * Implementation present for `Event`/`Dynamic`/`Behavior`
    * Reconsider erased dynamic/event/trigger types
* All combinators that take closures
    * Take custom traits
    * Blanket implementations for closures
* Better API for backends subscribing