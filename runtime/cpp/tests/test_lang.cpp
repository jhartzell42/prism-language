// What the emitted C++ uses besides the library's types (prism_rtl/lang.hpp).
//
// Written by Claude, 4 November 2026, on the claude-self-hosting branch. Subject to change.

#include "check.hpp"
#include "prism_rtl/lang.hpp"

// g++ 13 at -O1 under ASan warns that the Slot test's optional<optional<Int>> may be read
// uninitialized, which it is not (a known false positive of -Wmaybe-uninitialized).
#if defined(__GNUC__) && !defined(__clang__)
#pragma GCC diagnostic ignored "-Wmaybe-uninitialized"
#endif

using namespace prism_rtl;

namespace {

/// A value that counts its copies.
struct Counted {
    static inline int copies = 0;
    prism_rtl::Rc<int> p = prism_rtl::make_rc<int>(7);
    Counted() = default;
    Counted(const Counted& o) : p(o.p) { ++copies; }
    Counted(Counted&&) = default;
    Counted& operator=(const Counted&) = default;
    Counted& operator=(Counted&&) = default;
};

Fallible<Counted> made() { return Counted(); }

Fallible<long> moved(Text& failed) {
    Counted a = TRY(made());
    Fallible<Counted> bad = unexpected(Error("Prelude.Unmet", Text("no")));
    failed = Text("not yet");
    Counted b = TRY(std::move(bad));
    failed = Text("passed");
    return long(a.p.use_count() + b.p.use_count());
}

}  // namespace

TEST(try_moves_a_temporary_and_returns_a_failure) {
    Counted::copies = 0;
    Text failed;
    auto r = moved(failed);
    CHECK(!r.has_value() && r.error().kind_p == "Prelude.Unmet");
    CHECK(failed == Text("not yet"));
    CHECK_EQ(Counted::copies, 0);
}

TEST(a_slot_holds_an_optional_none_as_a_value) {
    Slot<std::optional<Int>> s;
    s.set(std::nullopt);
    CHECK(!s.take().has_value());
}
