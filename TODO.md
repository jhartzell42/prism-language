# Rewrite

* Fix compiler warnings
    * Move `worlds/rust/tests/common/mod.rs`
        * Place in `src/testing`
        * Make it `#[cfg(test)]` and `pub`
        * Don't use `mod common` in multiple roots to alias the same code
            * Yields compiler warnings, bro!
* Pull/merge Claude branch for fix for new bugs
    * Claude agent is fixing many critical bugs
* All `README.md` files
    * Next up: Root `README.md`
    * Find out where the pieces came from?
* Makefile and shell scripts
    * Especially comments
* Clean up overtesting
* Example chess app
    * Rename `chess` to `basic-chess`
    * Rewrite the code, especially any comments
    * Simplify the tests
* Everything else
    * Don't ship until it's almost entirely human-written
    * Pull into one commit