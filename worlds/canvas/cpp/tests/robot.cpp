// The robot as a command, for scripts (tests/chess_smoke.sh):
//
//   robot wait X Y RRGGBB [SECONDS]   wait until the screen's pixel (X, Y) is RRGGBB
//   robot click X Y                   click the left button at (X, Y)
//   robot close TITLE                 ask the window named TITLE to close
//
// Exit 0 on success, 1 otherwise.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <cstdlib>
#include <string>

#include "robot.hpp"

int main(int argc, char** argv) {
    robot::Robot r;
    if (!r) {
        std::fprintf(stderr, "robot: cannot open the display\n");
        return 1;
    }
    std::string cmd = argc > 1 ? argv[1] : "";
    if (cmd == "wait" && (argc == 5 || argc == 6)) {
        double secs = argc == 6 ? std::atof(argv[5]) : 20.0;
        auto rgb = std::uint32_t(std::strtoul(argv[4], nullptr, 16));
        return r.wait_for(std::atoi(argv[2]), std::atoi(argv[3]), rgb, secs) ? 0 : 1;
    }
    if (cmd == "click" && argc == 4) {
        r.click(std::atoi(argv[2]), std::atoi(argv[3]));
        return 0;
    }
    if (cmd == "close" && argc == 3) {
        if (r.close(argv[2])) return 0;
        std::fprintf(stderr, "robot: no window named %s\n", argv[2]);
        return 1;
    }
    std::fprintf(stderr, "usage: robot wait X Y RRGGBB [SECONDS] | robot click X Y | robot close TITLE\n");
    return 2;
}
