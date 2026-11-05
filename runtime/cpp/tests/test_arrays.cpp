// Written by Claude on the claude-self-hosting branch; subject to change.
//
// `Array` at the sizes around its chunks' edges (empty, 1, 2, 63, 64, 65, ...): built by
// `push_back`, `pushed`, a `std::vector` and a list; a view that shares its buffer written
// at each size; `take` and `drop_front` then written; `concat`, also with itself; `set`,
// `mut_at`, iteration, equality and order; a write that throws halfway; and the storage an
// array holds, which is linear in its length (claude/bugs/small-arrays-whole-chunks.md:
// every array used to take whole 64-element chunks).

#include <cstdint>
#include <numeric>
#include <stdexcept>
#include <vector>

#include "check.hpp"
#include "prism_rtl/prism_rtl.hpp"

using namespace prism_rtl;

namespace {

const std::size_t SIZES[] = {0, 1, 2, 3, 4, 7, 8, 31, 32, 63, 64, 65, 126, 127, 128, 129, 191, 192, 193, 300};

Array<Int> counting(std::size_t n) {
    Array<Int> a;
    for (std::size_t i = 0; i < n; ++i) a.push_back(Int(std::int64_t(i)));
    return a;
}

bool counts(const Array<Int>& a, std::size_t n, std::int64_t from = 0) {
    if (a.size() != n) return false;
    std::int64_t want = from;
    for (const Int& x : a)
        if (!(x == Int(want++))) return false;
    for (std::size_t i = 0; i < n; ++i)
        if (!(a[i] == Int(from + std::int64_t(i))) || a.get(i) != &a[i]) return false;
    return a.get(n) == nullptr;
}

/// The storage a buffer of `n` elements may hold: linear, about twice `n` at worst while
/// it is small, never a whole chunk for a few elements.
std::size_t bound(std::size_t n) { return 2 * n + 1; }

}  // namespace

TEST(an_array_holds_its_elements_at_every_size_around_a_chunk) {
    for (std::size_t n : SIZES) {
        Array<Int> pushed = counting(n);
        Array<Int> by_value;
        std::vector<Int> v;
        for (std::size_t i = 0; i < n; ++i) {
            by_value = by_value.pushed(Int(std::int64_t(i)));
            v.push_back(Int(std::int64_t(i)));
        }
        Array<Int> from_vector(v);
        CHECK(counts(pushed, n));
        CHECK(counts(by_value, n));
        CHECK(counts(from_vector, n));
        CHECK(pushed == by_value && by_value == from_vector);
        CHECK((pushed <=> from_vector) == std::strong_ordering::equal);
        CHECK_EQ(pushed.empty(), n == 0);
        std::int64_t sum = 0;
        for (const Int& x : pushed) sum += *x.to_i64();
        CHECK_EQ(sum, std::int64_t(n * (n == 0 ? 0 : n - 1) / 2));
    }
    Array<Int> listed{Int(0), Int(1), Int(2)};
    CHECK(listed == counting(3) && counting(3) < counting(4) && counting(4) > counting(3).pushed(Int(-1)));
}

TEST(a_shared_view_written_at_every_size_leaves_its_sharer_alone) {
    for (std::size_t n : SIZES) {
        Array<Int> a = counting(n);
        Array<Int> b = a;
        b.push_back(Int(-1));
        a.push_back(Int(std::int64_t(n)));
        CHECK(counts(a, n + 1));
        CHECK(counts(b.take(n), n) && b.size() == n + 1 && b[n] == Int(-1));
        Array<Int> c = a;
        if (n > 0) {
            c.set(n - 1, Int(99));
            CHECK(counts(a, n + 1) && c[n - 1] == Int(99) && c.size() == n + 1);
            Array<Int> d = a;
            d.mut_at(0) = Int(-5);
            CHECK(counts(a, n + 1) && d[0] == Int(-5) && counts(d.drop_front(1), n, 1));
            CHECK(a.updated(0, Int(7))[0] == Int(7) && a[0] == Int(0));
        }
        CHECK(a != b);
    }
}

TEST(a_taken_or_dropped_view_written_copies_only_its_own_elements) {
    for (std::size_t n : SIZES) {
        Array<Int> a = counting(n);
        for (std::size_t k : {std::size_t(0), std::size_t(1), n / 2, n, n + 3}) {
            std::size_t kk = std::min(k, n);
            Array<Int> front = a.take(k);
            Array<Int> back = a.drop_front(k);
            CHECK(counts(front, kk));
            CHECK(counts(back, n - kk, std::int64_t(kk)));
            front.push_back(Int(-1));
            back.push_back(Int(-2));
            CHECK(counts(a, n));
            CHECK(counts(front.take(kk), kk) && front[kk] == Int(-1));
            CHECK(counts(back.take(n - kk), n - kk, std::int64_t(kk)) && back[n - kk] == Int(-2));
            CHECK(front.capacity() <= bound(kk + 1));
            // `back` ends where `a`'s buffer does, so it appended there in place
            CHECK(back.capacity() <= bound(n + 1));
            CHECK(front.take(kk).concat(back.take(n - kk)) == a);
        }
    }
}

TEST(concatenation_at_every_size_also_of_an_array_with_itself) {
    for (std::size_t n : SIZES) {
        Array<Int> a = counting(n);
        Array<Int> twice = a.concat(a);
        CHECK(counts(a, n));
        CHECK(twice.size() == 2 * n && counts(twice.take(n), n) && counts(twice.drop_front(n), n));
        Array<Int> self = a;
        self = self.concat(self);
        CHECK(self == twice);
        Array<Int> rest = counting(n + 5).drop_front(n);
        CHECK(counts(a.concat(rest), n + 5));
        CHECK(Array<Int>().concat(a) == a && a.concat(Array<Int>()) == a);
    }
}

TEST(an_array_holds_storage_linear_in_its_length) {
    CHECK_EQ(Array<Int>().capacity(), std::size_t(0));
    for (std::size_t n : SIZES) {
        Array<Int> a = counting(n);
        CHECK(a.capacity() >= n);
        CHECK(a.capacity() <= bound(n));
        std::vector<Int> v(n, Int(1));
        CHECK(Array<Int>(v).capacity() <= bound(n));
        // a copy for a write copies the elements there are, into storage of their size
        Array<Int> shared = a;
        shared.push_back(Int(-1));
        CHECK(shared.capacity() <= bound(n + 1));
        if (n > 0) {
            Array<Int> written = a;
            written.set(0, Int(5));
            CHECK(written.capacity() <= bound(n));
        }
    }
    Array<Int> two{Int(1), Int(2)};
    CHECK(two.capacity() <= 3);
}

namespace {

/// A value whose copies and moves throw once a countdown, shared by all of them, runs out.
struct Brittle {
    static inline int budget = -1;
    int v = 0;
    explicit Brittle(int x) : v(x) {}
    Brittle(const Brittle& o) : v(o.v) { spend(); }
    Brittle(Brittle&& o) : v(o.v) { spend(); }
    Brittle& operator=(const Brittle& o) {
        spend();
        v = o.v;
        return *this;
    }
    Brittle& operator=(Brittle&& o) {
        spend();
        v = o.v;
        return *this;
    }
    static void spend() {
        if (budget == 0) throw std::runtime_error("brittle");
        if (budget > 0) --budget;
    }
};

}  // namespace

TEST(a_write_that_throws_halfway_leaves_the_array_as_it_was) {
    for (std::size_t n : SIZES) {
        Array<Brittle> a;
        for (std::size_t i = 0; i < n; ++i) a.push_back(Brittle(int(i)));
        // a view that no longer ends where its buffer does copies itself before a push;
        // the copy throws partway
        Array<Brittle> grown = a;
        grown.push_back(Brittle(-3));
        Array<Brittle> b = a;
        Brittle::budget = int(n / 2);
        bool threw = false;
        try {
            b.push_back(Brittle(-1));
        } catch (const std::runtime_error&) {
            threw = true;
        }
        Brittle::budget = -1;
        CHECK(threw);
        CHECK(a.size() == n && b.size() == (threw ? n : n + 1));
        for (std::size_t i = 0; i < n; ++i) CHECK(a[i].v == int(i) && b[i].v == int(i));
        // the newest view's push throws as the element moves into its slot (after any
        // storage for it is allocated); the next pushes still land
        Brittle::budget = 1;
        threw = false;
        try {
            a.push_back(Brittle(-2));
        } catch (const std::runtime_error&) {
            threw = true;
        }
        Brittle::budget = -1;
        CHECK(threw && a.size() == n);
        a.push_back(Brittle(int(n)));
        a.push_back(Brittle(int(n + 1)));
        CHECK(a.size() == n + 2 && a[n].v == int(n) && a[n + 1].v == int(n + 1));
        for (std::size_t i = 0; i < n; ++i) CHECK(a[i].v == int(i));
        CHECK(a.capacity() <= bound(n + 2));
    }
}

TEST(an_elements_address_holds_while_its_buffer_grows) {
    Array<Int> a;
    std::vector<const Int*> at;
    for (std::size_t i = 0; i < 300; ++i) {
        a.push_back(Int(std::int64_t(i)));
        at.push_back(&a[i]);
    }
    for (std::size_t i = 0; i < 300; ++i) CHECK(&a[i] == at[i] && *at[i] == Int(std::int64_t(i)));
    // updates in place through several references at once (`iter_mut().enumerate()`)
    Array<Int> b = a.take(10);
    std::vector<Int*> refs;
    for (std::size_t i = 0; i < b.size(); ++i) refs.push_back(&b.mut_at(i));
    for (std::size_t i = 0; i < refs.size(); ++i) *refs[i] = Int(std::int64_t(100 + i));
    CHECK(counts(b, 10, 100) && counts(a, 300));
}
