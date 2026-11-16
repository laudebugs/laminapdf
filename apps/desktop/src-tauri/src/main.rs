//! The `laminapdf-desktop` binary: runs the shell and exits non-zero when it can't start. Started
//! with `--crash-monitor`, it is instead the main process's crash monitor (D7d).

// The release build is a GUI app: no console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    // The main process's crash monitor (D7d) is this binary too, started with its own flag.
    if let Some(code) = laminapdf_desktop_lib::monitor_main(std::env::args_os().skip(1)) {
        return code;
    }
    match laminapdf_desktop_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // The log plugin may not be up yet when the app fails to start.
            eprintln!("Lamina PDF could not start: {error}");
            ExitCode::FAILURE
        }
    }
}
