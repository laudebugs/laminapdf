//! macOS: the memory limit. macOS doesn't enforce `RLIMIT_AS`, so the main process watches the
//! engine's physical footprint (`proc_pid_rusage`'s `ri_phys_footprint`, what Activity Monitor
//! shows as Memory) about once a second and kills it above the limit. The supervisor restarts it
//! like any crash; `wait` names the cause.
#![expect(unsafe_code, reason = "FFI: proc_pid_rusage")]

use std::io;
use std::process::Child;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

/// How often the footprint is read.
const PERIOD: Duration = Duration::from_secs(1);

/// The engine and its watchdog.
pub(super) struct Watched {
    child: Arc<Mutex<Child>>,
    /// The footprint the watchdog killed it at; 0 while it hasn't.
    killed_at: Arc<AtomicU64>,
    limit: u64,
}

impl Watched {
    /// Starts watching `child` against `limit` bytes; `log` hears if the watch stops early.
    ///
    /// # Errors
    /// The watchdog thread couldn't start. A release build then kills the engine and refuses it:
    /// without the watchdog it would run with no memory limit at all. A debug build logs it and
    /// lets the engine run.
    pub(super) fn new(child: Child, limit: u64, log: &Arc<dyn Fn(String) + Send + Sync>) -> io::Result<Self> {
        let child = Arc::new(Mutex::new(child));
        let killed_at = Arc::new(AtomicU64::new(0));
        let (watched, over, report) = (Arc::clone(&child), Arc::clone(&killed_at), Arc::clone(log));
        let started = thread::Builder::new()
            .name("engine-memory".into())
            .spawn(move || watch(&watched, &over, limit, report.as_ref()));
        if let Err(error) = started {
            if !cfg!(debug_assertions) {
                let _ = lock(&child).kill();
                return Err(error);
            }
            log(format!(
                "the engine's memory watchdog didn't start, so it has no memory limit: {error}"
            ));
        }
        Ok(Self {
            child,
            killed_at,
            limit,
        })
    }

    pub(super) fn id(&self) -> u32 {
        lock(&self.child).id()
    }

    pub(super) fn kill(&self) {
        // Fails only when it has already exited.
        let _ = lock(&self.child).kill();
    }

    /// Waits for the engine to end, without holding the lock the watchdog needs.
    pub(super) fn wait(&self) -> String {
        loop {
            match lock(&self.child).try_wait() {
                Ok(Some(status)) => {
                    let footprint = self.killed_at.load(Ordering::SeqCst);
                    if footprint > 0 {
                        return format!(
                            "memory limit: the watchdog stopped it at {} MiB (limit {} MiB)",
                            footprint >> 20,
                            self.limit >> 20
                        );
                    }
                    return super::describe(status);
                }
                Ok(None) => {}
                Err(error) => return format!("unknown ({error})"),
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

/// Reads the footprint every [`PERIOD`] until the engine ends or passes `limit`.
fn watch(child: &Mutex<Child>, killed_at: &AtomicU64, limit: u64, log: &(dyn Fn(String) + Send + Sync)) {
    loop {
        thread::sleep(PERIOD);
        let mut child = lock(child);
        // Exited (or reaped): its pid may belong to someone else now.
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        let footprint = match footprint(child.id()) {
            Ok(footprint) => footprint,
            Err(error) => {
                log(format!(
                    "the engine's memory watchdog stopped, so it has no memory limit: {error}"
                ));
                return;
            }
        };
        if footprint > limit {
            killed_at.store(footprint, Ordering::SeqCst);
            let _ = child.kill();
            return;
        }
    }
}

/// The process's physical footprint in bytes.
fn footprint(pid: u32) -> io::Result<u64> {
    let pid = libc::c_int::try_from(pid).map_err(|_| io::Error::other("a pid past c_int"))?;
    // SAFETY: a zeroed rusage_info_v2 is valid plain data; proc_pid_rusage writes at most one
    // rusage_info_v2 (the flavour asked for) through the pointer, which is valid for it.
    unsafe {
        let mut info: libc::rusage_info_v2 = std::mem::zeroed();
        let status = libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V2, (&raw mut info).cast());
        if status == 0 {
            Ok(info.ri_phys_footprint)
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

fn lock(child: &Mutex<Child>) -> MutexGuard<'_, Child> {
    child.lock().unwrap_or_else(PoisonError::into_inner)
}
