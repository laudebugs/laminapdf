//! [`Engines`] against the protocol crate's fake engine: a document whose file length is even is
//! encrypted with the fake's password.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::{OnceLock, mpsc};
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
fn a_window_whose_engine_failed_should_get_a_fresh_one_on_its_next_open() {
    let mut f = Fixture::new(Mode::CrashAfterReady);
    let _ = {
        let path = f.file(PLAIN);
        f.engines.open("main", &path)
    };
    while let Some(told) = f.told() {
        if matches!(told, Told::Failed(_)) {
            break;
        }
    }
    let failed = f.engines.windows().get("main").map(Supervisor::has_failed);
    // The Reopen button opens again: a new engine starts instead of the failed one answering.
    let _ = {
        let path = f.file(PLAIN);
        f.engines.open("main", &path)
    };
    let replaced = f.engines.windows().get("main").map(Supervisor::has_failed);
    assert_eq!((failed, replaced), (Some(true), Some(false)));
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

/// A 512 × 300 tile of `page` with `ticket`.
fn tile_of(doc: u32, page: u32, ticket: u64) -> TileQuery {
    TileQuery {
        doc: DocId(doc),
        page,
        scale: 1.5,
        rotation: laminapdf_desktop_protocol::ViewRotation::None,
        appearance: laminapdf_desktop_protocol::TileAppearance::Original,
        x: 0,
        y: 0,
        w: 512,
        h: 300,
        priority: laminapdf_desktop_protocol::Priority::Visible,
        ticket,
        encoding: tiles::Encoding::Webp,
    }
}

fn open_plain(f: &mut Fixture, window: &str) -> u32 {
    let path = f.file(PLAIN);
    f.engines.open(window, &path).unwrap().doc
}

#[test]
fn a_tile_should_come_back_at_the_size_asked_for() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = open_plain(&mut f, "main");
    let tile = f.engines.render_tile("main", &tile_of(doc, 0, 1)).unwrap();
    assert_eq!((tile.w, tile.h, tile.pixels.len()), (512, 300, 512 * 300 * 4));
}

#[test]
fn a_window_should_not_get_tiles_of_another_windows_document() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = open_plain(&mut f, "main");
    let refused = f.engines.render_tile("doc-2", &tile_of(doc, 0, 1));
    assert!(matches!(refused, Err(DesktopError::UnknownDocument)));
}

#[test]
fn a_locked_documents_tile_should_ask_for_its_password() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = {
        let path = f.file(ENCRYPTED);
        f.engines.open("main", &path).unwrap().doc
    };
    let locked = f.engines.render_tile("main", &tile_of(doc, 0, 1));
    assert!(matches!(
        locked,
        Err(DesktopError::Engine {
            kind: "needs-password",
            ..
        })
    ));
}

/// Waits (up to 5 s) until a tile render has its ticket in `engines`, so a cancel can find it.
fn until_waiting(engines: &Engines) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while engines.tickets().is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn cancelling_a_tile_should_end_its_wait_as_cancelled() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = open_plain(&mut f, "main");
    let engines = Arc::clone(&f.engines);
    // The fake holds page 2's tiles until they are cancelled.
    let waiting = std::thread::spawn(move || engines.render_tile("main", &tile_of(doc, 2, 7)));
    until_waiting(&f.engines);
    f.engines.cancel_tiles("main", &[7]);
    let result = waiting.join().unwrap();
    assert!(
        matches!(result, Err(DesktopError::Engine { kind: "cancelled", .. })) && f.engines.tickets().is_empty(),
        "{result:?}"
    );
}

#[test]
fn another_windows_cancel_should_not_touch_a_tile() {
    let mut f = Fixture::new(Mode::Healthy);
    let doc = open_plain(&mut f, "main");
    let _other = open_plain(&mut f, "doc-2");
    let engines = Arc::clone(&f.engines);
    let waiting = std::thread::spawn(move || engines.render_tile("main", &tile_of(doc, 2, 7)));
    until_waiting(&f.engines);
    // The same ticket from another window names nothing of main's.
    f.engines.cancel_tiles("doc-2", &[7]);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!waiting.is_finished());
    f.engines.cancel_tiles("main", &[7]);
    assert!(waiting.join().unwrap().is_err());
}

/// `src/engine/wire.fixture.json`: what this side and the webview's client must agree on. The
/// client's tests read the same file.
fn wire() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../src/engine/wire.fixture.json")).unwrap()
}

#[test]
fn every_tile_address_the_client_builds_should_parse_to_what_it_means() {
    use laminapdf_desktop_protocol::{Priority, TileAppearance, ViewRotation};

    use super::tiles::Encoding;

    for tile in wire()["tiles"].as_array().unwrap() {
        let address = &tile["address"];
        let parsed = parse_tile(tile["path"].as_str().unwrap(), tile["query"].as_str()).unwrap();
        let expect = &tile["parsed"];
        let clip = &address["clip"];
        assert_eq!(
            (
                u64::from(parsed.doc.0),
                u64::from(parsed.page),
                f64::from(parsed.scale),
                u64::from(parsed.x),
                u64::from(parsed.y),
                u64::from(parsed.w),
                u64::from(parsed.h),
                parsed.ticket
            ),
            (
                address["doc"].as_u64().unwrap(),
                address["page"].as_u64().unwrap(),
                address["scale"].as_f64().unwrap(),
                clip["x"].as_u64().unwrap(),
                clip["y"].as_u64().unwrap(),
                clip["width"].as_u64().unwrap(),
                clip["height"].as_u64().unwrap(),
                address["ticket"].as_u64().unwrap()
            )
        );
        let rotation = match parsed.rotation {
            ViewRotation::None => "none",
            ViewRotation::Quarter => "quarter",
            ViewRotation::Half => "half",
            ViewRotation::ThreeQuarters => "three-quarters",
        };
        let appearance = match parsed.appearance {
            TileAppearance::Original => "original",
            TileAppearance::Sepia => "sepia",
            TileAppearance::Dark => "dark",
            TileAppearance::HighContrast { ink, paper } => {
                let colour = |key: &str| -> Vec<u64> {
                    expect[key]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|c| c.as_u64().unwrap())
                        .collect()
                };
                assert_eq!(
                    (ink.map(u64::from).to_vec(), paper.map(u64::from).to_vec()),
                    (colour("ink"), colour("paper"))
                );
                "high-contrast"
            }
        };
        let priority = match parsed.priority {
            Priority::Visible => "visible",
            Priority::Prefetch => "prefetch",
        };
        let encoding = match parsed.encoding {
            Encoding::Webp => "webp",
            Encoding::Raw => "raw",
        };
        assert_eq!(
            [rotation, appearance, priority, encoding],
            ["rotation", "appearance", "priority", "encoding"].map(|key| expect[key].as_str().unwrap())
        );
    }
}

#[test]
fn the_engine_events_and_restart_states_should_be_the_ones_the_client_listens_for() {
    let wire = wire();
    assert_eq!(
        [RESTARTED_EVENT, FAILED_EVENT],
        [
            wire["events"]["restarted"].as_str().unwrap(),
            wire["events"]["failed"].as_str().unwrap()
        ]
    );
    let states: Vec<serde_json::Value> = [Reopened::Open, Reopened::Locked, Reopened::Closed]
        .iter()
        .map(|state| serde_json::to_value(state).unwrap())
        .collect();
    assert_eq!(serde_json::Value::from(states), wire["reopened"]);
}

#[test]
fn a_cancelled_tile_should_answer_the_kind_and_status_the_client_reads_as_an_abort() {
    let error = engine_error(FailureKind::Cancelled);
    let cancelled = &wire()["cancelled"];
    assert_eq!(
        (error.kind(), u64::from(tiles::status_of(&error).as_u16())),
        (
            cancelled["kind"].as_str().unwrap(),
            cancelled["status"].as_u64().unwrap()
        )
    );
}

/// Every log message this test process writes, at every level.
fn logged() -> &'static Mutex<Vec<String>> {
    use tauri_plugin_log::log;

    struct Capture;
    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, record: &log::Record<'_>) {
            if let Ok(mut lines) = logged().lock() {
                lines.push(record.args().to_string());
            }
        }
        fn flush(&self) {}
    }

    static LINES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    LINES.get_or_init(|| {
        // Tracing's `log` bridge hands its events here while no subscriber is set.
        if log::set_boxed_logger(Box::new(Capture)).is_ok() {
            log::set_max_level(log::LevelFilter::Trace);
        }
        Mutex::new(Vec::new())
    })
}

#[test]
fn a_password_should_never_reach_the_log() {
    let _ = logged();
    let mut f = Fixture::new(Mode::Healthy);
    let doc = DocId(
        {
            let path = f.file(ENCRYPTED);
            f.engines.open("main", &path)
        }
        .unwrap()
        .doc,
    );
    // A refused password, the right one, then a restart that reuses the kept one.
    let _ = f.engines.unlock("main", doc, "not-the-password");
    f.engines.unlock("main", doc, PASSWORD).unwrap();
    f.engines.kill("main");
    let _ = f.told();
    let lines = logged().lock().unwrap();
    assert!(!lines.is_empty(), "the restart logs at least its warning");
    let leaked: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains(PASSWORD) || line.contains("not-the-password"))
        .collect();
    assert!(leaked.is_empty(), "{leaked:?}");
}

#[test]
fn a_moved_document_should_open_in_its_new_window_and_leave_the_old_one() {
    let mut f = Fixture::new(Mode::Healthy);
    let path = f.file(PLAIN);
    let doc = DocId(f.engines.open("main", &path).unwrap().doc);
    let moved = f.engines.move_doc(doc, "main", "doc-1").unwrap();
    assert_eq!((moved.doc, moved.page_count), (doc.0, 3));
    assert!(f.engines.page_info("doc-1", doc).is_ok());
    assert!(matches!(
        f.engines.page_info("main", doc),
        Err(DesktopError::UnknownDocument)
    ));
}

#[test]
fn a_moved_encrypted_document_should_keep_its_password_and_not_ask_again() {
    let mut f = Fixture::new(Mode::Healthy);
    let path = f.file(ENCRYPTED);
    let doc = DocId(f.engines.open("main", &path).unwrap().doc);
    f.engines.unlock("main", doc, PASSWORD).unwrap();
    let moved = f.engines.move_doc(doc, "main", "doc-1").unwrap();
    assert!(!moved.needs_password && moved.pages.len() == 3, "{moved:?}");
    assert!(f.engines.registry.has_password(doc));
    // The old window closing afterwards doesn't touch the moved document.
    f.engines.close_window("main");
    assert!(f.engines.registry.has_password(doc));
    assert!(f.engines.page_info("doc-1", doc).is_ok());
    // Closing it for good wipes the password.
    f.engines.close("doc-1", doc).unwrap();
    assert!(!f.engines.registry.has_password(doc));
}

#[test]
fn a_move_the_new_window_refuses_should_leave_the_document_where_it_was() {
    let mut f = Fixture::new(Mode::RefuseReopen);
    let path = f.file(ENCRYPTED);
    let doc = DocId(f.engines.open("main", &path).unwrap().doc);
    f.engines.unlock("main", doc, PASSWORD).unwrap();
    // "doc-1" starts the second engine, which refuses every document.
    assert!(f.engines.move_doc(doc, "main", "doc-1").is_err());
    assert!(f.engines.page_info("main", doc).is_ok());
    assert!(matches!(
        f.engines.page_info("doc-1", doc),
        Err(DesktopError::UnknownDocument)
    ));
    assert!(f.engines.registry.has_password(doc));
}

#[test]
fn a_move_whose_unlock_fails_should_leave_the_document_unlocked_where_it_was() {
    let mut f = Fixture::new(Mode::RefuseUnlockLater);
    let path = f.file(ENCRYPTED);
    let doc = DocId(f.engines.open("main", &path).unwrap().doc);
    f.engines.unlock("main", doc, PASSWORD).unwrap();
    // "doc-1" starts the second engine, which opens the document but refuses its password.
    assert!(f.engines.move_doc(doc, "main", "doc-1").is_err());
    assert!(f.engines.page_info("main", doc).is_ok());
    assert!(f.engines.registry.owns("main", doc));
    assert!(f.engines.registry.has_password(doc));
}

#[test]
fn a_document_should_not_move_to_a_window_that_has_closed() {
    let mut f = Fixture::new(Mode::Healthy);
    let path = f.file(PLAIN);
    let doc = DocId(f.engines.open("main", &path).unwrap().doc);
    f.engines.close_window("doc-1");
    assert!(matches!(
        f.engines.move_doc(doc, "main", "doc-1"),
        Err(DesktopError::UnknownWindow)
    ));
    assert!(f.engines.registry.owns("main", doc));
}

#[test]
fn only_its_own_window_may_move_a_document() {
    let mut f = Fixture::new(Mode::Healthy);
    let path = f.file(PLAIN);
    let doc = DocId(f.engines.open("main", &path).unwrap().doc);
    assert!(matches!(
        f.engines.move_doc(doc, "doc-2", "doc-1"),
        Err(DesktopError::UnknownDocument)
    ));
}
