// `main` for a program prismc compiled to C++, run under the canvas World: the root
// `crate::PRISM_ROOT` (the Makefile's ROOT, `MainProgram` for a `@$main`), on the big
// stack the emitted main uses. The emitted main.cpp is linked beside this with its own
// `main` renamed `prism_emitted_main`; `test` and `--world` other than canvas go to it, so
// one binary still runs the program's tests and its CLI root.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <string>
#include <vector>

#include "crate.hpp"
#include "prism_world_canvas/canvas.hpp"

#ifndef PRISM_ROOT
#error "PRISM_ROOT must name the program class of the root to run"
#endif

int prism_emitted_main(int argc, char** argv);

int main(int argc, char** argv) {
    std::vector<std::string> args(argv + 1, argv + argc);
    bool canvas = args.empty() || (args.size() == 2 && args[0] == "--world" && args[1] == "canvas");
    if (!canvas) return prism_emitted_main(argc, argv);
    return crate::prelude::on_big_stack([] { return prism_world_canvas::run<crate::PRISM_ROOT>(); });
}
