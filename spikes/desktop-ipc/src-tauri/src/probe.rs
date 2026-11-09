//! Samples the RSS and CPU of the app process and of its webview processes every 100 ms
//! while a run is going, and keeps the peaks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Default, Clone, Copy, Serialize)]
pub struct Sample {
    pub main_rss_peak_mb: f64,
    pub web_rss_peak_mb: f64,
    /// Mean CPU of the app and webview processes together (100 = one core).
    pub cpu_avg_pct: f64,
    pub web_processes: usize,
}

#[derive(Default)]
pub struct Probe {
    running: Arc<AtomicBool>,
    result: Arc<Mutex<Sample>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// The webview's processes: descendants of the app (WebView2, WebKitGTK), and on macOS the
/// WebKit XPC services, which launchd starts, so we take those started after the app.
fn is_webview(system: &System, pid: Pid, app: Pid, app_start: u64) -> bool {
    let Some(process) = system.process(pid) else { return false };
    let name = process.name().to_string_lossy();
    if cfg!(target_os = "macos") && name.contains("com.apple.WebKit") {
        return process.start_time() + 1 >= app_start;
    }
    let mut parent = process.parent();
    for _ in 0..8 {
        match parent {
            Some(p) if p == app => return true,
            Some(p) => parent = system.process(p).and_then(sysinfo::Process::parent),
            None => return false,
        }
    }
    false
}

impl Probe {
    pub fn start(&self) {
        self.stop();
        self.running.store(true, Ordering::SeqCst);
        let (running, result) = (Arc::clone(&self.running), Arc::clone(&self.result));
        let handle = std::thread::spawn(move || {
            let app = Pid::from_u32(std::process::id());
            let mut system = System::new();
            let refresh = ProcessRefreshKind::nothing().with_memory().with_cpu();
            system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
            let app_start = system.process(app).map_or(0, sysinfo::Process::start_time);
            let (mut sample, mut cpu_sum, mut n) = (Sample::default(), 0.0, 0_u32);
            while running.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(100));
                system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
                let (mut web_rss, mut cpu, mut count) = (0_u64, 0.0_f64, 0);
                for (pid, process) in system.processes() {
                    if *pid == app {
                        let mb = process.memory() as f64 / 1_048_576.0;
                        sample.main_rss_peak_mb = sample.main_rss_peak_mb.max(mb);
                        cpu += f64::from(process.cpu_usage());
                    } else if is_webview(&system, *pid, app, app_start) {
                        web_rss += process.memory();
                        cpu += f64::from(process.cpu_usage());
                        count += 1;
                    }
                }
                sample.web_rss_peak_mb = sample.web_rss_peak_mb.max(web_rss as f64 / 1_048_576.0);
                sample.web_processes = sample.web_processes.max(count);
                cpu_sum += cpu;
                n += 1;
            }
            sample.cpu_avg_pct = if n == 0 { 0.0 } else { cpu_sum / f64::from(n) };
            *result.lock().unwrap_or_else(PoisonError::into_inner) = sample;
        });
        *self.thread.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);
    }

    /// Stops sampling and returns the run's peaks.
    pub fn stop(&self) -> Sample {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread.lock().unwrap_or_else(PoisonError::into_inner).take() {
            let _ = handle.join();
        }
        *self.result.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
