// The values: bignums (against Python's integers), Float spelling (against Rust's `{}`),
// Text, the spellings' round trip, typed values as pods, patterns, the scene's
// clickables, and a forwarding host (runtime/rust/tests/runtime.rs, num.rs and
// collections.rs's tests, and more).

#include <bit>
#include <cstring>
#include <random>

#include "check.hpp"
#include "prism_rtl/prism_rtl.hpp"

using namespace prism_rtl;

// ---- Numbers ----------------------------------------------------------------------------

TEST(small_values_overflow_into_bignums_and_back) {
    Int max = Int(INT64_MAX);
    Int big = max + Int(1);
    CHECK(!big.to_i64().has_value());
    CHECK_EQ(big.to_string(), std::string("9223372036854775808"));
    CHECK((big - Int(1)).to_i64() == std::optional<std::int64_t>(INT64_MAX));
    CHECK((-big).to_i64() == std::optional<std::int64_t>(INT64_MIN));
    CHECK(big > max && -(big + big) < Int(INT64_MIN));
}

TEST(division_floors_and_zero_is_nil) {
    auto q = [](std::int64_t a, std::int64_t b) {
        auto r = Int(a).div(Int(b));
        return r ? r->to_string() : std::string("nil");
    };
    CHECK_EQ(q(7, 2), std::string("3"));
    CHECK_EQ(q(-7, 2), std::string("-4"));
    CHECK_EQ(q(7, -2), std::string("-4"));
    CHECK_EQ(q(-7, -2), std::string("3"));
    CHECK_EQ(q(6, -2), std::string("-3"));
    CHECK_EQ(q(INT64_MIN, -1), std::string("9223372036854775808"));
    CHECK_EQ(q(1, 0), std::string("nil"));
    CHECK_EQ(Int(-7).modulo(Int(2)).to_string(), std::string("1"));
    CHECK_EQ(Int(5).modulo(Int(0)).to_string(), std::string("5"));
    CHECK_EQ(Int(INT64_MIN).modulo(Int(-1)).to_string(), std::string("0"));
}

TEST(nat_subtracts_only_with_a_question) {
    Nat two(2), three(3);
    CHECK(three.checked_sub(two) == std::optional<Nat>(Nat(1)));
    CHECK(!two.checked_sub(three).has_value());
    CHECK_EQ((two * three).to_string(), std::string("6"));
    CHECK(!Nat::from_int(Int(-1)).has_value());
    CHECK(Nat(7).div(Nat(2)) == std::optional<Nat>(Nat(3)));
    CHECK(!Nat(7).div(Nat(0)).has_value());
}

namespace {
struct Vector {
    const char *a, *b, *sum, *diff, *prod, *quot, *rem;
};
const Vector VECTORS[] = {
#include "bignum_vectors.inc"
};
Int parse(const char* s) { return *Int::parse(s); }
}  // namespace

TEST(bignums_agree_with_pythons_integers) {
    int checked = 0;
    for (const Vector& v : VECTORS) {
        Int a = parse(v.a), b = parse(v.b);
        CHECK_EQ(a.to_string(), std::string(v.a));
        CHECK_EQ((a + b).to_string(), std::string(v.sum));
        CHECK_EQ((a - b).to_string(), std::string(v.diff));
        CHECK_EQ((a * b).to_string(), std::string(v.prod));
        CHECK_EQ(a.div(b)->to_string(), std::string(v.quot));
        CHECK_EQ(a.modulo(b).to_string(), std::string(v.rem));
        // Canonical: a value that fits is small, and order agrees with the difference.
        CHECK_EQ((a + b) == parse(v.sum), true);
        CHECK_EQ((a < b), (parse(v.diff).is_negative()));
        ++checked;
    }
    CHECK_EQ(checked, 400);
}

TEST(bignums_against_int128_on_random_small_operands) {
    std::mt19937_64 rng(5);
    for (int i = 0; i < 20000; ++i) {
        int shift = int(rng() % 64);
        std::int64_t x = std::int64_t(rng()) >> shift, y = std::int64_t(rng()) >> int(rng() % 64);
        __int128 a = x, b = y;
        auto s128 = [](__int128 n) {
            bool neg = n < 0;
            unsigned __int128 m = neg ? ~(unsigned __int128)n + 1 : (unsigned __int128)n;
            std::string s;
            do {
                s.insert(s.begin(), char('0' + int(m % 10)));
                m /= 10;
            } while (m);
            return neg ? "-" + s : s;
        };
        CHECK_EQ((Int(x) + Int(y)).to_string(), s128(a + b));
        CHECK_EQ((Int(x) - Int(y)).to_string(), s128(a - b));
        CHECK_EQ((Int(x) * Int(y)).to_string(), s128(a * b));
        // The product's bignum divided back.
        if (y != 0) {
            Int p = Int(x) * Int(y);
            CHECK_EQ(p.div(Int(y))->to_string(), s128(a));
            CHECK_EQ(p.modulo(Int(y)).to_string(), std::string("0"));
            __int128 q = a / b, r = a % b;
            if (r != 0 && ((r < 0) != (b < 0))) {
                --q;
                r += b;
            }
            CHECK_EQ(Int(x).div(Int(y))->to_string(), s128(q));
            CHECK_EQ(Int(x).modulo(Int(y)).to_string(), s128(r));
        }
    }
}

TEST(bignum_identities_on_large_random_operands) {
    std::mt19937_64 rng(11);
    auto random_int = [&] {
        std::string s = (rng() % 3 == 0) ? "-" : "";
        std::size_t n = 1 + rng() % 120;
        s += char('1' + rng() % 9);
        for (std::size_t i = 1; i < n; ++i) s += char('0' + rng() % 10);
        return s;
    };
    for (int i = 0; i < 2000; ++i) {
        std::string sa = random_int(), sb = random_int(), sc = random_int();
        Int a = parse(sa.c_str()), b = parse(sb.c_str()), c = parse(sc.c_str());
        CHECK_EQ(a.to_string(), sa);
        CHECK((a + b) - b == a);
        CHECK(a * (b + c) == a * b + a * c);
        Int q = *a.div(b), r = a.modulo(b);
        CHECK(q * b + r == a);
        // The remainder has the divisor's sign and is smaller than it.
        CHECK(r == Int(0) || r.is_negative() == b.is_negative());
        CHECK((r.is_negative() ? -r : r) < (b.is_negative() ? -b : b));
        CHECK(*(a * b).div(b) == a);
    }
}

TEST(bignum_known_values) {
    Int f(1);
    for (int i = 1; i <= 30; ++i) f = f * Int(i);
    CHECK_EQ(f.to_string(), std::string("265252859812191058636308480000000"));
    Int p(1);
    for (int i = 0; i < 200; ++i) p = p * Int(2);
    CHECK_EQ(p.to_string(), std::string("1606938044258990275541962092341162602522202993782792835301376"));
    CHECK_EQ(p.div(p - Int(1))->to_string(), std::string("1"));
    CHECK_EQ(p.modulo(p - Int(1)).to_string(), std::string("1"));
    CHECK(!Int::parse("12a").has_value() && !Int::parse("-").has_value());
    CHECK_EQ(Int::parse("-0")->to_string(), std::string("0"));
    CHECK_EQ(Int::parse("000123")->to_string(), std::string("123"));
}

// ---- Text and collections -------------------------------------------------------------------

TEST(text_is_bytes_and_orders_bytewise) {
    Text t("h\xC3\xA9llo");  // héllo
    CHECK_EQ(t.len(), 6u);
    CHECK(Text("h\xC3\xA9").concat(Text("llo")) == t);
    CHECK(Text("ab") < Text("b"));
    CHECK(Text("z") < Text("\xC3\xA9"));
    CHECK(Text("") == Text());
    CHECK(!utf8_error_at("h\xC3\xA9\xE4\xB8\x96\xF0\x9F\x98\x80"));
    CHECK(utf8_error_at("a\xFF" "b") == std::optional<std::size_t>(1));
    CHECK(utf8_error_at("\xC0\x80") == std::optional<std::size_t>(0));      // overlong
    CHECK(utf8_error_at("x\xED\xA0\x80") == std::optional<std::size_t>(1)); // a surrogate
    CHECK(utf8_error_at("\xF4\x90\x80\x80") == std::optional<std::size_t>(0)); // past U+10FFFF
    CHECK(utf8_error_at("ab\xE4\xB8") == std::optional<std::size_t>(2));    // cut short
    bool refused = false;
    try {
        Text bad("a\xFF" "b");
    } catch (const Panic& p) {
        refused = std::string(p.what()).find("byte 1 is malformed") != std::string::npos;
    }
    CHECK(refused);  // a malformed byte is a fatal error, never U+FFFD
}

TEST(cursors_walk_characters_and_cut_views) {
    Text t("h\xE2\x86\x92\xE4\xB8\x96");  // h→世
    Cursor c0 = Cursor::front(t);
    auto s1 = c0.step();
    CHECK(s1 && s1->first == U'h' && s1->second.at() == 1u);
    auto s2 = s1->second.step();
    CHECK(s2 && s2->first == U'→' && s2->second.at() == 4u);
    auto s3 = s2->second.step();
    CHECK(s3 && s3->first == U'世' && s3->second.at() == 7u);
    CHECK(!s3->second.step() && !c0.step_back());
    CHECK(s3->second == Cursor::back(t));
    auto b = Cursor::back(t).step_back();
    CHECK(b && b->first == U'世' && b->second.at() == 4u);
    Cursor c2 = s2->second;
    CHECK_EQ(c2.before().to_string(), std::string("h\xE2\x86\x92"));
    CHECK_EQ(c2.after().to_string(), std::string("\xE4\xB8\x96"));
    auto inner = Cursor::front(c2.after()).step();
    CHECK(inner && inner->first == U'世' && inner->second.before() == c2.after() && inner->second.after().is_empty());
    CHECK(!Cursor::at_byte(t, 2) && !Cursor::at_byte(t, 8));
    CHECK_EQ(Cursor::at_byte(t, 4)->after().to_string(), std::string("\xE4\xB8\x96"));
    CHECK(s1->second < c2);
    CHECK_EQ(show(c2), std::string("Cursor(at=4)"));
}

TEST(collections_copy_on_write) {
    Map<Text, Int> m = Map<Text, Int>::unit(Text("a"), Int(1));
    Map<Text, Int> n = m;
    n.insert(Text("b"), Int(2));
    CHECK_EQ(m.size(), 1u);
    CHECK_EQ(n.size(), 2u);
    CHECK(*n.get(Text("b")) == Int(2) && !m.get(Text("b")));
    CHECK(m.removed(Text("a")).empty() && m.size() == 1);
    Array<Int> xs{Int(1), Int(2)};
    Array<Int> ys = xs.pushed(Int(3));
    CHECK(xs.size() == 2 && ys.size() == 3 && ys[2] == Int(3));
    CHECK(xs.updated(0, Int(9))[0] == Int(9) && xs[0] == Int(1));
    CHECK(ys.take(2) == xs && ys.drop_front(2) == Array<Int>{Int(3)});
}

// ---- Floats ---------------------------------------------------------------------------------

namespace {
struct FloatVector {
    unsigned long long bits;
    const char* rust;
};
const FloatVector FLOATS[] = {
#include "float_vectors.inc"
};
}  // namespace

TEST(floats_print_as_rusts_display_and_read_back) {
    int checked = 0;
    for (const FloatVector& v : FLOATS) {
        double f = std::bit_cast<double>(std::uint64_t(v.bits));
        CHECK_EQ(float_digits(f), std::string(v.rust));
        std::string shown = show(Float{f});
        std::string want = std::string(v.rust) + (std::strchr(v.rust, '.') ? "" : ".0");
        CHECK_EQ(shown, want);
        // The spelling reads back to the same bits.
        auto back = read::value(shown);
        CHECK(back.has_value() && back->if_float() && std::bit_cast<std::uint64_t>(back->if_float()->v) == v.bits);
        ++checked;
    }
    CHECK(checked > 600);
    CHECK_EQ(show(Float{std::numeric_limits<double>::quiet_NaN()}), std::string("nan"));
    CHECK_EQ(show(Float{-std::numeric_limits<double>::infinity()}), std::string("-inf"));
    CHECK(Pod::floating(std::nan("")) == Pod::floating(std::nan("")));  // total order: nan keys a map
    CHECK(Pod::floating(-0.0) < Pod::floating(0.0));
}

TEST(floats_compare_by_their_bits_every_nan_one) {
    // Type System round 6 FB: `==` is bit identity, `<=>` IEEE's totalOrder; NO: every NaN
    // one value, after +infinity, whatever its sign or payload.
    Float nan{std::nan("")}, zero{0.0}, neg_zero{-0.0}, one{1.0};
    CHECK(nan == nan && !(nan != nan) && (nan <=> nan) == 0);
    CHECK(!(zero == neg_zero) && neg_zero < zero);
    Float other{std::bit_cast<double>(0xfff8000000000001ull)};
    CHECK(Float{-std::nan("")} == nan && other == nan && (other <=> nan) == 0);
    CHECK(Float{-std::numeric_limits<double>::infinity()} < Float{-std::nan("")});
    CHECK(Float{std::numeric_limits<double>::infinity()} < other);
    CHECK(Float{std::numeric_limits<double>::infinity()} < nan);
    CHECK(one < Float{2.0} && Float{-2.0} < neg_zero);
    CHECK(zero + one == one && (zero - zero) == zero && zero * Float{-1.0} == neg_zero);
    CHECK(!(Float{0.1} + Float{0.2} == Float{0.3}));  // arithmetic stays IEEE
}

// ---- Spelling ---------------------------------------------------------------------------

TEST(spellings_round_trip) {
    const char* values[] = {
        "unit", "True", "-1", "42", "\"a \\\"b\\\"\\n\\t\\u{1}\"", "'x'", "nil", "? 3", "~ Contents(text=\"hello\")",
        "? Card(rank=Four, suit=Spades)", "Write \"end of page 1\\n\"", "? (Letter \"a\")", "{1: \"a\", 2: \"b\"}",
        "[[1, 2, 3]]", "[[]]", "((1, \"x\"))", "Ui.Button(label=\"Increment\")", "Test.Log", "click",
        "Error(kind=Fs.NotFound, message=\"gone\")", "1300us", "0s", "Rect 1, 2", "280.0", "-0.5",
        "0.30000000000000004", "nan", "-inf", "Press Point(x=280.0, y=400.0)",
        "Gui.Scene(window=WindowId \"Chess\", at=nil)", "Box(paint=(Rgb 1.0, 0.5, 0.0), inner=Node (Leaf 1))",
        "? (Node (Leaf 1), (Leaf 2))", "Name()", "\"h\xC3\xA9\"", "'\xC3\xA9'", "123456789012345678901234567890",
        "2h", "90min", "-3ms",
    };
    for (const char* s : values) {
        auto v = read::value(s);
        CHECK(v.has_value());
        if (v) CHECK_EQ(v->to_string(), std::string(s));
        else std::cerr << s << ": " << v.error() << "\n";
    }
    for (const char* s : {"0", "1/#0/1/#0/2", "2/[\"a\"]/0", "3/[1/#0/0]/0"}) {
        auto p = read::path(s);
        CHECK(p.has_value() && p->to_string() == s);
    }
    CHECK(!read::value("(1").has_value());
    CHECK(!read::value("\"open").has_value());
    CHECK(!read::value("1 2").has_value());
}

TEST(typed_values_print_and_cross_as_pods) {
    using M = Map<Text, Array<std::optional<Int>>>;
    M m = M::unit(Text("k"), Array<std::optional<Int>>{std::optional<Int>(Int(1)), std::nullopt});
    CHECK_EQ(show(m), std::string("{\"k\": [[? 1, nil]]}"));
    CHECK(from_pod<M>(to_pod(m)).value() == m);
    Fallible<Text> failed = unexpected(Error("Fs.NotFound", Text("gone")));
    CHECK_EQ(show(failed), std::string("Error(kind=Fs.NotFound, message=\"gone\")"));
    CHECK_EQ(to_pod(failed).to_string(), show(failed));
    auto back = from_pod<Fallible<Text>>(read::value("Error(kind=Fs.NotFound, message=\"gone\")").value());
    CHECK(back.has_value() && *back == failed);
    CHECK_EQ(show(std::tuple<Nat, Float>(Nat(3), Float{2.5})), std::string("((3, 2.5))"));
    auto t = from_pod<std::tuple<Int, Text, bool>>(read::value("((1, \"x\", True))").value());
    CHECK(t.has_value() && std::get<1>(*t) == Text("x"));
    CHECK(!from_pod<std::tuple<Int, Text>>(read::value("((1, 2))").value()).has_value());
    Pod shown = Pod::shown("e2", Pod::record("Spot", {{"down", Pod::int64(6)}}));
    CHECK_EQ(Path::root().at(0).child(Segment::key(shown)).at(0).to_string(), std::string("0/[e2]/0"));
    CHECK(read::pattern("Spot(down=6)").value().matches(shown));
    CHECK(!from_pod<std::uint8_t>(Pod::int64(256)).has_value());
    CHECK_EQ(from_pod<std::uint8_t>(Pod::int64(255)).value(), std::uint8_t(255));
    auto bad = from_pod<Int>(Pod::text("x"));
    CHECK(!bad.has_value() && bad.error().to_string() == "Runtime.Answer: expected an Int, got \"x\"");
    CHECK(!from_pod<Nat>(Pod::int64(-1)).has_value());
}

TEST(patterns_match) {
    Pod e = read::value("Ui.Button(label=\"OK\", enabled=True)").value();
    auto m = [&](const char* p) { return read::pattern(p).value().matches(e); };
    CHECK(m("Ui.Button(label=\"OK\", ..)") && m("Ui.*") && m("Ui.Button(label=ignore, enabled=True)"));
    CHECK(!m("Ui.Button(label=\"OK\")") && !m("Fs.*"));
    Pod err = read::value("Error(kind=Fs.NotFound, message=\"gone\")").value();
    CHECK(read::pattern("Error(kind=Fs.NotFound, ..)").value().matches(err));
    CHECK(Pattern::any().matches(err));
}

TEST(a_clickable_is_found_by_its_path_through_a_turn) {
    Pod rect = read::value("Rect(x=0.0, y=0.0, w=10.0, h=20.0)").value();
    Pod flat = Pod::element("Draw.Clickable", {{"rect", rect}, {"path", Pod::path(read::path("1/#0/0").value())}});
    Pod turn = read::value("Turn(x=0.0, y=0.0, angle=90.0, scale=2.0)").value();
    Pod group = Pod::element("Draw.Group", {{"turn", turn}, {"draws", Pod::array({flat})}});
    auto scene = [](Pod d) {
        return Pod::element("Gui.Scene", {{"window", read::value("WindowId \"w\"").value()}, {"draw", Pod::array({d})}});
    };
    Document doc;
    (void)doc.diff({Edit::added(Path::root().at(3), scene(flat))});
    auto found = find_clickables(doc, "1/#0/0");
    CHECK(found.size() == 1 && found[0].x == 5.0 && found[0].y == 10.0);
    CHECK(find_clickables(doc, "1/#0/1").empty());
    (void)doc.diff({Edit::changed(Path::root().at(3), scene(group))});
    auto c = find_clickables(doc, "1/#0/0");
    CHECK(c.size() == 1 && std::abs(c[0].x - -20.0) < 1e-9 && std::abs(c[0].y - 10.0) < 1e-9);
}

// ---- Hosting --------------------------------------------------------------------------------

namespace {

/// The program's `Elem` for this test: a button's level.
struct Elem {
    Text label;
    Pod to_pod() const { return Pod::element("Ui.Button", {{"label", Pod::text(label)}}); }
    bool operator==(const Elem&) const = default;
};

/// The program's `Output`: a click.
struct Output {
    static Result<Output> decode(const Elem&, const Pod& v) {
        if (auto c = v.if_choice(); c && c->ctor == "click") return Output{};
        return mismatch<Output>("click", v);
    }
    static Result<Output> decode_first(const Elem& e, const Pod& v) { return decode(e, v); }
};

/// An owner hosting one child button at child path `0`, every place forwarded: the child's
/// label is a hold, and a click on the forwarded element reaches the child in the tick. The
/// child's place is an entry of the host's document, taken in the owner's patch pass.
struct Owner : Node {
    Rows<Elem, Output> rows = new_rows<Elem, Output>();
    prism_rtl::Rc<Host<Elem, Output, Owner>> host;
    std::optional<Hold<Text>> label;
    prism_rtl::Rc<Entry<Elem>> place = Entry<Elem>::make("the child's place 0");
    int clicks = 0;

    Done deliver(const Path& place, const Output&, Cx& cx) {
        if (!place.segments()[0].is_pos(0)) return unexpected(Fault::answer("no such child place"));
        ++clicks;
        label->set(Text("clicked"), cx);
        cx.dirty(rc_from_this());  // the place reads the label: its entry is taken again
        return {};
    }
    Done patch(Cx& cx) override {
        place->take(Elem{label->get(cx)}, cx);
        return {};
    }
    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        // `9`: the child's place leaves `$doc`, and its row leaves with it (R8).
        if (at.size() == 1 && at[0].is_pos(9)) {
            for (const auto& [_, row] : *rows) row->leave_row(cx);
            *rows = Map<Path, prism_rtl::Rc<HostRow<Elem, Output>>>();
            return {};
        }
        // `hostpos/[p]/...`: the row at key `p`.
        if (at.size() >= 2 && at[0].is_pos(5) && at[1].kind() == Segment::Kind::Key) {
            auto key = from_pod<Path>(at[1].key_value());
            if (key)
                if (auto row = rows->get(*key)) return (*row)->answer(at.subspan(2), v, cx);
        }
        return unexpected(Fault::answer("no place at " + path_of(at).to_string()));
    }
};

struct OwnerProgram : Program {
    Runtime rt;
    prism_rtl::Rc<Owner> root;
    static Built<OwnerProgram> build(std::span<const Pod>, const ReadyState& ready) {
        auto p = std::make_unique<OwnerProgram>();
        Cx cx = p->rt.build(ready);
        auto owner = prism_rtl::make_rc<Owner>();
        owner->label.emplace(Text("press"), cx);
        Supplier<Elem, Output> supplier = [](const Path&, const Element<Elem>& e) { return element_blueprint<Elem, Output>(e); };
        owner->host = Host<Elem, Output, Owner>::make(
            Path::root().at(5), owner->rows, supplier, nullptr,
            [](const prism_rtl::Rc<Owner>& o, const Path& place, const Output& a, Cx& cx) {
                return o->deliver(place, a, cx);
            });
        owner->host->link(owner);
        owner->place->take(Elem{owner->label->get(cx)}, cx);
        // The host's `for` over `$doc`, as the emitted builder makes its rows.
        Path key = Path::root().at(0);
        prism_rtl::Rc<HostOf<Elem, Output>> h = owner->host;
        auto row = TRY((HostRow<Elem, Output>::make(Path::root().at(5).child(Segment::key(to_pod(key))), key, owner->place, h, cx)));
        owner->rows->insert(key, row);
        p->root = owner;
        auto out = TRY(p->rt.close(std::move(cx)));
        return std::pair{std::move(p), std::move(out)};
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
};

}  // namespace

TEST(a_forwarding_host_is_observably_not_there) {
    ReadyMap ready;
    auto built = OwnerProgram::build({}, ready);
    CHECK(built.has_value());
    auto& [program, out] = built.value();
    CHECK_EQ(out.patch.added.size(), 1u);
    CHECK_EQ(out.patch.added[0].first.to_string(), std::string("5/[0]/0"));
    CHECK_EQ(out.patch.added[0].second.to_string(), std::string("Ui.Button(label=\"press\")"));
    Path at = out.patch.added[0].first;  // its key is a path, which `[0]` would read back as an Int
    TickOut t1 = program->tick(TickIn::none().answer(at, Pod::name("click")), ready).value();
    CHECK_EQ(program->root->clicks, 1);  // in the same tick
    CHECK(t1.patch.changed.size() == 1 && t1.patch.changed[0].second.to_string() == "Ui.Button(label=\"clicked\")");
    CHECK(t1.pending);  // the place's entry moved: its publication is the next tick's, owed
    TickOut t2 = program->tick(TickIn::none(), ready).value();
    CHECK(!t2.pending && t2.patch.changed.empty());
    auto bad = program->tick(TickIn::none().answer(at, Pod::int64(1)), ready);
    CHECK(!bad.has_value() && bad.error().kind == "Runtime.Answer");
}

TEST(a_host_torn_down_with_its_rows_frees_everything) {
    // The row holds its host weakly, so no count cycle runs through the host: once the
    // rows leave and the program is dropped, every node and the host are freed (D5; LSan
    // reports zero leaks for this test).
    ReadyMap ready;
    prism_rtl::Weak<Owner> owner;
    prism_rtl::Weak<HostOf<Elem, Output>> host;
    prism_rtl::Weak<HostRow<Elem, Output>> row;
    {
        auto built = OwnerProgram::build({}, ready);
        CHECK(built.has_value());
        auto& program = built.value().first;
        owner = program->root;
        host = program->root->host;
        row = program->root->rows->begin()->second;
        TickOut t = program->tick(TickIn::none().answer(Path::root().at(9), Pod::unit()), ready).value();
        CHECK(t.patch.removed.size() == 1 && t.patch.removed[0].first.to_string() == "5/[0]/0");
        CHECK(row.expired());
        program->compact();
    }
    CHECK(owner.expired() && host.expired() && row.expired());
    // And with its rows still standing: dropping the program frees them too.
    {
        auto built = OwnerProgram::build({}, ready);
        owner = built.value().first->root;
        row = built.value().first->root->rows->begin()->second;
    }
    CHECK(owner.expired() && row.expired());
}

// ---- Persistence: an update never reaches a value that shares the contents -------------------

TEST(a_map_update_copies_only_its_path_and_leaves_its_sharers_alone) {
    Map<Int, Text> m;
    for (int i = 0; i < 200; ++i) m.insert(Int(i), Text(std::to_string(i)));
    Map<Int, Text> kept = m;
    m.insert(Int(7), Text("seven"));
    m.remove(Int(100));
    CHECK_EQ(kept.size(), std::size_t(200));
    CHECK(*kept.get(Int(7)) == Text("7"));
    CHECK(kept.contains(Int(100)) && !m.contains(Int(100)));
    *m.mut_get(Int(8)) = Text("eight");
    CHECK(*kept.get(Int(8)) == Text("8") && *m.get(Int(8)) == Text("eight"));
    int prev = -1;
    bool ordered = true;
    for (const auto& [k, v] : m) {
        int n = int(*k.to_i64());
        ordered = ordered && n > prev;
        prev = n;
    }
    CHECK(ordered);
    CHECK_EQ(m.size(), std::size_t(199));
}

TEST(an_array_view_grows_in_place_and_its_sharers_do_not_see_it) {
    Array<Int> a;
    for (int i = 0; i < 300; ++i) a = a.pushed(Int(i));
    Array<Int> b = a;
    const Int* first = a.get(0);
    a.push_back(Int(300));
    b.push_back(Int(-1));
    CHECK_EQ(a.size(), std::size_t(301));
    CHECK_EQ(b.size(), std::size_t(301));
    CHECK(a[300] == Int(300) && b[300] == Int(-1));
    CHECK(a.get(0) == first);
    Array<Int> tail = a.drop_front(10).take(5);
    CHECK(tail[0] == Int(10) && tail.size() == 5);
    tail.set(0, Int(99));
    CHECK(a[10] == Int(10) && tail[0] == Int(99));
    Array<bool> flags{true, false};
    CHECK(*flags.get(0) && !*flags.get(1));
}
