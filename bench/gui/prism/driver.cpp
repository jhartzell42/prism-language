// The Prism table's driver (bench/gui/README.md): bench.prism compiled to C++ runs under the
// canvas World with `Hooks`, and each step of an operation's script is a press at a
// button's text, found in the scene the World last read. A timed step's clock starts when
// the press is queued and stops at the next `Hooks::settled`: every tick it caused has
// run, the frame is painted and handed to the window server.
//
//   driver OP RUNS WARMUP     prints `OP RUN MS` per timed run, then `peak_rss_bytes N`
//   driver OP RUNS WARMUP check
//                             also counts each step's work and fails the run when it is
//                             out of proportion (bench/gui/README.md, "The checks")
//
// The emitted main.cpp is linked beside this with its `main` renamed (bench/gui/prism/Makefile).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <sys/resource.h>
#include <unistd.h>
#ifdef __APPLE__
#include <libproc.h>
#endif

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

/// A row of the table as the screen shows it: its id's text, its label, and where the
/// label is.
struct RowShown {
    std::string id, label;
    double y;
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

/// The table the program should have, followed as the app follows it: the ids its rows
/// were made with (each Create takes the next ids, as `AddRow` does), in slot order, the
/// selected row and the rows Update every 10th row marked.
struct Model {
    long next = 1;
    std::vector<long> ids;
    std::vector<bool> banged;
    long selected = 0;  // the selected row's position, counting from 1; 0 for none

    void press(const Press& p) {
        auto add = [&](int n) {
            for (int i = 0; i < n; ++i) ids.push_back(next++), banged.push_back(false);
        };
        if (p.row != 0 && p.text == "x") {
            ids.erase(ids.begin() + (p.row - 1)), banged.erase(banged.begin() + (p.row - 1));
            if (selected == p.row) selected = 0;
            else if (selected > p.row) --selected;
        } else if (p.row != 0) {
            selected = p.row;
        } else if (p.text == CREATE_1K || p.text == CREATE_10K) {
            ids.clear(), banged.clear(), selected = 0;  // the new rows take new slots
            add(p.text == CREATE_1K ? 1000 : 10000);
        } else if (p.text == APPEND_1K) {
            add(1000);
        } else if (p.text == CLEAR) {
            ids.clear(), banged.clear(), selected = 0;
        } else if (p.text == SWAP && ids.size() >= 999) {
            std::swap(ids[1], ids[998]);
            bool b = banged[1];
            banged[1] = banged[998], banged[998] = b;
        } else if (p.text == UPDATE) {
            for (std::size_t i = 0; i < banged.size(); i += 10) banged[i] = true;
        }
    }
};

/// The rows the screen shows, top to bottom: a row's texts are its id, its label (the
/// select button's) and its remove button's "x".
std::vector<RowShown> rows_shown(const Session& s) {
    std::vector<Shown> all;
    for (const auto& [_, scene] : s.shared().scenes) texts(scene.draw, all);
    std::vector<RowShown> rows;
    for (std::size_t i = 2; i < all.size(); ++i)
        if (all[i].text == "x") rows.push_back({all[i - 2].text, all[i - 1].text, all[i - 1].y});
    return rows;
}

bool ends_with(const std::string& t, const std::string& m) {
    return t.size() >= m.size() && t.compare(t.size() - m.size(), m.size(), m) == 0;
}

/// Whether the screen shows the table the model says, as far as the window reaches: the
/// first rows in order, each with its id, its "▶ " when selected and its " !!!" when
/// updated, and, when there are more rows than fit, rows down to near the window's bottom
/// edge (a row is about 30 high). Says what it saw when not.
bool shows(const Session& s, const Model& m) {
    auto rows = rows_shown(s);
    double h = s.shared().windows.empty() ? 0 : s.shared().windows.begin()->second.h;
    auto fail = [&](const std::string& why) {
        std::fprintf(stderr, "driver: the screen shows %zu rows of %zu: %s\n", rows.size(), m.ids.size(), why.c_str());
        return false;
    };
    if (rows.size() > m.ids.size()) return fail("more than the table has");
    if (!m.ids.empty() && rows.empty()) return fail("none");
    if (rows.size() < m.ids.size() && rows.back().y < h - 60) return fail("they stop short of the window's bottom edge");
    const std::string marker = "\u25b6 ";
    for (std::size_t i = 0; i < rows.size(); ++i) {
        const RowShown& r = rows[i];
        std::string n = std::to_string(i + 1);
        if (r.id != std::to_string(m.ids[i])) return fail("row " + n + " is id " + r.id + ", not " + std::to_string(m.ids[i]));
        bool sel = r.label.compare(0, marker.size(), marker) == 0;
        if (sel != (m.selected == long(i + 1))) return fail("row " + n + (sel ? " is" : " is not") + " marked selected");
        if (ends_with(r.label, " !!!") != m.banged[i]) return fail("row " + n + "'s label is \"" + r.label + "\"");
    }
    return true;
}

/// Instructions retired by the whole process so far, every thread's: the work a step
/// costs, which nothing else running on the machine changes (macOS's `proc_pid_rusage`).
long long instructions() {
#ifdef __APPLE__
    rusage_info_v4 u{};
    if (proc_pid_rusage(getpid(), RUSAGE_INFO_V4, reinterpret_cast<rusage_info_t*>(&u)) != 0) {
        std::perror("driver: proc_pid_rusage");
        std::exit(1);
    }
    return static_cast<long long>(u.ri_instructions);
#else
    std::fprintf(stderr, "driver: check counts instructions with macOS's proc_pid_rusage, which this system lacks\n");
    std::exit(2);
#endif
}

/// `check`'s first check, after each step settles: every `Draw.Text` in the scenes starts
/// inside the window, or within 8 of its edges, so the window library draws only what the
/// window shows.
bool drawn_inside(const Session& s) {
    if (s.shared().windows.empty()) return true;
    const WindowPlace& w = s.shared().windows.begin()->second;
    std::vector<Shown> all;
    for (const auto& [_, scene] : s.shared().scenes) texts(scene.draw, all);
    int outside = 0;
    for (const Shown& t : all)
        if (t.x > w.w + 8 || t.y > w.h + 8 || t.x < -8 || t.y < -8) ++outside;
    if (outside == 0) return true;
    std::fprintf(stderr, "driver: check: %d of the scenes' %zu texts are drawn outside the %gx%g window\n", outside, all.size(), w.w, w.h);
    return false;
}

/// `check`'s second check: these operations' timed step retires under a tenth of the
/// instructions of their setup, Create 1,000 rows.
bool small_op(const std::string& op) { return op == "update10th" || op == "select" || op == "swap" || op == "remove"; }

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
    bool check = argc == 5 && std::strcmp(argv[4], "check") == 0;
    if (argc != 4 && !check) {
        std::fprintf(stderr, "usage: driver OP RUNS WARMUP [check] (OP: create1k replace1k update10th select swap remove create10k append1k clear)\n");
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
    long long i0 = 0, setup = 0;
    int failed = 0;
    Model model;
    Hooks hooks;
    hooks.settled = [&](Session& s) {
        if (waiting) {
            long long work = check ? instructions() - i0 : 0;
            if (timing) {
                double ms = std::chrono::duration<double, std::milli>(Clock::now() - t0).count();
                std::printf("%s %d %.3f\n", op.c_str(), ++run, ms);
                std::fflush(stdout);
                if (!shows(s, model)) {
                    failed = 1;
                    s.exit(1);
                    return;
                }
            }
            if (check) {
                const Press& p = steps[next - 1];
                std::printf("check %s step %zu (%s row %d): %lld instructions\n", op.c_str(), next, p.text.c_str(), p.row, work);
                std::fflush(stdout);
                if (!timing && p.text == CREATE_1K) setup = work;
                bool small = !timing || !small_op(op) || work * 10 < setup;
                if (!small)
                    std::fprintf(stderr, "driver: check: %s retired %lld instructions, not under a tenth of Create 1,000 rows' %lld\n", op.c_str(), work, setup);
                if (!drawn_inside(s) || !small) {
                    failed = 1;
                    s.exit(1);
                    return;
                }
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
        model.press(p);
        if (check) i0 = instructions();
        t0 = Clock::now();
        s.input(s.shared().windows.begin()->first, gui::press(at->first, at->second));
    };
    int code = run_with<crate::MainProgram>(unwrapped, hooks);
    return failed ? 1 : code;
}
