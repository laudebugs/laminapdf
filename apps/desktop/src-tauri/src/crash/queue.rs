//! Where reports wait, as JSON files in the app's data folder (D7a): `held/` for those the user
//! hasn't let go (shown when the app asks; never sent), `outbox/` for those they have, until the
//! update Worker takes them. Each keeps at most [`MAX_KEPT`] reports, the oldest dropped first, and
//! none older than [`MAX_AGE`].

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::report::Report;
use crate::store::{read_json, write_json};

/// The most reports each folder keeps.
pub(crate) const MAX_KEPT: usize = 10;

/// How long a report is kept, sent or not.
pub(crate) const MAX_AGE: Duration = Duration::from_hours(30 * 24);

/// Which folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shelf {
    /// Not to be sent unless the user says so.
    Held,
    /// To be sent.
    Outbox,
}

impl Shelf {
    const fn folder(self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Outbox => "outbox",
        }
    }
}

/// The reports under one folder (the app's `crashes/`).
#[derive(Debug, Clone)]
pub(crate) struct Queue {
    root: PathBuf,
}

impl Queue {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn dir(&self, place: Shelf) -> PathBuf {
        self.root.join(place.folder())
    }

    /// Where `report` goes on `place`. Its ID is a UUID the app made, so it is a safe file name.
    fn path(&self, place: Shelf, id: &str) -> Option<PathBuf> {
        let safe = !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-');
        safe.then(|| self.dir(place).join(format!("{id}.json")))
    }

    /// Keeps `report` on `place`, then drops what is past the limits.
    pub(crate) fn put(&self, place: Shelf, report: &Report) -> io::Result<()> {
        let path = self
            .path(place, &report.report_id)
            .ok_or_else(|| io::Error::other("a report ID that isn't a UUID"))?;
        write_json(&path, report)?;
        self.prune(place, SystemTime::now());
        Ok(())
    }

    /// The reports on `place`, newest first.
    pub(crate) fn list(&self, place: Shelf) -> Vec<Report> {
        files(&self.dir(place))
            .into_iter()
            .filter_map(|(path, _)| read_json(&path, "crash report"))
            .collect()
    }

    /// Takes report `id` off `place`.
    pub(crate) fn take(&self, place: Shelf, id: &str) -> Option<Report> {
        let path = self.path(place, id)?;
        let report = read_json(&path, "crash report");
        let _ = fs::remove_file(&path);
        report
    }

    /// Moves report `id` from `held/` to `outbox/`; `false` when it isn't held.
    pub(crate) fn release(&self, id: &str) -> bool {
        self.take(Shelf::Held, id)
            .is_some_and(|report| self.put(Shelf::Outbox, &report).is_ok())
    }

    /// Drops every report on `place`.
    pub(crate) fn clear(&self, place: Shelf) {
        let _ = fs::remove_dir_all(self.dir(place));
    }

    /// Drops the reports on `place` older than [`MAX_AGE`] at `now` or past the newest [`MAX_KEPT`].
    pub(crate) fn prune(&self, place: Shelf, now: SystemTime) {
        for (index, (path, modified)) in files(&self.dir(place)).into_iter().enumerate() {
            let old = now.duration_since(modified).is_ok_and(|age| age > MAX_AGE);
            if old || index >= MAX_KEPT {
                let _ = fs::remove_file(path);
            }
        }
    }
}

/// The `.json` files in `dir` with their modification times, newest first.
fn files(dir: &Path) -> Vec<(PathBuf, SystemTime)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            let modified = fs::metadata(&path).and_then(|m| m.modified()).ok()?;
            Some((path, modified))
        })
        .collect();
    files.sort_by_key(|file| std::cmp::Reverse(file.1));
    files
}

#[cfg(test)]
mod tests {
    use super::super::report::{AppFacts, Crash, ReportChannel};
    use super::*;

    fn queue(name: &str) -> Queue {
        let root = std::env::temp_dir().join(format!("laminapdf-crash-queue-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Queue::new(root)
    }

    fn report() -> Report {
        let facts = AppFacts {
            app_version: "0.1.0".into(),
            channel: ReportChannel::Dev,
            os_version: "1".into(),
        };
        Crash::engine_exit("killed".into(), None).report(&facts, Duration::ZERO)
    }

    #[test]
    fn a_released_report_should_move_to_the_outbox() {
        let queue = queue("release");
        let report = report();
        queue.put(Shelf::Held, &report).unwrap();
        let moved = queue.release(&report.report_id);
        assert_eq!(
            (moved, queue.list(Shelf::Held).len(), queue.list(Shelf::Outbox)),
            (true, 0, vec![report])
        );
    }

    #[test]
    fn only_the_newest_reports_should_be_kept() {
        let queue = queue("limit");
        for _ in 0..MAX_KEPT + 3 {
            queue.put(Shelf::Outbox, &report()).unwrap();
        }
        assert_eq!(queue.list(Shelf::Outbox).len(), MAX_KEPT);
    }

    #[test]
    fn reports_past_their_age_should_be_dropped() {
        let queue = queue("age");
        queue.put(Shelf::Held, &report()).unwrap();
        queue.prune(Shelf::Held, SystemTime::now() + MAX_AGE + Duration::from_secs(60));
        assert_eq!(queue.list(Shelf::Held), Vec::<Report>::new());
    }

    #[test]
    fn an_id_that_isnt_a_uuid_should_not_name_a_file() {
        assert!(queue("ids").take(Shelf::Held, "../../preferences").is_none());
    }
}
