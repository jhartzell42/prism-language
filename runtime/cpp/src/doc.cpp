// The document and the patch diff (doc.hpp).

#include "prism_rtl/doc.hpp"

#include <algorithm>
#include <set>

namespace prism_rtl {

std::string Request::to_string() const {
    if (field) return path.to_string() + ": " + *field + "=" + value.to_string();
    return path.to_string() + ": " + value.to_string();
}

Patch Document::diff(std::vector<Edit> edits) {
    Patch patch;
    std::map<Path, Pod> changed;
    std::set<Path> born;
    for (Edit& edit : edits) {
        switch (edit.kind) {
        case Edit::Kind::Removed: {
            auto it = m_.find(edit.path);
            if (it != m_.end()) {
                changed.erase(edit.path);
                patch.removed.emplace_back(edit.path, std::move(it->second));
                m_.erase(it);
            }
            break;
        }
        case Edit::Kind::Added:
            born.insert(edit.path);
            m_.insert_or_assign(edit.path, edit.level);
            patch.added.emplace_back(edit.path, edit.level);
            break;
        case Edit::Kind::Changed: {
            if (born.contains(edit.path)) {
                // Built this tick and moved again before the tick's end (a host's owner
                // patched after its child): it goes out as it stands.
                for (auto it = patch.added.rbegin(); it != patch.added.rend(); ++it)
                    if (it->first == edit.path) {
                        it->second = edit.level;
                        break;
                    }
                m_.insert_or_assign(edit.path, edit.level);
                break;
            }
            auto it = m_.find(edit.path);
            if (it == m_.end() || it->second == edit.level) break;
            it->second = edit.level;
            changed.insert_or_assign(edit.path, edit.level);
            break;
        }
        }
    }
    patch.changed.assign(changed.begin(), changed.end());
    auto by_path = [](const auto& a, const auto& b) { return a.first < b.first; };
    std::stable_sort(patch.added.begin(), patch.added.end(), by_path);
    std::stable_sort(patch.removed.begin(), patch.removed.end(), by_path);
    return patch;
}

void Document::apply(const Patch& patch) {
    for (const auto& [p, _] : patch.removed) m_.erase(p);
    for (const auto& [p, e] : patch.added) m_.insert_or_assign(p, e);
    for (const auto& [p, e] : patch.changed) m_.insert_or_assign(p, e);
}

const Pod* Document::get(const Path& path) const {
    auto it = m_.find(path);
    return it == m_.end() ? nullptr : &it->second;
}

std::vector<const Pod*> Document::elements() const {
    std::vector<const Pod*> out;
    for (const auto& [_, l] : m_) l.elements(out);
    return out;
}

}  // namespace prism_rtl
