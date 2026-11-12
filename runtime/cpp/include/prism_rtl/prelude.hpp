// The prelude's two tags, answered by whatever World a program runs under (Semantics
// ch. 10, "Exceptions: a widget giving up"): the port of runtime/rust/src/world/prelude.rs.
//
// `Prelude` is the policy alone, which the CLI World and the Test World run inline; the
// canvas World's `UnwrapWorld` puts it around a World that answers no prelude tag itself.
// The policy prints nothing: a World that ends on a poisoned root asks `take_gave_up`
// for what to say.

#pragma once

#include <cstdint>
#include <optional>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "prism_rtl/doc.hpp"
#include "prism_rtl/pod.hpp"
#include "prism_rtl/world.hpp"

namespace prism_rtl {

/// The prelude's part of a World: `Prelude.Unwrap` and `Prelude.Exceptions` (Semantics
/// ch. 10). An unwrap of a value answers it; of an `Error` poisons the unwrap's subtree
/// and reports the exception to the nearest supervisor outside it, or gives up the
/// program.
class Prelude {
public:
    bool is_poisoned(const Path& p) const;
    void track(const Patch& patch);
    void unwrap(const Path& path, const Pod& value);
    std::vector<std::pair<Path, Pod>> take_answers();
    /// Whether the next tick is owed an answer of this policy's.
    bool owes() const { return !answers_.empty(); }
    /// Hand back answers taken for a tick that did not run.
    void give_back(std::vector<std::pair<Path, Pod>> answers);
    /// One line for each poisoning of the root since the last call: what the CLI World's
    /// policy prints on standard error (Semantics ch. 9).
    std::vector<std::string> take_gave_up();
    /// Print `take_gave_up` on standard error.
    void report_gave_up();
    void filter(TickIn& input, const Document& doc) const;
    std::optional<std::uint8_t> exit() const { return exit_; }

private:
    std::vector<Path> poisoned_;
    std::set<Path> exceptions_;
    std::vector<std::pair<Path, Pod>> answers_;
    std::vector<std::string> gave_up_;
    std::optional<std::uint8_t> exit_;
};

/// The subtree an unwrap stands in: up to its last generation or key segment.
Path subtree_of(const Path& p);
/// The element at a path in the document, or added by this tick's patch.
const Pod* element_of(const Document& doc, const TickOut& out, const Path& path);

}  // namespace prism_rtl
