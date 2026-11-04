// The runtime's own rules, each through the API emitted code calls (the cells, the
// tick, the diff, ports, pushes, the collector, drive): runtime/rust/tests/runtime.rs.

#include "check.hpp"
#include "prism_rtl/prism_rtl.hpp"

using namespace prism_rtl;

namespace {

ReadyMap ready() { return ReadyMap(); }

// ---- A probe node: what its methods saw, tick by tick ------------------------------------

struct Probe : Node {
    std::optional<Hold<Int>> hold;
    Cache<Int> cache;
    unsigned computed = 0;
    std::vector<std::string> seen;
    Defer<Text> defer;

    Int h(const Cx& cx) const { return hold->get(cx); }
    Int cached(const Cx& cx) {
        return cache.get(cx, [&] {
            ++computed;
            return h(cx);
        });
    }

    Done answer(std::span<const Segment>, const Pod& v, Cx& cx) override {
        auto n = TRY(from_pod<Int>(v));
        hold->set(n, cx);
        cache.mark(cx);
        cx.wake(3, me());
        cx.wake(3, me());  // once per height
        cx.dirty(me());
        return {};
    }
    Done wake(Cx& cx) override {
        Int c = cached(cx);
        seen.push_back("wake " + std::to_string(cx.tick()) + ": hold " + h(cx).to_string() + " cache " + c.to_string());
        if (seen.size() < 4) cx.wake(1, me());  // below the running height: clamped to the next one
        // A read after the tick during propagation (`!Updated`'s value): the read point
        // moves past the tick for the closure and comes back.
        auto [hold_after, cache_after] = cx.after([&](const Cx& later) {
            Int c2 = cached(later);
            return std::pair{h(later), c2};
        });
        seen.push_back("after " + std::to_string(cx.tick()) + ": hold " + hold_after.to_string() + " cache " +
                       cache_after.to_string());
        return {};
    }
    Done deferred(Cx& cx) override {
        auto f = defer.fired(cx);
        seen.push_back("deferred " + std::to_string(cx.tick()) + ": " + (f ? "Some(\"" + f->to_string() + "\")" : "None"));
        return {};
    }
    Done started(Cx& cx) override {
        seen.push_back("started " + std::to_string(cx.tick()));
        defer.arm(Text("again"), me(), cx);
        return {};
    }
    Done patch(Cx& cx) override {
        Int c = cached(cx);
        seen.push_back("patch " + std::to_string(cx.tick()) + ": hold " + h(cx).to_string() + " cache " + c.to_string());
        cx.changed(Path::root().at(0), Pod::element("Test.Probe", {{"n", to_pod(h(cx))}}));
        return {};
    }
};

}  // namespace

TEST(a_hold_reads_old_values_during_its_tick_and_new_ones_after) {
    Runtime rt;
    ReadyMap r = ready();
    Cx cx = rt.build(r);
    auto p = std::make_shared<Probe>();
    p->hold.emplace(Int(7), cx);
    cx.added(Path::root().at(0), Pod::element("Test.Probe", {{"n", Pod::int64(7)}}));
    cx.on_started(p);
    TickOut out = rt.close(std::move(cx)).value();
    CHECK(out.pending);  // a newborn that binds @started owes a tick
    NodeRef root = p;
    TickOut t1 = rt.tick(root, TickIn::none(), r).value();
    CHECK(t1.pending);  // an armed @Defer sets pending
    TickOut t2 = rt.tick(root, TickIn::none().answer(Path::root().at(0), Pod::int64(8)), r).value();
    CHECK(!t2.pending);
    CHECK(t2.patch.changed ==
          (std::vector<std::pair<Path, Pod>>{{Path::root().at(0), Pod::element("Test.Probe", {{"n", Pod::int64(8)}})}}));
    std::vector<std::string> want = {
        "started 1",
        "deferred 2: Some(\"again\")",
        "wake 2: hold 7 cache 7",
        "after 2: hold 8 cache 8",
        "wake 2: hold 7 cache 7",
        "after 2: hold 8 cache 8",
        "patch 2: hold 8 cache 8",
    };
    CHECK(p->seen == want);
    // The reader in the tick of the write reads the clean cache; the read after the tick
    // recomputes once and is cached; the reader back at the tick's point is not served
    // it, and computes without caching.
    CHECK_EQ(p->computed, 3u);
}

TEST(the_patch_diff_sends_only_what_moved) {
    Document doc;
    Path a = Path::root().at(0), b = Path::root().at(1);
    auto label = [](const char* t) { return Pod::element("Ui.Label", {{"text", Pod::text(t)}}); };
    (void)doc.diff({Edit::added(b, label("b")), Edit::added(a, label("a"))});
    Patch p = doc.diff({Edit::changed(a, label("a")), Edit::changed(b, label("B"))});
    CHECK(p.added.empty() && p.removed.empty());
    CHECK(p.changed == (std::vector<std::pair<Path, Pod>>{{b, label("B")}}));
    p = doc.diff({Edit::removed(a), Edit::added(a, label("new"))});
    CHECK(p.removed == (std::vector<std::pair<Path, Pod>>{{a, label("a")}}));  // torn down with its element as it stood
    CHECK(p.added == (std::vector<std::pair<Path, Pod>>{{a, label("new")}}));  // two widgets at one path, never Changed
    Document world_side;
    Patch both;
    both.added = {{a, label("new")}, {b, label("B")}};
    world_side.apply(both);
    CHECK(world_side == doc);
}

TEST(ports_subscribe_and_unsubscribe_in_constant_time) {
    auto port = std::make_shared<Port<Int>>();
    CHECK(port->demand() == Demand::Idle);
    port->lien();
    CHECK(port->demand() == Demand::Liened);
    auto heard = std::make_shared<std::vector<std::string>>();
    Handle a = port->subscribe([heard](const Int& v, Cx&) -> Done {
        heard->push_back("a " + v.to_string());
        return {};
    });
    // A subscriber that unsubscribes itself while the port fires.
    auto me = std::make_shared<std::optional<Handle>>();
    std::weak_ptr<Port<Int>> weak = port;
    Handle b = port->subscribe([heard, weak, me](const Int& v, Cx&) -> Done {
        heard->push_back("b " + v.to_string());
        if (auto p = weak.lock(); p && *me) p->unsubscribe(**me);
        return {};
    });
    *me = b;
    CHECK(port->demand() == Demand::Subscribed);
    ReadyMap r = ready();
    Runtime rt;
    Cx cx = rt.build(r);
    CHECK(port->fire(Int(1), cx).has_value());
    CHECK(port->fire(Int(2), cx).has_value());
    CHECK(*heard == (std::vector<std::string>{"a 1", "b 1", "a 2"}));
    CHECK(port->kept(cx) == std::optional<Int>(Int(2)));  // a subscriber arriving later this tick catches up
    port->unsubscribe(a);
    port->release();
    CHECK(port->demand() == Demand::Idle);
    Handle c = port->subscribe([](const Int&, Cx&) -> Done { return {}; });
    CHECK(c == a);  // a freed handle is reused
    CHECK(rt.close(std::move(cx)).has_value());
}

TEST(a_constant_and_never) {
    ReadyMap r = ready();
    Runtime rt;
    Cx cx = rt.build(r);
    CHECK(Dynamic<Text>::constant(Text("x")).sample(cx) == Text("x"));
    CHECK(Event<Int>::never().port->demand() == Demand::Idle);
    auto doubled = Dynamic<Int>::constant(Int(21)).map([](const Int& n) { return n * Int(2); });
    CHECK(doubled.sample(cx) == Int(42));
    // An event's transform and its filter fire through.
    auto src = std::make_shared<Port<std::optional<Int>>>();
    auto got = std::make_shared<std::vector<Int>>();
    Event<Int> plus = filter_some(Event<std::optional<Int>>::of(src)).map([](const Int& n) { return n + Int(1); });
    (void)plus.port->subscribe([got](const Int& n, Cx&) -> Done {
        got->push_back(n);
        return {};
    });
    CHECK(src->fire(std::optional<Int>(Int(1)), cx).has_value());
    CHECK(src->fire(std::nullopt, cx).has_value());
    CHECK(*got == std::vector<Int>{Int(2)});
}

TEST(a_hold_written_twice_in_a_tick_fails_it) {
    ReadyMap r = ready();
    Runtime rt;
    Cx cx = rt.build(r);
    Hold<Int> h(Int(0), cx);
    h.set(Int(1), cx);
    h.set(Int(2), cx);
    auto out = rt.close(std::move(cx));
    CHECK(!out.has_value() && out.error().kind == "Runtime.TwoWrites");
}

TEST(a_snapshot_answers_the_structure_and_the_dynamic_one_tick_behind) {
    ReadyMap r = ready();
    Runtime rt;
    Cx build = rt.build(r);
    Snapshot<Int> s("$rows");
    bool threw = false;
    try {
        (void)s.get(build);
    } catch (const Panic&) {
        threw = true;
    }
    CHECK(threw);
    CHECK(!s.take_compared(Int(1), build));
    CHECK_EQ(s.get(build), Int(1));
    (void)rt.close(std::move(build)).value();
    struct Writer : Node {
        Snapshot<Int>* s;
        std::vector<std::string> log;
        Done answer(std::span<const Segment>, const Pod&, Cx& cx) override {
            s->mark(cx);
            log.push_back("during " + s->current(cx).to_string());
            bool threw = false;
            try {
                cx.after([&](const Cx& later) { return s->current(later); });
            } catch (const Panic&) {
                threw = true;
            }
            log.push_back(threw ? "after panics" : "after reads");
            cx.build_later(me());
            return {};
        }
        Done build(Cx& cx) override {
            log.push_back(std::string("moved ") + (s->take_compared(Int(2), cx) ? "yes" : "no"));
            log.push_back("after " + cx.after([&](const Cx& c) { return s->current(c); }).to_string());
            log.push_back("construction " + s->current(cx).to_string());
            log.push_back("dynamic " + cx.after([&](const Cx& c) { return s->get(c); }).to_string());
            return {};
        }
    };
    auto w = std::make_shared<Writer>();
    w->s = &s;
    NodeRef root = w;
    (void)rt.tick(root, TickIn::none().answer(Path::root().at(0), Pod::unit()), r).value();
    CHECK(w->log == (std::vector<std::string>{"during 1", "after panics", "moved yes", "after 2", "construction 2", "dynamic 1"}));
}

// ---- Pushes, first values, requests -----------------------------------------------------

namespace {

struct Place : Node {
    Pod element;
    std::vector<Pod> heard;
    Done push(const Pod& e, const Pod& v, Cx& cx) override {
        CHECK(e == element);
        heard.push_back(v);
        cx.request(Path::root().at(1), "seen", v);
        cx.request(Path::root().at(0), std::nullopt, Pod::unit());
        return {};
    }
};

}  // namespace

TEST(pushes_reach_every_place_with_the_element_and_newborns_see_them) {
    Pod clock = Pod::element("Time.Clock", {});
    ReadyMap entries;
    entries.entries.emplace(clock, Pod::int64(1));
    Runtime rt;
    Cx cx = rt.build(entries);
    CHECK(cx.first_value(clock) == std::optional<Pod>(Pod::int64(1)));
    auto place = std::make_shared<Place>();
    place->element = clock;
    NodeRef node = place;
    cx.listen(clock, node);
    (void)rt.close(std::move(cx)).value();
    TickOut out = rt.tick(node, TickIn::none().push(clock, Pod::int64(2)), entries).value();
    CHECK(place->heard == std::vector<Pod>{Pod::int64(2)});
    std::vector<std::string> lines;
    for (const auto& r : out.requests) lines.push_back(r.to_string());
    // Requests in document order, named when a tag has several.
    CHECK(lines == (std::vector<std::string>{"0: unit", "1: seen=2"}));
}

// ---- The zombie collector ---------------------------------------------------------------

namespace {

/// A zombie row whose constant keeps another node (a value with its `$` inside).
struct Zombie : Node {
    NodeRef constant;
    void edges(const Visit& visit) const override {
        if (constant) visit(constant);
    }
    void release() override { constant.reset(); }
};

}  // namespace

TEST(two_zombies_that_lien_each_other_are_freed_and_a_held_one_is_kept) {
    ReadyMap r = ready();
    Runtime rt;
    Cx cx = rt.build(r);
    auto a = std::make_shared<Zombie>(), b = std::make_shared<Zombie>(), c = std::make_shared<Zombie>();
    a->constant = b;
    b->constant = a;
    c->constant = a;  // c keeps a, and the test keeps c: all three live
    for (const auto& z : {a, b, c}) cx.clean_up_later(z, true);
    (void)rt.close(std::move(cx)).value();
    std::weak_ptr<Zombie> wa = a, wb = b, wc = c;
    a.reset();
    b.reset();
    CHECK_EQ(rt.compact(), 0u);  // c is held from outside, and it reaches the loop
    c.reset();                   // its count frees it; the loop it kept is the collector's
    CHECK_EQ(rt.compact(), 2u);
    CHECK(wa.expired() && wb.expired() && wc.expired());  // no leaks, ever (D5)
    CHECK_EQ(rt.collector().watching(), 0u);
}

// ---- Worlds ----------------------------------------------------------------------------

namespace {

/// `Process.Exit(code=(3 on !started))` at 0.
struct Exits : Node {
    Done started(Cx& cx) override {
        cx.request(Path::root().at(0), std::nullopt, to_pod(std::uint8_t(3)));
        return {};
    }
};

struct ExitsProgram : Program {
    Runtime rt;
    NodeRef root;
    static Built<ExitsProgram> build(std::span<const Pod>, const ReadyState& ready) {
        auto p = std::make_unique<ExitsProgram>();
        Cx cx = p->rt.build(ready);
        cx.added(Path::root().at(0), Pod::element("Process.Exit", {}));
        p->root = std::make_shared<Exits>();
        cx.on_started(p->root);
        auto out = TRY(p->rt.close(std::move(cx)));
        return std::pair{std::move(p), std::move(out)};
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
};

/// A World that answers nothing and exits when a `Process.Exit` place requests.
struct Quiet : World {
    Document doc;
    std::optional<std::uint8_t> exit;
    std::size_t ticks = 0;
    std::shared_ptr<const ReadyState> ready() const override { return std::make_shared<ReadyMap>(); }
    void perform(const TickOut& out) override {
        ++ticks;
        doc.apply(out.patch);
        for (const auto& r : out.requests) {
            const Pod* p = doc.get(r.path);
            if (p && p->tag() && *p->tag() == "Process.Exit")
                if (auto code = from_pod<std::uint8_t>(r.value)) exit = *code;
        }
    }
    std::optional<TickIn> next_inputs() override { return std::nullopt; }
    std::optional<std::uint8_t> exit_code() const override { return exit; }
};

}  // namespace

TEST(drive_ticks_while_pending_and_exits_with_the_code_requested) {
    Quiet world;
    CHECK_EQ(int(drive<ExitsProgram>(world, {}, 10)), 3);
    CHECK_EQ(world.ticks, 2u);  // the build, then the owed tick
}
