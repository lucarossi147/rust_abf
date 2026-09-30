//! A `#[global_allocator]` wrapper that tracks the bytes allocated by the thread
//! that is currently running [`peak_bytes_during`].
//!
//! This module is only ever included (via `#[path]`) into a dedicated test binary
//! so that installing it as the global allocator doesn't affect any other test binary.
//!
//! Counting is strictly per-thread: only allocations and deallocations made by a
//! thread while it is inside `peak_bytes_during` are recorded, so other threads
//! (the test harness's output capture, other tests running in parallel, ...)
//! cannot pollute a measurement. The per-thread state lives in `const`-initialised
//! `thread_local!` cells of plain `Cell`s, which need neither lazy initialisation
//! nor a destructor, so touching them never allocates from inside the allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// Whether this thread is currently inside `peak_bytes_during`.
    static MEASURING: Cell<bool> = const { Cell::new(false) };
    /// Net bytes allocated by this thread since the measurement started
    /// (can go negative if the closure frees memory allocated before it).
    static CURRENT: Cell<isize> = const { Cell::new(0) };
    /// High-water mark of `CURRENT` (never below 0, the starting point).
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

/// A `GlobalAlloc` wrapper around [`System`] that, while a thread is measuring,
/// tracks that thread's net allocated bytes and their peak.
pub struct CountingAllocator;

impl CountingAllocator {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CountingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

fn record_alloc(size: usize) {
    // `try_with` so that access during thread teardown is a silent no-op.
    let _ = MEASURING.try_with(|measuring| {
        if measuring.get() {
            let _ = CURRENT.try_with(|current| {
                let new_current = current.get() + size as isize;
                current.set(new_current);
                let _ = PEAK.try_with(|peak| {
                    if new_current > peak.get() {
                        peak.set(new_current);
                    }
                });
            });
        }
    });
}

fn record_dealloc(size: usize) {
    let _ = MEASURING.try_with(|measuring| {
        if measuring.get() {
            let _ = CURRENT.try_with(|current| current.set(current.get() - size as isize));
        }
    });
}

// SAFETY: every call simply forwards to `System`, whose `GlobalAlloc` impl is
// sound for these arguments; we only add bookkeeping around it.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            record_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        record_dealloc(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            record_alloc(layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            record_dealloc(layout.size());
            record_alloc(new_size);
        }
        new_ptr
    }
}

/// Runs `f`, returning its result together with the peak number of bytes
/// allocated *by the calling thread* while it ran, relative to the start of the
/// call (so the baseline is always 0; memory allocated beforehand is not
/// counted, and memory freed during `f` reduces the running total).
///
/// Allocations made by other threads are ignored. `_alloc` must be the allocator
/// installed via `#[global_allocator]` in the calling binary; it is only taken
/// to make that requirement explicit at call sites. Calls must not be nested.
pub fn peak_bytes_during<T>(_alloc: &CountingAllocator, f: impl FnOnce() -> T) -> (T, usize) {
    CURRENT.with(|c| c.set(0));
    PEAK.with(|p| p.set(0));
    MEASURING.with(|m| m.set(true));
    let result = f();
    MEASURING.with(|m| m.set(false));
    let peak = PEAK.with(|p| p.get()).max(0) as usize;
    (result, peak)
}
