// The node primitives emitted code calls: the few generic pieces of state a template
// would otherwise emit identically in every program (claude/backend.md, "Node
// primitives").
//
// Every primitive is a plain member of its node (the library is single-threaded, so
// Rust's `RefCell` and `Cell` are ordinary fields), and each touches its state only inside
// its own methods, so no reference into a cell is held across a firing (Compiler ch. 4,
// "The ownership layer": a firing may deliver back into the node that fired). A read
// answers a copy; values are cheap to copy (collections.hpp).
//
// - `Hold`: a hold without a commit pass, two values each stamped with the tick that
//   wrote it (Graph ch. 4, "Holds without a commit pass").
// - `Stamped`: scratch between heights and a per-tick kept value; a stale stamp is an
//   empty field, and nothing clears it (Compiler ch. 3, CXa).
// - `Cache`: a computed behavior's cache with its stale stamp (Graph ch. 4,
//   "Invalidation by stamped marks").
// - `Snapshot`: a structural dynamic's value, taken in the construction step.
// - `Port`: an output event port across nodes: subscriptions with constant-time
//   handles, a lien count, and the tick's kept value for a subscriber that catches up.
// - `Event`, `Dynamic`, `Live`: the `!a`, `$a` and `Element r o` values that cross the
//   node interface.
// - `Defer`: `@Defer` (decisions.md D6).
// - `Job`, `Turn`: the step machine of an `unbounded` function, which `@Run` cycles.

#pragma once

#include "prism_rtl/rc.hpp"
#include <algorithm>
#include <any>
#include <deque>
#include <functional>
#include <limits>
#include <memory>
#include <optional>
#include <string>
#include <typeinfo>
#include <utility>
#include <variant>
#include <vector>

#include "prism_rtl/cx.hpp"

namespace prism_rtl {

// ---------------------------------------------------------------------------------------
// Hold
// ---------------------------------------------------------------------------------------

/// `$x <- init then !e`'s storage: the newer and the older value, each with the tick that
/// wrote it. A read during tick `n` answers the newer value stamped before `n`; from the
/// end of propagation on, the one stamped at or before `n`. Nothing installs anything at
/// the end of a tick.
template <class T>
class Hold {
public:
    /// A hold's initial value, stamped with the tick that built it.
    Hold(T init, const Cx& cx) : newer_(std::move(init)), stamp_(cx.tick()) {}
    /// The value at the context's read point: before the tick during propagation, after
    /// it from the end of propagation on.
    T get(const Cx& cx) const {
        if (older_ && stamp_ >= cx.read_point()) return older_->first;
        return newer_;
    }
    /// The tick's one write (G1, Semantics R3): the older slot takes the value, stamped
    /// now. A second write in a tick is a node woken twice for the same inputs: a fault of
    /// the tick.
    void set(T v, const Cx& cx) {
        if (stamp_ == cx.tick() && older_) {
            cx.fail_later(Fault("Runtime.TwoWrites", "a hold written twice in tick " + std::to_string(cx.tick())));
            return;
        }
        older_.emplace(std::move(newer_), stamp_);
        newer_ = std::move(v);
        stamp_ = cx.tick();
    }
    /// The newer value, whatever the tick: what a dead widget's hold is from then on.
    T newest() const { return newer_; }

private:
    T newer_;
    Tick stamp_;
    /// The older value and its stamp, once there has been a write.
    std::optional<std::pair<T, Tick>> older_;
};

// ---------------------------------------------------------------------------------------
// Stamped
// ---------------------------------------------------------------------------------------

/// A value and the tick that wrote it: a merge's input record, a kept firing, a fork
/// point. It reads as empty in every other tick.
template <class T>
class Stamped {
public:
    void set(T v, const Cx& cx) { cell_.emplace(std::move(v), cx.tick()); }
    /// The value written this tick, if one was.
    std::optional<T> get(const Cx& cx) const {
        if (cell_ && cell_->second == cx.tick()) return cell_->first;
        return std::nullopt;
    }
    /// The value written this tick, consumed: a combinator decides each input once.
    std::optional<T> take(const Cx& cx) {
        auto v = get(cx);
        if (v) cell_.reset();
        return v;
    }
    /// Whether it was written this tick: an event input that fired.
    bool fired(const Cx& cx) const { return cell_ && cell_->second == cx.tick(); }

private:
    std::optional<std::pair<T, Tick>> cell_;
};

// ---------------------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------------------

/// A computed behavior's cache: the value, the read point it was computed at, and the
/// first read point from which it is stale. A mark at a write in tick `n` makes it stale
/// from after `n`, so a reader in the tick of the write reads the value the tag law calls
/// clean, and the reader after recomputes once. A value computed at one read point serves
/// only reads at or past it (and before the stale point): a read after the tick made
/// during propagation (`Cx::after`) caches the value after the tick, and a reader back
/// at the tick's own read point is not served it.
template <class T>
class Cache {
public:
    /// The cached value at the read point, or `compute`'s, which is then cached. A read
    /// below the cached read point computes and keeps nothing: the cache stays the later
    /// value, and the marks that made it stale were already consumed by that read. A read
    /// is `const`, as Rust's `Cache::get(&self)` over its `RefCell`: what it caches is the
    /// cache's own state (`mutable`), never the node's.
    template <class F>
    T get(const Cx& cx, F&& compute) const {
        Tick at = cx.read_point();
        if (value_) {
            if (value_->second <= at && at < stale_from_) return value_->first;
            if (at < value_->second) return compute();
        }
        T v = compute();
        if (at >= stale_from_) stale_from_ = NEVER;
        value_.emplace(v, at);
        return v;
    }
    /// An input was written this tick: stale from after it.
    void mark(const Cx& cx) { stale_from_ = std::min(stale_from_, cx.tick() + 1); }

private:
    static constexpr Tick NEVER = std::numeric_limits<Tick>::max();
    mutable std::optional<std::pair<T, Tick>> value_;
    mutable Tick stale_from_ = NEVER;
};

// ---------------------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------------------

/// A structural dynamic's value (Semantics ch. 7, "Structural dynamics"; round 3 ST): a
/// host's child document, a `for`'s gathered `$(K ==> T)`, anything whose value is made of
/// constructed structure. Such a value after tick `n` cannot exist before `n`'s
/// construction step has run, so it is never computed during propagation: the node takes
/// it once, at the point of its construction step where the structure behind it has
/// settled (`Snapshot::take`, `Snapshot::take_compared`), and every read answers a
/// snapshot. A read at read point `n` (a sample during propagation, or under
/// `Cx::before`) answers the snapshot of `n - 1`; a read after the tick answers the
/// snapshot of `n`, taken or, when nothing marked the structure this tick, the one that
/// stands.
///
/// A write behind the structure `Snapshot::mark`s the snapshot; the node's construction
/// step takes it when marked. Reading a marked snapshot after the tick before it is taken
/// is a bug of the compiler, not a stale value: a read under `Cx::after` during
/// propagation, or a builder that runs before the structure it reads has settled. It
/// panics with the reason.
template <class T>
class Snapshot {
public:
    /// An empty snapshot; `what` names the dynamic in the panic message.
    explicit Snapshot(std::string what) : what_(std::move(what)) {}
    /// A write behind the structure this tick: the construction step takes the snapshot.
    void mark(const Cx& cx) { marked_ = cx.tick(); }
    /// Whether a write marked the structure this tick and the snapshot is not taken yet.
    bool marked(const Cx& cx) const { return marked_ == cx.tick(); }
    /// The snapshot of this tick taken, the structure after the tick; a second take in the
    /// same tick replaces the first. Without equality every take after the node's first
    /// tick moves the dynamic. Answers whether it moved.
    bool take(T v, const Cx& cx) {
        return put(std::move(v), cx, [](const T&, const T&) { return true; });
    }
    /// The snapshot taken and compared with the one that stood before this tick: whether
    /// the dynamic moved (R3: its update event fires only when the value changed). When it
    /// did not, the snapshot that stood is kept and `v` dropped.
    bool take_compared(T v, const Cx& cx) {
        return put(std::move(v), cx, [](const T& a, const T& b) { return !(a == b); });
    }
    /// Whether the node queues the publication now: the snapshot taken this tick moved the
    /// dynamic, and nothing queued it yet this tick (one firing per tick).
    bool queue(const Cx& cx) {
        bool first = moved_ == cx.tick() && queued_ != cx.tick();
        if (first) queued_ = cx.tick();
        return first;
    }
    /// At the start of tick `n + 1`: whether S(n) still differs from S(n-1), so the
    /// publication writes the dynamic.
    bool published(const Cx& cx) const { return moved_ && *moved_ + 1 == cx.tick(); }
    /// Whether a snapshot was ever taken: a place's entry takes its first at its node's
    /// build, uncompared.
    bool taken() const { return newest_.has_value(); }
    /// The dynamic at the read point: its value after the tick before the read point, the
    /// snapshot of the tick before that (S(m-1) after `m`); the build's snapshot until then.
    T get(const Cx& cx) const {
        Tick at = cx.read_point();
        if (!newest_) panic(what_ + " read before its first snapshot was taken");
        if (newest_->second + 2 <= at) return newest_->first;
        if (older_) return older_->first;
        return newest_->first;
    }
    /// The structure at the read point: the newest snapshot taken before it, S(n) from
    /// tick `n`'s construction step on, S(n-1) during `n`'s propagation.
    T current(const Cx& cx) const {
        Tick at = cx.read_point();
        if (!newest_) panic(what_ + " read before its first snapshot was taken");
        if (newest_->second < at) {
            if (at > cx.tick() && marked(cx)) {
                std::string how = cx.propagating() ? "under `Cx::after` during propagation"
                                                   : "by a builder that runs before the structure it reads has settled";
                panic(what_ + " read after tick " + std::to_string(cx.tick()) +
                      " before its snapshot was taken: a structure read " + how);
            }
            return newest_->first;
        }
        if (older_) return older_->first;
        panic(what_ + " read at read point " + std::to_string(at) + " before its first snapshot, taken in tick " +
              std::to_string(newest_->second));
    }

private:
    template <class D>
    bool put(T v, const Cx& cx, D differs) {
        bool moved = false;
        if (!newest_) {
            newest_.emplace(std::move(v), cx.tick());
        } else if (newest_->second == cx.tick()) {
            // A take equal to the snapshot it is compared with keeps that snapshot, so its
            // sharing survives the tick and the next comparison of it is cheap: no program
            // tells two equal values apart (Semantics ch. 11, "sharing is invisible").
            if (older_) moved = differs(older_->first, v);
            newest_->first = older_ && !moved ? older_->first : std::move(v);
        } else {
            moved = differs(newest_->first, v);
            older_ = std::move(newest_);
            newest_.emplace(moved ? std::move(v) : older_->first, cx.tick());
        }
        moved_ = moved ? std::optional<Tick>(cx.tick()) : std::nullopt;
        marked_.reset();
        return moved;
    }
    std::optional<std::pair<T, Tick>> newest_, older_;
    std::optional<Tick> marked_, moved_, queued_;
    std::string what_;
};

// ---------------------------------------------------------------------------------------
// Ports and the values that cross the node interface
// ---------------------------------------------------------------------------------------

/// A subscription's handle, for unsubscribing in constant time.
struct Handle {
    std::size_t index;
    bool operator==(const Handle&) const = default;
};

/// A subscriber: a typed closure holding its consumer weakly, since references held
/// downstream are weak (Graph ch. 4). Shared, so a firing copies the list cheaply.
template <class T>
using Subscriber = Rc<const std::function<Done(const T&, Cx&)>>;

/// How much an output is wanted (Graph ch. 4, "Demand"). Tags are a behavior's, and the
/// node that owns the behavior counts them.
enum class Demand { Idle, Liened, Subscribed };

/// An output event port across nodes.
template <class T>
class Port {
public:
    using Value = T;
    Handle subscribe(Subscriber<T> f) {
        if (!free_.empty()) {
            std::size_t i = free_.back();
            free_.pop_back();
            subs_[i] = std::move(f);
            return Handle{i};
        }
        subs_.push_back(std::move(f));
        return Handle{subs_.size() - 1};
    }
    /// Any callable `Done(const T&, Cx&)`.
    template <class F>
        requires std::is_invocable_r_v<Done, F, const T&, Cx&>
    Handle subscribe(F&& f) {
        return subscribe(make_rc<const std::function<Done(const T&, Cx&)>>(std::forward<F>(f)));
    }
    void unsubscribe(Handle h) {
        if (h.index < subs_.size() && subs_[h.index]) {
            subs_[h.index].reset();
            free_.push_back(h.index);
        }
    }
    void lien() { ++liens_; }
    void release() {
        if (liens_ > 0) --liens_;
    }
    Demand demand() const {
        if (std::any_of(subs_.begin(), subs_.end(), [](const auto& s) { return bool(s); })) return Demand::Subscribed;
        if (liens_ > 0) return Demand::Liened;
        return Demand::Idle;
    }
    /// Fire: keep the value for the rest of the tick, then deliver it to every
    /// subscriber. The list is copied first, so a subscriber may unsubscribe.
    Done fire(const T& v, Cx& cx) {
        kept_.set(v, cx);
        std::vector<Subscriber<T>> subs;
        for (const auto& s : subs_)
            if (s) subs.push_back(s);
        for (const auto& f : subs) TRY((*f)(v, cx));
        return {};
    }
    /// What it fired this tick: a subscriber arriving at a height after the firing
    /// catches up from it. A newborn or a swap never does.
    std::optional<T> kept(const Cx& cx) const { return kept_.get(cx); }

private:
    std::vector<Subscriber<T>> subs_;
    std::vector<std::size_t> free_;
    std::uint32_t liens_ = 0;
    Stamped<T> kept_;
};

/// An `!a` value: an output port and the node that owns it, held strongly, since a
/// value's `!` part liens its producer.
template <class T>
struct Event {
    NodeRef owner;
    Rc<Port<T>> port;

    /// `never`: a port nothing fires.
    static Event never() { return Event{nullptr, make_rc<Port<T>>()}; }
    /// A node's own event as a value: the port its firing method fires.
    static Event of(const Rc<Port<T>>& port) { return Event{nullptr, port}; }
    /// Events compare by identity, the port they fire, as dynamics do: an element's level
    /// holds the events of its event fields (ruling EP).
    bool operator==(const Event& o) const { return ptr_eq(port, o.port); }
    /// The event `f` of this one, a pure transform at each firing (a lifted call).
    template <class F>
    auto map(F f) const -> Event<std::invoke_result_t<F, const T&>> {
        using U = std::invoke_result_t<F, const T&>;
        auto out = make_rc<Port<U>>();
        port->subscribe([out, f](const T& v, Cx& cx) -> Done { return out->fire(f(v), cx); });
        return Event<U>{owner, out};
    }
    /// The event `f` of this one, `f` given each firing's read point: a lifted call that
    /// samples a dynamic at the firing (`(F $d) on !e` as an event value).
    template <class F>
    auto map_at(F f) const -> Event<std::invoke_result_t<F, const T&, const Cx&>> {
        using U = std::invoke_result_t<F, const T&, const Cx&>;
        auto out = make_rc<Port<U>>();
        port->subscribe([out, f](const T& v, Cx& cx) -> Done { return out->fire(f(v, cx), cx); });
        return Event<U>{owner, out};
    }
};

/// `!Filter`: the present values.
template <class U>
Event<U> filter_some(const Event<std::optional<U>>& e) {
    auto out = make_rc<Port<U>>();
    e.port->subscribe([out](const std::optional<U>& v, Cx& cx) -> Done {
        if (v) return out->fire(*v, cx);
        return {};
    });
    return Event<U>{e.owner, out};
}

/// A `$a` value: a read at the context's read point, its update event, and its owner.
/// The update event says only that the value moved; a reader reads it again, so a
/// projection of a dynamic shares its source's port.
template <class T>
struct Dynamic {
    NodeRef owner;
    Rc<const std::function<T(const Cx&)>> read;
    Rc<Port<Unit>> updates;

    /// `$ v`: a constant, whose update event never fires.
    static Dynamic constant(T v) {
        return Dynamic{nullptr, make_rc<const std::function<T(const Cx&)>>([v](const Cx&) { return v; }),
                       make_rc<Port<Unit>>()};
    }
    /// A dynamic with no owner over a read and the port that says it moved: a host's
    /// dynamic answer to a child place (`@ $d`), whose read goes through the owner's
    /// getter.
    template <class F>
    static Dynamic of(F read, const Rc<Port<Unit>>& updates) {
        return Dynamic{nullptr, make_rc<const std::function<T(const Cx&)>>(std::move(read)), updates};
    }
    T sample(const Cx& cx) const { return (*read)(cx); }
    /// Two dynamics are equal when they are the same dynamic: the same read and the same
    /// update event, never by their values (a dynamic has no `^Eq` in Prism, and no program
    /// compares one). This is what the runtime's own comparisons see where a value holds
    /// dynamics (a host document's entries, an element whose children are dynamics): a
    /// place's entry is the same dynamic for as long as the place lives (doubt EQ).
    bool operator==(const Dynamic& o) const { return ptr_eq(read, o.read) && ptr_eq(updates, o.updates); }
    /// The dynamic `f` of this one: the same owner and update event, the read mapped.
    template <class F>
    auto map(F f) const -> Dynamic<std::invoke_result_t<F, const T&>> {
        using U = std::invoke_result_t<F, const T&>;
        auto r = read;
        return Dynamic<U>{owner, make_rc<const std::function<U(const Cx&)>>([r, f](const Cx& cx) { return f((*r)(cx)); }),
                          updates};
    }
    /// The dynamic `f` of this one and `other` (a lift over two dynamics in a function that
    /// takes them whole, Type System ch. 4): read from both; its update event fires, once a
    /// tick, when either moves.
    template <class U, class F>
    auto map2(const Dynamic<U>& other, F f) const -> Dynamic<std::invoke_result_t<F, const T&, const U&>> {
        using V = std::invoke_result_t<F, const T&, const U&>;
        auto a = read;
        auto b = other.read;
        auto merged = make_rc<Port<Unit>>();
        for (const auto& src : {updates, other.updates}) {
            src->subscribe([merged](const Unit&, Cx& cx) -> Done { return merged->kept(cx) ? Done{} : merged->fire(Unit{}, cx); });
        }
        return Dynamic<V>{nullptr, make_rc<const std::function<V(const Cx&)>>([a, b, f](const Cx& cx) { return f((*a)(cx), (*b)(cx)); }),
                          merged};
    }
};

template <class E>
class Entry;

/// A live element, `Element r o`: what a host's supplier receives for each of its child's
/// places. `identity` is the place's level at its build, whose tag and plain fields never
/// change while it lives; `level` is its level over time, which a `$` field reads; `entry`
/// is the place's entry in its host's document (a forwarded element's place in an
/// enclosing host's document is the same entry).
template <class E>
struct Live {
    E identity;
    Dynamic<E> level;
    Rc<Entry<E>> entry;
    /// The place's requests as its host's row hears them: the event field's name when the
    /// tag has several, and the value fired.
    Rc<Port<std::pair<std::optional<std::string>, Pod>>> requests;

    /// One event field of the live element as an `!T` (an element pattern's `!field=!r`,
    /// ruling EP): each request the place makes through `field` (none: the tag's only
    /// event field), decoded, in the tick the place fires it. The event's type is the one
    /// the value is given to (a description's field): C++ deduces no template argument
    /// from a result, so `requested` gives a `Requested`, which converts to that `Event<T>`.
    struct Requested {
        Rc<Port<std::pair<std::optional<std::string>, Pod>>> requests;
        std::optional<std::string> field;

        template <class T>
        operator Event<T>() const {
            auto port = make_rc<Port<T>>();
            requests->subscribe([port, field = field](const std::pair<std::optional<std::string>, Pod>& request, Cx& cx) -> Done {
                if (request.first != field) return {};
                auto decoded = from_pod<T>(request.second);
                if (!decoded) return unexpected(decoded.error());
                return port->fire(*decoded, cx);
            });
            return Event<T>{nullptr, port};
        }
    };
    Requested requested(std::optional<std::string> field) const { return Requested{requests, std::move(field)}; }
};

/// An element applied as a plain value, before a statement or a host places it (Type System
/// ch. 2, "Names and sigils"): its tag's description record as the program's own emitted
/// type (`desc`, which a pattern takes back by its type, `element_view`), its level over
/// time, mapped from its `$` fields, and each event field's firings, the requests of the
/// place once it is placed. It reads nothing until it is placed.
template <class E>
struct Described {
    std::any desc;
    Dynamic<E> level;
    std::vector<std::pair<std::optional<std::string>, Event<Pod>>> requests;

    /// An event field of the description: its firings are the placed element's requests,
    /// `field` naming it when the tag has several.
    Described request(std::optional<std::string> field, Event<Pod> firings) const {
        Described out = *this;
        out.requests.emplace_back(std::move(field), std::move(firings));
        return out;
    }
};

/// A description of a tag: `desc` its description record, `level` its level over time.
template <class E, class D>
Described<E> described(D desc, Dynamic<E> level) {
    return Described<E>{std::any(std::move(desc)), std::move(level), {}};
}

/// An element value, `Element r o` (Semantics RULINGS round 12, EV: elements are ordinary
/// values): a live element, which a host's supplier receives, or a description, a tag
/// applied.
template <class E>
struct Element {
    std::variant<Live<E>, Described<E>> form;
};

/// A description as an element value.
template <class E>
Element<E> described_element(Described<E> d) {
    return Element<E>{std::move(d)};
}

/// An element seen as the description `D` of one tag: a description of that tag as it is, a
/// live element as `of_live` gives it (the description of its level, when it is of the
/// tag), else nothing.
template <class E, class D, class F>
std::optional<D> element_view(const Element<E>& e, F of_live) {
    if (const Live<E>* l = std::get_if<Live<E>>(&e.form)) return of_live(*l);
    const Described<E>& d = std::get<Described<E>>(e.form);
    if (const D* x = std::any_cast<D>(&d.desc)) return *x;
    return std::nullopt;
}

/// Whether an element is of the tag whose description is `D` (`element_view`).
template <class E, class D, class F>
bool element_is(const Element<E>& e, F of_live) {
    return element_view<E, D, F>(e, std::move(of_live)).has_value();
}

// ---------------------------------------------------------------------------------------
// @Defer, and the step machines @Run cycles
// ---------------------------------------------------------------------------------------

/// `@Defer: fn !a -> @!a`: each firing fires again in the next tick, by the runtime, with
/// no World involved, and an armed `@Defer` sets `pending` (decisions.md D6).
///
/// A `Defer` is a field of the node that binds it. `Defer::arm` records the value and
/// puts the node on the armed list; in the next tick the runtime calls the node's
/// `Node::deferred` at the lowest height, where `Defer::fired` gives the value.
template <class T>
class Defer {
public:
    /// The argument fired: fire it again next tick.
    void arm(T v, NodeRef node, Cx& cx) {
        armed_.emplace(std::move(v), cx.tick());
        cx.arm(std::move(node));
    }
    /// `@Defer`'s firing this tick: what was armed in the tick before.
    std::optional<T> fired(const Cx& cx) const {
        if (armed_ && armed_->second + 1 == cx.tick()) return armed_->first;
        return std::nullopt;
    }

private:
    std::optional<std::pair<T, Tick>> armed_;
};

/// `@Later: fn !a -> @!a` (Semantics Appendix B): each firing answered one tick later,
/// through a zero-delay `Time.After`. The firings whose wake the World has not answered
/// yet, oldest first: `LaterQueue::queue` at each firing, as the wake is asked;
/// `LaterQueue::answered` at each answer of the place, which gives the oldest back.
template <class T>
class LaterQueue {
public:
    /// The argument fired: kept until its wake is answered.
    void queue(T v) const { pending_.push_back(std::move(v)); }
    /// The place was answered: the oldest firing still kept. An answer with none kept is
    /// a World answering a wake nobody asked for.
    T answered() const {
        if (pending_.empty()) panic("an answer at the place of an `@Later` that asked for no wake");
        T v = std::move(pending_.front());
        pending_.pop_front();
        return v;
    }

private:
    mutable std::deque<T> pending_;
};

template <class B>
struct Turn;

/// An `unbounded` function stopped at a cycle point: its continuation (Semantics ch. 7,
/// "An unbounded function is a bounded body iterated"). The compiler emits one class per
/// group of unbounded functions, whose frames are the cycle points of the canonical rule
/// (a call of an unbounded function from an unbounded function), and `Start F, x` is the
/// job stopped before its first cycle.
template <class B>
class Job {
public:
    using Value = B;
    virtual ~Job() = default;
    /// One cycle: run bounded code from here to the next cycle point, or to the result.
    virtual Turn<B> cycle() const = 0;
};

template <class B>
using JobRef = Rc<const Job<B>>;

/// What a cycle ends in: stopped at a cycle point, where the next cycle resumes (`More`),
/// or the result (`Finished`).
template <class B>
struct Turn {
    using Value = B;
    struct More {
        JobRef<B> next;
    };
    struct Finished {
        B value;
    };
    std::variant<More, Finished> v;

    static Turn more(JobRef<B> next) { return Turn{More{std::move(next)}}; }
    static Turn finished(B value) { return Turn{Finished{std::move(value)}}; }
    const More* if_more() const { return std::get_if<More>(&v); }
    const Finished* if_finished() const { return std::get_if<Finished>(&v); }
};

}  // namespace prism_rtl
