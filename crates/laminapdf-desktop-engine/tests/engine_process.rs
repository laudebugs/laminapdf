//! The built engine binary under the main process's [`Supervisor`]: documents open and report
//! their pages from the engine, encrypted ones unlock, a killed engine is replaced, and tiles
//! match an in-process PDFium render, and a page printed band by band matches one whole print
//! render (those tests need PDFium: `cargo xtask render-golden`).
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
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::supervisor::{Config, Event, FontServer, RangeServer, Supervisor, SupervisorError};
use laminapdf_desktop_protocol::{
    DocId, EditCall, EditReport, FailureKind, FromEngine, Password, PrintBandRequest, Priority, Query, ReqId,
    SaveAnswer, SaveCall, SaveRequest, SaveWritten, SavedAs, TileAppearance, TileFormat, TileRequest, ToEngine,
    ViewRotation,
};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner, SystemFonts};
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

/// The system's fonts, listed once for every test, as the app lists them once.
fn system_fonts() -> Arc<SystemFonts> {
    static FONTS: OnceLock<Arc<SystemFonts>> = OnceLock::new();
    Arc::clone(FONTS.get_or_init(|| Arc::new(SystemFonts::scan())))
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
        // Sandboxed, as the app starts it: every test here runs inside the sandbox (D2c).
        let spawner = SandboxSpawner::new(
            PathBuf::from(env!("CARGO_BIN_EXE_laminapdf-desktop-engine")),
            Vec::new(),
            SandboxConfig::default(),
            |line| eprintln!("engine: {line}"),
        );
        let config = Config {
            fonts: Some(system_fonts() as Arc<dyn FontServer>),
            ..Config::default()
        };
        let supervisor = Supervisor::start(
            Box::new(spawner),
            Arc::clone(&files) as Arc<dyn RangeServer>,
            config,
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

    /// The new engine's pid once it restarted; the crash event before it is skipped.
    fn restarted(&self) -> Option<u32> {
        loop {
            match self.events.recv_timeout(Duration::from_secs(5)) {
                Ok(Event::Restarted { pid }) => return Some(pid),
                Ok(Event::Crashed { .. }) => {}
                _ => return None,
            }
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
    // text-latin.pdf, then every example document whose fonts are all embedded: a tile of each
    // first page. The sandboxed engine can't reach the system fonts the in-process render may
    // substitute for a missing one (ADR 0008 §5, fonts).
    let mut samples = vec![("golden/text-latin.pdf".to_owned(), fixture("golden/text-latin.pdf"))];
    let examples = laminapdf_test_fixtures::fixtures_dir().join("golden/examples");
    let mut names: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        })
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no example documents in {}", examples.display());
    for name in names {
        let rel = format!("golden/examples/{name}");
        let bytes = fixture(&rel);
        if text_in_embedded_fonts(&bytes) {
            samples.push((rel, bytes));
        }
    }
    eprintln!(
        "comparing {:?}",
        samples.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    let engine = Engine::start();
    for (n, (name, bytes)) in samples.into_iter().enumerate() {
        let (doc, len) = engine.add(n as u32 + 1, bytes.clone());
        engine.open(doc, len);
        assert_tile_matches(&engine, doc, bytes, &name);
    }
}

#[test]
fn a_print_band_of_a_document_that_is_not_open_should_be_refused() {
    let engine = Engine::start();
    let reply = engine.ask(|req| {
        ToEngine::RenderPrintBand(PrintBandRequest {
            req,
            doc: DocId(7),
            page: 0,
            dpi: 300,
            band: 0,
            format: TileFormat::Bgra8,
        })
    });
    assert!(
        matches!(
            reply,
            FromEngine::Error {
                kind: FailureKind::UnknownDocument,
                ..
            }
        ),
        "{reply:?}"
    );
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn a_page_printed_band_by_band_should_match_one_whole_print_render() {
    use laminapdf_render::{CancelToken, RenderBackend, RenderFlags, RenderLimits, RenderOpenOptions, RenderRequest};

    let bytes = fixture("golden/text-latin.pdf");
    let engine = Engine::start();
    let (doc, len) = engine.add(1, bytes.clone());
    engine.open(doc, len);
    let band = |band: u32, format: TileFormat| {
        engine
            .supervisor
            .request(|req| {
                ToEngine::RenderPrintBand(PrintBandRequest {
                    req,
                    doc,
                    page: 0,
                    dpi: 600,
                    band,
                    format,
                })
            })
            .unwrap()
    };
    // At 600 dpi a letter-size page needs several 16 MiB bands.
    let mut page = Vec::new();
    let (mut next, mut count, mut size) = (0, 1, (0, 0));
    while next < count {
        let (reply, pixels) = band(next, TileFormat::Rgba8);
        let FromEngine::PrintBand {
            dpi: 600,
            width,
            height,
            y,
            rows,
            bands,
            format: TileFormat::Rgba8,
            ..
        } = reply
        else {
            panic!("band {next}: {reply:?}");
        };
        assert_eq!(
            y as usize * width as usize * 4,
            page.len(),
            "band {next} starts where the last ended"
        );
        assert_eq!(pixels.len(), width as usize * rows as usize * 4);
        page.extend_from_slice(&pixels);
        (count, size) = (bands, (width, height));
        next += 1;
    }
    assert!(count > 1, "the page fits one band: the test proves nothing");
    assert_eq!(page.len(), size.0 as usize * size.1 as usize * 4);

    let (_, bgra) = band(0, TileFormat::Bgra8);
    let swapped: Vec<u8> = bgra
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|[b, g, r, a]| [*r, *g, *b, *a])
        .collect();
    assert!(
        page.starts_with(&swapped),
        "BGRA is the same pixels, red and blue swapped"
    );

    let backend = laminapdf_render_pdfium::PdfiumBackend::from_env().unwrap();
    let source: Arc<dyn laminapdf_render::ByteSource> = Arc::new(bytes);
    // The whole page at 600 dpi is over the default per-tile limit, which is why the engine bands it.
    let whole_page = RenderLimits {
        max_render_pixels: 64 * 1024 * 1024,
        ..RenderLimits::default()
    };
    let local = backend
        .open(source, &RenderOpenOptions::new().with_limits(whole_page))
        .unwrap();
    let request = RenderRequest::page(0, 600.0 / 72.0).with_flags(RenderFlags {
        print: true,
        ..RenderFlags::default()
    });
    let mut expected = vec![0; local.plan(&request).unwrap().byte_len()];
    local.render(&request, &mut expected, &CancelToken::new()).unwrap();
    // PDFium rounds anti-aliased edges a level apart when a band starts at another origin: a
    // few hundred of the 139 million bytes differ by one level (measured with this fixture).
    let worst = page
        .iter()
        .zip(&expected)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    assert!(worst <= 1, "a band differs from the whole render by {worst} levels");
}

/// The file has text, and holds every font it uses, so both renders draw the same glyphs. A
/// file without fonts (images only) has no text for the tile to hold.
fn text_in_embedded_fonts(bytes: &[u8]) -> bool {
    let doc = laminapdf::Document::open_bytes(bytes.to_vec(), laminapdf::OpenOptions::default()).unwrap();
    let fonts = doc.properties().unwrap().fonts;
    !fonts.is_empty() && fonts.iter().all(|font| font.embedded)
}

/// Renders one tile of `doc`'s first page in the engine and in process, and compares the pixels.
fn assert_tile_matches(engine: &Engine, doc: DocId, bytes: Vec<u8>, name: &str) {
    use laminapdf_render::{CancelToken, PixelRect, RenderBackend, RenderOpenOptions, RenderRequest};

    let tile = TileRequest {
        req: ReqId(0),
        doc,
        page: 0,
        scale: 1.5,
        rotation: ViewRotation::None,
        appearance: TileAppearance::Original,
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
    assert!(
        matches!(reply, FromEngine::Tile { w: 256, h: 192, .. }),
        "{name}: {reply:?}"
    );

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
        pixels.as_chunks::<4>().0.iter().any(|[r, ..]| *r < 128),
        "the tile holds no text"
    );
    assert!(
        pixels == expected,
        "{name}: the engine's tile differs from the in-process render"
    );
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn text_in_a_non_embedded_standard_font_should_still_render_in_the_sandbox() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("golden/text-std14.pdf"));
    engine.open(doc, len);
    let tile = |req| {
        ToEngine::RenderTile(TileRequest {
            req,
            doc,
            page: 0,
            scale: 1.0,
            rotation: ViewRotation::None,
            appearance: TileAppearance::Original,
            x: 0,
            y: 0,
            w: 512,
            h: 512,
            format: TileFormat::Rgba8,
            priority: Priority::Visible,
        })
    };
    let (reply, pixels) = engine.supervisor.request(tile).unwrap();
    assert!(matches!(reply, FromEngine::Tile { .. }), "{reply:?}");
    // PDFium's built-in fonts stand in for the system's: the text is drawn, not left out.
    let dark = pixels.as_chunks::<4>().0.iter().filter(|[r, ..]| *r < 128).count();
    assert!(dark > 500, "only {dark} dark pixels");
}

/// Chinese text in a font the file doesn't embed: the sandboxed engine can't open the system's
/// fonts, so its glyphs come from a face the main process serves (Windows: PDFium's GDI mapper).
#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn cjk_text_in_a_font_the_file_does_not_embed_should_render_in_the_sandbox() {
    let has_cjk = system_fonts()
        .faces()
        .iter()
        .any(|face| face.charsets.contains(laminapdf_desktop_protocol::Charsets::GB2312));
    if !has_cjk && !cfg!(windows) {
        eprintln!("this system has no Simplified Chinese font: nothing to render with");
        return;
    }
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("desktop/cjk-not-embedded.pdf"));
    engine.open(doc, len);
    let started = Instant::now();
    let (reply, pixels) = engine
        .supervisor
        .request(|req| {
            ToEngine::RenderTile(TileRequest {
                req,
                doc,
                page: 0,
                scale: 1.0,
                rotation: ViewRotation::None,
                appearance: TileAppearance::Original,
                x: 0,
                y: 0,
                w: 300,
                h: 100,
                format: TileFormat::Rgba8,
                priority: Priority::Visible,
            })
        })
        .unwrap();
    eprintln!("first CJK tile in {:?}", started.elapsed());
    assert!(matches!(reply, FromEngine::Tile { .. }), "{reply:?}");
    // Six glyphs at 40 pt cover thousands of pixels; a missing font leaves the page blank.
    let dark = pixels.as_chunks::<4>().0.iter().filter(|[r, ..]| *r < 128).count();
    assert!(dark > 1500, "only {dark} dark pixels: the glyphs didn't render");
}

/// The report of an edit call on `doc`.
fn edit(engine: &Engine, doc: DocId, call: &EditCall) -> EditReport {
    match engine.ask(|req| ToEngine::Edit {
        req,
        doc,
        call: call.clone(),
    }) {
        FromEngine::Edited { report, .. } => *report,
        other => panic!("expected Edited, got {other:?}"),
    }
}

/// An op log rotating page `id` to `rotate` degrees.
fn rotate(id: &str, rotate: u16) -> EditCall {
    EditCall::Apply {
        log: format!(r#"{{"v":1,"ops":[{{"op":"rotatePage","args":{{"page":"{id}","rotate":{rotate}}}}}]}}"#),
    }
}

#[test]
fn an_edit_should_turn_a_page_and_undo_and_redo_should_take_it_back_and_again() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/fpdf2.pdf"));
    engine.open(doc, len);
    let FromEngine::PageInfo { ids, sizes, .. } = engine.ask(|req| ToEngine::GetPageInfo { req, doc }) else {
        panic!("expected PageInfo");
    };
    let first = ids[0].clone();
    let turned = edit(&engine, doc, &rotate(&first, 90));
    let undone = edit(&engine, doc, &EditCall::Undo);
    let redone = edit(&engine, doc, &EditCall::Redo);
    let rotation = |report: &EditReport| report.pages[0].size.rotate;
    assert_eq!(
        (
            turned.changed,
            turned.touched.clone(),
            rotation(&turned),
            rotation(&undone),
            rotation(&redone)
        ),
        (true, vec![first], 90, sizes[0].rotate, 90)
    );
}

#[test]
fn an_undo_with_nothing_to_undo_should_change_nothing() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/fpdf2.pdf"));
    engine.open(doc, len);
    let report = edit(&engine, doc, &EditCall::Undo);
    assert!(!report.changed && report.reversal.is_none());
}

#[test]
fn an_op_on_a_page_that_does_not_exist_should_be_refused_with_its_code() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/fpdf2.pdf"));
    engine.open(doc, len);
    let reply = engine.ask(|req| ToEngine::Edit {
        req,
        doc,
        call: rotate("p9999.0", 90),
    });
    assert!(
        matches!(&reply, FromEngine::Error { kind: FailureKind::EditRefused, message, .. } if message == "unknown-page"),
        "{reply:?}"
    );
}

/// The Properties dialog's data for `doc`, parsed.
fn properties(engine: &Engine, doc: DocId) -> serde_json::Value {
    match engine.ask(|req| ToEngine::GetProperties { req, doc }) {
        FromEngine::Properties { json, .. } => {
            serde_json::from_str(&json).unwrap_or_else(|error| panic!("the data isn't JSON: {error}"))
        }
        other => panic!("expected Properties, got {other:?}"),
    }
}

#[test]
fn the_properties_should_show_a_metadata_edit_before_it_is_saved() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("golden/metadata.pdf"));
    engine.open(doc, len);
    edit(
        &engine,
        doc,
        &EditCall::Apply {
            log: r#"{"v":1,"ops":[{"op":"setMetadata","args":{"patch":{"title":"Edited title"}}}]}"#.into(),
        },
    );
    let data = properties(&engine, doc);
    assert_eq!(
        (
            &data["metadata"]["title"],
            data["xmp"]["xml"].as_str().map(|x| x.contains("Edited title"))
        ),
        (&serde_json::json!("Edited title"), Some(true))
    );
}

#[test]
fn the_properties_of_a_locked_document_should_wait_for_its_password() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("encrypted/r4-aes-128-user.pdf"));
    engine.open(doc, len);
    assert!(matches!(
        engine.ask(|req| ToEngine::GetProperties { req, doc }),
        FromEngine::NeedsPassword { .. }
    ));
}

/// The engine's answer to `query` on `doc`, parsed.
fn answer(engine: &Engine, doc: DocId, query: &Query) -> serde_json::Value {
    match engine.ask(|req| ToEngine::Query {
        req,
        doc,
        query: query.clone(),
    }) {
        FromEngine::Answer { json, .. } => {
            serde_json::from_str(&json).unwrap_or_else(|error| panic!("the answer isn't JSON: {error}"))
        }
        other => panic!("expected Answer, got {other:?}"),
    }
}

#[test]
fn the_label_ranges_should_show_a_label_edit_before_it_is_saved() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/fpdf2.pdf"));
    engine.open(doc, len);
    let FromEngine::PageInfo { ids, .. } = engine.ask(|req| ToEngine::GetPageInfo { req, doc }) else {
        panic!("expected PageInfo");
    };
    let before = answer(&engine, doc, &Query::LabelRanges);
    let log = format!(
        r#"{{"v":1,"ops":[{{"op":"setPageLabels","args":{{"ranges":[{{"page":"{}","style":"lowerRoman"}}]}}}}]}}"#,
        ids[0]
    );
    edit(&engine, doc, &EditCall::Apply { log });
    let after = answer(&engine, doc, &Query::LabelRanges);
    assert_eq!(
        (before, after[0]["style"].as_str()),
        (serde_json::json!([]), Some("lowerRoman"))
    );
}

#[test]
fn deleting_a_page_of_a_certified_file_should_be_asked_about_first() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("signed/certified-p1.pdf"));
    engine.open(doc, len);
    let FromEngine::PageInfo { ids, .. } = engine.ask(|req| ToEngine::GetPageInfo { req, doc }) else {
        panic!("expected PageInfo");
    };
    let log = format!(
        r#"{{"v":1,"ops":[{{"op":"deletePages","args":{{"pages":["{}"],"references":"retarget"}}}}]}}"#,
        ids[0]
    );
    let found = answer(&engine, doc, &Query::WouldViolate { log });
    assert_eq!(found[0]["restriction"].as_str(), Some("docMdp"));
}

/// The answer to a save call on `doc`, and the bytes its frame carried.
fn save(engine: &Engine, doc: DocId, call: &SaveCall) -> Result<(SaveAnswer, Vec<u8>), (FailureKind, String)> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let reply = engine.supervisor.request(|req| ToEngine::Save {
            req,
            doc,
            call: call.clone(),
        });
        match reply {
            Err(SupervisorError::Lost) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok((FromEngine::Saved { answer, .. }, payload)) => return Ok((*answer, payload)),
            Ok((FromEngine::Error { kind, message, .. }, _)) => return Err((kind, message)),
            other => panic!("expected Saved, got {other:?}"),
        }
    }
}

/// Writes `doc`'s save with `request` and reads its output back in parts of `part` bytes.
fn write_and_read(engine: &Engine, doc: DocId, request: SaveRequest, part: u32) -> (SaveWritten, Vec<u8>) {
    let Ok((SaveAnswer::Written(written), _)) = save(engine, doc, &SaveCall::Write(request)) else {
        panic!("expected Written");
    };
    let mut output = Vec::new();
    while (output.len() as u64) < written.len {
        let call = SaveCall::Read {
            offset: output.len() as u64,
            len: part,
        };
        let (_, bytes) = save(engine, doc, &call).unwrap();
        assert!(!bytes.is_empty(), "a read inside the output came back empty");
        output.extend(bytes);
    }
    (written, output)
}

/// The rotation of `bytes`' first page.
fn first_rotation(bytes: Vec<u8>) -> u16 {
    lamina_pages(bytes, None)[0].2
}

/// Opens `name` as document 1 and turns its first page to `degrees`; returns the document and
/// the file's bytes.
fn open_and_turn(engine: &Engine, name: &str, degrees: u16) -> (DocId, Vec<u8>) {
    let bytes = fixture(name);
    let (doc, len) = engine.add(1, bytes.clone());
    engine.open(doc, len);
    let FromEngine::PageInfo { ids, .. } = engine.ask(|req| ToEngine::GetPageInfo { req, doc }) else {
        panic!("expected PageInfo");
    };
    edit(engine, doc, &rotate(&ids[0], degrees));
    (doc, bytes)
}

#[test]
fn an_incremental_save_should_give_only_the_revision_to_append_and_adopt_the_file_with_it() {
    let engine = Engine::start();
    let (doc, file) = open_and_turn(&engine, "corpus/fpdf2.pdf", 90);
    let (written, tail) = write_and_read(&engine, doc, SaveRequest::default(), 100);
    let saved = [file.as_slice(), tail.as_slice()].concat();
    // The main process writes the file, then the engine adopts it: the next save appends to it.
    engine.add(1, saved.clone());
    let adopted = save(&engine, doc, &SaveCall::Adopt).map(|(answer, _)| answer);
    let FromEngine::PageInfo { ids, .. } = engine.ask(|req| ToEngine::GetPageInfo { req, doc }) else {
        panic!("expected PageInfo");
    };
    edit(&engine, doc, &rotate(&ids[0], 180));
    let (_, next) = write_and_read(&engine, doc, SaveRequest::default(), 1 << 20);
    let again = [saved.as_slice(), next.as_slice()].concat();
    assert_eq!(
        (
            written.mode,
            tail.len() as u64 == written.len,
            adopted,
            first_rotation(saved),
            first_rotation(again)
        ),
        (SavedAs::Incremental, true, Ok(SaveAnswer::Adopted), 90, 180)
    );
}

#[test]
fn a_full_rewrite_should_give_the_whole_file() {
    let engine = Engine::start();
    let (doc, _) = open_and_turn(&engine, "corpus/fpdf2.pdf", 270);
    let request = SaveRequest {
        rewrite: true,
        ..SaveRequest::default()
    };
    let (written, file) = write_and_read(&engine, doc, request, 1 << 20);
    engine.add(1, file.clone());
    let adopted = save(&engine, doc, &SaveCall::Adopt).map(|(answer, _)| answer);
    // The rewritten file is the document's now: the next edit and save go on from it.
    let FromEngine::PageInfo { ids, .. } = engine.ask(|req| ToEngine::GetPageInfo { req, doc }) else {
        panic!("expected PageInfo");
    };
    let turned = edit(&engine, doc, &rotate(&ids[0], 0));
    let (_, next) = write_and_read(&engine, doc, SaveRequest::default(), 1 << 20);
    let again = [file.as_slice(), next.as_slice()].concat();
    assert_eq!(
        (
            written.mode,
            adopted,
            first_rotation(file),
            turned.pages[0].size.rotate,
            first_rotation(again)
        ),
        (SavedAs::FullRewrite, Ok(SaveAnswer::Adopted), 270, 0, 0)
    );
}

#[test]
fn a_plan_should_say_how_the_save_would_go() {
    let engine = Engine::start();
    let (doc, _) = open_and_turn(&engine, "corpus/fpdf2.pdf", 90);
    let Ok((SaveAnswer::Planned(plan), _)) = save(&engine, doc, &SaveCall::Plan(SaveRequest::default())) else {
        panic!("expected Planned");
    };
    assert_eq!(
        (plan.mode, plan.refusal, plan.unchanged),
        (Some(SavedAs::Incremental), None, false)
    );
}

#[test]
fn an_adoption_with_no_output_waiting_should_be_refused() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/fpdf2.pdf"));
    engine.open(doc, len);
    let refused = save(&engine, doc, &SaveCall::Adopt);
    assert_eq!(refused, Err((FailureKind::SaveRefused, "no-pending-save".to_owned())));
}

#[test]
fn an_edit_after_a_write_should_drop_the_output_it_made_stale() {
    let engine = Engine::start();
    let (doc, _) = open_and_turn(&engine, "corpus/fpdf2.pdf", 90);
    write_and_read(&engine, doc, SaveRequest::default(), 1 << 20);
    edit(&engine, doc, &EditCall::Undo);
    let refused = save(&engine, doc, &SaveCall::Adopt);
    assert_eq!(refused, Err((FailureKind::SaveRefused, "no-pending-save".to_owned())));
}

/// The report of adding document `source` to `doc` as a source: the source's ID and its pages'.
fn add_source(engine: &Engine, doc: DocId, source: DocId) -> (String, Vec<String>) {
    match engine.ask(|req| ToEngine::AddSource {
        req,
        doc,
        source,
        password: None,
    }) {
        FromEngine::SourceAdded { source, pages, .. } => (source, pages),
        other => panic!("expected SourceAdded, got {other:?}"),
    }
}

#[test]
fn a_source_should_give_the_ids_of_its_pages_and_insert_them_into_the_document() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/fpdf2.pdf"));
    engine.open(doc, len);
    let (other, other_len) = engine.add(2, fixture("corpus/fpdf2.pdf"));
    engine.open(other, other_len);
    let (source, pages) = add_source(&engine, doc, other);
    let FromEngine::PageInfo { ids, .. } = engine.ask(|req| ToEngine::GetPageInfo { req, doc: other }) else {
        panic!("expected PageInfo");
    };
    let before = ids.len();
    assert_eq!(pages.len(), before, "one ID for each page of the source document");
    let log = format!(
        r#"{{"v":1,"ops":[{{"op":"insertPagesFrom","args":{{"source":"{source}","pages":["{}"],"at":"end"}}}}]}}"#,
        pages[0]
    );
    let report = edit(&engine, doc, &EditCall::Apply { log });
    // The same bytes are the same source, which the edit session already holds.
    let again = add_source(&engine, doc, other);
    assert_eq!(
        (report.changed, report.pages.len(), again.0 == source),
        (true, before + 1, true)
    );
}

#[test]
fn a_source_that_is_not_open_should_be_refused() {
    let engine = Engine::start();
    let (doc, len) = engine.add(1, fixture("corpus/fpdf2.pdf"));
    engine.open(doc, len);
    let reply = engine.ask(|req| ToEngine::AddSource {
        req,
        doc,
        source: DocId(9),
        password: None,
    });
    assert!(
        matches!(
            reply,
            FromEngine::Error {
                kind: FailureKind::UnknownDocument,
                ..
            }
        ),
        "{reply:?}"
    );
}
