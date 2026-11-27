//! Measures what the desktop app does to show an edit to the page in view (E6f): the page turned a
//! quarter, the engine writing one appended revision, PDFium (the native build) reopening the file
//! with it, and the page in view drawn again. The built engine runs under the main process's
//! supervisor with the document read by position from its file, as in the app.
//!
//! ```sh
//! cargo xtask desktop-reload [--gate]
//! # or, by hand, with PDFium in LAMINA_PDF_PDFIUM_DIR (`cargo xtask fetch-pdfium`):
//! cargo build --release -p laminapdf-desktop-engine
//! cargo run --release -p laminapdf-desktop-engine --example reload -- \
//!     --engine target/release/laminapdf-desktop-engine [--out target/desktop-reload] [--gate]
//! ```
//!
//! Each of the files of about 1, 10 and 100 MB (text pages with one image of seeded noise each,
//! written under `--out` the first time, never committed) is opened afresh [`RUNS`] times in a new
//! engine process; every run draws the page in view, then times the edit call and the page drawn
//! again from the call. The desktop has no quick pass, so the second is the figure the web
//! harness calls first new pixels. The medians go to `<out>/reload.json` as the nightly history
//! keeps them (`reload-first-pixels-<n>mb`, in nanoseconds). A turn stands in for a placed picture:
//! the engine is built without the builder's appearance writer, so it refuses `addContentObject`
//! (`appearance-unavailable`) and its protocol takes no image bytes yet. The reload is the same for
//! any op that touches a page: one appended revision, then PDFium opens the file with it.
//!
//! `--gate` exits with a failure when a median is over its budget in [`BUDGET_MS`] by more than
//! [`MARGIN`] (the E6 plan's §6a).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "a measuring tool: a panic is a failed run, and its counts are small"
)]

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use laminapdf::{BuildOptions, Builder, Image, PageSize, Pt, RectPt, SaveOptions};
use laminapdf_desktop_protocol::supervisor::{Config, RangeServer, Supervisor};
use laminapdf_desktop_protocol::{
    DocId, EditCall, FromEngine, Priority, TileAppearance, TileFormat, TileRequest, ToEngine, ViewRotation,
};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner, SystemFonts};
use laminapdf_test_fixtures::Rng;

/// The files, by their size in MB: the pages and the side of each page's noise image in pixels
/// (side × side × 3 bytes that Flate can't shrink). The web harness builds the same shapes.
const FILES: [(u32, u32, u32); 3] = [(1, 10, 184), (10, 40, 290), (100, 100, 590)];

/// Fresh opens of each file, one edit each: the median is reported.
const RUNS: usize = 3;

/// The budgets for the edit call plus the page drawn again, in ms, by file size (E6 plan §6a).
const BUDGET_MS: [(u32, f64); 3] = [(1, 30.0), (10, 50.0), (100, 250.0)];

/// How far over its budget a median may be before `--gate` fails: a regression of over 20 %.
const MARGIN: f64 = 0.20;

/// The file, read by position.
struct Whole(File);

impl RangeServer for Whole {
    fn read(&self, _: DocId, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len as usize];
        let mut filled = 0;
        while filled < buf.len() {
            let n = read_at(&self.0, &mut buf[filled..], offset + filled as u64)?;
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

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("reload: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Writes `pages` pages of text with one noise image each to `path`, unless it is there.
fn write_file(path: &Path, pages: u32, side: u32) -> Result<(), String> {
    if path.is_file() {
        return Ok(());
    }
    let mut rng = Rng::new(0x1a31_7a00 + u64::from(side));
    let mut builder = Builder::new(BuildOptions::default());
    let mut rgba = vec![255_u8; (side * side * 4) as usize];
    for index in 0..pages {
        for [r, g, b, _] in rgba.as_chunks_mut::<4>().0 {
            let [x, y, z, ..] = rng.next_u64().to_le_bytes();
            (*r, *g, *b) = (x, y, z);
        }
        let image = Image::rgba(side, side, &rgba).map_err(|e| e.to_string())?;
        let mut page = builder.page(PageSize::A4);
        let at = RectPt::from_xywh(Pt::new(56.0), Pt::new(96.0), Pt::new(160.0), Pt::new(160.0));
        page.surface().image(&image, at);
        page.finish().map_err(|e| format!("page {index}: {e}"))?;
    }
    let mut doc = builder.finish().map_err(|e| e.to_string())?;
    let saved = doc.save(&SaveOptions::default()).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, &saved.bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// The page in view drawn at the app's default zoom, as one tile: the first 1,280 × 800 pixels.
fn tile(engine: &Supervisor, doc: DocId) -> Result<(), String> {
    let (reply, _) = engine
        .request(|req| {
            ToEngine::RenderTile(TileRequest {
                req,
                doc,
                page: 0,
                scale: 1.5,
                rotation: ViewRotation::None,
                appearance: TileAppearance::Original,
                x: 0,
                y: 0,
                w: 892,
                h: 800,
                format: TileFormat::Rgba8,
                priority: Priority::Visible,
            })
        })
        .map_err(|e| e.to_string())?;
    match reply {
        FromEngine::Tile { .. } => Ok(()),
        other => Err(format!("the tile: {other:?}")),
    }
}

/// One run on `file`: a new engine, the file opened and the page drawn, then the edit. Returns the
/// ms the edit call took and the ms until the page was drawn again, both from the call.
fn measure(engine_path: &Path, file: &Path) -> Result<(f64, f64), String> {
    let handle = File::open(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let len = handle.metadata().map_err(|e| e.to_string())?.len();
    let spawner = SandboxSpawner::new(
        engine_path.to_path_buf(),
        Vec::new(),
        SandboxConfig::default(),
        |line| {
            eprintln!("engine: {line}");
        },
    );
    let config = Config {
        fonts: Some(Arc::new(SystemFonts::scan())),
        ..Config::default()
    };
    let engine =
        Supervisor::start(Box::new(spawner), Arc::new(Whole(handle)), config, |_, _| {}).map_err(|e| e.to_string())?;
    let doc = DocId(1);
    engine
        .request(|req| ToEngine::OpenDocument { req, doc, len })
        .map_err(|e| e.to_string())?;
    let (pages, _) = engine
        .request(|req| ToEngine::GetPageInfo { req, doc })
        .map_err(|e| e.to_string())?;
    let FromEngine::PageInfo { ids, .. } = pages else {
        return Err("no page info".to_owned());
    };
    tile(&engine, doc)?;
    let log = format!(
        r#"{{"v":1,"ops":[{{"op":"rotatePage","args":{{"page":"{}","rotate":90}}}}]}}"#,
        ids[0]
    );
    let start = Instant::now();
    let (edited, _) = engine
        .request(|req| ToEngine::Edit {
            req,
            doc,
            call: EditCall::Apply { log: log.clone() },
        })
        .map_err(|e| e.to_string())?;
    let call = start.elapsed();
    if !matches!(edited, FromEngine::Edited { .. }) {
        return Err(format!("the edit: {edited:?}"));
    }
    tile(&engine, doc)?;
    let drawn = start.elapsed();
    engine.shutdown();
    Ok((call.as_secs_f64() * 1000.0, drawn.as_secs_f64() * 1000.0))
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn run() -> Result<bool, String> {
    let (engine, out, gate) = args()?;
    let mut history = serde_json::Map::new();
    let mut ok = true;
    let mut table = String::from(
        "| File | Pages | Edit call (ms) | Page drawn again (ms) | Budget (ms) |\n| --- | --- | --- | --- | --- |\n",
    );
    for (mb, pages, side) in FILES {
        let file = out.join(format!("reload-{mb}mb.pdf"));
        write_file(&file, pages, side)?;
        let size = fs::metadata(&file).map_err(|e| e.to_string())?.len();
        let (mut calls, mut drawn) = (Vec::new(), Vec::new());
        for _ in 0..RUNS {
            let (call, shown) = measure(&engine, &file)?;
            calls.push(call);
            drawn.push(shown);
        }
        let (call, shown) = (median(&mut calls), median(&mut drawn));
        let budget = BUDGET_MS.iter().find(|(m, _)| *m == mb).map_or(f64::MAX, |(_, b)| *b);
        let miss = shown > budget * (1.0 + MARGIN);
        ok &= !miss;
        let _ = writeln!(
            table,
            "| {mb} MB ({:.1} MiB) | {pages} | {call:.0} | {shown:.0} | {budget:.0}{} |",
            size as f64 / 1_048_576.0,
            if miss { " **missed**" } else { "" }
        );
        history.insert(
            format!("reload-first-pixels-{mb}mb"),
            serde_json::json!((shown * 1e6).round() as u64),
        );
    }
    print!("{table}");
    fs::write(out.join("reload.md"), &table).map_err(|e| e.to_string())?;
    let json = serde_json::json!({ "history": history });
    fs::write(out.join("reload.json"), format!("{json:#}\n")).map_err(|e| e.to_string())?;
    if gate && !ok {
        eprintln!("reload: a median is over its budget by more than {} %", MARGIN * 100.0);
        return Ok(false);
    }
    Ok(true)
}

/// `--engine PATH`, `--out DIR` (default `target/desktop-reload`) and `--gate`.
fn args() -> Result<(PathBuf, PathBuf, bool), String> {
    let (mut engine, mut out, mut gate) = (None, PathBuf::from("target/desktop-reload"), false);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--gate" => gate = true,
            "--engine" | "--out" => {
                let value = PathBuf::from(args.next().ok_or_else(|| format!("{arg} needs a value"))?);
                if arg == "--engine" {
                    engine = Some(value);
                } else {
                    out = value;
                }
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    Ok((engine.ok_or("--engine is required")?, out, gate))
}
