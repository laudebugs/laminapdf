//! Measures the engine process the way the desktop app drives it (ADR 0008 §2's numbers): the built
//! engine under the main process's supervisor, the document read by position from its file.
//!
//! ```sh
//! cargo build --release -p laminapdf-desktop-engine
//! cargo run --release -p laminapdf-desktop-engine --example measure -- \
//!     --engine target/release/laminapdf-desktop-engine --file target/fixtures/1gb.pdf [--tiles 200]
//! ```
//!
//! Prints the time from spawning the engine to its `Ready` (PDFium loaded and the sandbox on), the
//! engine's resident memory when idle, the time from `OpenDocument` to `Opened` and
//! to `PageInfo`, the peak memory while opening, the reads it made, then (with `--tiles N` and
//! PDFium in `LAMINA_PDF_PDFIUM_DIR`) N 512 px tiles on pages spread through the document: tiles
//! a second, the bytes read for them, and the peak memory. Memory comes from `ps`, so macOS
//! and Linux only.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "a measuring tool: a panic is a failed run, and its counts are small"
)]

use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::supervisor::{Config, RangeServer, Supervisor};
use laminapdf_desktop_protocol::{DocId, FromEngine, Priority, TileFormat, TileRequest, ToEngine};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner, SystemFonts};

/// The file, read by position, counting what the engine asked for.
struct Counted {
    file: File,
    reads: AtomicU64,
    bytes: AtomicU64,
}

impl RangeServer for Counted {
    fn read(&self, _: DocId, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len as usize];
        let mut filled = 0;
        while filled < buf.len() {
            let n = read_at(&self.file, &mut buf[filled..], offset + filled as u64)?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        buf.truncate(filled);
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(filled as u64, Ordering::Relaxed);
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

/// Samples `pid`'s memory every 10 ms until stopped; returns the peak in MiB.
struct Peak {
    stop: Arc<AtomicBool>,
    highest: Arc<Mutex<f64>>,
    thread: thread::JoinHandle<()>,
}

impl Peak {
    fn watch(pid: u32) -> Self {
        let (stop, highest) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(0.0_f64)));
        let thread = {
            let (stop, highest) = (Arc::clone(&stop), Arc::clone(&highest));
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let now = rss_mib(pid);
                    let mut high = highest.lock().unwrap();
                    *high = high.max(now);
                    drop(high);
                    thread::sleep(Duration::from_millis(10));
                }
            })
        };
        Self { stop, highest, thread }
    }

    fn stop(self) -> f64 {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.thread.join();
        *self.highest.lock().unwrap()
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("measure: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let file = File::open(&args.file).map_err(|e| format!("{}: {e}", args.file.display()))?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    let files = Arc::new(Counted {
        file,
        reads: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
    });
    let spawner = SandboxSpawner::new(args.engine.clone(), Vec::new(), SandboxConfig::default(), |line| {
        eprintln!("engine: {line}");
    });
    // The app lists the system's fonts once, in the background; here before the clock starts.
    let fonts = SystemFonts::scan();
    println!("system fonts: {} faces listed in {:.1} ms", fonts.len(), fonts.took.as_secs_f64() * 1000.0);
    let config = Config {
        fonts: Some(Arc::new(fonts)),
        ..Config::default()
    };
    let starting = Instant::now();
    let engine = Supervisor::start(
        Box::new(spawner),
        Arc::clone(&files) as Arc<dyn RangeServer>,
        config,
        |_, _| {},
    )
    .map_err(|e| e.to_string())?;
    let start_time = starting.elapsed();
    let pid = engine.pid().ok_or("no engine")?;
    thread::sleep(Duration::from_millis(500));
    println!("file: {} ({} MiB)", args.file.display(), len >> 20);
    let sandbox = engine.sandbox().map(|level| level.describe()).unwrap_or_default();
    println!(
        "spawn → Ready: {:.1} ms (sandbox: {sandbox})",
        start_time.as_secs_f64() * 1000.0
    );
    println!("engine idle: {:.1} MiB", rss_mib(pid));

    let doc = DocId(1);
    let peak = Peak::watch(pid);
    let started = Instant::now();
    let opened = engine
        .request(|req| ToEngine::OpenDocument { req, doc, len })
        .map_err(|e| e.to_string())?;
    let open_time = started.elapsed();
    let pages = engine
        .request(|req| ToEngine::GetPageInfo { req, doc })
        .map_err(|e| e.to_string())?;
    let info_time = started.elapsed();
    let open_peak = peak.stop();
    let (page_count, repaired) = match (&opened.0, &pages.0) {
        (
            FromEngine::Opened {
                page_count, repaired, ..
            },
            FromEngine::PageInfo { sizes, .. },
        ) if sizes.len() == *page_count as usize => (*page_count, *repaired),
        other => return Err(format!("unexpected replies: {other:?}")),
    };
    let (reads, bytes) = (files.reads.load(Ordering::Relaxed), files.bytes.load(Ordering::Relaxed));
    println!("pages: {page_count}{}", if repaired { " (repaired)" } else { "" });
    println!("OpenDocument → Opened: {} ms", open_time.as_millis());
    println!("OpenDocument → PageInfo: {} ms", info_time.as_millis());
    println!("peak while opening: {open_peak:.1} MiB");
    println!(
        "reads while opening: {reads} ({:.1} MiB, {:.0} MiB/s of wall time)",
        bytes as f64 / 1_048_576.0,
        bytes as f64 / 1_048_576.0 / info_time.as_secs_f64()
    );
    println!("after opening: {:.1} MiB", rss_mib(pid));

    if args.tiles > 0 {
        tiles(&engine, &files, pid, page_count, args.tiles)?;
    }
    engine.shutdown();
    Ok(())
}

/// Renders `count` tiles on pages spread through the document, one after another.
fn tiles(engine: &Supervisor, files: &Counted, pid: u32, page_count: u32, count: u32) -> Result<(), String> {
    let before = files.bytes.load(Ordering::Relaxed);
    let peak = Peak::watch(pid);
    let started = Instant::now();
    let mut rendered = 0;
    for i in 0..count {
        let page = (u64::from(i) * u64::from(page_count) / u64::from(count)) as u32;
        let (reply, _) = engine
            .request(|req| {
                ToEngine::RenderTile(TileRequest {
                    req,
                    doc: DocId(1),
                    page,
                    scale_milli: 2000,
                    x: 0,
                    y: 0,
                    w: 512,
                    h: 512,
                    format: TileFormat::Rgba8,
                    priority: Priority::Visible,
                })
            })
            .map_err(|e| e.to_string())?;
        match reply {
            FromEngine::Tile { .. } => rendered += 1,
            other => return Err(format!("page {page}: {other:?}")),
        }
    }
    let took = started.elapsed();
    let bytes = files.bytes.load(Ordering::Relaxed) - before;
    println!(
        "tiles: {rendered} in {} ms ({:.0} tiles/s); read {:.1} MiB for them ({:.0} MiB/s)",
        took.as_millis(),
        f64::from(rendered) / took.as_secs_f64(),
        bytes as f64 / 1_048_576.0,
        bytes as f64 / 1_048_576.0 / took.as_secs_f64()
    );
    println!("peak while rendering: {:.1} MiB", peak.stop());
    Ok(())
}

struct Args {
    engine: PathBuf,
    file: PathBuf,
    tiles: u32,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let (mut engine, mut file, mut tiles) = (None, None, 0);
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
            match arg.as_str() {
                "--engine" => engine = Some(PathBuf::from(value)),
                "--file" => file = Some(PathBuf::from(value)),
                "--tiles" => tiles = value.parse().map_err(|_| format!("--tiles {value}: not a number"))?,
                _ => return Err(format!("unknown argument {arg}")),
            }
        }
        Ok(Self {
            engine: engine.ok_or("--engine is required")?,
            file: file.ok_or("--file is required")?,
            tiles,
        })
    }
}
