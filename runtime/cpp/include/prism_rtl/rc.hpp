// `Rc<T>` and `Weak<T>`: reference counting with plain (non-atomic) counts, the C++ twin
// of Rust's `std::rc::Rc` and `std::rc::Weak`, in place of `std::shared_ptr` and
// `std::weak_ptr`, whose counts are atomic. A program is single-threaded (claude/decisions.md,
// D4), and no World hands a counted value from one thread to another while both run
//, so nothing here is synchronized.
//
// - `make_rc<T>(args...)` makes the value and its counts in one allocation; `RcPlace<T>`
//   (and `rc_placed<T>` over it) does the same with a constructor the caller writes
//   (`new (place.at()) T{...}`), for a braced argument `make_rc` cannot forward, a `TRY`
//   among the arguments, or a constructor only the class can call.
// - `Rc<T>` from a `std::unique_ptr` takes the value over, its block apart from it.
// - `Rc<T>` is a pointer and the counts' block, as `std::shared_ptr` is: it converts to an
//   `Rc` of a base or of `const T`, and `rc_static_cast` casts it down.
// - `Weak<T>` holds the block, not the value: `lock()` is the value while a strong count
//   holds it, and the block goes when the last of either goes.
// - `RcFromThis<T>`, as `std::enable_shared_from_this`, gives an object made by `make_rc`
//   or `rc_placed` its own `Rc` (`rc_from_this()`); asking an object no `Rc` owns is a
//   panic.
// - `==` and `<=>` on `Rc<T>` are `T`'s, through the pointer, defined exactly when `T` has
//   them, as Rust's `Rc<T: PartialEq>` and `Rc<T: Ord>` are: an `Rc<const T>` of a value
//   (a recursive payload of a choice or record) compares, orders, prints and goes out as
//   plain data by the value, and a node or a cell, which has no `==`, has none through its
//   `Rc` either. Identity is `ptr_eq` (Rust's `Rc::ptr_eq`), always written out.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <compare>
#include <concepts>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <functional>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

namespace prism_rtl {

template <class T>
class Rc;
template <class T>
class Weak;
template <class T>
class RcFromThis;

namespace detail {

/// The counts' block. `weak` counts the weak references plus one for all the strong ones
/// together, so the block outlives the value while any `Weak` remains.
struct RcBlock {
    std::size_t strong = 1;
    std::size_t weak = 1;
    /// The value's destructor, when the last strong reference goes.
    virtual void destroy() noexcept = 0;
    /// The block itself, when the last reference of either kind goes.
    virtual void free() noexcept = 0;

protected:
    ~RcBlock() = default;
};

/// The value inside the block: one allocation.
template <class T>
struct RcInline final : RcBlock {
    alignas(T) unsigned char storage[sizeof(T)];
    T* value() noexcept { return std::launder(reinterpret_cast<T*>(storage)); }
    void destroy() noexcept override { value()->~T(); }
    void free() noexcept override { delete this; }
};

/// A value made elsewhere and handed over (a `std::unique_ptr`'s): the block apart from it.
template <class T>
struct RcAdopted final : RcBlock {
    T* p;
    explicit RcAdopted(T* p) noexcept : p(p) {}
    void destroy() noexcept override { delete p; }
    void free() noexcept override { delete this; }
};

[[noreturn]] inline void rc_fault(const char* what) {
    std::fprintf(stderr, "prism_rtl::Rc: %s\n", what);
    std::abort();
}

/// What `RcFromThis` keeps: the block of the `Rc` that made the object (no count of its
/// own: the object lives inside what the block counts). A copy of the object is not
/// made by an `Rc`, so it starts without one.
class RcFromThisBase {
public:
    mutable RcBlock* rc_block_ = nullptr;

protected:
    RcFromThisBase() = default;
    RcFromThisBase(const RcFromThisBase&) noexcept {}
    RcFromThisBase& operator=(const RcFromThisBase&) noexcept { return *this; }
    ~RcFromThisBase() = default;
};

template <class U>
void hook_from_this(U* p, RcBlock* b) noexcept {
    if constexpr (std::is_convertible_v<U*, const RcFromThisBase*>) {
        const RcFromThisBase* base = p;
        base->rc_block_ = b;
    }
}

struct RcAdoptTag {};

}  // namespace detail

/// A counted pointer: Rust's `Rc<T>`, with `std::shared_ptr`'s spelling.
template <class T>
class Rc {
public:
    using element_type = T;

    constexpr Rc() noexcept = default;
    constexpr Rc(std::nullptr_t) noexcept {}
    Rc(const Rc& o) noexcept : p_(o.p_), b_(o.b_) {
        if (b_) ++b_->strong;
    }
    Rc(Rc&& o) noexcept : p_(std::exchange(o.p_, nullptr)), b_(std::exchange(o.b_, nullptr)) {}
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Rc(const Rc<U>& o) noexcept : p_(o.p_), b_(o.b_) {
        if (b_) ++b_->strong;
    }
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Rc(Rc<U>&& o) noexcept : p_(std::exchange(o.p_, nullptr)), b_(std::exchange(o.b_, nullptr)) {}
    /// Ownership handed over from a `std::unique_ptr`: two allocations, the value's and the
    /// block's, as `std::shared_ptr` takes one.
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Rc(std::unique_ptr<U>&& u) {
        if (!u) return;
        b_ = new detail::RcAdopted<U>(u.get());
        U* raw = u.release();
        p_ = raw;
        detail::hook_from_this(raw, b_);
    }
    ~Rc() { release(); }

    Rc& operator=(const Rc& o) noexcept {
        Rc(o).swap(*this);
        return *this;
    }
    Rc& operator=(Rc&& o) noexcept {
        Rc(std::move(o)).swap(*this);
        return *this;
    }
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Rc& operator=(const Rc<U>& o) noexcept {
        Rc(o).swap(*this);
        return *this;
    }
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Rc& operator=(Rc<U>&& o) noexcept {
        Rc(std::move(o)).swap(*this);
        return *this;
    }
    Rc& operator=(std::nullptr_t) noexcept {
        reset();
        return *this;
    }

    void reset() noexcept { Rc().swap(*this); }
    void swap(Rc& o) noexcept {
        std::swap(p_, o.p_);
        std::swap(b_, o.b_);
    }

    T* get() const noexcept { return p_; }
    T& operator*() const noexcept { return *p_; }
    T* operator->() const noexcept { return p_; }
    explicit operator bool() const noexcept { return p_ != nullptr; }
    /// The strong count (0 for an empty `Rc`).
    long use_count() const noexcept { return b_ ? static_cast<long>(b_->strong) : 0; }

    friend bool operator==(const Rc& a, std::nullptr_t) noexcept { return a.p_ == nullptr; }

private:
    T* p_ = nullptr;
    detail::RcBlock* b_ = nullptr;

    /// An `Rc` over a block whose strong count the caller already counted for it.
    Rc(T* p, detail::RcBlock* b, detail::RcAdoptTag) noexcept : p_(p), b_(b) {}

    void release() noexcept {
        if (!b_) return;
        if (--b_->strong == 0) {
            b_->destroy();
            if (--b_->weak == 0) b_->free();
        }
    }

    template <class>
    friend class Rc;
    template <class>
    friend class Weak;
    template <class>
    friend class RcFromThis;
    template <class U, class... A>
    friend Rc<U> make_rc(A&&... args);
    template <class U>
    friend class RcPlace;
    template <class U, class V>
    friend Rc<U> rc_static_cast(const Rc<V>& r) noexcept;
};

/// Whether two `Rc`s point to the same allocation: Rust's `Rc::ptr_eq`, the one identity
/// comparison (an `Rc`'s `==` asks it first, then its value's, below).
template <class T, class U>
bool ptr_eq(const Rc<T>& a, const Rc<U>& b) noexcept {
    return static_cast<const void*>(a.get()) == static_cast<const void*>(b.get());
}

/// The value and its counts in one allocation: `std::make_shared`.
template <class T, class... A>
Rc<T> make_rc(A&&... args) {
    using V = std::remove_const_t<T>;
    auto* b = new detail::RcInline<V>;
    V* p;
    try {
        p = ::new (static_cast<void*>(b->storage)) V(std::forward<A>(args)...);
    } catch (...) {
        delete b;
        throw;
    }
    detail::hook_from_this(p, b);
    return Rc<T>(p, b, detail::RcAdoptTag{});
}

/// The block made first, its value placed in it after: `RcPlace<T> place;` then
/// `place.take(new (place.at()) T{...})`. The constructor's arguments are evaluated where
/// the caller writes them (so a `TRY` among them returns from the caller's function); a
/// place never taken frees its block.
template <class T>
class RcPlace {
    using V = std::remove_const_t<T>;

public:
    RcPlace() : b_(new detail::RcInline<V>) {}
    RcPlace(const RcPlace&) = delete;
    RcPlace& operator=(const RcPlace&) = delete;
    ~RcPlace() { delete b_; }
    /// Where the value goes.
    void* at() noexcept { return static_cast<void*>(b_->storage); }
    /// The value made at `at()`, counted.
    Rc<T> take(V* p) noexcept {
        if (!b_ || static_cast<void*>(p) != at()) detail::rc_fault("RcPlace::take of a value made elsewhere");
        detail::hook_from_this(p, b_);
        return Rc<T>(p, std::exchange(b_, nullptr), detail::RcAdoptTag{});
    }

private:
    detail::RcInline<V>* b_;
};

/// `make_rc` with the constructor the caller writes: `construct(at)` makes the value in
/// place (`new (at) T{...}`) and returns it.
template <class T, class F>
Rc<T> rc_placed(F&& construct) {
    RcPlace<T> place;
    return place.take(construct(place.at()));
}

/// `std::static_pointer_cast`: the same block, the pointer cast down.
template <class T, class U>
Rc<T> rc_static_cast(const Rc<U>& r) noexcept {
    if (r.b_) ++r.b_->strong;
    return Rc<T>(static_cast<T*>(r.p_), r.b_, detail::RcAdoptTag{});
}

/// A weak reference: Rust's `Weak<T>`, with `std::weak_ptr`'s spelling.
template <class T>
class Weak {
public:
    constexpr Weak() noexcept = default;
    Weak(const Weak& o) noexcept : p_(o.p_), b_(o.b_) {
        if (b_) ++b_->weak;
    }
    Weak(Weak&& o) noexcept : p_(std::exchange(o.p_, nullptr)), b_(std::exchange(o.b_, nullptr)) {}
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Weak(const Rc<U>& r) noexcept : p_(r.p_), b_(r.b_) {
        if (b_) ++b_->weak;
    }
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Weak(const Weak<U>& o) noexcept : p_(o.p_), b_(o.b_) {
        if (b_) ++b_->weak;
    }
    ~Weak() { release(); }

    Weak& operator=(const Weak& o) noexcept {
        Weak(o).swap(*this);
        return *this;
    }
    Weak& operator=(Weak&& o) noexcept {
        Weak(std::move(o)).swap(*this);
        return *this;
    }
    template <class U>
        requires std::is_convertible_v<U*, T*>
    Weak& operator=(const Rc<U>& r) noexcept {
        Weak(r).swap(*this);
        return *this;
    }

    void reset() noexcept { Weak().swap(*this); }
    void swap(Weak& o) noexcept {
        std::swap(p_, o.p_);
        std::swap(b_, o.b_);
    }
    /// Whether the value is gone (or there never was one).
    bool expired() const noexcept { return !b_ || b_->strong == 0; }
    /// The value, while a strong reference holds it; else an empty `Rc`.
    Rc<T> lock() const noexcept {
        if (expired()) return Rc<T>();
        ++b_->strong;
        return Rc<T>(p_, b_, detail::RcAdoptTag{});
    }

private:
    T* p_ = nullptr;
    detail::RcBlock* b_ = nullptr;

    void release() noexcept {
        if (b_ && --b_->weak == 0) b_->free();
    }

    template <class>
    friend class Weak;
};

/// `std::enable_shared_from_this`: an object `make_rc` or `rc_placed` made reaches its own
/// `Rc`.
template <class T>
class RcFromThis : public detail::RcFromThisBase {
public:
    Rc<T> rc_from_this() {
        auto* b = block();
        ++b->strong;
        return Rc<T>(static_cast<T*>(this), b, detail::RcAdoptTag{});
    }
    Rc<const T> rc_from_this() const {
        auto* b = block();
        ++b->strong;
        return Rc<const T>(static_cast<const T*>(this), b, detail::RcAdoptTag{});
    }

protected:
    RcFromThis() = default;
    RcFromThis(const RcFromThis&) noexcept = default;
    RcFromThis& operator=(const RcFromThis&) noexcept = default;
    ~RcFromThis() = default;

private:
    detail::RcBlock* block() const {
        auto* b = static_cast<const detail::RcFromThisBase*>(this)->rc_block_;
        if (!b || b->strong == 0) detail::rc_fault("rc_from_this of an object no Rc owns");
        return b;
    }
};

/// `Rc<T>`'s comparisons, its value's (Rust's `impl<T: PartialEq> PartialEq for Rc<T>`, and
/// `Ord` likewise): defined exactly when `T`'s are, so that sharing stays invisible
/// (Semantics ch. 11) and a node's or a cell's `Rc`, whose `T` has none, has none. `==`
/// compares the pointers first, as Rust's `Rc<T: Eq>` does: every `^Eq` is reflexive (a
/// `Float` is equal exactly when its bits are, Type System round 6 FB), so one value is
/// equal to itself without being read. `<=>` is `std::strong_ordering`, as the emitted
/// comparisons are throughout (Rust's `Ord` is total), and answers `equal` for one value
/// likewise. `T` may be incomplete where an `Rc<const T>` is declared as a member; it must
/// be complete where the `Rc`s are compared.
template <class T, class U>
    requires requires(const T& a, const U& b) {
        { a == b } -> std::convertible_to<bool>;
    }
bool operator==(const Rc<T>& a, const Rc<U>& b) {
    return ptr_eq(a, b) || *a == *b;
}
template <class T, class U>
    requires requires(const T& a, const U& b) {
        { a <=> b } -> std::same_as<std::strong_ordering>;
    }
std::strong_ordering operator<=>(const Rc<T>& a, const Rc<U>& b) {
    if (ptr_eq(a, b)) return std::strong_ordering::equal;
    return *a <=> *b;
}

}  // namespace prism_rtl
