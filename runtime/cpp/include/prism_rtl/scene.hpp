// `Draw.Clickable(rect, path)` in a `Gui.Scene` (decisions.md D9): the region `rect` of
// a window is the element at `path` of the program's own layout. Hit testing stays in
// Prism; the scene names it so the Test World can turn `click element path` into a
// `Press` on the window. The canvas World ignores the tag.

#pragma once

#include <string_view>
#include <vector>

#include "prism_rtl/doc.hpp"
#include "prism_rtl/pod.hpp"

namespace prism_rtl {

/// A `Draw.Clickable` found in a scene: the `Gui.Scene` place that drew it, the window
/// the scene is for, and its rectangle's centre in the window's coordinates.
struct Clickable {
    Path scene;
    Pod window;
    double x;
    double y;
};

/// Every `Draw.Clickable` whose path prints as `target` in the document's scenes. Inside
/// a `Draw.Group` a rectangle is in the group's frame, so the centre is taken out through
/// each enclosing turn, innermost first; a `Draw.Clip` moves nothing.
std::vector<Clickable> find_clickables(const Document& doc, std::string_view target);

}  // namespace prism_rtl
