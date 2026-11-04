// The zombie collector (gc.hpp).

#include "prism_rtl/gc.hpp"

#include <unordered_map>

#include "prism_rtl/cx.hpp"

namespace prism_rtl {

void Collector::watch(const std::vector<std::weak_ptr<Node>>& zombies) {
    watched_.insert(watched_.end(), zombies.begin(), zombies.end());
}

std::size_t Collector::watching() const {
    std::size_t n = 0;
    for (const auto& w : watched_) n += !w.expired();
    return n;
}

std::size_t Collector::collect() {
    std::erase_if(watched_, [](const std::weak_ptr<Node>& w) { return w.expired(); });
    std::vector<NodeRef> held;
    std::unordered_map<const Node*, std::size_t> index;
    for (const auto& w : watched_)
        if (auto n = w.lock(); n && index.emplace(n.get(), held.size()).second) held.push_back(std::move(n));

    // Reach, counting the references from inside.
    std::vector<std::size_t> inside(held.size(), 0);
    std::vector<std::vector<std::size_t>> edges;
    for (std::size_t i = 0; i < held.size(); ++i) {
        std::vector<std::size_t> out;
        NodeRef node = held[i];
        node->edges([&](const NodeRef& e) {
            auto [it, fresh] = index.emplace(e.get(), held.size());
            if (fresh) {
                held.push_back(e);
                inside.push_back(0);
            }
            inside[it->second] += 1;
            out.push_back(it->second);
        });
        edges.push_back(std::move(out));
    }

    // Keep what the outside holds, and everything it reaches. `held` holds one count of
    // each node itself.
    std::vector<bool> live(held.size(), false);
    std::vector<std::size_t> stack;
    for (std::size_t i = 0; i < held.size(); ++i)
        if (std::size_t(held[i].use_count()) - 1 > inside[i]) stack.push_back(i);
    while (!stack.empty()) {
        std::size_t i = stack.back();
        stack.pop_back();
        if (!live[i]) {
            live[i] = true;
            stack.insert(stack.end(), edges[i].begin(), edges[i].end());
        }
    }

    std::size_t garbage = 0;
    for (std::size_t i = 0; i < held.size(); ++i)
        if (!live[i]) {
            held[i]->release();
            ++garbage;
        }
    return garbage;
}

}  // namespace prism_rtl
