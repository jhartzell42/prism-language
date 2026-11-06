//! Written by Claude on the claude-self-hosting branch; subject to change.
//!
//! `Map` at the sizes around a 64-entry node's edges (empty, 1, 2, 63, 64, 65, ...),
//! through the calls emitted code and the runtime make: keys inserted in an order that
//! is neither sorted nor reversed, looked up, removed, updated in place; a clone that
//! shares the entries written at each size; `update`, `without`, `get_min`, `union` (the
//! prelude's `Union a, b`, `a`'s entries winning), collecting pairs (the later winning),
//! iteration in key order, equality, order, hashing and `Debug`; and the storage a map
//! holds, which must be linear in its entries (nothing for an empty map).

mod storage;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use prism_rtl::Map;
use storage::held;

const SIZES: [usize; 18] = [0, 1, 2, 3, 4, 7, 8, 31, 32, 63, 64, 65, 127, 128, 129, 192, 193, 300];

/// `0..n` in a scattered order: `i * 7919 mod p` for a prime `p >= n`, kept below `n`.
fn scattered(n: usize) -> Vec<usize> {
    let p = (n.max(2)..).find(|&p| (2..p).take_while(|d| d * d <= p).all(|d| p % d != 0)).unwrap();
    (0..p).map(|i| i * 7919 % p).filter(|&k| k < n).collect()
}

fn built(n: usize) -> Map<usize, String> {
    let mut m = Map::new();
    for k in scattered(n) {
        assert_eq!(m.insert(k, format!("v{k}")), None);
    }
    m
}

fn holds(m: &Map<usize, String>, keys: impl Iterator<Item = usize> + Clone, tag: &str) -> bool {
    let n = keys.clone().count();
    m.len() == n
        && m.is_empty() == (n == 0)
        && m.iter().map(|(k, _)| *k).eq(keys.clone())
        && m.keys().copied().eq(keys.clone())
        && m.values().cloned().eq(keys.clone().map(|k| format!("{tag}{k}")))
        && keys.clone().all(|k| m.get(&k) == Some(&format!("{tag}{k}")) && m.contains_key(&k) && m[&k] == format!("{tag}{k}"))
        && m.get_min().map(|(k, _)| *k) == keys.clone().next()
}

#[test]
fn a_map_holds_its_entries_in_key_order_at_every_size() {
    for n in SIZES {
        let m = built(n);
        assert!(holds(&m, 0..n, "v"), "n = {n}");
        assert!(m.get(&n).is_none() && !m.contains_key(&n));
        let collected: Map<usize, String> = scattered(n).into_iter().map(|k| (k, format!("v{k}"))).collect();
        assert_eq!(collected, m);
        let by_value: Vec<(usize, String)> = m.clone().into_iter().collect();
        assert_eq!(by_value, (0..n).map(|k| (k, format!("v{k}"))).collect::<Vec<_>>());
        assert_eq!(m.iter().len(), n);
    }
}

#[test]
fn removing_at_every_size_keeps_the_rest_in_order() {
    for n in SIZES {
        let mut m = built(n);
        for k in scattered(n).into_iter().filter(|k| k % 3 == 0) {
            assert_eq!(m.remove(&k), Some(format!("v{k}")), "n = {n}, k = {k}");
            assert_eq!(m.remove(&k), None);
        }
        assert!(holds(&m, (0..n).filter(|k| k % 3 != 0), "v"), "n = {n}");
        for k in scattered(n) {
            m.remove(&k);
        }
        assert!(m.is_empty() && m.len() == 0 && m.get_min().is_none() && m.iter().next().is_none());
    }
}

#[test]
fn a_shared_clone_written_at_every_size_leaves_its_sharer_alone() {
    for n in SIZES {
        let m = built(n);
        let mut a = m.clone();
        a.insert(n, format!("v{n}"));
        let mut b = m.clone();
        if n > 0 {
            b.remove(&(n / 2));
            *a.get_mut(&0).unwrap() = "changed".to_string();
        }
        assert!(a.get_mut(&(n + 7)).is_none());
        assert!(holds(&m, 0..n, "v"), "n = {n}");
        assert_eq!(a.len(), n + 1);
        assert!(n == 0 || (a[&0] == "changed" && b.len() == n - 1 && !b.contains_key(&(n / 2))));
        let up = m.update(1, "up".to_string());
        let down = m.without(&1);
        assert!(holds(&m, 0..n, "v"));
        assert_eq!(up[&1], "up");
        assert_eq!(up.len(), if n >= 2 { n } else { n + 1 });
        assert_eq!(down.len(), if n >= 2 { n - 1 } else { n });
        assert!(!down.contains_key(&1));
    }
}

#[test]
fn union_keeps_the_entries_of_the_map_it_is_given_over_its_own() {
    for n in SIZES {
        for m_size in [0, 1, n / 2, n, n + 10] {
            let mine = built(n);
            let theirs: Map<usize, String> = (0..m_size).map(|k| (k, format!("w{k}"))).collect();
            // the prelude's `Union a, b` is `b.union(a)`: `a`'s entries win
            let u = mine.clone().union(theirs.clone());
            let all = n.max(m_size);
            assert_eq!(u.len(), all, "n = {n}, m = {m_size}");
            for k in 0..all {
                let want = if k < m_size { format!("w{k}") } else { format!("v{k}") };
                assert_eq!(u[&k], want, "n = {n}, m = {m_size}, k = {k}");
            }
            assert!(holds(&mine, 0..n, "v"));
        }
    }
}

#[test]
fn equality_order_hash_and_debug_go_by_the_entries_in_key_order() {
    let hash = |m: &Map<usize, String>| {
        let mut h = DefaultHasher::new();
        m.hash(&mut h);
        h.finish()
    };
    for n in SIZES {
        let a = built(n);
        let b: Map<usize, String> = (0..n).rev().map(|k| (k, format!("v{k}"))).collect();
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let more = a.update(n, "z".to_string());
        assert!(a < more && a != more);
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    }
    let m: Map<usize, &str> = [(2usize, "b"), (1, "a"), (2, "c")].into_iter().collect();
    assert_eq!(format!("{m:?}"), r#"{1: "a", 2: "c"}"#);
}

/// The storage a map of `n` entries of 72 bytes may hold: a node per entry, its entry and
/// about 24 bytes beside it (two links, a height and a count, the count's header).
fn bound(n: usize) -> usize {
    n * (std::mem::size_of::<(usize, [u64; 8])>() + 64)
}

#[test]
fn a_map_holds_storage_linear_in_its_entries() {
    for n in SIZES {
        let (m, bytes) = held(|| (0..n).map(|k| (k, [k as u64; 8])).collect::<Map<usize, [u64; 8]>>());
        assert!(bytes <= bound(n), "{n} entries hold {bytes} bytes, more than {}", bound(n));
        let (_u, bytes) = held(|| m.update(n, [0; 8]));
        let depth = 2 * (usize::BITS - n.leading_zeros()) as usize + 2;
        assert!(bytes <= bound(depth), "an update of a {n}-entry map holds {bytes} bytes more, more than its path");
    }
}

// ---- Equality on shared structure, by counting the entries compared -----------------------

thread_local! {
    static COMPARED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// A value that counts its comparisons; `Cmp(NAN)` is equal to nothing, itself included,
/// as a `Float` NaN is under IEEE equality (Type System ch. 2).
#[derive(Clone, Debug)]
struct Cmp(usize);

const NAN: usize = usize::MAX;

impl PartialEq for Cmp {
    fn eq(&self, other: &Cmp) -> bool {
        COMPARED.with(|c| c.set(c.get() + 1));
        self.0 == other.0 && self.0 != NAN
    }
}

fn compared<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let before = COMPARED.with(|c| c.get());
    let r = f();
    (r, COMPARED.with(|c| c.get()) - before)
}

fn cmps(n: usize) -> Map<usize, Cmp> {
    scattered(n).into_iter().map(|k| (k, Cmp(k))).collect()
}

/// The entries one comparison may look at whatever the map's size: a few per level of a
/// tree of 20,000 (height at most 21).
const BOUNDED: usize = 128;
const BIG: usize = 20000;

#[test]
fn a_map_compared_with_what_shares_its_nodes_compares_a_bounded_number_of_entries() {
    for n in [0, 1, 2, 64, 65, 300, BIG] {
        let a = cmps(n);
        let b = a.clone();
        // the first comparison may look at every entry once; after it, shared nodes answer
        // at once
        assert!(a == b);
        let (eq, k) = compared(|| a == b);
        assert!(eq && k <= BOUNDED, "a map of {n} compared with its clone compared {k} entries");
        if n > 0 {
            let c = a.update(n / 2, Cmp(n / 2));
            let (eq, k) = compared(|| a == c);
            assert!(eq && k <= BOUNDED, "a map of {n} compared with an equal update compared {k} entries");
            let d = a.update(n / 2, Cmp(n + 1));
            let (eq, k) = compared(|| a == d);
            assert!(!eq && k <= BOUNDED, "a map of {n} compared with a changed update compared {k} entries");
            let e = a.update(n, Cmp(n));
            let (eq, k) = compared(|| e.without(&n) == a);
            assert!(eq && k <= BOUNDED, "a map of {n} compared with itself grown and cut compared {k} entries");
        }
    }
}

#[test]
fn a_shared_map_holding_a_nan_is_not_equal_to_itself() {
    for n in [1, 2, 64, 65, 300, BIG] {
        let a = cmps(n).update(n / 3, Cmp(NAN));
        let b = a.clone();
        assert!(a != b && a != b, "a map of {n} with a NaN");
        assert!(a.update(n, Cmp(n)) != a.update(n, Cmp(n)));
    }
    let floats: Map<u8, f64> = [(1, 1.0), (2, f64::NAN)].into_iter().collect();
    assert!(floats != floats.clone());
}

#[test]
fn maps_of_maps_compare_shared_inner_maps_at_once() {
    let inner = cmps(BIG);
    let outer = || -> Map<usize, (Map<usize, Cmp>, u8)> { (0..10usize).map(|k| (k, (inner.clone(), 1u8))).collect() };
    let (a, b) = (outer(), outer());
    assert!(a == b);
    let (eq, k) = compared(|| a == b);
    assert!(eq && k <= BOUNDED, "two maps of ten shared inner maps (in a tuple, as a record holds them) compared {k} entries");
}
