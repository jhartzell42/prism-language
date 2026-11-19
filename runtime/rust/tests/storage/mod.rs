//! Written by Claude on the claude-self-hosting branch; subject to change.
//!
//! The storage a test holds: this test binary's allocator counts the bytes live, and
//! [`held`] answers how many a closure's result keeps.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

thread_local! {
    static LIVE: Cell<isize> = const { Cell::new(0) };
}

// SAFETY: every call goes to `System`, with the caller's layout unchanged; the counter is a
// thread-local `Cell` of plain integers (no allocation, no reentrancy into the allocator).
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let _ = LIVE.try_with(|n| n.set(n.get() + l.size() as isize));
        // SAFETY: the caller's contract for `alloc`, passed through.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let _ = LIVE.try_with(|n| n.set(n.get() - l.size() as isize));
        // SAFETY: the caller's contract for `dealloc`, passed through.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let _ = LIVE.try_with(|n| n.set(n.get() + new as isize - l.size() as isize));
        // SAFETY: the caller's contract for `realloc`, passed through.
        unsafe { System.realloc(p, l, new) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// The bytes `f` leaves allocated while its result lives.
pub fn held<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let before = LIVE.with(Cell::get);
    let r = f();
    let after = LIVE.with(Cell::get);
    (r, (after - before).max(0) as usize)
}
