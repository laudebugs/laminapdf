//! [`DocumentRegistry`]: every open document, its file, its window and its kept password.
//!
//! Paths enter only from trusted places in the main process (a dialog opened from Rust, the OS's
//! open-file events, the recent list, a watched file that changed; later session restore). The
//! webview names documents by [`DocId`] and never sends a path, and a path never leaves this
//! module for the webview or an engine.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use laminapdf_desktop_protocol::DocId;
use laminapdf_desktop_protocol::supervisor::RangeServer;

use super::bytesource::FileSource;
use super::secret::SessionPassword;

/// One open document.
struct Entry {
    /// The granted path, never sent anywhere: only its file name leaves the main process.
    path: PathBuf,
    /// The same file's canonical path, which tells a second open of it (through a link, a
    /// different spelling) from another file.
    canonical: Option<PathBuf>,
    source: Arc<FileSource>,
    window: String,
    password: Option<SessionPassword>,
}

/// What reopening a window's documents in a new engine needs.
pub(crate) struct Reopen {
    pub(crate) doc: DocId,
    pub(crate) len: u64,
    /// For `UnlockDocument`; wiped when dropped.
    pub(crate) password: Option<laminapdf_desktop_protocol::Password>,
}

#[derive(Default)]
pub(crate) struct DocumentRegistry {
    entries: Mutex<BTreeMap<DocId, Entry>>,
    next: AtomicU32,
}

impl DocumentRegistry {
    /// Opens `path` for `window` and gives it an id; returns the id and the file's length.
    pub(crate) fn register(&self, path: &Path, window: &str) -> io::Result<(DocId, u64)> {
        let source = FileSource::open(path)?;
        let len = source.len();
        let doc = DocId(self.next.fetch_add(1, Ordering::Relaxed) + 1);
        self.lock().insert(
            doc,
            Entry {
                path: path.to_owned(),
                canonical: std::fs::canonicalize(path).ok(),
                source: Arc::new(source),
                window: window.to_owned(),
                password: None,
            },
        );
        Ok((doc, len))
    }

    /// Whether `doc` is open in `window`: a window may touch only its own documents.
    pub(crate) fn owns(&self, window: &str, doc: DocId) -> bool {
        self.lock().get(&doc).is_some_and(|e| e.window == window)
    }

    /// Makes `window` `doc`'s owner: its engine may read the file, and only its webview may use
    /// the document. A tab moving between windows takes its document, file and kept password
    /// with it.
    pub(crate) fn set_window(&self, doc: DocId, window: &str) {
        if let Some(entry) = self.lock().get_mut(&doc) {
            window.clone_into(&mut entry.window);
        }
    }

    /// The document already open from `path`, and its window: opening the same file twice
    /// focuses its tab instead. Files compare by canonical path, so a link or another spelling
    /// of an open file is found too.
    pub(crate) fn open_from(&self, path: &Path) -> Option<(DocId, String)> {
        let wanted = std::fs::canonicalize(path).ok()?;
        self.lock()
            .iter()
            .find(|(_, entry)| entry.canonical.as_ref() == Some(&wanted))
            .map(|(doc, entry)| (*doc, entry.window.clone()))
    }

    /// `doc`'s file, while it is open: for the watcher, which follows a moved tab. Never sent to
    /// a webview or an engine.
    pub(crate) fn path_of(&self, doc: DocId) -> Option<PathBuf> {
        self.lock().get(&doc).map(|entry| entry.path.clone())
    }

    /// `doc`'s file name without its folder, for the window's title, while it is open.
    pub(crate) fn name_of(&self, doc: DocId) -> Option<String> {
        self.lock().get(&doc).map(|entry| {
            entry
                .path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
        })
    }

    /// `doc`'s file length, while it is open.
    pub(crate) fn len_of(&self, doc: DocId) -> Option<u64> {
        self.lock().get(&doc).map(|entry| entry.source.len())
    }

    /// Keeps `password` for `doc` until it closes.
    pub(crate) fn keep_password(&self, doc: DocId, password: SessionPassword) {
        if let Some(entry) = self.lock().get_mut(&doc) {
            entry.password = Some(password);
        }
    }

    /// A copy of `doc`'s kept password, for reopening it from a changed file; wiped when dropped.
    pub(crate) fn password_of(&self, doc: DocId) -> Option<laminapdf_desktop_protocol::Password> {
        self.lock()
            .get(&doc)
            .and_then(|entry| entry.password.as_ref().map(SessionPassword::to_password))
    }

    /// Wipes `doc`'s kept password (a restarted engine refused it).
    pub(crate) fn forget_password(&self, doc: DocId) {
        if let Some(entry) = self.lock().get_mut(&doc) {
            entry.password = None;
        }
    }

    /// Whether a password is kept for `doc`.
    #[cfg(test)]
    pub(crate) fn has_password(&self, doc: DocId) -> bool {
        self.lock().get(&doc).is_some_and(|e| e.password.is_some())
    }

    /// Closes `doc`: its file handle and its password (wiped) go. `false` if it wasn't open.
    pub(crate) fn remove(&self, doc: DocId) -> bool {
        self.lock().remove(&doc).is_some()
    }

    /// Closes every document of `window`; returns their ids.
    pub(crate) fn remove_window(&self, window: &str) -> Vec<DocId> {
        let mut entries = self.lock();
        let docs: Vec<DocId> = entries
            .iter()
            .filter(|(_, e)| e.window == window)
            .map(|(doc, _)| *doc)
            .collect();
        for doc in &docs {
            entries.remove(doc);
        }
        docs
    }

    /// What a new engine for `window` needs to reopen its documents, in id order.
    pub(crate) fn reopen_list(&self, window: &str) -> Vec<Reopen> {
        self.lock()
            .iter()
            .filter(|(_, e)| e.window == window)
            .map(|(doc, e)| Reopen {
                doc: *doc,
                len: e.source.len(),
                password: e.password.as_ref().map(SessionPassword::to_password),
            })
            .collect()
    }

    /// The reads `window`'s engine may make: its own documents only.
    pub(crate) fn files_of(self: &Arc<Self>, window: &str) -> WindowFiles {
        WindowFiles {
            registry: Arc::clone(self),
            window: window.to_owned(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<DocId, Entry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One window's documents, as its engine reads them. An engine asking for another window's
/// document (a compromised engine guessing ids) gets an error.
pub(crate) struct WindowFiles {
    registry: Arc<DocumentRegistry>,
    window: String,
}

impl RangeServer for WindowFiles {
    fn read(&self, doc: DocId, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let source = {
            let entries = self.registry.lock();
            match entries.get(&doc) {
                Some(entry) if entry.window == self.window => Arc::clone(&entry.source),
                _ => return Err(io::Error::new(io::ErrorKind::NotFound, "no such document")),
            }
        };
        // The read runs without the registry's lock.
        source.read(offset, len)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("laminapdf-registry-{}-{name}", std::process::id()));
            std::fs::File::create(&path)
                .unwrap()
                .write_all(b"%PDF-1.7 test")
                .unwrap();
            Self(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn registry_with(name: &str, window: &str) -> (Temp, Arc<DocumentRegistry>, DocId) {
        let temp = Temp::new(name);
        let registry = Arc::new(DocumentRegistry::default());
        let (doc, _) = registry.register(&temp.0, window).unwrap();
        (temp, registry, doc)
    }

    #[test]
    fn closing_a_document_should_drop_its_password() {
        let (_t, registry, doc) = registry_with("close", "main");
        registry.keep_password(doc, SessionPassword::new("pw"));
        registry.remove(doc);
        assert!(!registry.has_password(doc));
    }

    #[test]
    fn a_refused_password_should_be_forgotten() {
        let (_t, registry, doc) = registry_with("refused", "main");
        registry.keep_password(doc, SessionPassword::new("pw"));
        registry.forget_password(doc);
        assert!(!registry.has_password(doc));
    }

    #[test]
    fn closing_a_window_should_close_its_documents_only() {
        let temp = Temp::new("windows");
        let registry = DocumentRegistry::default();
        let (a, _) = registry.register(&temp.0, "main").unwrap();
        let (b, _) = registry.register(&temp.0, "doc-1").unwrap();
        assert_eq!(
            (registry.remove_window("main"), registry.owns("doc-1", b)),
            (vec![a], true)
        );
    }

    #[test]
    fn the_reopen_list_should_carry_the_kept_password() {
        let (_t, registry, doc) = registry_with("reopen", "main");
        registry.keep_password(doc, SessionPassword::new("pw"));
        let list = registry.reopen_list("main");
        assert!(
            matches!(list.as_slice(), [r] if r.doc == doc && r.password.as_ref().is_some_and(|p| p.expose() == "pw"))
        );
    }

    #[test]
    fn a_window_should_not_read_another_windows_document() {
        let (_t, registry, doc) = registry_with("other", "main");
        let err = registry.files_of("doc-1").read(doc, 0, 4).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn a_window_should_read_its_own_document() {
        let (_t, registry, doc) = registry_with("own", "main");
        assert_eq!(registry.files_of("main").read(doc, 0, 4).unwrap(), b"%PDF");
    }
}
