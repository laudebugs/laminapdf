//! The monitor path end to end (ADR 0014 §10), on Unix: a process connects to the monitor's main
//! server and faults; the monitor, out of process, writes its minidump into the main folder, which
//! the next launch would read into a held report. Here we drive the monitor and a faulting
//! subprocess (this test binary re-run with a role), then summarise the dump it leaves.
//!
//! The subprocess pattern: the test binary re-runs itself with `--exact subprocess_role` and an
//! environment that tells it to be the monitor or the faulter. Run without that environment the
//! role test is a no-op, so it is harmless under a normal test run.
#![cfg(all(unix, feature = "summary", feature = "debug-crash"))]
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use laminapdf_desktop_crash::summary::summarize_bytes;
use laminapdf_desktop_crash::{MONITOR_ARG, MonitorPaths, capture_crashes, run_monitor};

const ROLE: &str = "LAMINA_CRASH_ROLE";
const MAIN_SOCK: &str = "LAMINA_CRASH_MAIN_SOCK";
const MAIN_DIR: &str = "LAMINA_CRASH_MAIN_DIR";
const ENG_SOCK: &str = "LAMINA_CRASH_ENG_SOCK";
const ENG_DIR: &str = "LAMINA_CRASH_ENG_DIR";
const FAULT_SOCK: &str = "LAMINA_CRASH_FAULT_SOCK";
const FAULT_PID: &str = "LAMINA_CRASH_FAULT_PID";

/// Re-run with `LAMINA_CRASH_ROLE`, this test is the monitor or the faulter; without it, a no-op.
#[test]
fn subprocess_role() {
    match std::env::var(ROLE).ok().as_deref() {
        Some("monitor") => {
            let args: Vec<OsString> = [
                MONITOR_ARG.into(),
                env(MAIN_SOCK),
                env(MAIN_DIR),
                env(ENG_SOCK),
                env(ENG_DIR),
            ]
            .into_iter()
            .collect();
            let _ = run_monitor(args.into_iter());
        }
        Some("faulter") => {
            let socket = env(FAULT_SOCK);
            let server: u32 = std::env::var(FAULT_PID).unwrap().parse().unwrap();
            // Kept alive until the fault; dropping it would detach the handler.
            let _capture = capture_crashes(std::path::Path::new(&socket), server).unwrap();
            laminapdf_desktop_crash::fault();
        }
        _ => {}
    }
}

fn env(key: &str) -> OsString {
    std::env::var_os(key).unwrap()
}

fn this_binary() -> PathBuf {
    std::env::current_exe().unwrap()
}

#[test]
fn a_main_process_fault_under_the_monitor_should_leave_a_summarisable_dump() {
    let dir = std::env::temp_dir().join(format!("laminapdf-monitor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let paths = MonitorPaths::new(&dir);

    // The monitor, out of process.
    let monitor = Command::new(this_binary())
        .args(["--exact", "subprocess_role", "--nocapture"])
        .env(ROLE, "monitor")
        .env(MAIN_SOCK, &paths.main_socket)
        .env(MAIN_DIR, &paths.main_dir)
        .env(ENG_SOCK, &paths.engine_socket)
        .env(ENG_DIR, &paths.engine_dir)
        .spawn()
        .unwrap();
    wait_for(&paths.main_socket);

    // The faulter connects to the main server and faults; the monitor writes its dump.
    let faulter = Command::new(this_binary())
        .args(["--exact", "subprocess_role", "--nocapture"])
        .env(ROLE, "faulter")
        .env(FAULT_SOCK, &paths.main_socket)
        .env(FAULT_PID, monitor.id().to_string())
        .spawn()
        .unwrap();
    let faulter_pid = faulter.id();

    let dump = wait_for_dump(&paths.main_dir);
    let summary = summarize_bytes(dump).unwrap();
    // End both subprocesses rather than wait on them: the monitor stays up until its main client
    // disconnects, and on macOS that is through a mach port, so don't depend on the timing here.
    let mut faulter = faulter;
    let _ = faulter.kill();
    let _ = faulter.wait();
    let mut monitor = monitor;
    let _ = monitor.kill();
    let _ = monitor.wait();

    // The dump is the faulter's, with a real fault reason: the next launch would read it into a
    // held report, as the marked crash tests in the app check for the queueing side.
    assert_eq!(
        (summary.pid, summary.crash_reason.is_empty(), summary.frames.is_empty()),
        (Some(faulter_pid), false, false)
    );
}

fn wait_for(path: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < deadline, "the monitor never bound {}", path.display());
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_dump(dir: &std::path::Path) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(path) = std::fs::read_dir(dir).ok().and_then(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|e| e == "dmp"))
        }) {
            std::thread::sleep(Duration::from_millis(200));
            return std::fs::read(path).unwrap();
        }
        assert!(Instant::now() < deadline, "no minidump landed in the main folder");
        std::thread::sleep(Duration::from_millis(50));
    }
}
