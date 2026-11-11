//! Unix: the engine starts with only its three pipes. The main process holds descriptors that
//! aren't close-on-exec (GTK, the webview and D-Bus open some), and an inherited socket or file would
//! get around the sandbox, so the child closes every descriptor above 2 before `exec`, however
//! high it is:
//!
//! - **Linux:** `close_range(3, ~0)` (5.9 and later); before that, the child lists `/proc/self/fd`
//!   with `getdents64` into a buffer on its stack and closes each one; without `/proc`, it closes
//!   every number up to the hard `RLIMIT_NOFILE`.
//! - **macOS:** std gives no way to pass `POSIX_SPAWN_CLOEXEC_DEFAULT` to its `posix_spawn`, so the
//!   child asks the kernel for its open descriptors (`proc_pidinfo(PROC_PIDLISTFDS)`) into a
//!   buffer the parent sized, and closes each one; if they don't fit, it closes every number up to
//!   the hard `RLIMIT_NOFILE`.
//!
//! Nothing allocates between `fork` and `exec`.
#![expect(
    unsafe_code,
    reason = "FFI: pre_exec and the system calls it makes in the forked child"
)]

use std::os::unix::process::CommandExt as _;
use std::process::Command;

/// The highest descriptor number the fallback loop closes: the hard `RLIMIT_NOFILE`, clamped to
/// 2^20, Linux's default ceiling on any descriptor (`fs.nr_open`; macOS's `kern.maxfilesperproc`
/// is lower), so an unlimited hard limit still ends.
fn highest_descriptor() -> libc::c_int {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a writable rlimit.
    let ok = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &raw mut limit) } == 0;
    let hard = if ok { limit.rlim_max } else { 1 << 20 };
    libc::c_int::try_from(hard)
        .unwrap_or(libc::c_int::MAX)
        .clamp(1024, 1 << 20)
}

/// Makes `command` close every descriptor above 2 in the child, between `fork` and `exec`.
pub(super) fn close_inherited(command: &mut Command) {
    let last = highest_descriptor();
    #[cfg(target_os = "macos")]
    let mut listed = macos::Listed::sized();
    // SAFETY: the closure runs in the forked child before exec and calls only async-signal-safe
    // system calls (close_range, open, getdents64, proc_pidinfo, close) on memory allocated before
    // the fork; it touches no lock. Closing descriptors the child doesn't need can't break the
    // exec: std's own error pipe is close-on-exec, so closing it early only loses an exec error
    // report.
    unsafe {
        command.pre_exec(move || {
            #[cfg(target_os = "linux")]
            let closed = linux::close_all();
            #[cfg(target_os = "macos")]
            let closed = listed.close_all();
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            let closed = false;
            if !closed {
                for fd in 3..=last {
                    libc::close(fd);
                }
            }
            Ok(())
        });
    }
}

#[cfg(target_os = "linux")]
mod linux {
    /// A `linux_dirent64`'s record length (after the 8-byte inode and 8-byte offset) and its name
    /// (after the 1-byte type), as getdents64(2) lays them out.
    const RECLEN_AT: usize = 16;
    const NAME_AT: usize = 19;

    /// Closes every descriptor above 2: `close_range`, else a `/proc/self/fd` listing. `false` when
    /// neither is available.
    ///
    /// # Safety
    /// Only between `fork` and `exec` in the child: it closes descriptors others may hold.
    pub(super) unsafe fn close_all() -> bool {
        // SAFETY: close_range takes three integers and reads no memory.
        if unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) } == 0 {
            return true;
        }
        // SAFETY: a NUL-terminated path; the descriptor is closed below.
        let dir = unsafe { libc::open(c"/proc/self/fd".as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY) };
        if dir < 0 {
            return false;
        }
        let mut buf = [0u8; 4096];
        loop {
            // SAFETY: getdents64 writes at most `buf.len()` bytes into `buf`, which outlives it.
            let read = unsafe { libc::syscall(libc::SYS_getdents64, dir, buf.as_mut_ptr(), buf.len()) };
            let Ok(read) = usize::try_from(read) else { break };
            if read == 0 {
                break;
            }
            let mut at = 0;
            while at + NAME_AT <= read {
                let Some(record) = buf.get(at..read) else { break };
                let Some(&[low, high]) = record.get(RECLEN_AT..RECLEN_AT + 2) else {
                    break;
                };
                let reclen = usize::from(u16::from_ne_bytes([low, high]));
                let name = record.get(NAME_AT..reclen.min(record.len())).unwrap_or(&[]);
                let mut fd: libc::c_int = 0;
                let mut digits = 0;
                for &byte in name.iter().take_while(|b| b.is_ascii_digit()) {
                    fd = fd.saturating_mul(10).saturating_add(libc::c_int::from(byte - b'0'));
                    digits += 1;
                }
                if digits > 0 && fd > 2 && fd != dir {
                    // SAFETY: closing a number that isn't open does nothing.
                    unsafe { libc::close(fd) };
                }
                if reclen == 0 {
                    break;
                }
                at += reclen;
            }
        }
        // SAFETY: `dir` is the descriptor opened above, closed once.
        unsafe { libc::close(dir) };
        true
    }
}

#[cfg(target_os = "macos")]
mod macos {
    /// Room for the parent's open descriptors and more opened before the fork.
    const SPARE: usize = 1024;

    /// A buffer for the child's descriptor list, allocated in the parent.
    pub(super) struct Listed(Vec<libc::proc_fdinfo>);

    impl Listed {
        /// Sized for the parent's descriptors now, plus [`SPARE`].
        pub(super) fn sized() -> Self {
            // SAFETY: a null buffer of size 0 asks only for the size needed.
            let bytes =
                unsafe { libc::proc_pidinfo(libc::getpid(), libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
            let now = usize::try_from(bytes).unwrap_or(0) / size_of::<libc::proc_fdinfo>();
            Self(vec![
                libc::proc_fdinfo {
                    proc_fd: -1,
                    proc_fdtype: 0
                };
                now + SPARE
            ])
        }

        /// Closes every listed descriptor above 2; `false` when the list may not have fitted.
        ///
        /// # Safety
        /// Only between `fork` and `exec` in the child: it closes descriptors others may hold.
        pub(super) unsafe fn close_all(&mut self) -> bool {
            let room = self.0.len() * size_of::<libc::proc_fdinfo>();
            let Ok(size) = libc::c_int::try_from(room) else {
                return false;
            };
            // SAFETY: proc_pidinfo writes at most `size` bytes into the buffer allocated before the
            // fork, which outlives the call.
            let bytes = unsafe {
                libc::proc_pidinfo(
                    libc::getpid(),
                    libc::PROC_PIDLISTFDS,
                    0,
                    self.0.as_mut_ptr().cast(),
                    size,
                )
            };
            let Ok(bytes) = usize::try_from(bytes) else {
                return false;
            };
            if bytes == 0 || bytes >= room {
                return false;
            }
            let count = bytes / size_of::<libc::proc_fdinfo>();
            for info in self.0.iter().take(count) {
                if info.proc_fd > 2 {
                    // SAFETY: closing a listed descriptor of this process.
                    unsafe { libc::close(info.proc_fd) };
                }
            }
            true
        }
    }
}
