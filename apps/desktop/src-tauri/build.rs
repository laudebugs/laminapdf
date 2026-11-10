//! Tauri's build step. App commands go through an app manifest, so each needs an explicit
//! `allow-*` permission in a capability file before the webview may call it (ADR 0008 §6).

use std::process::ExitCode;

/// Every command a release build has.
const COMMANDS: &[&str] = &["app_info", "open_dialog", "page_info", "close_document"];

/// With the `e2e` feature: the end-to-end tests' commands too.
const E2E_COMMANDS: &[&str] = &[
    "app_info",
    "open_dialog",
    "page_info",
    "close_document",
    "debug_kill_engine",
];

fn main() -> ExitCode {
    let commands = if std::env::var_os("CARGO_FEATURE_E2E").is_some() {
        E2E_COMMANDS
    } else {
        COMMANDS
    };
    let manifest = tauri_build::AppManifest::new().commands(commands);
    match tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // A build script's stderr shows when it fails; `cargo:warning` keeps the reason visible.
            println!("cargo:warning=tauri-build failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}
