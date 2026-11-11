//! The engine's half: the lock-down it applies to itself once PDFium is loaded and before it reads
//! a document. Each OS's order is in ADR 0008 §5.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use laminapdf_desktop_protocol::SandboxLevel;

use crate::{SandboxConfig, SandboxError, sandbox_disabled};

/// Locks the engine down: call it once, from the only thread, after loading PDFium and before
/// reading any document. Returns the layers that are on, for `Ready`.
///
/// - **Linux:** no-new-privs, Landlock (as much as the kernel offers: none before 5.13),
///   `RLIMIT_AS`, then the seccomp-bpf allow-list.
/// - **macOS:** a deny-by-default Seatbelt profile.
/// - **Windows:** `RevertToSelf` drops the start-up impersonation token, leaving the locked-down
///   primary token; then the mitigation policies that can only follow loading PDFium. The job,
///   integrity level and child-process policy came with the process ([`crate::spawn_sandboxed`]);
///   this checks and reports them.
///
/// With the sandbox off for diagnosis ([`sandbox_disabled`]) it does nothing and says so.
///
/// # Errors
/// [`SandboxError`] when a layer the engine can't do without fails to apply: the engine then
/// refuses to serve.
pub fn apply_engine_sandbox(config: &SandboxConfig) -> Result<SandboxLevel, SandboxError> {
    if sandbox_disabled() {
        return Ok(SandboxLevel {
            off: true,
            ..SandboxLevel::default()
        });
    }
    #[cfg(target_os = "linux")]
    return linux::apply(*config);
    #[cfg(target_os = "macos")]
    return macos::apply(*config);
    #[cfg(windows)]
    return windows::apply(*config);
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = config;
        Ok(SandboxLevel::default())
    }
}

/// One line on the engine's stderr, which the main process logs.
#[cfg_attr(
    not(any(target_os = "linux", target_os = "macos", windows)),
    expect(dead_code, reason = "only the sandboxed OSes log from here")
)]
fn log(message: &str) {
    use std::io::Write as _;
    // Nothing to do if stderr is gone.
    let _ = writeln!(std::io::stderr().lock(), "sandbox: {message}");
}
