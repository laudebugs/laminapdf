//! [`Engines`] against the protocol crate's fake engine: a document whose file length is even is
//! encrypted with the fake's password.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use laminapdf_desktop_protocol::supervisor::fake::{FakeSpawner, Mode, PASSWORD};

use super::*;

/// Records what the windows were told.
struct Recorder(Mutex<mpsc::Sender<Told>>);

#[derive(Debug, PartialEq, Eq)]
enum Told {
    Restarted(Restarted),
    Failed(Failed),
}

impl Notify for Recorder {
    fn restarted(&self, payload: Restarted) {
        let _ = self.0.lock().unwrap().send(Told::Restarted(payload));
    }

    fn failed(&self, payload: Failed) {
        let _ = self.0.lock().unwrap().send(Told::Failed(payload));
    }
}

struct Fixture {
    engines: Arc<Engines>,
    told: mpsc::Receiver<Told>,
    files: Vec<PathBuf>,
}

impl Fixture {
    fn new(mode: Mode) -> Self {
        let (tx, told) = mpsc::channel();
        let spawner = FakeSpawner::new(mode);
        let launcher: Launcher = Arc::new(move || Box::new(spawner.clone()));
        let config = Config {
            backoff: Duration::from_millis(10),
            ..Config::default()
        };
        Self {
            engines: Engines::new(launcher, config, Arc::new(Recorder(Mutex::new(tx)))),
            told,
            files: Vec::new(),
        }
    }

    /// A file of `len` bytes: odd opens plainly, even is encrypted.
    fn file(&mut self, len: usize) -> PathBuf {
        // One counter for the whole test binary: tests run in parallel, and a name built from one
        // fixture's own count let two tests write, then delete, the same file.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("laminapdf-engines-{}-{n}-{len}.pdf", std::process::id()));
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&vec![b'x'; len])
            .unwrap();
        self.files.push(path.clone());
        path
    }

    fn told(&self) -> Option<Told> {
        self.told.recv_timeout(Duration::from_secs(5)).ok()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.engines.shutdown();
        for file in &self.files {
            let _ = std::fs::remove_file(file);
        }
    }
}

const PLAIN: usize = 11;
const ENCRYPTED: usize = 12;

#[test]
fn opening_should_return_the_page_sizes_from_the_engine() {
    let mut f = Fixture::new(Mode::Healthy);
    let meta = {
        let path = f.file(PLAIN);
        f.engines.open("main", &path)
    }
    .unwrap();
    assert_eq!(
        (meta.page_count, meta.pages.len(), meta.pages.first().map(|p| p.width)),
        (3, 3, Some(612.0))
    );
}

#[test]
fn an_encrypted_document_should_open_locked_without_pages() {
    let mut f = Fixture::new(Mode::Healthy);
    let meta = {
        let path = f.file(ENCRYPTED);
        f.engines.open("main", &path)
    }
    .unwrap();
    assert!(meta.needs_password && meta.pages.is_empty());
}

#[test]
fn a_wrong_password_should_be_refused_and_not_kept() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = DocId(
        {
            let path = f.file(ENCRYPTED);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    let refused = f.engines.unlock("main", doc, "nope");
    assert!(
        matches!(
            refused,
            Err(DesktopError::Engine {
                kind: "wrong-password",
                ..
            })
        ) && !f.engines.registry.has_password(doc)
    );
}

#[test]
fn the_right_password_should_be_kept_until_the_document_closes() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = DocId(
        {
            let path = f.file(ENCRYPTED);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    let unlocked = f.engines.unlock("main", doc, PASSWORD).unwrap();
    let kept = f.engines.registry.has_password(doc);
    f.engines.close("main", doc).unwrap();
    assert_eq!(
        (unlocked.pages.len(), kept, f.engines.registry.has_password(doc)),
        (3, true, false)
    );
}

#[test]
fn a_restart_should_reopen_and_unlock_without_asking() {
    let mut f = Fixture::new(Mode::Healthy);
    let plain = {
        let path = f.file(PLAIN);
        f.engines.open("main", &path)
    }
    .unwrap()
    .doc;
    let secret = DocId(
        {
            let path = f.file(ENCRYPTED);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    f.engines.unlock("main", secret, PASSWORD).unwrap();
    f.engines.kill("main");
    assert_eq!(
        f.told(),
        Some(Told::Restarted(Restarted {
            window: "main".into(),
            docs: vec![
                RestartedDoc {
                    doc: plain,
                    state: Reopened::Open
                },
                RestartedDoc {
                    doc: secret.0,
                    state: Reopened::Open
                },
            ],
        }))
    );
}

#[test]
fn a_refused_kept_password_should_be_wiped_and_the_document_reported_locked() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = DocId(
        {
            let path = f.file(ENCRYPTED);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    f.engines.registry.keep_password(doc, SessionPassword::new("stale"));
    f.engines.kill("main");
    let told = f.told();
    assert!(
        matches!(&told, Some(Told::Restarted(r)) if r.docs == [RestartedDoc { doc: doc.0, state: Reopened::Locked }])
            && !f.engines.registry.has_password(doc),
        "{told:?}"
    );
}

#[test]
fn a_document_that_fails_to_reopen_should_be_closed() {
    let mut f = Fixture::new(Mode::RefuseReopen);
    let doc = DocId(
        {
            let path = f.file(PLAIN);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    f.engines.kill("main");
    let told = f.told();
    assert!(
        matches!(&told, Some(Told::Restarted(r)) if r.docs == [RestartedDoc { doc: doc.0, state: Reopened::Closed }])
            && !f.engines.registry.owns("main", doc),
        "{told:?}"
    );
}

#[test]
fn a_document_that_kills_the_engine_should_be_closed_and_the_others_kept() {
    let mut f = Fixture::new(Mode::DieReopeningFirst);
    let first = {
        let path = f.file(PLAIN);
        f.engines.open("main", &path)
    }
    .unwrap()
    .doc;
    let second = {
        let path = f.file(PLAIN);
        f.engines.open("main", &path)
    }
    .unwrap()
    .doc;
    f.engines.kill("main");
    let (one, two) = (f.told(), f.told());
    assert_eq!(
        (one, two),
        (
            Some(Told::Restarted(Restarted {
                window: "main".into(),
                docs: vec![RestartedDoc {
                    doc: first,
                    state: Reopened::Closed
                }],
            })),
            Some(Told::Restarted(Restarted {
                window: "main".into(),
                docs: vec![RestartedDoc {
                    doc: second,
                    state: Reopened::Open
                }],
            })),
        )
    );
}

#[test]
fn pages_should_work_again_after_a_restart() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = DocId(
        {
            let path = f.file(PLAIN);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    f.engines.kill("main");
    let _ = f.told();
    // The event is sent from the restart handler, before its gate opens: ask again until it does.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut pages = f.engines.page_info("main", doc);
    while matches!(
        pages,
        Err(DesktopError::Engine {
            kind: "engine-restarted",
            ..
        })
    ) && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(10));
        pages = f.engines.page_info("main", doc);
    }
    assert_eq!(pages.unwrap().len(), 3);
}

#[test]
fn an_engine_that_keeps_crashing_should_be_reported_failed() {
    let mut f = Fixture::new(Mode::CrashAfterReady);
    let _ = {
        let path = f.file(PLAIN);
        f.engines.open("main", &path)
    };
    let mut last = None;
    while let Some(told) = f.told() {
        let failed = matches!(told, Told::Failed(_));
        last = Some(told);
        if failed {
            break;
        }
    }
    assert_eq!(last, Some(Told::Failed(Failed { window: "main".into() })));
}

#[test]
fn a_window_should_not_reach_another_windows_document() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = DocId(
        {
            let path = f.file(PLAIN);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    assert!(matches!(
        f.engines.page_info("doc-2", doc),
        Err(DesktopError::UnknownDocument)
    ));
}

#[test]
fn closing_a_window_should_close_its_documents() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = DocId(
        {
            let path = f.file(ENCRYPTED);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    f.engines.unlock("main", doc, PASSWORD).unwrap();
    f.engines.close_window("main");
    assert!(!f.engines.registry.owns("main", doc) && !f.engines.registry.has_password(doc));
}

#[test]
fn a_missing_file_should_fail_to_open() {
    let f = Fixture::new(Mode::Healthy);
    let missing = std::env::temp_dir().join("laminapdf-engines-missing.pdf");
    assert!(matches!(f.engines.open("main", &missing), Err(DesktopError::Io(_))));
}
