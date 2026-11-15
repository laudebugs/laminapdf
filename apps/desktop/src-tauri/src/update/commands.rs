//! The update commands the webview may call, each allowed by name in `capabilities/main.json`. They
//! read and change the user's update settings and start checks; none takes a URL, a header or the
//! install ID, so the webview can't point the updater anywhere or learn the ID.

use std::sync::Arc;

use serde::Deserialize;
use tauri::{Manager as _, State};
use tauri_plugin_opener::OpenerExt as _;

use super::policy::STABLE_AVAILABLE;
use super::settings::Channel;
use super::{UpdateStatus, Updates};
use crate::DesktopError;

/// The update status.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its state by value")]
pub(crate) fn update_status(updates: State<'_, Arc<Updates>>) -> UpdateStatus {
    updates.status()
}

/// Check now: checks and downloads an update, and reports a failure in the status.
#[tauri::command]
pub(crate) async fn update_check_now(app: tauri::AppHandle) -> Result<UpdateStatus, DesktopError> {
    let updates = Arc::clone(&app.state::<Arc<Updates>>());
    if !updates.applied().can_check() {
        return Err(DesktopError::UpdatesManaged);
    }
    updates.check(&app, true).await;
    Ok(updates.status())
}

/// Turns automatic updates on or off.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its AppHandle by value"
)]
pub(crate) fn update_set_enabled(app: tauri::AppHandle, enabled: bool) -> Result<UpdateStatus, DesktopError> {
    let updates = app.state::<Arc<Updates>>();
    if !updates.user_decides() {
        return Err(DesktopError::UpdatesManaged);
    }
    updates.store.update(|settings| settings.enabled = enabled);
    updates.announce(&app);
    Ok(updates.status())
}

/// Switches between Stable and Beta. A nightly build can't switch (it trusts only the nightly key),
/// and Stable can't be chosen before the first stable release.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its AppHandle by value"
)]
pub(crate) fn update_set_channel(app: tauri::AppHandle, channel: Channel) -> Result<UpdateStatus, DesktopError> {
    let updates = app.state::<Arc<Updates>>();
    if !updates.user_decides() {
        return Err(DesktopError::UpdatesManaged);
    }
    let allowed = match channel {
        Channel::Beta => !updates.status().nightly,
        Channel::Stable => STABLE_AVAILABLE && !updates.status().nightly,
        Channel::Nightly => false,
    };
    if !allowed {
        return Err(DesktopError::ChannelUnavailable);
    }
    updates.store.update(|settings| settings.channel = Some(channel));
    updates.announce(&app);
    Ok(updates.status())
}

/// Replaces the anonymous install ID; the next check sends the new one.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its AppHandle by value"
)]
pub(crate) fn update_reset_install_id(app: tauri::AppHandle) -> UpdateStatus {
    let updates = app.state::<Arc<Updates>>();
    updates.store.reset_install_id();
    updates.status()
}

/// Restart to update: saves the session, installs the waiting update and starts the new version.
/// On Windows the installer exits the app and starts it again itself.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its AppHandle by value"
)]
pub(crate) fn update_restart_to_install(app: tauri::AppHandle) -> Result<(), DesktopError> {
    let updates = Arc::clone(&app.state::<Arc<Updates>>());
    if !updates.has_ready() {
        return Err(DesktopError::UpdateNotReady);
    }
    // The windows that close now stay in the session, which is written before the install.
    crate::session::app::quitting(&app);
    updates.install_on_quit();
    app.restart()
}

/// A page about updates, which Rust opens in the browser: the webview names it, never its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UpdateLink {
    /// What the update server receives and keeps (ADR 0014 §9).
    Privacy,
    /// The release notes of the update that is ready.
    WhatsNew,
}

/// The privacy page, which says what an update check sends.
const PRIVACY_PAGE: &str = "https://laminapdf.com/privacy/";

/// The release notes of `version`.
fn release_notes(version: &str) -> String {
    format!("https://github.com/laudebugs/laminapdf/releases/tag/v{version}")
}

/// Opens `link` in the browser. What's new needs a ready update, whose version names the notes.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its AppHandle by value"
)]
pub(crate) fn update_open_link(app: tauri::AppHandle, link: UpdateLink) -> Result<(), DesktopError> {
    let url = match link {
        UpdateLink::Privacy => PRIVACY_PAGE.to_owned(),
        UpdateLink::WhatsNew => match app.state::<Arc<Updates>>().status().state {
            super::State::Ready { version, .. } => release_notes(&version),
            _ => return Err(DesktopError::UpdateNotReady),
        },
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| DesktopError::Internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_release_notes_should_be_the_versions_tag() {
        assert_eq!(
            release_notes("0.2.0"),
            "https://github.com/laudebugs/laminapdf/releases/tag/v0.2.0"
        );
    }

    #[test]
    fn links_should_be_named_never_given_as_addresses() {
        let link: UpdateLink = serde_json::from_str(r#""whats-new""#).unwrap();
        assert_eq!(link, UpdateLink::WhatsNew);
        assert!(serde_json::from_str::<UpdateLink>(r#""https://example.com/""#).is_err());
    }
}
