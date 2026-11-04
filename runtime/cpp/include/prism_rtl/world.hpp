// The protocol every World shares with every program, and the loop the CLI and canvas
// Worlds run (Semantics ch. 9, "The foreign World's rules"; Graph ch. 3).
//
// A program is a `Program`: `build` is tick 0, `tick` one tick, `compact` the free time
// between. A tick is sync for the World: it hands in `TickIn` (answers by path, pushes
// keyed by element value) and gets `TickOut` back (the patch, the requests in document
// order, the result, `pending`). The one call into the World inside a tick is the
// first-value query, `ReadyState::first_value`, answered from what the World holds.
// Everything that crosses is a `Pod`: the World holds no node and no typed value.
//
// - test_world.hpp: the Test World, which runs scripts in the test language and prints
//   transcripts to the byte.
// - scene.hpp: `Draw.Clickable` in a `Gui.Scene`, which the Test World clicks by path
//   (decisions.md D9) and the canvas World ignores.

#pragma once

#include "prism_rtl/rc.hpp"
#include <cstdint>
#include <cstdio>
#include <map>
#include <memory>
#include <optional>
#include <span>
#include <utility>
#include <vector>

#include "prism_rtl/doc.hpp"
#include "prism_rtl/pod.hpp"

namespace prism_rtl {

/// A tick's inputs: answers by path, to places that existed as the tick opened, and
/// pushes keyed by element value, to every place with that element.
struct TickIn {
    std::vector<std::pair<Path, Pod>> answers;
    std::vector<std::pair<Pod, Pod>> pushes;

    static TickIn none() { return {}; }
    TickIn answer(Path path, Pod v) && {
        answers.emplace_back(std::move(path), std::move(v));
        return std::move(*this);
    }
    TickIn push(Pod element, Pod v) && {
        pushes.emplace_back(std::move(element), std::move(v));
        return std::move(*this);
    }
};

/// The root's result, when it is not `unit`.
struct Outcome {
    enum class Kind : std::uint8_t {
        /// A plain result at the build, or a dynamic one at the build and when it changes.
        Value,
        /// An event result that fired this tick.
        Fired,
    };
    Kind kind;
    Pod pod;
    static Outcome value(Pod p) { return Outcome{Kind::Value, std::move(p)}; }
    static Outcome fired(Pod p) { return Outcome{Kind::Fired, std::move(p)}; }
    bool operator==(const Outcome&) const = default;
};

/// A tick's outputs.
struct TickOut {
    Tick tick = 0;
    Patch patch;
    /// In document order, torn-down places included.
    std::vector<Request> requests;
    std::optional<Outcome> result;
    /// Set while a newborn owes a tick or a `@Defer` is armed: a World that sees it ticks
    /// again, with no inputs if it has none (decisions.md D6).
    bool pending = false;
    bool operator==(const TickOut&) const = default;
};

/// What a World holds as the tick opens, plus pure computation over it (round 47 RS).
class ReadyState {
public:
    virtual ~ReadyState() = default;
    /// The first value of a new place whose element's `Level` is `element`.
    virtual std::optional<Pod> first_value(const Pod& element) const = 0;
};

/// Entries keyed by element value, as the test language's `with` lines write them.
struct ReadyMap : ReadyState {
    std::map<Pod, Pod> entries;
    std::optional<Pod> first_value(const Pod& element) const override {
        auto it = entries.find(element);
        if (it == entries.end()) return std::nullopt;
        return it->second;
    }
};

/// A compiled program, as every World drives it. The templates emit one subclass per
/// program, around its root node and a `Runtime`, with a static
///
///     static Result<std::pair<std::unique_ptr<P>, TickOut>> build(std::span<const Pod> args, const ReadyState&);
///
/// for tick 0, `tick` on no state: the root built from its arguments (as plain data, so
/// a test line's `@Normal (App ..)` reaches it).
class Program {
public:
    virtual ~Program() = default;
    /// One tick. On a fault the tick never happened, and the World decides what next.
    virtual Result<TickOut> tick(const TickIn& input, const ReadyState& ready) = 0;
    /// The compact round, in the free time after a tick's outputs have left.
    virtual void compact() {}
};

/// What a program type's static `build` answers.
template <class P>
using Built = Result<std::pair<std::unique_ptr<P>, TickOut>>;

/// A poisoned root or a failed tick (Semantics ch. 9, "The CLI World's policy").
inline constexpr std::uint8_t EXIT_FAILED = 1;
/// A program that fails to check, before tick 0.
inline constexpr std::uint8_t EXIT_UNCHECKED = 2;

/// A foreign World, as `drive` sees it.
class World {
public:
    virtual ~World() = default;
    /// The ready state the next tick's first-value queries are answered from.
    virtual Rc<const ReadyState> ready() const = 0;
    /// Perform a tick's outputs: its requests in document order, torn-down places
    /// included, before acting on the patch.
    virtual void perform(const TickOut& out) = 0;
    /// The next tick's inputs, or none when nothing the World holds can cause another.
    virtual std::optional<TickIn> next_inputs() = 0;
    /// The code the program asked to exit with, once it has.
    virtual std::optional<std::uint8_t> exit_code() const = 0;
    /// A failed tick, the World's own event. The CLI policy: report it and exit 1.
    virtual std::uint8_t failed(const Fault& fault) {
        std::fprintf(stderr, "prism: a tick failed: %s\n", fault.to_string().c_str());
        return EXIT_FAILED;
    }
};

/// Run a program under a World until it exits: build, then tick while the World has
/// inputs or the program is `pending`, compacting after every tick. `patience` bounds the
/// ticks: a World's patience, not a semantics.
template <class P>
std::uint8_t drive(World& world, std::span<const Pod> args, std::size_t patience) {
    auto built = P::build(args, *world.ready());
    if (!built) return world.failed(built.error());
    std::unique_ptr<P> program = std::move(built.value().first);
    TickOut out = std::move(built.value().second);
    for (std::size_t i = 0; i < patience; ++i) {
        world.perform(out);
        if (auto code = world.exit_code()) return *code;
        program->compact();
        std::optional<TickIn> input = world.next_inputs();
        if (!input) {
            if (!out.pending) return 0;
            input = TickIn::none();
        }
        auto next = program->tick(*input, *world.ready());
        if (!next) return world.failed(next.error());
        out = std::move(next).value();
    }
    std::fprintf(stderr, "prism: the World's patience ran out after %zu ticks\n", patience);
    return EXIT_FAILED;
}

}  // namespace prism_rtl
