//! Tauri's build step. App commands go through an app manifest, so each needs an explicit
//! `allow-*` permission in a capability file before the webview may call it (ADR 0008 §6).

use std::process::ExitCode;

fn main() -> ExitCode {
    let manifest = tauri_build::AppManifest::new().commands(&["app_info"]);
    match tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // A build script's stderr shows when it fails; `cargo:warning` keeps the reason visible.
            println!("cargo:warning=tauri-build failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}
