//! Written by Claude on the claude-self-hosting branch; subject to change.
//!
//! `[[a]]`: a persistent array, a balanced tree of chunks and a tail, the same design as
//! the C++ runtime's `Array` (runtime/cpp/include/prism_rtl/collections.hpp).
//!
//! - **The tail** holds the last elements, at most 64, in a vector of its own that grows
//!   1, 2, 4, ..., 64 slots while the array is small (64 at once in an array of 64 or
//!   more). A push lands there: in place when the array holds the tail alone, else after
//!   copying the tail (at most 64 elements). A full tail goes into the tree whole, as a
//!   chunk, with no element copied.
//! - **The tree** holds the rest: chunks (vectors of 1 to 64 elements) at its leaves, and
//!   above them nodes of two children, each with its length and height, balanced as an
//!   AVL tree is (the children's heights differ by one at most), so its height is at most
//!   about 1.44 log2 of its chunks. Two trees are concatenated by the AVL join (the
//!   shorter hung on the taller's spine, rebalanced on the way up), and a tree is cut at an
//!   index by joining the pieces along the path to it; both copy one path and one chunk
//!   at most. Where one side of a concatenation is a single chunk that fits beside its
//!   neighbour, the two become one chunk, so arrays built by appending small ones stay
//!   dense.
//! - **Clones** share everything; an update copies what it changes (`Rc::make_mut` on the
//!   path to an element), so no array sharing a node sees it.
//!
//! The costs, `n` elements (Semantics Appendix B, Arrays; runtime/rust/README.md): `len`
//! constant; indexing log (constant in the tail, where every array of 64 or fewer
//! elements is); `push_back` amortised constant (at most 64 elements copied, and once in
//! 64 pushes one path of the tree); `append` log; `take`, `skip`, `update`, `set` and
//! indexing in place log, copying one chunk at most; iteration linear. Storage: at most
//! `2n + 1` slots while the array is small, `n + 64` beyond, plus a node per chunk.
//!
//! No `unsafe`. A panic partway through an update (an element's `clone` panicking) leaves
//! a valid array, possibly without the elements of the node being rebuilt; programs do
//! not catch panics (a panic ends the program), so nothing reads it.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

const CHUNK: usize = 64;

#[derive(Clone)]
enum Node<T> {
    Leaf(Vec<T>),
    Join { left: Rc<Node<T>>, right: Rc<Node<T>>, len: usize, height: u8 },
}

impl<T> Node<T> {
    fn len(&self) -> usize {
        match self {
            Node::Leaf(v) => v.len(),
            Node::Join { len, .. } => *len,
        }
    }
    fn height(&self) -> u8 {
        match self {
            Node::Leaf(_) => 0,
            Node::Join { height, .. } => *height,
        }
    }
}

/// Two siblings whose heights differ by one at most, under a node.
fn join<T>(left: Rc<Node<T>>, right: Rc<Node<T>>) -> Rc<Node<T>> {
    let len = left.len() + right.len();
    let height = left.height().max(right.height()) + 1;
    Rc::new(Node::Join { left, right, len, height })
}

fn children<T>(n: &Rc<Node<T>>) -> (Rc<Node<T>>, Rc<Node<T>>) {
    match &**n {
        Node::Join { left, right, .. } => (left.clone(), right.clone()),
        Node::Leaf(_) => unreachable!("a node taller than its sibling by two is a join"),
    }
}

/// Two siblings whose heights differ by two at most, under a node, rotated to balance.
fn balance<T>(l: Rc<Node<T>>, r: Rc<Node<T>>) -> Rc<Node<T>> {
    let (hl, hr) = (l.height(), r.height());
    if hl > hr + 1 {
        let (ll, lr) = children(&l);
        if ll.height() >= lr.height() {
            join(ll, join(lr, r))
        } else {
            let (lrl, lrr) = children(&lr);
            join(join(ll, lrl), join(lrr, r))
        }
    } else if hr > hl + 1 {
        let (rl, rr) = children(&r);
        if rr.height() >= rl.height() {
            join(join(l, rl), rr)
        } else {
            let (rll, rlr) = children(&rl);
            join(join(l, rll), join(rlr, rr))
        }
    } else {
        join(l, r)
    }
}

/// `l`'s elements, then `r`'s: the AVL join, log in the taller's height.
fn concat<T>(l: Rc<Node<T>>, r: Rc<Node<T>>) -> Rc<Node<T>> {
    let (hl, hr) = (l.height(), r.height());
    if hl > hr + 1 {
        let (ll, lr) = children(&l);
        balance(ll, concat(lr, r))
    } else if hr > hl + 1 {
        let (rl, rr) = children(&r);
        balance(concat(l, rl), rr)
    } else {
        join(l, r)
    }
}

/// `extra` at the end of the tree's last chunk, which has room for it; the path to it
/// this tree's own.
fn extend_last<T: Clone>(n: &mut Rc<Node<T>>, extra: Vec<T>) {
    match Rc::make_mut(n) {
        Node::Leaf(v) => {
            v.reserve_exact(extra.len());
            v.extend(extra);
        }
        Node::Join { right, len, .. } => {
            *len += extra.len();
            extend_last(right, extra);
        }
    }
}

/// `extra` at the start of the tree's first chunk, which has room for it.
fn extend_first<T: Clone>(n: &mut Rc<Node<T>>, mut extra: Vec<T>) {
    match Rc::make_mut(n) {
        Node::Leaf(v) => {
            extra.reserve_exact(v.len());
            extra.append(v);
            *v = extra;
        }
        Node::Join { left, len, .. } => {
            *len += extra.len();
            extend_first(left, extra);
        }
    }
}

fn last_len<T>(mut n: &Node<T>) -> usize {
    loop {
        match n {
            Node::Leaf(v) => return v.len(),
            Node::Join { right, .. } => n = right,
        }
    }
}

fn first_len<T>(mut n: &Node<T>) -> usize {
    loop {
        match n {
            Node::Leaf(v) => return v.len(),
            Node::Join { left, .. } => n = left,
        }
    }
}

/// A chunk of its own: moved out when this was its only holder, else cloned (64 at most).
fn own_chunk<T: Clone>(v: Rc<Vec<T>>) -> Vec<T> {
    Rc::try_unwrap(v).unwrap_or_else(|shared| shared.as_ref().clone())
}

/// `l`'s elements, then `r`'s, a chunk merged into its neighbour when one side is a
/// single chunk that fits there.
fn seam<T: Clone>(mut l: Rc<Node<T>>, mut r: Rc<Node<T>>) -> Rc<Node<T>> {
    if let Node::Leaf(v) = &*r {
        if last_len(&l) + v.len() <= CHUNK {
            let v = match Rc::try_unwrap(r) {
                Ok(Node::Leaf(v)) => v,
                Ok(Node::Join { .. }) => unreachable!("matched a chunk"),
                Err(shared) => match &*shared {
                    Node::Leaf(v) => v.clone(),
                    Node::Join { .. } => unreachable!("matched a chunk"),
                },
            };
            extend_last(&mut l, v);
            return l;
        }
    }
    if let Node::Leaf(v) = &*l {
        if first_len(&r) + v.len() <= CHUNK {
            let v = match Rc::try_unwrap(l) {
                Ok(Node::Leaf(v)) => v,
                Ok(Node::Join { .. }) => unreachable!("matched a chunk"),
                Err(shared) => match &*shared {
                    Node::Leaf(v) => v.clone(),
                    Node::Join { .. } => unreachable!("matched a chunk"),
                },
            };
            extend_first(&mut r, v);
            return r;
        }
    }
    concat(l, r)
}

/// The first `k` elements, `0 < k < len`.
fn take_tree<T: Clone>(n: &Rc<Node<T>>, k: usize) -> Rc<Node<T>> {
    match &**n {
        Node::Leaf(v) => Rc::new(Node::Leaf(v[..k].to_vec())),
        Node::Join { left, right, .. } => {
            let ll = left.len();
            match k.cmp(&ll) {
                Ordering::Equal => left.clone(),
                Ordering::Less => take_tree(left, k),
                Ordering::Greater => concat(left.clone(), take_tree(right, k - ll)),
            }
        }
    }
}

/// Everything after the first `k` elements, `0 < k < len`.
fn drop_tree<T: Clone>(n: &Rc<Node<T>>, k: usize) -> Rc<Node<T>> {
    match &**n {
        Node::Leaf(v) => Rc::new(Node::Leaf(v[k..].to_vec())),
        Node::Join { left, right, .. } => {
            let ll = left.len();
            match k.cmp(&ll) {
                Ordering::Equal => right.clone(),
                Ordering::Greater => drop_tree(right, k - ll),
                Ordering::Less => concat(drop_tree(left, k), right.clone()),
            }
        }
    }
}

fn get_mut<T: Clone>(n: &mut Rc<Node<T>>, i: usize) -> &mut T {
    match Rc::make_mut(n) {
        Node::Leaf(v) => &mut v[i],
        Node::Join { left, right, .. } => {
            let ll = left.len();
            if i < ll { get_mut(left, i) } else { get_mut(right, i - ll) }
        }
    }
}

/// A tree of these chunks, in order, balanced (siblings' sizes differ by one at most).
fn balanced<T>(chunks: &[Rc<Node<T>>]) -> Rc<Node<T>> {
    if chunks.len() == 1 {
        return chunks[0].clone();
    }
    let mid = chunks.len() / 2;
    join(balanced(&chunks[..mid]), balanced(&chunks[mid..]))
}

/// `[[a]]` (module header).
pub struct Array<T> {
    tree: Option<Rc<Node<T>>>,
    tail: Option<Rc<Vec<T>>>,
    len: usize,
}

impl<T> Clone for Array<T> {
    fn clone(&self) -> Self {
        Array { tree: self.tree.clone(), tail: self.tail.clone(), len: self.len }
    }
}

impl<T> Default for Array<T> {
    fn default() -> Self {
        Array { tree: None, tail: None, len: 0 }
    }
}

impl<T> Array<T> {
    pub fn new() -> Self {
        Array::default()
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn tail_len(&self) -> usize {
        self.tail.as_ref().map_or(0, |t| t.len())
    }
    /// The slots a tail of `have` elements grows to: doubling while the array is small,
    /// a whole chunk once it is not.
    fn grown(&self, have: usize) -> usize {
        if self.len >= CHUNK { CHUNK } else { (2 * have).clamp(1, CHUNK) }
    }
    pub fn get(&self, i: usize) -> Option<&T> {
        if i >= self.len {
            return None;
        }
        let in_tree = self.len - self.tail_len();
        if i >= in_tree {
            return self.tail.as_ref().map(|t| &t[i - in_tree]);
        }
        let mut n = self.tree.as_deref()?;
        let mut i = i;
        loop {
            match n {
                Node::Leaf(v) => return v.get(i),
                Node::Join { left, right, .. } => {
                    let ll = left.len();
                    if i < ll {
                        n = left;
                    } else {
                        i -= ll;
                        n = right;
                    }
                }
            }
        }
    }
    pub fn front(&self) -> Option<&T> {
        self.get(0)
    }
    pub fn back(&self) -> Option<&T> {
        self.len.checked_sub(1).and_then(|i| self.get(i))
    }
    pub fn last(&self) -> Option<&T> {
        self.back()
    }
    /// The elements in order.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            stack: self.tree.as_deref().into_iter().collect(),
            chunk: [].iter(),
            tail: self.tail.as_ref().map_or(&[][..], |t| &t[..]).iter(),
            left: self.len,
        }
    }
    /// The slots of storage the array's chunks and tail hold, for the tests and for
    /// measurement.
    pub fn capacity(&self) -> usize {
        fn slots<T>(n: &Node<T>) -> usize {
            match n {
                Node::Leaf(v) => v.capacity(),
                Node::Join { left, right, .. } => slots(left) + slots(right),
            }
        }
        self.tree.as_deref().map_or(0, slots) + self.tail.as_ref().map_or(0, |t| t.capacity())
    }
}

impl<T: Clone> Array<T> {
    /// An array of one element.
    pub fn unit(x: T) -> Self {
        Array { tree: None, tail: Some(Rc::new(vec![x])), len: 1 }
    }
    pub fn push_back(&mut self, x: T) {
        let tail_len = self.tail_len();
        if tail_len == CHUNK {
            let full = own_chunk(self.tail.take().expect("a full tail"));
            let chunk = Rc::new(Node::Leaf(full));
            self.tree = Some(match self.tree.take() {
                None => chunk,
                Some(t) => concat(t, chunk),
            });
            let mut fresh = Vec::with_capacity(CHUNK);
            fresh.push(x);
            self.tail = Some(Rc::new(fresh));
        } else {
            let cap = self.grown(tail_len).max(tail_len + 1);
            match &mut self.tail {
                None => {
                    let mut fresh = Vec::with_capacity(cap);
                    fresh.push(x);
                    self.tail = Some(Rc::new(fresh));
                }
                Some(t) => match Rc::get_mut(t) {
                    Some(v) => {
                        if v.len() == v.capacity() {
                            v.reserve_exact(cap - v.len());
                        }
                        v.push(x);
                    }
                    None => {
                        let mut fresh = Vec::with_capacity(cap);
                        fresh.extend(t.iter().cloned());
                        fresh.push(x);
                        *t = Rc::new(fresh);
                    }
                },
            }
        }
        self.len += 1;
    }
    pub fn push_front(&mut self, x: T) {
        let mut a = Array::unit(x);
        a.append(std::mem::take(self));
        *self = a;
    }
    /// `other`'s elements after this array's: log, copying a chunk or two at most.
    pub fn append(&mut self, other: Self) {
        if other.len == 0 {
            return;
        }
        if self.len == 0 {
            *self = other;
            return;
        }
        let Array { tree: their_tree, tail: their_tail, len: their_len } = other;
        let total = self.len + their_len;
        let mut left = self.tree.take();
        if let Some(t) = self.tail.take() {
            let theirs = their_tail.as_ref().map_or(0, |t| t.len());
            if their_tree.is_none() && t.len() + theirs <= CHUNK {
                let mut v = own_chunk(t);
                let theirs = own_chunk(their_tail.expect("a non-empty array without a tree has a tail"));
                v.reserve_exact(theirs.len());
                v.extend(theirs);
                self.tree = left;
                self.tail = Some(Rc::new(v));
                self.len = total;
                return;
            }
            let chunk = Rc::new(Node::Leaf(own_chunk(t)));
            left = Some(match left {
                None => chunk,
                Some(l) => seam(l, chunk),
            });
        }
        self.tree = match (left, their_tree) {
            (Some(l), Some(r)) => Some(seam(l, r)),
            (l, None) => l,
            (None, r) => r,
        };
        self.tail = their_tail;
        self.len = total;
    }
    /// The first `k` elements; `k` is at most the length.
    #[must_use]
    pub fn take(&self, k: usize) -> Self {
        assert!(k <= self.len, "Array::take: {k} elements of {}", self.len);
        if k == self.len {
            return self.clone();
        }
        let in_tree = self.len - self.tail_len();
        if k >= in_tree {
            let tail = self.tail.as_ref().filter(|_| k > in_tree).map(|t| Rc::new(t[..k - in_tree].to_vec()));
            return Array { tree: self.tree.clone(), tail, len: k };
        }
        if k == 0 {
            return Array::new();
        }
        Array { tree: self.tree.as_ref().map(|t| take_tree(t, k)), tail: None, len: k }
    }
    /// Everything after the first `k` elements; `k` is at most the length.
    #[must_use]
    pub fn skip(&self, k: usize) -> Self {
        assert!(k <= self.len, "Array::skip: {k} elements of {}", self.len);
        if k == 0 {
            return self.clone();
        }
        let in_tree = self.len - self.tail_len();
        if k >= in_tree {
            let tail = self.tail.as_ref().filter(|_| k < self.len).map(|t| Rc::new(t[k - in_tree..].to_vec()));
            return Array { tree: None, tail, len: self.len - k };
        }
        Array { tree: self.tree.as_ref().map(|t| drop_tree(t, k)), tail: self.tail.clone(), len: self.len - k }
    }
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        if i >= self.len {
            return None;
        }
        let in_tree = self.len - self.tail_len();
        if i >= in_tree {
            return self.tail.as_mut().map(|t| &mut Rc::make_mut(t)[i - in_tree]);
        }
        self.tree.as_mut().map(|t| get_mut(t, i))
    }
    /// The elements in order, to update in place: every node made this array's own first
    /// (the shared ones copied, linear at worst), then each chunk's slice.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        fn chunks<'a, T: Clone>(n: &'a mut Rc<Node<T>>, out: &mut Vec<&'a mut [T]>) {
            match Rc::make_mut(n) {
                Node::Leaf(v) => out.push(&mut v[..]),
                Node::Join { left, right, .. } => {
                    chunks(left, out);
                    chunks(right, out);
                }
            }
        }
        let mut out = Vec::new();
        if let Some(t) = self.tree.as_mut() {
            chunks(t, &mut out);
        }
        if let Some(t) = self.tail.as_mut() {
            out.push(&mut Rc::make_mut(t)[..]);
        }
        out.into_iter().flat_map(|c| c.iter_mut())
    }
    /// Set element `i` to `x`, answering what it was.
    pub fn set(&mut self, i: usize, x: T) -> T {
        std::mem::replace(&mut self[i], x)
    }
    /// The array with element `i` set to `x`; this one is unchanged.
    #[must_use]
    pub fn update(&self, i: usize, x: T) -> Self {
        let mut a = self.clone();
        a[i] = x;
        a
    }
}

impl<T> From<Vec<T>> for Array<T> {
    /// Chunks of 64 moved out of the vector, and the rest as the tail: each element moved
    /// once, into storage of exactly its count.
    fn from(v: Vec<T>) -> Self {
        let len = v.len();
        if len == 0 {
            return Array::new();
        }
        let full = (len - 1) / CHUNK;
        let mut it = v.into_iter();
        let chunks: Vec<Rc<Node<T>>> = (0..full).map(|_| Rc::new(Node::Leaf(it.by_ref().take(CHUNK).collect()))).collect();
        let tail: Vec<T> = it.collect();
        Array { tree: if chunks.is_empty() { None } else { Some(balanced(&chunks)) }, tail: Some(Rc::new(tail)), len }
    }
}

impl<T: Clone> From<&[T]> for Array<T> {
    fn from(xs: &[T]) -> Self {
        Array::from(xs.to_vec())
    }
}

impl<T> FromIterator<T> for Array<T> {
    fn from_iter<I: IntoIterator<Item = T>>(i: I) -> Self {
        Array::from(i.into_iter().collect::<Vec<T>>())
    }
}

impl<T: Clone> Extend<T> for Array<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, i: I) {
        for x in i {
            self.push_back(x);
        }
    }
}

impl<T> Index<usize> for Array<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let len = self.len;
        self.get(i).unwrap_or_else(|| panic!("Array::index: index {i} of an array of {len}"))
    }
}

impl<T: Clone> IndexMut<usize> for Array<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let len = self.len;
        self.get_mut(i).unwrap_or_else(|| panic!("Array::index_mut: index {i} of an array of {len}"))
    }
}

/// The elements in order: the tree's nodes still to visit on a stack, the chunk being
/// read, then the tail.
pub struct Iter<'a, T> {
    stack: Vec<&'a Node<T>>,
    chunk: std::slice::Iter<'a, T>,
    tail: std::slice::Iter<'a, T>,
    left: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            if let Some(x) = self.chunk.next() {
                self.left -= 1;
                return Some(x);
            }
            match self.stack.pop() {
                Some(Node::Leaf(v)) => self.chunk = v.iter(),
                Some(Node::Join { left, right, .. }) => {
                    self.stack.push(right);
                    self.stack.push(left);
                }
                None => {
                    let x = self.tail.next()?;
                    self.left -= 1;
                    return Some(x);
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}

impl<'a, T> IntoIterator for &'a Array<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

/// The elements in order, by value: a node's elements moved out when the array held it
/// alone, cloned when another array shares it.
pub struct IntoIter<T> {
    stack: Vec<Rc<Node<T>>>,
    chunk: std::vec::IntoIter<T>,
    tail: Option<Rc<Vec<T>>>,
    left: usize,
}

impl<T: Clone> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        loop {
            if let Some(x) = self.chunk.next() {
                self.left -= 1;
                return Some(x);
            }
            match self.stack.pop() {
                Some(n) => match Rc::unwrap_or_clone(n) {
                    Node::Leaf(v) => self.chunk = v.into_iter(),
                    Node::Join { left, right, .. } => {
                        self.stack.push(right);
                        self.stack.push(left);
                    }
                },
                None => self.chunk = own_chunk(self.tail.take()?).into_iter(),
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl<T: Clone> ExactSizeIterator for IntoIter<T> {}

impl<T: Clone> IntoIterator for Array<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { stack: self.tree.into_iter().collect(), chunk: Vec::new().into_iter(), tail: self.tail, left: self.len }
    }
}

impl<T: PartialEq> PartialEq for Array<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}

impl<T: Eq> Eq for Array<T> {}

impl<T: PartialOrd> PartialOrd for Array<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<T: Ord> Ord for Array<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<T: Hash> Hash for Array<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for x in self.iter() {
            x.hash(state);
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Array<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
