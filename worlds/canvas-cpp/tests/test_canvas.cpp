// The canvas World's parts that need no display: the ready state, the patch read into
// windows and scenes, the input queue, every draw command painted offscreen at scale 1 and
// 2, and the font. The C++ form of the unit tests in worlds/canvas/src (world.rs,
// render.rs, font.rs), test for test.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <cmath>
#include <cstring>

#include "check.hpp"
#include "prism_world_canvas/canvas.hpp"

using namespace prism_rtl;
using namespace prism_world_canvas;

namespace {

Pod text(const std::string& s) { return Pod::text(Text(s)); }
Path at(std::uint32_t n) { return Path::root().at(n); }

TickOut outputs(Patch patch) {
    TickOut out;
    out.patch = std::move(patch);
    return out;
}

std::pair<CanvasWorld, std::shared_ptr<Shared>> world() {
    auto shared = std::make_shared<Shared>();
    return {CanvasWorld(shared, Font::bundled()), shared};
}

Pod window_el(const std::string& title) {
    return Pod::element("Gui.Window", {{"title", text(title)}, {"size", gui::extent(512.0, 560.0)}});
}

// ---- world.rs ---------------------------------------------------------------------------

TEST(a_window_is_answered_with_its_title_and_metrics_from_the_font) {
    auto [w, _] = world();
    auto ready = w.ready();
    CHECK(ready->first_value(window_el("Chess")) == std::optional<Pod>(gui::window_id("Chess")));
    Pod metrics = Pod::element("Gui.Metrics", {{"text", text("turn")}});
    auto [tw, th] = Font::bundled()->extent(U"turn");
    CHECK(ready->first_value(metrics) == std::optional<Pod>(gui::extent(tw, th)));
    CHECK(!ready->first_value(Pod::element("Gui.Focus", {})));
}

TEST(the_patch_keeps_windows_and_scenes_by_path) {
    auto [w, shared] = world();
    Pod scene = Pod::element("Gui.Scene", {{"window", gui::window_id("Chess")}, {"draw", Pod::array({})}});
    w.perform(outputs(Patch{{{at(1), window_el("Chess")}, {at(3), scene}}, {}, {}}));
    CHECK(shared->windows.at(at(1)) == (WindowPlace{gui::window_id("Chess"), "Chess", 512.0, 560.0}));
    CHECK_EQ(shared->draw_lists(gui::window_id("Chess")).size(), std::size_t(1));
    CHECK(shared->dirty);
    // Retitling keeps the id the first value answered.
    Pod retitled = window_el("Mate");
    w.perform(outputs(Patch{{}, {{at(1), retitled}}, {}}));
    CHECK(shared->windows.at(at(1)).id == gui::window_id("Chess"));
    CHECK_EQ(shared->windows.at(at(1)).title, std::string("Mate"));
    w.perform(outputs(Patch{{}, {}, {{at(1), retitled}, {at(3), Pod::unit()}}}));
    CHECK(shared->windows.empty() && shared->scenes.empty());
}

TEST(inputs_go_one_per_tick_and_resizes_coalesce) {
    auto [w, shared] = world();
    shared->push_input(at(1), gui::resized(600.0, 600.0));
    shared->push_input(at(1), gui::resized(700.0, 650.0));
    shared->push_input(at(1), gui::press(10.0, 20.0));
    shared->push_input(at(1), gui::press(30.0, 40.0));
    std::vector<Pod> got;
    while (auto i = w.next_inputs()) {
        CHECK_EQ(i->answers.size(), std::size_t(1));
        got.push_back(i->answers[0].second);
    }
    CHECK(got == (std::vector<Pod>{gui::resized(700.0, 650.0), gui::press(10.0, 20.0), gui::press(30.0, 40.0)}));
}

TEST(input_for_a_window_that_left_goes_nowhere) {
    auto [w, shared] = world();
    w.perform(outputs(Patch{{{at(1), window_el("Chess")}}, {}, {}}));
    shared->push_input(at(1), gui::press(1.0, 2.0));
    w.perform(outputs(Patch{{}, {}, {{at(1), window_el("Chess")}}}));
    CHECK(!w.next_inputs());
}

// ---- render.rs --------------------------------------------------------------------------

Pod el(const std::string& tag, Fields fields) { return Pod::element(tag, std::move(fields)); }

/// A red `Draw.Rect`.
Pod red_rect(double x, double y, double w, double h) { return el("Draw.Rect", {{"rect", gui::rect(x, y, w, h)}, {"paint", Pod::name("Red")}}); }

struct Canvas {
    cairo_surface_t* s;
    explicit Canvas(cairo_surface_t* s) : s(s) {}
    Canvas(const Canvas&) = delete;
    ~Canvas() { cairo_surface_destroy(s); }
    Rgb rgb(int x, int y) const {
        auto* data = cairo_image_surface_get_data(s);
        int stride = cairo_image_surface_get_stride(s);
        std::uint32_t px;
        std::memcpy(&px, data + y * stride + x * 4, 4);
        return {std::uint8_t(px >> 16), std::uint8_t(px >> 8), std::uint8_t(px)};
    }
};

/// Paint one draw list at `scale` onto a 100 by 100 logical canvas.
std::unique_ptr<Canvas> paint(float scale, std::vector<Pod> draws) {
    int size = int(100 * scale);
    auto c = std::make_unique<Canvas>(cairo_image_surface_create(CAIRO_FORMAT_RGB24, size, size));
    Pod list = Pod::array(std::move(draws));
    Renderer(Font::bundled()).render(c->s, scale, {&list});
    return c;
}

constexpr Rgb RED = {200, 40, 40};

TEST(a_rect_is_painted_in_logical_pixels) {
    auto p = paint(2.0f, {red_rect(10, 10, 20, 20)});
    CHECK(p->rgb(40, 40) == RED);         // inside, at twice the scale
    CHECK(p->rgb(70, 70) == BACKGROUND);  // outside
}

TEST(a_clip_cuts_and_a_group_turns_about_its_pivot) {
    Pod clipped = el("Draw.Clip", {{"rect", gui::rect(0, 0, 15, 100)}, {"draws", Pod::array({red_rect(10, 10, 20, 20)})}});
    auto p = paint(1.0f, {clipped});
    CHECK(p->rgb(12, 20) == RED);
    CHECK(p->rgb(20, 20) == BACKGROUND);
    // A bar right of the pivot (50, 50), turned 90 degrees clockwise, hangs below it.
    Pod turn = Pod::record("Turn", {{"x", Pod::floating(50)}, {"y", Pod::floating(50)}, {"angle", Pod::floating(90)}, {"scale", Pod::floating(1)}});
    Pod group = el("Draw.Group", {{"turn", turn}, {"draws", Pod::array({red_rect(60, 48, 30, 4)})}});
    auto q = paint(1.0f, {group});
    CHECK(q->rgb(50, 75) == RED);
    CHECK(q->rgb(75, 50) == BACKGROUND);
}

TEST(text_is_inked_below_its_point_and_clickables_draw_nothing) {
    Pod t = el("Draw.Text", {{"at", gui::point(4, 4)}, {"text", text("White's turn")}});
    Pod click = el("Draw.Clickable", {{"rect", gui::rect(0, 50, 100, 50)}, {"path", Pod::unit()}});
    auto p = paint(1.0f, {t, click});
    auto dark = [&](int y0, int y1) {
        for (int x = 0; x < 100; ++x)
            for (int y = y0; y < y1; ++y)
                if (std::get<0>(p->rgb(x, y)) < 128) return true;
        return false;
    };
    CHECK(dark(4, 18));                      // the line is inked
    CHECK(!dark(0, 4) && !dark(20, 100));    // and nothing else is
}

TEST(an_image_is_scaled_to_its_rect_and_a_missing_one_draws_nothing) {
    std::string src = std::string(PRISM_REPO) + "/examples/chess/pieces/kl.png";
    Pod king = el("Draw.Image", {{"rect", gui::rect(0, 0, 100, 100)}, {"src", text(src)}});
    auto p = paint(1.0f, {king});
    bool shows = false;
    for (int x = 0; x < 100; ++x)
        for (int y = 0; y < 100; ++y) shows |= p->rgb(x, y) != BACKGROUND;
    CHECK(shows);  // the king shows
    Pod missing = el("Draw.Image", {{"rect", gui::rect(0, 0, 100, 100)}, {"src", text("no/such.png")}});
    auto q = paint(1.0f, {missing});
    bool blank = true;
    for (int x = 0; x < 100; ++x)
        for (int y = 0; y < 100; ++y) blank &= q->rgb(x, y) == BACKGROUND;
    CHECK(blank);
}

TEST(paints_follow_the_gloss_palette) {
    CHECK(paint_rgb(Pod::name("Paper")) == (Rgb{255, 255, 255}));
    CHECK(paint_rgb(Pod::name("Ink")) == (Rgb{0, 0, 0}));
    Pod rgb = Pod::choice("Rgb", {Pod::floating(1.0), Pod::floating(0.5), Pod::floating(0.0)});
    CHECK(paint_rgb(rgb) == (Rgb{255, 128, 0}));
    CHECK(paint_rgb(Pod::name("Mauve")) == GREY_UNKNOWN);
}

// ---- font.rs ----------------------------------------------------------------------------

TEST(a_line_is_fourteen_high_like_the_test_worlds) {
    auto f = Font::bundled();
    CHECK_EQ(f->extent(U"White's").second, 14.0);
    CHECK_EQ(f->extent(U"").first, 0.0);
}

TEST(widths_are_whole_and_grow_with_the_text) {
    auto f = Font::bundled();
    double one = f->extent(U"turn").first;
    double two = f->extent(U"turn turn").first;
    CHECK_EQ(one, std::round(one));
    CHECK(one > 0.0 && two > one * 2.0);
}

/// The Rust World's answers, from fontdue over the same file: a layout comes out the
/// same under both.
TEST(widths_are_the_rust_worlds) {
    auto f = Font::bundled();
    CHECK_EQ(f->extent(U"White's").first, 45.0);
    CHECK_EQ(f->extent(U"Black's turn").first, 71.0);
    CHECK_EQ(f->extent(U"checkmated").first, 74.0);
    CHECK_EQ(f->extent(U"Create 1,000 rows").first, 111.0);
    CHECK_EQ(f->extent(U"pretty red table").first, 95.0);
}

}  // namespace
