# Prism

The specification and plan for Prism live in [`claude/`](claude/), one chapter per file:

0. [Introduction](claude/ch00-introduction.md)
1. [Worked examples](claude/ch01-worked-examples.md)
2. [Sigils, lexical structure, values, expressions](claude/ch02-language-core.md)
3. [Events, dynamics, and time](claude/ch03-events-dynamics-time.md)
4. [Widgets and switching](claude/ch04-widgets-switching.md)
5. [Elements and interception](claude/ch05-elements-interception.md)
6. [Worlds](claude/ch06-worlds.md)
7. [Use cases in depth](claude/ch07-use-cases.md)
8. [Errors](claude/ch08-errors.md)
9. [Conformance and testing](claude/ch09-conformance-testing.md)
10. [The Rust runtime](claude/ch10-rust-runtime.md)
11. [The Rust language implementation](claude/ch11-rust-lang.md)
12. [Rust Worlds and the world core](claude/ch12-rust-worlds.md)
13. [The Haskell implementation](claude/ch13-haskell.md)
14. [Plan](claude/ch14-plan.md)
15. [Future work](claude/ch15-future-work.md)
16. [Appendices](claude/ch16-appendices.md)

`claude/DESIGN-RULES.md` holds the binding design decisions and voice rules the chapters were written against; where a chapter disagrees with it, it wins. `claude/lint/` holds the lint report for each chapter; chapters 3, 4, 8, 10, 15 and 16 have had their lint applied, the others have not yet.

The original language description is [`LANGUAGE-ORIG.md`](LANGUAGE-ORIG.md).
