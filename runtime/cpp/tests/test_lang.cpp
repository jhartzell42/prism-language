// What the emitted C++ uses besides the library's types (prism_rtl/lang.hpp).
//
// Written by Claude, 4 November 2026, on the claude-self-hosting branch. Subject to change.

#include "check.hpp"
#include "prism_rtl/lang.hpp"

using namespace prism_rtl;

namespace {

Fallible<Text> twice(const Fallible<Text>& x) {
    Fallible<Text> kept = x;
    Text first = TRY(kept);
    Text second = TRY(kept);
    return Text(first.to_string() + second.to_string());
}

}  // namespace

TEST(try_on_a_named_value_leaves_it_as_it_was) {
    auto r = twice(Text("ab"));
    CHECK(r.has_value() && *r == Text("abab"));
    auto e = twice(unexpected(Error("Prelude.Unmet", Text("no"))));
    CHECK(!e.has_value() && e.error().kind == "Prelude.Unmet");
}

TEST(a_slot_holds_an_optional_none_as_a_value) {
    Slot<std::optional<Int>> s;
    s.set(std::nullopt);
    CHECK(!s.take().has_value());
}
