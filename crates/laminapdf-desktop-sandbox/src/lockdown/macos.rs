//! macOS: a Seatbelt profile, applied at run time once PDFium is loaded (Chromium's renderer
//! sandbox works the same way, through the same kernel policy engine).
//!
//! App Sandbox, the entitlement-based alternative, was tried first (ADR 0008 §5). It works
//! for an ad-hoc signed helper inside the bundle, but it still lets the engine read system files
//! such as `/etc/hosts` and start processes (which inherit it), and a process under it can't
//! apply a stricter Seatbelt profile on top. This profile denies all of that.
#![expect(
    unsafe_code,
    reason = "FFI: sandbox_init from libsystem_sandbox; fcntl, dup2 and setrlimit"
)]

use std::ffi::{CStr, c_char, c_int};

use laminapdf_desktop_protocol::{Layer, SandboxLevel};

use crate::{SandboxConfig, SandboxError};

/// Deny everything; the engine's pipes are already open, and reading or writing an open
/// descriptor is not checked. Nothing else is needed after loading: no files (fonts come from the
/// main process, ADR 0008 §5), no Mach services, no sysctl, no network, no `process-fork` or
/// `process-exec`.
const PROFILE: &CStr =
    c"(version 1)\n(deny default)\n(allow signal (target self))\n(allow process-info* (target self))\n";

// libsystem_sandbox, part of libSystem: `sandbox_init` is public API, deprecated since macOS 10.8
// without a public replacement; Chromium uses the same library through its private successors.
unsafe extern "C" {
    fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *mut c_char) -> c_int;
    fn sandbox_free_error(errorbuf: *mut c_char);
}

pub(super) fn apply(config: SandboxConfig) -> Result<SandboxLevel, SandboxError> {
    // In the Mac App Store build the engine inherits the app's App Sandbox, which refuses a
    // Seatbelt profile on top; macOS names the container of every process it sandboxes. That
    // sandbox lets the app connect out, so the engine also stops opening descriptors. Only that
    // build's main process accepts this answer (ADR 0014 §11).
    if laminapdf_desktop_protocol::in_app_sandbox() {
        watch_own_memory(config.memory_limit)?;
        no_new_descriptors()?;
        return Ok(SandboxLevel {
            layers: Layer::STORE_BUILD.to_vec(),
        });
    }
    let mut error: *mut c_char = std::ptr::null_mut();
    // SAFETY: `PROFILE` is a static NUL-terminated string; flags 0 means it is a profile's text,
    // not a named profile; `error` is a valid out-pointer, set only on failure.
    let status = unsafe { sandbox_init(PROFILE.as_ptr(), 0, &raw mut error) };
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
        return Err(SandboxError::Seatbelt { status, message });
    }
    Ok(SandboxLevel {
        layers: vec![Layer::Seatbelt],
    })
}

/// Caps `RLIMIT_NOFILE`, soft and hard, at one past the highest descriptor the engine holds, after
/// filling every free number below it with a copy of that one, so no call can make a descriptor:
/// no socket, file or pipe. The engine needs none after it locks down (its pipes and, with crash
/// reports, its crash socket are open; fonts come through the pipes). A lowered hard limit can't be
/// raised again without privileges.
fn no_new_descriptors() -> Result<(), SandboxError> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a writable rlimit.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &raw mut limit) } != 0 {
        return Err(SandboxError::last_os("getrlimit(RLIMIT_NOFILE)"));
    }
    let scan = c_int::try_from(limit.rlim_cur.min(1 << 16)).unwrap_or(1 << 16);
    // SAFETY: F_GETFD on any number only asks whether it is an open descriptor.
    let open = |fd: c_int| unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1;
    let highest = (0..scan).rev().find(|&fd| open(fd)).unwrap_or(0);
    for hole in (0..highest).filter(|&fd| !open(fd)) {
        // SAFETY: dup2 of the open descriptor `highest` onto the free number `hole`; the copy is
        // never used, it only takes the number.
        if unsafe { libc::dup2(highest, hole) } < 0 {
            return Err(SandboxError::last_os("dup2"));
        }
    }
    let cap = libc::rlim_t::try_from(highest).unwrap_or(0) + 1;
    let capped = libc::rlimit {
        rlim_cur: cap,
        rlim_max: cap,
    };
    // SAFETY: `capped` is a valid rlimit that outlives the call.
    if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &raw const capped) } != 0 {
        return Err(SandboxError::last_os("setrlimit(RLIMIT_NOFILE)"));
    }
    Ok(())
}

/// How often the engine reads its own footprint in App Sandbox: more often than the main
/// process's watchdog elsewhere, since nothing else watches.
const OWN_MEMORY_PERIOD: std::time::Duration = std::time::Duration::from_millis(250);

/// In App Sandbox the main process can't read the engine's footprint, so the engine reads its own
/// and aborts past `limit` bytes; the main process restarts it like any crash, and the line this
/// writes on stderr reaches its log. A footprint it can't read or a thread that can't start fails
/// the lock-down, so the engine never reports a memory limit it doesn't have.
fn watch_own_memory(limit: u64) -> Result<(), SandboxError> {
    let pid = std::process::id();
    crate::footprint::footprint(pid).map_err(|source| SandboxError::Os {
        step: "reading the engine's own memory",
        source,
    })?;
    std::thread::Builder::new()
        .name("engine-own-memory".into())
        .spawn(move || {
            loop {
                std::thread::sleep(OWN_MEMORY_PERIOD);
                match crate::footprint::footprint(pid) {
                    Ok(footprint) if footprint > limit => {
                        super::log(&format!(
                            "memory limit: the engine stopped itself at {} MiB (limit {} MiB)",
                            footprint >> 20,
                            limit >> 20
                        ));
                        std::process::abort();
                    }
                    Ok(_) => {}
                    Err(error) => {
                        super::log(&format!(
                            "the engine can't read its own memory, so it has no memory limit: {error}"
                        ));
                        return;
                    }
                }
            }
        })
        .map(drop)
        .map_err(|source| SandboxError::Os {
            step: "the memory watch thread",
            source,
        })
}
