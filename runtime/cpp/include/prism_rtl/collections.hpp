// The collections Prism's types compile to: `Text`, `k ==> v` (`Map`), `[[a]]` (`Array`).
//
// Values are immutable and cheap to copy: each is a `shared_ptr` to its contents, so a
// copy is a pointer copy, and an update copies the contents only when they are shared
// (copy on write). A unique value updates in place, which is what the emitted side's
// `&` blocks rely on (Compiler ch. 3, "Unique values"): `insert`, `remove`, `push_back`
// and `set` mutate this value, copying first when another value shares it; `inserted`,
// `removed`, `pushed` and `updated` leave this value alone and answer the new one.
//
// The cost promises of Semantics Appendix B are the Rust library's (a rope, a persistent
// ordered map, an RRB vector: logarithmic updates under sharing). Here an update of a
// shared value is linear; an update of a unique one is the standard container's. See
// doubt CW in README.md.

#pragma once

#include <algorithm>
#include <compare>
#include <cstdint>
#include <initializer_list>
#include <map>
#include <memory>
#include <optional>
#include <ostream>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

namespace prism_rtl {

// ---- UTF-8 ----------------------------------------------------------------------------

/// The Unicode scalar values of UTF-8 text; a malformed byte reads as U+FFFD.
std::u32string utf8_decode(std::string_view s);
/// One scalar value appended as UTF-8.
void utf8_push(std::string& out, char32_t c);
std::string utf8_encode(std::u32string_view s);

// ---- Text -------------------------------------------------------------------------------

/// `Text`: a sequence of Unicode scalar values, ordered by its characters. `Len` and a
/// character's index are constant time; `Take`, `Drop` and concatenation copy.
class Text {
public:
    Text() = default;
    Text(const char* utf8) : Text(std::string_view(utf8)) {}
    Text(std::string_view utf8);
    Text(const std::string& utf8) : Text(std::string_view(utf8)) {}
    explicit Text(std::u32string chars);

    /// `Len`: the number of characters.
    std::size_t len() const { return chars().size(); }
    bool is_empty() const { return len() == 0; }
    /// `Take text, n`: the first `n` characters.
    Text take(std::size_t n) const;
    /// `Drop text, n`: everything after the first `n` characters.
    Text drop_front(std::size_t n) const;
    /// Concatenation, as interpolation does it.
    Text concat(const Text& other) const;
    std::optional<char32_t> char_at(std::size_t i) const;
    /// The characters.
    const std::u32string& chars() const;
    /// The text as UTF-8.
    std::string to_string() const { return utf8_encode(chars()); }

    friend bool operator==(const Text& a, const Text& b) { return a.chars() == b.chars(); }
    friend std::strong_ordering operator<=>(const Text& a, const Text& b) {
        return a.chars().compare(b.chars()) <=> 0;
    }
    friend Text operator+(const Text& a, const Text& b) { return a.concat(b); }

private:
    std::shared_ptr<const std::u32string> s_;
};

inline std::ostream& operator<<(std::ostream& o, const Text& t) { return o << t.to_string(); }

// ---- Map --------------------------------------------------------------------------------

/// `k ==> v`: an ordered map, copy on write.
template <class K, class V>
class Map {
public:
    using Inner = std::map<K, V>;
    using const_iterator = typename Inner::const_iterator;

    Map() = default;
    Map(std::initializer_list<std::pair<const K, V>> es) : m_(std::make_shared<Inner>(es)) {}
    /// A map of one entry.
    static Map unit(K k, V v) {
        Map m;
        m.insert(std::move(k), std::move(v));
        return m;
    }

    std::size_t size() const { return m_ ? m_->size() : 0; }
    bool empty() const { return size() == 0; }
    /// The value at `k`, or null.
    const V* get(const K& k) const {
        if (!m_) return nullptr;
        auto it = m_->find(k);
        return it == m_->end() ? nullptr : &it->second;
    }
    bool contains(const K& k) const { return get(k) != nullptr; }

    /// Set `k` to `v` in this value (in place when unique).
    void insert(K k, V v) { mut().insert_or_assign(std::move(k), std::move(v)); }
    void remove(const K& k) {
        if (contains(k)) mut().erase(k);
    }
    /// The map with `k` set to `v`; this one is unchanged.
    Map inserted(K k, V v) const {
        Map m = *this;
        m.insert(std::move(k), std::move(v));
        return m;
    }
    Map removed(const K& k) const {
        Map m = *this;
        m.remove(k);
        return m;
    }

    const_iterator begin() const { return inner().begin(); }
    const_iterator end() const { return inner().end(); }
    const Inner& inner() const {
        static const Inner none;
        return m_ ? *m_ : none;
    }
    /// The entries to update in place (copied first when shared): an `&` block's `&m[k]`.
    Inner& items_mut() { return mut(); }

    friend bool operator==(const Map& a, const Map& b) { return a.m_ == b.m_ || a.inner() == b.inner(); }
    friend auto operator<=>(const Map& a, const Map& b) { return a.inner() <=> b.inner(); }

private:
    Inner& mut() {
        if (!m_) m_ = std::make_shared<Inner>();
        else if (m_.use_count() > 1) m_ = std::make_shared<Inner>(*m_);
        return *m_;
    }
    std::shared_ptr<Inner> m_;
};

// ---- Array ------------------------------------------------------------------------------

/// How an `Array<bool>` stores an element: a `bool` with an address of its own (a
/// `std::vector<bool>` packs bits, whose elements have none).
struct BoolCell {
    bool v;
    BoolCell(bool b = false) : v(b) {}
    operator const bool&() const { return v; }
    operator bool&() { return v; }
    friend bool operator==(BoolCell a, BoolCell b) { return a.v == b.v; }
    friend auto operator<=>(BoolCell a, BoolCell b) { return a.v <=> b.v; }
};

/// `[[a]]`: a vector, copy on write.
template <class T>
class Array {
public:
    using Stored = std::conditional_t<std::is_same_v<T, bool>, BoolCell, T>;
    using Inner = std::vector<Stored>;
    using const_iterator = typename Inner::const_iterator;

    Array() = default;
    Array(std::initializer_list<T> xs) : v_(std::make_shared<Inner>(xs.begin(), xs.end())) {}
    explicit Array(std::vector<T> xs) : v_(adopt(std::move(xs))) {}
    explicit Array(Inner xs)
        requires(!std::is_same_v<Inner, std::vector<T>>)
        : v_(std::make_shared<Inner>(std::move(xs))) {}

    std::size_t size() const { return v_ ? v_->size() : 0; }
    bool empty() const { return size() == 0; }
    /// The element at `i`, or null.
    const T* get(std::size_t i) const { return i < size() ? &static_cast<const T&>((*v_)[i]) : nullptr; }
    const T& operator[](std::size_t i) const { return (*v_)[i]; }

    void push_back(T x) { mut().push_back(std::move(x)); }
    void set(std::size_t i, T x) { mut().at(i) = std::move(x); }
    Array pushed(T x) const {
        Array a = *this;
        a.push_back(std::move(x));
        return a;
    }
    Array updated(std::size_t i, T x) const {
        Array a = *this;
        a.set(i, std::move(x));
        return a;
    }
    Array concat(const Array& o) const {
        Array a = *this;
        if (!o.empty()) a.mut().insert(a.mut().end(), o.begin(), o.end());
        return a;
    }
    /// The first `n` elements.
    Array take(std::size_t n) const {
        if (n >= size()) return *this;
        return Array(Inner(begin(), begin() + std::ptrdiff_t(n)));
    }
    /// Everything after the first `n`.
    Array drop_front(std::size_t n) const {
        if (n == 0) return *this;
        if (n >= size()) return Array();
        return Array(Inner(begin() + std::ptrdiff_t(n), end()));
    }

    const_iterator begin() const { return inner().begin(); }
    const_iterator end() const { return inner().end(); }
    const Inner& inner() const {
        static const Inner none;
        return v_ ? *v_ : none;
    }
    /// The elements to update in place (copied first when shared): an `&` block's
    /// `&xs[i]`, a `for &s <- &xs:` loop.
    Inner& items_mut() { return mut(); }

    friend bool operator==(const Array& a, const Array& b) { return a.v_ == b.v_ || a.inner() == b.inner(); }
    friend auto operator<=>(const Array& a, const Array& b) { return a.inner() <=> b.inner(); }

private:
    static std::shared_ptr<Inner> adopt(std::vector<T>&& xs) {
        if constexpr (std::is_same_v<Inner, std::vector<T>>) return std::make_shared<Inner>(std::move(xs));
        else return std::make_shared<Inner>(xs.begin(), xs.end());
    }
    Inner& mut() {
        if (!v_) v_ = std::make_shared<Inner>();
        else if (v_.use_count() > 1) v_ = std::make_shared<Inner>(*v_);
        return *v_;
    }
    std::shared_ptr<Inner> v_;
};

}  // namespace prism_rtl
