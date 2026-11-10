//! The built engine binary under the main process's [`Supervisor`]: documents open and report
//! their pages from the engine, encrypted ones unlock, a killed engine is replaced, and tiles
//! match an in-process PDFium render (that test needs PDFium: `cargo xtask render-golden`).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "tests: a panic is a test failure; fixture offsets are small"
)]

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::supervisor::{CommandSpawner, Config, Event, RangeServer, Supervisor, SupervisorError};
use laminapdf_desktop_protocol::{
    DocId, FailureKind, FromEngine, Password, Priority, ReqId, TileFormat, TileRequest, ToEngine,
};
use laminapdf_test_fixtures::{fixture, read_at};

/// The test's files, by document, standing in for the main process's open files.
#[derive(Default)]
struct Files(Mutex<BTreeMap<DocId, Vec<u8>>>);

impl RangeServer for Files {
    fn read(&self, doc: DocId, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let files = self.0.lock().unwrap();
        let bytes = files.get(&doc).ok_or_else(|| io::Error::other("unknown document"))?;
        if offset > bytes.len() as u64 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let mut buf = vec![0; len as usize];
        let n = read_at(bytes, offset, &mut buf);
        buf.truncate(n);
        Ok(buf)
    }
}

struct Engine {
    supervisor: Supervisor,
    files: Arc<Files>,
    events: mpsc::Receiver<Event>,
}

impl Engine {
    fn start() -> Self {
        let files = Arc::new(Files::default());
        let (tx, events) = mpsc::channel();
        let tx = Mutex::new(tx);
        let spawner = CommandSpawner::new(
            PathBuf::from(env!("CARGO_BIN_EXE_laminapdf-desktop-engine")),
            Vec::new(),
            |line| eprintln!("engine: {line}"),
        );
        let supervisor = Supervisor::start(
            Box::new(spawner),
            Arc::clone(&files) as Arc<dyn RangeServer>,
            Config::default(),
            move |_, event| match event {
                Event::Log(line) => eprintln!("supervisor: {line}"),
                event => {
                    let _ = tx.lock().unwrap().send(event);
                }
            },
        )
        .unwrap();
        Self {
            supervisor,
            files,
            events,
        }
    }

    fn add(&self, doc: u32, bytes: Vec<u8>) -> (DocId, u64) {
        let len = bytes.len() as u64;
        self.files.0.lock().unwrap().insert(DocId(doc), bytes);
        (DocId(doc), len)
    }

    fn open(&self, doc: DocId, len: u64) -> FromEngine {
        self.ask(|req| ToEngine::OpenDocument { req, doc, len })
    }

    fn unlock(&self, doc: DocId, password: &str) -> FromEngine {
        self.ask(|req| ToEngine::UnlockDocument {
            req,
            doc,
            password: Password::new(password.into()),
        })
    }

    /// Sends a request and returns its reply. Right after a restart the supervisor answers `Lost`
    /// until the restart handler has returned (its restoring gate): that answer is asked again.
    fn ask(&self, make: impl Fn(ReqId) -> ToEngine) -> FromEngine {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match self.supervisor.request(&make) {
                Err(SupervisorError::Lost) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
                reply => return reply.unwrap().0,
            }
        }
    }

    fn restarted(&self) -> Option<u32> {
        match self.events.recv_timeout(Duration::from_secs(5)) {
            Ok(Event::Restarted { pid }) => Some(pid),
            _ => None,
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.supervisor.shutdown();
    }
}

fn lamina_pages(bytes: Vec<u8>, password: Option<&str>) -> Vec<(f32, f32, u16)> {
    let mut options = laminapdf::OpenOptions::new();
    if let Some(password) = password {
        options = options.with_password(password);
    }
    let doc = laminapdf::Document::open_bytes(bytes, options).unwrap();
    doc.page_infos()
        .unwrap()
        .iter()
        .map(|p| (p.width as f32, p.height as f32, p.rotation))
        .collect()
}

fn sizes(reply: &FromEngine) -> Vec<(f32, f32, u16)> {
    match reply {
        FromEngine::PageInfo { sizes, .. } => sizes.iter().map(|s| (s.width, s.height, s.rotate)).collect(),
        other => panic!("expected PageInfo, got {other:?}"),
    }
}

#[test]
fn a_document_should_report_the_pages_lamina_reads() {
    let engine = Engine::start();
    let bytes = fixture("corpus/fpdf2.pdf");
    let expected = lamina_pages(bytes.clone(), None);
    let (doc, len) = engine.add(1, bytes);
    assert!(
        matches!(engine.open(doc, len), FromEngine::Opened { page_count, needs_password: false, .. } if page_count as usize == expected.len())
    );
    assert_eq!(sizes(&engine.ask(|req| ToEngine::GetPageInfo { req, doc })), expected);
}

#[test]
fn bytes_that_are_not_a_pdf_should_fail_to_open_without_killing_the_engine() {
    let engine = Engine::start();
    let pid = engine.supervisor.pid();
    let (doc, len) = engine.add(1, b"not a pdf at all, just words".to_vec());
    assert!(matches!(
        engine.open(doc, len),
        FromEngine::Error {
            kind: FailureKind::Invalid,
            ..
        }
    ));
    assert_eq!(engine.supervisor.pid(), pid);
}

#[test]
fn a_file_that_fails_to_read_should_fail_the_open() {
    let engine = Engine::start();
    // The engine is told the file is longer than it is: reads past the end fail.
    let (doc, len) = engine.add(1, fixture("corpus/hello.pdf"));
    let reply = engine.open(doc, len + 1_000_000);
    assert!(matches!(reply, FromEngine::Error { .. }), "{reply:?}");
}

#[test]
fn a_killed_engine_should_restart_within_two_seconds_and_open_again() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/hello.pdf"));
    assert!(matches!(engine.open(doc, len), FromEngine::Opened { .. }));
    let before = engine.supervisor.pid();
    let killed = Instant::now();
    engine.supervisor.kill();
    let after = engine.restarted();
    let took = killed.elapsed();
    assert!(
        after.is_some() && after != before && took < Duration::from_secs(2),
        "{before:?} → {after:?} in {took:?}"
    );
    assert!(matches!(engine.open(doc, len), FromEngine::Opened { .. }));
}

#[test]
fn an_encrypted_document_should_wait_for_its_password() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("encrypted/r4-aes-128-user.pdf"));
    assert!(matches!(
        engine.open(doc, len),
        FromEngine::Opened {
            needs_password: true,
            encrypted: true,
            page_count: 0,
            ..
        }
    ));
    assert!(matches!(
        engine.ask(|req| ToEngine::GetPageInfo { req, doc }),
        FromEngine::NeedsPassword { .. }
    ));
}

#[test]
fn a_wrong_password_should_be_refused() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("encrypted/r4-aes-128-user.pdf"));
    engine.open(doc, len);
    assert!(matches!(
        engine.unlock(doc, "not-it"),
        FromEngine::Error {
            kind: FailureKind::WrongPassword,
            ..
        }
    ));
}

#[test]
fn the_password_should_unlock_the_document_again_in_a_restarted_engine() {
    let engine = Engine::start();
    let bytes = fixture("encrypted/r4-aes-128-user.pdf");
    let expected = lamina_pages(bytes.clone(), Some("user-pw"));
    let (doc, len) = engine.add(1, bytes);
    engine.open(doc, len);
    assert!(matches!(engine.unlock(doc, "user-pw"), FromEngine::Unlocked { .. }));
    engine.supervisor.kill();
    assert!(engine.restarted().is_some());
    // What the main process does after a restart: reopen, then unlock with the kept password.
    assert!(matches!(
        engine.open(doc, len),
        FromEngine::Opened {
            needs_password: true,
            ..
        }
    ));
    assert!(
        matches!(engine.unlock(doc, "user-pw"), FromEngine::Unlocked { page_count, .. } if page_count as usize == expected.len())
    );
    assert_eq!(sizes(&engine.ask(|req| ToEngine::GetPageInfo { req, doc })), expected);
}

#[test]
fn a_closed_document_should_be_forgotten() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/hello.pdf"));
    engine.open(doc, len);
    engine.supervisor.send(&ToEngine::CloseDocument { doc });
    assert!(matches!(
        engine.ask(|req| ToEngine::GetPageInfo { req, doc }),
        FromEngine::Error {
            kind: FailureKind::UnknownDocument,
            ..
        }
    ));
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn a_tile_should_match_an_in_process_pdfium_render() {
    use laminapdf_render::{CancelToken, PixelRect, RenderBackend, RenderOpenOptions, RenderRequest};

    let engine = Engine::start();
    let bytes = fixture("corpus/fpdf2.pdf");
    let (doc, len) = engine.add(1, bytes.clone());
    engine.open(doc, len);
    let tile = TileRequest {
        req: ReqId(0),
        doc,
        page: 0,
        scale_milli: 1500,
        x: 64,
        y: 128,
        w: 256,
        h: 192,
        format: TileFormat::Rgba8,
        priority: Priority::Visible,
    };
    let (reply, pixels) = engine
        .supervisor
        .request(|req| ToEngine::RenderTile(TileRequest { req, ..tile }))
        .unwrap();
    assert!(matches!(reply, FromEngine::Tile { w: 256, h: 192, .. }), "{reply:?}");

    let backend = laminapdf_render_pdfium::PdfiumBackend::from_env().unwrap();
    let source: Arc<dyn laminapdf_render::ByteSource> = Arc::new(bytes);
    let local = backend.open(source, &RenderOpenOptions::new()).unwrap();
    let request = RenderRequest::page(0, 1.5).with_clip(PixelRect {
        x: 64,
        y: 128,
        width: 256,
        height: 192,
    });
    let mut expected = vec![0; local.plan(&request).unwrap().byte_len()];
    local.render(&request, &mut expected, &CancelToken::new()).unwrap();
    assert!(
        pixels == expected,
        "the engine's tile differs from the in-process render"
    );
}
