//! How a window's engine process starts: sandboxed, with arguments read at each start.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use laminapdf_desktop_protocol::supervisor::{Spawned, Spawner};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner};

/// Starts the engine inside its sandbox (ADR 0008 §5), reading its arguments at every start, a
/// supervisor's restart included. The crash monitor's socket is among them, and a monitor that
/// restarted listens on a new one: arguments read once would point a later engine at the old.
pub(crate) struct SandboxedEngine<A> {
    program: PathBuf,
    args: A,
    log: Arc<dyn Fn(String) + Send + Sync>,
}

impl<A: Fn() -> Vec<OsString> + Send + Sync> SandboxedEngine<A> {
    /// `program` with what `args` returns at each start; each line of the engine's stderr goes to
    /// `log`.
    pub(crate) fn new(program: PathBuf, args: A, log: impl Fn(String) + Send + Sync + 'static) -> Self {
        Self {
            program,
            args,
            log: Arc::new(log),
        }
    }
}

impl<A: Fn() -> Vec<OsString> + Send + Sync> Spawner for SandboxedEngine<A> {
    fn spawn(&self) -> io::Result<Spawned> {
        let log = Arc::clone(&self.log);
        SandboxSpawner::new(
            self.program.clone(),
            (self.args)(),
            SandboxConfig::default(),
            move |line| {
                log(line);
            },
        )
        .spawn()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn each_start_should_read_the_arguments_again() {
        let reads = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&reads);
        let engine = SandboxedEngine::new(
            PathBuf::from("/bin/sh"),
            move || {
                counted.fetch_add(1, Ordering::SeqCst);
                vec!["-c".into(), "exit 0".into()]
            },
            |_| {},
        );
        for _ in 0..2 {
            let mut started = engine.spawn().unwrap();
            let _ = started.process.wait();
        }
        assert_eq!(reads.load(Ordering::SeqCst), 2);
    }
}
