// Written by Claude on the claude-self-hosting branch; subject to change.
//
// `Map::united`, the prelude's `Union a, b` (Semantics Appendix B: "the left winning;
// `m` log `(n+m)`"): its result, and its cost, counted by the copies of a key type that
// counts them: the smaller map's entries go into the larger, whichever side it is.

#include <cstdint>

#include "check.hpp"
#include "prism_rtl/prism_rtl.hpp"

using namespace prism_rtl;

namespace {

/// A key that counts its copies.
struct Key {
    static inline std::size_t copies = 0;
    int k = 0;
    explicit Key(int x) : k(x) {}
    Key(const Key& o) : k(o.k) { ++copies; }
    Key(Key&&) noexcept = default;
    Key& operator=(const Key& o) {
        ++copies;
        k = o.k;
        return *this;
    }
    Key& operator=(Key&&) noexcept = default;
    friend bool operator<(const Key& a, const Key& b) { return a.k < b.k; }
};

Map<Key, int> keys(int from, int to, int value) {
    Map<Key, int> m;
    for (int i = from; i < to; ++i) m.insert(Key(i), value);
    return m;
}

}  // namespace

TEST(union_keeps_the_left_maps_entries_over_the_rights) {
    for (auto [n, m] : {std::pair{0, 0}, {0, 5}, {5, 0}, {3, 7}, {7, 3}, {100, 100}}) {
        Map<Key, int> a = keys(0, n, 1), b = keys(n / 2, n / 2 + m, 2);
        Map<Key, int> u = a.united(b);
        int all = std::max(n, n / 2 + m);
        CHECK_EQ(u.size(), std::size_t(all));
        for (int i = 0; i < all; ++i) CHECK_EQ(*u.get(Key(i)), i < n ? 1 : 2);
        CHECK(a.size() == std::size_t(n) && b.size() == std::size_t(m));
    }
}

TEST(union_costs_the_smaller_maps_entries_whichever_side_it_is) {
    constexpr int BIG = 20000, SMALL = 10;
    Map<Key, int> big = keys(0, BIG, 1), small = keys(BIG - SMALL / 2, BIG + SMALL / 2, 2);
    // each of the small map's entries copies at most a path's keys (about 1.44 log2 n)
    const std::size_t bound = SMALL * 64;
    Key::copies = 0;
    Map<Key, int> left_big = big.united(small);
    CHECK(Key::copies <= bound);
    Key::copies = 0;
    Map<Key, int> right_big = small.united(big);
    CHECK(Key::copies <= bound);
    CHECK(left_big.size() == std::size_t(BIG + SMALL / 2) && right_big.size() == std::size_t(BIG + SMALL / 2));
    CHECK(*left_big.get(Key(BIG - 1)) == 1 && *right_big.get(Key(BIG - 1)) == 2);
    CHECK(*left_big.get(Key(BIG)) == 2 && *right_big.get(Key(0)) == 1);
}
