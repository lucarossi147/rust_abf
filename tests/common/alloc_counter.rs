//! A `#[global_allocator]` wrapper that tracks current and peak bytes allocated.
//!
//! This module is only ever included (via `#[path]`) into a dedicated test binary
//! so that installing it as the global allocator doesn't affect any other test binary.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A `GlobalAlloc` wrapper around [`System`] that tracks the number of bytes
/// currently allocated and the peak number of bytes allocated since the last reset.
pub struct CountingAllocator {
    current_bytes: AtomicUsize,
    peak_bytes: AtomicUsize,
}

impl CountingAllocator {
    pub const fn new() -> Self {
        Self {
            current_bytes: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
        }
    }

    fn record_alloc(&self, size: usize) {
        let new_current = self.current_bytes.fetch_add(size, Ordering::SeqCst) + size;
        self.peak_bytes.fetch_max(new_current, Ordering::SeqCst);
    }

    fn record_dealloc(&self, size: usize) {
        self.current_bytes.fetch_sub(size, Ordering::SeqCst);
    }

    fn current_bytes(&self) -> usize {
        self.current_bytes.load(Ordering::SeqCst)
    }

    fn reset_peak(&self) {
        self.peak_bytes
            .store(self.current_bytes(), Ordering::SeqCst);
    }

    fn peak_bytes(&self) -> usize {
        self.peak_bytes.load(Ordering::SeqCst)
    }
}

impl Default for CountingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: every call simply forwards to `System`, whose `GlobalAlloc` impl is
// sound for these arguments; we only add bookkeeping around it.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            self.record_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        self.record_dealloc(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            self.record_alloc(layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            self.record_dealloc(layout.size());
            self.record_alloc(new_size);
        }
        new_ptr
    }
}

/// Runs `f`, returning its result together with the peak number of bytes
/// allocated (relative to the allocator's current baseline) while it ran.
///
/// `alloc` must be the same allocator installed via `#[global_allocator]` in
/// the calling binary, otherwise the measurement is meaningless.
pub fn peak_bytes_during<T>(alloc: &CountingAllocator, f: impl FnOnce() -> T) -> (T, usize) {
    alloc.reset_peak();
    let result = f();
    (result, alloc.peak_bytes())
}
