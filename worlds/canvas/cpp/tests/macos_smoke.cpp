// A window on macOS: the program of smoke_program.hpp opens one `Gui.Window` with a red
// rectangle in it, under the Cocoa loop. When the loop settles, the hook checks that the
// rectangle's pixel in the frame handed to the window server is red, then presses there
// through `Session::input`, as a click would. The program hears the `Press` and tears both
// places down; the World ends the run with 0. xvfb_smoke.cpp's test with `Hooks` in place
// of the X11 robot, so it needs a logged-in session but no accessibility permission.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <unistd.h>

#include "smoke_program.hpp"

using namespace prism_rtl;
using namespace prism_world_canvas;
using namespace smoke;

int main() {
    alarm(30);  // a window that never settles ends the test
    bool painted = false, pressed = false;
    Hooks hooks;
    hooks.settled = [&](Session& s) {
        if (pressed || s.shared().windows.empty()) return;
        const Path& window = s.shared().windows.begin()->first;
        auto px = s.pixel(window, CX, CY);
        if (!px) return;
        painted = *px == RED;
        if (!painted) std::fprintf(stderr, "macos_smoke: the rectangle's pixel is %06x, not %06x\n", unsigned(*px), unsigned(RED));
        pressed = true;
        s.input(window, gui::press(CX, CY));
    };
    int code = run_with<SmokeProgram>(unwrapped, hooks);
    bool ok = code == 0 && painted && heard;
    if (code != 0) std::fprintf(stderr, "macos_smoke: the run's exit code is %d\n", code);
    if (!painted) std::fprintf(stderr, "macos_smoke: the scene's red never reached the window server\n");
    if (!heard) std::fprintf(stderr, "macos_smoke: the press never reached the program\n");
    if (!ok) return 1;
    std::printf("macos_smoke: a window opened, painted red, heard a press, closed; exit 0\n");
    return 0;
}
