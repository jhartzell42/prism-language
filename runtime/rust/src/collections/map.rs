//! Written by Claude on the claude-self-hosting branch; subject to change.
//!
//! `k ==> v`: a persistent ordered map, an AVL tree of counted nodes, one node per entry,
//! as the C++ runtime's `Map` is (runtime/cpp/include/prism_rtl/collections.hpp). A clone
//! is a count. An update goes through `Rc::make_mut` on the path to its key: a node this
//! map alone holds changes in place, and a node another map shares is copied first, so
//! no map sharing it sees the update. Each node holds its height and the number of entries
//! under it.
//!
//! The costs (Semantics Appendix B, Maps), `n` entries: `insert`, `remove`, `get`,
//! `get_mut`, `update`, `without` and `get_min` are logarithmic (the tree's height is at
//! most about 1.44 log n, and an update copies at most the nodes on one path); `len` is
//! constant (the root's count); `iter`, `keys` and `values` are linear, in key order;
//! collecting `n` pairs is `n log n`, the later pair winning a shared key; `union` is
//! `m log (n + m)`, `m` the smaller map's size. An empty map allocates nothing and a map
//! of `n` entries holds `n` nodes (claude/bugs/small-arrays-whole-chunks.md: `im_rc`'s
//! `OrdMap` allocated a node of 64 entries for every map, an empty one included).
//!
//! No `unsafe`. A panic partway through an update (a key's `cmp` or a clone panicking)
//! leaves a valid tree that may have lost the entries under the node being rebuilt;
//! programs do not catch panics (a panic ends the program), so nothing reads it.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::Index;
use std::rc::Rc;

type Link<K, V> = Option<Rc<Node<K, V>>>;

#[derive(Clone)]
struct Node<K, V> {
    entry: (K, V),
    left: Link<K, V>,
    right: Link<K, V>,
    height: u8,
    count: usize,
}

fn height<K, V>(l: &Link<K, V>) -> u8 {
    l.as_ref().map_or(0, |n| n.height)
}

fn count<K, V>(l: &Link<K, V>) -> usize {
    l.as_ref().map_or(0, |n| n.count)
}

impl<K, V> Node<K, V> {
    fn leaf(k: K, v: V) -> Node<K, V> {
        Node { entry: (k, v), left: None, right: None, height: 1, count: 1 }
    }
    /// Height and count from the children.
    fn fix(&mut self) {
        self.height = height(&self.left).max(height(&self.right)) + 1;
        self.count = count(&self.left) + count(&self.right) + 1;
    }
    fn balance(&self) -> i32 {
        i32::from(height(&self.left)) - i32::from(height(&self.right))
    }
}

/// The node at `link`, this map's own (copied first when another map shares it).
fn own<K: Clone, V: Clone>(link: &mut Link<K, V>) -> &mut Node<K, V> {
    Rc::make_mut(link.as_mut().expect("a node on the path"))
}

/// The node at `link`, taken out of the tree by value (copied when shared).
fn take_node<K: Clone, V: Clone>(link: &mut Link<K, V>) -> Node<K, V> {
    let rc = link.take().expect("a node on the path");
    Rc::try_unwrap(rc).unwrap_or_else(|shared| (*shared).clone())
}

/// `link`'s left child up in its place.
fn rotate_right<K: Clone, V: Clone>(link: &mut Link<K, V>) {
    let mut n = take_node(link);
    let mut l = take_node(&mut n.left);
    n.left = l.right.take();
    n.fix();
    l.right = Some(Rc::new(n));
    l.fix();
    *link = Some(Rc::new(l));
}

/// `link`'s right child up in its place.
fn rotate_left<K: Clone, V: Clone>(link: &mut Link<K, V>) {
    let mut n = take_node(link);
    let mut r = take_node(&mut n.right);
    n.right = r.left.take();
    n.fix();
    r.left = Some(Rc::new(n));
    r.fix();
    *link = Some(Rc::new(r));
}

/// The node at `link` with its height and count fixed and its balance restored, after one
/// of its subtrees grew or shrank by one level at most. The node is this map's own.
fn rebalance<K: Clone, V: Clone>(link: &mut Link<K, V>) {
    let n = own(link);
    n.fix();
    let b = n.balance();
    if b > 1 {
        if n.left.as_ref().is_some_and(|l| l.balance() < 0) {
            rotate_left(&mut n.left);
        }
        rotate_right(link);
    } else if b < -1 {
        if n.right.as_ref().is_some_and(|r| r.balance() > 0) {
            rotate_right(&mut n.right);
        }
        rotate_left(link);
    }
}

fn insert<K: Ord + Clone, V: Clone>(link: &mut Link<K, V>, k: K, v: V) -> Option<V> {
    if link.is_none() {
        *link = Some(Rc::new(Node::leaf(k, v)));
        return None;
    }
    let n = own(link);
    let old = match k.cmp(&n.entry.0) {
        Ordering::Less => insert(&mut n.left, k, v),
        Ordering::Greater => insert(&mut n.right, k, v),
        Ordering::Equal => {
            n.entry.0 = k;
            return Some(std::mem::replace(&mut n.entry.1, v));
        }
    };
    if old.is_none() {
        rebalance(link);
    }
    old
}

/// The least entry under `link`, taken out; `link` is not empty.
fn remove_min<K: Clone, V: Clone>(link: &mut Link<K, V>) -> (K, V) {
    if own(link).left.is_none() {
        let n = take_node(link);
        *link = n.right;
        return n.entry;
    }
    let min = remove_min(&mut own(link).left);
    rebalance(link);
    min
}

/// `k`'s entry taken out from under `link`; `k` is a key there.
fn remove<K, V, BK>(link: &mut Link<K, V>, k: &BK) -> (K, V)
where
    K: Ord + Clone + Borrow<BK>,
    V: Clone,
    BK: Ord + ?Sized,
{
    let n = own(link);
    let out = match k.cmp(n.entry.0.borrow()) {
        Ordering::Less => remove(&mut n.left, k),
        Ordering::Greater => remove(&mut n.right, k),
        Ordering::Equal => {
            let mut n = take_node(link);
            match (n.left.is_some(), n.right.is_some()) {
                (false, _) => *link = n.right.take(),
                (_, false) => *link = n.left.take(),
                (true, true) => {
                    let min = remove_min(&mut n.right);
                    let out = std::mem::replace(&mut n.entry, min);
                    *link = Some(Rc::new(n));
                    rebalance(link);
                    return out;
                }
            }
            return n.entry;
        }
    };
    rebalance(link);
    out
}

fn get_mut<'a, K, V, BK>(link: &'a mut Link<K, V>, k: &BK) -> Option<&'a mut V>
where
    K: Ord + Clone + Borrow<BK>,
    V: Clone,
    BK: Ord + ?Sized,
{
    let n = Rc::make_mut(link.as_mut()?);
    match k.cmp(n.entry.0.borrow()) {
        Ordering::Less => get_mut(&mut n.left, k),
        Ordering::Greater => get_mut(&mut n.right, k),
        Ordering::Equal => Some(&mut n.entry.1),
    }
}

/// `k ==> v` (module header).
pub struct Map<K, V> {
    root: Link<K, V>,
}

impl<K, V> Clone for Map<K, V> {
    fn clone(&self) -> Self {
        Map { root: self.root.clone() }
    }
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Map { root: None }
    }
}

impl<K, V> Map<K, V> {
    pub fn new() -> Self {
        Map::default()
    }
    pub fn len(&self) -> usize {
        count(&self.root)
    }
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
    /// The entries in key order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        let mut it = Iter { stack: Vec::with_capacity(usize::from(height(&self.root))), left: self.len() };
        it.descend(&self.root);
        it
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.iter().map(|(k, _)| k)
    }
    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.iter().map(|(_, v)| v)
    }
    /// The entry of the least key.
    pub fn get_min(&self) -> Option<&(K, V)> {
        let mut n = self.root.as_deref()?;
        while let Some(l) = n.left.as_deref() {
            n = l;
        }
        Some(&n.entry)
    }
    /// The entry of the greatest key.
    pub fn get_max(&self) -> Option<&(K, V)> {
        let mut n = self.root.as_deref()?;
        while let Some(r) = n.right.as_deref() {
            n = r;
        }
        Some(&n.entry)
    }
    pub fn get<BK>(&self, k: &BK) -> Option<&V>
    where
        BK: Ord + ?Sized,
        K: Borrow<BK>,
    {
        let mut at = self.root.as_deref();
        while let Some(n) = at {
            match k.cmp(n.entry.0.borrow()) {
                Ordering::Less => at = n.left.as_deref(),
                Ordering::Greater => at = n.right.as_deref(),
                Ordering::Equal => return Some(&n.entry.1),
            }
        }
        None
    }
    pub fn contains_key<BK>(&self, k: &BK) -> bool
    where
        BK: Ord + ?Sized,
        K: Borrow<BK>,
    {
        self.get(k).is_some()
    }
}

impl<K: Ord + Clone, V: Clone> Map<K, V> {
    /// A map of one entry.
    pub fn unit(k: K, v: V) -> Self {
        Map { root: Some(Rc::new(Node::leaf(k, v))) }
    }
    /// Set `k` to `v` in this map; the value `k` had, if any.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        insert(&mut self.root, k, v)
    }
    /// Take `k` out of this map; the value it had, if any. A key the map does not hold
    /// copies nothing.
    pub fn remove<BK>(&mut self, k: &BK) -> Option<V>
    where
        BK: Ord + ?Sized,
        K: Borrow<BK>,
    {
        if !self.contains_key(k) {
            return None;
        }
        Some(remove(&mut self.root, k).1)
    }
    /// The value at `k`, to update in place (the path to it this map's own first); `None`
    /// when `k` is not a key, with nothing copied.
    pub fn get_mut<BK>(&mut self, k: &BK) -> Option<&mut V>
    where
        BK: Ord + ?Sized,
        K: Borrow<BK>,
    {
        if !self.contains_key(k) {
            return None;
        }
        get_mut(&mut self.root, k)
    }
    /// The map with `k` set to `v`; this one is unchanged.
    #[must_use]
    pub fn update(&self, k: K, v: V) -> Self {
        let mut m = self.clone();
        m.insert(k, v);
        m
    }
    /// The map without `k`; this one is unchanged.
    #[must_use]
    pub fn without<BK>(&self, k: &BK) -> Self
    where
        BK: Ord + ?Sized,
        K: Borrow<BK>,
    {
        let mut m = self.clone();
        m.remove(k);
        m
    }
    /// Both maps' entries, `other`'s over this one's on a shared key (as `extend` gives
    /// them): the prelude's `Union a, b` is `b.union(a)`. The smaller map's entries go
    /// into the larger.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if other.len() <= self.len() {
            let mut m = self;
            for (k, v) in other {
                m.insert(k, v);
            }
            m
        } else {
            let mut m = other;
            for (k, v) in self {
                if !m.contains_key(&k) {
                    m.insert(k, v);
                }
            }
            m
        }
    }
}

/// The entries in key order, as `(&K, &V)`: the nodes whose left subtrees are done, on a
/// stack.
pub struct Iter<'a, K, V> {
    stack: Vec<&'a Node<K, V>>,
    left: usize,
}

impl<'a, K, V> Iter<'a, K, V> {
    fn descend(&mut self, mut at: &'a Link<K, V>) {
        while let Some(n) = at {
            self.stack.push(n);
            at = &n.left;
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        let n = self.stack.pop()?;
        self.descend(&n.right);
        self.left -= 1;
        Some((&n.entry.0, &n.entry.1))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

impl<'a, K, V> IntoIterator for &'a Map<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The entries in key order, by value: each node's entry moved out when the map held it
/// alone, cloned when another map shares it.
pub struct IntoIter<K, V> {
    stack: Vec<Rc<Node<K, V>>>,
    left: usize,
}

impl<K, V> IntoIter<K, V> {
    fn descend(&mut self, mut at: Link<K, V>) {
        while let Some(n) = at {
            at = n.left.clone();
            self.stack.push(n);
        }
    }
}

impl<K: Clone, V: Clone> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        let n = self.stack.pop()?;
        self.left -= 1;
        match Rc::try_unwrap(n) {
            Ok(mut n) => {
                n.left = None;
                self.descend(n.right.take());
                Some(n.entry)
            }
            Err(n) => {
                self.descend(n.right.clone());
                Some(n.entry.clone())
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl<K: Clone, V: Clone> ExactSizeIterator for IntoIter<K, V> {}

impl<K: Clone, V: Clone> IntoIterator for Map<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(mut self) -> IntoIter<K, V> {
        let mut it = IntoIter { stack: Vec::new(), left: self.len() };
        it.descend(self.root.take());
        it
    }
}

impl<K, V, RK, RV> FromIterator<(RK, RV)> for Map<K, V>
where
    K: Ord + Clone + From<RK>,
    V: Clone + From<RV>,
{
    fn from_iter<I: IntoIterator<Item = (RK, RV)>>(i: I) -> Self {
        let mut m = Map::new();
        m.extend(i);
        m
    }
}

impl<K, V, RK, RV> Extend<(RK, RV)> for Map<K, V>
where
    K: Ord + Clone + From<RK>,
    V: Clone + From<RV>,
{
    fn extend<I: IntoIterator<Item = (RK, RV)>>(&mut self, i: I) {
        for (k, v) in i {
            self.insert(K::from(k), V::from(v));
        }
    }
}

impl<K, V, BK> Index<&BK> for Map<K, V>
where
    BK: Ord + ?Sized,
    K: Borrow<BK>,
{
    type Output = V;
    fn index(&self, k: &BK) -> &V {
        self.get(k).expect("Map::index: a key the map does not hold")
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for Map<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<K: Eq, V: Eq> Eq for Map<K, V> {}

impl<K: PartialOrd, V: PartialOrd> PartialOrd for Map<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<K: Ord, V: Ord> Ord for Map<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<K: Hash, V: Hash> Hash for Map<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for e in self.iter() {
            e.hash(state);
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Map<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}
