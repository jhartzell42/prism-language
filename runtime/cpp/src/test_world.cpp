// The Test World (test_world.hpp).

#include "prism_rtl/test_world.hpp"

#include <algorithm>
#include <charconv>
#include <optional>
#include <variant>

#include "prism_rtl/doc.hpp"
#include "prism_rtl/prelude.hpp"
#include "prism_rtl/print.hpp"
#include "prism_rtl/read.hpp"
#include "prism_rtl/scene.hpp"

namespace prism_rtl {

namespace {

// ---- Rust's string methods, on ASCII markers --------------------------------------------

bool ws(char c) { return c == ' ' || (c >= '\t' && c <= '\r'); }
std::string_view trim_start(std::string_view s) {
    while (!s.empty() && ws(s.front())) s.remove_prefix(1);
    return s;
}
std::string_view trim_end(std::string_view s) {
    while (!s.empty() && ws(s.back())) s.remove_suffix(1);
    return s;
}
std::string_view trim(std::string_view s) { return trim_end(trim_start(s)); }
std::optional<std::string_view> strip_prefix(std::string_view s, std::string_view p) {
    if (s.starts_with(p)) return s.substr(p.size());
    return std::nullopt;
}
std::optional<std::pair<std::string_view, std::string_view>> split_once(std::string_view s, std::string_view sep) {
    auto i = s.find(sep);
    if (i == std::string_view::npos) return std::nullopt;
    return std::pair{s.substr(0, i), s.substr(i + sep.size())};
}
/// Rust's `str::lines`: split at `\n`, a trailing `\r` dropped, no empty last line.
std::vector<std::string_view> lines(std::string_view s) {
    std::vector<std::string_view> out;
    while (!s.empty()) {
        auto i = s.find('\n');
        std::string_view l = s.substr(0, i);
        if (l.ends_with('\r')) l.remove_suffix(1);
        out.push_back(l);
        if (i == std::string_view::npos) break;
        s.remove_prefix(i + 1);
    }
    return out;
}
std::string join(const std::vector<std::string>& xs, std::string_view sep) {
    std::string out;
    for (std::size_t i = 0; i < xs.size(); ++i) out += (i ? std::string(sep) : "") + xs[i];
    return out;
}
template <class T>
std::optional<T> parse_number(std::string_view s) {
    if (s.starts_with('+')) s.remove_prefix(1);
    T v{};
    auto r = std::from_chars(s.data(), s.data() + s.size(), v);
    if (s.empty() || r.ec != std::errc() || r.ptr != s.data() + s.size()) return std::nullopt;
    return v;
}

using Err = std::string;

// ---- The script: Conformance Appendix A ----------------------------------------------------

struct Target {
    /// `at path`, the path as printed; otherwise a pattern.
    std::optional<std::string> at;
    Pattern pattern;
};

struct Delivery {
    enum Kind { Send, Click, Element, Output } kind;
    Target target;
    Pod value;
    /// `click element path`, the path as printed.
    std::string element;
    /// `answer p`.
    Pattern pattern;
};

struct StepKind {
    /// `tick` and `compute`: a tick with no inputs; the word says why it is there.
    enum Kind { Build, Tick, ComputeUntilIdle, Push, Deliver } kind;
    Pod element, value;
    std::vector<Delivery> deliveries;
};

struct Check {
    enum Kind { Expect, ExpectNone, ExpectCount, ExpectTop, ExpectTopNone, Requested, Unrequested, Returns, Fired, Quiet, Settles, Aborted, Snapshot } kind;
    Pattern p, q;
    std::size_t count = 0;
    std::string file;

    /// The check as a failure names it, after Rust's `{:?}`.
    std::string debug() const {
        static const char* names[] = {"Expect", "ExpectNone", "ExpectCount", "ExpectTop", "ExpectTopNone", "Requested", "Unrequested",
                                      "Returns", "Fired", "Quiet", "Settles", "Aborted", "Snapshot"};
        std::string n = names[kind];
        auto pat = [](const Pattern& x) { return "Pattern(" + x.pod.to_string() + ")"; };
        switch (kind) {
        case Quiet: case Settles: case Aborted: return n;
        case Snapshot: return n + "(\"" + file + "\")";
        case ExpectCount: return n + "(" + pat(p) + ", " + std::to_string(count) + ")";
        case Requested: return n + "(" + pat(p) + ", " + pat(q) + ")";
        default: return n + "(" + pat(p) + ")";
        }
    }
};

struct Step {
    std::size_t line;
    std::vector<std::pair<Pod, Pod>> withs;
    StepKind kind;
    std::vector<std::pair<Check, std::size_t>> checks;
};

struct TestBlock {
    std::string name, program;
    std::vector<Pod> args;
    std::optional<std::uint64_t> seed;
    std::vector<Step> steps;
};

struct CheckBlock {
    std::string name;
    struct Line {
        std::string expr;
        Pattern pattern;
        std::size_t line;
    };
    std::vector<Line> lines;
};

using Block = std::variant<TestBlock, CheckBlock>;
using Lines = std::vector<std::pair<std::size_t, std::string_view>>;

/// Strip a comment: a `#` at the start of a line or after a space, outside a string (in
/// a path, `#` followed by a digit is a generation).
std::string_view strip_comment(std::string_view line) {
    bool in_str = false;
    char prev = ' ';
    for (std::size_t i = 0; i < line.size(); ++i) {
        char c = line[i];
        if (c == '"' && prev != '\\') in_str = !in_str;
        else if (c == '#' && !in_str && ws(prev)) return line.substr(0, i);
        prev = c;
    }
    return line;
}

std::size_t indent(std::string_view line) { return line.size() - trim_start(line).size(); }

std::string at_line(std::size_t n, const std::string& e) { return "line " + std::to_string(n) + ": " + e; }

Expected<Pattern, Err> pattern(std::size_t n, std::string_view s) {
    auto p = read::pattern(trim(s));
    if (!p) return unexpected(at_line(n, p.error()));
    return p;
}
Expected<Pod, Err> value(std::size_t n, std::string_view s) {
    auto v = read::value(trim(s));
    if (!v) return unexpected(at_line(n, v.error()));
    return v;
}
Expected<std::string, Err> printed_path(std::size_t n, std::string_view s) {
    auto p = read::path(trim(s));
    if (!p) return unexpected(at_line(n, p.error()));
    return p->to_string();
}

/// Split at commas at bracket depth zero, outside strings.
std::vector<std::string> split_commas(std::string_view s) {
    std::vector<std::string> out;
    std::string cur;
    int depth = 0;
    bool in_str = false;
    char prev = ' ';
    for (char c : s) {
        if (c == '"' && prev != '\\') in_str = !in_str;
        else if ((c == '(' || c == '[' || c == '{') && !in_str) ++depth;
        else if ((c == ')' || c == ']' || c == '}') && !in_str) --depth;
        else if (c == ',' && !in_str && depth == 0) {
            out.push_back(std::exchange(cur, {}));
            prev = c;
            continue;
        }
        cur += c;
        prev = c;
    }
    out.push_back(cur);
    return out;
}

/// Split `E = v` at the first `=` at depth zero that is not part of `==` or `=>`.
Expected<std::pair<Pod, Pod>, Err> split_entry(std::size_t n, std::string_view s) {
    int depth = 0;
    bool in_str = false;
    for (std::size_t i = 0; i < s.size(); ++i) {
        char c = s[i];
        if (c == '"' && (i == 0 || s[i - 1] != '\\')) in_str = !in_str;
        else if ((c == '(' || c == '[' || c == '{') && !in_str) ++depth;
        else if ((c == ')' || c == ']' || c == '}') && !in_str) --depth;
        else if (c == '=' && !in_str && depth == 0 && !(i + 1 < s.size() && (s[i + 1] == '=' || s[i + 1] == '>'))) {
            auto e = value(n, s.substr(0, i));
            if (!e) return unexpected(e.error());
            auto v = value(n, s.substr(i + 1));
            if (!v) return unexpected(v.error());
            return std::pair{*e, *v};
        }
    }
    return unexpected(at_line(n, "an entry is `element = value`"));
}

Expected<Target, Err> parse_target(std::size_t n, std::string_view s) {
    if (auto path = strip_prefix(trim(s), "at ")) {
        auto p = printed_path(n, *path);
        if (!p) return unexpected(p.error());
        return Target{*p, Pattern{}};
    }
    auto p = pattern(n, s);
    if (!p) return unexpected(p.error());
    return Target{std::nullopt, *p};
}

Expected<Check, Err> parse_check_line(std::size_t n, std::string_view c) {
    std::string_view word = c, rest;
    if (auto wr = split_once(c, " ")) std::tie(word, rest) = *wr;
    auto two = [&]() -> Expected<std::pair<Pattern, std::string>, Err> {
        auto parts = split_commas(rest);
        if (parts.size() != 2) return unexpected(at_line(n, "`" + std::string(word) + "` takes two operands"));
        auto p = pattern(n, parts[0]);
        if (!p) return unexpected(p.error());
        return std::pair{*p, std::string(trim(parts[1]))};
    };
    auto one = [&](Check::Kind k) -> Expected<Check, Err> {
        auto p = pattern(n, rest);
        if (!p) return unexpected(p.error());
        return Check{k, *p, {}, 0, {}};
    };
    if (word == "expect") return one(Check::Expect);
    if (word == "expect_none") return one(Check::ExpectNone);
    if (word == "expect_count") {
        auto t = two();
        if (!t) return unexpected(t.error());
        auto k = parse_number<std::size_t>(t->second);
        if (!k) return unexpected(at_line(n, "a count"));
        return Check{Check::ExpectCount, t->first, {}, *k, {}};
    }
    if (word == "expect_top") return one(Check::ExpectTop);
    if (word == "expect_top_none") return one(Check::ExpectTopNone);
    if (word == "requested") {
        auto t = two();
        if (!t) return unexpected(t.error());
        auto v = pattern(n, t->second);
        if (!v) return unexpected(v.error());
        return Check{Check::Requested, t->first, *v, 0, {}};
    }
    if (word == "unrequested") return one(Check::Unrequested);
    if (word == "returns") return one(Check::Returns);
    if (word == "fired") return one(Check::Fired);
    if (word == "quiet") return Check{Check::Quiet, {}, {}, 0, {}};
    if (word == "settles") return Check{Check::Settles, {}, {}, 0, {}};
    if (word == "aborted") return Check{Check::Aborted, {}, {}, 0, {}};
    if (word == "snapshot") {
        std::string_view f = trim(rest);
        while (f.starts_with('"')) f.remove_prefix(1);
        while (f.ends_with('"')) f.remove_suffix(1);
        return Check{Check::Snapshot, {}, {}, 0, std::string(f)};
    }
    return unexpected(at_line(n, "unknown check `" + std::string(word) + "`"));
}

Expected<Step, Err> parse_step(std::size_t n, std::string_view text) {
    auto parts = split_commas(text);
    std::vector<std::pair<Pod, Pod>> withs;
    std::vector<Delivery> deliveries;
    std::optional<StepKind> kind;
    for (std::size_t i = 0; i < parts.size(); ++i) {
        std::string_view part = trim(parts[i]);
        std::string_view word = part, rest;
        if (auto wr = split_once(part, " ")) {
            word = wr->first;
            rest = trim(wr->second);
        }
        if (word == "with") {
            auto e = split_entry(n, rest);
            if (!e) return unexpected(e.error());
            withs.push_back(*e);
        } else if (word == "build" && rest.empty()) {
            kind = StepKind{StepKind::Build, {}, {}, {}};
        } else if ((word == "tick" || word == "compute") && rest.empty()) {
            kind = StepKind{StepKind::Tick, {}, {}, {}};
        } else if (word == "compute" && rest == "until idle") {
            kind = StepKind{StepKind::ComputeUntilIdle, {}, {}, {}};
        } else if (word == "push") {
            auto e = split_entry(n, rest);
            if (!e) return unexpected(e.error());
            kind = StepKind{StepKind::Push, e->first, e->second, {}};
        } else if (word == "click") {
            if (auto path = strip_prefix(rest, "element ")) {
                auto p = printed_path(n, *path);
                if (!p) return unexpected(p.error());
                deliveries.push_back(Delivery{Delivery::Element, {}, {}, *p, {}});
            } else {
                auto t = parse_target(n, rest);
                if (!t) return unexpected(t.error());
                deliveries.push_back(Delivery{Delivery::Click, *t, {}, {}, {}});
            }
        } else if (word == "answer") {
            auto p = pattern(n, rest);
            if (!p) return unexpected(p.error());
            deliveries.push_back(Delivery{Delivery::Output, {}, {}, {}, *p});
        } else if (word == "send") {
            std::optional<std::string> v;
            if (i + 1 < parts.size()) {
                std::string_view first = trim_start(parts[i + 1]);
                first = first.substr(0, std::find_if(first.begin(), first.end(), ws) - first.begin());
                if (first != "send" && first != "click" && first != "answer" && first != "with") v = parts[i + 1];
            }
            if (!v) return unexpected(at_line(n, "send p, v"));
            auto t = parse_target(n, rest);
            if (!t) return unexpected(t.error());
            auto val = value(n, *v);
            if (!val) return unexpected(val.error());
            deliveries.push_back(Delivery{Delivery::Send, *t, *val, {}, {}});
            ++i;
        } else {
            return unexpected(at_line(n, "`" + std::string(part) + "`: a comma here needs parentheses (Appendix A.3)"));
        }
    }
    StepKind k;
    if (kind && deliveries.empty()) k = *kind;
    else if (!kind && !deliveries.empty()) k = StepKind{StepKind::Deliver, {}, {}, std::move(deliveries)};
    else if (!kind) return unexpected(at_line(n, "a step with no tick"));
    else return unexpected(at_line(n, "deliveries beside build, tick or push"));
    return Step{n, std::move(withs), std::move(k), {}};
}

Expected<TestBlock, Err> parse_test(std::size_t no, std::string_view head, std::span<const std::pair<std::size_t, std::string_view>> body) {
    if (!head.ends_with(':')) return unexpected(at_line(no, "a test line ends with :"));
    head.remove_suffix(1);
    auto parts = split_commas(head);
    if (parts.size() < 2) return unexpected(at_line(no, "test name, @program:"));
    auto program = strip_prefix(trim(parts[1]), "@");
    if (!program) return unexpected(at_line(no, "the program is written @name"));
    std::string_view prog = *program, first;
    if (auto pf = split_once(prog, " ")) std::tie(prog, first) = *pf;
    TestBlock t{std::string(trim(parts[0])), std::string(prog), {}, std::nullopt, {}};
    if (!trim(first).empty()) {
        auto v = value(no, first);
        if (!v) return unexpected(v.error());
        t.args.push_back(*v);
    }
    for (std::size_t i = 2; i < parts.size(); ++i) {
        if (auto n = strip_prefix(trim(parts[i]), "seed ")) {
            auto s = parse_number<std::uint64_t>(trim(*n));
            if (!s) return unexpected(at_line(no, "a seed is a whole number"));
            t.seed = *s;
        } else {
            auto v = value(no, parts[i]);
            if (!v) return unexpected(v.error());
            t.args.push_back(*v);
        }
    }
    std::size_t step_col = body.empty() ? 4 : indent(body[0].second);
    for (const auto& [n, l] : body) {
        std::string_view text = trim(l);
        if (indent(l) == step_col) {
            auto s = parse_step(n, text);
            if (!s) return unexpected(s.error());
            t.steps.push_back(std::move(*s));
            continue;
        }
        auto c = strip_prefix(text, "=>");
        if (t.steps.empty() || !c)
            return unexpected(at_line(n, "a check is `=> ...` under a step; write a long step on one line"));
        auto check = parse_check_line(n, trim(*c));
        if (!check) return unexpected(check.error());
        t.steps.back().checks.emplace_back(*check, n);
    }
    return t;
}

Expected<CheckBlock, Err> parse_check(std::size_t no, std::string_view head, std::span<const std::pair<std::size_t, std::string_view>> body) {
    if (!head.ends_with(':')) return unexpected(at_line(no, "a check line ends with :"));
    head.remove_suffix(1);
    CheckBlock c{std::string(trim(head)), {}};
    for (const auto& [n, raw] : body) {
        std::string_view l = trim(raw);
        if (auto ep = split_once(l, "=>")) {
            auto p = pattern(n, ep->second);
            if (!p) return unexpected(p.error());
            c.lines.push_back({std::string(trim(ep->first)), *p, n});
        } else {
            // A bare line is short for `=> True`.
            c.lines.push_back({std::string(l), Pattern{Pod::boolean(true)}, n});
        }
    }
    return c;
}

/// A line at the left edge of a test module (claude/modules-style-pass.md) beside its tests:
/// a line of its header (`emit`, `mod`, `test mod`, `use`), or a helper type, function or
/// value, its `derive` or signature line. The compiled program holds it, so the script
/// passes over it, body and all.
bool is_declaration(std::string_view head) {
    for (std::string_view k : {"emit ", "mod ", "test mod ", "use ", "type ", "derive ", "fn ", "let "})
        if (head.substr(0, k.size()) == k) return true;
    std::size_t colon = head.find(':');
    if (colon == std::string_view::npos || colon == 0) return false;
    std::string_view name = head.substr(0, colon);
    bool letter = false;
    for (char c : name) {
        bool alpha = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
        if (!alpha && !(c >= '0' && c <= '9') && std::string_view("_@$!?~&").find(c) == std::string_view::npos) return false;
        letter = letter || alpha;
    }
    return letter;
}

Expected<std::vector<Block>, Err> parse_script(std::string_view src) {
    Lines ls;
    std::size_t no = 0;
    for (std::string_view l : lines(src)) {
        ++no;
        std::string_view s = trim_end(strip_comment(l));
        if (!trim(s).empty()) ls.emplace_back(no, s);
    }
    std::vector<Block> blocks;
    for (std::size_t i = 0; i < ls.size();) {
        auto [n, head] = ls[i];
        std::size_t end = i + 1;
        while (end < ls.size() && indent(ls[end].second) != 0) ++end;
        std::span<const std::pair<std::size_t, std::string_view>> body(ls.data() + i + 1, end - i - 1);
        if (is_declaration(head)) {
            i = end;
            continue;
        }
        if (auto rest = strip_prefix(head, "test "); rest && indent(head) == 0) {
            auto t = parse_test(n, *rest, body);
            if (!t) return unexpected(t.error());
            blocks.emplace_back(std::move(*t));
        } else if (auto rest = strip_prefix(head, "check "); rest && indent(head) == 0) {
            auto c = parse_check(n, *rest, body);
            if (!c) return unexpected(c.error());
            blocks.emplace_back(std::move(*c));
        } else {
            return unexpected(at_line(n, "expected `test` or `check` at the left edge"));
        }
        i = end;
    }
    return blocks;
}

// ---- A test's run ------------------------------------------------------------------------

/// The Test World's ready state: the test's entries, then the policy for pictures.
struct TestReady : ReadyState {
    const std::map<Pod, Pod>& entries;
    explicit TestReady(const std::map<Pod, Pod>& e) : entries(e) {}
    std::optional<Pod> first_value(const Pod& element) const override {
        if (auto it = entries.find(element); it != entries.end()) return it->second;
        auto text = [&](std::string_view f) -> std::optional<Text> {
            const Pod* p = element.field(f);
            const Text* t = p ? p->as_text() : nullptr;
            if (!t) return std::nullopt;
            return *t;
        };
        const std::string* tag = element.tag();
        if (!tag) return std::nullopt;
        if (*tag == "Gui.Window") {
            auto t = text("title");
            if (!t) return std::nullopt;
            return Pod::choice("WindowId", {Pod::text(*t)});
        }
        if (*tag == "Gui.Metrics") {
            auto t = text("text");
            if (!t) return std::nullopt;
            return Pod::record("Extent", {{"w", Pod::floating(9.0 * double(t->len()))}, {"h", Pod::floating(14.0)}});
        }
        return std::nullopt;
    }
};

/// What a step's last tick left, for its checks.
struct Ticked {
    Result<TickOut> out;
    Document before;
};

struct Deck {
    std::vector<Pod> cards;
    std::size_t next = 0;
    std::size_t pending = 0;
};

struct Run {
    const TestWorld::Builder* builder;
    std::vector<Pod> args;
    std::unique_ptr<Program> program;
    /// The document as the World has it, from the patches.
    Document doc;
    std::map<Pod, Pod> ready;
    std::size_t next_tick = 0;
    bool pending = false;
    /// The prelude's policy: the answers `Prelude.Unwrap` and `Prelude.Exceptions` are
    /// owed, and what is poisoned.
    Prelude prelude;
    std::optional<Pod> last_result;
    /// Each `Gui.Window` place's id, as its first value answered it.
    std::map<Path, Pod> windows;
    std::optional<XorShift> rng;
    /// Each `Cards.Deck` place's shuffle, the next card, and its unanswered requests.
    std::map<Path, Deck> decks;
    /// Every tick that closed, its input and the ready state it ran under, the build's
    /// first: replayed onto a new build after a tick that aborts (Conformance AB).
    std::vector<std::pair<TickIn, std::map<Pod, Pod>>> log;

    Expected<Unit, Err> step(const Step& step, std::string& out, std::vector<std::string>& failures, const std::string& test) {
        for (const auto& [e, v] : step.withs) ready.insert_or_assign(e, v);
        if (step.kind.kind != StepKind::Build && !program)
            return unexpected(Err(next_tick == 0 ? "the first step of a test is `build`" : "an earlier tick aborted"));
        std::optional<Ticked> ticked;
        switch (step.kind.kind) {
        case StepKind::Build:
            if (next_tick > 0) return unexpected(Err("`build` after the program was built"));
            ticked = one_tick(TickIn::none(), {}, out);
            break;
        case StepKind::Tick:
            ticked = one_tick(TickIn::none(), {}, out);
            break;
        case StepKind::ComputeUntilIdle: {
            auto t = until_idle(out);
            if (!t) return unexpected(t.error());
            ticked = std::move(*t);
            break;
        }
        case StepKind::Push: {
            const Pod &e = step.kind.element, &v = step.kind.value;
            ready.insert_or_assign(e, v);
            std::vector<std::string> shown{e.to_string() + " = " + v.to_string()};
            ticked = one_tick(TickIn::none().push(e, v), shown, out);
            break;
        }
        case StepKind::Deliver: {
            TickIn input;
            for (const Delivery& d : step.kind.deliveries) {
                Expected<std::pair<Path, Pod>, Err> pv = unexpected(Err());
                switch (d.kind) {
                case Delivery::Output: pv = deal(d.pattern); break;
                case Delivery::Element: pv = click_element(d.element); break;
                case Delivery::Send: {
                    auto p = place(d.target);
                    if (!p) return unexpected(p.error());
                    pv = std::pair{*p, d.value};
                    break;
                }
                case Delivery::Click: {
                    auto p = place(d.target);
                    if (!p) return unexpected(p.error());
                    pv = std::pair{*p, Pod::name("click")};
                    break;
                }
                }
                if (!pv) return unexpected(pv.error());
                auto& [path, value] = *pv;
                if (std::any_of(input.answers.begin(), input.answers.end(), [&](const auto& a) { return a.first == path; }))
                    return unexpected("two deliveries to one place, " + path.to_string());
                input.answers.emplace_back(path, value);
            }
            std::stable_sort(input.answers.begin(), input.answers.end(), [](const auto& a, const auto& b) { return a.first < b.first; });
            ticked = one_tick(std::move(input), {}, out);
            break;
        }
        }
        check(step, *ticked, failures, test);
        return Unit{};
    }

    Expected<Ticked, Err> until_idle(std::string& out) {
        std::size_t first = next_tick, count = 0;
        std::string ticks;
        Ticked last{TickOut{}, doc};
        while (pending) {
            if (count == COMPUTE_PATIENCE)
                return unexpected("`compute until idle` ran out of patience after " + std::to_string(count) + " ticks");
            std::string one;
            last = one_tick(TickIn::none(), {}, one);
            // A tick with every section empty prints only in the count.
            if (lines(one).size() > 2) ticks += one;
            ++count;
            if (!last.out) break;
        }
        if (count == 0) out += "\ncompute until idle: 0 ticks\n";
        else if (count == 1) out += "\ncompute until idle: 1 tick, " + std::to_string(first) + " to " + std::to_string(first) + "\n";
        else
            out += "\ncompute until idle: " + std::to_string(count) + " ticks, " + std::to_string(first) + " to " +
                   std::to_string(first + count - 1) + "\n";
        out += ticks;
        return last;
    }

    /// The one place a `send` or `click` names, in the document as the tick opens.
    Expected<Path, Err> place(const Target& t) const {
        if (t.at) {
            for (const auto& [p, _] : doc)
                if (p.to_string() == *t.at) return p;
            return unexpected("no place at " + *t.at);
        }
        std::vector<Path> found;
        for (const auto& [p, e] : doc)
            if (t.pattern.matches(e)) found.push_back(p);
        if (found.size() == 1) return found[0];
        if (found.empty()) return unexpected("no place matches Pattern(" + t.pattern.pod.to_string() + ")");
        std::vector<std::string> names;
        for (const auto& p : found) names.push_back(p.to_string());
        return unexpected(std::to_string(found.size()) + " places match: " + join(names, ", "));
    }

    /// `answer p`: the next card of the matching deck place's shuffle.
    Expected<std::pair<Path, Pod>, Err> deal(const Pattern& p) {
        auto path = place(Target{std::nullopt, p});
        if (!path) return unexpected(path.error());
        if (!rng) return unexpected(Err("`answer` deals from the test's seed, and this test names none"));
        auto it = decks.find(*path);
        if (it == decks.end()) return unexpected(path->to_string() + " is not a Cards.Deck place");
        Deck& d = it->second;
        if (d.pending == 0) return unexpected("the deck at " + path->to_string() + " has no pending request to answer");
        if (d.next >= d.cards.size()) return unexpected("the deck at " + path->to_string() + " has dealt all 52 cards");
        Pod card = d.cards[d.next];
        ++d.next;
        --d.pending;
        return std::pair{*path, card};
    }

    /// `click element path`: a `Press` at the centre of the one `Draw.Clickable` naming
    /// `path` in the windows' last scenes, to the window the scene draws.
    Expected<std::pair<Path, Pod>, Err> click_element(const std::string& target) const {
        std::vector<std::string> ws;
        for (const auto& [p, id] : windows) ws.push_back(p.to_string() + " (" + id.to_string() + ")");
        std::string searched = join(ws, ", ");
        auto found = find_clickables(doc, target);
        if (found.empty()) return unexpected("no Draw.Clickable names " + target + "; searched " + searched);
        if (found.size() > 1)
            return unexpected(std::to_string(found.size()) + " Draw.Clickables name " + target + "; searched " + searched);
        const Clickable& c = found[0];
        std::vector<Path> matching;
        for (const auto& [p, id] : windows)
            if (id == c.window) matching.push_back(p);
        if (matching.size() != 1)
            return unexpected("the scene at " + c.scene.to_string() + " draws for " + c.window.to_string() + ", and " +
                              std::to_string(matching.size()) + " windows have that id");
        Pod point = Pod::record("Point", {{"x", Pod::floating(c.x)}, {"y", Pod::floating(c.y)}});
        return std::pair{matching[0], Pod::choice("Press", {point})};
    }

    /// Run one tick, print it, and lend the program its compact round. The tick's answers
    /// are `input`'s and the ones the prelude's policy owes, in path order, less those to
    /// a poisoned place; `pushed` is what `in:` shows before them.
    Ticked one_tick(TickIn input, std::span<const std::string> pushed, std::string& out) {
        Document before = doc;
        std::vector<std::pair<Path, Pod>> owed = prelude.take_outputs();
        input.answers.insert(input.answers.end(), owed.begin(), owed.end());
        std::erase_if(input.answers, [&](const auto& a) { return prelude.is_poisoned(a.first); });
        std::stable_sort(input.answers.begin(), input.answers.end(), [](const auto& a, const auto& b) { return a.first < b.first; });
        std::vector<std::string> shown(pushed.begin(), pushed.end());
        for (const auto& [p, v] : input.answers) shown.push_back(p.to_string() + ": " + v.to_string());
        TestReady r(ready);
        std::pair<TickIn, std::map<Pod, Pod>> logged{input, ready};
        Result<TickOut> result = unexpected(Fault("", ""));
        if (program) {
            result = program->tick(input, r);
        } else {
            auto built = (*builder)(args, r);
            if (built) {
                program = std::move(built.value().first);
                result = std::move(built.value().second);
            } else result = unexpected(built.error());
        }
        print_tick(out, next_tick, shown, result);
        ++next_tick;
        if (result) {
            const TickOut& o = *result;
            // The prelude's tags, as a World performs them: the requests against the
            // document as the tick opened and the tick's newborns.
            prelude.track(o.patch);
            for (const auto& r : o.requests) {
                if (prelude.is_poisoned(r.path)) continue;
                const Pod* element = element_of(before, o, r.path);
                const std::string* tag = element ? element->tag() : nullptr;
                if (tag && *tag == "Prelude.Unwrap") prelude.unwrap(r.path, r.value);
            }
            doc.apply(o.patch);
            pending = o.pending || prelude.owes();
            for (const auto& outcome : o.results)
                if (outcome.kind == Outcome::Kind::Value) last_result = outcome.pod;
            observe(o);
            if (program) program->compact();
            log.push_back(std::move(logged));
        } else {
            // A tick that aborts leaves the state before it (Conformance AB): the program
            // built again and the ticks that closed run again, their output already had.
            prelude.give_back(std::move(owed));
            program = replay();
        }
        return Ticked{std::move(result), std::move(before)};
    }

    /// The program as the last tick that closed left it: built, then every logged tick
    /// run again under the ready state it ran under; null when there is none.
    std::unique_ptr<Program> replay() const {
        if (log.empty()) return nullptr;
        auto built = (*builder)(args, TestReady(log.front().second));
        if (!built) return nullptr;
        std::unique_ptr<Program> p = std::move(built.value().first);
        for (std::size_t i = 1; i < log.size(); ++i) {
            if (!p->tick(log[i].first, TestReady(log[i].second))) return nullptr;
            p->compact();
        }
        return p;
    }

    /// What the policies learn from a tick: windows' ids, decks' shuffles and requests.
    void observe(const TickOut& o) {
        for (const auto& [p, _] : o.patch.removed) {
            windows.erase(p);
            decks.erase(p);
        }
        for (const auto& [p, e] : o.patch.added) {
            const std::string* tag = e.tag();
            if (!tag) continue;
            if (*tag == "Gui.Window") {
                if (auto id = TestReady(ready).first_value(e)) windows.insert_or_assign(p, *id);
            } else if (*tag == "Cards.Deck") {
                if (rng) decks.insert_or_assign(p, Deck{rng->shuffle(), 0, 0});
            }
        }
        for (const auto& r : o.requests)
            if (auto it = decks.find(r.path); it != decks.end()) ++it->second.pending;
    }

    void check(const Step& step, const Ticked& t, std::vector<std::string>& failures, const std::string& test) const {
        auto all = doc.elements();
        auto top = [&](const Pattern& p) {
            for (const auto& [path, e] : doc)
                if (path.len() == 1 && p.matches(e)) return true;
            return false;
        };
        bool expected_abort = false;
        for (const auto& [check, line] : step.checks) {
            std::string at = "test " + test + " line " + std::to_string(line) + ": ";
            if (!t.out) {
                expected_abort |= check.kind == Check::Aborted;
                if (check.kind != Check::Aborted) failures.push_back(at + check.debug() + " after an aborted tick");
                continue;
            }
            const TickOut& o = *t.out;
            auto fired = [&](const Pattern& p) {
                for (const auto& outcome : o.results)
                    if (outcome.kind == Outcome::Kind::Fired && p.matches(outcome.pod)) return true;
                return false;
            };
            auto requested = [&](const Pattern& p, const Pattern& v) {
                for (const auto& r : o.requests) {
                    const Pod* place = doc.get(r.path);
                    if (!place) place = t.before.get(r.path);
                    if (place && p.matches(*place) && v.matches(r.value)) return true;
                }
                return false;
            };
            auto any = [&](const Pattern& p) { return std::any_of(all.begin(), all.end(), [&](const Pod* e) { return p.matches(*e); }); };
            bool ok = false;
            switch (check.kind) {
            case Check::Aborted: ok = false; break;
            case Check::Settles: ok = true; break;
            case Check::Expect: ok = any(check.p); break;
            case Check::ExpectNone: ok = !any(check.p); break;
            case Check::ExpectCount:
                ok = std::size_t(std::count_if(all.begin(), all.end(), [&](const Pod* e) { return check.p.matches(*e); })) == check.count;
                break;
            case Check::ExpectTop: ok = top(check.p); break;
            case Check::ExpectTopNone: ok = !top(check.p); break;
            case Check::Requested: ok = requested(check.p, check.q); break;
            case Check::Unrequested: ok = !requested(check.p, Pattern::any()); break;
            case Check::Returns: ok = last_result && check.p.matches(*last_result); break;
            case Check::Fired: ok = fired(check.p); break;
            case Check::Quiet: ok = !fired(Pattern::any()); break;
            case Check::Snapshot:
                failures.push_back(at + "snapshot \"" + check.file + "\": this World draws no SVG");
                continue;
            }
            if (!ok) failures.push_back(at + check.debug() + " did not hold");
        }
        if (!t.out && !expected_abort)
            failures.push_back("test " + test + " line " + std::to_string(step.line) + ": the tick aborted: " + t.out.error().to_string());
    }
};

}  // namespace

// ---- XorShift -----------------------------------------------------------------------------

std::uint64_t XorShift::next_output() {
    std::uint64_t x = s_;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    s_ = x;
    return x * 0x2545F4914F6CDD1Dull;
}

std::vector<Pod> XorShift::shuffle() {
    static const char* suits[] = {"Clubs", "Diamonds", "Hearts", "Spades"};
    static const char* ranks[] = {"Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Jack", "Queen", "King", "Ace"};
    std::vector<Pod> deck;
    for (const char* s : suits)
        for (const char* r : ranks) deck.push_back(Pod::record("Card", {{"rank", Pod::name(r)}, {"suit", Pod::name(s)}}));
    for (std::size_t i = deck.size() - 1; i >= 1; --i) {
        std::size_t j = std::size_t(next_output() % (std::uint64_t(i) + 1));
        std::swap(deck[i], deck[j]);
    }
    return deck;
}

// ---- The transcript -----------------------------------------------------------------------

std::string strip_comment_header(std::string_view s) {
    while (!s.empty()) {
        auto i = s.find('\n');
        std::string_view l = s.substr(0, i == std::string_view::npos ? s.size() : i + 1);
        if (!(l.starts_with("# ") || trim_end(l) == "#")) break;
        s.remove_prefix(l.size());
    }
    return std::string(s);
}

void print_tick(std::string& out, std::size_t n, std::span<const std::string> inputs, const Result<TickOut>& result) {
    out += "\ntick " + std::to_string(n) + "\n";
    auto section = [&](const char* name, const std::vector<std::string>& ls) {
        if (ls.empty()) return;
        out += "  " + std::string(name) + ":\n";
        for (const auto& l : ls) out += "    " + l + "\n";
    };
    section("in", std::vector<std::string>(inputs.begin(), inputs.end()));
    if (!result) {
        out += "  aborted\n  | " + result.error().to_string() + "\n";
        return;
    }
    const TickOut& o = *result;
    auto places = [](const std::vector<std::pair<Path, Pod>>& ps) {
        std::vector<std::string> ls;
        for (const auto& [p, e] : ps) ls.push_back(p.to_string() + ": " + e.to_string());
        return ls;
    };
    section("added", places(o.patch.added));
    section("changed", places(o.patch.changed));
    section("torn down", places(o.patch.removed));
    std::vector<std::string> rs;
    for (const auto& r : o.requests) rs.push_back(r.to_string());
    section("requested", rs);
    for (const auto& outcome : o.results) {
        if (outcome.kind == Outcome::Kind::Value) out += "  result: " + outcome.pod.to_string() + "\n";
        else out += "  result: fired: " + outcome.pod.to_string() + "\n";
    }
}

std::string diff(std::string_view expected, std::string_view got) {
    auto e = lines(expected), g = lines(got);
    std::string out;
    for (std::size_t i = 0; i < std::max(e.size(), g.size()); ++i) {
        if (i < e.size() && i < g.size() && e[i] == g[i]) {
            out += "  " + std::string(e[i]) + "\n";
            continue;
        }
        if (i < e.size()) out += "- " + std::string(e[i]) + "\n";
        if (i < g.size()) out += "+ " + std::string(g[i]) + "\n";
    }
    return out;
}

// ---- TestWorld ----------------------------------------------------------------------------

Report TestWorld::run(std::string_view script) const {
    auto blocks = parse_script(script);
    if (!blocks) return Report{"", {blocks.error()}, 2};
    std::string out;
    std::vector<std::string> failures;
    for (std::size_t i = 0; i < blocks->size(); ++i) {
        if (i > 0) out += '\n';
        if (auto t = std::get_if<TestBlock>(&(*blocks)[i])) {
            out += "test " + t->name + "\n";
            auto b = programs_.find(t->program);
            if (b == programs_.end()) {
                failures.push_back("test " + t->name + ": no program named @" + t->program);
                continue;
            }
            Run run{&b->second, t->args, nullptr, Document(), {}, 0, false, {}, std::nullopt, {}, std::nullopt, {}, {}};
            if (t->seed) run.rng.emplace(*t->seed);
            for (const Step& step : t->steps) {
                auto r = run.step(step, out, failures, t->name);
                if (!r) {
                    failures.push_back("test " + t->name + " line " + std::to_string(step.line) + ": " + r.error());
                    break;
                }
            }
        } else {
            const CheckBlock& c = std::get<CheckBlock>((*blocks)[i]);
            out += "check " + c.name + "\n";
            std::optional<std::vector<Result<Pod>>> compiled;
            if (auto f = checks_.find(c.name); f != checks_.end()) compiled = f->second();
            for (std::size_t k = 0; k < c.lines.size(); ++k) {
                const auto& [expr, pat, line] = c.lines[k];
                Expected<Pod, std::string> v = unexpected(std::string());
                if (compiled) {
                    if (k < compiled->size() && (*compiled)[k]) v = *(*compiled)[k];
                    else v = unexpected(k < compiled->size() ? (*compiled)[k].error().to_string() : Fault("Test.Check", "no compiled line").to_string());
                } else v = read::value(expr);
                if (v && pat.matches(*v)) {
                    out += std::to_string(k + 1) + ": ok\n";
                } else {
                    std::string shown = v ? v->to_string() : "(error: " + v.error() + ")";
                    out += std::to_string(k + 1) + ": " + shown + "\n";
                    failures.push_back("check " + c.name + " line " + std::to_string(line) + ": " + expr + " was " + shown);
                }
            }
        }
    }
    std::uint8_t code = failures.empty() ? 0 : 1;
    return Report{std::move(out), std::move(failures), code};
}

Expected<Report, std::string> TestWorld::verify(std::string_view script, std::string_view expected) const {
    Report report = run(script);
    std::string want = strip_comment_header(expected);
    if (report.exit_code == 2) return unexpected("the script did not parse: " + join(report.failures, "; "));
    if (report.transcript != want) return unexpected("the transcript differs:\n" + diff(want, report.transcript));
    if (!report.failures.empty()) return unexpected("checks failed:\n" + join(report.failures, "\n"));
    return report;
}

}  // namespace prism_rtl
