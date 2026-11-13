// The canvas World as the tick loop sees it (world.hpp), and the prelude's wrapper.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_world_canvas/world.hpp"

#include <algorithm>
#include <cstdio>

#include "prism_world_canvas/font.hpp"
#include "prism_world_canvas/render.hpp"

namespace prism_world_canvas {

using prism_rtl::TickIn;
using prism_rtl::TickOut;

namespace gui {

Pod window_id(const std::string& title) { return Pod::choice("WindowId", {Pod::text(prism_rtl::Text(title))}); }
Pod point(double x, double y) { return Pod::record("Point", {{"x", Pod::floating(x)}, {"y", Pod::floating(y)}}); }
Pod extent(double w, double h) { return Pod::record("Extent", {{"w", Pod::floating(w)}, {"h", Pod::floating(h)}}); }
Pod rect(double x, double y, double w, double h) {
    return Pod::record("Rect", {{"x", Pod::floating(x)}, {"y", Pod::floating(y)}, {"w", Pod::floating(w)}, {"h", Pod::floating(h)}});
}
Pod press(double x, double y) { return Pod::choice("Press", {point(x, y)}); }
Pod resized(double w, double h) { return Pod::choice("Resized", {extent(w, h)}); }
Pod pressed(Pod key) { return Pod::choice("Pressed", {std::move(key)}); }

}  // namespace gui

namespace {

bool is_resize(const Pod& v) {
    auto c = v.structure().if_choice();
    return c && c->ctor == "Resized";
}

std::optional<std::string> text_field(const Pod& e, std::string_view name) {
    const Pod* f = e.field(name);
    if (!f) return std::nullopt;
    const prism_rtl::Text* t = f->as_text();
    if (!t) return std::nullopt;
    return t->to_string();
}

}  // namespace

void Shared::push_input(Path window, Pod input) {
    if (!queue.empty() && is_resize(input)) {
        auto& [p, last] = queue.back();
        if (p == window && is_resize(last)) {
            last = std::move(input);
            return;
        }
    }
    queue.emplace_back(std::move(window), std::move(input));
}

std::vector<const Pod*> Shared::draw_lists(const Pod& id) const {
    std::vector<const Pod*> out;
    for (const auto& [_, s] : scenes)
        if (s.window == id) out.push_back(&s.draw);
    return out;
}

std::optional<Pod> CanvasReady::first_value(const Pod& element) const {
    const std::string* tag = element.tag();
    if (!tag) return std::nullopt;
    if (*tag == "Gui.Window") {
        auto title = text_field(element, "title");
        if (!title) return std::nullopt;
        return gui::window_id(*title);
    }
    if (*tag == "Gui.Metrics") {
        const Pod* f = element.field("text");
        const prism_rtl::Text* t = f ? f->as_text() : nullptr;
        if (!t) return std::nullopt;
        auto [w, h] = font_->extent(t->chars());
        return gui::extent(w, h);
    }
    return std::nullopt;
}

CanvasWorld::CanvasWorld(prism_rtl::Rc<Shared> shared, prism_rtl::Rc<const Font> font)
    : shared_(std::move(shared)), font_(font), ready_(prism_rtl::make_rc<CanvasReady>(std::move(font))) {}

prism_rtl::Rc<const prism_rtl::ReadyState> CanvasWorld::ready() const { return ready_; }

void CanvasWorld::not_answered(const std::string& tag) {
    if (tag.rfind("Prelude.", 0) == 0 || !warned_.insert(tag).second) return;
    std::string tags;
    for (const auto& t : TAGS) tags += (tags.empty() ? "" : ", ") + t;
    std::fprintf(stderr, "prism canvas: this World does not answer %s; it answers %s (a Ui element needs a @Window around it)\n",
                 tag.c_str(), tags.c_str());
}

void CanvasWorld::place(const Path& path, const Pod& element, bool added) {
    const std::string* tag = element.tag();
    if (!tag) return;
    Shared& shared = *shared_;
    if (*tag == "Gui.Window") {
        std::string title = text_field(element, "title").value_or("");
        auto it = shared.windows.find(path);
        if (it != shared.windows.end() && !added) {
            // `size` is identity and the id was answered once: only the title moves.
            it->second.title = title;
        } else {
            WindowPlace w;
            if (const Pod* size = element.field("size")) {
                w.w = number(size->field("w"));
                w.h = number(size->field("h"));
            }
            w.id = gui::window_id(title);
            w.title = title;
            shared.windows[path] = std::move(w);
        }
        shared.dirty = true;
    } else if (*tag == "Gui.Scene") {
        const Pod* window = element.field("window");
        const Pod* draw = element.field("draw");
        shared.scenes[path] = ScenePlace{window ? *window : Pod::unit(), draw ? *draw : Pod::array({})};
        shared.dirty = true;
    } else if (*tag == "Gui.Metrics" || *tag == "Gui.Focus") {
    } else {
        not_answered(*tag);
    }
}

void CanvasWorld::perform(const TickOut& out) {
    Shared& shared = *shared_;
    for (const auto& [path, _] : out.patch.removed) {
        bool had_window = shared.windows.erase(path) > 0;
        bool had_scene = shared.scenes.erase(path) > 0;
        shared.dirty |= had_window || had_scene;
        // Input queued for a window that left goes nowhere (a late result).
        if (had_window)
            shared.queue.erase(std::remove_if(shared.queue.begin(), shared.queue.end(),
                                              [&](const auto& q) { return q.first == path; }),
                               shared.queue.end());
    }
    for (const auto& [path, e] : out.patch.added) place(path, e, true);
    for (const auto& [path, e] : out.patch.changed) place(path, e, false);
}

std::optional<TickIn> CanvasWorld::next_inputs() {
    Shared& shared = *shared_;
    if (shared.queue.empty()) return std::nullopt;
    auto [path, input] = std::move(shared.queue.front());
    shared.queue.pop_front();
    return TickIn::none().answer(std::move(path), std::move(input));
}

// ---- The prelude's wrapper ----------------------------------------------------------------

UnwrapWorld::UnwrapWorld(std::unique_ptr<prism_rtl::World> inner) : inner_(std::move(inner)) {}

void UnwrapWorld::perform(const TickOut& out) {
    prelude_.track(out.patch);
    std::vector<prism_rtl::Request> theirs;
    for (const auto& r : out.requests) {
        if (prelude_.is_poisoned(r.path)) continue;
        const Pod* element = prism_world_cli::element_of(doc_, out, r.path);
        const std::string* tag = element ? element->tag() : nullptr;
        if (tag && *tag == "Prelude.Unwrap")
            prelude_.unwrap(r.path, r.value);
        else
            theirs.push_back(r);
    }
    prelude_.report_gave_up();
    doc_.apply(out.patch);
    if (theirs.size() == out.requests.size()) {
        inner_->perform(out);
    } else {
        TickOut mine = out;
        mine.requests = std::move(theirs);
        inner_->perform(mine);
    }
}

std::optional<TickIn> UnwrapWorld::next_inputs() {
    auto mine = prelude_.take_outputs();
    std::optional<TickIn> input = inner_->next_inputs();
    if (!input) {
        if (mine.empty()) return std::nullopt;
        input = TickIn::none();
    }
    for (auto& a : mine) input->answers.push_back(std::move(a));
    prelude_.filter(*input, doc_);
    return input;
}

std::optional<std::uint8_t> UnwrapWorld::exit_code() const {
    if (auto e = prelude_.exit()) return e;
    return inner_->exit_code();
}

}  // namespace prism_world_canvas
