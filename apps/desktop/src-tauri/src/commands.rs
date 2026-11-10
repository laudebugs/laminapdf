//! The app commands the webview may call, each allowed by name in `capabilities/main.json`.

use serde::Serialize;

/// What `app_info` reports: enough for an About box and a bug report, nothing about documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// The app's version (the workspace's lockstep version, ADR 0017).
    pub version: String,
    /// `macos`, `windows` or `linux`.
    pub os: &'static str,
    /// `aarch64`, `x86_64` …
    pub arch: &'static str,
    /// The system webview's version, when the platform reports it.
    pub webview_version: Option<String>,
}

impl AppInfo {
    /// This build on this machine.
    #[must_use]
    pub fn current(version: &str) -> Self {
        Self {
            version: version.to_owned(),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            webview_version: tauri::webview_version().ok(),
        }
    }
}

/// The app's version, OS, CPU and webview version: proves the IPC and permission plumbing.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its AppHandle by value"
)]
pub(crate) fn app_info(app: tauri::AppHandle) -> AppInfo {
    AppInfo::current(&app.package_info().version.to_string())
}

#[cfg(test)]
mod tests {
    use super::AppInfo;
    use crate::DesktopError;

    #[test]
    fn app_info_should_serialise_with_camel_case_keys() {
        let info = AppInfo {
            version: "1.2.3".into(),
            os: "macos",
            arch: "aarch64",
            webview_version: Some("1".into()),
        };
        let json = serde_json::to_value(&info).unwrap_or_default();
        assert_eq!(
            json,
            serde_json::json!({ "version": "1.2.3", "os": "macos", "arch": "aarch64", "webviewVersion": "1" })
        );
    }

    #[test]
    fn desktop_error_should_serialise_as_kind_and_message() {
        let error = DesktopError::Tauri(tauri::Error::WebviewNotFound);
        let json = serde_json::to_value(&error).unwrap_or_default();
        assert_eq!(json["kind"], "tauri");
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|m| m.starts_with("the desktop shell failed"))
        );
    }
}
