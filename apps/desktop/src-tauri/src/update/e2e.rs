//! The update test's hooks, in end-to-end builds only (feature `e2e`). When the app's config
//! folder holds `update-test.json`, `{ "server": "http://127.0.0.1:…", "report": "<file>" }`, the
//! app follows the `staging` channel at that server, checks at once instead of after 10 s, and
//! installs an update and restarts as soon as one is ready, as Restart to update does. Each step is
//! written to the report file, one line each, which `e2e/update/update-test.ts` reads:
//! `started <version>`, `no update`, `ready <version>` and `failed: <error>`.
//!
//! A file, not environment variables: on Windows the NSIS installer relaunches the updated app
//! through the desktop shell, as the signed-in user, so the new version doesn't inherit the old
//! one's environment.
//!
//! The server may be plain HTTP only because the test passes the updater's
//! `dangerousInsecureTransportProtocol` with `--config`; no committed configuration sets it, and
//! release builds have none of this (CI checks their binary for `update-test.json`): they compile
//! `e2e_off.rs` instead, whose hooks do nothing.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use std::time::Duration;

use super::plan::{self, FIRST_CHECK};

/// The file in the config folder that turns the test on.
const FILE: &str = "update-test.json";

/// The channel the test serves: the Worker's, which the app never offers.
const CHANNEL: &str = "staging";

/// The test's settings, as `update-test.json` holds them.
#[derive(Debug, Deserialize)]
struct UpdateTest {
    /// The update server the test serves, such as `http://127.0.0.1:8790`.
    server: String,
    /// The file the test reads the app's steps from.
    report: PathBuf,
}

/// The test's settings, read once when the app starts; `None` when this run isn't the test.
static SETTINGS: OnceLock<Option<UpdateTest>> = OnceLock::new();

/// Reads `update-test.json` from the app's config folder, `config`.
pub(super) fn load(config: &Path) {
    SETTINGS.get_or_init(|| {
        let text = std::fs::read_to_string(config.join(FILE)).ok()?;
        serde_json::from_str(&text)
            .inspect_err(|error| tracing::warn!("could not read {FILE}: {error}"))
            .ok()
    });
}

fn settings() -> Option<&'static UpdateTest> {
    SETTINGS.get().and_then(Option::as_ref)
}

/// The check to make: `request`, sent to the test's server on its channel when this run is the test.
pub(super) fn request(request: plan::CheckRequest) -> plan::CheckRequest {
    match settings() {
        Some(test) => plan::CheckRequest {
            url: plan::endpoint_of(&test.server, CHANNEL),
            ..request
        },
        None => request,
    }
}

/// The wait before the first check: none in the test, which reports the version that started.
pub(super) fn first_check(version: &str) -> Duration {
    if settings().is_none() {
        return FIRST_CHECK;
    }
    report(&format!("started {version}"));
    Duration::ZERO
}

/// Reports how a check ended; `true` when an update is ready and the test installs it at once.
pub(super) fn checked(outcome: &Result<Option<super::Ready>, tauri_plugin_updater::Error>) -> bool {
    match outcome {
        Ok(Some(ready)) => report(&format!("ready {}", ready.update.version)),
        Ok(None) => report("no update"),
        Err(error) => report(&format!("failed: {error}")),
    }
    matches!(outcome, Ok(Some(_))) && settings().is_some()
}

/// Writes `line` to the test's report, when this run is the update test.
fn report(line: &str) {
    let Some(test) = settings() else {
        return;
    };
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&test.report)
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        tracing::warn!("could not write the update test's report: {error}");
    }
}

/// Installs the update that is ready and restarts, on the main thread, as Restart to update does.
pub(super) fn install_now(app: &tauri::AppHandle) {
    let handle = app.clone();
    let queued = app.run_on_main_thread(move || {
        if let Err(error) = super::commands::update_restart_to_install(handle) {
            report(&format!("failed: {error}"));
        }
    });
    if let Err(error) = queued {
        report(&format!("failed: {error}"));
    }
}
