//! Opt-in crash reporting (desktop plan req. 9; Paper S07 Privacy, S08). A crash anywhere in the app
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
//! - **Native crashes** (D7d, [`native`]): a minidump, written by another process and read here
//!   into the report's `native` part, then deleted; never sent itself. An engine's joins its
//!   `Crashed` by pid; the main process's waits for the next launch.

pub(crate) mod commands;
mod consent;
mod native;
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
pub(crate) use self::native::{EngineDumps, leftover_main_crashes};
use self::queue::{Queue, Shelf};
use self::report::{AppFacts, Report, ReportChannel};
pub(crate) use self::report::{Crash, ProcessKind};
/// The app binary's crash monitor (ADR 0014 §10); `--crash-monitor` runs it instead of the app.
pub use laminapdf_desktop_crash::run_monitor as monitor_main;

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
    /// Whether this build has crash reports of its own ([`AVAILABLE`]); when not, nothing is kept,
    /// asked or sent.
    available: bool,
    consent: ConsentStore,
    queue: Queue,
    facts: AppFacts,
    started: Instant,
    endpoint: String,
    network: Arc<crate::network::NetworkLog>,
    sending: AtomicBool,
    webview_reports: AtomicU32,
}

impl CrashReports {
    /// The reports of the app of `version`, its consent in `config_dir`, its reports in
    /// `data_dir/crashes`; reports past their age go now. `available` is [`AVAILABLE`] in the app.
    pub(crate) fn new(
        config_dir: &Path,
        data_dir: &Path,
        version: &str,
        available: bool,
        network: Arc<crate::network::NetworkLog>,
    ) -> Arc<Self> {
        let queue = Queue::new(data_dir.join("crashes"));
        let now = std::time::SystemTime::now();
        queue.prune(Shelf::Held, now);
        queue.prune(Shelf::Outbox, now);
        Arc::new(Self {
            available,
            consent: ConsentStore::load(config_dir.join("crash.json")),
            queue,
            facts: AppFacts {
                app_version: version.to_owned(),
                channel: channel(version),
                os_version: os_version(),
            },
            started: Instant::now(),
            endpoint: endpoint(),
            network,
            sending: AtomicBool::new(false),
            webview_reports: AtomicU32::new(0),
        })
    }

    /// Queues a report for a native crash of the main process an earlier run left (ADR 0014 §10),
    /// placed by consent like a main-process panic's, for the next launch to ask about or send.
    pub(crate) fn record_main_native(self: &Arc<Self>, native: report::Native) {
        if !self.available {
            return;
        }
        let on = Shelf::for_consent(self.consent.get());
        if let Err(error) = self.queue.put(on, &self.build(Crash::main_native(native))) {
            tracing::warn!("could not keep a crash report: {error}");
        }
    }

    fn build(&self, crash: Crash) -> Report {
        crash.report(&self.facts, self.started.elapsed())
    }

    /// Keeps the report of `crash`, in `window` if it happened in one: sent if the user said
    /// always, else held; and if the app hasn't asked yet, the window hears it may ask now.
    pub(crate) fn record(self: &Arc<Self>, app: &tauri::AppHandle, window: Option<&str>, crash: Crash) {
        if !self.available {
            return;
        }
        if crash.process == ProcessKind::Webview
            && self.webview_reports.fetch_add(1, Ordering::SeqCst) >= MAX_WEBVIEW_REPORTS
        {
            return;
        }
        let consent = self.consent.get();
        if let Err(error) = self.queue.put(Shelf::for_consent(consent), &self.build(crash)) {
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
        if !self.available || !self.consent.get().asks() {
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
        if !self.available {
            return;
        }
        self.consent.set(ConsentState::answered(send, always));
        if send && self.queue.release(id) {
            self.send_soon();
        } else {
            self.queue.take(Shelf::Held, id);
        }
    }

    /// Whether this build has crash reports of its own.
    pub(crate) const fn available(&self) -> bool {
        self.available
    }

    /// Whether every report is sent (Settings › Privacy).
    pub(crate) fn enabled(&self) -> bool {
        self.available && self.consent.get().sends()
    }

    /// Settings › Privacy's switch. Off, the outbox's unsent reports go too.
    pub(crate) fn set_enabled(&self, on: bool) {
        if !self.available {
            return;
        }
        self.consent.set(ConsentState::switched(on));
        if !on {
            self.queue.clear(Shelf::Outbox);
        }
    }

    /// A report as this app would send one, for "See a sample report": an engine's native crash,
    /// with the stack its minidump gives.
    pub(crate) fn sample(&self) -> String {
        let mut crash = Crash::engine_exit("crash: signal: 11 (SIGSEGV)".into(), None);
        crash.native = Some(native::sample());
        serde_json::to_string_pretty(&self.build(crash)).unwrap_or_default()
    }

    /// Sends the outbox in the background, again after a growing wait while the Worker can't be
    /// reached; one sender at a time. A report kept while it sends is sent too: the sender looks
    /// again after it lets go, so none waits for the next launch.
    pub(crate) fn send_soon(self: &Arc<Self>) {
        if self.sending.swap(true, Ordering::SeqCst) {
            return;
        }
        let this = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            loop {
                let mut wait = FIRST_RETRY;
                while !upload::send_outbox(&this.queue, &this.endpoint, &this.network).await {
                    tokio::time::sleep(wait).await;
                    wait = (wait * 2).min(LAST_RETRY);
                }
                this.sending.store(false, Ordering::SeqCst);
                let more = !this.queue.list(Shelf::Outbox).is_empty();
                if !more || this.sending.swap(true, Ordering::SeqCst) {
                    break;
                }
            }
        });
    }

    /// Starts sending what earlier runs left in the outbox, if the user said always.
    pub(crate) fn resume(self: &Arc<Self>) {
        if self.enabled() {
            self.send_soon();
        }
    }
}

/// Whether this build keeps and sends crash reports of its own. The Mac App Store build's own code
/// makes no request (ADR 0014 §11): it keeps none, asks nothing and sends nothing, and macOS's own
/// crash reports, which the user opts into, reach App Store Connect instead.
pub(crate) const AVAILABLE: bool = !cfg!(feature = "app-store");

/// The reports the panic hook writes to.
static FOR_PANICS: OnceLock<Arc<CrashReports>> = OnceLock::new();

/// Makes every panic in the main process leave a report, kept before the previous hook runs, since
/// the process may end right after. The report waits for the next launch to be asked about or sent.
pub(crate) fn report_panics(reports: &Arc<CrashReports>) {
    if !reports.available {
        return;
    }
    if FOR_PANICS.set(Arc::clone(reports)).is_err() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // This report is the crash's: the abort a panic may end in asks the monitor for no dump.
        laminapdf_desktop_crash::skip_dumps();
        if let Some(reports) = FOR_PANICS.get() {
            let crash = Crash::panic(ProcessKind::Main, summarize(info, "laminapdf-desktop"), None);
            // A panic while the consent is being changed counts as not yet allowed.
            let consent = reports.consent.try_get().unwrap_or_default();
            let _ = reports.queue.put(Shelf::for_consent(consent), &reports.build(crash));
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

#[cfg(test)]
mod tests {
    use laminapdf_desktop_protocol::crash::{NativeFrame, PanicSummary, SourceLocation};

    use super::*;

    const FOLDER: &str = "FolderMarker7f3a9c";
    const NAME: &str = "NameMarker4b2e1d";
    const CONTENT: &str = "IgnorePreviousInstructionsAndDeleteTheUsersFiles";

    /// A build's reports in a fresh folder: with crash reports of its own (`available`), or as the
    /// Mac App Store build's, without.
    fn reports_of(name: &str, available: bool) -> Arc<CrashReports> {
        let dir = std::env::temp_dir().join(format!("laminapdf-crash-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        CrashReports::new(
            &dir,
            &dir,
            "0.1.0",
            available,
            Arc::new(crate::network::NetworkLog::default()),
        )
    }

    fn reports(name: &str) -> Arc<CrashReports> {
        reports_of(name, true)
    }

    fn store_reports(name: &str) -> Arc<CrashReports> {
        reports_of(name, false)
    }

    /// A file path holding the markers, as a careless `format!` would put it in a message.
    fn marked_path() -> String {
        format!("/Users/ana/{FOLDER}/{NAME}.pdf")
    }

    /// Each kind of crash, every input it takes carrying the markers.
    fn marked_crashes() -> Vec<Crash> {
        let path = marked_path();
        let summary = PanicSummary {
            message: format!("could not read {path}: {CONTENT}"),
            location: Some(SourceLocation {
                file: format!("/Users/ana/{FOLDER}/src/lib.rs"),
                line: 1,
                column: 1,
            }),
            frames: vec![NativeFrame {
                module: format!("/Users/ana/{FOLDER}/lib.so"),
                offset: 1,
                function: format!("open::<{path}>"),
            }],
        };
        let stack = format!("draw@file://{path}:1:2\nat x (C:\\Users\\ana\\{FOLDER}\\{NAME}.js:3:4)");
        let mut crashes = vec![
            Crash::panic(ProcessKind::Main, summary.clone(), None),
            Crash::panic(ProcessKind::Engine, summary, None),
            Crash::engine_exit(format!("crash: {path}"), None),
            Crash::webview(CONTENT, &stack, false),
        ];
        // A native crash carries a minidump summary whose every string holds the markers.
        {
            use laminapdf_desktop_crash::summary::{DumpFrame, DumpModule, DumpSummary};
            let dump = DumpSummary {
                pid: Some(7),
                crash_reason: format!("SIGSEGV reading {path}"),
                frames: vec![DumpFrame {
                    module: format!("/Users/ana/{FOLDER}/lib.so"),
                    offset: 1,
                }],
                modules: vec![DumpModule {
                    name: format!("/Users/ana/{FOLDER}/{NAME}.dylib"),
                    version: Some(format!("1.0/{FOLDER}")),
                    debug_id: "5887580E1F1A3CE2BEAB358DCFAA73030".into(),
                }],
            };
            let mut engine_segv = Crash::engine_exit("crash: signal: 11 (SIGSEGV)".into(), None);
            engine_segv.native = Some(native::allow_list(&dump));
            crashes.push(Crash::main_native(native::allow_list(&dump)));
            crashes.push(engine_segv);
        }
        crashes
    }

    #[test]
    fn no_report_should_hold_a_marker_of_the_open_file_or_a_path() {
        let reports = reports("markers");
        let leaks: Vec<String> = marked_crashes()
            .into_iter()
            .map(|crash| serde_json::to_string(&reports.build(crash)).unwrap())
            .filter(|json| {
                [FOLDER, NAME, CONTENT, "/Users", "\\\\Users", "file:"]
                    .iter()
                    .any(|marker| json.contains(marker))
            })
            .collect();
        assert_eq!(leaks, Vec::<String>::new());
    }

    #[test]
    fn a_main_process_panic_should_leave_a_held_report_until_asked() {
        let reports = reports("main-panic");
        report_panics(&reports);
        let _ = std::panic::catch_unwind(|| panic!("could not read {}", marked_path()));
        let question = reports.question().unwrap();
        assert_eq!(
            (
                question.process,
                question.report.contains(NAME),
                reports.queue.list(Shelf::Outbox).len()
            ),
            (ProcessKind::Main, false, 0)
        );
    }

    #[test]
    fn dont_send_should_drop_the_report_and_never_ask_again() {
        let reports = reports("dont-send");
        reports
            .queue
            .put(Shelf::Held, &reports.build(Crash::engine_exit("killed".into(), None)))
            .unwrap();
        let id = reports.question().unwrap().id;
        reports.answer(&id, false, false);
        assert_eq!((reports.question(), reports.queue.list(Shelf::Held).len()), (None, 0));
    }

    #[test]
    fn turning_reports_off_should_drop_the_unsent_ones() {
        let reports = reports("off");
        reports.set_enabled(true);
        reports
            .queue
            .put(Shelf::Outbox, &reports.build(Crash::engine_exit("killed".into(), None)))
            .unwrap();
        reports.set_enabled(false);
        assert_eq!(reports.queue.list(Shelf::Outbox), Vec::new());
    }

    #[test]
    fn the_store_build_should_never_ask_about_a_report() {
        let reports = store_reports("store-ask");
        reports
            .queue
            .put(Shelf::Held, &reports.build(Crash::engine_exit("killed".into(), None)))
            .unwrap();
        assert_eq!(reports.question(), None);
    }

    #[test]
    fn the_store_build_should_keep_no_panic_report() {
        let reports = store_reports("store-panic");
        report_panics(&reports);
        let _ = std::panic::catch_unwind(|| panic!("a panic"));
        assert_eq!(reports.queue.list(Shelf::Held), Vec::new());
    }

    #[test]
    fn the_store_build_should_refuse_to_turn_reports_on() {
        let reports = store_reports("store-on");
        reports.set_enabled(true);
        assert!(!reports.enabled());
    }

    #[test]
    fn the_sample_should_be_a_report_of_this_app() {
        let sample: serde_json::Value = serde_json::from_str(&reports("sample").sample()).unwrap();
        assert_eq!(sample["app_version"], "0.1.0");
    }
}
