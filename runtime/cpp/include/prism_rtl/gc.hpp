// The zombie collector: trial deletion restricted to zombies, in the compact round.
//
// Values and nodes are reference counted (`Rc`, rc.hpp), and the event flow is a
// DAG, so counts free everything but one loop: zombies whose constants lien each other.
// Row A's hold keeps a value with row B's `$b` inside, and B's keeps A's `$a`; both rows
// leave in one tick, and neither count reaches zero (Graph ch. 4, worked examples, "The
// one loop"). D5 says no leaks, ever, so the compact round looks for that loop and
// nothing else (Compiler ch. 5, Research, "Reference counting without compaction").
//
// The method is trial deletion over the zombies the runtime watches
// (`Cx::clean_up_later`):
//
// - **Reach.** From the watched zombies, follow `Node::edges` (the strong references
//   each node's fields hold) to the set of nodes they keep.
// - **Count from inside.** For each node, the references from inside the set. A node
//   whose `use_count` is larger is held from outside: by the tree, a live value, or a
//   subscriber's end of an edge.
// - **Keep what the outside reaches;** release the rest. `Node::release` drops a
//   garbage node's fields, which breaks the loop, and the counts free the bytes.
//
// A zombie stays watched until it dies, so a round costs the live zombies and what they
// keep, never the tree (doubt GC in docs/backend.md).

#pragma once

#include "prism_rtl/rc.hpp"
#include <cstddef>
#include <memory>
#include <vector>

namespace prism_rtl {

class Node;

/// The zombies the compact round looks at.
class Collector {
public:
    void watch(const std::vector<Weak<Node>>& zombies);
    /// How many zombies are still alive and watched.
    std::size_t watching() const;
    /// One round: release every node the watched zombies keep that nothing outside them
    /// holds. Returns how many nodes were released.
    std::size_t collect();

private:
    std::vector<Weak<Node>> watched_;
};

}  // namespace prism_rtl
