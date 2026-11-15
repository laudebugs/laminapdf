//! The update test's hooks (D5c), in end-to-end builds only (feature `e2e`). When
//! `LAMINA_E2E_UPDATE_SERVER` names a server, the app follows the `staging` channel there, checks
//! at once instead of after 10 s, and installs an update and restarts as soon as one is ready, as
//! Restart to update does. Each step is written to the file `LAMINA_E2E_UPDATE_REPORT` names, one
//! line each, which `e2e/update/update-test.ts` reads: `started <version>`, `no update`,
//! `ready <version>` and `failed: <error>`.
//!
//! The server may be plain HTTP only because the test passes the updater's
//! `dangerousInsecureTransportProtocol` with `--config`; no committed configuration sets it, and
//! release builds have none of this (CI checks their binary for `LAMINA_E2E_UPDATE`).

use std::io::Write as _;

use super::plan;

/// The update server the test serves, such as `http://127.0.0.1:8790`.
const SERVER_ENV: &str = "LAMINA_E2E_UPDATE_SERVER";

/// The file the test reads the app's steps from.
const REPORT_ENV: &str = "LAMINA_E2E_UPDATE_REPORT";

/// The channel the test serves: the Worker's, which the app never offers.
const CHANNEL: &str = "staging";

/// The test's server, when this run is the update test.
pub(crate) fn server() -> Option<String> {
    std::env::var(SERVER_ENV).ok().filter(|server| !server.is_empty())
}

/// The endpoint the test's checks go to, when this run is the update test.
pub(crate) fn endpoint() -> Option<String> {
    server().map(|server| plan::endpoint_of(&server, CHANNEL))
}

/// Writes `line` to the test's report, when there is one.
pub(crate) fn report(line: &str) {
    let Some(path) = std::env::var_os(REPORT_ENV) else {
        return;
    };
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        tracing::warn!("could not write the update test's report: {error}");
    }
}

/// Installs the update that is ready and restarts, on the main thread, as Restart to update does.
pub(crate) fn install_now(app: &tauri::AppHandle) {
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
