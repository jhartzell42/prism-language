// The Prism table's driver (bench/gui/README.md): bench.prism compiled to C++ runs under the
// canvas World with `Hooks`, and each step of an operation's script is a press at a
// button's text, found in the scene the World last read. A timed step's clock starts when
// the press is queued and stops at the next `Hooks::settled`: every tick it caused has
// run, the frame is painted and handed to the window server.
//
//   driver OP RUNS WARMUP     prints `OP RUN MS` per timed run, then `peak_rss_bytes N`
//   driver OP RUNS WARMUP check
//                             also counts each step's work by phase, and fails the run
//                             when a scene draws outside the window (bench/gui/README.md,
//                             "The checks")
//
// The work is instructions retired where the system counts them: macOS's `proc_pid_rusage`
// and `thread_selfcounts`, or on Linux a perf_event counter (PERF_COUNT_HW_INSTRUCTIONS)
// per process and per thread. When the Linux kernel refuses the counter (no PMU in a VM:
// ENOENT or EOPNOTSUPP; or not permitted: EACCES) the driver counts each thread's CPU
// nanoseconds instead (`CLOCK_THREAD_CPUTIME_ID`, the process's `CLOCK_PROCESS_CPUTIME_ID`)
// and every `check` and `phases` line names its unit (`unit=instructions`, `unit=ns`), so
// nanoseconds are never read as instructions.
//
// The emitted main.cpp is linked beside this with its `main` renamed (bench/gui/prism/Makefile).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <sys/resource.h>
#include <unistd.h>
#ifdef __APPLE__
#include <libproc.h>
#endif
#ifdef __linux__
#include <linux/perf_event.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <time.h>
#include <cerrno>
#endif

#include <atomic>
#include <chrono>
#include <cstdint>
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

/// What a step's work is counted in: instructions retired, or, on a Linux machine whose
/// kernel gives no instructions counter, CPU nanoseconds.
namespace work {

enum class Unit { Instructions, InstructionsUser, Ns };

#ifdef __APPLE__
Unit unit = Unit::Instructions;
void choose() {}
#else
int process_fd = -1;  // the process-wide counter, every thread's (inherited by threads made after it)
Unit unit = Unit::Ns;

/// A perf_event instructions counter on the calling thread, with `inherit` counting the
/// threads it makes afterwards too; `user_only` leaves the kernel's instructions out, which
/// is all an unprivileged process may count under perf_event_paranoid 2.
int perf_open(bool inherit, bool user_only) {
    perf_event_attr a{};
    a.type = PERF_TYPE_HARDWARE;
    a.size = sizeof a;
    a.config = PERF_COUNT_HW_INSTRUCTIONS;
    a.inherit = inherit;
    a.exclude_kernel = user_only;
    a.exclude_hv = user_only;
    return int(syscall(SYS_perf_event_open, &a, 0, -1, -1, PERF_FLAG_FD_CLOEXEC));
}

bool refused(int err) { return err == ENOENT || err == EOPNOTSUPP || err == EACCES || err == EPERM || err == ENOSYS; }

/// The unit, chosen once before any other thread exists: the process counter opened with
/// inherit, all instructions if allowed, else user-space ones, else nanoseconds.
void choose() {
    for (bool user_only : {false, true}) {
        int fd = perf_open(true, user_only);
        if (fd >= 0) {
            process_fd = fd;
            unit = user_only ? Unit::InstructionsUser : Unit::Instructions;
            if (user_only) std::fprintf(stderr, "driver: counting user-space instructions only (the kernel refused kernel counting: perf_event_paranoid)\n");
            return;
        }
        if (!refused(errno)) {
            std::perror("driver: perf_event_open");
            std::exit(1);
        }
    }
    std::fprintf(stderr, "driver: no instructions counter on this machine (perf_event_open: %s); counting CPU nanoseconds instead\n", std::strerror(errno));
    unit = Unit::Ns;
}

std::uint64_t read_counter(int fd) {
    std::uint64_t v = 0;
    if (read(fd, &v, sizeof v) != ssize_t(sizeof v)) {
        std::perror("driver: reading the perf_event counter");
        std::exit(1);
    }
    return v;
}

std::uint64_t cpu_ns(clockid_t clock) {
    timespec t{};
    if (clock_gettime(clock, &t) != 0) {
        std::perror("driver: clock_gettime");
        std::exit(1);
    }
    return std::uint64_t(t.tv_sec) * 1000000000ull + std::uint64_t(t.tv_nsec);
}
#endif

const char* name() { return unit == Unit::Instructions ? "instructions" : unit == Unit::InstructionsUser ? "instructions-user" : "ns"; }

}  // namespace work

/// The whole process's work so far, every thread's, in `work::unit`: instructions retired,
/// which nothing else running on the machine changes (macOS's `proc_pid_rusage`; Linux's
/// inherited perf_event counter), or CPU nanoseconds.
long long process_work() {
#ifdef __APPLE__
    rusage_info_v4 u{};
    if (proc_pid_rusage(getpid(), RUSAGE_INFO_V4, reinterpret_cast<rusage_info_t*>(&u)) != 0) {
        std::perror("driver: proc_pid_rusage");
        std::exit(1);
    }
    return static_cast<long long>(u.ri_instructions);
#else
    if (work::unit == work::Unit::Ns) return static_cast<long long>(work::cpu_ns(CLOCK_PROCESS_CPUTIME_ID));
    return static_cast<long long>(work::read_counter(work::process_fd));
#endif
}

/// The calling thread's work so far, in `work::unit`: on macOS libsystem_kernel's
/// `thread_selfcounts` (exported, in no header), whose flavour 1 fills the thread's
/// instructions and cycles, two 64-bit counts; on Linux the thread's own perf_event
/// counter, opened at its first call (inherit off: this thread alone), or its CPU clock.
#ifdef __APPLE__
extern "C" int thread_selfcounts(int flavor, void* buffer, std::size_t bytes);
#endif
std::uint64_t thread_work() {
#ifdef __APPLE__
    std::uint64_t c[2] = {0, 0};
    if (thread_selfcounts(1, c, sizeof c) != 0) {
        std::fprintf(stderr, "driver: thread_selfcounts failed\n");
        std::exit(1);
    }
    return c[0];
#else
    if (work::unit == work::Unit::Ns) return work::cpu_ns(CLOCK_THREAD_CPUTIME_ID);
    thread_local int fd = -1;
    if (fd < 0) {
        fd = work::perf_open(false, work::unit == work::Unit::InstructionsUser);
        if (fd < 0) {
            std::perror("driver: perf_event_open for a thread (the process's counter opened)");
            std::exit(1);
        }
    }
    return work::read_counter(fd);
#endif
}

/// The work of each phase (prism_rtl's `probe`), by what each thread did between its marks
/// (`work::unit`): each thread's current phase and its count at its last mark, and the
/// totals per phase over every thread. The canvas World ticks on its big stack's thread and
/// paints on the main one, so a thread's count is its own.
namespace phases {

using prism_rtl::probe::Phase;
using prism_rtl::probe::PHASES;

std::atomic<std::uint64_t> total[PHASES];
std::atomic<std::uint64_t> ticks{0};

struct At {
    bool set = false;
    Phase phase = Phase::Loop;
    std::uint64_t since = 0;
};
thread_local At at;

/// The probe's hook: this thread's work since its last mark goes to the phase it was in.
void mark(Phase next) {
    std::uint64_t now = thread_work();
    if (at.set) total[std::size_t(at.phase)].fetch_add(now - at.since, std::memory_order_relaxed);
    if (next == Phase::Propagate) ticks.fetch_add(1, std::memory_order_relaxed);
    at = At{true, next, now};
}

struct Totals {
    std::uint64_t phase[PHASES] = {};
    std::uint64_t ticks = 0;
};

/// The totals so far, this thread's work since its last mark counted to its phase.
Totals now() {
    if (at.set) {
        std::uint64_t c = thread_work();
        total[std::size_t(at.phase)].fetch_add(c - at.since, std::memory_order_relaxed);
        at.since = c;
    }
    Totals t;
    for (std::size_t i = 0; i < PHASES; ++i) t.phase[i] = total[i].load(std::memory_order_relaxed);
    t.ticks = ticks.load(std::memory_order_relaxed);
    return t;
}

/// One step's line: each phase's work from `from` to `to`, the rest of the process's
/// (`all`, every thread's) as `other`, the ticks, and the unit.
void print(const std::string& op, const char* kind, const Totals& from, const Totals& to, long long all) {
    static const char* names[PHASES] = {"loop", "propagate", "build", "patch", "diff", "perform", "compact", "paint"};
    std::printf("phases %s %s", op.c_str(), kind);
    std::uint64_t sum = 0;
    for (std::size_t i = 0; i < PHASES; ++i) {
        std::uint64_t d = to.phase[i] - from.phase[i];
        sum += d;
        std::printf(" %s=%llu", names[i], (unsigned long long)d);
    }
    unsigned long long a = (unsigned long long)all;
    std::printf(" other=%llu total=%llu ticks=%llu unit=%s\n", a > sum ? a - sum : 0ull, a, (unsigned long long)(to.ticks - from.ticks), work::name());
    std::fflush(stdout);
}

}  // namespace phases

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
    long long i0 = 0;
    phases::Totals p0, p1;
    if (check) work::choose();  // before the World's threads exist, so the process counter inherits to them
    if (check && !prism_rtl::probe::set_hook(phases::mark)) {
        std::fprintf(stderr, "driver: the phase probe's hook was set already\n");
        return 1;
    }
    int failed = 0;
    Model model;
    Hooks hooks;
    hooks.settled = [&](Session& s) {
        if (waiting) {
            long long work = check ? process_work() - i0 : 0;
            if (check) p1 = phases::now();
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
                std::printf("check %s step %zu (%s row %d): %lld %s\n", op.c_str(), next, p.text.c_str(), p.row, work, work::name());
                std::fflush(stdout);
                phases::print(op, timing ? "timed" : p.text == CREATE_1K ? "setup" : "untimed", p0, p1, work);
                if (!drawn_inside(s)) {
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
        if (check) {
            p0 = phases::now();
            i0 = process_work();
        }
        t0 = Clock::now();
        s.input(s.shared().windows.begin()->first, gui::press(at->first, at->second));
    };
    int code = run_with<crate::MainProgram>(unwrapped, hooks);
    return failed ? 1 : code;
}
