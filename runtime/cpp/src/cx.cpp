// The tick's skeleton (cx.hpp).

#include "prism_rtl/cx.hpp"

#include <algorithm>

namespace prism_rtl {

Done Node::answer(std::span<const Segment> at, const Pod&, Cx&) {
    return unexpected(Fault::answer("no place at " + path_of(at).to_string()));
}

namespace detail {

void List::add(NodeRef n) {
    const Node* key = n.get();
    auto it = seen.find(key);
    if (it != seen.end() && it->second >= walked) return;
    seen[key] = nodes.size();
    nodes.push_back(std::move(n));
}

/// Walk a list, including what is added to it while walking.
static Done walk(Gathered& g, List Gathered::* pick, Done (Node::*f)(Cx&), Cx& cx) {
    std::size_t i = (g.*pick).walked;
    while (i < (g.*pick).nodes.size()) {
        NodeRef n = (g.*pick).nodes[i];
        (g.*pick).walked = i + 1;
        TRY(((*n).*f)(cx));
        ++i;
    }
    return {};
}

}  // namespace detail

void Cx::wake(std::uint32_t height, NodeRef node) {
    std::size_t h = height_ ? std::max(height, *height_ + 1) : height;
    if (g_.heights.size() <= h) g_.heights.resize(h + 1);
    g_.heights[h].add(std::move(node));
}

void Cx::clean_up_later(NodeRef node, bool zombie) {
    if (zombie) g_.zombies.push_back(node);
    g_.cleanups.push_back(std::move(node));
}

std::optional<Pod> Cx::first_value(const Pod& element) const {
    for (auto it = pushes_.rbegin(); it != pushes_.rend(); ++it)
        if (it->first == element) return it->second;
    return ready_->first_value(element);
}

Cx Runtime::build(const ReadyState& ready) { return Cx(0, 1, Cx::Phase::Constructing, ready, {}); }

Result<TickOut> Runtime::tick(const NodeRef& root, const TickIn& input, const ReadyState& ready) {
    for (auto& n : std::exchange(cleanups_, {})) n->clean_up();
    Tick n = tick_ + 1;
    Cx cx(n, n, Cx::Phase::Propagating, ready, input.pushes);
    for (auto& node : std::exchange(started_, {})) TRY(node->started(cx));
    for (auto& node : std::exchange(armed_, {})) TRY(node->deferred(cx));
    for (auto& deliver : std::exchange(later_, {})) TRY(deliver(cx));
    for (const auto& [path, v] : input.answers) TRY(root->answer(path.segments(), v, cx));
    for (const auto& [element, v] : input.pushes) {
        std::vector<NodeRef> live;
        if (auto it = listeners_.find(element); it != listeners_.end())
            for (const auto& w : it->second)
                if (auto node = w.lock()) live.push_back(std::move(node));
        for (auto& node : live) TRY(node->push(element, v, cx));
    }
    return close(std::move(cx));
}

Result<TickOut> Runtime::close(Cx&& cx) {
    using detail::Gathered;
    for (std::size_t h = 0; h < cx.g_.heights.size(); ++h) {
        cx.height_ = std::uint32_t(h);
        for (std::size_t i = 0; i < cx.g_.heights[h].nodes.size(); ++i) {
            NodeRef node = cx.g_.heights[h].nodes[i];
            TRY(node->wake(cx));
        }
    }
    cx.height_.reset();
    cx.read_at_ = cx.tick_ + 1;
    cx.phase_ = Cx::Phase::Constructing;
    // A walk can list nodes for an earlier one (a builder's new row is patched; a row
    // built in the patch pass is built): the walks run again until nothing new is listed.
    for (;;) {
        TRY(detail::walk(cx.g_, &Gathered::builds, &Node::build, cx));
        TRY(detail::walk(cx.g_, &Gathered::swaps, &Node::swap, cx));
        TRY(detail::walk(cx.g_, &Gathered::leaves, &Node::leave, cx));
        TRY(detail::walk(cx.g_, &Gathered::patches, &Node::patch, cx));
        const Gathered& g = cx.g_;
        auto done = [](const detail::List& l) { return l.walked >= l.nodes.size(); };
        if (done(g.builds) && done(g.swaps) && done(g.leaves) && done(g.patches)) break;
    }

    if (cx.late_) return unexpected(*cx.late_);
    Gathered g = std::move(cx.g_);
    if (g.fault) return unexpected(*g.fault);
    std::stable_sort(g.requests.begin(), g.requests.end(), [](const Request& a, const Request& b) { return a.path < b.path; });
    for (auto& [element, node] : g.listens) listeners_[element].push_back(node);
    for (auto it = listeners_.begin(); it != listeners_.end();) {
        auto& ns = it->second;
        std::erase_if(ns, [](const std::weak_ptr<Node>& w) { return w.expired(); });
        it = ns.empty() ? listeners_.erase(it) : std::next(it);
    }
    collector_.watch(g.zombies);
    cleanups_.insert(cleanups_.end(), g.cleanups.begin(), g.cleanups.end());
    started_ = std::move(g.started.nodes);
    armed_ = std::move(g.armed.nodes);
    later_ = std::move(g.later);
    tick_ = cx.tick_;
    TickOut out;
    out.tick = cx.tick_;
    out.patch = document_.diff(std::move(g.edits));
    out.requests = std::move(g.requests);
    out.result = std::move(g.result);
    out.pending = !started_.empty() || !armed_.empty() || !later_.empty();
    return out;
}

std::size_t Runtime::compact() {
    for (auto& n : std::exchange(cleanups_, {})) n->clean_up();
    return collector_.collect();
}

}  // namespace prism_rtl
