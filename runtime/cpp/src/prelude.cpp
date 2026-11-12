// The prelude's two tags (prelude.hpp).

#include "prism_rtl/prelude.hpp"

#include <algorithm>
#include <cstdio>
#include <iterator>

namespace prism_rtl {

Path subtree_of(const Path& p) {
    auto segs = p.segments();
    std::size_t end = 0;
    for (std::size_t i = 0; i < segs.size(); ++i)
        if (segs[i].kind() != Segment::Kind::Pos) end = i + 1;
    return path_of(segs.subspan(0, end));
}

namespace {

bool under(const Path& p, const Path& q) {
    auto a = p.segments();
    auto b = q.segments();
    return b.size() <= a.size() && std::equal(b.begin(), b.end(), a.begin());
}

Path parent_of(const Path& p) {
    auto segs = p.segments();
    return path_of(segs.subspan(0, segs.empty() ? 0 : segs.size() - 1));
}

}  // namespace

const Pod* element_of(const Document& doc, const TickOut& out, const Path& path) {
    if (auto e = doc.get(path)) return e;
    for (const auto& [p, e] : out.patch.added)
        if (p == path) return &e;
    return nullptr;
}

bool Prelude::is_poisoned(const Path& p) const {
    return std::any_of(poisoned_.begin(), poisoned_.end(), [&](const Path& q) { return under(p, q); });
}

void Prelude::track(const Patch& patch) {
    for (const auto& [p, e] : patch.removed) exceptions_.erase(p);
    for (const auto& [p, e] : patch.added) {
        auto tag = e.tag();
        if (tag && *tag == "Prelude.Exceptions") exceptions_.insert(p);
    }
}

void Prelude::unwrap(const Path& path, const Pod& value) {
    const Pod& v = value.structure();
    if (auto x = v.if_ok()) {
        answers_.emplace_back(path, *x);
        return;
    }
    const Pod* error = v.if_failed();
    if (!error) {
        answers_.emplace_back(path, v);
        return;
    }
    Path sub = subtree_of(path);
    Pod kind = error->field("kind") ? *error->field("kind") : Pod::name("Prelude.Unknown");
    Pod message = error->field("message") ? *error->field("message") : Pod::text(Text(""));
    Pod exception = Pod::record("Exception", {{"kind", kind}, {"message", message}, {"spans", Pod::array({})}, {"path", Pod::some(Pod::path(sub))}});
    std::optional<Path> supervisor;
    for (const Path& p : exceptions_)
        if (!under(p, sub) && under(sub, parent_of(p)) && (!supervisor || p.len() > supervisor->len())) supervisor = p;
    poisoned_.push_back(sub);
    if (supervisor && !sub.is_empty()) {
        answers_.emplace_back(*supervisor, exception);
        return;
    }
    const Text* m = message.as_text();
    gave_up_.push_back("prism: the program gave up: " + kind.to_string() + ": " + (m ? m->to_string() : std::string()));
    if (!exit_) exit_ = EXIT_FAILED;
}

std::vector<std::pair<Path, Pod>> Prelude::take_answers() { return std::exchange(answers_, {}); }

void Prelude::give_back(std::vector<std::pair<Path, Pod>> answers) {
    answers_.insert(answers_.begin(), std::make_move_iterator(answers.begin()), std::make_move_iterator(answers.end()));
}

std::vector<std::string> Prelude::take_gave_up() { return std::exchange(gave_up_, {}); }

void Prelude::report_gave_up() {
    for (const std::string& line : take_gave_up()) std::fprintf(stderr, "%s\n", line.c_str());
}

void Prelude::filter(TickIn& input, const Document& doc) const {
    std::erase_if(input.answers, [&](const auto& a) { return is_poisoned(a.first) || !doc.get(a.first); });
}

}  // namespace prism_rtl
