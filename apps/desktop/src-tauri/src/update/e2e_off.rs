//! The update test's hooks in every build without the `e2e` feature: they do nothing, so a release
//! build checks 10 s after start, asks the channel's real endpoint and installs only on quit or on
//! Restart to update. `e2e.rs` has the test's.

use std::path::Path;
use std::time::Duration;

use super::plan::{self, FIRST_CHECK};

/// Nothing to read.
pub(super) const fn load(_config: &Path) {}

/// The check as planned.
pub(super) const fn request(request: plan::CheckRequest) -> plan::CheckRequest {
    request
}

/// The usual wait before the first check.
pub(super) const fn first_check(_version: &str) -> Duration {
    FIRST_CHECK
}

/// Nothing to report, and nothing installs at once.
pub(super) const fn checked<T, E>(_outcome: &Result<T, E>) -> bool {
    false
}

/// Never called: `checked` never asks for it.
pub(super) const fn install_now(_app: &tauri::AppHandle) {}
