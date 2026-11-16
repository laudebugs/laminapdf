//! Whether the last run quit unexpectedly: `running`, an empty file in the app's config folder,
//! written as the app starts and removed when it quits normally. A panic, a native crash, a kill or
//! a power cut leaves it behind, so the next launch knows, whatever made a report or none.
//!
//! It is written first in `setup`, so only a crash before that (the builder, the plugins) looks
//! like a normal launch. A second launch hands its files to the running app before `setup` and
//! never touches it.

use std::io;
use std::path::Path;

/// The marker's name in the app's config folder.
const FILE: &str = "running";

/// Marks this run as running: `true` when the last run left its mark, so didn't quit normally.
pub(crate) fn launched(config: &Path) -> bool {
    let file = config.join(FILE);
    let left = file.exists();
    if let Err(error) = std::fs::create_dir_all(config).and_then(|()| std::fs::write(&file, b"")) {
        tracing::warn!("could not mark the app as running: {error}");
    }
    left
}

/// The app is quitting normally, or restarting: the next launch isn't after a crash.
pub(crate) fn quit_normally(config: &Path) {
    match std::fs::remove_file(config.join(FILE)) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!("could not clear the running mark: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-marker-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_first_launch_should_not_be_after_a_crash() {
        assert!(!launched(&dir("first")));
    }

    #[test]
    fn a_launch_after_a_run_that_never_quit_should_be_after_a_crash() {
        let dir = dir("crash");
        launched(&dir);
        assert!(launched(&dir));
    }

    #[test]
    fn a_launch_after_a_normal_quit_should_not_be_after_a_crash() {
        let dir = dir("quit");
        launched(&dir);
        quit_normally(&dir);
        assert!(!launched(&dir));
    }
}
