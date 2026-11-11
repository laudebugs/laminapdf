//! The engine's sandbox, from the outside (ADR 0008 §5): the built engine started the way the app
//! starts it (`spawn_sandboxed`), running `--sandbox-selftest=<probe>`, one probe per run. What
//! must be blocked is blocked on every OS; on Linux a forbidden system call kills the engine
//! (SIGSYS) rather than failing. Threads and rendering keep working. With PDFium in
//! `LAMINA_PDF_PDFIUM_DIR` the thread probe renders too.
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

use std::ffi::OsString;
use std::io::Read as _;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use laminapdf_desktop_protocol::supervisor::{Config, Event, Process as _, RangeServer, Supervisor};
use laminapdf_desktop_protocol::{DocId, Layer};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner, spawn_sandboxed};

fn engine() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_laminapdf-desktop-engine"))
}

/// What one selftest run printed on stdout and how it ended.
struct Run {
    stdout: String,
    stderr: String,
    exit: String,
}

impl Run {
    /// The outcome printed for `probe`: `blocked (…)` or `allowed (…)`, or `None` if it died first.
    fn outcome(&self, probe: &str) -> Option<&str> {
        let prefix = format!("selftest {probe}: ");
        self.stdout.lines().find_map(|line| line.strip_prefix(prefix.as_str()))
    }

    /// Killed by seccomp, or (with the `seccomp-trap` feature, which `--all-features` turns on)
    /// trapped and reported by name.
    fn killed_by_seccomp(&self) -> bool {
        self.exit.contains("SIGSYS") || self.stderr.contains("seccomp refused system call")
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
    Run { stdout, stderr, exit }
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
    // Every layer this OS requires, by its log name (the main process refuses an engine without).
    let expected: Vec<String> = Layer::REQUIRED.iter().map(ToString::to_string).collect();
    for layer in &expected {
        assert!(layers.split(' ').any(|l| l == layer), "{layer} missing from {layers}");
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
        // The main process's watchdog kills it while it holds the memory.
        assert!(
            outcome.is_none() && run.exit.contains("memory limit"),
            "{outcome:?} ({})",
            run.exit
        );
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
        stderr: String::new(),
        exit: child.process.wait(),
    };
    assert_eq!(plain.outcome("sandbox"), Some("off"));
    let limit = sandboxed.outcome("nproc").unwrap();
    assert!(limit.starts_with("allowed"), "{limit}");
    assert_eq!(Some(limit), plain.outcome("nproc"), "RLIMIT_NPROC changed");
}

#[cfg(unix)]
#[test]
fn the_engine_should_inherit_no_descriptor_but_its_pipes() {
    // A descriptor without close-on-exec, as GTK or D-Bus leave in the main process.
    let file = std::fs::File::open(engine()).unwrap();
    let leaked = laminapdf_desktop_sandbox::probe::inheritable_copy(std::os::fd::AsFd::as_fd(&file)).unwrap();
    let run = selftest("descriptors");
    drop(leaked);
    assert_eq!(
        run.outcome("descriptors"),
        Some("blocked (no descriptor above 2)"),
        "{}",
        run.exit
    );
}

/// No document is served in these tests.
struct NoFiles;

impl RangeServer for NoFiles {
    fn read(&self, _: DocId, _: u64, _: u32) -> std::io::Result<Vec<u8>> {
        Err(std::io::Error::other("no files"))
    }
}

/// The engine started with `args` and `sandbox` under a supervisor, as the app runs it, and the
/// supervisor's events.
fn supervised(args: &[&str], sandbox: SandboxConfig) -> (Supervisor, mpsc::Receiver<Event>) {
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let args = args.iter().map(OsString::from).collect();
    let spawner = SandboxSpawner::new(engine(), args, sandbox, |line| eprintln!("engine: {line}"));
    let config = Config {
        backoff: Duration::from_millis(10),
        ..Config::default()
    };
    let supervisor = Supervisor::start(Box::new(spawner), Arc::new(NoFiles), config, move |_, event| {
        if let Event::Log(line) = &event {
            eprintln!("supervisor: {line}");
        }
        let _ = tx.lock().unwrap().send(event);
    })
    .unwrap();
    (supervisor, events)
}

/// Waits for the next restart; whether a log line before it contained `logged`.
fn restart_logging(events: &mpsc::Receiver<Event>, logged: &str) -> (Option<u32>, bool) {
    let mut seen = false;
    loop {
        match events.recv_timeout(Duration::from_secs(20)) {
            Ok(Event::Log(line)) => seen |= line.contains(logged),
            Ok(Event::Restarted { pid }) => return (Some(pid), seen),
            Ok(Event::Failed) | Err(_) => return (None, seen),
        }
    }
}

/// An engine that runs past its memory limit (`exhaust`: `Ready`, then allocation until it dies)
/// is stopped by the sandbox (Windows: the job; macOS: the main process's watchdog; Linux:
/// `RLIMIT_AS`, an abort), and the supervisor restarts it.
#[test]
fn an_engine_past_its_memory_limit_should_be_stopped_and_restarted() {
    let sandbox = SandboxConfig {
        memory_limit: 256 << 20,
    };
    let (supervisor, events) = supervised(&["--sandbox-selftest=exhaust"], sandbox);
    let first = supervisor.pid();
    let (restarted, named) = restart_logging(&events, "memory limit");
    supervisor.shutdown();
    assert!(restarted.is_some() && restarted != first, "no restart");
    if cfg!(any(windows, target_os = "macos")) {
        assert!(named, "the exit was not logged as the memory limit");
    }
}

/// The supervisor's side of a sandbox kill (Unix: the test sends the signal). Not with
/// `seccomp-trap`, whose handler catches the signal.
#[cfg(all(unix, not(feature = "seccomp-trap")))]
#[test]
fn an_engine_killed_by_seccomp_should_be_restarted_and_logged_as_a_violation() {
    let (supervisor, events) = supervised(&[], SandboxConfig::default());
    assert!(
        supervisor.sandbox().is_some_and(|level| level.missing().is_empty()),
        "the engine is not sandboxed"
    );
    let pid = supervisor.pid().unwrap();
    let killed = Command::new("kill").args(["-SYS", &pid.to_string()]).status().unwrap();
    assert!(killed.success());
    let (restarted, named) = restart_logging(&events, "sandbox violation");
    supervisor.shutdown();
    assert!(named, "the exit was not logged as a sandbox violation");
    assert!(restarted.is_some_and(|new| new != pid), "no restart");
}
