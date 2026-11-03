// The Test World against the books' transcripts, byte for byte: the counter and the half
// counter (prism_runtime_lib/tests/counter.rs) and `@Normal` with `Normalize` through
// `@Run` (tests/normalize.rs), written by hand as docs/backend.md says the templates emit
// them, in C++; and runtime.rs's Test World rules.

#include <fstream>
#include <set>
#include <sstream>

#include "check.hpp"
#include "prism_rtl/prism_rtl.hpp"
#include "prism_rtl/test_world.hpp"

using namespace prism_rtl;

namespace {

std::string slurp(const std::string& name) {
    std::ifstream f(std::string(PRISM_TESTS_DIR) + "/conformance/" + name);
    if (!f) check::fail(__FILE__, __LINE__, "cannot read " + name);
    std::stringstream s;
    s << f.rdbuf();
    return s.str();
}

// ---- The `Ui` family, as its tag declarations derive `^Pod` -----------------------------

/// `tag Ui.Button of Element !Click: label: $Text`: the `Level` the patch sends.
struct UiButton {
    Text label;
    Pod to_pod() const { return Pod::element("Ui.Button", {{"label", prism_rtl::to_pod(label)}}); }
};

/// `tag Ui.Label of Element: text: $Text`.
struct UiLabel {
    Text text;
    Pod to_pod() const { return Pod::element("Ui.Label", {{"text", prism_rtl::to_pod(text)}}); }
};

/// `type Click = unit type click`: a button's answer.
struct Click {
    static Result<Click> from_pod(const Pod& p) {
        if (auto c = p.if_choice(); c && c->ctor == "click" && c->args.empty()) return Click{};
        return mismatch<Click>("click", p);
    }
};

// ---- counter.prism ----------------------------------------------------------------------
//
// @counter:
//     !inc <- Ui.Button(label=$"Increment")                 # 0
//     !dec <- Ui.Button(label=$"Decrement")                 # 1
//     $count <- 0 then $count + (-1 on !dec <| +1 on !inc)
//     Ui.Label(text=$"Count: {$count}")                     # 2

/// `@counter`: a static widget, one node, one state. The merge inside it records its
/// inputs and decides in the node's one wake (Compiler ch. 3, option (c)).
struct Counter : Node {
    /// The merge's distance from the node's inputs.
    static constexpr std::uint32_t WAKE = 1;
    Path path;
    /// `$count`.
    Hold<Int> count;
    /// `!inc` and `!dec` as the merge's stamped scratch, never cleared.
    Stamped<Click> inc, dec;

    Counter(Path p, const Cx& cx) : path(std::move(p)), count(Int(0), cx) {}

    static std::shared_ptr<Counter> make(Path path, Cx& cx) {
        auto node = std::make_shared<Counter>(std::move(path), cx);
        cx.added(node->path.at(0), to_pod(UiButton{"Increment"}));
        cx.added(node->path.at(1), to_pod(UiButton{"Decrement"}));
        cx.added(node->path.at(2), node->label(cx));
        return node;
    }
    /// `!inc`: an answer at place 0 records and asks for the wake.
    void on_inc(Click v, Cx& cx) {
        inc.set(v, cx);
        cx.wake(WAKE, me());
    }
    /// `!dec`: an answer at place 1.
    void on_dec(Click v, Cx& cx) {
        dec.set(v, cx);
        cx.wake(WAKE, me());
    }
    /// `!delta`, the merge's firing: `$count + !delta`, then the hold's write.
    void delta(const Int& d, Cx& cx) {
        count.set(count.get(cx) + d, cx);
        cx.dirty(me());  // the label reads `$count`
    }
    /// Place 2's `Level`, pulled after the tick.
    Pod label(const Cx& cx) const { return to_pod(UiLabel{Text("Count: " + count.get(cx).to_string())}); }

    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        if (at.size() == 1 && at[0].is_pos(0)) {
            PRISM_TRY_LET(c, from_pod<Click>(v));
            on_inc(c, cx);
        } else if (at.size() == 1 && at[0].is_pos(1)) {
            PRISM_TRY_LET(c, from_pod<Click>(v));
            on_dec(c, cx);
        } else return unexpected(Fault::answer("no answering place at " + path_of(at).to_string()));
        return {};
    }
    /// `-1 on !dec <| +1 on !inc`: the left wins.
    Done wake(Cx& cx) override {
        if (dec.fired(cx)) delta(Int(-1), cx);
        else if (inc.fired(cx)) delta(Int(1), cx);
        return {};
    }
    Done patch(Cx& cx) override {
        cx.changed(path.at(2), label(cx));
        return {};
    }
};

/// The program: the root node and the runtime.
struct CounterProgram : Program {
    Runtime rt;
    std::shared_ptr<Counter> root;
    static Built<CounterProgram> build(std::span<const Pod>, const ReadyState& ready) {
        auto p = std::make_unique<CounterProgram>();
        Cx cx = p->rt.build(ready);
        p->root = Counter::make(Path::root(), cx);
        PRISM_TRY_LET(out, p->rt.close(std::move(cx)));
        return std::pair{std::move(p), std::move(out)};
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
    void compact() override { rt.compact(); }
};

// ---- half-counter.prism -----------------------------------------------------------------
//
// @halfcounter:
//     !increment <- Ui.Button(label=$"Increment")          # 0
//     let !delta = +1 on !increment
//     $count <- 0 then $count + !delta
//     Ui.Label(text=$"Count: {$count}")                    # 1

/// `@halfcounter`: no combinator, so no wake. `increment` calls `delta`, which calls the
/// setter (Compiler ch. 3, "The half counter").
struct HalfCounter : Node {
    Path path;
    Hold<Int> count;
    HalfCounter(Path p, const Cx& cx) : path(std::move(p)), count(Int(0), cx) {}
    static std::shared_ptr<HalfCounter> make(Path path, Cx& cx) {
        auto node = std::make_shared<HalfCounter>(std::move(path), cx);
        cx.added(node->path.at(0), to_pod(UiButton{"Increment"}));
        cx.added(node->path.at(1), node->label(cx));
        return node;
    }
    void increment(Click, Cx& cx) { delta(Int(1), cx); }
    void delta(const Int& d, Cx& cx) {
        count.set(count.get(cx) + d, cx);
        cx.dirty(me());
    }
    Pod label(const Cx& cx) const { return to_pod(UiLabel{Text("Count: " + count.get(cx).to_string())}); }
    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        if (at.size() == 1 && at[0].is_pos(0)) {
            PRISM_TRY_LET(c, from_pod<Click>(v));
            increment(c, cx);
            return {};
        }
        return unexpected(Fault::answer("no answering place at " + path_of(at).to_string()));
    }
    Done patch(Cx& cx) override {
        cx.changed(path.at(1), label(cx));
        return {};
    }
};

struct HalfCounterProgram : Program {
    Runtime rt;
    std::shared_ptr<HalfCounter> root;
    static Built<HalfCounterProgram> build(std::span<const Pod>, const ReadyState& ready) {
        auto p = std::make_unique<HalfCounterProgram>();
        Cx cx = p->rt.build(ready);
        p->root = HalfCounter::make(Path::root(), cx);
        PRISM_TRY_LET(out, p->rt.close(std::move(cx)));
        return std::pair{std::move(p), std::move(out)};
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
};

TestWorld counters() { return TestWorld().program<CounterProgram>("counter").program<HalfCounterProgram>("halfcounter"); }

void verify(const TestWorld& w, const std::string& name) {
    auto r = w.verify(slurp(name + ".prismtest"), slurp(name + ".expected"));
    if (!r) check::fail(__FILE__, __LINE__, name + ": " + r.error());
}

}  // namespace

TEST(two_clicks_in_one_tick_print_count_minus_one) { verify(counters(), "sem-ch01-counter"); }
TEST(the_conformance_counter_counts) { verify(counters(), "conf-ch01-counter"); }
TEST(the_example_counter_keeps_the_left) { verify(counters(), "counter"); }
TEST(the_half_counter_calls_straight_through) { verify(counters(), "compiler-ch03-halfcounter"); }

TEST(the_semantics_transcript_is_byte_for_byte) {
    Report report = counters().run(slurp("sem-ch01-counter.prismtest"));
    CHECK(report.failures.empty());
    CHECK(report.transcript.ends_with("  changed:\n    2: Ui.Label(text=\"Count: -1\")\n"));
}

TEST(a_failed_check_is_reported_beside_the_transcript) {
    Report report = counters().run("test wrong, @counter:\n    build\n        => expect Ui.Label(text=\"Count: 9\")\n");
    CHECK_EQ(int(report.exit_code), 1);
    CHECK(!report.failures.empty() && report.failures[0].find("line 3") != std::string::npos);
}

TEST(an_answer_to_no_place_aborts_the_tick) {
    Report report = counters().run("test nowhere, @counter:\n    build\n    click at 2\n        => aborted\n    tick\n");
    CHECK(report.transcript.find("tick 1\n  in:\n    2: click\n  aborted\n  | Runtime.Answer") != std::string::npos);
    CHECK_EQ(report.failures.size(), 1u);
    CHECK(!report.failures.empty() && report.failures[0].find("an earlier tick aborted") != std::string::npos);
}

TEST(a_static_widget_owes_no_tick) {
    ReadyMap ready;
    auto built = CounterProgram::build({}, ready);
    CHECK(!built.value().second.pending);
    CHECK_EQ(built.value().second.patch.added.size(), 3u);
}

// ---- type Term = choice: Var Nat | Lam Term | App Term, Term; derive ^Eq, ^Show ---------

namespace {

struct Term;
using TermRef = std::shared_ptr<const Term>;

struct Term {
    enum Kind { Var, Lam, App } kind;
    Nat n;
    TermRef a, b;
    static Term var(Nat n) { return Term{Var, std::move(n), nullptr, nullptr}; }
    static Term lam(Term b) { return Term{Lam, Nat(), std::make_shared<const Term>(std::move(b)), nullptr}; }
    static Term app(TermRef f, TermRef x) { return Term{App, Nat(), std::move(f), std::move(x)}; }
    friend bool operator==(const Term& x, const Term& y) {
        if (x.kind != y.kind) return false;
        if (x.kind == Var) return x.n == y.n;
        if (x.kind == Lam) return *x.a == *y.a;
        return *x.a == *y.a && *x.b == *y.b;
    }

    /// Derived `^Show`: the printer decides the parentheses.
    void show(Printer& p) const {
        switch (kind) {
        case Var: return p.choice("Var", {n});
        case Lam: return p.choice("Lam", {a});
        case App: return p.choice("App", {a, b});
        }
    }
    /// Derived `^Pod`, both ways: a root argument arrives as plain data.
    Pod to_pod() const {
        switch (kind) {
        case Var: return Pod::choice("Var", {prism_rtl::to_pod(n)});
        case Lam: return Pod::choice("Lam", {prism_rtl::to_pod(a)});
        case App: return Pod::choice("App", {prism_rtl::to_pod(a), prism_rtl::to_pod(b)});
        }
        return Pod();
    }
    static Result<Term> from_pod(const Pod& whole) {
        const Pod& p = whole.structure();
        auto c = p.if_choice();
        if (c && c->ctor == "Var" && c->args.size() == 1) {
            PRISM_TRY_LET(n, prism_rtl::from_pod<Nat>(c->args[0]));
            return var(n);
        }
        if (c && c->ctor == "Lam" && c->args.size() == 1) {
            PRISM_TRY_LET(b, prism_rtl::from_pod<TermRef>(c->args[0]));
            return Term{Lam, Nat(), b, nullptr};
        }
        if (c && c->ctor == "App" && c->args.size() == 2) {
            PRISM_TRY_LET(f, prism_rtl::from_pod<TermRef>(c->args[0]));
            PRISM_TRY_LET(x, prism_rtl::from_pod<TermRef>(c->args[1]));
            return app(f, x);
        }
        return mismatch<Term>("a Term", p);
    }
};

TermRef boxed(Term t) { return std::make_shared<const Term>(std::move(t)); }

// ---- The plain functions ----------------------------------------------------------------

/// `Shift: fn Int, Nat, Term -> Term`.
Term shift(const Int& d, const Nat& cut, const Term& t) {
    switch (t.kind) {
    case Term::Var:
        if (t.n < cut) return t;
        return Term::var(Nat::from_int(t.n.as_int() + d).value_or(Nat(0)));
    case Term::Lam: return Term::lam(shift(d, cut + Nat(1), *t.a));
    case Term::App: return Term::app(boxed(shift(d, cut, *t.a)), boxed(shift(d, cut, *t.b)));
    }
    return t;
}

/// `Subst: fn Nat, Term, Term -> Term`.
Term subst(const Nat& j, const Term& s, const Term& t) {
    switch (t.kind) {
    case Term::Var: return t.n == j ? s : t;
    case Term::Lam: return Term::lam(subst(j + Nat(1), shift(Int(1), Nat(0), s), *t.a));
    case Term::App: return Term::app(boxed(subst(j, s, *t.a)), boxed(subst(j, s, *t.b)));
    }
    return t;
}

/// `?Reduce: fn Term -> ?Term`: one leftmost-outermost reduction.
std::optional<Term> reduce(const Term& t) {
    switch (t.kind) {
    case Term::Var: return std::nullopt;
    case Term::Lam:
        if (auto b = reduce(*t.a)) return Term::lam(*b);
        return std::nullopt;
    case Term::App:
        if (t.a->kind == Term::Lam) return shift(Int(-1), Nat(0), subst(Nat(0), shift(Int(1), Nat(0), *t.b), *t.a->a));
        if (auto f = reduce(*t.a)) return Term::app(boxed(*f), t.b);
        if (auto x = reduce(*t.b)) return Term::app(t.a, boxed(*x));
        return std::nullopt;
    }
    return std::nullopt;
}

// ---- Normalize: unbounded fn Term -> Term, as a step machine ----------------------------
//
// fn Normalize t = match ?Reduce t:
//     ? t2: Normalize t2          # a cycle point: an unbounded call from an unbounded fn
//     nil: t

/// The job stopped at a cycle point: `Normalize` about to be entered with `t`. The call
/// is in tail position, so its continuation is "return what it returns", and the job
/// needs no stack.
struct Normalize : Job<Term> {
    Term t;
    explicit Normalize(Term t) : t(std::move(t)) {}
    Turn<Term> cycle() const override {
        if (auto t2 = reduce(t)) return Turn<Term>::more(std::make_shared<const Normalize>(*t2));
        return Turn<Term>::finished(t);
    }
};

// ---- @Normal ----------------------------------------------------------------------------
//
// fn @Normal term:
//     !done <- @Run Normalize, term                           # 0
//     $shown <- "working" then Describe !done
//     Ui.Label(text=$shown)                                   # 1
//
// with `@Run` (Semantics Appendix B) in a static position, so its fields are merged in.

struct Normal : Node {
    /// The merge `!started <| !again` decides one height above the inputs.
    static constexpr std::uint32_t WAKE = 1;
    Path path;
    /// `@Run`'s `!started`, a stamped input of the merge.
    Stamped<Unit> run_started;
    /// `@Run`'s `!again`.
    Defer<Unit> run_again;
    /// `@Machine`'s state: the job.
    Hold<JobRef<Term>> run_job;
    /// `$shown`.
    Hold<Text> shown;

    Normal(Path p, Term term, const Cx& cx)
        : path(std::move(p)), run_job(std::make_shared<const Normalize>(std::move(term)), cx), shown(Text("working"), cx) {}

    static std::shared_ptr<Normal> make(Path path, Term term, Cx& cx) {
        auto node = std::make_shared<Normal>(std::move(path), std::move(term), cx);
        cx.on_started(node);  // `@Run` binds `@started`: the next tick is owed
        cx.added(node->path.at(1), node->label(cx));
        return node;
    }
    /// `!done` fired: `Describe !done` writes `$shown`, which the label reads.
    void done(const Term& t, Cx& cx) {
        shown.set(show_text(t), cx);
        cx.dirty(me());
    }
    Pod label(const Cx& cx) const { return Pod::element("Ui.Label", {{"text", to_pod(shown.get(cx))}}); }

    Done started(Cx& cx) override {
        run_started.set(Unit{}, cx);
        cx.wake(WAKE, me());
        return {};
    }
    Done deferred(Cx& cx) override {
        cx.wake(WAKE, me());
        return {};
    }
    /// `!turns <- @Machine Cycle, job, (!started <| !again)`: one cycle per firing; a
    /// `More` arms `!again`, a `Finished` fires `!done`.
    Done wake(Cx& cx) override {
        if (!run_started.fired(cx) && !run_again.fired(cx)) return {};
        Turn<Term> turn = run_job.get(cx)->cycle();
        if (auto m = turn.if_more()) {
            run_job.set(m->next, cx);
            run_again.arm(Unit{}, me(), cx);
        } else done(turn.if_finished()->value, cx);
        return {};
    }
    Done patch(Cx& cx) override {
        cx.changed(path.at(1), label(cx));
        return {};
    }
};

struct NormalProgram : Program {
    Runtime rt;
    std::shared_ptr<Normal> root;
    static Built<NormalProgram> build(std::span<const Pod> args, const ReadyState& ready) {
        if (args.size() != 1) return unexpected(Fault::answer("@Normal takes one Term"));
        PRISM_TRY_LET(term, from_pod<Term>(args[0]));
        auto p = std::make_unique<NormalProgram>();
        Cx cx = p->rt.build(ready);
        p->root = Normal::make(Path::root(), term, cx);
        PRISM_TRY_LET(out, p->rt.close(std::move(cx)));
        return std::pair{std::move(p), std::move(out)};
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
};

TestWorld normals() { return TestWorld().program<NormalProgram>("Normal"); }

}  // namespace

TEST(two_reductions_take_three_cycles) {
    verify(normals(), "sem-ch07-normalize");
    Report report = normals().run(slurp("sem-ch07-normalize.prismtest"));
    CHECK(report.transcript.find("compute until idle: 3 ticks, 1 to 3\n") != std::string::npos);
}

TEST(compute_lines_watch_the_cycles_one_tick_each) {
    Report report = normals().run(
        "test cycles, @Normal (App (Lam (Var 0)), (Lam (Var 0))):\n    build\n    compute\n    compute\n        => expect "
        "Ui.Label(text=\"Lam (Var 0)\")\n    compute until idle\n");
    CHECK(report.failures.empty());
    CHECK_EQ(report.transcript,
             std::string("test cycles\n\ntick 0\n  added:\n    1: Ui.Label(text=\"working\")\n\ntick 1\n\ntick 2\n  changed:\n    1: "
                         "Ui.Label(text=\"Lam (Var 0)\")\n\ncompute until idle: 0 ticks\n"));
}

TEST(the_term_prints_as_the_book_spells_it) {
    Term t = Term::app(boxed(Term::lam(Term::var(Nat(0)))), boxed(Term::var(Nat(1))));
    CHECK_EQ(show(t), std::string("App (Lam (Var 0)), (Var 1)"));
    CHECK(from_pod<Term>(read::value(show(t)).value()).value() == t);
}

// ---- The Test World's own rules (runtime.rs) ----------------------------------------------

TEST(seed_seven_deals_the_four_of_spades_first) {
    auto deck = XorShift(7).shuffle();
    CHECK_EQ(deck[0].to_string(), std::string("Card(rank=Four, suit=Spades)"));
    std::set<std::string> all;
    for (const Pod& c : deck) all.insert(c.to_string());
    CHECK_EQ(all.size(), 52u);
}

TEST(scripts_parse_or_exit_two_and_checks_print_ok_or_the_value) {
    CHECK_EQ(int(TestWorld().run("  build\n").exit_code), 2);
    Report r = TestWorld().run("check sums:\n    3 => 3\n    \"a\" => \"b\"\n    True\n");
    CHECK_EQ(r.transcript, std::string("check sums\n1: ok\n2: \"a\"\n3: ok\n"));
    CHECK_EQ(int(r.exit_code), 1);
    TestWorld compiled = TestWorld().check("sums", [] {
        return std::vector<Result<Pod>>{Pod::int64(3), Pod::text("b"), Pod::boolean(true)};
    });
    CHECK_EQ(int(compiled.run("check sums:\n    1 + 2 => 3\n    \"a\" + \"b\" => \"b\"\n    True\n").exit_code), 0);
    CHECK_EQ(strip_comment_header("# a note\n#\ntest t\n"), std::string("test t\n"));
}
