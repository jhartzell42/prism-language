//! Written by Claude on the claude-self-hosting branch; subject to change.
//!
//! `Array` at the sizes around its chunks' edges (empty, 1, 2, 63, 64, 65, ...), through
//! the calls emitted code and the runtime make: built by `push_back`, `push_front`, a `Vec`
//! and an iterator; a clone that shares the elements written at each size; `take` and
//! `skip` then written; `append`, also of an array to itself; `set`, `update`, indexing in
//! place, iteration, equality and order; and the storage an array holds, which must be
//! linear in its length, never a whole chunk for a few elements; and the costs, by counting
//! the elements cloned. The storage is counted by this test binary's allocator (`storage`).

mod storage;

use prism_rtl::Array;
use storage::held;

const SIZES: [usize; 20] = [0, 1, 2, 3, 4, 7, 8, 31, 32, 63, 64, 65, 126, 127, 128, 129, 191, 192, 193, 300];

/// An element of a compiler's size (a `Ty`, an `Ex`): 64 bytes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Big([u64; 8]);

fn big(i: usize) -> Big {
    Big([i as u64; 8])
}

fn counting(n: usize) -> Array<usize> {
    let mut a = Array::new();
    for i in 0..n {
        a.push_back(i);
    }
    a
}

fn counts(a: &Array<usize>, n: usize, from: usize) -> bool {
    a.len() == n
        && a.iter().copied().eq(from..from + n)
        && (0..n).all(|i| a[i] == from + i && a.get(i) == Some(&(from + i)))
        && a.get(n).is_none()
        && a.is_empty() == (n == 0)
}

#[test]
fn an_array_holds_its_elements_at_every_size_around_a_chunk() {
    for n in SIZES {
        let pushed = counting(n);
        let mut fronted = Array::new();
        for i in (0..n).rev() {
            fronted.push_front(i);
        }
        let from_vec = Array::from((0..n).collect::<Vec<_>>());
        let collected: Array<usize> = (0..n).collect();
        for a in [&pushed, &fronted, &from_vec, &collected] {
            assert!(counts(a, n, 0), "n = {n}");
            assert_eq!(a, &pushed);
            assert_eq!(a.cmp(&pushed), std::cmp::Ordering::Equal);
        }
        assert_eq!(pushed.iter().sum::<usize>(), (0..n).sum());
        assert!(counting(n) < counting(n + 1));
        let mut lower = counting(n + 1);
        lower.set(n, 0);
        assert!(n == 0 || lower < counting(n + 1));
    }
}

#[test]
fn a_shared_clone_written_at_every_size_leaves_its_sharer_alone() {
    for n in SIZES {
        let mut a = counting(n);
        let mut b = a.clone();
        b.push_back(usize::MAX);
        a.push_back(n);
        assert!(counts(&a, n + 1, 0), "n = {n}");
        assert!(counts(&b.take(n), n, 0) && b[n] == usize::MAX);
        let mut c = a.clone();
        c.push_front(7);
        assert!(counts(&a, n + 1, 0) && c[0] == 7 && counts(&c.skip(1), n + 1, 0));
        let mut d = a.clone();
        d.set(n, 99);
        assert!(counts(&a, n + 1, 0) && d[n] == 99);
        let mut e = a.clone();
        e[0] = 5;
        assert!(counts(&a, n + 1, 0) && e[0] == 5 && counts(&e.skip(1), n, 1));
        assert!(a.update(0, 9)[0] == 9 && a[0] == 0);
        assert_ne!(a, b);
    }
}

#[test]
fn a_taken_or_skipped_array_written_copies_only_its_own_elements() {
    for n in SIZES {
        let a = counting(n);
        for k in [0, 1.min(n), n / 2, n] {
            let mut front = a.take(k);
            let mut back = a.skip(k);
            assert!(counts(&front, k, 0) && counts(&back, n - k, k), "n = {n}, k = {k}");
            front.push_back(usize::MAX);
            back.push_back(usize::MAX - 1);
            assert!(counts(&a, n, 0));
            assert!(counts(&front.take(k), k, 0) && front[k] == usize::MAX);
            assert!(counts(&back.take(n - k), n - k, k) && back[n - k] == usize::MAX - 1);
            let mut joined = front.take(k);
            joined.append(back.take(n - k));
            assert_eq!(joined, a);
        }
    }
}

#[test]
fn appending_at_every_size_also_an_array_to_itself() {
    for n in SIZES {
        let a = counting(n);
        let mut twice = a.clone();
        twice.append(a.clone());
        assert!(counts(&a, n, 0));
        assert!(twice.len() == 2 * n && counts(&twice.take(n), n, 0) && counts(&twice.skip(n), n, 0));
        let mut rest = counting(n + 5).skip(n);
        let mut joined = a.clone();
        joined.append(rest.clone());
        assert!(counts(&joined, n + 5, 0));
        rest.append(Array::new());
        assert!(counts(&rest, 5, n));
        let mut empty = Array::new();
        empty.append(a.clone());
        assert_eq!(empty, a);
    }
}

/// The storage an array of `n` 64-byte elements may hold: linear in `n`, about twice it
/// at worst while the array is small, never whole chunks for a few elements; 256 bytes for
/// the array's own bookkeeping.
fn bound(n: usize) -> usize {
    256 + 2 * n * std::mem::size_of::<Big>()
}

#[test]
fn an_array_holds_storage_linear_in_its_length() {
    for n in SIZES {
        let (a, bytes) = held(|| {
            let mut a = Array::new();
            for i in 0..n {
                a.push_back(big(i));
            }
            a
        });
        assert!(bytes <= bound(n), "{n} elements pushed hold {bytes} bytes, more than {}", bound(n));
        let (_v, bytes) = held(|| Array::from((0..n).map(big).collect::<Vec<_>>()));
        assert!(bytes <= bound(n), "{n} elements from a Vec hold {bytes} bytes, more than {}", bound(n));
        // a write to a shared array copies the elements there are, into storage of their size
        let (_b, bytes) = held(|| {
            let mut b = a.clone();
            b.push_back(big(n));
            b
        });
        assert!(bytes <= bound(n + 1), "a shared {n}-element array pushed onto holds {bytes} bytes more");
        let (_c, bytes) = held(|| {
            let mut c = a.clone();
            c.push_front(big(n));
            c
        });
        assert!(bytes <= bound(n + 1), "a shared {n}-element array pushed in front of holds {bytes} bytes more");
        if n > 0 {
            let (_d, bytes) = held(|| a.update(0, big(9)));
            assert!(bytes <= bound(n), "a shared {n}-element array updated holds {bytes} bytes more");
        }
    }
}

// ---- Costs (Semantics Appendix B, Arrays), by counting the elements cloned ------------------

thread_local! {
    static CLONES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// An element that counts its clones.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Counted(usize);

impl Clone for Counted {
    fn clone(&self) -> Counted {
        CLONES.with(|c| c.set(c.get() + 1));
        Counted(self.0)
    }
}

fn clones<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let before = CLONES.with(|c| c.get());
    let r = f();
    (r, CLONES.with(|c| c.get()) - before)
}

fn counted(n: usize, from: usize) -> Array<Counted> {
    let mut a = Array::new();
    for i in 0..n {
        a.push_back(Counted(from + i));
    }
    a
}

fn counts_from(a: &Array<Counted>, n: usize, from: usize) -> bool {
    a.len() == n && a.iter().map(|c| c.0).eq(from..from + n)
}

/// The elements one operation may clone whatever the array's length: a chunk's worth (64)
/// a few times over, and never the array.
const BOUNDED: usize = 256;
const BIG: usize = 20000;

#[test]
fn a_push_onto_any_array_clones_a_bounded_number_of_elements() {
    let a = counted(BIG, 0);
    for end in [0usize, 1, 2] {
        let (b, n) = clones(|| {
            let mut b = a.clone();
            b.push_back(Counted(usize::MAX - end));
            b
        });
        assert!(n <= BOUNDED, "a push onto a shared array cloned {n} elements");
        assert!(b.len() == BIG + 1 && b[BIG].0 == usize::MAX - end);
    }
    let ((front, back), n) = clones(|| {
        let mut front = a.take(BIG / 2);
        front.push_back(Counted(7));
        let mut back = a.skip(10);
        back.push_back(Counted(8));
        (front, back)
    });
    assert!(n <= 2 * BOUNDED, "pushes onto a taken and a skipped array cloned {n} elements");
    assert!(counts_from(&front.take(BIG / 2), BIG / 2, 0) && front[BIG / 2].0 == 7);
    assert!(counts_from(&back.take(BIG - 10), BIG - 10, 10) && back[BIG - 10].0 == 8);
    assert!(counts_from(&a, BIG, 0));
}

#[test]
fn appending_two_large_arrays_clones_neither() {
    let a = counted(BIG, 0);
    let b = counted(BIG, BIG);
    let ((ab, aa), n) = clones(|| {
        let mut ab = a.clone();
        ab.append(b.clone());
        let mut aa = a.clone();
        aa.append(a.clone());
        (ab, aa)
    });
    assert!(n <= 2 * BOUNDED, "two appends of {BIG}-element arrays cloned {n} elements");
    assert!(counts_from(&ab, 2 * BIG, 0));
    assert!(counts_from(&aa.take(BIG), BIG, 0) && counts_from(&aa.skip(BIG), BIG, 0));
    assert!(counts_from(&a, BIG, 0) && counts_from(&b, BIG, BIG));
}

#[test]
fn a_write_to_a_shared_array_clones_a_bounded_number_of_elements() {
    let a = counted(BIG, 0);
    for i in [0, BIG / 2, BIG - 1] {
        let (b, n) = clones(|| {
            let mut b = a.clone();
            b.set(i, Counted(usize::MAX));
            b[BIG - 1 - i].0 = 1;
            b
        });
        assert!(n <= 2 * BOUNDED, "two writes to a shared array cloned {n} elements");
        assert!(b[i].0 == usize::MAX && b[BIG - 1 - i].0 == 1 && counts_from(&a, BIG, 0));
    }
}

#[test]
fn an_array_from_a_vec_clones_nothing_into_storage_of_its_size() {
    for n in SIZES {
        let v: Vec<Big> = (0..n).map(big).collect();
        let (a, bytes) = held(|| Array::from(v));
        // the elements' own bytes, and 128 for each chunk of 64 (its count and node)
        assert!(bytes <= 256 + n * std::mem::size_of::<Big>() + n / 64 * 128, "{n} elements from a Vec hold {bytes} bytes");
        assert!(a.iter().cloned().eq((0..n).map(big)));
        let (_c, k) = clones(|| Array::from((0..n).map(Counted).collect::<Vec<_>>()));
        assert_eq!(k, 0);
    }
}

#[test]
fn iter_mut_updates_every_element_of_this_array_only() {
    for n in SIZES {
        let a = counting(n);
        let mut b = a.clone();
        for (i, x) in b.iter_mut().enumerate() {
            *x += i + 1;
        }
        assert!(counts(&a, n, 0));
        assert!(b.iter().copied().eq((0..n).map(|i| 2 * i + 1)), "n = {n}");
    }
}

#[test]
fn arrays_cut_and_joined_in_many_ways_hold_their_elements_in_order() {
    // pieces of every size, joined in order, then cut at every 37th place both ways
    let mut whole: Array<usize> = Array::new();
    let mut next = 0;
    for size in (0..40).chain([63, 64, 65, 127, 128, 129, 500, 1000]) {
        let piece: Array<usize> = (next..next + size).collect();
        next += size;
        whole.append(piece);
        assert!(counts(&whole, next, 0), "after a piece of {size}");
    }
    for k in (0..=next).step_by(37) {
        assert!(counts(&whole.take(k), k, 0) && counts(&whole.skip(k), next - k, k), "k = {k}");
        let mut rejoined = whole.take(k);
        rejoined.append(whole.skip(k));
        assert!(counts(&rejoined, next, 0));
        let mut fronted = whole.skip(k);
        for i in (0..k.min(70)).rev() {
            fronted.push_front(k - 70.min(k) + i);
        }
        assert!(counts(&fronted, next - k + k.min(70), k - k.min(70)));
    }
    let owned: Vec<usize> = whole.clone().into_iter().collect();
    assert!(owned.into_iter().eq(0..next));
}

// ---- Equality on shared structure, by counting the elements compared ----------------------

thread_local! {
    static COMPARED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// An element that counts its comparisons; `Cmp(NAN)` is equal to nothing, itself
/// included, as a `Float` NaN is under IEEE equality (Type System ch. 2).
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

fn cmps(n: usize) -> Array<Cmp> {
    let mut a = Array::new();
    for i in 0..n {
        a.push_back(Cmp(i));
    }
    a
}

#[test]
fn an_array_compared_with_what_shares_its_nodes_compares_a_bounded_number_of_elements() {
    for n in [0, 1, 64, 65, 300, BIG] {
        let a = cmps(n);
        let b = a.clone();
        // the first comparison may look at every element once; after it, shared nodes
        // answer at once
        assert!(a == b);
        let (eq, k) = compared(|| a == b);
        assert!(eq && k <= BOUNDED, "an array of {n} compared with its clone compared {k} elements");
        if n > 0 {
            let c = a.update(n / 2, Cmp(n / 2));
            let (eq, k) = compared(|| a == c);
            assert!(eq && k <= BOUNDED, "an array of {n} compared with an equal update compared {k} elements");
            let d = a.update(n / 2, Cmp(n + 1));
            let (eq, k) = compared(|| a == d);
            assert!(!eq && k <= BOUNDED, "an array of {n} compared with a changed update compared {k} elements");
        }
    }
}

#[test]
fn a_shared_array_holding_a_nan_is_not_equal_to_itself() {
    for n in [1, 64, 65, 300, BIG] {
        let a = cmps(n).update(n - 1, Cmp(NAN));
        let b = a.clone();
        assert!(a != b && a != b, "an array of {n} with a NaN at its end");
        let c = cmps(n).update(0, Cmp(NAN));
        let tail = c.skip(0);
        assert!(c != tail && c != c.clone(), "an array of {n} with a NaN at its start");
    }
    let floats: Array<f64> = Array::from(vec![1.0, f64::NAN, 3.0]);
    assert!(floats != floats.clone());
}

#[test]
fn arrays_of_arrays_compare_shared_inner_arrays_at_once() {
    let inner = cmps(BIG);
    let outer = |k: usize| -> Array<Array<Cmp>> { (0..10).map(|_| inner.clone()).chain([cmps(k)]).collect() };
    let (a, b) = (outer(3), outer(3));
    assert!(a == b);
    let (eq, k) = compared(|| a == b);
    assert!(eq && k <= BOUNDED, "two arrays of ten shared inner arrays compared {k} elements");
}
