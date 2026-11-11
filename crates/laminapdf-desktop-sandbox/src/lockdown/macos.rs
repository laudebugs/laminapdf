//! macOS: a Seatbelt profile, applied at run time once PDFium is loaded (Chromium's renderer
//! sandbox works the same way, through the same kernel policy engine).
//!
//! App Sandbox, the entitlement-based alternative, was tried first (ADR 0008 §5). It works
//! for an ad-hoc signed helper inside the bundle, but it still lets the engine read system files
//! such as `/etc/hosts` and start processes (which inherit it), and a process under it can't
//! apply a stricter Seatbelt profile on top. This profile denies all of that.
#![expect(unsafe_code, reason = "FFI: sandbox_init from libsystem_sandbox")]

use std::ffi::{CStr, CString, c_char, c_int};

use laminapdf_desktop_protocol::SandboxLevel;

use crate::{SandboxConfig, SandboxError};

/// Deny everything; the engine's pipes are already open, and reading or writing an open
/// descriptor is not checked. Nothing else is needed after loading: no files (PDFium falls back
/// to its built-in fonts, ADR 0008 §5), no Mach services, no sysctl, no network, no
/// `process-fork` or `process-exec`.
const PROFILE: &str =
    "(version 1)\n(deny default)\n(allow signal (target self))\n(allow process-info* (target self))\n";

// libsystem_sandbox, part of libSystem: `sandbox_init` is public API, deprecated since macOS 10.8
// without a public replacement; Chromium uses the same library through its private successors.
unsafe extern "C" {
    fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *mut c_char) -> c_int;
    fn sandbox_free_error(errorbuf: *mut c_char);
}

pub(super) fn apply(_config: SandboxConfig) -> Result<SandboxLevel, SandboxError> {
    let profile = CString::new(PROFILE).map_err(|_| SandboxError::Os {
        step: "the Seatbelt profile",
        code: 0,
    })?;
    let mut error: *mut c_char = std::ptr::null_mut();
    // SAFETY: `profile` is a NUL-terminated string that outlives the call; flags 0 means it is a
    // profile's text, not a named profile; `error` is a valid out-pointer, set only on failure.
    let status = unsafe { sandbox_init(profile.as_ptr(), 0, &raw mut error) };
    if status != 0 {
        let message = if error.is_null() {
            String::new()
        } else {
            // SAFETY: on failure `sandbox_init` sets `error` to a NUL-terminated string it
            // allocated, which stays valid until `sandbox_free_error` below.
            let text = unsafe { CStr::from_ptr(error) }.to_string_lossy().into_owned();
            // SAFETY: `error` came from `sandbox_init` and is freed once.
            unsafe { sandbox_free_error(error) };
            text
        };
        super::log(&format!("sandbox_init: {message}"));
        return Err(SandboxError::Os {
            step: "sandbox_init",
            code: status,
        });
    }
    Ok(SandboxLevel {
        seatbelt: true,
        ..SandboxLevel::default()
    })
}
