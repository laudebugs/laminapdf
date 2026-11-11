//! Unix: the engine starts with only its three pipes. The main process holds descriptors that
//! aren't close-on-exec (GTK, the webview and D-Bus open some), and an inherited socket or file would
//! get around the sandbox, so the child closes every descriptor above 2 before `exec`.
#![expect(
    unsafe_code,
    reason = "FFI: pre_exec and the close calls it makes in the forked child"
)]

use std::os::unix::process::CommandExt as _;
use std::process::Command;

/// Makes `command` close every descriptor above 2 in the child, between `fork` and `exec`.
pub(super) fn close_inherited(command: &mut Command) {
    // SAFETY: sysconf is called here, in the parent, and has no preconditions.
    let open_max = unsafe { libc::sysconf(libc::_SC_OPEN_MAX) };
    // A huge or unlimited table would make the fallback loop slow; 64 Ki covers every real one.
    let last = libc::c_int::try_from(open_max).unwrap_or(65_536).clamp(256, 65_536);
    // SAFETY: the closure runs in the forked child before exec and calls only async-signal-safe
    // functions (the close_range system call and close); it allocates nothing and touches no
    // lock. Closing descriptors the child doesn't need can't break the exec: std's own error pipe
    // is close-on-exec and above 2, so closing it early only loses an exec error report.
    unsafe {
        command.pre_exec(move || {
            #[cfg(target_os = "linux")]
            if libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) == 0 {
                return Ok(());
            }
            for fd in 3..last {
                libc::close(fd);
            }
            Ok(())
        });
    }
}
