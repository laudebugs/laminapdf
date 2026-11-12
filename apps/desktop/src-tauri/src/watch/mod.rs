//! Watching open documents' files, so one that another app changes reloads (ADR 0008 §11).
//!
//! One `notify` watcher watches the **folder** of each open file, not the file: an atomic save
//! (vim, VS Code, most editors) writes a temporary file and renames it over the original, which
//! leaves a file watch on the old, unlinked inode. Events are matched to files by path, and the
//! file's identity is part of its [`FileStamp`] only to notice a replacement, never to follow the
//! old file. After an event, a file waits [`DEBOUNCE`] for quiet, then is looked at ([`Settle`]):
//! still changing, it is looked at again one period later; settled, it reloads; missing, its
//! windows are told, and it reloads if it comes back.
//!
//! One thread owns the state and the OS watcher: commands and the OS's events arrive on one
//! channel, and reloads run on that thread, one at a time. [`Reload`] is how it reaches the app,
//! so the tests drive it with a fake.

pub(crate) mod app;
mod stamp;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::DocId;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
pub(crate) use stamp::FileStamp;
use stamp::{Change, Settle, Verdict};

/// How long a file must be quiet after an event before it is looked at.
pub(crate) const DEBOUNCE: Duration = Duration::from_millis(300);

/// What a reload came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reloaded {
    /// The document reopened from the changed file under a new id; the old one is closed.
    Replaced(DocId),
    /// The changed file couldn't be read or opened; the old view stays, and the next change retries.
    Failed,
    /// Reloading is turned off: the change is ignored.
    Skipped,
}

/// How the watcher reaches the app: a document's file changed or went.
pub(crate) trait Reload: Send + Sync {
    /// The file `path` behind `window`'s document `doc` changed and has settled: reopen it.
    fn modified(&self, window: &str, doc: DocId, path: &Path) -> Reloaded;
    /// The file behind `window`'s document `doc` is gone: deleted or moved away.
    fn missing(&self, window: &str, doc: DocId);
}

enum Command {
    Watch { path: PathBuf, window: String, doc: DocId },
    Unwatch { window: String, doc: DocId },
    UnwatchWindow(String),
    Event(notify::Result<notify::Event>),
}

/// The watcher's handle: commands go to its thread, which stops when every handle is gone.
pub(crate) struct Watcher {
    tx: Sender<Command>,
}

impl Watcher {
    /// Starts the watcher's thread and the OS watcher.
    ///
    /// # Errors
    /// The OS's error when its watcher or the thread can't start.
    pub(crate) fn start(reload: Arc<dyn Reload>) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let events = tx.clone();
        let os = notify::recommended_watcher(move |event| {
            // The watcher's thread is gone only when the app is exiting.
            let _ = events.send(Command::Event(event));
        })
        .map_err(std::io::Error::other)?;
        let worker = Worker {
            files: BTreeMap::new(),
            dirs: BTreeMap::new(),
            os,
            reload,
        };
        thread::Builder::new()
            .name("file-watcher".into())
            .spawn(move || worker.run(&rx))?;
        Ok(Self { tx })
    }

    /// Watches `path`, the file behind `window`'s document `doc`.
    pub(crate) fn watch(&self, path: &Path, window: &str, doc: DocId) {
        self.send(Command::Watch {
            path: path.to_owned(),
            window: window.to_owned(),
            doc,
        });
    }

    /// Stops watching for `window`'s document `doc` (it closed).
    pub(crate) fn unwatch(&self, window: &str, doc: DocId) {
        self.send(Command::Unwatch {
            window: window.to_owned(),
            doc,
        });
    }

    /// Stops watching for every document of `window` (it closed).
    pub(crate) fn unwatch_window(&self, window: &str) {
        self.send(Command::UnwatchWindow(window.to_owned()));
    }

    fn send(&self, command: Command) {
        if self.tx.send(command).is_err() {
            tracing::warn!("the file watcher has stopped");
        }
    }
}

/// One watched file.
struct Watched {
    /// The file as its documents read it: when they opened, or last reloaded.
    opened: FileStamp,
    /// Every window's document that shows it.
    docs: Vec<(String, DocId)>,
    settle: Settle,
    /// When to look at it next, after an event.
    due: Option<Instant>,
    /// Its windows were told it is gone.
    missing: bool,
}

struct Worker {
    /// By canonical path.
    files: BTreeMap<PathBuf, Watched>,
    /// Each watched folder and how many watched files are in it.
    dirs: BTreeMap<PathBuf, usize>,
    os: RecommendedWatcher,
    reload: Arc<dyn Reload>,
}

impl Worker {
    fn run(mut self, rx: &Receiver<Command>) {
        loop {
            let next = self.files.values().filter_map(|file| file.due).min();
            let command = match next {
                Some(due) => match rx.recv_timeout(due.saturating_duration_since(Instant::now())) {
                    Ok(command) => Some(command),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                },
                None => match rx.recv() {
                    Ok(command) => Some(command),
                    Err(_) => return,
                },
            };
            if let Some(command) = command {
                self.handle(command);
            }
            self.look_at_due();
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Watch { path, window, doc } => self.watch(&path, window, doc),
            Command::Unwatch { window, doc } => self.unwatch(|w, d| w == window && d == doc),
            Command::UnwatchWindow(window) => self.unwatch(|w, _| w == window),
            Command::Event(Ok(event)) => {
                let due = Instant::now() + DEBOUNCE;
                let everything = event.paths.is_empty() || event.need_rescan();
                for (path, file) in &mut self.files {
                    if everything || event.paths.iter().any(|p| p == path) {
                        file.due = Some(due);
                    }
                }
            }
            Command::Event(Err(error)) => tracing::warn!("the file watcher reported an error: {error}"),
        }
    }

    fn watch(&mut self, path: &Path, window: String, doc: DocId) {
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
        if let Some(file) = self.files.get_mut(&path) {
            file.docs.push((window, doc));
            return;
        }
        let Ok(opened) = FileStamp::read(&path) else {
            tracing::debug!(path = %path.display(), "not watching a file that can't be read");
            return;
        };
        let Some(dir) = path.parent().map(Path::to_owned) else {
            return;
        };
        if !self.dirs.contains_key(&dir)
            && let Err(error) = self.os.watch(&dir, RecursiveMode::NonRecursive)
        {
            tracing::warn!("could not watch an open document's folder: {error}");
            return;
        }
        *self.dirs.entry(dir).or_insert(0) += 1;
        self.files.insert(
            path,
            Watched {
                opened,
                docs: vec![(window, doc)],
                settle: Settle::default(),
                due: None,
                missing: false,
            },
        );
    }

    fn unwatch(&mut self, gone: impl Fn(&str, DocId) -> bool) {
        let mut emptied = Vec::new();
        for (path, file) in &mut self.files {
            file.docs.retain(|(window, doc)| !gone(window, *doc));
            if file.docs.is_empty() {
                emptied.push(path.clone());
            }
        }
        for path in emptied {
            self.files.remove(&path);
            let Some(dir) = path.parent() else {
                continue;
            };
            if let Some(count) = self.dirs.get_mut(dir) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.dirs.remove(dir);
                    let _ = self.os.unwatch(dir);
                }
            }
        }
    }

    fn look_at_due(&mut self) {
        let now = Instant::now();
        let due: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|(_, file)| file.due.is_some_and(|due| due <= now))
            .map(|(path, _)| path.clone())
            .collect();
        for path in due {
            self.look_at(&path);
        }
    }

    fn look_at(&mut self, path: &Path) {
        let Some(file) = self.files.get_mut(path) else {
            return;
        };
        file.due = None;
        let stamp = FileStamp::read(path).ok();
        match file.settle.look(&file.opened, stamp.as_ref()) {
            Verdict::Wait => file.due = Some(Instant::now() + DEBOUNCE),
            // A file that came back as it was still needs its windows told it is there again.
            Verdict::Settled(Change::Unchanged) if file.missing => {
                if let Some(stamp) = stamp {
                    self.reload_all(path, stamp);
                }
            }
            Verdict::Settled(Change::Unchanged) => {}
            Verdict::Settled(Change::Missing) => {
                if !file.missing {
                    file.missing = true;
                    for (window, doc) in &file.docs {
                        self.reload.missing(window, *doc);
                    }
                }
            }
            Verdict::Settled(Change::Modified(stamp)) => self.reload_all(path, stamp),
        }
    }

    /// Reloads every document that shows `path`, now at `stamp`. A document that fails keeps its
    /// old stamp, so the next change tries again.
    fn reload_all(&mut self, path: &Path, stamp: FileStamp) {
        let Some(file) = self.files.get_mut(path) else {
            return;
        };
        let mut all = true;
        for (window, doc) in &mut file.docs {
            match self.reload.modified(window, *doc, path) {
                Reloaded::Replaced(new) => *doc = new,
                Reloaded::Skipped => {}
                Reloaded::Failed => all = false,
            }
        }
        if all {
            file.opened = stamp;
            file.missing = false;
        }
    }
}

#[cfg(test)]
mod tests;
