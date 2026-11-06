// Written by Claude on the claude-self-hosting branch; subject to change.
//
// `Map::united`, the prelude's `Union a, b` (Semantics Appendix B: "the left winning;
// `m` log `(n+m)`"): its result, and its cost, counted by the copies of a key type that
// counts them: the smaller map's entries go into the larger, whichever side it is. And
// `==` on maps that share nodes, by counting the values compared: a shared node answers at
// once, and a NaN (a value equal to nothing, itself included) keeps a map unequal to
// itself, shared or not.

#include <cmath>
#include <cstdint>
#include <tuple>
#include <vector>

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

// ---- Equality on shared structure, by counting the values compared ---------------------

namespace {

/// A value that counts its comparisons; `Cmp{NAN_}` is equal to nothing, itself included,
/// as a `Float` NaN is under IEEE equality (Type System ch. 2).
struct Cmp {
    static inline std::size_t compared = 0;
    int v = 0;
    friend bool operator==(const Cmp& a, const Cmp& b) {
        ++compared;
        return a.v == b.v && a.v != -1;
    }
};
constexpr int NAN_ = -1;

/// `0..n` in a scattered order: `i * 7919 mod p` for a prime `p >= n`, kept below `n`.
std::vector<int> scattered(int n) {
    int p = std::max(n, 2);
    for (;; ++p) {
        bool prime = true;
        for (int d = 2; d * d <= p; ++d)
            if (p % d == 0) prime = false;
        if (prime) break;
    }
    std::vector<int> out;
    for (int i = 0; i < p; ++i)
        if (long(i) * 7919 % p < n) out.push_back(int(long(i) * 7919 % p));
    return out;
}

Map<int, Cmp> cmps(int n) {
    Map<int, Cmp> m;
    for (int k : scattered(n)) m.insert(k, Cmp{k});
    return m;
}

/// The values one comparison may look at whatever the map's size: a few per level of a
/// tree of 20,000 (height at most 21).
constexpr std::size_t BOUNDED = 128;
constexpr int BIG = 20000;

template <class F>
std::size_t compared(F f) {
    std::size_t before = Cmp::compared;
    f();
    return Cmp::compared - before;
}

}  // namespace

TEST(a_map_compared_with_what_shares_its_nodes_compares_a_bounded_number_of_values) {
    for (int n : {0, 1, 2, 64, 65, 300, BIG}) {
        Map<int, Cmp> a = cmps(n), b = a;
        // the first comparison may look at every value once; after it, shared nodes answer
        // at once
        CHECK(a == b);
        bool eq = false;
        CHECK(compared([&] { eq = a == b; }) <= BOUNDED && eq);
        if (n == 0) continue;
        Map<int, Cmp> c = a.inserted(n / 2, Cmp{n / 2});
        CHECK(compared([&] { eq = a == c; }) <= BOUNDED && eq);
        Map<int, Cmp> d = a.inserted(n / 2, Cmp{n + 1});
        CHECK(compared([&] { eq = a == d; }) <= BOUNDED && !eq);
        Map<int, Cmp> e = a.inserted(n, Cmp{n}).removed(n);
        CHECK(compared([&] { eq = e == a; }) <= BOUNDED && eq);
    }
}

TEST(a_shared_map_holding_a_nan_is_not_equal_to_itself) {
    for (int n : {1, 2, 64, 65, 300, BIG}) {
        Map<int, Cmp> a = cmps(n).inserted(n / 3, Cmp{NAN_});
        Map<int, Cmp> b = a;
        CHECK(!(a == b) && !(a == b) && !(a == a));
        CHECK(!(a.inserted(n, Cmp{n}) == a.inserted(n, Cmp{n})));
    }
    Map<int, double> floats{{1, 1.0}, {2, std::nan("")}};
    Map<int, double> same = floats;
    CHECK(!(floats == same));
}

TEST(maps_of_maps_compare_shared_inner_maps_at_once) {
    Map<int, Cmp> inner = cmps(BIG);
    auto outer = [&] {
        Map<int, std::tuple<Map<int, Cmp>, int>> m;
        for (int k = 0; k < 10; ++k) m.insert(k, std::tuple<Map<int, Cmp>, int>(inner, 1));
        return m;
    };
    auto a = outer(), b = outer();
    CHECK(a == b);
    bool eq = false;
    CHECK(compared([&] { eq = a == b; }) <= BOUNDED && eq);
}
