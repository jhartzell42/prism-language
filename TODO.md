# Big Picture Plan
* Rewrite tests using test backend
* Test `dynamic_widget()` combinator
* Write and test `fold_dyn` combinator
* Write button/label sample app
* List combinator for widgets
* GUI backend
    * `egui`?
    * Write tic-tac-toe
    * Write chess
* Implement programming language
    * v1: As translator to generate a Rust module
    * v2: As interpreter

# Hygiene refactors:
* Use `thiserror` everywhere
* Add named fields to types in `erased.rs`
* Get proper test logging
* Add documentation to public methods