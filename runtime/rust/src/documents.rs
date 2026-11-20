//! The standard library's `Documents` builtins (library/documents.prism, the declarations
//! with no body): what a document's hosting reads off a place's path, and the roles'
//! element trees put together. The emitted program calls each by the path its
//! declaration names (ruling RT: `= runtime "documents::seat_path_of"`), generic over the
//! program's own types.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use std::rc::Rc;

use crate::cells::Dynamic;
use crate::collections::{Array, Map};
use crate::existential::Packed;
use crate::pod::{FromPod, Path, Pod, Segment};

/// `SeatPathOf`: a seat's own prefix of a place's path, the role's position and the
/// seat's key (`0/[White]/#g/2` is White's `0/[White]`).
pub fn seat_path_of(path: &Path) -> Path {
    Path(path.0.iter().take(2).cloned().collect())
}

/// `SeatIdAt`: within one role's tree, a place's first key is its seat id, read back at
/// the role's own seat id type.
pub fn seat_id_at<T: FromPod>(path: &Path) -> T {
    let key = path.0.iter().find_map(|s| match s {
        Segment::Key(k) => Some(k),
        _ => None,
    });
    let key = key.expect("a place in a role's tree with no key (its seat id) on its path");
    T::from_pod(key).unwrap_or_else(|f| panic!("a seat's key is no seat id of its role: {}", f.message))
}

/// `$Merge`: the roles' trees as one, each role's tree under the role's position in the
/// list (`0/..` the first role's, `1/..` the second's).
pub fn merge<G: Clone + 'static>(trees: &Array<Dynamic<Map<Path, Dynamic<G>>>>) -> Dynamic<Map<Path, Dynamic<G>>> {
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

/// `$GuestsOf`: a role's tree as guests. Each place of the document's language (`is_seat`:
/// a tag of the `Seat` family; a place of another language is not the document's) becomes
/// the existential `Guest d`: the role, the seat id its path's first key gives, and its
/// element, packed with the role's variant number (`kind`).
pub fn guests_of<F, S, E, N>(kind: N, role: &F, tree: &Dynamic<Map<Path, Dynamic<E>>>, is_seat: impl Fn(&E) -> bool + 'static) -> Dynamic<Map<Path, Dynamic<Packed>>>
where
    F: Clone + 'static,
    S: FromPod + Clone + 'static,
    E: Clone + 'static,
    N: Clone + 'static,
{
    let role = role.clone();
    let is_seat = Rc::new(is_seat);
    tree.map(move |places| {
        let mut guests = Map::new();
        for (path, element) in places.keys().zip(places.values()) {
            if !is_seat(&element.sample_detached()) {
                continue;
            }
            // (the id as plain data, whatever `S` is: a guest is packed as the existential's
            // body holds it, its seat id an associated type at the hidden role, and every
            // opened arm unpacks it so, prismc's `UnpackedAt`)
            let (role, kind, id) = (role.clone(), kind.clone(), seat_id_at::<Pod>(path));
            guests.insert(path.clone(), element.map(move |e| Packed::of((role.clone(), id.clone(), e.clone())).with_kind(kind.clone()).packed()));
        }
        guests
    })
}
