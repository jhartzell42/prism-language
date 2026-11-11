// Hosting (Semantics ch. 9, "`@HostDocument` and its forms"; claude/backend.md, "Hosting"):
// what is the same in every program when a host document is lowered as a `for` over its
// child's document.
//
// A host is Prism inside the same graph and the same tick. The compiler inlines the hosted
// child into the node of the host's block (its owner), in a scope of its own. The child's
// document is `$(Path ==> $Elem c)` (Jimmy, 6 November 2026): each child place is an
// `Entry`, a structural dynamic of its own kept in the node the place is in, and the outer
// is a `HostDoc` the owner keeps as a snapshot, taken when a place came or went (the
// program reads its dynamics; the host's own rows its entries, `host_k_doc_direct`). The
// owner emits a dispatch from a child place's path and a typed answer into the child's
// input method (`host_k_answer`), and holds a `Host` made of it, and a `for` over the
// places whose rows are `HostRow`s: one per child place, built when the key appears in the
// child's places after a tick and torn down at the end of the tick its key leaves (R7, R8,
// the `for` machinery of the emitted code). Nothing here is plain data: the child's
// elements are `E`, the program's `Elem` type, and an answer is `A`, the program's
// `Answer` type (one alternative per tag, carrying that tag's output type).
//
// A row calls the supplier with the place's path and its live element (`Live`: the
// identity, the level over time read through the place's entry, and the entry) and builds the
// blueprint it answers with at `host/[p]`, handing it a `Sink`, the typed way back into
// the child's place. The blueprint forms decide *when* an answer reaches the child:
//
// - **Forwarding** (`element_blueprint`, `@Element e`) is re-emission: a new element of
//   the host's own at `host/[p]/0` whose level follows `e`'s, and whose answers and
//   pushes from the World go to the child's place in the same tick, as the tick's
//   inputs. A host that forwards everything is observably not there. A host in a hosted
//   scope re-emits into the enclosing host's document instead (`Host`'s `enclosing`).
// - **A computed answer** is delayed: an event output's firing (`event_blueprint`) or a
//   dynamic output's later update (`dynamic_blueprint`) is queued with
//   `Cx::answer_later` and delivered as an input of the next tick, which is owed
//   (`pending`, like an armed `@Defer`). A dynamic output's first value is read in the
//   birth tick, synchronously (as a World's first-value query is), and a plain output
//   (`plain_blueprint`) is the first value once. A computed answer in flight to a place
//   whose key left is dropped (`HostRow`'s `left`). Nested hosts add one tick per
//   computing layer and none per forwarding layer.
//
// **First values.** A child place with a first value asks its host at its build
// (`Host::first_value`), before the owner's node exists: the supplier row for that place
// is built early and kept for the builder, which finds it present. This is the one early
// construction, and it is why a `Host` reaches its owner through a link filled in once
// the node is made (`Host::link`).
//
// **Ownership.** The owner holds its `Host` strongly and the host holds its rows; a row
// holds its host weakly (the host outlives its rows), so no count cycle runs through a
// host (doubt HW in README.md).

#pragma once

#include "prism_rtl/rc.hpp"
#include <concepts>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "prism_rtl/cells.hpp"
#include "prism_rtl/collections.hpp"
#include "prism_rtl/cx.hpp"
#include "prism_rtl/pod.hpp"
#include "prism_rtl/print.hpp"

namespace prism_rtl {

// ---------------------------------------------------------------------------------------
// The document as `$(Path ==> $Elem c)`: one entry per place, the outer keyed by places
// ---------------------------------------------------------------------------------------

/// A dynamic is not plain data: a World reads a level that holds dynamics deep, each sampled
/// at the read point (`level_pod`). Reaching this is a bug of the compiler, never a value.
template <class T>
struct ToPod<Dynamic<T>> {
    [[noreturn]] static Pod to_pod(const Dynamic<T>&) {
        panic("a dynamic made plain data without a read point: a level that holds dynamics is read with `level_pod`");
    }
};

/// No World hands a program a dynamic.
template <class T>
struct FromPod<Dynamic<T>> {
    static Result<Dynamic<T>> from_pod(const Pod& p) { return mismatch<Dynamic<T>>("a dynamic, which is never plain data", p); }
};

/// A dynamic does not print (no `^Show`); reaching this is a bug of the compiler.
template <class T>
struct Show<Dynamic<T>> {
    [[noreturn]] static void show(const Dynamic<T>&, Printer&) { panic("a dynamic shown: a dynamic has no `^Show`"); }
};

/// A level as a World reads it (claude/incremental-host-document.md, doubt WD): plain data,
/// with every dynamic it holds sampled at the read point, deep. The program's `Elem` has a
/// member `level_pod`; anything else (a test's `Pod`) is its plain data.
template <class E>
Pod level_pod(const E& e, const Cx& cx) {
    if constexpr (requires { e.level_pod(cx); }) return e.level_pod(cx);
    else return to_pod(e);
}

/// The update events of the dynamics a level holds, and of theirs.
template <class E>
void level_ports(const E& e, const Cx& cx, std::vector<Rc<Port<Unit>>>& out) {
    if constexpr (requires { e.level_ports(cx, out); }) e.level_ports(cx, out);
}

/// A map of dynamics as a World reads it: each entry sampled at the read point, deep.
template <class K, class E>
Pod dynamics_pod(const Map<K, Dynamic<E>>& m, const Cx& cx) {
    std::vector<std::pair<Pod, Pod>> out;
    for (const auto& [k, d] : m) out.emplace_back(to_pod(k), level_pod(d.sample(cx), cx));
    return Pod::map(std::move(out));
}

/// The update events of a map of dynamics and of every dynamic their values hold.
template <class K, class E>
void dynamics_ports(const Map<K, Dynamic<E>>& m, const Cx& cx, std::vector<Rc<Port<Unit>>>& out) {
    for (const auto& [k, d] : m) {
        out.push_back(d.updates);
        level_ports(d.sample(cx), cx, out);
    }
}

/// A World place whose level holds dynamics: subscribed to every one of them as it was last
/// sent, so a move of any nested entry patches the place again in the tick it moves (doubt
/// WD). Renewed each time the place is sent.
class Watch {
public:
    Watch() = default;
    Watch(const Watch&) = delete;
    Watch& operator=(const Watch&) = delete;
    Watch(Watch&&) = default;
    Watch& operator=(Watch&&) = default;
    ~Watch() { drop_all(); }
    /// The level sent as `node`'s place: its nested dynamics watched from now on.
    template <class E>
    void renew(const E& level, const NodeRef& node, const Cx& cx) {
        drop_all();
        std::vector<Rc<Port<Unit>>> ports;
        level_ports(level, cx, ports);
        Weak<Node> weak = node;
        for (auto& p : ports) {
            Handle h = p->subscribe([weak](const Unit&, Cx& cx) -> Done {
                if (auto n = weak.lock()) cx.dirty(n);
                return {};
            });
            subs_.emplace_back(p, h);
        }
    }
    /// The place left: nothing is watched.
    void drop_all() {
        for (auto& [p, h] : subs_) p->unsubscribe(h);
        subs_.clear();
    }

private:
    std::vector<std::pair<Rc<Port<Unit>>, Handle>> subs_;
};

/// `@Flatten` over a dynamic the node holds as a value (a `for` row's entry `$$a`, a host
/// document's entry): the subscription to its current inner's update event, which tells the
/// node (`moved`, its emitted method) that the inner moved, so what reads the flatten reads
/// it again. Following another inner lets go of the last; dropped with its node.
class Follow {
public:
    Follow() = default;
    Follow(const Follow&) = delete;
    Follow& operator=(const Follow&) = delete;
    Follow(Follow&&) = default;
    Follow& operator=(Follow&&) = default;
    ~Follow() { drop_sub(); }
    /// From now on the inner `updates` tells `node` through `moved`.
    template <class N, class R>
    void follow(const Rc<Port<Unit>>& updates, const Rc<N>& node, R (N::*moved)(Cx&)) {
        drop_sub();
        Weak<N> weak = node;
        Handle h = updates->subscribe([weak, moved](const Unit&, Cx& cx) -> Done {
            if (auto n = weak.lock()) ((*n).*moved)(cx);
            return {};
        });
        sub_.emplace(updates, h);
    }

private:
    void drop_sub() {
        if (sub_) updates_unsubscribe();
    }
    void updates_unsubscribe() {
        sub_->first->unsubscribe(sub_->second);
        sub_.reset();
    }
    std::optional<std::pair<Rc<Port<Unit>>, Handle>> sub_;
};

/// What a row that holds no block (a row shared by every call of a widget that calls
/// itself) tells in a hosted scope when a place of its may have moved: the node that
/// built it, through that node's own telling (`moved_by`).
using Moved = std::function<void(Cx&)>;

/// `node`'s method `moved` as a `Moved`: the node held weakly, told while it lives.
template <class N, class R>
Moved moved_by(const Rc<N>& node, R (N::*moved)(Cx&)) {
    Weak<N> weak = node;
    return [weak, moved](Cx& cx) {
        if (auto n = weak.lock()) ((*n).*moved)(cx);
    };
}

/// `!MergeAll` of a list that is not a literal (Semantics Appendix B): the list is a plain
/// value, so its events are fixed at the build. Each event's port is heard into a stamped
/// input, and the node is told (`heard`, which asks its wake); the wake takes the values
/// fired this tick, in the list's order, as one firing.
template <class T>
class MergeList {
public:
    MergeList() = default;
    MergeList(const MergeList&) = delete;
    MergeList& operator=(const MergeList&) = delete;
    MergeList(MergeList&&) = default;
    MergeList& operator=(MergeList&&) = default;
    ~MergeList() {
        for (auto& [p, h] : subs_) p->unsubscribe(h);
    }
    /// Hears every event of `events`, telling `node` through `heard` when one fires.
    template <class N, class R>
    void listen(const Array<Event<T>>& events, const Rc<N>& node, R (N::*heard)(Cx&)) {
        auto inputs = std::make_shared<std::vector<Stamped<T>>>(events.size());
        inputs_ = inputs;
        for (std::size_t i = 0; i < events.size(); ++i) {
            const Event<T>& ev = events[i];
            Weak<N> weak = node;
            Handle h = ev.port->subscribe([inputs, i, weak, heard](const T& v, Cx& cx) -> Done {
                (*inputs)[i].set(v, cx);
                if (auto n = weak.lock()) ((*n).*heard)(cx);
                return {};
            });
            subs_.emplace_back(ev.port, h);
        }
    }
    /// The values the list's events fired this tick, in the list's order; none fired, none.
    std::optional<Array<T>> take(const Cx& cx) {
        Array<T> all;
        if (inputs_)
            for (auto& s : *inputs_)
                if (auto x = s.take(cx)) all.push_back(std::move(*x));
        if (all.size() == 0) return std::nullopt;
        return all;
    }

private:
    std::shared_ptr<std::vector<Stamped<T>>> inputs_;
    std::vector<std::pair<Rc<Port<T>>, Handle>> subs_;
};

/// A held `@Switch`'s subscription to its current inner event (prismc's `HeldSwitch`):
/// listening to another event lets go of the one before. The event is kept whole: its
/// owner (a `merge_all`'s node) lives while the switch listens.
template <class T>
class Listen {
public:
    Listen() = default;
    Listen(const Listen&) = delete;
    Listen& operator=(const Listen&) = delete;
    Listen(Listen&&) = default;
    Listen& operator=(Listen&&) = default;
    ~Listen() { drop_sub(); }
    /// From now on `ev` tells `node` through `heard`, and the event before tells nothing.
    template <class N, class R, class V>
    void listen(const Event<T>& ev, const Rc<N>& node, R (N::*heard)(V, Cx&)) {
        drop_sub();
        Weak<N> weak = node;
        Handle h = ev.port->subscribe([weak, heard](const T& v, Cx& cx) -> Done {
            if (auto n = weak.lock()) ((*n).*heard)(v, cx);
            return {};
        });
        sub_.emplace(ev, h);
    }

private:
    void drop_sub() {
        if (sub_) {
            sub_->first.port->unsubscribe(sub_->second);
            sub_.reset();
        }
    }
    std::optional<std::pair<Event<T>, Handle>> sub_;
};

/// `!MergeAll` of a list as an event value: a node of its own that hears the list's events
/// (`MergeList`) and, woken, fires the values of the tick in the list's order.
template <class T>
class MergeNode : public Node {
public:
    MergeList<T> list;
    Rc<Port<Array<T>>> out = make_rc<Port<Array<T>>>();
    Done wake(Cx& cx) override {
        if (auto all = list.take(cx)) return out->fire(std::move(*all), cx);
        return {};
    }
    void heard(Cx& cx) { cx.wake(0, self_.lock()); }
    Weak<MergeNode> self_;
};

/// The event `!MergeAll events` (Semantics Appendix B): its node woken at the height after
/// the one an input fired at (the runtime's clamp), owned by the event.
template <class T>
Event<Array<T>> merge_all(const Array<Event<T>>& events) {
    auto node = make_rc<MergeNode<T>>();
    node->self_ = node;
    node->list.listen(events, node, &MergeNode<T>::heard);
    return Event<Array<T>>{node, node->out};
}

/// One place of a hosted child, as its host's document holds it (Jimmy, 6 November 2026:
/// the document is `$(Path ==> $Elem c)`): a structural dynamic of its own (Semantics ch. 7,
/// round 3 ST, doubt EP). Its node takes a snapshot of the place's level at its build and
/// again whenever the place may have moved (its node's patch pass, as a World place is
/// sent); the dynamic the document holds for the place samples the snapshot of the tick
/// before (S(m-1) after `m`), and its update event fires at the start of the tick after a
/// take that changed the level, which that tick owes. Its comparison is the level's own
/// equality, a dynamic it holds equal only to itself (doubt EQ). The rows that read the
/// place as it stands (a host's row forwarding it, at any depth of hosts) are dirtied by a
/// take that moved it, so a forwarded element re-emits in the same tick.
template <class E>
class Entry : public RcFromThis<Entry<E>> {
public:
    /// An entry with no snapshot yet; `what` names the place in a fault.
    static Rc<Entry> make(std::string what) {
        auto entry = rc_placed<Entry>([&](void* place) { return new (place) Entry(std::move(what)); });
        Weak<Entry> weak = entry;
        auto read = make_rc<const std::function<E(const Cx&)>>([weak](const Cx& cx) {
            auto e = weak.lock();
            if (!e) panic("a place's entry is read only while its place lives");
            return e->snap_.get(cx);
        });
        entry->dynamic_ = Dynamic<E>{nullptr, read, entry->published_};
        return entry;
    }
    /// The place's level now: the first snapshot at its node's build, then each time the
    /// place may have moved, compared with the one that stood. A take that moved the level
    /// dirties the rows reading it as it stands and queues the publication, once per tick.
    void take(E level, Cx& cx) {
        if (!snap_.taken()) {
            snap_.take(std::move(level), cx);
            return;
        }
        if (!snap_.take_compared(std::move(level), cx)) return;
        std::vector<NodeRef> rows;
        std::erase_if(watchers_, [](const Weak<Node>& w) { return !w.lock(); });
        for (auto& w : watchers_)
            if (auto r = w.lock()) rows.push_back(r);
        for (auto& r : rows) cx.dirty(r);
        if (snap_.queue(cx)) {
            Rc<Entry> me = rc_static_cast<Entry>(this->rc_from_this());
            cx.answer_later([me](Cx& cx) { return me->publish(cx); });
        }
    }
    /// The place's level as it stands: the snapshot taken this tick from the construction
    /// step on, the one before during propagation.
    E current(const Cx& cx) const { return snap_.current(cx); }
    /// The dynamic the document holds for this place, `$Elem`: the same one for as long as
    /// the place lives.
    const Dynamic<E>& dynamic() const { return dynamic_; }
    /// A row that reads the place as it stands: dirtied when a take moves it.
    void watch(Weak<Node> row) { watchers_.push_back(std::move(row)); }

private:
    explicit Entry(std::string what) : snap_(std::move(what)), published_(make_rc<Port<Unit>>()) {}
    /// At the start of the tick after a take that moved it: the entry's update event fires.
    Done publish(Cx& cx) {
        if (snap_.published(cx)) return published_->fire(Unit{}, cx);
        return {};
    }
    Snapshot<E> snap_;
    Rc<Port<Unit>> published_;
    Dynamic<E> dynamic_;
    std::vector<Weak<Node>> watchers_;
};

/// A host's child document as one structural dynamic of the owner (pick ST): the outer of
/// `$(Path ==> $Elem c)`. `doc` is what the program reads, each place's dynamic by its path
/// relative to the child; `places` the entries themselves, which the host's own rows read.
/// It is taken when a place of the child came or went, and two are equal when they hold the
/// same entries at the same paths, so a value change of a place never moves it.
template <class E>
struct HostDoc {
    Map<Path, Dynamic<E>> doc;
    Map<Path, Rc<Entry<E>>> places;
    void insert(Path path, Rc<Entry<E>> entry) {
        doc.insert(path, entry->dynamic());
        places.insert(std::move(path), std::move(entry));
    }
    bool operator==(const HostDoc& o) const {
        if (places.size() != o.places.size()) return false;
        auto a = places.begin();
        auto b = o.places.begin();
        for (; a != places.end(); ++a, ++b) {
            const auto& [k1, e1] = *a;
            const auto& [k2, e2] = *b;
            if (!(k1 == k2) || !ptr_eq(e1, e2)) return false;
        }
        return true;
    }
};

/// `$each <- @PerKey $m` (Semantics Appendix B; round 9 PK, Jimmy's): a dynamic map taken
/// apart by key, `$(k ==> v)` into `$(k ==> $v)`, the shape a host's document has. The
/// outer is a hold the node's readers read, written only when a key came or went; each
/// key's inner is a hold of its own with its own update event, the same dynamic for as long
/// as the key stays (keys are identity, ch. 4), written and fired only when a write of `$m`
/// gives the key a value unequal (`==`) to the one it holds (`@Distinct`, per key). A new
/// key's inner starts at its value and fires nothing in its birth tick (R7); a key that
/// leaves drops out of the outer and its inner keeps its last value for the row that
/// still reads it in the leaving tick (R8), and a key that comes back gets a new inner. A
/// write of `$m` costs one `==` of the maps (at once when the map is the one last taken,
/// sharing being invisible, ch. 11) and otherwise one `==` per kept key. The Rust
/// runtime's `PerKey` is the same.
template <class K, class V>
class PerKey {
public:
    /// At the build: an inner per key of the source, each at its value, and the outer over
    /// them. Once; a read before the build has done it.
    void start(Map<K, V> source, const Cx& cx) const {
        if (outer_) return;
        std::vector<std::pair<K, Inner>> inners;
        std::vector<std::pair<K, Dynamic<V>>> outer;
        inners.reserve(source.size());
        outer.reserve(source.size());
        for (const auto& [k, v] : source) {
            Inner i = inner(v, cx);
            outer.emplace_back(k, i.dynamic);
            inners.emplace_back(k, std::move(i));
        }
        outer_.emplace(Map<K, Dynamic<V>>::from_sorted(std::move(outer)), cx);
        inners_ = Map<K, Inner>::from_sorted(std::move(inners));
        last_ = std::move(source);
    }
    /// The outer at the read point; before the build, set up from `source` first.
    template <class F>
    Map<K, Dynamic<V>> outer(const Cx& cx, F source) const {
        if (!outer_) start(source(), cx);
        return outer_->get(cx);
    }
    /// `$m` written with `source` (its update event's value): each kept key whose value
    /// changed has its inner written and fired, the new keys get inners, and the outer is
    /// written when a key came or went. Answers whether the outer moved.
    bool written(Map<K, V> source, Cx& cx) {
        if (!outer_) {
            start(std::move(source), cx);
            return false;
        }
        if (source == last_) return false;
        // one walk of both in key order: the kept keys compared, the new ones given inners
        Map<K, Inner> old = inners_;
        std::vector<Rc<Port<Unit>>> fired;
        std::vector<Inner> born;
        bool moved = false;
        auto o = old.begin();
        for (const auto& [k, v] : source) {
            // the keys that left before this one
            while (o != old.end() && (*o).first < k) {
                ++o;
                moved = true;
            }
            if (o != old.end() && (*o).first == k) {
                const Inner& i = (*o).second;
                if (!(i.cell->hold.newest() == v)) {
                    i.cell->hold.set(v, cx);
                    fired.push_back(i.cell->updates);
                }
                ++o;
            } else {
                born.push_back(inner(v, cx));
                moved = true;
            }
        }
        if (o != old.end()) moved = true;
        // a key came or went: the inners by the source's keys, and the outer over them
        if (moved) {
            std::vector<std::pair<K, Inner>> next;
            std::vector<std::pair<K, Dynamic<V>>> outer;
            next.reserve(source.size());
            outer.reserve(source.size());
            std::size_t b = 0;
            for (const auto& [k, v] : source) {
                const Inner* kept = old.get(k);
                Inner i = kept ? *kept : born.at(b++);
                outer.emplace_back(k, i.dynamic);
                next.emplace_back(k, std::move(i));
            }
            inners_ = Map<K, Inner>::from_sorted(std::move(next));
            outer_->set(Map<K, Dynamic<V>>::from_sorted(std::move(outer)), cx);
        }
        last_ = std::move(source);
        for (const auto& p : fired) cx.keep(p->fire(Unit{}, cx));
        return moved;
    }

private:
    /// One key's hold and update event.
    struct Cell {
        Hold<V> hold;
        Rc<Port<Unit>> updates;
    };
    /// One key's inner: its cell, and the dynamic over it the outer holds.
    struct Inner {
        Rc<Cell> cell;
        Dynamic<V> dynamic;
        bool operator==(const Inner& o) const { return ptr_eq(cell, o.cell); }
    };
    static Inner inner(const V& v, const Cx& cx) {
        Rc<Cell> cell = make_rc<Cell>(Cell{Hold<V>(v, cx), make_rc<Port<Unit>>()});
        auto read = make_rc<const std::function<V(const Cx&)>>([cell](const Cx& cx) { return cell->hold.get(cx); });
        return Inner{cell, Dynamic<V>{nullptr, read, cell->updates}};
    }
    // mutable as a cell's state is (Rust's `RefCell`): a read before the build sets it up
    mutable std::optional<Hold<Map<K, Dynamic<V>>>> outer_;
    mutable Map<K, Inner> inners_;
    mutable Map<K, V> last_;
};

/// The way back into a child's place: a typed answer, delivered now.
template <class A>
using Sink = std::function<Done(const A&, Cx&)>;

/// A row's own places in a hosted scope: each place's entry by its path relative to the
/// path the row was built at.
template <class E>
using RowDocument = std::function<std::vector<std::tuple<Path, Rc<Entry<E>>>>(const Cx&)>;

/// A host's typed answer to a place of a row's own, the position counted from the row's
/// first.
template <class A>
using RowDeliver = std::function<Done(std::span<const Segment>, const A&, Cx&)>;

/// What a blueprint built for a child place gives back: its node (whose places are under
/// the row's path), the place's first value when the output has one, and whether the
/// blueprint forwards the element (then the child place's requests go out with it, at
/// the row's `0`, and an enclosing host's answers to that place come back to the child).
template <class E, class A>
struct Supplied {
    NodeRef node;
    std::optional<A> first;
    bool forwards = false;
    /// A place of the blueprint's own at the row's `0` in a hosted scope (an element value
    /// placed, `described_blueprint`): its entry, which the enclosing host's document holds.
    Rc<Entry<E>> entry = nullptr;
    /// How many places of its own the blueprint built from the base it was given (a
    /// widget's blueprint, `widget_blueprint`; ruling TC: places are counted as built).
    std::uint32_t built = 0;
    /// How a widget's blueprint leaves: its row's places torn down, the row with them.
    /// The other forms leave by their node.
    std::function<void(Cx&)> leave = nullptr;
    /// A widget's blueprint built in a hosted scope: its row's own places, entries of the
    /// enclosing host's document, by their paths relative to the path it was built at.
    RowDocument<E> document = nullptr;
    /// A widget's blueprint built in a hosted scope: an enclosing host's typed answer to a
    /// place of its row's, the position counted from the row's first.
    RowDeliver<A> deliver = nullptr;
};

/// The program's answer type, which a World's answer to an element decodes into by the
/// element's tag (emitted beside `Elem`): `static Result<A> decode(const E&, const Pod&)`,
/// and `decode_first`, a World's first value for the element: an answer, but for a bundle's
/// tag, whose first value is its plain parts (`Answer`'s first-value case, pick HO).
template <class A, class E>
concept AnswerOf = std::copyable<A> && requires(const E& e, const Pod& p) {
    { A::decode(e, p) } -> std::same_as<Result<A>>;
    { A::decode_first(e, p) } -> std::same_as<Result<A>>;
};

template <class E, class A>
class HostOf;

/// Where a node's places go (claude/backend.md, "Hosting"): the document (`hosted` null),
/// or an enclosing host's document, which answers a first-value query itself.
template <class E, class A>
struct Scope {
    Rc<HostOf<E, A>> hosted;
    static Scope world() { return Scope{nullptr}; }
    static Scope in(Rc<HostOf<E, A>> host) { return Scope{std::move(host)}; }
    bool is_world() const { return !hosted; }
};

/// `@o` as a value (claude/backend.md, "Types"): built at a path with the sink that answers
/// the child's place, in the scope of the host whose row builds it, its own places
/// numbered from the base it is given (0 under a host's row; a block that applies the
/// value gives its next position as built).
template <class E, class A>
using Blueprint = std::function<Result<Supplied<E, A>>(Path, Sink<A>, const Scope<E, A>&, std::uint32_t, Cx&)>;

/// A host's supplier, in the `@HostDocument` form: a blueprint for every child place.
template <class E, class A>
using Supplier = std::function<Blueprint<E, A>(const Path&, const Element<E>&)>;

// ---------------------------------------------------------------------------------------
// The blueprint forms
// ---------------------------------------------------------------------------------------

namespace detail {

/// A node with no places and nothing to do.
struct Inert : Node {};

/// A subscription a blueprint's node holds, let go when the row leaves.
template <class T>
struct Subscribed : Node {
    Rc<Port<T>> port;
    std::optional<Handle> handle;
    explicit Subscribed(Rc<Port<T>> p) : port(std::move(p)) {}
    Done leave(Cx&) override {
        if (handle) port->unsubscribe(*std::exchange(handle, std::nullopt));
        return {};
    }
};

/// The two subscriptions of a bundle answer with a `$` part (`bundle_dynamic_blueprint`),
/// let go when the row leaves.
template <class T>
struct SubscribedBoth : Node {
    Rc<Port<T>> event;
    std::optional<Handle> event_handle;
    Rc<Port<Unit>> updates;
    std::optional<Handle> updates_handle;
    SubscribedBoth(Rc<Port<T>> e, Rc<Port<Unit>> u) : event(std::move(e)), updates(std::move(u)) {}
    Done leave(Cx&) override {
        if (event_handle) event->unsubscribe(*std::exchange(event_handle, std::nullopt));
        if (updates_handle) updates->unsubscribe(*std::exchange(updates_handle, std::nullopt));
        return {};
    }
};

/// A computed answer: delivered as an input of the next tick, which is owed.
template <class A>
Done later(const Sink<A>& sink, A v, Cx& cx) {
    cx.answer_later([sink, v](Cx& cx) { return sink(v, cx); });
    return {};
}

}  // namespace detail

/// A widget's row as its blueprint built it: the row's node, how it leaves, how many
/// places it built, the widget's plain result (unit when it gives none), and, for a row
/// built in a hosted scope, its places' entries and a host's typed answer to one.
template <class E, class A, class R>
struct BuiltWidget {
    NodeRef node;
    std::function<void(Cx&)> leave;
    std::uint32_t built = 0;
    R result;
    RowDocument<E> document = nullptr;
    RowDeliver<A> deliver = nullptr;
};

/// A host's telling as a `Moved`: the host held weakly, told while it lives.
template <class E, class A>
Moved host_moved(const Rc<HostOf<E, A>>& host);

/// A widget with places as a blueprint value (Semantics ch. 8: a blueprint builds nothing
/// until it is applied): `build` makes the widget's row at the path, its places numbered
/// from the base, in the World's scope; `build_hosted` makes it in a hosted scope, its
/// places entries of the host's document, given the host and what tells the host's owner
/// that a place of the row's came or went (`HostOf::moved`); `answer` makes the widget's
/// plain result the place's first value (nothing for a widget that gives none).
template <class E, class A, class Build, class BuildHosted, class AnswerFn>
Blueprint<E, A> widget_blueprint(Build build, BuildHosted build_hosted, AnswerFn answer) {
    return [build, build_hosted, answer](Path path, Sink<A>, const Scope<E, A>& scope, std::uint32_t base, Cx& cx) -> Result<Supplied<E, A>> {
        if (scope.is_world()) {
            auto made = TRY(build(path, base, cx));
            return Supplied<E, A>{made.node, answer(std::move(made.result)), false, nullptr, made.built, made.leave};
        }
        auto made = TRY(build_hosted(path, base, scope.hosted, host_moved<E, A>(scope.hosted), cx));
        return Supplied<E, A>{made.node, answer(std::move(made.result)), false, nullptr, made.built, made.leave, made.document, made.deliver};
    };
}

/// A widget that gives an event back as a blueprint value: as `widget_blueprint`, the
/// row's result the port its event fires. Each firing is an answer to the place the
/// blueprint answers, made of the event's value by `answer` and given to the sink of what
/// applied the blueprint, in the tick of the firing. The subscription lives as long as
/// the row, whose port it is.
template <class E, class A, class Build, class BuildHosted, class AnswerFn>
Blueprint<E, A> widget_event_blueprint(Build build, BuildHosted build_hosted, AnswerFn answer) {
    return [build, build_hosted, answer](Path path, Sink<A> sink, const Scope<E, A>& scope, std::uint32_t base, Cx& cx) -> Result<Supplied<E, A>> {
        auto hear = [sink, answer](const auto& v, Cx& cx) -> Done { return sink(answer(v), cx); };
        if (scope.is_world()) {
            auto made = TRY(build(path, base, cx));
            made.result->subscribe(hear);
            return Supplied<E, A>{made.node, std::nullopt, false, nullptr, made.built, made.leave};
        }
        auto made = TRY(build_hosted(path, base, scope.hosted, host_moved<E, A>(scope.hosted), cx));
        made.result->subscribe(hear);
        return Supplied<E, A>{made.node, std::nullopt, false, nullptr, made.built, made.leave, made.document, made.deliver};
    };
}

/// The rows a blueprint value of a block's own widget built (Semantics ch. 8: a blueprint
/// builds as often as it is applied), held weakly: the block's write behind what their
/// places read marks each one that lives. A row that left is dropped from the list when
/// the next one is added. Copies share the list.
template <class N>
class WeakRows {
public:
    WeakRows() : rows_(make_rc<std::vector<Weak<N>>>()) {}
    /// A row the blueprint built.
    void add(const Rc<N>& row) const {
        std::erase_if(*rows_, [](const Weak<N>& w) { return !w.lock(); });
        rows_->push_back(Weak<N>(row));
    }
    /// The rows that live.
    std::vector<Rc<N>> live() const {
        std::vector<Rc<N>> out;
        for (const Weak<N>& w : *rows_)
            if (auto row = w.lock()) out.push_back(row);
        return out;
    }

private:
    Rc<std::vector<Weak<N>>> rows_;
};

/// `@ x`: no places, the first value `x`, once.
template <class E, class A>
Blueprint<E, A> plain_blueprint(A x) {
    return [x](Path, Sink<A>, const Scope<E, A>&, std::uint32_t, Cx&) -> Result<Supplied<E, A>> {
        return Supplied<E, A>{make_rc<detail::Inert>(), x, false};
    };
}

/// `@ never`: no places, no first value, no answer.
template <class E, class A>
Blueprint<E, A> never_blueprint() {
    return [](Path, Sink<A>, const Scope<E, A>&, std::uint32_t, Cx&) -> Result<Supplied<E, A>> {
        return Supplied<E, A>{make_rc<detail::Inert>(), std::nullopt, false};
    };
}

/// `@ !e`: each firing of `e` is an answer to the child's place, a tick late.
template <class E, class A>
Blueprint<E, A> event_blueprint(Event<A> e) {
    return [e](Path, Sink<A> sink, const Scope<E, A>&, std::uint32_t, Cx&) -> Result<Supplied<E, A>> {
        auto node = make_rc<detail::Subscribed<A>>(e.port);
        node->handle = e.port->subscribe([sink](const A& v, Cx& cx) { return detail::later(sink, v, cx); });
        return Supplied<E, A>{node, std::nullopt, false};
    };
}

/// `@ ((x, !e))` for a bundle's tag (pick HO): `x`, the plain parts, the place's first value,
/// once; each firing of `e` an answer to the child's place, a tick late.
template <class E, class A>
Blueprint<E, A> bundle_blueprint(A first, Event<A> e) {
    return [first, e](Path, Sink<A> sink, const Scope<E, A>&, std::uint32_t, Cx&) -> Result<Supplied<E, A>> {
        auto node = make_rc<detail::Subscribed<A>>(e.port);
        node->handle = e.port->subscribe([sink](const A& v, Cx& cx) { return detail::later(sink, v, cx); });
        return Supplied<E, A>{node, first, false};
    };
}

/// `@ ((!e, $d))` for a bundle's tag with a `$` part (Type System ch. 2, "Bundles"; round
/// 3 DY): `d` is the `$` part in the tag's first-value case, `e` the event part in the
/// tag's own. `d`'s value is the place's first value, read at the build; each later
/// change of `d` is a push to the place and each firing of `e` an answer, both inputs of
/// the next tick, which is owed. As `dynamic_blueprint`, several marks of `d` in one tick
/// are one push, sampled when it is delivered.
template <class E, class A>
Blueprint<E, A> bundle_dynamic_blueprint(Dynamic<A> d, Event<A> e) {
    return [d, e](Path, Sink<A> sink, const Scope<E, A>&, std::uint32_t, Cx& cx) -> Result<Supplied<E, A>> {
        A first = d.sample(cx);
        auto node = make_rc<detail::SubscribedBoth<A>>(e.port, d.updates);
        node->event_handle = e.port->subscribe([sink](const A& v, Cx& cx) { return detail::later(sink, v, cx); });
        auto queued = make_rc<Stamped<Unit>>();
        node->updates_handle = d.updates->subscribe([d, sink, queued](const Unit&, Cx& cx) -> Done {
            if (queued->fired(cx)) return {};
            queued->set(Unit{}, cx);
            cx.answer_later([d, sink](Cx& cx) { return sink(d.sample(cx), cx); });
            return {};
        });
        return Supplied<E, A>{node, std::move(first), false};
    };
}

/// `@ $d`: the value after the birth tick is the place's first value, read now,
/// synchronously, as a World's first-value query is; each later change of `d` is a push to
/// the child's place as an input of the next tick, which is owed (Semantics ch. 9 as ruled,
/// round 3 DY). The port says only that `d` moved: the value is sampled when the push is
/// delivered, at the next tick's read point, so it is the value after the tick of the
/// change whatever height the change came at, and several marks in one tick are one push.
template <class E, class A>
Blueprint<E, A> dynamic_blueprint(Dynamic<A> d) {
    return [d](Path, Sink<A> sink, const Scope<E, A>&, std::uint32_t, Cx& cx) -> Result<Supplied<E, A>> {
        A first = d.sample(cx);
        auto node = make_rc<detail::Subscribed<Unit>>(d.updates);
        auto queued = make_rc<Stamped<Unit>>();
        node->handle = d.updates->subscribe([d, sink, queued](const Unit&, Cx& cx) -> Done {
            if (queued->fired(cx)) return {};
            queued->set(Unit{}, cx);
            cx.answer_later([d, sink](Cx& cx) { return sink(d.sample(cx), cx); });
            return {};
        });
        return Supplied<E, A>{node, std::move(first), false};
    };
}

/// The owner a supplier's dynamic answer reads through (`@ $d`, round 3 DY): the supplier
/// is made in the owner's build, before its node, and reads the node's getter for `d` once
/// it is linked (`Link::link`, beside `Host::link`). While the owner is being built, an
/// early row's first value reads the dynamic's value at the build instead (the emitted
/// read's other arm), since nothing fires at a build. Copies share one link.
template <class P>
class Link {
public:
    Link() : w_(make_rc<Weak<P>>()) {}
    void link(const Rc<P>& owner) const { *w_ = owner; }
    /// The owner, once linked and while it lives.
    Rc<P> owner() const { return w_->lock(); }

private:
    Rc<Weak<P>> w_;
};

/// `Link::upgrade`, as `Weak::upgrade` (lang.hpp's `upgrade`): a generation built before its
/// block's node is made holds its block through a link (a static widget of several clauses
/// whose result the block reads at its build), and reads it as a row reads its `parent`.
template <class P>
std::optional<Rc<P>> upgrade(const Link<P>& l) {
    if (auto p = l.owner()) return p;
    return std::nullopt;
}

namespace detail {

/// A forwarding blueprint's node: one place, the child's element re-emitted.
template <class E, class A>
struct Forward : Node {
    Path at;
    E element;
    Dynamic<E> level;
    Sink<A> sink;
    /// The re-emission on the level's updates, in the World's scope.
    std::optional<Handle> handle;

    Forward(Path at, E element, Dynamic<E> level, Sink<A> sink)
        : at(std::move(at)), element(std::move(element)), level(std::move(level)), sink(std::move(sink)) {}

    Done answer(std::span<const Segment> rest, const Pod& v, Cx& cx) override {
        if (rest.size() == 1 && rest[0].is_pos(0)) {
            auto a = TRY(A::decode(element, v));
            return sink(a, cx);
        }
        return unexpected(Fault::answer("no place at " + path_of(rest).to_string() + " under a forwarded element"));
    }
    /// A push is to the element's `$` output: of a bundle's tag its `$` part, which the
    /// tag's first-value case carries (`decode_first`; any other tag's is its answer).
    Done push(const Pod&, const Pod& v, Cx& cx) override {
        auto a = TRY(A::decode_first(element, v));
        return sink(a, cx);
    }
    Done leave(Cx& cx) override {
        if (handle) {
            level.updates->unsubscribe(*std::exchange(handle, std::nullopt));
            cx.removed(at);
        }
        return {};
    }
};

/// A placed description's node: one place, the description's element.
template <class E, class A>
struct Placed : Node {
    Path at;
    /// The level at the build: its tag decodes the place's answers.
    E element;
    Dynamic<E> level;
    Sink<A> sink;
    /// The re-emission on the level's updates.
    std::optional<Handle> handle;
    /// The event fields' subscriptions, the place's requests.
    std::vector<std::pair<Rc<Port<Pod>>, Handle>> requests;
    /// In a hosted scope, the place's entry in the enclosing host's document.
    Rc<Entry<E>> entry = nullptr;

    Placed(Path at, E element, Dynamic<E> level, Sink<A> sink)
        : at(std::move(at)), element(std::move(element)), level(std::move(level)), sink(std::move(sink)) {}

    Done answer(std::span<const Segment> rest, const Pod& v, Cx& cx) override {
        if (rest.size() == 1 && rest[0].is_pos(0)) {
            auto a = TRY(A::decode(element, v));
            return sink(a, cx);
        }
        return unexpected(Fault::answer("no place at " + path_of(rest).to_string() + " under a placed element value"));
    }
    /// A push is to the element's `$` output: of a bundle's tag its `$` part, which the
    /// tag's first-value case carries (`decode_first`; any other tag's is its answer).
    Done push(const Pod&, const Pod& v, Cx& cx) override {
        auto a = TRY(A::decode_first(element, v));
        return sink(a, cx);
    }
    Done leave(Cx& cx) override {
        if (handle) {
            level.updates->unsubscribe(*std::exchange(handle, std::nullopt));
            if (!entry) cx.removed(at);
        }
        for (auto& [port, h] : requests) port->unsubscribe(h);
        requests.clear();
        return {};
    }
};

}  // namespace detail

/// `@Element d` of a description (Type System ch. 2: an element applied is a plain value
/// until a statement or a host places it): a new element at `path/0`, as an inline element
/// statement there is. Its level is read at the build and sent (`cx.added`), and sent again
/// in the tick it moves; its first value is the World's, decoded by its tag; every answer
/// and push there is handed to the sink, the child's place; each firing of an event field
/// is the place's request. In a hosted scope the place is the row's own in the enclosing
/// host's document: an entry of its own (`Supplied::entry`), which takes its level at the
/// build and each time it moves; an enclosing host's answer there is handed to the sink.
template <class E, class A>
    requires AnswerOf<A, E>
Blueprint<E, A> described_blueprint(const Described<E>& d) {
    return [d](Path path, Sink<A> sink, const Scope<E, A>& scope, std::uint32_t base, Cx& cx) -> Result<Supplied<E, A>> {
        Path at = path.at(base);
        E identity = d.level.sample(cx);
        auto node = make_rc<detail::Placed<E, A>>(at, identity, d.level, std::move(sink));
        Weak<detail::Placed<E, A>> weak = node;
        std::optional<A> first;
        if (scope.is_world()) {
            Pod element = level_pod(identity, cx);
            if (auto v = cx.first_value(element)) {
                auto a = TRY(A::decode_first(identity, *v));
                first = std::move(a);
            }
            node->handle = d.level.updates->subscribe([weak](const Unit&, Cx& cx) -> Done {
                if (auto n = weak.lock()) cx.changed(n->at, level_pod(n->level.sample(cx), cx));
                return {};
            });
            for (const auto& [field, firings] : d.requests) {
                auto f = field;
                Handle h = firings.port->subscribe([at, f](const Pod& v, Cx& cx) -> Done {
                    cx.request(at, f, v);
                    return {};
                });
                node->requests.emplace_back(firings.port, h);
            }
            cx.added(at, element);
            if (first) cx.listen(element, node);
            return Supplied<E, A>{node, std::move(first), false, nullptr, 1};
        }
        // a place of the row's own in the enclosing host's document: an entry that takes the
        // level at the build and again each time it moves
        Rc<Entry<E>> entry = Entry<E>::make("an element value's place");
        entry->take(identity, cx);
        auto f = TRY(scope.hosted->first_value(at, identity, entry, cx));
        first = std::move(f);
        node->entry = entry;
        node->handle = d.level.updates->subscribe([weak](const Unit&, Cx& cx) -> Done {
            if (auto n = weak.lock()) n->entry->take(n->level.sample(cx), cx);
            return {};
        });
        // each firing of an event field is the place's request, a request of the enclosing
        // host's child: its row for the place hears it and hands it on
        for (const auto& [field, firings] : d.requests) {
            auto f = field;
            auto outer = scope.hosted;
            Handle h = firings.port->subscribe([at, f, outer](const Pod& v, Cx& cx) -> Done {
                return outer->request_from(at, f, v, cx);
            });
            node->requests.emplace_back(firings.port, h);
        }
        return Supplied<E, A>{node, std::move(first), false, entry, 1};
    };
}

/// `@Element e`: the child's element re-emitted at `path/0`, its level following the
/// child's, every answer and push there handed to the child's place in the same tick (the
/// transparent rule). In the World's scope the place goes out with `cx.added` and its first
/// value is the World's; in a hosted scope nothing goes out (the enclosing host's document
/// reads the row), and the first value is the enclosing host's.
template <class E, class A>
    requires AnswerOf<A, E>
Blueprint<E, A> element_blueprint(const Element<E>& value) {
    if (const Described<E>* d = std::get_if<Described<E>>(&value.form)) return described_blueprint<E, A>(*d);
    const Live<E>& e = std::get<Live<E>>(value.form);
    return [e](Path path, Sink<A> sink, const Scope<E, A>& scope, std::uint32_t base, Cx& cx) -> Result<Supplied<E, A>> {
        Path at = path.at(base);
        // The level at the build is the identity: a row is built from the entry after the
        // tick, and an early row's entry cannot be read until its host's node is made.
        Pod element = level_pod(e.identity, cx);
        std::optional<A> first;
        if (scope.is_world()) {
            if (auto v = cx.first_value(element)) {
                auto a = TRY(A::decode_first(e.identity, *v));
                first = std::move(a);
            }
        } else {
            // the forwarded element is the same place for the enclosing host: its entry
            auto f = TRY(scope.hosted->first_value(at, e.identity, e.entry, cx));
            first = std::move(f);
        }
        auto node = make_rc<detail::Forward<E, A>>(at, e.identity, e.level, std::move(sink));
        if (scope.is_world()) {
            Weak<detail::Forward<E, A>> weak = node;
            node->handle = e.level.updates->subscribe([weak](const Unit&, Cx& cx) -> Done {
                if (auto n = weak.lock()) cx.changed(n->at, level_pod(n->level.sample(cx), cx));
                return {};
            });
            cx.added(at, element);
            if (first) cx.listen(element, node);
        }
        // in a hosted scope the forwarded element is the child's place itself: its entry
        return Supplied<E, A>{node, std::move(first), true, scope.is_world() ? nullptr : e.entry, 1};
    };
}

/// `@HostSome`'s supplier made `@HostDocument`'s (Semantics ch. 9, the derivation): `nil`
/// means "not mine, forward it".
template <class E, class A>
    requires AnswerOf<A, E>
Supplier<E, A> forward_on_none(std::function<std::optional<Blueprint<E, A>>(const Path&, const Element<E>&)> some) {
    return [some](const Path& p, const Element<E>& e) -> Blueprint<E, A> {
        if (auto b = some(p, e)) return *b;
        return element_blueprint<E, A>(e);
    };
}

// ---------------------------------------------------------------------------------------
// The host and the rows of its `for`
// ---------------------------------------------------------------------------------------

/// A host as its rows reach it: the typed dispatch into the child's places, a child place's
/// first value (the early construction), the supplier, and the scope the rows' blueprints
/// build in. Implemented by `Host`, which erases the owner's type. A row reads its place
/// through the place's entry, not through the host.
template <class E, class A>
class HostOf {
public:
    virtual ~HostOf() = default;
    virtual Done deliver(const Path& place, const A& v, Cx& cx) const = 0;
    virtual Result<std::optional<A>> first_value(const Path& place, const E& element, const Rc<Entry<E>>& entry,
                                                 Cx& cx) = 0;
    virtual const Supplier<E, A>& supplier() const = 0;
    virtual Scope<E, A> scope() const = 0;
    /// A request from the place `place` of this host's child, which a host under this one
    /// hands up (a forwarded element's, or a placed description's): `Host::request`.
    virtual Done request_from(const Path& place, std::optional<std::string> field, Pod value, Cx& cx) const = 0;
    /// A place of this host's child came or went behind its document (a row a blueprint
    /// built there built or tore down a place of its own): the owner's `host_k_moved`.
    /// Nothing before the owner's node is made: its document's first snapshot is taken
    /// after.
    virtual void moved(Cx& cx) const = 0;
};

template <class E, class A>
Moved host_moved(const Rc<HostOf<E, A>>& host) {
    Weak<HostOf<E, A>> weak = host;
    return [weak](Cx& cx) {
        if (auto h = weak.lock()) h->moved(cx);
    };
}

template <class E, class A>
class HostRow;

/// The rows of a host's `for p, e <- $doc`, which the owner keeps in a field.
template <class E, class A>
using Rows = Rc<Map<Path, Rc<HostRow<E, A>>>>;

template <class E, class A>
Rows<E, A> new_rows() {
    return make_rc<Map<Path, Rc<HostRow<E, A>>>>();
}

/// One row of a host's `for p, e <- $doc`: the supplier's blueprint for child place `key`,
/// built at `host/[key]`. Its entry is the place's own (pick RE), which it reads as it
/// stands; the row ends at the end of the tick its key leaves (R8), its blueprint's node
/// with it, and nothing reads its entry after.
template <class E, class A>
class HostRow : public Node {
public:
    /// Built after propagation, from the entry after the tick (R7): the supplier is asked
    /// for the place's blueprint, and the blueprint is built under the row's path, in the
    /// host's scope. The `for` machinery of the emitted code calls this with the host as
    /// the row's parent. (Rust's `HostRow::build`: in C++ `build` is the virtual
    /// build-list method, which a static of that name would hide.)
    static Result<Rc<HostRow>> make(Path path, Path key, Rc<Entry<E>> value, const Rc<HostOf<E, A>>& host,
                                                  Cx& cx) {
        E identity = value->current(cx);
        return early(std::move(path), std::move(key), std::move(identity), std::move(value), host, cx);
    }

    /// Built with the place's identity given: a first-value query at the place's build,
    /// before its entry has a snapshot (the early construction).
    static Result<Rc<HostRow>> early(Path path, Path key, E value, Rc<Entry<E>> entry, const Rc<HostOf<E, A>>& host,
                                     Cx& cx) {
        auto node = rc_placed<HostRow>(
            [&](void* place) { return new (place) HostRow(std::move(path), std::move(key), std::move(entry), host); });
        node->place_->watch(Weak<Node>(NodeRef(node)));
        Weak<HostRow> row = node;
        node->sink_ = [row](const A& v, Cx& cx) -> Done {
            // An answer to a row that left, or one that is gone, is dropped (Semantics
            // ch. 9, "A late result is dropped"): a computed answer in flight to a place
            // whose key left.
            auto r = row.lock();
            if (r && !r->left_) return r->host()->deliver(r->key_, v, cx);
            return {};
        };
        Element<E> element = node->element(std::move(value));
        Blueprint<E, A> blueprint = host->supplier()(node->key_, element);
        auto supplied = TRY(blueprint(node->path_, node->sink_, host->scope(), 0, cx));
        node->supplied_ = std::move(supplied);
        return node;
    }

    /// An answer the host sends down to the row's place (`@HostList`): delivered as the
    /// next tick's input, and dropped if the key left by then.
    void answer_later(A v, Cx& cx) const {
        Sink<A> sink = sink_;
        cx.answer_later([sink, v](Cx& cx) { return sink(v, cx); });
    }

    /// The blueprint's first value: the place's, when its output has one.
    std::optional<A> first() const { return supplied_ ? supplied_->first : std::nullopt; }

    /// The row's entry at the read point: the place's level as it stands (its snapshot of
    /// this tick from the construction step on).
    E entry(const Cx& cx) const { return place_->current(cx); }

    /// The child place's request: out with the forwarded element, or nowhere when the
    /// supplier answered the place.
    /// The live element's event field hears it first (`Live::requested`).
    Done request(std::optional<std::string> field, Pod value, Cx& cx) const {
        TRY(requests_->fire(std::pair<std::optional<std::string>, Pod>{field, value}, cx));
        if (forwards()) {
            // the forwarded element is the host's own at `0` under the row: out to the
            // World, or, under a host that is itself hosted, a place of the enclosing
            // host's child, whose row decides again (a request rises host by host, each
            // adding its key)
            Scope<E, A> scope = host()->scope();
            if (scope.hosted) return scope.hosted->request_from(path_.at(0), std::move(field), std::move(value), cx);
            cx.request(path_.at(0), std::move(field), std::move(value));
        }
        return {};
    }

    /// The row's own places, for an enclosing host's document: the forwarded element at
    /// `0`, when there is one, which is the child's place itself (its entry); a placed
    /// element value's; or the places of a widget's row the blueprint built.
    std::vector<std::pair<Path, Rc<Entry<E>>>> entries(const Cx& cx) const {
        if (forwards()) return {{Path::root().at(0), place_}};
        if (supplied_ && supplied_->document) {
            std::vector<std::pair<Path, Rc<Entry<E>>>> out;
            for (auto& [p, e] : supplied_->document(cx)) out.emplace_back(p, e);
            return out;
        }
        if (supplied_ && supplied_->entry) return {{Path::root().at(0), supplied_->entry}};
        return {};
    }

    /// An enclosing host's typed answer to a place under the row: the forwarded element's,
    /// handed to the child's place now; a place of a widget's row the blueprint built,
    /// handed to the row.
    Done deliver(std::span<const Segment> at, const A& v, Cx& cx) const {
        if (supplied_ && supplied_->deliver) return supplied_->deliver(at, v, cx);
        if (at.size() == 1 && at[0].is_pos(0) && (forwards() || (supplied_ && supplied_->entry))) return sink_(v, cx);
        return unexpected(Fault::answer("no hosted place at " + path_of(at).to_string() + " under the host row at " +
                                        path_.to_string()));
    }

    /// The key left the child's document: the blueprint's node leaves with the row at the
    /// end of this tick (R8; a forwarded element is torn down in this tick's patch), and an
    /// answer in flight is dropped. The emitted prelude's `Row` calls this.
    void leave_row(Cx& cx) {
        left_ = true;
        if (supplied_) {
            NodeRef node = std::move(supplied_->node);
            std::function<void(Cx&)> leave = std::move(supplied_->leave);
            supplied_.reset();
            if (leave) leave(cx);
            else cx.keep(node->leave(cx));
        }
    }

    /// A World's answer under the row: to the blueprint's node (a forwarded element).
    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        if (supplied_) {
            NodeRef inner = supplied_->node;
            return inner->answer(at, v, cx);
        }
        return unexpected(Fault::answer("no place at " + path_of(at).to_string() + " under the host row at " +
                                        path_.to_string()));
    }
    /// The entry may have moved (the host marked the row): the live element's readers
    /// read it again, and a forwarded element re-emits.
    Done patch(Cx& cx) override { return level_moved_->fire(Unit{}, cx); }

    const Path& path() const { return path_; }
    const Path& key() const { return key_; }

private:
    HostRow(Path path, Path key, Rc<Entry<E>> place, const Rc<HostOf<E, A>>& host)
        : path_(std::move(path)), key_(std::move(key)), place_(std::move(place)), host_(host),
          level_moved_(make_rc<Port<Unit>>()),
          requests_(make_rc<Port<std::pair<std::optional<std::string>, Pod>>>()) {}

    Rc<HostOf<E, A>> host() const {
        auto h = host_.lock();
        if (!h) panic("a host outlives its rows");
        return h;
    }
    bool forwards() const { return supplied_ && supplied_->forwards; }

    /// The live element the supplier sees: the place's level at the build, and its level
    /// over time, the row's entry.
    Element<E> element(E identity) {
        Weak<HostRow> me = rc_static_cast<HostRow>(rc_from_this());
        auto read = make_rc<const std::function<E(const Cx&)>>([me](const Cx& cx) {
            auto r = me.lock();
            if (!r) panic("a live element outlives its host's row");
            return r->entry(cx);
        });
        return Element<E>{Live<E>{std::move(identity), Dynamic<E>{nullptr, read, level_moved_}, place_, requests_}};
    }

    Path path_;
    Path key_;
    Rc<Entry<E>> place_;
    Weak<HostOf<E, A>> host_;
    /// The key left: a computed answer in flight to the place is dropped.
    bool left_ = false;
    Sink<A> sink_;
    std::optional<Supplied<E, A>> supplied_;
    /// The entry moved: readers of the live element's level (a forwarded element) read it
    /// again.
    Rc<Port<Unit>> level_moved_;
    /// The place's requests, which the live element's event fields hear.
    Rc<Port<std::pair<std::optional<std::string>, Pod>>> requests_;
};

/// A host of owner `P` (the node its line and its child are inlined into): its place, its
/// rows, its supplier, the enclosing host when the line stands in a hosted scope, and the
/// owner's emitted typed dispatch.
template <class E, class A, class P>
    requires AnswerOf<A, E>
class Host : public HostOf<E, A>, public RcFromThis<Host<E, A, P>> {
public:
    using DeliverFn = std::function<Done(const Rc<P>&, const Path&, const A&, Cx&)>;
    using MovedFn = std::function<void(const Rc<P>&, Cx&)>;

    /// Made in the owner's build, before its node: `rows` is the field the `for` keeps,
    /// `enclosing` the host whose scope the line stands in, if any; `deliver` and `moved`
    /// the owner's emitted `host_k_answer` and `host_k_moved`.
    static Rc<Host> make(Path at, Rows<E, A> rows, Supplier<E, A> supplier,
                                      Rc<HostOf<E, A>> enclosing, DeliverFn deliver, MovedFn moved) {
        return rc_placed<Host>([&](void* place) {
            return new (place) Host(std::move(at), std::move(rows), std::move(supplier), std::move(enclosing),
                                    std::move(deliver), std::move(moved));
        });
    }

    /// The owner's node, once made: the rows read the document and answer through it.
    void link(const Rc<P>& owner) { owner_ = owner; }

    /// `@HostList`'s answers sent down by path (Semantics ch. 9, the list form; rulings
    /// round 12, AQ): each firing of `answers` hands the place at each of its keys that
    /// answer, typed by its tag, as a computed answer, the next tick's input, which is
    /// owed. A key with no place is dropped, as an answer to a place that left is.
    void hear(const Event<Map<Path, A>>& answers) {
        Weak<Host> me = this->rc_from_this();
        // the subscription holds the host weakly and lives as long as the event's port
        (void)answers.port->subscribe([me](const Map<Path, A>& m, Cx& cx) -> Done {
            auto host = me.lock();
            if (!host) return {};
            return host->send_down(m, cx);
        });
    }

    /// A child place's request: out with its forwarded element, or nowhere when the
    /// supplier answered the place (a host answers its child; a request reaches a World
    /// only from an element the host's blueprint emits).
    Done request(const Path& place, std::optional<std::string> field, Pod value, Cx& cx) const {
        if (auto row = rows_->get(place)) {
            return (*row)->request(std::move(field), std::move(value), cx);
        }
        return unexpected(Fault::answer("a request from " + place.to_string() + ", which has no row under the host at " +
                                        at_.to_string()));
    }

    Done deliver(const Path& place, const A& v, Cx& cx) const override { return deliver_(owner(), place, v, cx); }
    /// A child place's first value, asked at its build (Semantics ch. 9, "Queries are
    /// sync"): the supplier row for the place is built now, before the owner's node, and
    /// kept where the builder finds it. The one early construction.
    Result<std::optional<A>> first_value(const Path& place, const E& element, const Rc<Entry<E>>& entry,
                                         Cx& cx) override {
        Rc<HostOf<E, A>> me = this->rc_from_this();
        auto row = TRY((HostRow<E, A>::early(at_.child(Segment::key(to_pod(place))), place, element, entry, me, cx)));
        std::optional<A> first = row->first();
        rows_->insert(place, row);
        return first;
    }
    const Supplier<E, A>& supplier() const override { return supplier_; }
    Scope<E, A> scope() const override { return enclosing_ ? Scope<E, A>::in(enclosing_) : Scope<E, A>::world(); }
    Done request_from(const Path& place, std::optional<std::string> field, Pod value, Cx& cx) const override {
        return request(place, std::move(field), std::move(value), cx);
    }

    void moved(Cx& cx) const override {
        if (auto owner = owner_.lock()) moved_(owner, cx);
    }

    const Rows<E, A>& rows() const { return rows_; }

private:
    Host(Path at, Rows<E, A> rows, Supplier<E, A> supplier, Rc<HostOf<E, A>> enclosing, DeliverFn deliver, MovedFn moved)
        : at_(std::move(at)), rows_(std::move(rows)), supplier_(std::move(supplier)), enclosing_(std::move(enclosing)),
          deliver_(std::move(deliver)), moved_(std::move(moved)) {}

    Done send_down(const Map<Path, A>& m, Cx& cx) const {
        for (const auto& [p, v] : m) {
            if (auto row = rows_->get(p)) {
                Rc<HostRow<E, A>> r = *row;
                r->answer_later(v, cx);
            }
        }
        return {};
    }

    Rc<P> owner() const {
        auto o = owner_.lock();
        if (!o) panic("a host's node is linked at its build and outlives its rows");
        return o;
    }

    Path at_;
    Weak<P> owner_;
    Rows<E, A> rows_;
    Supplier<E, A> supplier_;
    Rc<HostOf<E, A>> enclosing_;
    DeliverFn deliver_;
    MovedFn moved_;
};

}  // namespace prism_rtl
