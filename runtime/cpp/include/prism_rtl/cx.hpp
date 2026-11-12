// The tick's skeleton: the `Node` interface emitted nodes implement, the context `Cx`
// their methods take, and the `Runtime` that keeps what a tick leaves for the next.
// Emitted code decides everything a node does; the runtime keeps the lists and walks
// them in Graph's order (Graph ch. 4, "A tick, in order"):
//
// 1. last round's undone clean-ups; the started list and the armed `@Defer`s fire; the
//    answers a host's supplier computed last tick and the update events of structural
//    dynamics that moved last tick (`Cx::output_later`), then the World's answers by
//    path and its pushes by element;
// 2. the wakes by height, a bucket per height, each node once per height;
// 3. propagation ends, and every read from here on is after the tick;
// 4. the build list (a builder drops what left, then builds); 5. the swap list; 6. the
//    detached widgets leave; 7. the patch pass;
// 8. the outputs: the patch diffed against the document last sent, the requests in
//    document order, the result, and `pending`.
//
// Then the compact round, `Runtime::compact`: the clean-ups, then the zombie collector.

#pragma once

#include "prism_rtl/rc.hpp"
#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>

#include "prism_rtl/doc.hpp"
#include "prism_rtl/gc.hpp"
#include "prism_rtl/pod.hpp"
#include "prism_rtl/result.hpp"
#include "prism_rtl/world.hpp"

namespace prism_rtl {

class Cx;

/// A node: one widget instance's storage and the methods its block compiles to. Each
/// method defaults to nothing, so a node implements what its block needs. A node is
/// always made by `make_rc` or `rc_placed` (rc.hpp), so a method can put the node on a list with
/// `me()` (Rust's `self: Rc<Self>`).
class Node : public RcFromThis<Node> {
public:
    virtual ~Node() = default;
    /// A wake at the height the node asked for: its internal combinators decide, once.
    virtual Done wake(Cx&) { return {}; }
    /// `@started`, in the tick after the build.
    virtual Done started(Cx&) { return {}; }
    /// An `@Defer` this node armed last tick fires.
    virtual Done deferred(Cx&) { return {}; }
    /// An answer to the place at `at`, relative to this node: the path index is emitted
    /// dispatch, each node matching its own positions, generations and keys.
    virtual Done answer(std::span<const Segment> at, const Pod& v, Cx& cx);
    /// A push to a place this node registered with `Cx::listen`.
    virtual Done push(const Pod&, const Pod&, Cx&) { return {}; }
    /// The build list: drop what left, then build from the values after the tick.
    virtual Done build(Cx&) { return {}; }
    /// The swap list: a switch or flatten takes its new inner.
    virtual Done swap(Cx&) { return {}; }
    /// The detached widgets' leaving: write the zombie or the dead state.
    virtual Done leave(Cx&) { return {}; }
    /// The patch pass: `Changed` for each place whose live fields a mark reached.
    virtual Done patch(Cx&) { return {}; }
    /// The compact round, after a death or a zombie write: unsubscribe what the new state
    /// no longer reads, release what dead fields held, split off live paths.
    virtual void clean_up() {}
    /// The nodes this node's fields hold strongly: what the zombie collector follows.
    using Visit = std::function<void(const Rc<Node>&)>;
    virtual void edges(const Visit&) const {}
    /// The collector found this node garbage: let go of every value held.
    virtual void release() {}

    /// This node, shared: what a method puts on a list.
    Rc<Node> me() { return rc_from_this(); }
    template <class T>
    Rc<T> me_as() {
        return rc_static_cast<T>(rc_from_this());
    }
};

using NodeRef = Rc<Node>;

/// A firing waiting for the start of the next tick: a closure over the node method that
/// fires (a computed answer into a child's place, a structural dynamic's update event).
using Later = std::function<Done(Cx&)>;

namespace detail {

/// A list of nodes, each at most once until the walk passes it: a node added again after
/// its turn is walked again (a builder marked by a row built in the same walk).
struct List {
    std::vector<NodeRef> nodes;
    std::unordered_map<const Node*, std::size_t> seen;
    std::size_t walked = 0;
    void add(NodeRef n);
};

/// What a tick gathers as it runs.
struct Gathered {
    std::vector<List> heights;
    List builds, swaps, leaves, patches, started, armed;
    std::vector<Edit> edits;
    std::vector<Request> requests;
    std::vector<Outcome> results;
    std::vector<std::pair<Pod, Weak<Node>>> listens;
    std::vector<Weak<Node>> zombies;
    std::vector<NodeRef> cleanups;
    /// What this tick leaves for the start of the next, which is owed: the answers a
    /// host's supplier computed, each the typed delivery into the child's place (hosting,
    /// host.hpp), and the update events of structural dynamics that moved (their value
    /// after this tick, `Snapshot`).
    std::vector<Later> later;
    /// A fault raised where no `Done` goes back (a row leaving inside a builder's walk):
    /// the tick fails at its close.
    std::optional<Fault> fault;
};

}  // namespace detail

/// One tick's context: the tick and its read point, the lists, the outputs so far.
class Cx {
public:
    Cx(const Cx&) = delete;
    Cx& operator=(const Cx&) = delete;
    Cx(Cx&&) = default;
    Cx& operator=(Cx&&) = default;

    /// The tick being run: what writes are stamped with.
    Tick tick() const { return tick_; }
    /// The read point: the tick during propagation (a read before it), the tick plus one
    /// from the end of propagation on (a read after it).
    Tick read_point() const { return read_at_; }
    /// Whether the tick is still propagating: its firings and wakes, before the
    /// construction step. A structural dynamic (`Snapshot`) has no value after the tick
    /// yet, so a read of one after the tick is a bug here and a value from the
    /// construction step on.
    bool propagating() const { return phase_ == Phase::Propagating; }
    /// A read after the tick, made during propagation: `f` runs with the read point past
    /// the tick, so every hold it reads answers its value after this tick, and the read
    /// point is then put back. `!Updated $d` fires `$d`'s value after the tick in the tick
    /// of the change (Semantics R3); this is how its node computes it at its height.
    template <class F>
    decltype(auto) after(F&& f) const {
        ReadAt moved(*this, tick_ + 1);
        return std::forward<F>(f)(*this);
    }
    /// A read as the tick opened, made after propagation: `f` runs with the read point at
    /// the tick, so every hold it reads answers its value before this tick's writes, and
    /// the read point is then put back. A `for` row whose key left reads its entry's last
    /// value this way (Semantics R8: the row ends at the end of the tick its key leaves;
    /// the entry after that tick does not exist).
    template <class F>
    decltype(auto) before(F&& f) const {
        ReadAt moved(*this, tick_);
        return std::forward<F>(f)(*this);
    }

    /// The height whose wakes run now; none outside the heights' run.
    std::optional<std::uint32_t> running_height() const { return height_; }
    /// Ask for a wake at `height`, once per height. A wake asked for at or below the
    /// running height runs at the next one (the clamp, Graph ch. 4).
    void wake(std::uint32_t height, NodeRef node);
    /// A builder whose dynamic changed: it builds after propagation.
    void build_later(NodeRef node) { g_.builds.add(std::move(node)); }
    /// A switch or flatten whose outer was marked.
    void swap_later(NodeRef node) { g_.swaps.add(std::move(node)); }
    /// A widget detached this tick: it runs to the end of the tick, then leaves.
    void leave_later(NodeRef node) { g_.leaves.add(std::move(node)); }
    /// A node whose places' live fields a mark reached: the patch pass asks it.
    void dirty(NodeRef node) { g_.patches.add(std::move(node)); }
    /// A newborn that binds `@started`: it fires next tick, and that tick is owed.
    void on_started(NodeRef node) { g_.started.add(std::move(node)); }
    /// A node with an armed `@Defer` (`Defer::arm` calls this).
    void arm(NodeRef node) { g_.armed.add(std::move(node)); }
    /// A node that took its dead or zombie state: its clean-up runs in the compact
    /// round, and a zombie is watched by the collector until it dies.
    void clean_up_later(NodeRef node, bool zombie);

    /// A place built, with its level: the document takes it.
    void added(Path path, Pod level) { g_.edits.push_back(Edit::added(std::move(path), std::move(level))); }
    /// A place torn down.
    void removed(Path path) { g_.edits.push_back(Edit::removed(std::move(path))); }
    /// A place whose live fields may have moved: the diff decides.
    void changed(Path path, Pod level) { g_.edits.push_back(Edit::changed(std::move(path), std::move(level))); }
    /// A request field fired, with its `Edge` value; `field` names it when the tag has
    /// several.
    void request(Path path, std::optional<std::string> field, Pod value) {
        g_.requests.push_back(Request{std::move(path), std::move(field), std::move(value)});
    }
    /// A firing for the start of the next tick, which is owed (like an `@Defer`): an
    /// answer a host's supplier computed (an event output's firing, a dynamic output's
    /// update), delivered to the child's place as an input of that tick (Semantics ch. 9
    /// as ruled: a computed answer is a tick late); the update event of a structural
    /// dynamic that moved this tick, fired at n+1 with the value after n (ch. 7, round 3
    /// ST).
    void output_later(Later deliver) { g_.later.push_back(std::move(deliver)); }
    /// A fault a cell raises where it holds only a `const Cx&`: the tick fails at its
    /// close.
    void fail_later(Fault f) const {
        if (!late_) late_ = std::move(f);
    }
    /// A fault where no `Done` goes back: the tick fails at its close.
    void keep(const Done& done) {
        if (!done && !g_.fault) g_.fault = done.error();
    }
    /// The root's result: a plain one at the build, a dynamic one when it changes, an
    /// event one when it fires.
    void result(Outcome outcome) { g_.results.push_back(std::move(outcome)); }

    /// A new place's first value: the ready state as the tick opened, with the tick's
    /// pushes on top (Semantics ch. 9, "Queries are sync, from a ready state").
    std::optional<Pod> first_value(const Pod& element) const;
    /// Register a place for the pushes keyed by its element's value.
    void listen(Pod element, const NodeRef& node) { g_.listens.emplace_back(std::move(element), node); }

private:
    friend class Runtime;
    /// Where a tick stands: propagating (its firings and wakes), or constructing (the
    /// walks after propagation and the patch pass). The build, tick 0, constructs only.
    enum class Phase { Propagating, Constructing };

    /// The read point moved for a scope, and put back at its end.
    struct ReadAt {
        const Cx& cx;
        Tick saved;
        ReadAt(const Cx& cx, Tick at) : cx(cx), saved(cx.read_at_) { cx.read_at_ = at; }
        ~ReadAt() { cx.read_at_ = saved; }
    };

    Cx(Tick tick, Tick read_at, Phase phase, const ReadyState& ready, std::vector<std::pair<Pod, Pod>> pushes)
        : tick_(tick), read_at_(read_at), phase_(phase), ready_(&ready), pushes_(std::move(pushes)) {}

    Tick tick_;
    /// The read point: mutable, so `Cx::after` can move it past the tick through a const
    /// reference and move it back.
    mutable Tick read_at_;
    Phase phase_;
    std::optional<std::uint32_t> height_;
    const ReadyState* ready_;
    std::vector<std::pair<Pod, Pod>> pushes_;
    detail::Gathered g_;
    /// A fault a cell raises through a const reference (a hold written twice).
    mutable std::optional<Fault> late_;
};

/// What a program keeps between ticks besides its root: the tick counter, the started
/// list, the armed `@Defer`s, the computed answers due, the push index, the clean-ups,
/// the document last sent, and the collector's zombies.
class Runtime {
public:
    /// Tick 0: a build reads values only, all of them after the tick.
    Cx build(const ReadyState& ready);
    /// One tick as a World calls it: open, fire, deliver, close.
    Result<TickOut> tick(const NodeRef& root, const TickIn& input, const ReadyState& ready);
    /// Run the heights, the walks after propagation and the patch pass; gather outputs.
    Result<TickOut> close(Cx&& cx);
    /// The compact round, between ticks: the clean-ups, then the zombie collector.
    /// Returns how many nodes the collector found garbage.
    std::size_t compact();
    /// The document as last sent.
    const Document& document() const { return document_; }
    const Collector& collector() const { return collector_; }

private:
    Tick tick_ = 0;
    std::vector<NodeRef> started_;
    std::vector<NodeRef> armed_;
    std::vector<Later> later_;
    std::map<Pod, std::vector<Weak<Node>>> listeners_;
    std::vector<NodeRef> cleanups_;
    Document document_;
    Collector collector_;
};

}  // namespace prism_rtl
