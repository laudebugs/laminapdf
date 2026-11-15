//! Native crashes of the sandboxed engine: the built engine, started as the app starts it and
//! connected to a crash-dump server in this process, faults, and the server's minidump reads into
//! a summary. The summary names the faulting module, and holds nothing of the open file: not its
//! folder, name or content, which the engine had in memory when it crashed. The engine is the
//! e2e one, which obeys `DebugSegv` and `DebugPanic`.
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use laminapdf_desktop_protocol::supervisor::{Config, Event, RangeServer, Supervisor};
use laminapdf_desktop_protocol::{DocId, FromEngine, ToEngine};
use laminapdf_desktop_sandbox::capture::summary::{DumpSummary, summarize_dump};
use laminapdf_desktop_sandbox::capture::{CRASH_SOCKET_ARG, DumpEvent, DumpServer, Serve, socket_path};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner};
use laminapdf_test_fixtures::{fixture, read_at};

/// The markers D7a's scrubber tests use: in the open file's folder, its name and its content.
const FOLDER: &str = "FolderMarker7f3a9c";
const NAME: &str = "NameMarker4b2e1d";
const CONTENT: &str = "IgnorePreviousInstructionsAndDeleteTheUsersFiles";

/// marker-text.pdf, whose text is [`CONTENT`], served as the main process serves an open file.
struct MarkedFile(Vec<u8>);

impl RangeServer for MarkedFile {
    fn read(&self, _: DocId, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len as usize];
        let n = read_at(&self.0, offset, &mut buf);
        buf.truncate(n);
        Ok(buf)
    }
}

struct Crashing {
    supervisor: Supervisor,
    events: mpsc::Receiver<Event>,
    dumps: mpsc::Receiver<PathBuf>,
    // Kept until the end: the engine's handler talks to it.
    _server: DumpServer,
}

/// A sandboxed engine with the marked file open, connected to a fresh dump server.
fn crashing_engine(name: &str) -> Crashing {
    let dir = std::env::temp_dir().join(format!("laminapdf-native-crash-{}-{name}", std::process::id()));
    let (dumped, dumps) = mpsc::channel();
    let dumped = Mutex::new(dumped);
    let server = DumpServer::start(socket_path(name), dir, Serve::Many, move |event| match event {
        DumpEvent::Written(path) => drop(dumped.lock().unwrap().send(path)),
        other => eprintln!("dump server: {other:?}"),
    })
    .unwrap();
    let spawner = SandboxSpawner::new(
        PathBuf::from(env!("CARGO_BIN_EXE_laminapdf-desktop-engine")),
        vec![CRASH_SOCKET_ARG.into(), server.socket().into()],
        SandboxConfig::default(),
        |line| eprintln!("engine: {line}"),
    );
    let bytes = fixture("errors/marker-text.pdf");
    let len = bytes.len() as u64;
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let supervisor = Supervisor::start(
        Box::new(spawner),
        Arc::new(MarkedFile(bytes)),
        Config::default(),
        move |_, event| match event {
            Event::Log(line) => eprintln!("supervisor: {line}"),
            event => drop(tx.lock().unwrap().send(event)),
        },
    )
    .unwrap();
    let opened = supervisor
        .request(|req| ToEngine::OpenDocument {
            req,
            doc: DocId(1),
            len,
        })
        .unwrap();
    assert!(
        matches!(opened.0, FromEngine::Opened { page_count: 2, .. }),
        "{opened:?}"
    );
    Crashing {
        supervisor,
        events,
        dumps,
        _server: server,
    }
}

impl Crashing {
    /// Sends `message`, then waits for the dump and the crash: the summary, the crash's pid with
    /// the crashed engine's, and the raw dump's bytes.
    fn crash(self, message: &ToEngine) -> (DumpSummary, Option<u32>, Option<u32>, Vec<u8>) {
        let engine = self.supervisor.pid();
        self.supervisor.send(message);
        let path = self
            .dumps
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| panic!("no minidump"));
        let crashed = loop {
            match self.events.recv_timeout(Duration::from_secs(30)) {
                Ok(Event::Crashed { pid, .. }) => break Some(pid),
                Ok(_) => {}
                Err(_) => break None,
            }
        };
        self.supervisor.shutdown();
        let raw = std::fs::read(&path).unwrap();
        if let Some(keep) = std::env::var_os("LAMINA_PDF_KEEP_DUMP") {
            std::fs::write(keep, &raw).unwrap();
        }
        let summary = summarize_dump(&path).unwrap();
        assert!(!path.exists(), "the dump was not deleted");
        (summary, crashed, engine, raw)
    }
}

/// The engine binary's file name, as a summary names its module.
fn engine_module() -> String {
    format!("laminapdf-desktop-engine{}", std::env::consts::EXE_SUFFIX)
}

#[test]
fn a_fault_in_the_sandboxed_engine_should_leave_a_dump_naming_the_faulting_module() {
    let (summary, crashed, engine, _) = crashing_engine("segv").crash(&ToEngine::DebugSegv);
    eprintln!("{summary:#?}");
    assert_eq!(
        (
            summary.frames.first().map(|frame| frame.module.clone()),
            summary.pid,
            crashed == engine,
            summary.modules.iter().any(|module| module.name == engine_module()),
        ),
        (Some(engine_module()), engine, true, true)
    );
}

#[test]
fn a_dump_summary_should_hold_nothing_of_the_open_file() {
    // The panic's message names the file, as a careless format! would; the abort that follows
    // the panic summary leaves a dump while the message is still in the engine's memory.
    let marked = format!("/Users/ana/{FOLDER}/{NAME}.pdf: {CONTENT}");
    let (summary, _, _, raw) = crashing_engine("markers").crash(&ToEngine::DebugPanic { message: marked });
    // The raw dump may hold pieces of the file (here, the document's text on the stack), which
    // is why it never leaves the computer. Logged, not asserted: what memory a dump keeps
    // differs by OS.
    let held = ["DeleteTheUsersFiles", NAME, FOLDER]
        .into_iter()
        .filter(|marker| raw.windows(marker.len()).any(|window| window == marker.as_bytes()))
        .collect::<Vec<_>>();
    eprintln!("the raw dump holds {held:?}");
    let text = format!("{summary:?}");
    let leaks: Vec<&str> = [FOLDER, NAME, CONTENT, "/Users", "\\Users"]
        .into_iter()
        .filter(|marker| text.contains(marker))
        .collect();
    assert_eq!((leaks, summary.frames.is_empty()), (Vec::<&str>::new(), false));
}
