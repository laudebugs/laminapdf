//! In-memory log of outgoing network requests made by the main process (updater and crash
//! reporter). The webview's own traffic stays on local app schemes; this log lets Privacy ›
//! Network activity show every request that actually left the machine, with only its host, kind,
//! time, byte count and outcome — never a URL path, query string, file name or install ID.
//!
//! Nothing is written to disk. In the Mac App Store build (`app-store` feature), the app's own
//! code makes no network request at all (ADR 0014 §11), so the log stays empty.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::Emitter as _;

/// Event broadcast to every window when the session's network activity log changes.
pub(crate) const CHANGED_EVENT: &str = "network://changed";

/// Whether this build is the Mac App Store build, whose own code makes no outgoing request.
pub(crate) const STORE_BUILD: bool = cfg!(feature = "app-store");

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
    /// True in the Mac App Store build, which makes no outgoing requests.
    pub store_build: bool,
    /// Outgoing requests recorded during this session, oldest first.
    pub entries: Vec<NetworkEntry>,
}

/// Session-only, in-memory log of outgoing main-process requests.
#[derive(Default)]
pub(crate) struct NetworkLog {
    entries: Mutex<VecDeque<NetworkEntry>>,
    app: OnceLock<tauri::AppHandle>,
}

impl NetworkLog {
    /// A log that broadcasts [`CHANGED_EVENT`] to every window of `app` whenever an entry is added.
    pub(crate) fn with_app(app: tauri::AppHandle) -> Self {
        let handle = OnceLock::new();
        let _ = handle.set(app);
        Self {
            entries: Mutex::new(VecDeque::new()),
            app: handle,
        }
    }

    /// Current snapshot of this session's outgoing requests.
    pub(crate) fn snapshot(&self) -> NetworkSnapshot {
        if STORE_BUILD {
            return NetworkSnapshot {
                store_build: true,
                entries: Vec::new(),
            };
        }
        let entries = lock(&self.entries).iter().cloned().collect();
        NetworkSnapshot {
            store_build: false,
            entries,
        }
    }

    /// Appends a sanitised entry for `url` (host only) without broadcasting.
    fn push(&self, kind: RequestKind, url: &str, outcome: RequestOutcome, bytes: u64, time_ms: u64) {
        if STORE_BUILD {
            return;
        }
        let mut entries = lock(&self.entries);
        if entries.len() >= MAX_ENTRIES {
            let _ = entries.pop_front();
        }
        entries.push_back(NetworkEntry {
            kind,
            host: host_only(url),
            time_ms,
            outcome,
            bytes,
        });
    }

    /// Records an outgoing request to `url` (storing only its host) and notifies every window.
    pub(crate) fn record(&self, kind: RequestKind, url: &str, outcome: RequestOutcome, bytes: u64) {
        if STORE_BUILD {
            return;
        }
        self.push(kind, url, outcome, bytes, now_ms());
        if let Some(app) = self.app.get()
            && let Err(error) = app.emit(CHANGED_EVENT, self.snapshot())
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
pub(crate) fn host_only(url: &str) -> String {
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
    fn log_records_only_allowed_fields_and_respects_store_build() {
        let log = NetworkLog::default();
        log.push(
            RequestKind::UpdateCheck,
            "https://updates.laminapdf.com/desktop/beta/linux-x86_64/0.1.0?token=secret-id",
            RequestOutcome::Ok,
            0,
            1_700_000_000_000,
        );
        log.push(
            RequestKind::CrashReport,
            "https://updates.laminapdf.com/crash?doc=/Users/alice/secret.pdf",
            RequestOutcome::Ok,
            4096,
            1_700_000_005_000,
        );

        let snap = log.snapshot();
        assert_eq!(snap.store_build, STORE_BUILD);
        if STORE_BUILD {
            assert_eq!(snap.entries, Vec::new());
            return;
        }
        assert_eq!(snap.entries.len(), 2);
        assert_eq!(snap.entries[0].kind, RequestKind::UpdateCheck);
        assert_eq!(snap.entries[0].host, "updates.laminapdf.com");
        assert_eq!(snap.entries[0].bytes, 0);
        assert_eq!(snap.entries[1].kind, RequestKind::CrashReport);
        assert_eq!(snap.entries[1].host, "updates.laminapdf.com");
        assert_eq!(snap.entries[1].bytes, 4096);

        let json = serde_json::to_string(&snap).unwrap_or_default();
        assert!(!json.contains("secret"));
        assert!(!json.contains("token"));
        assert!(!json.contains(".pdf"));
        assert!(!json.contains("desktop/beta"));
        assert!(!json.contains("alice"));
    }

    #[test]
    fn log_caps_entries_dropping_oldest_first() {
        let log = NetworkLog::default();
        for index in 0..(MAX_ENTRIES + 5) {
            log.push(
                RequestKind::UpdateCheck,
                "https://updates.laminapdf.com/check",
                RequestOutcome::Ok,
                u64::try_from(index).unwrap_or(0),
                u64::try_from(index).unwrap_or(0),
            );
        }
        let snap = log.snapshot();
        if STORE_BUILD {
            assert_eq!(snap.entries, Vec::new());
            return;
        }
        assert_eq!(snap.entries.len(), MAX_ENTRIES);
        assert_eq!(snap.entries.first().map(|e| e.time_ms), Some(5));
        assert_eq!(
            snap.entries.last().map(|e| e.time_ms),
            Some(u64::try_from(MAX_ENTRIES + 4).unwrap_or(0))
        );
    }

    #[test]
    fn outgoing_http_clients_only_live_in_logged_modules() {
        fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect(&path, out);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }

        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect(&src, &mut files);
        let mut offenders = Vec::new();
        for path in files {
            let rel = path
                .strip_prefix(&src)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if matches!(rel.as_str(), "crash/upload.rs" | "update/mod.rs" | "network.rs") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            if source.contains("reqwest::")
                || source.contains("updater_builder")
                || source.contains("TcpStream")
                || source.contains("UdpSocket")
            {
                offenders.push(rel);
            }
        }
        assert!(
            offenders.is_empty(),
            "new outgoing HTTP code must record to NetworkLog: {offenders:?}"
        );
    }
}
