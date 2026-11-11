//! The main process's half: starting the engine inside its sandbox, and saying how it ended.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::thread;

use laminapdf_desktop_protocol::supervisor::{Process, Spawned, Spawner};

use crate::{MEMORY_LIMIT_ARG, SandboxConfig, SandboxError};

/// A started engine: its three pipes and the process.
pub struct SandboxedChild {
    /// The engine's stdin.
    pub stdin: Box<dyn Write + Send>,
    /// The engine's stdout.
    pub stdout: Box<dyn Read + Send>,
    /// The engine's stderr: its log lines.
    pub stderr: Box<dyn Read + Send>,
    /// The process.
    pub process: SandboxedProcess,
}

/// Starts `command` (the engine binary and its arguments) inside the sandbox, with piped stdio and
/// [`MEMORY_LIMIT_ARG`] added for the engine's own half.
///
/// On Windows the process starts suspended in a job object, with a locked-down Low-integrity
/// token, an impersonation token for its start-up, and mitigation policies that forbid child
/// processes; then it is resumed. Elsewhere it starts with only its three pipes open (every other
/// descriptor closed before `exec`) and sandboxes itself; on macOS a watchdog thread enforces the
/// memory limit. With the
/// sandbox off for diagnosis ([`sandbox_disabled`](crate::sandbox_disabled)) it starts plainly everywhere.
///
/// # Errors
/// [`SandboxError`] when the process or a sandbox object can't be created.
pub fn spawn_sandboxed(mut command: Command, config: &SandboxConfig) -> Result<SandboxedChild, SandboxError> {
    command.arg(MEMORY_LIMIT_ARG).arg(config.memory_limit.to_string());
    #[cfg(windows)]
    if !crate::sandbox_disabled() {
        return windows::spawn(&command, config);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: the engine is a console program; the app shows no console.
        command.creation_flags(0x0800_0000);
    }
    #[cfg(unix)]
    if !crate::sandbox_disabled() {
        unix::close_inherited(&mut command);
    }
    let mut child = command.spawn()?;
    let missing = || SandboxError::Spawn(io::Error::other("a pipe was not set up"));
    let stdin = child.stdin.take().ok_or_else(missing)?;
    let stdout = child.stdout.take().ok_or_else(missing)?;
    let stderr = child.stderr.take().ok_or_else(missing)?;
    #[cfg(target_os = "macos")]
    let inner = if crate::sandbox_disabled() {
        Inner::Std(child)
    } else {
        Inner::Watched(macos::Watched::new(child, config.memory_limit))
    };
    #[cfg(not(target_os = "macos"))]
    let inner = Inner::Std(child);
    Ok(SandboxedChild {
        stdin: Box::new(stdin),
        stdout: Box::new(stdout),
        stderr: Box::new(stderr),
        process: SandboxedProcess(inner),
    })
}

/// An engine process: [`Process`] for the supervisor, whose `wait` names how it ended.
pub struct SandboxedProcess(Inner);

enum Inner {
    Std(Child),
    #[cfg(target_os = "macos")]
    Watched(macos::Watched),
    #[cfg(windows)]
    Job(windows::JobProcess),
}

impl Process for SandboxedProcess {
    fn id(&self) -> u32 {
        match &self.0 {
            Inner::Std(child) => child.id(),
            #[cfg(target_os = "macos")]
            Inner::Watched(process) => process.id(),
            #[cfg(windows)]
            Inner::Job(process) => process.id(),
        }
    }

    fn kill(&mut self) {
        match &mut self.0 {
            // Fails only when it has already exited.
            Inner::Std(child) => drop(child.kill()),
            #[cfg(target_os = "macos")]
            Inner::Watched(process) => process.kill(),
            #[cfg(windows)]
            Inner::Job(process) => process.kill(),
        }
    }

    fn wait(&mut self) -> String {
        match &mut self.0 {
            Inner::Std(child) => match child.wait() {
                Ok(status) => describe(status),
                Err(error) => format!("unknown ({error})"),
            },
            #[cfg(target_os = "macos")]
            Inner::Watched(process) => process.wait(),
            #[cfg(windows)]
            Inner::Job(process) => process.wait(),
        }
    }
}

/// How a process ended, with the sandbox's own endings named: the supervisor logs this line, so
/// a sandbox violation reads as one rather than as an ordinary crash.
fn describe(status: ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt as _;
        match status.signal() {
            Some(libc::SIGSYS) => {
                return "sandbox violation: SIGSYS (a forbidden system call)".into();
            }
            Some(libc::SIGABRT) => {
                return "crash: aborted (SIGABRT; an allocation past the sandbox's memory limit aborts too)".into();
            }
            // The supervisor's own kill (a hang, a shutdown's reap) or the OS's.
            Some(libc::SIGKILL) => return "killed (SIGKILL)".into(),
            Some(_) => return format!("crash: {status}"),
            None => {}
        }
    }
    status.to_string()
}

/// Starts engines for the supervisor: [`spawn_sandboxed`], with each line of the engine's stderr
/// copied to a log.
pub struct SandboxSpawner {
    program: PathBuf,
    args: Vec<OsString>,
    config: SandboxConfig,
    log: Arc<dyn Fn(String) + Send + Sync>,
}

impl SandboxSpawner {
    /// `program` with `args`, sandboxed per `config`; each line the engine writes to stderr goes to
    /// `log`.
    pub fn new(
        program: PathBuf,
        args: Vec<OsString>,
        config: SandboxConfig,
        log: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        Self {
            program,
            args,
            config,
            log: Arc::new(log),
        }
    }
}

impl Spawner for SandboxSpawner {
    fn spawn(&self) -> io::Result<Spawned> {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        let child = spawn_sandboxed(command, &self.config).map_err(|error| match error {
            SandboxError::Spawn(error) => error,
            // Kept whole: the supervisor's log shows the step and the OS's error under it.
            other => io::Error::other(other),
        })?;
        let log = Arc::clone(&self.log);
        let stderr = child.stderr;
        let _ = thread::Builder::new().name("engine-stderr".into()).spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                log(line);
            }
        });
        Ok(Spawned {
            stdin: child.stdin,
            stdout: Box::new(BufReader::new(child.stdout)),
            process: Box::new(child.process),
        })
    }
}
