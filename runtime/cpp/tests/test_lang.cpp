// What the emitted C++ uses besides the library's types (prism_rtl/lang.hpp).
//
// Written by Claude, 4 November 2026, on the claude-self-hosting branch. Subject to change.

#include "check.hpp"
#include "prism_rtl/lang.hpp"

using namespace prism_rtl;

namespace {

// A named operand of `?` is copied (`copy_of`, what the emitter writes where the model
// says the operand is named), so it is left as it was.
Fallible<Text> twice(const Fallible<Text>& x) {
    Fallible<Text> kept = x;
    Text first = TRY(copy_of(kept));
    Text second = TRY(copy_of(kept));
    return Text(first.to_string() + second.to_string());
}

/// A value that counts its copies.
struct Counted {
    static inline int copies = 0;
    std::shared_ptr<int> p = std::make_shared<int>(7);
    Counted() = default;
    Counted(const Counted& o) : p(o.p) { ++copies; }
    Counted(Counted&&) = default;
    Counted& operator=(const Counted&) = default;
    Counted& operator=(Counted&&) = default;
};

Fallible<Counted> made() { return Counted(); }

Fallible<long> moved_and_copied(Fallible<Counted>& named, long& named_count) {
    Counted a = TRY(made());
    Counted b = TRY(copy_of(named));
    named_count = named->p.use_count();
    return long(a.p.use_count());
}

}  // namespace

TEST(try_on_a_named_value_leaves_it_as_it_was) {
    auto r = twice(Text("ab"));
    CHECK(r.has_value() && *r == Text("abab"));
    auto e = twice(unexpected(Error("Prelude.Unmet", Text("no"))));
    CHECK(!e.has_value() && e.error().kind == "Prelude.Unmet");
}

TEST(try_moves_a_temporary_and_copies_a_named_value_once) {
    Fallible<Counted> named = Counted();
    Counted::copies = 0;
    long named_count = 0;
    auto r = moved_and_copied(named, named_count);
    CHECK(r.has_value() && *r == 1);
    CHECK_EQ(Counted::copies, 1);
    CHECK_EQ(named_count, 2L);
    CHECK_EQ(named->p.use_count(), 1L);
}

TEST(a_slot_holds_an_optional_none_as_a_value) {
    Slot<std::optional<Int>> s;
    s.set(std::nullopt);
    CHECK(!s.take().has_value());
}
