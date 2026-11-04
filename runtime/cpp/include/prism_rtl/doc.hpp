// The document and the patch diff (Semantics ch. 8, "The patch"; Graph ch. 4, "The
// patch pass and the request list").
//
// A program's patch pass writes `Edit`s: a place built, torn down, or possibly
// changed. `Document::diff` turns them into the tick's `Patch` against the document
// last sent, so `Changed` goes out only for a place whose `Level` really moved, a torn-down
// place comes with its element as it stood after the previous tick, and every list is in
// path order. A World keeps its own copy of the document with `Document::apply`.

#pragma once

#include <map>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "prism_rtl/pod.hpp"

namespace prism_rtl {

/// What the patch pass says about one place.
struct Edit {
    enum class Kind : std::uint8_t {
        Added,
        /// The place's `Level` after the tick; the diff drops it if nothing moved.
        Changed,
        Removed,
    };
    Kind kind;
    Path path;
    /// Unit for `Removed`.
    Pod level;

    static Edit added(Path p, Pod level) { return Edit{Kind::Added, std::move(p), std::move(level)}; }
    static Edit changed(Path p, Pod level) { return Edit{Kind::Changed, std::move(p), std::move(level)}; }
    static Edit removed(Path p) { return Edit{Kind::Removed, std::move(p), Pod()}; }
    bool operator==(const Edit&) const = default;
};

/// One tick's change to the document, each list in path order.
struct Patch {
    std::vector<std::pair<Path, Pod>> added;
    std::vector<std::pair<Path, Pod>> changed;
    /// With the element as it stood after the previous tick.
    std::vector<std::pair<Path, Pod>> removed;

    bool is_empty() const { return added.empty() && changed.empty() && removed.empty(); }
    bool operator==(const Patch&) const = default;
};

/// A request that fired: a place's path, the request field's name when its tag has more
/// than one, and the field's `Edge` value.
struct Request {
    Path path;
    std::optional<std::string> field;
    Pod value;

    /// The transcript's `requested:` line: `path: value`, or `path: field=value`.
    std::string to_string() const;
    bool operator==(const Request&) const = default;
};

/// Each place's `Level`, by path.
class Document {
public:
    using const_iterator = std::map<Path, Pod>::const_iterator;

    /// The tick's patch from the patch pass's edits, applied to this document. A path
    /// torn down and built again in one tick (a `for` key removed and added back) is two
    /// widgets: `Removed` and `Added`, never `Changed`.
    Patch diff(std::vector<Edit> edits);
    /// A World's side: the document after a patch.
    void apply(const Patch& patch);

    const Pod* get(const Path& path) const;
    const_iterator begin() const { return m_.begin(); }
    const_iterator end() const { return m_.end(); }
    std::size_t len() const { return m_.size(); }
    bool is_empty() const { return m_.empty(); }
    /// Every element in the document at any depth.
    std::vector<const Pod*> elements() const;
    bool operator==(const Document&) const = default;

private:
    std::map<Path, Pod> m_;
};

}  // namespace prism_rtl
