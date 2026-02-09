# Plan

* Cyclic dynamic values
* All combinators that take closures
    * Take custom traits
    * Blanket implementations for closures
* Write and test `fold_dyn()` combinator
    * Requires builder/cyclic
* Write button/label sample app
* List combinator for widgets
* GUI backend
    * `egui`?
    * Features:
        * Horizontal/vertical alignment
        * Image with background color
        * Textbox
        * Button
        * Label
* Write sample apps
    * Mortgage calculator
    * Chess
    * Tabbed root app chooser
* Universal erased copy/clone value type
    * Different debug impl vs release
        * Release impl doesn't check if types match
    * Anything you store in it has to implement `PrismValue`
        * Blanket implementation for `Copy`
            * "Runtime" queries size to determine whether to box
        * Implementation present for `Event`/`Dynamic`/`Behavior`
    * Reconsider erased dynamic/event/trigger types
* Define AST for PL
    * In `prism_language`

# Hygiene refactors:

* Use `thiserror` everywhere
* Add named fields to types in `erased.rs`
* Get proper test logging
* Add documentation to public methods