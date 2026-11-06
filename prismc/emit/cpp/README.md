# prismc/emit/cpp/

The module `Emit.Cpp` (claude/modules-style-pass.md): Emission for C++: the model lowering decided (lower.prism) spelled as a C++20 program over `runtime/cpp` (`prism_rtl` in C++, whose README maps every Rust type of the library to its C++ form) through the templates (templates/cpp.prism, templates/cpp_prelude.prism): `Makefile`, three headers per Prism module in its directory (`src/DIR/decl.hpp`, `src/DIR/types.hpp`, `src/DIR/module.hpp`, DIR its path, `check/env` for `Check.Env`) and a fourth when it has generic definitions (`src/DIR/generic.hpp`), two for `Elem` and `Answer` (`src/elem.decl.hpp`, `src/elem.types.hpp`), the program classes (`src/programs.hpp`), `src/main.cpp` and the modules' definitions in translation units, `src/unit_N.cpp`, several small modules to a unit (prismc/README.md, picks MH and SF).

- `mod.prism`: `Emit.Cpp` itself: What the printer looks up; Types.
- `expr.prism`: `Emit.Cpp.Expr`, a child module: Expressions.
- `items.prism`: `Emit.Cpp.Items`, a child module: Items.
- `patterns.prism`: `Emit.Cpp.Patterns`, a child module: Patterns.
- `prelude.prism`: `Emit.Cpp.Prelude`, a child module: The prelude of every program compiled to C++ (Semantics Appendix B; prelude/prelude.prism): its builtins and the functions it writes in Prism, as C++ over `prism_rtl`, emitted as `src/prelude.hpp` word for word: the C++ form of templates/prelude.prism, function for function, under the same names (a C++ keyword with `_` after it: `or_`, `int_`).
- `program.prism`: `Emit.Cpp.Program`, a child module: Nodes and programs; The program.
- `stmts.prism`: `Emit.Cpp.Stmts`, a child module: Statements.
- `templates/`: `Emit.Cpp.Templates`, a child module with children of its own (its README).

Written by Claude on the claude-self-hosting branch; subject to change.
