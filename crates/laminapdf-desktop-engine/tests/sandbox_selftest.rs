//! The engine's sandbox, from the outside (ADR 0008 §5): the built engine started the way the app
//! starts it (`spawn_sandboxed`), running `--sandbox-selftest=<probe>`, one probe per run. What
//! must be blocked is blocked on every OS; on Linux a forbidden system call kills the engine
//! (SIGSYS) rather than failing. Threads and rendering keep working. With PDFium in
//! `LAMINA_PDF_PDFIUM_DIR` the thread probe renders too.
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

use std::io::Read as _;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use laminapdf_desktop_protocol::supervisor::{Config, Event, Process as _, RangeServer, Supervisor};
use laminapdf_desktop_protocol::{DocId, SandboxLevel};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner, spawn_sandboxed};

fn engine() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_laminapdf-desktop-engine"))
}

/// What one selftest run printed on stdout and how it ended.
struct Run {
    stdout: String,
    exit: String,
}

impl Run {
    /// The outcome printed for `probe`: `blocked (…)` or `allowed (…)`, or `None` if it died first.
    fn outcome(&self, probe: &str) -> Option<&str> {
        let prefix = format!("selftest {probe}: ");
        self.stdout.lines().find_map(|line| line.strip_prefix(prefix.as_str()))
    }

    fn killed_by_seccomp(&self) -> bool {
        self.exit.contains("SIGSYS")
    }
}

fn selftest_with(probe: &str, config: SandboxConfig) -> Run {
    let mut command = Command::new(engine());
    command.arg(format!("--sandbox-selftest={probe}"));
    let mut child = spawn_sandboxed(command, &config).unwrap();
    drop(child.stdin);
    let mut stdout = String::new();
    child.stdout.read_to_string(&mut stdout).unwrap();
    let mut stderr = String::new();
    child.stderr.read_to_string(&mut stderr).unwrap();
    let exit = child.process.wait();
    eprintln!("{probe}: {stdout}{stderr}exit: {exit}");
    Run { stdout, exit }
}

fn selftest(probe: &str) -> Run {
    selftest_with(probe, SandboxConfig::default())
}

/// A forbidden probe: Linux kills the engine; elsewhere the call fails.
fn assert_blocked(probe: &str) {
    let name = probe.split(':').next().unwrap();
    let run = selftest(probe);
    if cfg!(target_os = "linux") && run.outcome(name).is_none() {
        assert!(
            run.killed_by_seccomp(),
            "{name}: neither blocked nor killed: {}",
            run.exit
        );
        return;
    }
    let outcome = run
        .outcome(name)
        .unwrap_or_else(|| panic!("{name}: no outcome ({})", run.exit));
    assert!(outcome.starts_with("blocked"), "{name}: {outcome}");
}

#[test]
fn the_selftest_should_name_this_os_sandbox_layers() {
    let run = selftest("threads");
    let layers = run.outcome("sandbox").unwrap();
    let expected: &[&str] = if cfg!(target_os = "linux") {
        &["no-new-privs", "seccomp", "memory-limit"]
    } else if cfg!(target_os = "macos") {
        &["seatbelt"]
    } else if cfg!(windows) {
        &[
            "job",
            "memory-limit",
            "low-integrity",
            "restricted-token",
            "mitigations",
        ]
    } else {
        &[]
    };
    for layer in expected {
        assert!(layers.split(' ').any(|l| l == *layer), "{layer} missing from {layers}");
    }
}

#[test]
fn reading_a_file_should_be_blocked() {
    let path = std::env::temp_dir().join(format!("laminapdf-sandbox-read-{}", std::process::id()));
    std::fs::write(&path, b"a secret").unwrap();
    assert_blocked(&format!("read:{}", path.display()));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reading_a_system_file_should_be_blocked() {
    assert_blocked("read");
}

#[test]
fn writing_a_file_should_be_blocked() {
    let path = std::env::temp_dir().join(format!("laminapdf-sandbox-write-{}", std::process::id()));
    assert_blocked(&format!("write:{}", path.display()));
    assert!(!path.exists(), "the engine created {}", path.display());
}

#[test]
fn writing_to_the_home_folder_should_be_blocked() {
    assert_blocked("write");
}

#[test]
fn a_tcp_connection_should_be_blocked() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    assert_blocked(&format!("connect:{}", listener.local_addr().unwrap()));
}

#[test]
fn starting_a_process_should_be_blocked() {
    assert_blocked("spawn");
}

#[test]
fn threads_should_start_render_and_join() {
    let run = selftest("threads");
    let outcome = run
        .outcome("threads")
        .unwrap_or_else(|| panic!("no outcome ({})", run.exit));
    assert!(outcome.starts_with("allowed (4 threads joined"), "{outcome}");
    if std::env::var_os("LAMINA_PDF_PDFIUM_DIR").is_some() {
        assert!(outcome.contains("4 tiles rendered"), "{outcome}");
    }
}

#[test]
fn memory_past_the_limit_should_be_refused_where_the_os_enforces_one() {
    let run = selftest_with(
        "memory",
        SandboxConfig {
            memory_limit: 256 << 20,
        },
    );
    let outcome = run.outcome("memory");
    if cfg!(target_os = "macos") {
        // macOS enforces no memory limit (ADR 0008 §5's gaps).
        assert!(outcome.is_some_and(|o| o.starts_with("allowed")), "{outcome:?}");
    } else {
        // Refused (try_reserve failed) or, for an allocation std can't fail, the process ended.
        assert!(
            outcome.is_none_or(|o| o.starts_with("blocked")),
            "{outcome:?} ({})",
            run.exit
        );
    }
}

#[cfg(unix)]
#[test]
fn fork_should_be_blocked() {
    assert_blocked("fork");
}

#[cfg(unix)]
#[test]
fn exec_should_be_blocked() {
    let run = selftest("exec");
    // If execve worked, the engine became /bin/sh and printed no outcome, exiting normally.
    if cfg!(target_os = "linux") {
        assert!(run.killed_by_seccomp(), "{}", run.exit);
    } else {
        assert!(
            run.outcome("exec").is_some_and(|o| o.starts_with("blocked")),
            "{}",
            run.exit
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn clone3_should_answer_enosys() {
    let run = selftest("clone3");
    assert_eq!(run.outcome("clone3"), Some("blocked (ENOSYS)"), "{}", run.exit);
}

#[cfg(target_os = "linux")]
#[test]
fn the_process_limit_should_stay_what_it_was_without_the_sandbox() {
    let sandboxed = selftest("nproc");
    // The same engine with the sandbox off (a debug build honours the switch).
    let mut command = Command::new(engine());
    command
        .arg("--sandbox-selftest=nproc")
        .env(laminapdf_desktop_sandbox::SANDBOX_ENV, "off");
    let mut child = spawn_sandboxed(command, &SandboxConfig::default()).unwrap();
    let mut plain = String::new();
    child.stdout.read_to_string(&mut plain).unwrap();
    let plain = Run {
        stdout: plain,
        exit: child.process.wait(),
    };
    assert_eq!(plain.outcome("sandbox"), Some("off"));
    let limit = sandboxed.outcome("nproc").unwrap();
    assert!(limit.starts_with("allowed"), "{limit}");
    assert_eq!(Some(limit), plain.outcome("nproc"), "RLIMIT_NPROC changed");
}

/// No document is served in these tests.
struct NoFiles;

impl RangeServer for NoFiles {
    fn read(&self, _: DocId, _: u64, _: u32) -> std::io::Result<Vec<u8>> {
        Err(std::io::Error::other("no files"))
    }
}

/// An engine that dies of SIGSYS (here sent by the test, as seccomp's kill would) is a crash: the
/// supervisor logs it as a sandbox violation and starts a new, sandboxed one.
#[cfg(unix)]
#[test]
fn an_engine_killed_by_seccomp_should_be_restarted_and_logged_as_a_violation() {
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let spawner = SandboxSpawner::new(engine(), Vec::new(), SandboxConfig::default(), |_| {});
    let supervisor = Supervisor::start(
        Box::new(spawner),
        Arc::new(NoFiles),
        Config {
            backoff: Duration::from_millis(10),
            ..Config::default()
        },
        move |_, event| {
            let _ = tx.lock().unwrap().send(event);
        },
    )
    .unwrap();
    assert_ne!(
        supervisor.sandbox(),
        Some(SandboxLevel::default()),
        "the engine is not sandboxed"
    );
    let pid = supervisor.pid().unwrap();
    let killed = Command::new("kill").args(["-SYS", &pid.to_string()]).status().unwrap();
    assert!(killed.success());
    let mut logged = false;
    let restarted = loop {
        match events.recv_timeout(Duration::from_secs(10)) {
            Ok(Event::Log(line)) => logged |= line.contains("sandbox violation"),
            Ok(Event::Restarted { pid }) => break Some(pid),
            Ok(Event::Failed) | Err(_) => break None,
        }
    };
    assert!(logged, "the exit was not logged as a sandbox violation");
    assert!(restarted.is_some_and(|new| new != pid), "no restart");
    supervisor.shutdown();
}
