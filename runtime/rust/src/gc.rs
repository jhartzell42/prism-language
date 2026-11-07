//! The zombie collector: trial deletion restricted to zombies, in the compact round.
//!
//! Values and nodes are reference counted, and the event flow is a DAG, so counts free
//! everything but one loop: zombies whose constants lien each other. Row A's hold keeps
//! a value with row B's `$b` inside, and B's keeps A's `$a`; both rows leave in one tick,
//! and neither count reaches zero (Graph ch. 4, worked examples, "The one loop"). D5
//! says no leaks, ever, so the compact round looks for that loop and nothing else
//! (Compiler ch. 5, Research, "Reference counting without compaction").
//!
//! The method is trial deletion over the zombies the runtime watches ([`crate::Cx::clean_up_later`]):
//!
//! - **Reach.** From the watched zombies, follow [`Node::edges`] (the strong references
//!   each node's fields hold) to the set of nodes they keep.
//! - **Count from inside.** For each node, the references from inside the set. A node
//!   whose `Rc` count is larger is held from outside: by the tree, a live value, or a
//!   subscriber's end of an edge.
//! - **Keep what the outside reaches;** release the rest. [`Node::release`] drops a
//!   garbage node's fields, which breaks the loop, and the counts free the bytes.
//!
//! A zombie stays watched until it dies, so a round costs the live zombies and what they
//! keep, never the tree (doubt GC in claude/backend.md).

use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::cx::Node;

/// The zombies the compact round looks at.
#[derive(Default)]
pub struct Collector {
    watched: Vec<Weak<dyn Node>>,
}

impl Collector {
    pub fn watch(&mut self, zombies: impl Iterator<Item = Weak<dyn Node>>) {
        self.watched.extend(zombies);
    }

    /// How many zombies are still alive and watched.
    pub fn watching(&self) -> usize {
        self.watched.iter().filter(|w| w.strong_count() > 0).count()
    }

    /// One round: release every node the watched zombies keep that nothing outside them
    /// holds. Returns how many nodes were released.
    pub fn collect(&mut self) -> usize {
        self.watched.retain(|w| w.strong_count() > 0);
        let mut held: Vec<Rc<dyn Node>> = self.watched.iter().filter_map(Weak::upgrade).collect();
        let mut index: HashMap<*const (), usize> = HashMap::new();
        held.retain(|n| index.insert(Rc::as_ptr(n) as *const (), index.len()).is_none());

        // Reach, counting the references from inside.
        let mut inside: Vec<usize> = vec![0; held.len()];
        let mut edges: Vec<Vec<usize>> = Vec::new();
        let mut i = 0;
        while i < held.len() {
            let mut out = Vec::new();
            held[i].clone().edges(&mut |e: &Rc<dyn Node>| {
                let k = Rc::as_ptr(e) as *const ();
                let j = *index.entry(k).or_insert_with(|| {
                    held.push(e.clone());
                    inside.push(0);
                    held.len() - 1
                });
                inside[j] += 1;
                out.push(j);
            });
            edges.push(out);
            i += 1;
        }

        // Keep what the outside holds, and everything it reaches.
        let mut live = vec![false; held.len()];
        let mut stack: Vec<usize> = (0..held.len()).filter(|&i| Rc::strong_count(&held[i]) - 1 > inside[i]).collect();
        while let Some(i) = stack.pop() {
            if !std::mem::replace(&mut live[i], true) {
                stack.extend(edges[i].iter().copied());
            }
        }

        let garbage: Vec<usize> = (0..held.len()).filter(|&i| !live[i]).collect();
        for &i in &garbage {
            held[i].release();
        }
        garbage.len()
    }
}
