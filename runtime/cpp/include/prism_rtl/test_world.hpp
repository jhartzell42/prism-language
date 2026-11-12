// The Test World: runs a script in the test language against any `Program` and prints
// its transcript to the byte (Conformance ch. 1 and Appendix A), the same bytes as the
// Rust Test World (runtime/rust/src/world/test_world.rs).
//
// - **One line is one tick,** headed `tick n` from 0, except `compute until idle`, which
//   ticks while the program says `pending` and prints `compute until idle: k ticks, n to
//   m`, then each of its ticks that has a section (decisions.md D6). With none to run it
//   prints `0 ticks`, and with one, `1 tick, n to n`.
// - **Nothing arrives that the test did not write,** but the prelude's own answers. It
//   never opens a tick on its own, never answers a request unless the test writes
//   `answer`, and answers first-value queries from the `with` and `push` entries, then by
//   Pitch ch. 6's policy for pictures: `Gui.Window` with `WindowId title`, `Gui.Metrics`
//   from a font nine units a character and fourteen high.
// - **`Prelude.Unwrap` and `Prelude.Exceptions` are answered as every World answers
//   them** (prelude.hpp; Semantics ch. 10): an unwrap's `~ x` at tick n is answered with
//   `x` as an input of the next tick the test opens, shown under that tick's `in:` as a
//   delivery is, and the test is owed that tick. A poisoned subtree is served nothing
//   again, a test's own delivery included.
// - **Sections**, each omitted when empty: `in:`, `added:`, `changed:`, `torn down:`,
//   `requested:`, `result:`; `aborted` in place of everything after `in:` for a failed
//   tick, with `| ...` lines of this implementation's own wording.
// - **Checks** (`=> expect ..`) are reported as failures beside the transcript.
//
// Steps: `build`, `tick`, `compute`, `compute until idle`, `with E = v`, `push E = v`,
// `send p, v`, `click p`, `send at path, v`, `click at path`, `click element path`,
// `answer p` (the seeded deck), several deliveries joined by commas. Values and
// patterns are read in the transcript's spelling (read.hpp). A `check` block runs the
// lines a program registered for it with `TestWorld::check`; with none, each line's
// left side is read as a value.
//
// A tick that aborts leaves no state to go on from, since this implementation keeps no
// checkpoints: the test's later steps fail (doubt AB in claude/backend.md).

#pragma once

#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <span>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "prism_rtl/pod.hpp"
#include "prism_rtl/world.hpp"

namespace prism_rtl {

/// The most ticks `compute until idle` opens: the Test World's patience.
inline constexpr std::size_t COMPUTE_PATIENCE = 100000;

/// A script's transcript, its failures, and the exit code: 0 when every test passes, 1
/// when a check fails, 2 when the script does not parse.
struct Report {
    std::string transcript;
    std::vector<std::string> failures;
    std::uint8_t exit_code = 0;
};

/// The programs a `test name, @program:` line can name, and the compiled `check` blocks.
class TestWorld {
public:
    using Builder = std::function<Result<std::pair<std::unique_ptr<Program>, TickOut>>(std::span<const Pod>, const ReadyState&)>;
    using Lines = std::function<std::vector<Result<Pod>>()>;

    /// Register a program under its root's name, as a test line writes it after `@`.
    template <class P>
    TestWorld& program(const std::string& name) & {
        programs_[name] = [](std::span<const Pod> args, const ReadyState& ready)
            -> Result<std::pair<std::unique_ptr<Program>, TickOut>> {
            auto built = P::build(args, ready);
            if (!built) return unexpected(built.error());
            auto& [p, out] = built.value();
            return std::pair<std::unique_ptr<Program>, TickOut>(std::move(p), std::move(out));
        };
        return *this;
    }
    template <class P>
    TestWorld program(const std::string& name) && {
        program<P>(name);
        return std::move(*this);
    }
    /// Register a compiled `check` block: each line's value, in order.
    TestWorld& check(const std::string& name, Lines lines) & {
        checks_[name] = std::move(lines);
        return *this;
    }
    TestWorld check(const std::string& name, Lines lines) && {
        check(name, std::move(lines));
        return std::move(*this);
    }

    /// Run a script and print its transcript.
    Report run(std::string_view script) const;
    /// Run a script and compare its transcript with an `.expected` file's, byte for byte.
    Expected<Report, std::string> verify(std::string_view script, std::string_view expected) const;

private:
    std::map<std::string, Builder> programs_;
    std::map<std::string, Lines> checks_;
};

/// xorshift64*, the generator every implementation shares (Conformance ch. 1, "The
/// seeded deck"): the state is the seed, or `0x9E3779B97F4A7C15` for 0.
class XorShift {
public:
    explicit XorShift(std::uint64_t seed) : s_(seed == 0 ? 0x9E3779B97F4A7C15ull : seed) {}
    std::uint64_t next_output();
    /// Clubs, Diamonds, Hearts, Spades, each Two to Ace, shuffled by Fisher-Yates from
    /// the top: for `i` from 51 down to 1, `j` is the next output modulo `i + 1`.
    std::vector<Pod> shuffle();

private:
    std::uint64_t s_;
};

/// An `.expected` file without its leading `# ` comment lines.
std::string strip_comment_header(std::string_view s);
/// Print one tick in the transcript's format.
void print_tick(std::string& out, std::size_t n, std::span<const std::string> inputs, const Result<TickOut>& result);
/// A line-by-line difference, for a failed `verify`.
std::string diff(std::string_view expected, std::string_view got);

}  // namespace prism_rtl
