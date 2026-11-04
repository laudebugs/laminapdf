//! The module's memory, as `@laminapdf/sdk` needs to see it after a trap.
//!
//! A failed allocation aborts (stable wasm32 is `panic = "abort"`), and an abort is a
//! `RuntimeError: unreachable` like any other trap. The allocator below records the failure
//! first, so `allocationFailed()` tells the SDK which code to throw: `out-of-memory` (growth
//! past the link-time maximum, ADR 0016) or `wasm-trap`. The SDK reads it only after a trap,
//! and a trap is followed by a fresh instance, whose flag starts clear.
//!
//! With the `test-hooks` feature (never in the shipped modules: `cargo xtask wasm` checks their
//! exports), `__trap` and `__grow` cause each trap on demand.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, Ordering};

use wasm_bindgen::prelude::*;

/// Set when the allocator returned null.
static ALLOCATION_FAILED: AtomicBool = AtomicBool::new(false);

/// The system allocator, recording a failure in [`ALLOCATION_FAILED`].
struct Recording;

/// `ptr`, noting when it is null (a failed allocation).
fn record(ptr: *mut u8) -> *mut u8 {
    if ptr.is_null() {
        ALLOCATION_FAILED.store(true, Ordering::Relaxed);
    }
    ptr
}

#[expect(
    unsafe_code,
    reason = "a global allocator is an unsafe trait; this one forwards to System"
)]
// SAFETY: every method forwards its arguments unchanged to `System`, which upholds the
// `GlobalAlloc` contract, and returns what `System` returned.
unsafe impl GlobalAlloc for Recording {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds `alloc`'s contract, the one `System.alloc` needs.
        record(unsafe { System.alloc(layout) })
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as for `alloc`.
        record(unsafe { System.alloc_zeroed(layout) })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller passes a pointer this allocator (so `System`) returned, with its layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller upholds `realloc`'s contract for a pointer `System` returned.
        record(unsafe { System.realloc(ptr, layout, new_size) })
    }
}

// Every target, so the native unit tests run on it too.
#[global_allocator]
static ALLOCATOR: Recording = Recording;

/// Whether an allocation has failed in this instance: after a trap, `true` means it ran out of
/// memory.
#[wasm_bindgen(js_name = allocationFailed)]
pub fn allocation_failed() -> bool {
    ALLOCATION_FAILED.load(Ordering::Relaxed)
}

/// Traps at once, as a bug would (`test-hooks` builds only).
#[cfg(feature = "test-hooks")]
#[wasm_bindgen(js_name = __trap)]
pub fn trap() {
    std::process::abort();
}

/// Allocates `bytes` more and never frees it, in 16 MiB blocks, so growing past the module's
/// maximum memory fails an allocation and traps (`test-hooks` builds only).
#[cfg(feature = "test-hooks")]
#[wasm_bindgen(js_name = __grow)]
pub fn grow(bytes: usize) {
    const BLOCK: usize = 16 << 20;
    let mut left = bytes;
    while left > 0 {
        let block = left.min(BLOCK);
        // Reserved, never written: the engine maps the pages but needn't back them.
        std::mem::forget(Vec::<u8>::with_capacity(block));
        left -= block;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_null_pointer_should_set_the_flag_and_a_valid_one_should_not() {
        let mut byte = 0_u8;
        assert_eq!(record(&raw mut byte), &raw mut byte);
        assert!(!allocation_failed());
        assert!(record(std::ptr::null_mut()).is_null());
        assert!(allocation_failed());
    }
}
