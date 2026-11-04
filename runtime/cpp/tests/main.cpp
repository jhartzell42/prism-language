// Runs every registered test, or those whose names contain an argument.

#include <cstring>
#include <exception>

#include "check.hpp"

int main(int argc, char** argv) {
    int ran = 0;
    for (const auto& t : check::registry()) {
        bool wanted = argc < 2;
        for (int i = 1; i < argc; ++i) wanted |= std::strstr(t.name, argv[i]) != nullptr;
        if (!wanted) continue;
        int before = check::failures();
        try {
            t.body();
        } catch (const std::exception& e) {
            check::fail(t.name, 0, std::string("threw: ") + e.what());
        }
        ++ran;
        std::cout << (check::failures() == before ? "ok   " : "FAIL ") << t.name << "\n";
    }
    std::cout << ran << " tests, " << check::failures() << " failures\n";
    return check::failures() == 0 ? 0 : 1;
}
