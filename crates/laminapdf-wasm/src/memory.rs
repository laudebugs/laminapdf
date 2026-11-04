//! The module's memory, as `@laminapdf/sdk` needs to see it after a trap.
//!
//! A failed allocation aborts (stable wasm32 is `panic = "abort"`), and an abort is a
//! `RuntimeError: unreachable` like any other trap. The allocator below records whether the
//! most recent allocation failed, so `allocationFailed()` tells the SDK which code to throw:
//! `out-of-memory` (growth past the link-time maximum, ADR 0016 §8) or `wasm-trap`. A failure
//! that code handles (`try_reserve`) is cleared by the next allocation that succeeds, so it never
//! makes a later, unrelated trap read as out-of-memory.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, Ordering};

use wasm_bindgen::prelude::*;

/// Whether the most recent allocation returned null.
static ALLOCATION_FAILED: AtomicBool = AtomicBool::new(false);

/// The system allocator, recording each allocation's outcome in [`ALLOCATION_FAILED`].
struct Recording;

/// `ptr`, noting in `flag` whether it is null (a failed allocation).
fn record(flag: &AtomicBool, ptr: *mut u8) -> *mut u8 {
    flag.store(ptr.is_null(), Ordering::Relaxed);
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
        record(&ALLOCATION_FAILED, unsafe { System.alloc(layout) })
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as for `alloc`.
        record(&ALLOCATION_FAILED, unsafe { System.alloc_zeroed(layout) })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller passes a pointer this allocator (so `System`) returned, with its layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller upholds `realloc`'s contract for a pointer `System` returned.
        record(&ALLOCATION_FAILED, unsafe { System.realloc(ptr, layout, new_size) })
    }
}

// Every target, so the native unit tests run on it too.
#[global_allocator]
static ALLOCATOR: Recording = Recording;

/// Whether the most recent allocation failed: after a trap, `true` means the module ran out of
/// memory.
#[wasm_bindgen(js_name = allocationFailed)]
pub fn allocation_failed() -> bool {
    ALLOCATION_FAILED.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A flag of the test's own: the global one sees every allocation of the parallel tests.
    #[test]
    fn a_null_pointer_should_set_the_flag_and_the_next_allocation_should_clear_it() {
        let flag = AtomicBool::new(false);
        let mut byte = 0_u8;
        assert!(record(&flag, std::ptr::null_mut()).is_null());
        assert!(flag.load(Ordering::Relaxed));
        assert_eq!(record(&flag, &raw mut byte), &raw mut byte);
        assert!(!flag.load(Ordering::Relaxed));
    }
}
