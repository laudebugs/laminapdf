//! In-memory log of outgoing network requests made by the main process (updater and crash
//! reporter). The webview's own traffic stays on local app schemes; this log lets Tools ›
//! Network activity show every request that left the machine, with only its host, kind, time,
//! byte count and outcome — never a URL path, query string, file name or install ID.
//!
//! Nothing is written to disk. In the Mac App Store build (`app-store` feature), the app's own
//! code makes no network request (ADR 0014 §11), so the log stays empty.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::Emitter as _;

/// Event broadcast to every window when the session's network activity log changes.
const CHANGED_EVENT: &str = "network://changed";

/// Maximum entries retained in memory per session; oldest drop first.
const MAX_ENTRIES: usize = 200;

/// Why the main process made an outgoing request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RequestKind {
    /// Checking the update server for a newer release.
    UpdateCheck,
    /// Downloading a signed release bundle.
    UpdateDownload,
    /// Uploading an opt-in crash report.
    CrashReport,
}

/// How an outgoing request finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RequestOutcome {
    /// The server answered with success.
    Ok,
    /// The request could not complete (offline, timeout or server error).
    Failed,
    /// The server permanently refused the payload (non-retryable 4xx).
    Refused,
}

/// One outgoing request recorded for the current session: host, kind, time, bytes and outcome only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetworkEntry {
    pub kind: RequestKind,
    /// Host name only — never a path, query string, fragment, file name or install ID.
    pub host: String,
    /// When the request finished, in milliseconds since the Unix epoch.
    pub time_ms: u64,
    pub outcome: RequestOutcome,
    /// Bytes transferred (upload body or downloaded bundle; 0 for manifest checks).
    pub bytes: u64,
}

/// Snapshot handed to the webview by `network_activity` and `network://changed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetworkSnapshot {
    /// Monotonic sequence number incremented on every recorded request, so out-of-order events
    /// never overwrite a newer snapshot in the webview.
    pub seq: u64,
    /// True in the Mac App Store build, whose own code makes no outgoing request.
    pub store_build: bool,
    /// Outgoing requests recorded during this session, oldest first.
    pub entries: Vec<NetworkEntry>,
}

#[derive(Default)]
struct LogState {
    entries: VecDeque<NetworkEntry>,
    seq: u64,
}

/// Session-only, in-memory log of outgoing main-process requests.
pub(crate) struct NetworkLog {
    store_build: bool,
    state: Mutex<LogState>,
    app: Option<tauri::AppHandle>,
}

impl Default for NetworkLog {
    fn default() -> Self {
        Self::new(crate::update::policy::APP_STORE)
    }
}

impl NetworkLog {
    fn new(store_build: bool) -> Self {
        Self {
            store_build,
            state: Mutex::new(LogState::default()),
            app: None,
        }
    }

    /// A log that broadcasts [`CHANGED_EVENT`] to every window of `app` whenever an entry is added.
    pub(crate) fn with_app(app: tauri::AppHandle) -> Self {
        Self {
            store_build: crate::update::policy::APP_STORE,
            state: Mutex::new(LogState::default()),
            app: Some(app),
        }
    }

    /// Current snapshot of this session's outgoing requests.
    pub(crate) fn snapshot(&self) -> NetworkSnapshot {
        if self.store_build {
            return NetworkSnapshot {
                seq: 0,
                store_build: true,
                entries: Vec::new(),
            };
        }
        let state = lock(&self.state);
        NetworkSnapshot {
            seq: state.seq,
            store_build: false,
            entries: state.entries.iter().cloned().collect(),
        }
    }

    /// Appends a sanitised entry for `url` (host only) and returns the updated snapshot.
    fn push(
        &self,
        kind: RequestKind,
        url: &str,
        outcome: RequestOutcome,
        bytes: u64,
        time_ms: u64,
    ) -> Option<NetworkSnapshot> {
        if self.store_build {
            return None;
        }
        let mut state = lock(&self.state);
        if state.entries.len() >= MAX_ENTRIES {
            let _ = state.entries.pop_front();
        }
        state.entries.push_back(NetworkEntry {
            kind,
            host: host_only(url),
            time_ms,
            outcome,
            bytes,
        });
        state.seq = state.seq.saturating_add(1);
        Some(NetworkSnapshot {
            seq: state.seq,
            store_build: false,
            entries: state.entries.iter().cloned().collect(),
        })
    }

    /// Records an outgoing request to `url` (storing only its host) and notifies every window.
    pub(crate) fn record(&self, kind: RequestKind, url: &str, outcome: RequestOutcome, bytes: u64) {
        let Some(snapshot) = self.push(kind, url, outcome, bytes, now_ms()) else {
            return;
        };
        if let Some(app) = self.app.as_ref()
            && let Err(error) = app.emit(CHANGED_EVENT, snapshot)
        {
            tracing::debug!("could not broadcast network activity: {error}");
        }
    }
}

/// Returns the session's outgoing network activity log.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "tauri::command extracts State by value")]
pub(crate) fn network_activity(log: tauri::State<'_, Arc<NetworkLog>>) -> NetworkSnapshot {
    log.snapshot()
}

/// Extracts only the host name from `url`, dropping scheme, credentials, port, path, query and fragment.
fn host_only(url: &str) -> String {
    tauri::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn host_only_drops_path_query_credentials_and_fragments() {
        assert_eq!(
            host_only(
                "https://user:secret@updates.laminapdf.com:8443/v1/beta/darwin-aarch64?install_id=abc123&file=report.pdf#frag"
            ),
            "updates.laminapdf.com"
        );
        assert_eq!(host_only("http://127.0.0.1:4123/manifest.json"), "127.0.0.1");
        assert_eq!(host_only("not a url"), "unknown");
    }

    #[test]
    fn direct_log_records_only_allowed_fields_and_increments_seq() {
        let log = NetworkLog::new(false);
        assert_eq!(log.snapshot().seq, 0);

        let first = log
            .push(
                RequestKind::UpdateCheck,
                "https://updates.laminapdf.com/desktop/beta/linux-x86_64/0.1.0?token=secret-id",
                RequestOutcome::Ok,
                0,
                1_700_000_000_000,
            )
            .expect("direct build pushes return a snapshot");
        assert_eq!(first.seq, 1);

        let second = log
            .push(
                RequestKind::CrashReport,
                "https://updates.laminapdf.com/crash?doc=/Users/alice/secret.pdf",
                RequestOutcome::Ok,
                4096,
                1_700_000_005_000,
            )
            .expect("direct build pushes return a snapshot");
        assert_eq!(second.seq, 2);

        let snap = log.snapshot();
        assert!(!snap.store_build);
        assert_eq!(snap.seq, 2);
        assert_eq!(snap.entries.len(), 2);
        assert_eq!(snap.entries[0].kind, RequestKind::UpdateCheck);
        assert_eq!(snap.entries[0].host, "updates.laminapdf.com");
        assert_eq!(snap.entries[0].bytes, 0);
        assert_eq!(snap.entries[1].kind, RequestKind::CrashReport);
        assert_eq!(snap.entries[1].host, "updates.laminapdf.com");
        assert_eq!(snap.entries[1].bytes, 4096);

        let json = serde_json::to_string(&snap).expect("snapshot serialises");
        assert!(json.contains("updates.laminapdf.com"));
        assert!(!json.contains("secret"));
        assert!(!json.contains("token"));
        assert!(!json.contains(".pdf"));
        assert!(!json.contains("desktop/beta"));
        assert!(!json.contains("alice"));
    }

    #[test]
    fn store_log_stays_empty_and_ignores_pushes() {
        let log = NetworkLog::new(true);
        assert!(
            log.push(
                RequestKind::UpdateCheck,
                "https://updates.laminapdf.com/check",
                RequestOutcome::Ok,
                0,
                1_700_000_000_000,
            )
            .is_none()
        );
        let snap = log.snapshot();
        assert!(snap.store_build);
        assert_eq!(snap.seq, 0);
        assert_eq!(snap.entries, Vec::new());
    }

    #[test]
    fn default_log_matches_compile_time_app_store_flag() {
        let snap = NetworkLog::default().snapshot();
        assert_eq!(snap.store_build, crate::update::policy::APP_STORE);
    }

    #[test]
    fn log_caps_entries_dropping_oldest_first() {
        let log = NetworkLog::new(false);
        for index in 0..(MAX_ENTRIES + 5) {
            let _ = log.push(
                RequestKind::UpdateCheck,
                "https://updates.laminapdf.com/check",
                RequestOutcome::Ok,
                u64::try_from(index).unwrap_or(0),
                u64::try_from(index).unwrap_or(0),
            );
        }
        let snap = log.snapshot();
        assert_eq!(snap.entries.len(), MAX_ENTRIES);
        assert_eq!(snap.seq, u64::try_from(MAX_ENTRIES + 5).unwrap_or(0));
        assert_eq!(snap.entries.first().map(|e| e.time_ms), Some(5));
        assert_eq!(
            snap.entries.last().map(|e| e.time_ms),
            Some(u64::try_from(MAX_ENTRIES + 4).unwrap_or(0))
        );
    }

    #[test]
    fn outgoing_http_clients_only_live_in_logged_modules() {
        fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("read src-tauri/src directory") {
                let path = entry.expect("read directory entry").path();
                if path.is_dir() {
                    collect(&path, out);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }

        const FORBIDDEN: &[&str] = &[
            "reqwest::",
            "updater_builder",
            ".updater(",
            "UpdaterExt",
            "tauri_plugin_http",
            "hyper::",
            "ureq::",
            "TcpStream",
            "TcpSocket",
            "UdpSocket",
        ];

        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect(&src, &mut files);
        assert!(!files.is_empty(), "expected Rust source files under {}", src.display());

        let mut offenders = Vec::new();
        let mut saw_logged = 0usize;
        for path in files {
            let rel = path
                .strip_prefix(&src)
                .expect("file under src")
                .to_string_lossy()
                .replace('\\', "/");
            let source = std::fs::read_to_string(&path).expect("read Rust source file");
            if matches!(rel.as_str(), "crash/upload.rs" | "update/mod.rs") {
                assert!(
                    source.contains(".record("),
                    "logged outgoing module {rel} must still record to NetworkLog"
                );
                saw_logged += 1;
                continue;
            }
            if rel == "network.rs" {
                continue;
            }
            if let Some(hit) = FORBIDDEN.iter().find(|pattern| source.contains(**pattern)) {
                offenders.push(format!("{rel} ({hit})"));
            }
        }
        assert_eq!(saw_logged, 2, "expected both crash/upload.rs and update/mod.rs");
        assert!(
            offenders.is_empty(),
            "new outgoing HTTP code must record to NetworkLog: {offenders:?}"
        );
    }
}
