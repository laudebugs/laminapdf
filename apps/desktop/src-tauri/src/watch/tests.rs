//! The watcher against the real file system and the OS's events, with a fake app: an in-place
//! write, an atomic save, a deletion and a file that comes back. Each waits for what it expects, up
//! to a few seconds, since the OS reports changes late (`FSEvents` by about a second).

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Heard {
    Modified(DocId),
    Missing(DocId),
}

/// Records what the watcher reports; each reload gives the document a new id (old + 100).
#[derive(Default)]
struct Fake {
    heard: Mutex<Vec<Heard>>,
}

impl Reload for Fake {
    fn modified(&self, _window: &str, doc: DocId, _path: &Path) -> Reloaded {
        self.heard().push(Heard::Modified(doc));
        Reloaded::Replaced(DocId(doc.0 + 100))
    }

    fn missing(&self, _window: &str, doc: DocId) {
        self.heard().push(Heard::Missing(doc));
    }
}

impl Fake {
    fn heard(&self) -> std::sync::MutexGuard<'_, Vec<Heard>> {
        self.heard.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits until the watcher has reported `count` things, or 10 s; then a little longer, so a
    /// report that shouldn't come would be seen.
    fn wait_for(&self, count: usize) -> Vec<Heard> {
        let start = Instant::now();
        while self.heard().len() < count && start.elapsed() < Duration::from_secs(10) {
            thread::sleep(Duration::from_millis(50));
        }
        thread::sleep(4 * DEBOUNCE);
        self.heard().clone()
    }
}

/// A temporary folder with `doc.pdf` in it, and a watcher with a fake app watching that file.
struct Setup {
    dir: PathBuf,
    file: PathBuf,
    fake: Arc<Fake>,
    watcher: Watcher,
}

impl Setup {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("laminapdf-watch-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("doc.pdf");
        std::fs::write(&file, b"%PDF-1.7 first").unwrap();
        let fake = Arc::new(Fake::default());
        let watcher = Watcher::start(Arc::clone(&fake) as Arc<dyn Reload>).unwrap();
        watcher.watch(&file, "main", DocId(1));
        // Let the OS's watch begin before the test changes anything.
        thread::sleep(Duration::from_millis(500));
        Self {
            dir,
            file,
            fake,
            watcher,
        }
    }
}

impl Drop for Setup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_file_written_in_place_should_reload_once() {
    let setup = Setup::new("in-place");
    std::fs::write(&setup.file, b"%PDF-1.7 second, longer").unwrap();
    assert_eq!(setup.fake.wait_for(1), vec![Heard::Modified(DocId(1))]);
}

#[test]
fn an_atomic_save_should_reload_and_never_report_missing() {
    let setup = Setup::new("atomic");
    let temp = setup.dir.join(".doc.pdf.swp");
    std::fs::write(&temp, b"%PDF-1.7 saved by an editor").unwrap();
    std::fs::rename(&temp, &setup.file).unwrap();
    assert_eq!(setup.fake.wait_for(1), vec![Heard::Modified(DocId(1))]);
}

#[test]
fn a_deleted_file_should_be_reported_missing() {
    let setup = Setup::new("deleted");
    std::fs::remove_file(&setup.file).unwrap();
    assert_eq!(setup.fake.wait_for(1), vec![Heard::Missing(DocId(1))]);
}

#[test]
fn a_file_that_comes_back_should_reload() {
    let setup = Setup::new("back");
    std::fs::remove_file(&setup.file).unwrap();
    setup.fake.wait_for(1);
    std::fs::write(&setup.file, b"%PDF-1.7 restored").unwrap();
    assert_eq!(
        setup.fake.wait_for(2),
        vec![Heard::Missing(DocId(1)), Heard::Modified(DocId(1))]
    );
}

#[test]
fn a_reloaded_document_should_be_watched_under_its_new_id() {
    let setup = Setup::new("new-id");
    std::fs::write(&setup.file, b"%PDF-1.7 second").unwrap();
    setup.fake.wait_for(1);
    std::fs::write(&setup.file, b"%PDF-1.7 third, longer").unwrap();
    assert_eq!(
        setup.fake.wait_for(2),
        vec![Heard::Modified(DocId(1)), Heard::Modified(DocId(101))]
    );
}

#[test]
fn a_closed_document_should_not_be_reported() {
    let setup = Setup::new("closed");
    setup.watcher.unwatch("main", DocId(1));
    thread::sleep(Duration::from_millis(200));
    std::fs::write(&setup.file, b"%PDF-1.7 second").unwrap();
    assert_eq!(setup.fake.wait_for(1), vec![]);
}

#[test]
fn another_file_in_the_folder_should_not_reload() {
    let setup = Setup::new("neighbour");
    std::fs::write(setup.dir.join("other.pdf"), b"%PDF-1.7 other").unwrap();
    assert_eq!(setup.fake.wait_for(1), vec![]);
}
