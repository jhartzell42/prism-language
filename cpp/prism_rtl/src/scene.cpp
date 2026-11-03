// The scene's clickables (scene.hpp).

#include "prism_rtl/scene.hpp"

#include <cmath>
#include <functional>
#include <numbers>

#include "prism_rtl/print.hpp"

namespace prism_rtl {

static double num(const Pod* p) {
    if (!p) return 0.0;
    return p->as_f64().value_or(0.0);
}

/// A point in a group's frame, in its parent's: turned clockwise on the screen by
/// `angle` degrees about the pivot `(x, y)`, then scaled about it; the inverse of the
/// window library's `Untransform`.
static std::pair<double, double> turned(const Pod& t, double px, double py) {
    double cx = num(t.field("x")), cy = num(t.field("y")), rad = num(t.field("angle")) * (std::numbers::pi / 180.0);
    const Pod* s = t.field("scale");
    double scale = s ? num(s) : 1.0;
    double ux = px - cx, uy = py - cy;
    return {cx + scale * (ux * std::cos(rad) - uy * std::sin(rad)), cy + scale * (ux * std::sin(rad) + uy * std::cos(rad))};
}

static void walk(const Pod& p, std::string_view target, std::vector<Pod>& turns,
                 const std::function<void(double, double)>& hit) {
    const std::string* tag = p.tag();
    if (auto items = p.structure().if_array()) {
        for (const Pod& d : *items) walk(d, target, turns, hit);
    } else if (tag && *tag == "Draw.Clickable") {
        const Pod* at = p.field("path");
        const Pod* r = p.field("rect");
        if (!at || !at->structure().if_path() || !r) return;
        if (print_path(*at->structure().if_path()) != target) return;
        double x = num(r->field("x")) + num(r->field("w")) / 2.0, y = num(r->field("y")) + num(r->field("h")) / 2.0;
        for (auto it = turns.rbegin(); it != turns.rend(); ++it) std::tie(x, y) = turned(*it, x, y);
        hit(x, y);
    } else if (tag && *tag == "Draw.Group") {
        const Pod* t = p.field("turn");
        turns.push_back(t ? *t : Pod::unit());
        if (const Pod* d = p.field("draws")) walk(*d, target, turns, hit);
        turns.pop_back();
    } else if (tag && *tag == "Draw.Clip") {
        if (const Pod* d = p.field("draws")) walk(*d, target, turns, hit);
    }
}

std::vector<Clickable> find_clickables(const Document& doc, std::string_view target) {
    std::vector<Clickable> out;
    for (const auto& [path, level] : doc) {
        if (!level.tag() || *level.tag() != "Gui.Scene") continue;
        const Pod* w = level.field("window");
        Pod window = w ? *w : Pod::unit();
        if (const Pod* draw = level.field("draw")) {
            std::vector<Pod> turns;
            walk(*draw, target, turns, [&](double x, double y) { out.push_back(Clickable{path, window, x, y}); });
        }
    }
    return out;
}

}  // namespace prism_rtl
