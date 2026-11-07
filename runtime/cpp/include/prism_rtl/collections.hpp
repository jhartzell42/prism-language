// The collections Prism's types compile to: `Text`, `k ==> v` (`Map`), `[[a]]` (`Array`).
//
// Values are immutable and cheap to copy: a copy is a pointer copy. `insert`, `remove`,
// `push_back` and `set` update this value (the emitted side's `&` blocks, Compiler ch. 3,
// "Unique values"); `inserted`, `removed`, `pushed` and `updated` leave it alone and
// answer the new one. Neither ever changes another value that shares the contents.
//
// The costs are Semantics Appendix B's promises, each kept (runtime/cpp/README.md, "Arrays
// and maps: the costs"): a `Map` is a persistent AVL tree (an update copies the path to its
// key, logarithmic); an `Array` a persistent balanced tree of chunks with a tail (indexing
// and `concat` logarithmic, `push_back` amortised constant, any update copying one path
// and one chunk at most). A `Text` is a view of a shared UTF-8 buffer (a cursor's
// `before` and `after` are constant) and copies on concatenation.
//
// `==` on a map or an array answers at once for a node both sides share at the same place,
// when every value under it is known to equal itself: a node remembers that once asked
// (`same`), and an update forgets it on what it changes. A shared node holding a value
// equal to nothing (a `Float` NaN, IEEE equality: Type System ch. 2) stays unequal, as the
// structural comparison has it, so sharing stays invisible (Semantics ch. 11).

#pragma once

#include "prism_rtl/rc.hpp"
#include <algorithm>
#include <bit>
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
/// Where `s` stops being well-formed UTF-8 (as Rust's `str::from_utf8` reads it: no
/// overlong form, no surrogate, nothing past U+10FFFF), or none when it is well-formed.
std::optional<std::size_t> utf8_error_at(std::string_view s);

// ---- Text -------------------------------------------------------------------------------

/// `Text` (Semantics Appendix B, Text; round 4 TX): UTF-8, a byte range of a shared
/// buffer, ordered bytewise (which for UTF-8 is code-point order). Its range always starts
/// and ends on a character boundary. `Len` is bytes; a `Cursor`'s `before` and `after` are
/// views of the same buffer; concatenation copies. A text is made only from well-formed
/// UTF-8: anything else is a fatal error (a `Panic`) naming the byte, as Rust's `str` cannot
/// hold it; a World checks the bytes it reads first (the CLI World's `Fs.NotText`). A
/// program's literal (`Text::lit`) is its bytes where they are, with no buffer or count.
class Text {
public:
    Text() = default;
    /// A literal of the program: bytes in static storage, well-formed UTF-8 (prismc wrote
    /// them from its source), never copied.
    static Text lit(std::string_view static_utf8) {
        Text t;
        t.p_ = static_utf8.data();
        t.n_ = static_utf8.size();
        return t;
    }
    /// Bytes the caller made from well-formed pieces (texts, numbers, literals), so not
    /// checked again: interpolation's result, a concatenation.
    static Text of_utf8(std::string&& utf8);
    Text(const char* utf8) : Text(std::string_view(utf8)) {}
    Text(std::string_view utf8);
    Text(const std::string& utf8) : Text(std::string_view(utf8)) {}
    Text(std::string&& utf8);

    /// `Len`: the number of bytes.
    std::size_t len() const { return n_; }
    bool is_empty() const { return n_ == 0; }
    /// The bytes.
    std::string_view view() const { return std::string_view(p_, n_); }
    /// `Split`'s pieces: the bytes between each `sep` (not empty), each a view of this
    /// text's bytes, with no copy.
    std::vector<Text> split(std::string_view sep) const;
    /// Concatenation, as interpolation does it.
    Text concat(const Text& other) const;
    /// The characters, decoded.
    std::u32string chars() const { return utf8_decode(view()); }
    /// The text as UTF-8.
    std::string to_string() const { return std::string(view()); }

    friend bool operator==(const Text& a, const Text& b) { return a.view() == b.view(); }
    friend std::strong_ordering operator<=>(const Text& a, const Text& b) {
        return a.view().compare(b.view()) <=> 0;
    }
    friend Text operator+(const Text& a, const Text& b) { return a.concat(b); }

private:
    friend class Cursor;
    /// The bytes `a` to `b` of this text, on character boundaries: a view of the buffer.
    Text slice(std::size_t a, std::size_t b) const {
        Text t;
        t.s_ = s_;
        t.p_ = p_ + a;
        t.n_ = b - a;
        return t;
    }
    /// The buffer the bytes are in (none for a literal), and the bytes.
    Rc<const std::string> s_;
    const char* p_ = "";
    std::size_t n_ = 0;
};

inline std::ostream& operator<<(std::ostream& o, const Text& t) { return o << t.view(); }

/// `Cursor` (Semantics Appendix B, Text): a text and a byte position in it, always on a
/// character boundary. Opaque to programs; cursors compare by their position.
class Cursor {
public:
    Cursor() = default;
    /// `Front` and `Back`.
    static Cursor front(const Text& t) { return Cursor(t, 0); }
    static Cursor back(const Text& t) { return Cursor(t, t.len()); }
    /// `?CursorAt`: the cursor at byte `n`, none past the end or inside a character.
    static std::optional<Cursor> at_byte(const Text& t, std::size_t n);
    /// `At`: the byte index.
    std::size_t at() const { return at_; }
    /// The whole text the cursor is in (what `?Match` searches, seeing the bytes before
    /// the cursor).
    const Text& text() const { return text_; }
    /// `?Step`: the character at the cursor and the cursor past it, none at the end.
    std::optional<std::pair<char32_t, Cursor>> step() const;
    /// `?StepBack`: the character before the cursor and the cursor at its start, none at
    /// the start.
    std::optional<std::pair<char32_t, Cursor>> step_back() const;
    /// `Before` and `After`: views of the same bytes.
    Text before() const { return text_.slice(0, at_); }
    Text after() const { return text_.slice(at_, text_.len()); }

    friend bool operator==(const Cursor& a, const Cursor& b) { return a.at_ == b.at_; }
    friend std::strong_ordering operator<=>(const Cursor& a, const Cursor& b) { return a.at_ <=> b.at_; }

private:
    Cursor(Text t, std::size_t at) : text_(std::move(t)), at_(at) {}
    Text text_;
    std::size_t at_ = 0;
};

// ---- Map --------------------------------------------------------------------------------

namespace detail {
/// What a map's or an array's node knows of whether every value under it equals itself.
inline constexpr std::uint8_t UNKNOWN = 0, YES = 1, NO = 2;
}  // namespace detail

/// `k ==> v`: a persistent ordered map, an AVL tree. An update changes the nodes on the
/// path to its key in place where this map alone holds them, and copies a node another map
/// shares before changing it (`own`), so no map sharing it sees the update: the Rust
/// runtime's `Map` (runtime/rust/src/collections/map.rs, `Rc::make_mut`). A map built up by
/// inserts into one value, as a fold does, copies nothing.
template <class K, class V>
class Map {
public:
    using value_type = std::pair<const K, V>;

private:
    struct Node {
        value_type kv;
        Rc<Node> left, right;
        int height;
        std::size_t count;
        /// Whether every entry under this node equals itself, once asked (`self_equal`):
        /// `detail::UNKNOWN`, `YES` or `NO`. Forgotten when the node is changed (`own`).
        mutable std::uint8_t same = detail::UNKNOWN;
        /// Height and count from the children.
        void fix() {
            height = std::max(Map::height(left), Map::height(right)) + 1;
            count = Map::count(left) + Map::count(right) + 1;
        }
    };
    using Ptr = Rc<Node>;
    static bool self_equal(const Node* n) {
        if (n->same == detail::UNKNOWN)
            n->same = (!n->left || self_equal(n->left.get())) && (!n->right || self_equal(n->right.get())) && n->kv == n->kv
                          ? detail::YES
                          : detail::NO;
        return n->same == detail::YES;
    }
    /// The two trees' entries compared in key order, as `std::equal` over them would,
    /// except that a subtree both have at the same place in the order is one node: its
    /// entries compared with themselves, which `self_equal` answers from the nodes once
    /// known. Equal counts line the orders up, so a subtree is opened only while the other
    /// side's next item is not the same node: the larger of two, or a subtree against an
    /// entry. Each item is a node, whole (`entry` false) or its own entry alone (its right
    /// subtree below it on the stack).
    static bool equal_trees(const Node* a, const Node* b) {
        struct Item {
            const Node* n;
            bool entry;
        };
        // an in-order walk holds at most two items per level, and one more
        std::size_t room = 2 * std::size_t(std::max(a ? a->height : 0, b ? b->height : 0)) + 2;
        std::vector<Item> sa, sb;
        sa.reserve(room);
        sb.reserve(room);
        if (a) sa.push_back({a, false});
        if (b) sb.push_back({b, false});
        auto open = [](std::vector<Item>& s, const Node* n) {
            s.pop_back();
            if (n->right) s.push_back({n->right.get(), false});
            s.push_back({n, true});
            if (n->left) s.push_back({n->left.get(), false});
        };
        while (!sa.empty() && !sb.empty()) {
            Item x = sa.back(), y = sb.back();
            if (!x.entry && !y.entry) {
                if (x.n == y.n) {
                    if (!self_equal(x.n)) return false;
                    sa.pop_back();
                    sb.pop_back();
                } else if (x.n->count >= y.n->count) {
                    open(sa, x.n);
                } else {
                    open(sb, y.n);
                }
            } else if (x.entry && y.entry) {
                if (!(x.n->kv == y.n->kv)) return false;
                sa.pop_back();
                sb.pop_back();
            } else if (!x.entry) {
                open(sa, x.n);
            } else {
                open(sb, y.n);
            }
        }
        return sa.empty() && sb.empty();
    }
    static int height(const Ptr& n) { return n ? n->height : 0; }
    static std::size_t count(const Ptr& n) { return n ? n->count : 0; }
    static Ptr make(value_type kv, Ptr l, Ptr r) {
        int h = std::max(height(l), height(r)) + 1;
        std::size_t c = count(l) + count(r) + 1;
        return make_rc<Node>(Node{std::move(kv), std::move(l), std::move(r), h, c});
    }
    /// The node at `link`, this map's own (copied first when another map shares it), to be
    /// changed. Every change to a node goes through this, which forgets what the node knew
    /// of itself.
    static Node* own(Ptr& link) {
        if (link.use_count() > 1) link = make_rc<Node>(*link);
        link->same = detail::UNKNOWN;
        return link.get();
    }
    /// `link`'s left child up in its place.
    static void rotate_right(Ptr& link) {
        Node* n = own(link);
        Ptr l = std::move(n->left);
        Node* ln = own(l);
        n->left = std::move(ln->right);
        n->fix();
        ln->right = std::move(link);
        ln->fix();
        link = std::move(l);
    }
    /// `link`'s right child up in its place.
    static void rotate_left(Ptr& link) {
        Node* n = own(link);
        Ptr r = std::move(n->right);
        Node* rn = own(r);
        n->right = std::move(rn->left);
        n->fix();
        rn->left = std::move(link);
        rn->fix();
        link = std::move(r);
    }
    /// `link`'s node, its children already balanced and one of them changed, rebalanced.
    static void rebalance(Ptr& link) {
        Node* n = own(link);
        n->fix();
        int hl = height(n->left), hr = height(n->right);
        if (hl > hr + 1) {
            if (height(n->left->left) < height(n->left->right)) rotate_left(n->left);
            rotate_right(link);
        } else if (hr > hl + 1) {
            if (height(n->right->right) < height(n->right->left)) rotate_right(n->right);
            rotate_left(link);
        }
    }
    /// A balanced tree of `es[lo, hi)`, entries in ascending key order: one node each.
    static Ptr built(std::vector<std::pair<K, V>>& es, std::size_t lo, std::size_t hi) {
        if (lo == hi) return nullptr;
        std::size_t mid = lo + (hi - lo) / 2;
        Ptr l = built(es, lo, mid);
        Ptr r = built(es, mid + 1, hi);
        return make(value_type(std::move(es[mid].first), std::move(es[mid].second)), std::move(l), std::move(r));
    }
    static void put(Ptr& link, const K& k, const V& v) {
        if (!link) {
            link = make(value_type(k, v), nullptr, nullptr);
            return;
        }
        Node* n = own(link);
        if (k < n->kv.first) put(n->left, k, v);
        else if (n->kv.first < k) put(n->right, k, v);
        else {
            n->kv.second = v;
            return;
        }
        rebalance(link);
    }
    /// The least entry under `link` taken out, the tree rebalanced; `min` is its node.
    static void take_min(Ptr& link, Ptr& min) {
        Node* n = own(link);
        if (!n->left) {
            min = std::move(link);
            link = std::move(min->right);
            min->right = nullptr;
            return;
        }
        take_min(n->left, min);
        rebalance(link);
    }
    static void erase(Ptr& link, const K& k) {
        if (!link) return;
        Node* n = own(link);
        if (k < n->kv.first) erase(n->left, k);
        else if (n->kv.first < k) erase(n->right, k);
        else if (!n->left) {
            link = std::move(n->right);
            return;
        } else if (!n->right) {
            link = std::move(n->left);
            return;
        } else {
            // the least of the right subtree takes this node's place
            Ptr min;
            take_min(n->right, min);
            Node* m = own(min);
            m->left = std::move(n->left);
            m->right = std::move(n->right);
            link = std::move(min);
        }
        rebalance(link);
    }
    /// The value at `k` in this map's own node (the path owned), null when `k` is not a key.
    static V* own_at(Ptr& link, const K& k) {
        if (!link) return nullptr;
        if (k < link->kv.first) {
            if (!link->left || !contains_under(link->left.get(), k)) return nullptr;
            return own_at(own(link)->left, k);
        }
        if (link->kv.first < k) {
            if (!link->right || !contains_under(link->right.get(), k)) return nullptr;
            return own_at(own(link)->right, k);
        }
        return &own(link)->kv.second;
    }
    static bool contains_under(const Node* n, const K& k) {
        for (; n;) {
            if (k < n->kv.first) n = n->left.get();
            else if (n->kv.first < k) n = n->right.get();
            else return true;
        }
        return false;
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
    /// Whether the two are one map, the same root node (both empty included): then they
    /// are equal. No program asks this (Prism has no identity); a builder uses it to skip
    /// a source it already built from.
    bool same(const Map& o) const { return root_.get() == o.root_.get(); }
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
    /// The value at `k`, to update in place (the path to it this map's own, copied where
    /// another map shares it, so that no map sharing it sees the update); null when `k` is
    /// not a key.
    V* mut_get(const K& k) { return own_at(root_, k); }

    /// Set `k` to `v` in this value.
    void insert(K k, V v) { put(root_, k, v); }
    void remove(const K& k) { erase(root_, k); }
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
    /// `Union a, b` (`a.united(b)`): this map's entries over `b`'s. The smaller map's
    /// entries go into the larger: `min(n, m) log (n + m)`.
    Map united(const Map& b) const {
        if (b.size() <= size()) {
            Map m = *this;
            for (const auto& [k, v] : b)
                if (!m.contains(k)) m.insert(k, v);
            return m;
        }
        Map m = b;
        for (const auto& [k, v] : *this) m.insert(k, v);
        return m;
    }

    const_iterator begin() const { return const_iterator(root_.get()); }
    const_iterator end() const { return const_iterator(); }

    /// The map of entries given in strictly ascending key order, built balanced at once (a
    /// node per entry, no key compared): what a map of a map's entries, key by key, is.
    static Map from_sorted(std::vector<std::pair<K, V>> es) {
        Map m;
        m.root_ = built(es, 0, es.size());
        return m;
    }

    friend bool operator==(const Map& a, const Map& b) {
        if (a.size() != b.size()) return false;
        if (a.root_ && a.root_ == b.root_) return self_equal(a.root_.get());
        return equal_trees(a.root_.get(), b.root_.get());
    }
    friend auto operator<=>(const Map& a, const Map& b) {
        return std::lexicographical_compare_three_way(a.begin(), a.end(), b.begin(), b.end());
    }

private:
    Ptr root_;
};

// ---- Array ------------------------------------------------------------------------------

/// `[[a]]`: a persistent array, a balanced tree of chunks and a tail, the design of the
/// Rust runtime's `Array` (runtime/rust/src/collections/array.rs).
///
/// - **The tail** is a chunk of the last elements, at most 64, in a vector that grows 1, 2,
///   4, ..., 64 slots while the array is small (64 at once in an array of 64 or more). A
///   push lands there: in place when the array holds the tail alone, else after copying it
///   (64 elements at most). A full tail goes into the tree whole, no element copied.
/// - **The tree's** leaves are chunks of 1 to 64 elements, and above them nodes of two
///   children, each with its length and height, balanced as an AVL tree is. Two trees are
///   concatenated by the AVL join, a tree is cut at an index by joining the pieces along
///   the path to it, and a single chunk that fits beside its neighbour merges into it.
/// - **Copies** share every node; an update copies the nodes on the path to what it
///   changes that another array shares (`own`), so no array sharing them sees it.
///
/// Costs (runtime/cpp/README.md, "Arrays and maps: the costs"): `size` constant; indexing
/// log (constant in the tail); `push_back` amortised constant; `concat` log; `take`,
/// `drop_front`, `set`, `mut_at` log, copying one path and one chunk at most; iteration
/// linear. A reference from `mut_at` or `[]` holds until the array is next pushed onto or
/// copied for a write elsewhere: a push may move the tail's elements as its vector grows.
template <class T>
class Array {
public:
    /// A chunk's slot: the element itself (a `std::vector<Elem>` is a plain vector, where a
    /// `std::vector<bool>` would pack bits and give no `bool&`).
    struct Elem {
        T v;
        Elem(const T& x) : v(x) {}
        Elem(T&& x) : v(std::move(x)) {}
    };

private:
    static constexpr std::size_t CHUNK = 64;
    /// A chunk when `left` is null (`elems`), else a join of two (`len`, `height`).
    struct Node {
        std::vector<Elem> elems;
        Rc<Node> left, right;
        std::size_t len = 0;
        unsigned char height = 0;
        /// Whether every element under this node equals itself, once asked (`self_equal`):
        /// `detail::UNKNOWN`, `YES` or `NO`. Every change to a node goes through `own`, or
        /// the in-place push onto a tail this array alone holds, and each forgets it.
        mutable std::uint8_t same = detail::UNKNOWN;
        std::size_t size() const { return left ? len : elems.size(); }
    };
    using Ptr = Rc<Node>;

    static bool self_equal(const Node* n) {
        if (n->same == detail::UNKNOWN) {
            bool r = n->left ? self_equal(n->left.get()) && self_equal(n->right.get())
                             : std::all_of(n->elems.begin(), n->elems.end(), [](const Elem& e) { return e.v == e.v; });
            n->same = r ? detail::YES : detail::NO;
        }
        return n->same == detail::YES;
    }
    /// The two arrays' elements compared in order, as `std::equal` would, except that a
    /// node both have at the same index is compared with itself, which `self_equal`
    /// answers from the node once known. Equal lengths line the indices up, so a node is
    /// opened only while the other side's next item is not the same node: the longer of
    /// two, or a node against a run; two runs compare the elements they overlap by. Each
    /// item is a node (`run` null) or the rest of a chunk.
    static bool equal_arrays(const Array& a, const Array& b) {
        struct Item {
            const Node* n;
            const Elem* run;
            std::size_t len;
        };
        auto start = [](const Array& x) {
            std::vector<Item> s;
            if (x.tail_ && !x.tail_->elems.empty()) s.push_back({x.tail_.get(), nullptr, x.tail_->elems.size()});
            if (x.tree_) s.push_back({x.tree_.get(), nullptr, x.tree_->size()});
            return s;
        };
        auto open = [](std::vector<Item>& s, const Node* n) {
            s.pop_back();
            if (n->left) {
                s.push_back({n->right.get(), nullptr, n->right->size()});
                s.push_back({n->left.get(), nullptr, n->left->size()});
            } else {
                s.push_back({nullptr, n->elems.data(), n->elems.size()});
            }
        };
        std::vector<Item> sa = start(a), sb = start(b);
        while (!sa.empty() && !sb.empty()) {
            Item& x = sa.back();
            Item& y = sb.back();
            if (x.n && y.n) {
                if (x.n == y.n) {
                    if (!self_equal(x.n)) return false;
                    sa.pop_back();
                    sb.pop_back();
                } else if (x.len >= y.len) {
                    open(sa, x.n);
                } else {
                    open(sb, y.n);
                }
            } else if (!x.n && !y.n) {
                std::size_t m = std::min(x.len, y.len);
                for (std::size_t i = 0; i < m; ++i)
                    if (!(x.run[i].v == y.run[i].v)) return false;
                x.run += m, x.len -= m, y.run += m, y.len -= m;
                bool xe = x.len == 0, ye = y.len == 0;
                if (xe) sa.pop_back();
                if (ye) sb.pop_back();
            } else if (x.n) {
                open(sa, x.n);
            } else {
                open(sb, y.n);
            }
        }
        return sa.empty() && sb.empty();
    }

    static Ptr chunk(std::vector<Elem>&& v) { return make_rc<Node>(Node{std::move(v), nullptr, nullptr, 0, 0}); }
    /// Two siblings whose heights differ by one at most, under a node.
    static Ptr join(Ptr l, Ptr r) {
        std::size_t len = l->size() + r->size();
        unsigned char h = static_cast<unsigned char>(std::max(l->height, r->height) + 1);
        return make_rc<Node>(Node{{}, std::move(l), std::move(r), len, h});
    }
    /// Two siblings whose heights differ by two at most, rotated to balance.
    static Ptr balance(Ptr l, Ptr r) {
        if (l->height > r->height + 1) {
            Ptr ll = l->left, lr = l->right;
            if (ll->height >= lr->height) return join(std::move(ll), join(std::move(lr), std::move(r)));
            return join(join(std::move(ll), lr->left), join(lr->right, std::move(r)));
        }
        if (r->height > l->height + 1) {
            Ptr rl = r->left, rr = r->right;
            if (rr->height >= rl->height) return join(join(std::move(l), std::move(rl)), std::move(rr));
            return join(join(std::move(l), rl->left), join(rl->right, std::move(rr)));
        }
        return join(std::move(l), std::move(r));
    }
    /// `l`'s elements, then `r`'s: the AVL join, log in the taller's height.
    static Ptr concat(Ptr l, Ptr r) {
        if (l->height > r->height + 1) return balance(l->left, concat(l->right, std::move(r)));
        if (r->height > l->height + 1) return balance(concat(std::move(l), r->left), r->right);
        return join(std::move(l), std::move(r));
    }
    /// The node at `p` this array's own: copied first when another array shares it (a
    /// chunk's elements copied, with room for `room` more).
    static Node& own(Ptr& p, std::size_t room = 0) {
        if (p.use_count() > 1) {
            if (p->left) {
                p = make_rc<Node>(Node{{}, p->left, p->right, p->len, p->height});
            } else {
                std::vector<Elem> v;
                v.reserve(p->elems.size() + room);
                v.insert(v.end(), p->elems.begin(), p->elems.end());
                p = chunk(std::move(v));
            }
        }
        p->same = detail::UNKNOWN;  // about to change: a copy knows nothing yet, nor does a node changed in place
        return *p;
    }
    static std::size_t last_size(const Node* n) {
        while (n->left) n = n->right.get();
        return n->elems.size();
    }
    static std::size_t first_size(const Node* n) {
        while (n->left) n = n->left.get();
        return n->elems.size();
    }
    /// `extra` at the end of the last chunk, which has room for it.
    static void extend_last(Ptr& p, const std::vector<Elem>& extra) {
        Node& n = own(p, extra.size());
        if (n.left) {
            extend_last(n.right, extra);
            n.len += extra.size();
            return;
        }
        n.elems.reserve(n.elems.size() + extra.size());
        n.elems.insert(n.elems.end(), extra.begin(), extra.end());
    }
    /// `extra` at the start of the first chunk, which has room for it.
    static void extend_first(Ptr& p, const std::vector<Elem>& extra) {
        Node& n = own(p);
        if (n.left) {
            extend_first(n.left, extra);
            n.len += extra.size();
            return;
        }
        std::vector<Elem> v;
        v.reserve(extra.size() + n.elems.size());
        v.insert(v.end(), extra.begin(), extra.end());
        v.insert(v.end(), std::make_move_iterator(n.elems.begin()), std::make_move_iterator(n.elems.end()));
        n.elems = std::move(v);
    }
    /// `l`'s elements, then `r`'s, a single chunk merged into its neighbour where it fits.
    /// The trees are values; `l` and `r` are left as they were if anything throws.
    static Ptr seam(Ptr l, Ptr r) {
        if (!r->left && last_size(l.get()) + r->elems.size() <= CHUNK) {
            extend_last(l, r->elems);
            return l;
        }
        if (!l->left && first_size(r.get()) + l->elems.size() <= CHUNK) {
            extend_first(r, l->elems);
            return r;
        }
        return concat(std::move(l), std::move(r));
    }
    /// The first `k` elements, `0 < k < size`.
    static Ptr take_tree(const Ptr& n, std::size_t k) {
        if (!n->left) return chunk(std::vector<Elem>(n->elems.begin(), n->elems.begin() + std::ptrdiff_t(k)));
        std::size_t ll = n->left->size();
        if (k == ll) return n->left;
        if (k < ll) return take_tree(n->left, k);
        return concat(n->left, take_tree(n->right, k - ll));
    }
    /// Everything after the first `k` elements, `0 < k < size`.
    static Ptr drop_tree(const Ptr& n, std::size_t k) {
        if (!n->left) return chunk(std::vector<Elem>(n->elems.begin() + std::ptrdiff_t(k), n->elems.end()));
        std::size_t ll = n->left->size();
        if (k == ll) return n->right;
        if (k > ll) return drop_tree(n->right, k - ll);
        return concat(drop_tree(n->left, k), n->right);
    }
    static T& at_mut(Ptr& p, std::size_t i) {
        Node& n = own(p);
        if (!n.left) return n.elems[i].v;
        std::size_t ll = n.left->size();
        return i < ll ? at_mut(n.left, i) : at_mut(n.right, i - ll);
    }
    /// A tree of `cs[lo, hi)`, balanced (siblings' sizes differ by one at most).
    static Ptr balanced(const std::vector<Ptr>& cs, std::size_t lo, std::size_t hi) {
        if (hi - lo == 1) return cs[lo];
        std::size_t mid = lo + (hi - lo) / 2;
        return join(balanced(cs, lo, mid), balanced(cs, mid, hi));
    }
    static std::size_t slots(const Node* n) {
        return n->left ? slots(n->left.get()) + slots(n->right.get()) : n->elems.capacity();
    }
    static void chunks_mut(Ptr& p, std::vector<std::pair<Elem*, std::size_t>>& out) {
        Node& n = own(p);
        if (n.left) {
            chunks_mut(n.left, out);
            chunks_mut(n.right, out);
        } else if (!n.elems.empty()) {
            out.emplace_back(n.elems.data(), n.elems.size());
        }
    }

    std::size_t tail_size() const { return tail_ ? tail_->elems.size() : 0; }
    /// The slots a tail of `have` elements grows to: doubling while the array is small, a
    /// whole chunk once it is not.
    std::size_t grown(std::size_t have) const {
        return len_ >= CHUNK ? CHUNK : std::min(CHUNK, std::max<std::size_t>(1, 2 * have));
    }

public:
    /// In order, chunk by chunk: the chunk being read, and the index the next one starts
    /// at, found from the root when the chunk ends (log, once per chunk of 64), so an
    /// iterator allocates nothing.
    class const_iterator {
    public:
        using iterator_category = std::forward_iterator_tag;
        using value_type = T;
        using difference_type = std::ptrdiff_t;
        using pointer = const T*;
        using reference = const T&;
        const_iterator() = default;
        reference operator*() const { return at_->v; }
        pointer operator->() const { return &at_->v; }
        const_iterator& operator++() {
            ++at_;
            --left_;
            if (at_ == stop_ && left_ > 0) refill();
            return *this;
        }
        const_iterator operator++(int) {
            const_iterator t = *this;
            ++*this;
            return t;
        }
        friend bool operator==(const const_iterator& a, const const_iterator& b) { return a.left_ == b.left_; }

    private:
        friend class Array;
        explicit const_iterator(const Array& a) : a_(&a), left_(a.len_) {
            if (left_ > 0) refill();
        }
        /// The chunk holding element `next_`, read from there.
        void refill() {
            std::size_t i = next_, in_tree = a_->len_ - a_->tail_size();
            const Node* n;
            if (i >= in_tree) {
                n = a_->tail_.get();
                i -= in_tree;
            } else {
                n = a_->tree_.get();
                while (n->left) {
                    std::size_t ll = n->left->size();
                    if (i < ll) {
                        n = n->left.get();
                    } else {
                        i -= ll;
                        n = n->right.get();
                    }
                }
            }
            at_ = n->elems.data() + i;
            stop_ = n->elems.data() + n->elems.size();
            next_ += std::size_t(stop_ - at_);
        }
        const Array* a_ = nullptr;
        const Elem* at_ = nullptr;
        const Elem* stop_ = nullptr;
        std::size_t left_ = 0, next_ = 0;
    };
    using iterator = const_iterator;

    Array() = default;
    Array(std::initializer_list<T> xs) : Array(std::vector<T>(xs)) {}
    /// Chunks of 64 moved out of the vector, and the rest as the tail: each element moved
    /// once, into storage of exactly its count.
    explicit Array(std::vector<T> xs) {
        std::size_t n = xs.size();
        if (n == 0) return;
        std::size_t full = (n - 1) / CHUNK;
        auto from = [&](std::size_t lo, std::size_t hi) {
            std::vector<Elem> v;
            v.reserve(hi - lo);
            for (std::size_t i = lo; i < hi; ++i) v.emplace_back(std::move(xs[i]));
            return chunk(std::move(v));
        };
        if (full > 0) {
            std::vector<Ptr> cs;
            cs.reserve(full);
            for (std::size_t c = 0; c < full; ++c) cs.push_back(from(c * CHUNK, (c + 1) * CHUNK));
            tree_ = balanced(cs, 0, full);
        }
        tail_ = from(full * CHUNK, n);
        len_ = n;
    }

    std::size_t size() const { return len_; }
    bool empty() const { return len_ == 0; }
    /// The element at `i`, or null.
    const T* get(std::size_t i) const {
        if (i >= len_) return nullptr;
        std::size_t in_tree = len_ - tail_size();
        if (i >= in_tree) return &tail_->elems[i - in_tree].v;
        const Node* n = tree_.get();
        while (n->left) {
            std::size_t ll = n->left->size();
            if (i < ll) {
                n = n->left.get();
            } else {
                i -= ll;
                n = n->right.get();
            }
        }
        return &n->elems[i].v;
    }
    const T& operator[](std::size_t i) const { return *get(i); }
    /// The slots of storage this array's chunks and tail hold (shared with the arrays
    /// sharing them): for the tests and for measurement.
    std::size_t capacity() const { return (tree_ ? slots(tree_.get()) : 0) + (tail_ ? tail_->elems.capacity() : 0); }

    void push_back(T x) {
        std::size_t have = tail_size();
        if (have == CHUNK) {
            // The full tail joins the tree as it is; a fresh tail takes `x`. Everything
            // that can throw (the allocations, `x`'s move) happens before the array changes.
            Ptr fresh = chunk({});
            fresh->elems.reserve(CHUNK);
            fresh->elems.emplace_back(std::move(x));
            Ptr tree = tree_ ? concat(tree_, tail_) : tail_;
            tree_ = std::move(tree);
            tail_ = std::move(fresh);
        } else if (!tail_ || tail_.use_count() > 1) {
            std::vector<Elem> v;
            v.reserve(std::max(have + 1, grown(have)));
            if (tail_) v.insert(v.end(), tail_->elems.begin(), tail_->elems.end());
            v.emplace_back(std::move(x));
            tail_ = chunk(std::move(v));
        } else {
            tail_->same = detail::UNKNOWN;
            std::vector<Elem>& v = tail_->elems;
            if (v.size() == v.capacity()) v.reserve(std::max(have + 1, grown(have)));
            v.emplace_back(std::move(x));
        }
        ++len_;
    }
    void set(std::size_t i, T x) { mut_at(i) = std::move(x); }
    /// The element at `i` to update in place (the path to it this array's own first).
    T& mut_at(std::size_t i) {
        if (i >= len_) throw std::out_of_range("Array::mut_at");
        std::size_t in_tree = len_ - tail_size();
        if (i >= in_tree) return own(tail_).elems[i - in_tree].v;
        return at_mut(tree_, i);
    }
    /// Every element to update in place, chunk by chunk (every node this array's own
    /// first): `iter_mut`, linear.
    std::vector<std::pair<Elem*, std::size_t>> chunks_mut() {
        std::vector<std::pair<Elem*, std::size_t>> out;
        if (tree_) chunks_mut(tree_, out);
        if (tail_) chunks_mut(tail_, out);
        return out;
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
    /// This array's elements, then `o`'s: log, copying a chunk or two at most.
    Array concat(const Array& o) const {
        if (o.len_ == 0) return *this;
        if (len_ == 0) return o;
        Array a;
        a.len_ = len_ + o.len_;
        if (!o.tree_ && tail_size() + o.tail_size() <= CHUNK) {
            std::vector<Elem> v;
            v.reserve(tail_size() + o.tail_size());
            if (tail_) v.insert(v.end(), tail_->elems.begin(), tail_->elems.end());
            v.insert(v.end(), o.tail_->elems.begin(), o.tail_->elems.end());
            a.tree_ = tree_;
            a.tail_ = chunk(std::move(v));
            return a;
        }
        Ptr left = tree_;
        if (tail_) left = left ? seam(std::move(left), tail_) : tail_;
        a.tree_ = !left ? o.tree_ : !o.tree_ ? std::move(left) : seam(std::move(left), o.tree_);
        a.tail_ = o.tail_;
        return a;
    }
    /// The first `n` elements (all of them when there are fewer).
    Array take(std::size_t n) const {
        if (n >= len_) return *this;
        if (n == 0) return Array();
        Array a;
        a.len_ = n;
        std::size_t in_tree = len_ - tail_size();
        if (n >= in_tree) {
            a.tree_ = tree_;
            if (n > in_tree) a.tail_ = chunk(std::vector<Elem>(tail_->elems.begin(), tail_->elems.begin() + std::ptrdiff_t(n - in_tree)));
        } else {
            a.tree_ = take_tree(tree_, n);
        }
        return a;
    }
    /// Everything after the first `n`.
    Array drop_front(std::size_t n) const {
        if (n == 0) return *this;
        if (n >= len_) return Array();
        Array a;
        a.len_ = len_ - n;
        std::size_t in_tree = len_ - tail_size();
        if (n >= in_tree) {
            a.tail_ = chunk(std::vector<Elem>(tail_->elems.begin() + std::ptrdiff_t(n - in_tree), tail_->elems.end()));
        } else {
            a.tree_ = drop_tree(tree_, n);
            a.tail_ = tail_;
        }
        return a;
    }

    const_iterator begin() const { return const_iterator(*this); }
    const_iterator end() const { return const_iterator(); }

    friend bool operator==(const Array& a, const Array& b) {
        if (a.len_ != b.len_) return false;
        // An array of a chunk or less is walked as it is.
        if (a.len_ <= CHUNK) return std::equal(a.begin(), a.end(), b.begin(), [](const T& x, const T& y) { return x == y; });
        return equal_arrays(a, b);
    }
    friend auto operator<=>(const Array& a, const Array& b) {
        return std::lexicographical_compare_three_way(a.begin(), a.end(), b.begin(), b.end());
    }

private:
    Ptr tree_, tail_;
    std::size_t len_ = 0;
};

}  // namespace prism_rtl
