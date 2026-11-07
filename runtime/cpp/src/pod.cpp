// `Pod`, `Segment`, `Path` (pod.hpp).

#include "prism_rtl/pod.hpp"

#include <algorithm>
#include <bit>
#include <cstring>

#include "prism_rtl/print.hpp"

namespace prism_rtl {

std::strong_ordering operator<=>(F64 a, F64 b) {
    // IEEE 754's totalOrder, as Rust's `f64::total_cmp`: flip the magnitude bits of a
    // negative number, then compare as signed integers.
    auto key = [](double d) {
        std::int64_t x = std::bit_cast<std::int64_t>(d);
        return x ^ std::int64_t(std::uint64_t(x >> 63) >> 1);
    };
    return key(a.v) <=> key(b.v);
}

// ---- Segment and Path ----------------------------------------------------------------------

Segment Segment::key(Pod k) { return Segment(Kind::Key, 0, make_rc<const Pod>(std::move(k))); }

bool operator==(const Segment& a, const Segment& b) { return (a <=> b) == 0; }

std::strong_ordering operator<=>(const Segment& a, const Segment& b) {
    if (a.kind_ != b.kind_) return a.kind_ <=> b.kind_;
    if (a.kind_ == Segment::Kind::Key) return *a.key_ <=> *b.key_;
    return a.n_ <=> b.n_;
}

Path Path::child(Segment seg) const {
    Path p = *this;
    p.segs_.push_back(std::move(seg));
    return p;
}

Path Path::join(const Path& rest) const {
    Path p = *this;
    p.segs_.insert(p.segs_.end(), rest.segs_.begin(), rest.segs_.end());
    return p;
}

std::strong_ordering operator<=>(const Path& a, const Path& b) {
    return std::lexicographical_compare_three_way(a.segs_.begin(), a.segs_.end(), b.segs_.begin(), b.segs_.end());
}

std::string Path::to_string() const { return print_path(*this); }

// ---- Pod -----------------------------------------------------------------------------------

template <class A>
static std::strong_ordering seq(const A& a, const A& b) {
    return std::lexicographical_compare_three_way(a.begin(), a.end(), b.begin(), b.end());
}

std::strong_ordering operator<=>(const Pod& a, const Pod& b) {
    if (a.v_.index() != b.v_.index()) return a.v_.index() <=> b.v_.index();
    using K = Pod::Kind;
    switch (a.kind()) {
    case K::Unit:
    case K::Nil:
        return std::strong_ordering::equal;
    case K::Bool:
        return *a.if_bool() <=> *b.if_bool();
    case K::Int:
        return *a.if_int() <=> *b.if_int();
    case K::Float:
        return *a.if_float() <=> *b.if_float();
    case K::Char:
        return *a.if_char() <=> *b.if_char();
    case K::Text:
        return *a.if_text() <=> *b.if_text();
    case K::Duration:
        return *a.if_duration() <=> *b.if_duration();
    case K::Record:
    case K::Element: {
        const Pod::Named& x = a.kind() == K::Record ? *a.if_record() : *a.if_element();
        const Pod::Named& y = b.kind() == K::Record ? *b.if_record() : *b.if_element();
        if (auto c = x.name <=> y.name; c != 0) return c;
        return seq(x.fields, y.fields);
    }
    case K::Choice: {
        const Pod::Choice &x = *a.if_choice(), &y = *b.if_choice();
        if (auto c = x.ctor <=> y.ctor; c != 0) return c;
        return seq(x.args, y.args);
    }
    case K::Tuple:
        return seq(*a.if_tuple(), *b.if_tuple());
    case K::Array:
        return seq(*a.if_array(), *b.if_array());
    case K::Map:
        return seq(*a.if_map(), *b.if_map());
    case K::Some:
        return *a.if_some() <=> *b.if_some();
    case K::Ok:
        return *a.if_ok() <=> *b.if_ok();
    case K::Failed:
        return *a.if_failed() <=> *b.if_failed();
    case K::Path:
        return *a.if_path() <=> *b.if_path();
    case K::Shown:
        // The text takes no part: the structure decides.
        return *a.if_shown()->structure <=> *b.if_shown()->structure;
    }
    return std::strong_ordering::equal;
}

const Pod& Pod::structure() const {
    if (auto s = if_shown()) return s->structure->structure();
    return *this;
}

const Pod* Pod::field(std::string_view name) const {
    const Pod& s = structure();
    const Named* n = s.if_record() ? s.if_record() : s.if_element();
    if (!n) return nullptr;
    for (const auto& [f, v] : n->fields)
        if (f == name) return &v;
    return nullptr;
}

const std::string* Pod::tag() const {
    auto e = structure().if_element();
    return e ? &e->name : nullptr;
}

std::optional<double> Pod::as_f64() const {
    const Pod& s = structure();
    if (auto f = s.if_float()) return f->v;
    if (auto n = s.if_int())
        if (auto i = n->to_i64()) return double(*i);
    return std::nullopt;
}

void Pod::elements(std::vector<const Pod*>& out) const {
    const Pod& s = structure();
    switch (s.kind()) {
    case Kind::Element:
        out.push_back(this);
        for (const auto& [_, v] : s.if_element()->fields) v.elements(out);
        break;
    case Kind::Record:
        for (const auto& [_, v] : s.if_record()->fields) v.elements(out);
        break;
    case Kind::Choice:
        for (const Pod& v : s.if_choice()->args) v.elements(out);
        break;
    case Kind::Tuple:
        for (const Pod& v : *s.if_tuple()) v.elements(out);
        break;
    case Kind::Array:
        for (const Pod& v : *s.if_array()) v.elements(out);
        break;
    case Kind::Map:
        for (const auto& [_, v] : *s.if_map()) v.elements(out);
        break;
    case Kind::Some:
        s.if_some()->elements(out);
        break;
    case Kind::Ok:
        s.if_ok()->elements(out);
        break;
    default:
        break;
    }
}

std::string Pod::to_string() const { return show(*this); }

Fault mismatch_fault(std::string_view want, const Pod& got) {
    return Fault::answer("expected " + std::string(want) + ", got " + got.to_string());
}

}  // namespace prism_rtl
