// The collections Prism's types compile to: `Text`, `k ==> v` (`Map`), `[[a]]` (`Array`).
//
// Values are immutable and cheap to copy: a copy is a pointer copy. `insert`, `remove`,
// `push_back` and `set` update this value (the emitted side's `&` blocks, Compiler ch. 3,
// "Unique values"); `inserted`, `removed`, `pushed` and `updated` leave it alone and
// answer the new one. Neither ever changes another value that shares the contents.
//
// The cost promises are Semantics Appendix B's, as the Rust library's persistent
// structures keep them: a `Map` is a persistent AVL tree (an update copies the path to its
// key, logarithmic); an `Array` is a view of a shared buffer of chunks, so `take` and
// `drop_front` are constant and so is a `push_back` onto the newest view of its buffer
// (the way an array built by a fold grows); an element's address never moves. A `Text`
// copies on `take`, `drop_front` and concatenation.

#pragma once

#include <algorithm>
#include <compare>
#include <cstdint>
#include <initializer_list>
#include <map>
#include <new>
#include <memory>
#include <optional>
#include <ostream>
#include <string>
#include <string_view>
#include <type_traits>
#include <stdexcept>
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

/// `k ==> v`: a persistent ordered map, an AVL tree whose updates copy the path to the
/// key they change and share the rest.
template <class K, class V>
class Map {
public:
    using value_type = std::pair<const K, V>;

private:
    struct Node {
        value_type kv;
        std::shared_ptr<const Node> left, right;
        int height;
        std::size_t count;
    };
    using Ptr = std::shared_ptr<const Node>;
    static int height(const Ptr& n) { return n ? n->height : 0; }
    static std::size_t count(const Ptr& n) { return n ? n->count : 0; }
    static Ptr make(value_type kv, Ptr l, Ptr r) {
        int h = std::max(height(l), height(r)) + 1;
        std::size_t c = count(l) + count(r) + 1;
        return std::make_shared<const Node>(Node{std::move(kv), std::move(l), std::move(r), h, c});
    }
    static Ptr balance(value_type kv, Ptr l, Ptr r) {
        int hl = height(l), hr = height(r);
        if (hl > hr + 1) {
            if (height(l->left) >= height(l->right)) return make(l->kv, l->left, make(std::move(kv), l->right, std::move(r)));
            return make(l->right->kv, make(l->kv, l->left, l->right->left), make(std::move(kv), l->right->right, std::move(r)));
        }
        if (hr > hl + 1) {
            if (height(r->right) >= height(r->left)) return make(r->kv, make(std::move(kv), std::move(l), r->left), r->right);
            return make(r->left->kv, make(std::move(kv), std::move(l), r->left->left), make(r->kv, r->left->right, r->right));
        }
        return make(std::move(kv), std::move(l), std::move(r));
    }
    static Ptr put(const Ptr& n, const K& k, const V& v) {
        if (!n) return make(value_type(k, v), nullptr, nullptr);
        if (k < n->kv.first) return balance(n->kv, put(n->left, k, v), n->right);
        if (n->kv.first < k) return balance(n->kv, n->left, put(n->right, k, v));
        return make(value_type(k, v), n->left, n->right);
    }
    static Ptr take_min(const Ptr& n, const Node*& min) {
        if (!n->left) {
            min = n.get();
            return n->right;
        }
        return balance(n->kv, take_min(n->left, min), n->right);
    }
    static Ptr erase(const Ptr& n, const K& k) {
        if (!n) return n;
        if (k < n->kv.first) return balance(n->kv, erase(n->left, k), n->right);
        if (n->kv.first < k) return balance(n->kv, n->left, erase(n->right, k));
        if (!n->left) return n->right;
        if (!n->right) return n->left;
        const Node* min = nullptr;
        Ptr r = take_min(n->right, min);
        return balance(min->kv, n->left, std::move(r));
    }
    /// The path to `k` copied, its node's value answered for update in place.
    static Ptr copy_to(const Ptr& n, const K& k, V*& at) {
        if (!n) return n;
        Ptr l = n->left, r = n->right;
        if (k < n->kv.first) l = copy_to(n->left, k, at);
        else if (n->kv.first < k) r = copy_to(n->right, k, at);
        auto fresh = std::make_shared<Node>(Node{n->kv, l, r, n->height, n->count});
        if (!(k < n->kv.first) && !(n->kv.first < k)) at = &fresh->kv.second;
        return fresh;
    }

public:
    /// In order of the keys.
    class const_iterator {
    public:
        using iterator_category = std::forward_iterator_tag;
        using value_type = Map::value_type;
        using difference_type = std::ptrdiff_t;
        using pointer = const value_type*;
        using reference = const value_type&;
        const_iterator() = default;
        explicit const_iterator(const Node* root) { descend(root); }
        reference operator*() const { return stack_.back()->kv; }
        pointer operator->() const { return &stack_.back()->kv; }
        const_iterator& operator++() {
            const Node* n = stack_.back();
            stack_.pop_back();
            descend(n->right.get());
            return *this;
        }
        const_iterator operator++(int) {
            const_iterator t = *this;
            ++*this;
            return t;
        }
        friend bool operator==(const const_iterator& a, const const_iterator& b) {
            return a.stack_.empty() ? b.stack_.empty() : (!b.stack_.empty() && a.stack_.back() == b.stack_.back());
        }

    private:
        void descend(const Node* n) {
            for (; n; n = n->left.get()) stack_.push_back(n);
        }
        std::vector<const Node*> stack_;
    };
    using iterator = const_iterator;

    Map() = default;
    Map(std::initializer_list<value_type> es) {
        for (const auto& [k, v] : es) insert(k, v);
    }
    /// A map of one entry.
    static Map unit(K k, V v) {
        Map m;
        m.insert(std::move(k), std::move(v));
        return m;
    }

    std::size_t size() const { return count(root_); }
    bool empty() const { return !root_; }
    /// The value at `k`, or null.
    const V* get(const K& k) const {
        for (const Node* n = root_.get(); n;) {
            if (k < n->kv.first) n = n->left.get();
            else if (n->kv.first < k) n = n->right.get();
            else return &n->kv.second;
        }
        return nullptr;
    }
    bool contains(const K& k) const { return get(k) != nullptr; }
    /// The value at `k`, to update in place (the path to it copied first, so that no map
    /// sharing it sees the update); null when `k` is not a key.
    V* mut_get(const K& k) {
        if (!contains(k)) return nullptr;
        V* at = nullptr;
        root_ = copy_to(root_, k, at);
        return at;
    }

    /// Set `k` to `v` in this value.
    void insert(K k, V v) { root_ = put(root_, k, v); }
    void remove(const K& k) {
        if (contains(k)) root_ = erase(root_, k);
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

    const_iterator begin() const { return const_iterator(root_.get()); }
    const_iterator end() const { return const_iterator(); }

    friend bool operator==(const Map& a, const Map& b) {
        return a.root_ == b.root_ || (a.size() == b.size() && std::equal(a.begin(), a.end(), b.begin()));
    }
    friend auto operator<=>(const Map& a, const Map& b) {
        return std::lexicographical_compare_three_way(a.begin(), a.end(), b.begin(), b.end());
    }

private:
    Ptr root_;
};

// ---- Array ------------------------------------------------------------------------------

/// `[[a]]`: a view (an offset and a length) of a buffer of fixed-size chunks that views
/// share. A `push_back` onto the view that ends where the buffer does appends in place;
/// any other update copies the view's elements into a buffer of its own first.
template <class T>
class Array {
    static constexpr std::size_t CHUNK = 64;
    struct Slot {
        alignas(T) unsigned char bytes[sizeof(T)];
    };
    struct Buffer {
        std::vector<std::unique_ptr<Slot[]>> chunks;
        std::size_t size = 0;
        Buffer() = default;
        Buffer(const Buffer&) = delete;
        Buffer& operator=(const Buffer&) = delete;
        ~Buffer() {
            for (std::size_t i = 0; i < size; ++i) at(i).~T();
        }
        T& at(std::size_t i) { return *std::launder(reinterpret_cast<T*>(chunks[i / CHUNK][i % CHUNK].bytes)); }
        void push(T x) {
            if (size % CHUNK == 0) chunks.push_back(std::make_unique<Slot[]>(CHUNK));
            new (chunks[size / CHUNK][size % CHUNK].bytes) T(std::move(x));
            ++size;
        }
    };

public:
    class const_iterator {
    public:
        using iterator_category = std::random_access_iterator_tag;
        using value_type = T;
        using difference_type = std::ptrdiff_t;
        using pointer = const T*;
        using reference = const T&;
        const_iterator() = default;
        const_iterator(const Array* a, std::size_t i) : a_(a), i_(i) {}
        reference operator*() const { return (*a_)[i_]; }
        pointer operator->() const { return &(*a_)[i_]; }
        const_iterator& operator++() {
            ++i_;
            return *this;
        }
        const_iterator operator++(int) {
            const_iterator t = *this;
            ++i_;
            return t;
        }
        const_iterator& operator--() {
            --i_;
            return *this;
        }
        const_iterator& operator+=(difference_type d) {
            i_ = std::size_t(difference_type(i_) + d);
            return *this;
        }
        const_iterator operator+(difference_type d) const { return const_iterator(a_, std::size_t(difference_type(i_) + d)); }
        difference_type operator-(const const_iterator& o) const { return difference_type(i_) - difference_type(o.i_); }
        reference operator[](difference_type d) const { return (*a_)[std::size_t(difference_type(i_) + d)]; }
        friend bool operator==(const const_iterator& a, const const_iterator& b) { return a.i_ == b.i_; }
        friend auto operator<=>(const const_iterator& a, const const_iterator& b) { return a.i_ <=> b.i_; }

    private:
        const Array* a_ = nullptr;
        std::size_t i_ = 0;
    };
    using iterator = const_iterator;

    Array() = default;
    Array(std::initializer_list<T> xs) {
        for (const T& x : xs) push_back(x);
    }
    explicit Array(std::vector<T> xs) {
        for (std::size_t i = 0; i < xs.size(); ++i) push_back(T(std::move(xs[i])));
    }

    std::size_t size() const { return len_; }
    bool empty() const { return len_ == 0; }
    /// The element at `i`, or null.
    const T* get(std::size_t i) const { return i < len_ ? &(*this)[i] : nullptr; }
    const T& operator[](std::size_t i) const { return b_->at(off_ + i); }

    void push_back(T x) {
        if (!b_ || off_ + len_ != b_->size) own();
        b_->push(std::move(x));
        ++len_;
    }
    void set(std::size_t i, T x) {
        if (i >= len_) throw std::out_of_range("Array::set");
        unique();
        b_->at(i) = std::move(x);
    }
    /// The element at `i` to update in place (this view's elements made its own first).
    T& mut_at(std::size_t i) {
        if (i >= len_) throw std::out_of_range("Array::mut_at");
        unique();
        return b_->at(i);
    }
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
        for (const T& x : o) a.push_back(x);
        return a;
    }
    /// The first `n` elements.
    Array take(std::size_t n) const {
        Array a = *this;
        a.len_ = std::min(n, len_);
        return a;
    }
    /// Everything after the first `n`.
    Array drop_front(std::size_t n) const {
        Array a = *this;
        std::size_t k = std::min(n, len_);
        a.off_ += k;
        a.len_ -= k;
        return a;
    }

    const_iterator begin() const { return const_iterator(this, 0); }
    const_iterator end() const { return const_iterator(this, len_); }

    friend bool operator==(const Array& a, const Array& b) {
        return a.len_ == b.len_ && (a.len_ == 0 || (a.b_ == b.b_ && a.off_ == b.off_) || std::equal(a.begin(), a.end(), b.begin()));
    }
    friend auto operator<=>(const Array& a, const Array& b) {
        return std::lexicographical_compare_three_way(a.begin(), a.end(), b.begin(), b.end());
    }

private:
    /// This view's elements in a buffer of their own.
    void own() {
        auto fresh = std::make_shared<Buffer>();
        for (std::size_t i = 0; i < len_; ++i) fresh->push((*this)[i]);
        b_ = std::move(fresh);
        off_ = 0;
    }
    /// The buffer this view's alone: no other view, nothing outside the view.
    void unique() {
        if (!b_ || b_.use_count() > 1 || off_ != 0 || len_ != b_->size) own();
    }
    std::shared_ptr<Buffer> b_;
    std::size_t off_ = 0, len_ = 0;
};

}  // namespace prism_rtl
