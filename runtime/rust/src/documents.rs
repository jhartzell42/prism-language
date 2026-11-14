//! The standard library's `Documents` builtins (library/documents.prism, the declarations
//! with no body): what a document's hosting reads off a place's path, and the roles'
//! element trees put together. The emitted program calls each by its path from the
//! prelude (`crate::prelude::documents_seat_path_of`), generic over the program's own types.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use crate::cells::Dynamic;
use crate::collections::{Array, Map};
use crate::pod::{FromPod, Path, Segment};

/// `SeatPathOf`: a seat's own prefix of a place's path, the role's position and the
/// seat's key (`0/[White]/#g/2` is White's `0/[White]`).
pub fn documents_seat_path_of(path: &Path) -> Path {
    Path(path.0.iter().take(2).cloned().collect())
}

/// `SeatIdAt`: within one role's tree, a place's first key is its seat id, read back at
/// the role's own seat id type.
pub fn documents_seat_id_at<T: FromPod>(path: &Path) -> T {
    let key = path.0.iter().find_map(|s| match s {
        Segment::Key(k) => Some(k),
        _ => None,
    });
    let key = key.expect("a place in a role's tree with no key (its seat id) on its path");
    T::from_pod(key).unwrap_or_else(|f| panic!("a seat's key is no seat id of its role: {}", f.message))
}

/// `$Merge`: the roles' trees as one, each role's tree under the role's position in the
/// list (`0/..` the first role's, `1/..` the second's).
pub fn documents_merge<G: Clone + 'static>(trees: &Array<Dynamic<Map<Path, Dynamic<G>>>>) -> Dynamic<Map<Path, Dynamic<G>>> {
    let mut merged: Dynamic<Map<Path, Dynamic<G>>> = Dynamic::constant(Map::new());
    for (i, tree) in trees.iter().enumerate() {
        let prefix = Path(vec![Segment::Pos(i as u32)]);
        merged = merged.map2(tree, move |sofar, role| {
            let mut out = sofar.clone();
            for (path, guest) in role.keys().zip(role.values()) {
                out.insert(prefix.join(path), guest.clone());
            }
            out
        });
    }
    merged
}
