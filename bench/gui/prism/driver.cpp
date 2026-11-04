// The Prism table's driver (bench/gui/README.md): bench.prism compiled to C++ runs under the
// canvas World with `Hooks`, and each step of an operation's script is a press at a
// button's text, found in the scene the World last read. A timed step's clock starts when
// the press is queued and stops at the next `Hooks::settled`: every tick it caused has
// run, the frame is painted and handed to the window server.
//
//   driver OP RUNS WARMUP     prints `OP RUN MS` per timed run, then `peak_rss_bytes N`
//
// The emitted main.cpp is linked beside this with its `main` renamed (bench/gui/prism/Makefile).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <sys/resource.h>

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <map>
#include <string>
#include <vector>

#include "programs.hpp"
#include "prism_world_canvas/canvas.hpp"

int prism_emitted_main(int argc, char** argv);

namespace {

using namespace prism_world_canvas;
using Clock = std::chrono::steady_clock;

/// One press: the button whose text is `text`, or with `row` set, row `row`'s label
/// (`text` empty) or its remove button (`text` "x"), counting from 1.
struct Press {
    std::string text;
    int row = 0;
    bool timed = false;
};

Press button(const char* text, bool timed = false) { return Press{text, 0, timed}; }

const char* CREATE_1K = "Create 1,000 rows";
const char* CREATE_10K = "Create 10,000 rows";
const char* APPEND_1K = "Append 1,000 rows";
const char* UPDATE = "Update every 10th row";
const char* CLEAR = "Clear";
const char* SWAP = "Swap rows";

/// Each operation's script for one run, as js-framework-benchmark sets it up: the timed
/// press after its setup, and a clear after, untimed.
std::map<std::string, std::vector<Press>> scripts() {
    return {
        {"create1k", {button(CREATE_1K, true), button(CLEAR)}},
        {"replace1k", {button(CREATE_1K), button(CREATE_1K, true), button(CLEAR)}},
        {"update10th", {button(CREATE_1K), button(UPDATE, true), button(CLEAR)}},
        {"select", {button(CREATE_1K), Press{"", 2, true}, button(CLEAR)}},
        {"swap", {button(CREATE_1K), button(SWAP, true), button(CLEAR)}},
        {"remove", {button(CREATE_1K), Press{"x", 2, true}, button(CLEAR)}},
        {"create10k", {button(CREATE_10K, true), button(CLEAR)}},
        {"append1k", {button(CREATE_1K), button(APPEND_1K, true), button(CLEAR)}},
        {"clear", {button(CREATE_1K), button(CLEAR, true)}},
    };
}

struct Shown {
    double x, y;
    std::string text;
};

double number(const Pod* p) {
    auto v = p ? p->as_f64() : std::nullopt;
    return v ? *v : 0;
}

/// Every `Draw.Text` in a draw list, in drawing order; the window library's frames here
/// neither turn nor scale, so a text's point is the window's.
void texts(const Pod& draws, std::vector<Shown>& out) {
    const auto* xs = draws.structure().if_array();
    if (!xs) return;
    for (const Pod& d : *xs) {
        const std::string* tag = d.structure().tag();
        if (!tag) continue;
        if (*tag == "Draw.Text") {
            const Pod* at = d.structure().field("at");
            const Pod* t = d.structure().field("text");
            if (at && t && t->as_text()) out.push_back({number(at->field("x")), number(at->field("y")), t->as_text()->to_string()});
        } else if (*tag == "Draw.Clip" || *tag == "Draw.Group") {
            if (const Pod* inner = d.structure().field("draws")) texts(*inner, out);
        }
    }
}

/// Where to press for `p`, in the window's logical pixels: just inside the text's point,
/// which a button insets 8 from its corner.
std::optional<std::pair<double, double>> target(const Session& s, const Press& p) {
    std::vector<Shown> all;
    for (const auto& [_, scene] : s.shared().scenes) texts(scene.draw, all);
    int xs = 0;
    for (std::size_t i = 0; i < all.size(); ++i) {
        const Shown& t = all[i];
        if (p.row == 0) {
            if (t.text == p.text) return std::pair{t.x + 2, t.y + 2};
            continue;
        }
        if (t.text != "x" || ++xs != p.row) continue;
        if (p.text == "x") return std::pair{t.x + 2, t.y + 2};
        if (i == 0) return std::nullopt;
        return std::pair{all[i - 1].x + 2, all[i - 1].y + 2};
    }
    return std::nullopt;
}

/// What the screen must show after an operation's timed step: the rows (one "x" each),
/// and a text that must be there, if any. A wrong number fails the run.
struct Expect {
    int rows;
    std::string marker;  // a text that starts with this ("▶ ") or ends with it (" !!!")
};

Expect expected(const std::string& op) {
    static const std::map<std::string, Expect> all = {
        {"create1k", {1000, ""}}, {"replace1k", {1000, ""}}, {"update10th", {1000, " !!!"}},
        {"select", {1000, "\u25b6 "}}, {"swap", {1000, ""}}, {"remove", {999, ""}},
        {"create10k", {10000, ""}}, {"append1k", {2000, ""}}, {"clear", {0, ""}},
    };
    return all.at(op);
}

/// Whether the screen shows what `e` asks for; says what it saw when not.
bool shows(const Session& s, const Expect& e) {
    std::vector<Shown> all;
    for (const auto& [_, scene] : s.shared().scenes) texts(scene.draw, all);
    int rows = 0;
    bool marked = e.marker.empty();
    for (const Shown& t : all) {
        if (t.text == "x") ++rows;
        const std::string& m = e.marker;
        if (!m.empty() && t.text.size() >= m.size() &&
            (t.text.compare(0, m.size(), m) == 0 || t.text.compare(t.text.size() - m.size(), m.size(), m) == 0))
            marked = true;
    }
    if (rows == e.rows && marked) return true;
    std::fprintf(stderr, "driver: the screen shows %d rows (wanted %d)%s\n", rows, e.rows, marked ? "" : ", and no marked label");
    return false;
}

long peak_rss_bytes() {
    rusage u{};
    getrusage(RUSAGE_SELF, &u);
#ifdef __APPLE__
    return long(u.ru_maxrss);
#else
    return long(u.ru_maxrss) * 1024;
#endif
}

}  // namespace

int main(int argc, char** argv) {
    if (argc != 4) {
        std::fprintf(stderr, "usage: driver OP RUNS WARMUP (OP: create1k replace1k update10th select swap remove create10k append1k clear)\n");
        return 2;
    }
    std::string op = argv[1];
    int runs = std::atoi(argv[2]), warmup = std::atoi(argv[3]);
    auto all = scripts();
    auto it = all.find(op);
    if (it == all.end() || runs <= 0 || warmup < 0) {
        std::fprintf(stderr, "driver: unknown operation %s, or a bad count\n", op.c_str());
        return 2;
    }
    std::vector<Press> steps;
    for (int r = 0; r < warmup + runs; ++r)
        for (Press p : it->second) {
            if (r < warmup) p.timed = false;
            steps.push_back(p);
        }

    std::size_t next = 0;
    int run = 0;
    bool waiting = false, timing = false;
    Clock::time_point t0;
    int failed = 0;
    Hooks hooks;
    hooks.settled = [&](Session& s) {
        if (waiting && timing) {
            double ms = std::chrono::duration<double, std::milli>(Clock::now() - t0).count();
            std::printf("%s %d %.3f\n", op.c_str(), ++run, ms);
            std::fflush(stdout);
            if (!shows(s, expected(op))) {
                failed = 1;
                s.exit(1);
                return;
            }
        }
        waiting = false;
        if (next == steps.size()) {
            std::printf("peak_rss_bytes %ld\n", peak_rss_bytes());
            s.exit(0);
            return;
        }
        if (s.shared().windows.empty()) return;
        const Press& p = steps[next++];
        auto at = target(s, p);
        if (!at) {
            std::fprintf(stderr, "driver: no button \"%s\" (row %d) on the screen\n", p.text.c_str(), p.row);
            failed = 1;
            s.exit(1);
            return;
        }
        waiting = true;
        timing = p.timed;
        t0 = Clock::now();
        s.input(s.shared().windows.begin()->first, gui::press(at->first, at->second));
    };
    int code = run_with<crate::MainProgram>(unwrapped, hooks);
    return failed ? 1 : code;
}
