//! Opt-in crash reporting (D7a; desktop plan req. 9, S07 Privacy, S08). A crash anywhere in the app
//! becomes a [`report::Report`]: built from an allow-list, scrubbed of paths and names, and kept on
//! this computer. It leaves only with the user's consent ([`consent`]): asked once, after the
//! first crash, or always from Settings › Privacy. What is sent is exactly what the app showed.
//!
//! Where crashes come from:
//!
//! - **The main process's panics**: the panic hook ([`report_panics`]) writes the report at once,
//!   since the process may be about to end; the next launch asks about it (S08).
//! - **The engine's crashes**: the supervisor's `Crashed` event, with the summary the engine's own
//!   hook sent if it panicked, else how it ended. The window gets [`ASK_EVENT`] when the app may
//!   ask: a notice in its status bar, not a dialog, since the engine has already restarted.
//! - **The webview's uncaught errors**: `report_ui_error`, with the stack parsed here, in Rust.
//!
//! Native crashes of the main process and the engine's minidumps are D7d's.

pub(crate) mod commands;
mod consent;
mod queue;
mod report;
mod scrub;
mod upload;

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::crash::summarize;
use serde::Serialize;
use tauri::{Emitter as _, EventTarget};

use self::consent::{ConsentState, ConsentStore};
use self::queue::{Queue, Shelf};
pub(crate) use self::report::{Crash, ProcessKind};
use self::report::{AppFacts, Report, ReportChannel};

/// The event a window gets when the app may ask about a crash that happened while it ran.
pub(crate) const ASK_EVENT: &str = "crash://ask";

/// The most webview errors reported in one run: a page that throws in a loop sends no flood.
const MAX_WEBVIEW_REPORTS: u32 = 5;

/// The first wait before sending again after the Worker couldn't be reached, doubled up to an hour.
const FIRST_RETRY: Duration = Duration::from_secs(60);
const LAST_RETRY: Duration = Duration::from_secs(3600);

/// The question the app asks about a crash: which process, and the report, as it would be sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Question {
    pub id: String,
    pub process: ProcessKind,
    /// The report's JSON, laid out for reading; its fields and values are exactly those sent.
    pub report: String,
}

/// The app's crash reports: the consent, the reports kept on disk, and their sending.
pub(crate) struct CrashReports {
    consent: ConsentStore,
    queue: Queue,
    facts: AppFacts,
    started: Instant,
    endpoint: String,
    sending: AtomicBool,
    webview_reports: AtomicU32,
}

impl CrashReports {
    /// The reports of the app of `version`, its consent in `config_dir`, its reports in
    /// `data_dir/crashes`; reports past their age go now.
    pub(crate) fn new(config_dir: &Path, data_dir: &Path, version: &str) -> Arc<Self> {
        let queue = Queue::new(data_dir.join("crashes"));
        let now = std::time::SystemTime::now();
        queue.prune(Shelf::Held, now);
        queue.prune(Shelf::Outbox, now);
        Arc::new(Self {
            consent: ConsentStore::load(config_dir.join("crash.json")),
            queue,
            facts: AppFacts {
                app_version: version.to_owned(),
                channel: channel(version),
                os_version: os_version(),
            },
            started: Instant::now(),
            endpoint: endpoint(),
            sending: AtomicBool::new(false),
            webview_reports: AtomicU32::new(0),
        })
    }

    fn build(&self, crash: Crash) -> Report {
        crash.report(&self.facts, self.started.elapsed())
    }

    /// Keeps the report of `crash`, in `window` if it happened in one: sent if the user said
    /// always, else held; and if the app hasn't asked yet, the window hears it may ask now.
    pub(crate) fn record(self: &Arc<Self>, app: &tauri::AppHandle, window: Option<&str>, crash: Crash) {
        if crash.process == ProcessKind::Webview
            && self.webview_reports.fetch_add(1, Ordering::SeqCst) >= MAX_WEBVIEW_REPORTS
        {
            return;
        }
        let consent = self.consent.get();
        let place = if consent.sends() { Shelf::Outbox } else { Shelf::Held };
        if let Err(error) = self.queue.put(place, &self.build(crash)) {
            tracing::warn!("could not keep a crash report: {error}");
            return;
        }
        if consent.sends() {
            self.send_soon();
        } else if let (true, Some(window)) = (consent.asks(), window) {
            let _ = app.emit_to(EventTarget::webview_window(window), ASK_EVENT, ());
        }
    }

    /// The newest held report, while the app may still ask.
    pub(crate) fn question(&self) -> Option<Question> {
        if !self.consent.get().asks() {
            return None;
        }
        let report = self.queue.list(Shelf::Held).into_iter().next()?;
        Some(Question {
            id: report.report_id.clone(),
            process: report.process,
            report: serde_json::to_string_pretty(&report).ok()?,
        })
    }

    /// The user's answer about report `id`: send it or drop it, and whether to send from now on.
    pub(crate) fn answer(self: &Arc<Self>, id: &str, send: bool, always: bool) {
        self.consent.set(ConsentState::answered(send, always));
        if send && self.queue.release(id) {
            self.send_soon();
        } else {
            self.queue.take(Shelf::Held, id);
        }
    }

    /// Whether every report is sent (Settings › Privacy).
    pub(crate) fn enabled(&self) -> bool {
        self.consent.get().sends()
    }

    /// Settings › Privacy's switch. Off, the outbox's unsent reports go too.
    pub(crate) fn set_enabled(&self, on: bool) {
        self.consent.set(ConsentState::switched(on));
        if !on {
            self.queue.clear(Shelf::Outbox);
        }
    }

    /// A report as this app would send one, for "See a sample report".
    pub(crate) fn sample(&self) -> String {
        let crash = Crash::engine_exit("crash: signal: 11 (SIGSEGV)".into(), None);
        serde_json::to_string_pretty(&self.build(crash)).unwrap_or_default()
    }

    /// Sends the outbox in the background, again after a growing wait while the Worker can't be
    /// reached; one pass at a time.
    pub(crate) fn send_soon(self: &Arc<Self>) {
        if self.sending.swap(true, Ordering::SeqCst) {
            return;
        }
        let this = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let mut wait = FIRST_RETRY;
            while !upload::send_outbox(&this.queue, &this.endpoint).await {
                tokio::time::sleep(wait).await;
                wait = (wait * 2).min(LAST_RETRY);
            }
            this.sending.store(false, Ordering::SeqCst);
        });
    }

    /// Starts sending what earlier runs left in the outbox, if the user said always.
    pub(crate) fn resume(self: &Arc<Self>) {
        if self.enabled() {
            self.send_soon();
        }
    }
}

/// The reports the panic hook writes to.
static FOR_PANICS: OnceLock<Arc<CrashReports>> = OnceLock::new();

/// Makes every panic in the main process leave a report, kept before the previous hook runs, since
/// the process may end right after. The report waits for the next launch to be asked about or sent.
pub(crate) fn report_panics(reports: &Arc<CrashReports>) {
    if FOR_PANICS.set(Arc::clone(reports)).is_err() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(reports) = FOR_PANICS.get() {
            let crash = Crash::panic(ProcessKind::Main, summarize(info, "laminapdf-desktop"), None);
            // A panic while the consent is being changed counts as not yet allowed.
            let sends = reports.consent.try_get().is_some_and(ConsentState::sends);
            let place = if sends { Shelf::Outbox } else { Shelf::Held };
            let _ = reports.queue.put(place, &reports.build(crash));
        }
        previous(info);
    }));
}

/// The channel this build follows; `dev` for a debug build.
fn channel(version: &str) -> ReportChannel {
    use crate::update::settings::Channel;
    if cfg!(debug_assertions) {
        return ReportChannel::Dev;
    }
    match crate::update::policy::Build::current(version).channel {
        Channel::Stable => ReportChannel::Stable,
        Channel::Beta => ReportChannel::Beta,
        Channel::Nightly => ReportChannel::Nightly,
    }
}

/// The OS's version, as the schema allows it: digits, letters and a few separators, 40 at most.
fn os_version() -> String {
    os_info::get()
        .version()
        .to_string()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ' ' | '(' | ')' | '_' | '-'))
        .take(40)
        .collect()
}

/// Where reports go: the update Worker; the end-to-end build may point it at a local server.
fn endpoint() -> String {
    #[cfg(feature = "e2e")]
    if let Ok(endpoint) = std::env::var("LAMINA_PDF_CRASH_ENDPOINT") {
        return endpoint;
    }
    upload::CRASH_ENDPOINT.to_owned()
}
