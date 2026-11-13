//! Measures the engines' memory for documents spread over windows (ADR 0008's open question on
//! pooling): one engine per window, as the desktop app runs them, each under the main process's
//! supervisor and sandbox, with 1, 5 and 20 documents spread over 1 and 4 windows. Each document
//! has a 512 px tile of its first page drawn, as a window showing it would.
//!
//! ```sh
//! cargo build --release -p laminapdf-desktop-engine
//! cargo run --release -p laminapdf-desktop-engine --example windows -- \
//!     --engine target/release/laminapdf-desktop-engine \
//!     --pdfium apps/desktop/src-tauri/resources/pdfium --files fixtures/golden/examples
//! ```
//!
//! Prints, for each case, the engines' total resident memory when started and after the
//! documents are open and drawn. Memory comes from `ps`, so macOS and Linux only.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "a measuring tool: a panic is a failed run, and its counts are small"
)]

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use laminapdf_desktop_protocol::supervisor::{Config, RangeServer, Supervisor};
use laminapdf_desktop_protocol::{
    DocId, FromEngine, Priority, TileAppearance, TileFormat, TileRequest, ToEngine, ViewRotation,
};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner, SystemFonts};

/// The windows and document counts measured.
const WINDOWS: [usize; 2] = [1, 4];
const DOCUMENTS: [usize; 3] = [1, 5, 20];

/// Every open document's file, read by position.
#[derive(Default)]
struct Files(Mutex<BTreeMap<u32, Arc<File>>>);

impl Files {
    fn add(&self, doc: DocId, file: File) {
        self.0.lock().unwrap().insert(doc.0, Arc::new(file));
    }
}

impl RangeServer for Files {
    fn read(&self, doc: DocId, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let file = self
            .0
            .lock()
            .unwrap()
            .get(&doc.0)
            .cloned()
            .ok_or_else(|| io::Error::other("unknown document"))?;
        let mut buf = vec![0; len as usize];
        let mut filled = 0;
        while filled < buf.len() {
            let n = read_at(&file, &mut buf[filled..], offset + filled as u64)?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        buf.truncate(filled);
        Ok(buf)
    }
}

#[cfg(unix)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, offset)
}

#[cfg(windows)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buf, offset)
}

/// The resident memory of `pid` in MiB, from `ps`.
fn rss_mib(pid: u32) -> f64 {
    Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|out| String::from_utf8_lossy(&out.stdout).trim().parse::<f64>().ok())
        .map_or(0.0, |kib| kib / 1024.0)
}

fn total_mib(engines: &[Supervisor]) -> f64 {
    engines.iter().filter_map(Supervisor::pid).map(rss_mib).sum()
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("windows: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let mut pdfs: Vec<PathBuf> = std::fs::read_dir(&args.files)
        .map_err(|e| format!("{}: {e}", args.files.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    pdfs.sort();
    if pdfs.is_empty() {
        return Err(format!("no PDFs in {}", args.files.display()));
    }
    let fonts = Arc::new(SystemFonts::scan());
    println!("| Windows | Documents | Engines started (MiB) | Documents open and drawn (MiB) | Per engine (MiB) |");
    println!("| ---: | ---: | ---: | ---: | ---: |");
    for windows in WINDOWS {
        for documents in DOCUMENTS {
            let (started, open) = measure(&args, &pdfs, &fonts, windows, documents)?;
            println!(
                "| {windows} | {documents} | {started:.0} | {open:.0} | {:.0} |",
                open / windows as f64
            );
        }
    }
    Ok(())
}

/// Starts `windows` engines, opens `documents` of `pdfs` (round-robin over the windows), draws a
/// tile of each one's first page, and returns the engines' memory before and after.
fn measure(
    args: &Args,
    pdfs: &[PathBuf],
    fonts: &Arc<SystemFonts>,
    windows: usize,
    documents: usize,
) -> Result<(f64, f64), String> {
    let files = Arc::new(Files::default());
    let mut engines = Vec::new();
    for _ in 0..windows {
        let spawner = SandboxSpawner::new(
            args.engine.clone(),
            vec!["--pdfium".into(), args.pdfium.clone().into_os_string()],
            SandboxConfig::default(),
            |_| {},
        );
        let config = Config {
            fonts: Some(Arc::clone(fonts) as _),
            ..Config::default()
        };
        let engine = Supervisor::start(
            Box::new(spawner),
            Arc::clone(&files) as Arc<dyn RangeServer>,
            config,
            |_, _| {},
        )
        .map_err(|e| e.to_string())?;
        engines.push(engine);
    }
    thread::sleep(Duration::from_millis(500));
    let started = total_mib(&engines);
    for i in 0..documents {
        let doc = DocId(i as u32 + 1);
        let path = &pdfs[i % pdfs.len()];
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        files.add(doc, file);
        let engine = &engines[i % windows];
        let (opened, _) = engine
            .request(|req| ToEngine::OpenDocument { req, doc, len })
            .map_err(|e| e.to_string())?;
        if !matches!(opened, FromEngine::Opened { .. }) {
            return Err(format!("{}: {opened:?}", path.display()));
        }
        let (tile, _) = engine
            .request(|req| {
                ToEngine::RenderTile(TileRequest {
                    req,
                    doc,
                    page: 0,
                    scale: 2.0,
                    rotation: ViewRotation::None,
                    appearance: TileAppearance::Original,
                    x: 0,
                    y: 0,
                    w: 512,
                    h: 512,
                    format: TileFormat::Rgba8,
                    priority: Priority::Visible,
                })
            })
            .map_err(|e| e.to_string())?;
        if !matches!(tile, FromEngine::Tile { .. }) {
            return Err(format!("{}: {tile:?}", path.display()));
        }
    }
    thread::sleep(Duration::from_millis(500));
    let open = total_mib(&engines);
    for engine in engines {
        engine.shutdown();
    }
    Ok((started, open))
}

struct Args {
    engine: PathBuf,
    pdfium: PathBuf,
    files: PathBuf,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let (mut engine, mut pdfium, mut files) = (None, None, None);
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let value = PathBuf::from(args.next().ok_or_else(|| format!("{arg} needs a value"))?);
            match arg.as_str() {
                "--engine" => engine = Some(value),
                "--pdfium" => pdfium = Some(value),
                "--files" => files = Some(value),
                _ => return Err(format!("unknown argument {arg}")),
            }
        }
        Ok(Self {
            engine: engine.ok_or("--engine is required")?,
            pdfium: pdfium.ok_or("--pdfium is required")?,
            files: files.ok_or("--files is required")?,
        })
    }
}
