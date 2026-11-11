//! The sandbox of Lamina PDF's desktop engine process (ADR 0008 §5). The engine parses and renders
//! untrusted PDFs; once it has loaded PDFium it needs nothing but its pipes, its threads and
//! memory, so the sandbox takes away the file system, the network, new processes and other
//! processes.
//!
//! Two halves, one per process:
//!
//! - **The main process** starts the engine with [`spawn_sandboxed`], or through
//!   [`SandboxSpawner`], the supervisor's way of starting engines. On Windows that is where most
//!   of the sandbox comes from: a job object, Low integrity, a locked-down token whose first
//!   thread impersonates a less restricted one until PDFium is loaded, and mitigation policies
//!   that forbid child processes. On macOS and Linux the engine is started plainly.
//! - **The engine** calls `apply_engine_sandbox` (feature `engine`) once PDFium is loaded and
//!   before it reads any document: Seatbelt on macOS; no-new-privs, Landlock, `RLIMIT_AS` and a
//!   seccomp-bpf allow-list on Linux; on Windows it drops the impersonation token
//!   (`RevertToSelf`) and adds the mitigation policies that only work after loading.
//!
//! Each OS's [`SandboxLevel`] says which layers are on; the engine sends it in `Ready` and the
//! main process logs it. `LAMINA_PDF_ENGINE_SANDBOX=off` turns the sandbox off for diagnosis, in
//! debug builds only ([`sandbox_disabled`]).

mod spawn;
#[cfg(windows)]
mod win32;

#[cfg(feature = "engine")]
mod lockdown;
#[cfg(feature = "engine")]
pub mod probe;

pub use laminapdf_desktop_protocol::SandboxLevel;
#[cfg(feature = "engine")]
pub use lockdown::apply_engine_sandbox;
pub use spawn::{SandboxSpawner, SandboxedChild, SandboxedProcess, spawn_sandboxed};

/// The environment variable that turns the sandbox off in a debug build.
pub const SANDBOX_ENV: &str = "LAMINA_PDF_ENGINE_SANDBOX";

/// The engine's argument that carries [`SandboxConfig::memory_limit`].
pub const MEMORY_LIMIT_ARG: &str = "--memory-limit";

/// What the sandbox allows the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxConfig {
    /// Bytes of memory: the job's committed-memory limit on Windows; on Linux `RLIMIT_AS` is
    /// twice it, since address space counts reservations (thread stacks, malloc arenas) too.
    /// macOS has no enforced limit (ADR 0008 §5).
    pub memory_limit: u64,
}

impl SandboxConfig {
    /// 2 GiB: over ten times what the engine peaks at opening the 1 GB fixture (ADR 0008 §2),
    /// far below a laptop's memory.
    pub const DEFAULT_MEMORY_LIMIT: u64 = 2 << 30;
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit: Self::DEFAULT_MEMORY_LIMIT,
        }
    }
}

/// Why the sandbox couldn't be set up. The engine refuses to serve without it, and the main
/// process reports the engine as failing to start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SandboxError {
    /// An OS call failed: which step, and the OS's error number.
    #[error("{step} failed (OS error {code})")]
    Os {
        /// What was being set up.
        step: &'static str,
        /// `errno` or `GetLastError()`.
        code: i32,
    },
    /// The engine had more than one thread when it locked itself down, so some of them would
    /// escape a per-thread restriction (Landlock).
    #[error("the engine had {0} threads when the sandbox was applied")]
    Threads(usize),
    /// The seccomp filter couldn't be built or installed.
    #[error("seccomp: {0}")]
    Seccomp(String),
    /// The Landlock ruleset couldn't be built or applied.
    #[error("landlock: {0}")]
    Landlock(String),
    /// The engine couldn't be started.
    #[error("starting the engine: {0}")]
    Spawn(#[from] std::io::Error),
}

impl SandboxError {
    /// The last OS error, for `step`.
    #[cfg(any(windows, all(target_os = "linux", feature = "engine")))]
    pub(crate) fn last_os(step: &'static str) -> Self {
        Self::Os {
            step,
            code: std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
        }
    }
}

/// Whether the sandbox is off for diagnosis: `LAMINA_PDF_ENGINE_SANDBOX=off` in a debug build.
/// A release build ignores the variable, so a user's environment can't turn the sandbox off.
#[must_use]
pub fn sandbox_disabled() -> bool {
    cfg!(debug_assertions) && std::env::var_os(SANDBOX_ENV).is_some_and(|value| value == "off")
}
