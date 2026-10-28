//! Minimal C-ABI surface for `wasm32-unknown-unknown`, loadable with plain
//! `WebAssembly.instantiate` (no wasm-bindgen).
//!
//! Memory protocol: the host calls `alloc(len)`, copies bytes into linear memory, and frees
//! with `dealloc(ptr, len)`. Buffers returned by the module (`hello_pdf`) are freed the same
//! way, using the length written to `out_len`.
#![expect(unsafe_code, reason = "FFI boundary: raw pointers from the WASM host")]

use std::alloc::Layout;

/// Alignment of every buffer handed to the host: enough for any scalar the host may store
/// in one (e.g. `hello_pdf`'s `usize` out-parameter, or an `f64`).
const ALIGN: usize = 8;

/// The layout `alloc`/`dealloc` use for a `len`-byte buffer (`None` if `len` is too large).
fn layout(len: usize) -> Option<Layout> {
    Layout::from_size_align(len, ALIGN).ok()
}

/// Copies `bytes` into a fresh [`alloc`] buffer. Returns null if the allocation fails.
fn into_host_buffer(bytes: &[u8]) -> *mut u8 {
    let ptr = alloc(bytes.len());
    if !ptr.is_null() && !bytes.is_empty() {
        // SAFETY: `ptr` is a fresh, non-null allocation of exactly `bytes.len()` bytes,
        // so it is valid for that many writes and cannot overlap `bytes`.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
    }
    ptr
}

/// Allocates `len` bytes aligned to 8 and returns a pointer to them, or null when the
/// allocation fails or `len` is too large. For `len == 0` it returns a non-null, aligned,
/// dangling pointer that must not be read or written (`dealloc(ptr, 0)` is a no-op).
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let Some(layout) = layout(len) else {
        return std::ptr::null_mut();
    };
    if len == 0 {
        // `std::alloc::alloc` must not be called with a zero-size layout.
        return std::ptr::without_provenance_mut(ALIGN);
    }
    // SAFETY: `layout` has a non-zero size (checked above) and a valid alignment.
    unsafe { std::alloc::alloc(layout) }
}

/// Frees a buffer obtained from [`alloc`] or [`hello_pdf`]. Null pointers and `len == 0`
/// are ignored.
///
/// # Safety
/// `ptr` must come from [`alloc`]`(len)` or be a `hello_pdf` result of length `len`,
/// and must not be freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let Some(layout) = layout(len) else {
        return;
    };
    // SAFETY: per the contract above, `ptr` was returned by `alloc(len)` (directly or via
    // `hello_pdf`), i.e. by `std::alloc::alloc` with this exact layout, and is still live.
    unsafe { std::alloc::dealloc(ptr, layout) };
}

/// Builds the golden `hello.pdf`, writes its length to `*out_len` and returns the bytes
/// (free with `dealloc(ptr, *out_len)`). Returns null with `*out_len = 0` on failure.
///
/// # Safety
/// `out_len` must be valid for writing a `usize` (4 bytes on wasm32). It need not be
/// aligned: the value is written with an unaligned store.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hello_pdf(out_len: *mut usize) -> *mut u8 {
    let (ptr, len) = match crate::samples::hello() {
        Ok(bytes) => match into_host_buffer(&bytes) {
            p if p.is_null() => (p, 0),
            p => (p, bytes.len()),
        },
        Err(_) => (std::ptr::null_mut(), 0),
    };
    // SAFETY: the caller guarantees `out_len` is valid for a `usize` write; alignment is
    // not required by `write_unaligned`.
    unsafe { out_len.write_unaligned(len) };
    ptr
}

/// Opens the `len` bytes at `ptr` and returns the page count, or -1 on any error.
///
/// # Safety
/// `ptr` must point at `len` initialized bytes (any pointer is fine when `len == 0`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn page_count(ptr: *const u8, len: usize) -> i32 {
    let data = if len == 0 || ptr.is_null() {
        &[][..]
    } else {
        // SAFETY: the caller guarantees `ptr..ptr+len` is initialized and live.
        unsafe { std::slice::from_raw_parts(ptr, len) }
    };
    crate::open(data)
        .and_then(|doc| doc.page_count())
        .ok()
        .and_then(|n| i32::try_from(n).ok())
        .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_should_return_eight_byte_aligned_buffers() {
        let ptrs: Vec<(*mut u8, usize)> = (1..=33).map(|len| (alloc(len), len)).collect();
        let aligned = ptrs.iter().all(|(p, _)| !p.is_null() && p.addr().is_multiple_of(ALIGN));
        for (p, len) in ptrs {
            // SAFETY: each `p` came from `alloc(len)` above and is freed once.
            unsafe { dealloc(p, len) };
        }
        assert!(aligned);
    }

    #[test]
    fn alloc_should_return_non_null_dangling_pointer_for_zero_length() {
        let p = alloc(0);
        // SAFETY: `dealloc(_, 0)` is documented as a no-op.
        unsafe { dealloc(p, 0) };
        assert!(!p.is_null() && p.addr().is_multiple_of(ALIGN));
    }

    #[test]
    fn alloc_should_return_null_when_length_overflows_layout() {
        assert!(alloc(usize::MAX).is_null());
    }

    #[test]
    fn hello_pdf_should_write_length_through_unaligned_pointer_and_match_samples() {
        let mut slot = [0u8; 1 + size_of::<usize>()];
        // Deliberately misaligned out-parameter (offset 1 into a byte buffer).
        let out_len = slot.as_mut_ptr().wrapping_add(1).cast::<usize>();
        // SAFETY: `out_len` points at `size_of::<usize>()` writable bytes inside `slot`.
        let ptr = unsafe { hello_pdf(out_len) };
        // SAFETY: `hello_pdf` just wrote a `usize` there; read it back the same way.
        let len = unsafe { out_len.read_unaligned() };
        let bytes = if ptr.is_null() {
            Vec::new()
        } else {
            // SAFETY: `hello_pdf` returned `len` initialized bytes at `ptr`.
            unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
        };
        // SAFETY: `ptr` is a `hello_pdf` result of length `len`, freed once.
        unsafe { dealloc(ptr, len) };
        assert_eq!(Ok(bytes), crate::samples::hello());
    }
}
