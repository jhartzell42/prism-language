//! Written by Claude on the claude-self-hosting branch; subject to change.
//!
//! `Array` at the sizes around its chunks' edges (empty, 1, 2, 63, 64, 65, ...), through
//! the calls emitted code and the runtime make: built by `push_back`, `push_front`, a `Vec`
//! and an iterator; a clone that shares the elements written at each size; `take` and
//! `skip` then written; `append`, also of an array to itself; `set`, `update`, indexing in
//! place, iteration, equality and order; and the storage an array holds, which must be
//! linear in its length (claude/bugs/small-arrays-whole-chunks.md: every array takes whole
//! 64-element chunks). The storage is counted by this test binary's allocator (`storage`).

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
#[ignore = "the Rust half of claude/bugs/small-arrays-whole-chunks.md is open: im_rc::Vector holds 64-element chunks"]
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
