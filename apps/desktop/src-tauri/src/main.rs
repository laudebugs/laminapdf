//! The `laminapdf-desktop` binary: runs the shell and exits non-zero when it can't start.

// The release build is a GUI app: no console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match laminapdf_desktop_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // The log plugin may not be up yet when the app fails to start.
            eprintln!("Lamina PDF could not start: {error}");
            ExitCode::FAILURE
        }
    }
}
