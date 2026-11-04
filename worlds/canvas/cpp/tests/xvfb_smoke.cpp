// A window on a real X server: the program of smoke_program.hpp opens one `Gui.Window`
// with a red rectangle in it. A second thread, a client of its own on the same display,
// waits until the rectangle's pixel on the root window is red, then clicks it through
// XTest. The program hears the `Press` and tears both places down; with no window left and
// nothing owed, the World ends the run with 0.
// worlds/canvas/rust/tests/xvfb_smoke.rs in C++.
//
// With no `DISPLAY`, the test starts itself again under `xvfb-run -a`; with no `xvfb-run`
// it says so and passes. The child has 30 seconds.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <sys/wait.h>
#include <unistd.h>

#include <atomic>
#include <csignal>
#include <cstdlib>
#include <thread>

#include "robot.hpp"
#include "smoke_program.hpp"

using namespace prism_rtl;
using namespace prism_world_canvas;
using namespace smoke;

namespace {

/// Set when the scene's red showed on the screen.
std::atomic<bool> painted{false};

void run_robot() {
    robot::Robot r;
    if (!r) {
        std::fprintf(stderr, "smoke: the robot cannot open the display\n");
        return;
    }
    if (r.wait_for(CX, CY, RED, 20.0)) painted = true;
    r.click(CX, CY);
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
}

}  // namespace

int main(int, char** argv) {
    if (std::getenv("DISPLAY")) {
        XInitThreads();
        std::thread robot(run_robot);
        int code = run<SmokeProgram>();
        robot.join();
        bool ok = code == 0 && painted && heard;
        if (code != 0) std::fprintf(stderr, "xvfb_smoke: the run's exit code is %d\n", code);
        if (!painted) std::fprintf(stderr, "xvfb_smoke: the scene's red never reached the screen\n");
        if (!heard) std::fprintf(stderr, "xvfb_smoke: the window's input never reached the program\n");
        if (!ok) return 1;
        std::printf("xvfb_smoke: a window opened, painted, heard a click, closed; exit 0\n");
        return 0;
    }
    if (std::getenv("PRISM_SMOKE_CHILD")) {
        std::fprintf(stderr, "xvfb_smoke: xvfb-run gave the child no DISPLAY\n");
        return 1;
    }
    if (std::system("command -v xvfb-run > /dev/null 2>&1") != 0) {
        std::printf("xvfb_smoke: skipped, no display and no xvfb-run\n");
        return 0;
    }
    pid_t child = fork();
    if (child == 0) {
        setenv("PRISM_SMOKE_CHILD", "1", 1);
        execlp("xvfb-run", "xvfb-run", "-a", argv[0], static_cast<char*>(nullptr));
        _exit(127);
    }
    auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(30);
    while (std::chrono::steady_clock::now() < deadline) {
        int status = 0;
        if (waitpid(child, &status, WNOHANG) == child) {
            if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
                std::printf("xvfb_smoke: passed under xvfb-run\n");
                return 0;
            }
            std::fprintf(stderr, "xvfb_smoke: the child under xvfb-run failed (status %d)\n", status);
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    kill(child, SIGKILL);
    std::fprintf(stderr, "xvfb_smoke: the window never sent input within 30 s\n");
    return 1;
}
