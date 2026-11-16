//! Native crash capture for Lamina PDF's desktop reader (ADR 0014 §10): a process that faults asks
//! another process, which is not sandboxed, to write its minidump, and the dump is later read into
//! an allow-listed summary and deleted. The raw dump holds the crashing process's memory, so
//! possibly document bytes and paths: it never leaves the computer.
//!
//! Three parts, each in its own place so that nothing hostile is handled in the main process:
//!
//! - **The monitor** ([`run_monitor`]) is the app's own binary started with a flag, outside the
//!   sandbox. It hosts the minidump servers: one for the main process, one for its engines. A
//!   hostile message can crash only the monitor; the main process restarts it.
//! - **A crashing process** ([`capture_crashes`]) installs a `crash_handler` that asks its server
//!   for a dump. On Unix the engine does this before it locks itself down, so the connection
//!   exists before the sandbox would refuse it.
//! - **Reading a dump** ([`summary`], feature `summary`) turns the bytes into a [`summary::DumpSummary`].
//!   The engine binary does it in a `--summarize-dump` mode under the full sandbox; the main process
//!   only reads the summary's JSON, so it never links the minidump parser.
//!
//! On Windows the engine takes no part: it opens no socket (Winsock would give it a way onto the
//! network), and the monitor watches it as its debugger instead, writing its dump itself
//! (`capture::debugger`). The main process's own server checks that a dump request comes from the
//! process it names (the connection's peer, from the kernel): the workspace carries `minidumper`
//! with that check (`third_party/minidumper`, ADR 0014 §10).

pub mod summary;

mod capture;

pub use capture::{
    CaptureError, CrashCapture, DUMP_EXTENSION, MONITOR_ARG, MonitorPaths, capture_crashes, capture_crashes_within,
    run_monitor, skip_dumps,
};

#[cfg(feature = "test-support")]
pub use capture::test_support;

/// The engine's argument that names the monitor's engine-dump socket.
pub const CRASH_SOCKET_ARG: &str = "--crash-socket";

/// The engine's argument that names the monitor's process id, for `PR_SET_PTRACER` (Linux).
pub const CRASH_SERVER_PID_ARG: &str = "--crash-server-pid";

/// The argument that makes the engine a one-shot minidump reader (`main.rs`'s `summarize_dump`).
pub const SUMMARIZE_DUMP_ARG: &str = "--summarize-dump";

/// Faults on purpose: a write through a null pointer, for the end-to-end tests' real native crash
/// (`ToEngine::DebugSegv`). Feature `debug-crash`, which only the e2e engine turns on.
#[cfg(feature = "debug-crash")]
#[expect(unsafe_code, reason = "the deliberate fault the end-to-end tests ask for")]
pub fn fault() {
    let null = std::ptr::null_mut::<u64>();
    // SAFETY: none: this is the deliberate fault the end-to-end tests ask for. The write through a
    // null pointer faults (SIGSEGV, EXC_BAD_ACCESS, an access violation) and the process ends in
    // its crash handler; `write_volatile` keeps the compiler from removing it.
    unsafe { null.write_volatile(0x1a3e_57ed) };
}
