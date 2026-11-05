# emit/cpp/

The module `Emit.Cpp` (claude/modules-style-pass.md): Emission for C++: the model lowering decided (lower.prism) spelled as a C++20 program over `runtime/cpp` (`prism_rtl` in C++, whose README maps every Rust type of the library to its C++ form) through the templates (templates/cpp.prism, templates/cpp_prelude.prism): `Makefile`, three headers per Prism module in its directory (`src/DIR/decl.hpp`, `src/DIR/types.hpp`, `src/DIR/module.hpp`, DIR its path, `check/env` for `Check.Env`) and a fourth when it has generic definitions (`src/DIR/generic.hpp`), two for `Elem` and `Answer` (`src/elem.decl.hpp`, `src/elem.types.hpp`), the program classes (`src/programs.hpp`), `src/main.cpp` and one `src/DIR/mod.cpp` (or several) per module (prismc/README.md, pick MH).

- `mod.prism`: the main file: the header; what the printer looks up, types.
- `expr.prism`: a section; expressions.
- `items.prism`: a section; items.
- `patterns.prism`: a section; patterns.
- `program.prism`: a section; nodes and programs, the program.
- `stmts.prism`: a section; statements.
- Submodules: `prelude/` (`Emit.Cpp.Prelude`), `templates/` (`Emit.Cpp.Templates`).

Written by Claude on the claude-self-hosting branch; subject to change.
