//! Minimal C-ABI surface for `wasm32-unknown-unknown`, loadable with plain
//! `WebAssembly.instantiate` (no wasm-bindgen).
//!
//! Memory protocol: the host calls `alloc(len)`, copies bytes into linear memory, and frees
//! with `dealloc(ptr, len)`. Buffers returned by the module (`hello_pdf`) are freed the same
//! way, using the length written to `out_len`.
#![expect(unsafe_code, reason = "FFI boundary: raw pointers from the WASM host")]

/// Allocates `len` bytes and returns a pointer to them (never null; dangling for `len == 0`).
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = std::mem::ManuallyDrop::new(Vec::<u8>::with_capacity(len));
    buf.as_mut_ptr()
}

/// Frees a buffer obtained from [`alloc`] or [`hello_pdf`].
///
/// # Safety
/// `ptr` must come from [`alloc`]`(len)` or be a `hello_pdf` result of length `len`,
/// and must not be freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: per the contract above, `ptr` owns an allocation of capacity `len`.
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
}

/// Builds the golden `hello.pdf`, writes its length to `*out_len` and returns the bytes
/// (free with `dealloc(ptr, *out_len)`). Returns null with `*out_len = 0` on failure.
///
/// # Safety
/// `out_len` must be a valid, aligned pointer to a `usize` in linear memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hello_pdf(out_len: *mut usize) -> *mut u8 {
    let (ptr, len) = match crate::samples::hello() {
        Ok(bytes) => {
            let boxed = bytes.into_boxed_slice();
            let len = boxed.len();
            (Box::into_raw(boxed).cast::<u8>(), len)
        }
        Err(_) => (std::ptr::null_mut(), 0),
    };
    // SAFETY: the caller guarantees `out_len` is valid for writes.
    unsafe { out_len.write(len) };
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
