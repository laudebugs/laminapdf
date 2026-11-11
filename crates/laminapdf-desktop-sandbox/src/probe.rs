//! The selftest's probes (`laminapdf-desktop-engine --sandbox-selftest`): each tries one thing the
//! sandbox must stop, or must still allow, and says what happened. On Linux a forbidden system
//! call kills the process (SIGSYS) instead of failing, so those probes never return there: the
//! caller sees the signal.
#![cfg_attr(unix, expect(unsafe_code, reason = "FFI: fork, execve, clone3 and getrlimit"))]

use std::fmt;
use std::io::{self, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// What a probe saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The sandbox stopped it; why, in the OS's words.
    Blocked(String),
    /// It worked.
    Allowed(String),
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blocked(why) => write!(f, "blocked ({why})"),
            Self::Allowed(what) => write!(f, "allowed ({what})"),
        }
    }
}

/// Reads `path`.
#[must_use]
pub fn read(path: &Path) -> Outcome {
    match std::fs::read(path) {
        Ok(bytes) => Outcome::Allowed(format!("read {} bytes", bytes.len())),
        Err(error) => Outcome::Blocked(error.to_string()),
    }
}

/// Creates `path` and writes a byte to it (removing it again if that worked).
#[must_use]
pub fn write(path: &Path) -> Outcome {
    let written = std::fs::File::create(path).and_then(|mut file| file.write_all(b"x"));
    match written {
        Ok(()) => {
            let _ = std::fs::remove_file(path);
            Outcome::Allowed("created a file".into())
        }
        Err(error) => Outcome::Blocked(error.to_string()),
    }
}

/// Opens a TCP connection to `addr`. A refused or unanswered connection still means a socket was
/// created and the network reached, so only a failure before that counts as blocked.
#[must_use]
pub fn connect(addr: SocketAddr) -> Outcome {
    // On Windows the locked-down token can't start Winsock, and std panics on that failure.
    let connected = std::panic::catch_unwind(|| TcpStream::connect_timeout(&addr, Duration::from_secs(2)));
    let Ok(connected) = connected else {
        return Outcome::Blocked("the socket layer failed to start".into());
    };
    match connected {
        Ok(_) => Outcome::Allowed(format!("connected to {addr}")),
        Err(error) if matches!(error.kind(), io::ErrorKind::ConnectionRefused | io::ErrorKind::TimedOut) => {
            Outcome::Allowed(format!("a socket reached {addr}, which didn't answer: {error}"))
        }
        Err(error) => Outcome::Blocked(error.to_string()),
    }
}

/// Starts `program` with `args` and waits for it.
#[must_use]
pub fn spawn(program: &Path, args: &[&str]) -> Outcome {
    match Command::new(program).args(args).status() {
        Ok(status) => Outcome::Allowed(format!("ran {}: {status}", program.display())),
        Err(error) => Outcome::Blocked(error.to_string()),
    }
}

/// Allocates and touches up to `bytes` of memory in 64 MiB steps.
#[must_use]
pub fn allocate(bytes: u64) -> Outcome {
    const STEP: usize = 64 << 20;
    let mut held: Vec<Vec<u8>> = Vec::new();
    let mut total = 0u64;
    while total < bytes {
        let mut chunk = Vec::new();
        if let Err(error) = chunk.try_reserve_exact(STEP) {
            return Outcome::Blocked(format!("{error} after {} MiB", total >> 20));
        }
        chunk.resize(STEP, 1);
        held.push(chunk);
        total += STEP as u64;
    }
    Outcome::Allowed(format!("held {} MiB", total >> 20))
}

/// `fork()`: the child exits at once.
#[cfg(unix)]
#[must_use]
pub fn fork() -> Outcome {
    // SAFETY: fork has no preconditions; the child calls only the async-signal-safe `_exit`.
    match unsafe { libc::fork() } {
        -1 => Outcome::Blocked(io::Error::last_os_error().to_string()),
        0 => {
            // SAFETY: `_exit` ends the child without running the parent's destructors.
            unsafe { libc::_exit(0) }
        }
        pid => {
            let mut status = 0;
            // SAFETY: `pid` is this process's child; `status` is a writable int.
            unsafe { libc::waitpid(pid, &raw mut status, 0) };
            Outcome::Allowed(format!("forked process {pid}"))
        }
    }
}

/// `execve(program)` in this process: if it works, this process becomes `program`, so the caller
/// sees `program`'s exit instead of an outcome.
#[cfg(unix)]
#[must_use]
pub fn exec(program: &Path) -> Outcome {
    use std::os::unix::ffi::OsStrExt as _;
    let Ok(path) = std::ffi::CString::new(program.as_os_str().as_bytes()) else {
        return Outcome::Blocked("the path holds a NUL".into());
    };
    let argv = [path.as_ptr(), std::ptr::null()];
    let envp = [std::ptr::null()];
    // SAFETY: `path` is NUL-terminated; `argv` and `envp` are null-terminated arrays of valid
    // pointers that outlive the call. On success it doesn't return.
    unsafe { libc::execve(path.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
    Outcome::Blocked(io::Error::last_os_error().to_string())
}

/// A raw `clone3` with no arguments: the sandbox answers `ENOSYS`; without it the kernel answers
/// `EINVAL` (or `EFAULT`).
#[cfg(target_os = "linux")]
#[must_use]
pub fn clone3() -> Outcome {
    // SAFETY: clone3 with a null argument structure and size 0 fails before creating anything.
    let result = unsafe { libc::syscall(libc::SYS_clone3, std::ptr::null::<u8>(), 0usize) };
    let error = io::Error::last_os_error();
    match (result, error.raw_os_error()) {
        (-1, Some(libc::ENOSYS)) => Outcome::Blocked("ENOSYS".into()),
        _ => Outcome::Allowed(format!("returned {result}: {error}")),
    }
}

/// `RLIMIT_NPROC`'s soft and hard values, which the sandbox must leave alone.
#[cfg(target_os = "linux")]
#[must_use]
pub fn nproc_limit() -> Option<(u64, u64)> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a writable rlimit.
    let ok = unsafe { libc::getrlimit(libc::RLIMIT_NPROC, &raw mut limit) } == 0;
    ok.then_some((limit.rlim_cur, limit.rlim_max))
}
