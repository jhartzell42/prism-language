// The canvas World's parts that need no display: the ready state, the patch read into
// windows and scenes, the input queue, every draw command painted offscreen at scale 1 and
// 2, and the font. The C++ form of the unit tests in worlds/canvas/rust/src (world.rs,
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

std::pair<CanvasWorld, prism_rtl::Rc<Shared>> world() {
    auto shared = prism_rtl::make_rc<Shared>();
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

// ---- damage.rs --------------------------------------------------------------------------

constexpr double DW = 300, DH = 240;

Pod d_rect(double x, double y, double w, double h, const char* paint) {
    return el("Draw.Rect", {{"rect", gui::rect(x, y, w, h)}, {"paint", Pod::name(paint)}});
}
Pod d_text(double x, double y, const std::string& t) { return el("Draw.Text", {{"at", gui::point(x, y)}, {"text", text(t)}}); }

/// damage.rs's xorshift, so the scenes are the same every run.
struct Rng {
    std::uint64_t s;
    std::uint64_t next() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        return s;
    }
    std::size_t below(std::size_t n) { return std::size_t(next() % n); }
};

/// damage.rs's table: a header button, rows of a label, a clickable, a remove button and
/// its `x`, some at fractional positions, and extras drawn over them.
struct Table {
    std::vector<std::string> labels;
    double top = 40;
    const char* paint = "Grey";
    std::vector<Pod> extras;
    Pod scene() const {
        std::vector<Pod> d = {d_rect(2, 2, 120, 26, "Grey"), d_text(8, 8, "Create 1,000 rows")};
        for (std::size_t i = 0; i < labels.size(); ++i) {
            double y = top + 22.5 * double(i);
            if (y > DH + 8) break;
            d.push_back(d_text(4, y + 4, labels[i]));
            d.push_back(el("Draw.Clickable", {{"rect", gui::rect(0, y, 200, 22)}, {"path", Pod::unit()}}));
            d.push_back(d_rect(220.25, y + 1, 20.5, 20, paint));
            d.push_back(d_text(226, y + 4, "x"));
        }
        for (const Pod& e : extras) d.push_back(e);
        return Pod::array(std::move(d));
    }
};

std::string word(Rng& rng) {
    static const char* words[] = {"pretty", "large", "big", "small", "tall", "red", "yellow", "table", "chair", "▶ brown"};
    return words[rng.below(10)];
}

Pod extra(Rng& rng) {
    switch (rng.below(5)) {
        case 0: return d_rect(double(rng.below(250)) + 0.3, double(rng.below(200)) + 0.7, 40.4, 13.2, "Blue");
        case 1: {
            Pod r = gui::rect(double(rng.below(200)) + 0.5, double(rng.below(150)), 50, 30.25);
            return el("Draw.Clip", {{"rect", r}, {"draws", Pod::array({d_rect(0, 0, 300, 240, "Yellow"), d_text(30, 30, "clipped text")})}});
        }
        case 2: {
            Pod turn = Pod::record("Turn", {{"x", Pod::floating(150)}, {"y", Pod::floating(120)},
                                            {"angle", Pod::floating(double(rng.below(360)) + 0.5)}, {"scale", Pod::floating(0.75)}});
            return el("Draw.Group", {{"turn", turn}, {"draws", Pod::array({d_rect(140, 100, 60, 8, "Green"), d_text(150, 110, "turned")})}});
        }
        case 3: {
            std::string src = std::string(PRISM_REPO) + "/examples/chess/pieces/kl.png";
            Pod r = gui::rect(double(rng.below(250)) + 0.25, double(rng.below(180)), 37.5, 37.5);
            return el("Draw.Image", {{"rect", r}, {"src", text(src)}});
        }
        default: return d_text(double(rng.below(250)) + 0.5, double(rng.below(220)) + 0.25, "over");
    }
}

/// One change to the table, as damage.rs's `change`.
void change(Table& t, Rng& rng) {
    std::size_t n = t.labels.size();
    switch (rng.below(10)) {
        case 0:
            if (n > 0) {
                std::size_t i = rng.below(n);
                t.labels[i] = "▶ " + t.labels[i];
            }
            break;
        case 1:
            if (n > 1) {
                std::size_t i = rng.below(n), j = rng.below(n);
                std::swap(t.labels[i], t.labels[j]);
            }
            break;
        case 2:
            if (n > 0) t.labels.erase(t.labels.begin() + std::ptrdiff_t(rng.below(n)));
            break;
        case 3: {
            std::size_t i = rng.below(n + 1);
            t.labels.insert(t.labels.begin() + std::ptrdiff_t(i), word(rng));
            break;
        }
        case 4: t.extras.push_back(extra(rng)); break;
        case 5:
            if (!t.extras.empty()) t.extras.erase(t.extras.begin() + std::ptrdiff_t(rng.below(t.extras.size())));
            break;
        case 6:
            if (t.extras.size() > 1) {
                std::size_t k = t.extras.size();
                std::size_t i = rng.below(k), j = rng.below(k);
                std::swap(t.extras[i], t.extras[j]);
            }
            break;
        case 7: t.top = t.top == 40 ? 40.5 : 40; break;
        case 8: t.paint = std::string_view(t.paint) == "Grey" ? "Red" : "Grey"; break;
        case 9:
            if (rng.below(4) == 0) t.labels.clear();
            break;
    }
}

/// An RGB24 surface's pixels, the unused top byte dropped.
std::vector<std::uint32_t> pixels_of(cairo_surface_t* s) {
    cairo_surface_flush(s);
    int w = cairo_image_surface_get_width(s), h = cairo_image_surface_get_height(s);
    int stride = cairo_image_surface_get_stride(s);
    const unsigned char* data = cairo_image_surface_get_data(s);
    std::vector<std::uint32_t> out(std::size_t(w) * std::size_t(h));
    for (int y = 0; y < h; ++y)
        for (int x = 0; x < w; ++x) {
            std::uint32_t px;
            std::memcpy(&px, data + std::size_t(y) * std::size_t(stride) + std::size_t(x) * 4, 4);
            out[std::size_t(y) * std::size_t(w) + std::size_t(x)] = px & 0xffffff;
        }
    return out;
}

std::pair<int, int> size_at(float scale) { return {int(std::lround(float(DW) * scale)), int(std::lround(float(DH) * scale))}; }

/// Damage painting equals a full repaint of the last scene, pixel for pixel, after every
/// step of a long sequence of changes, at three scales.
TEST(painting_the_damage_equals_a_full_repaint) {
    for (float scale : {1.0f, 2.0f, 1.5f}) {
        std::uint32_t bits;
        std::memcpy(&bits, &scale, 4);
        Rng rng{0x9e3779b97f4a7c15ull ^ bits};
        Renderer renderer(Font::bundled()), reference(Font::bundled());
        auto [w, h] = size_at(scale);
        Canvas frame(cairo_image_surface_create(CAIRO_FORMAT_RGB24, w, h));
        Painter painter;
        Table t;
        for (int i = 0; i < 14; ++i) t.labels.push_back(word(rng));
        int partial = 0;
        for (int step = 0; step < 400; ++step) {
            Pod scene = t.scene();
            Pod second = Pod::array({d_rect(10, 200, 30, 10, "Felt")});
            std::vector<const Pod*> lists = {&scene};
            if (step % 7 == 3) lists.push_back(&second);
            Damage damage = painter.paint(renderer, frame.s, scale, lists);
            if (!damage.full) ++partial;
            Canvas want(cairo_image_surface_create(CAIRO_FORMAT_RGB24, w, h));
            reference.render(want.s, scale, lists);
            auto got = pixels_of(frame.s), expected = pixels_of(want.s);
            if (got != expected) {
                std::size_t at = 0;
                while (got[at] == expected[at]) ++at;
                ::check::fail(__FILE__, __LINE__,
                              "scale " + std::to_string(scale) + ", step " + std::to_string(step) +
                                  ": the frame differs from a full repaint at (" + std::to_string(at % std::size_t(w)) + ", " +
                                  std::to_string(at / std::size_t(w)) + ")");
                return;
            }
            change(t, rng);
        }
        CHECK(partial > 300);
    }
}

/// A small change damages a small part of the frame; the same scene damages none; the
/// first frame and a new size repaint the whole.
TEST(a_small_change_damages_a_small_box) {
    float scale = 2;
    Renderer renderer(Font::bundled());
    Painter painter;
    auto [w, h] = size_at(scale);
    Canvas frame(cairo_image_surface_create(CAIRO_FORMAT_RGB24, w, h));
    Rng rng{7};
    Table t;
    for (int i = 0; i < 30; ++i) t.labels.push_back(word(rng));
    Pod s1 = t.scene();
    CHECK(painter.paint(renderer, frame.s, scale, {&s1}) == (Damage{true, {}}));
    CHECK(painter.paint(renderer, frame.s, scale, {&s1}) == (Damage{false, {}}));
    t.labels[2] = "▶ " + t.labels[2];
    Pod s2 = t.scene();
    Damage d = painter.paint(renderer, frame.s, scale, {&s2});
    CHECK(!d.full && !d.rects.empty());
    std::uint64_t area = 0;
    for (const IRect& r : d.rects) area += r.area();
    CHECK(area * 50 < std::uint64_t(w) * std::uint64_t(h));
    painter.forget();
    CHECK(painter.paint(renderer, frame.s, scale, {&s2}) == (Damage{true, {}}));
    Canvas bigger(cairo_image_surface_create(CAIRO_FORMAT_RGB24, w + 10, h));
    CHECK(painter.paint(renderer, bigger.s, scale, {&s2}) == (Damage{true, {}}));
}

/// A command at a non-finite place cannot be bounded: the whole frame.
TEST(an_unbounded_command_repaints_the_whole_frame) {
    Renderer renderer(Font::bundled());
    Painter painter;
    Canvas frame(cairo_image_surface_create(CAIRO_FORMAT_RGB24, 100, 100));
    Pod first = Pod::array({d_rect(1, 1, 5, 5, "Red")});
    painter.paint(renderer, frame.s, 1, {&first});
    Pod turn = Pod::record("Turn", {{"x", Pod::floating(std::nan(""))}, {"y", Pod::floating(0)}, {"angle", Pod::floating(0)}, {"scale", Pod::floating(1)}});
    Pod group = el("Draw.Group", {{"turn", turn}, {"draws", Pod::array({d_rect(1, 1, 5, 5, "Red")})}});
    Pod second = Pod::array({d_rect(1, 1, 5, 5, "Red"), group});
    CHECK(painter.paint(renderer, frame.s, 1, {&second}) == (Damage{true, {}}));
    // inside a clip too
    Pod clip = el("Draw.Clip", {{"rect", gui::rect(0, 0, 1e30, 50)}, {"draws", Pod::array({d_text(std::nan(""), 3, "a")})}});
    Pod third = Pod::array({d_rect(1, 1, 5, 5, "Red"), clip});
    CHECK(painter.paint(renderer, frame.s, 1, {&third}) == (Damage{true, {}}));
}

}  // namespace
