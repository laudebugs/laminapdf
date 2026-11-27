//! Times what the desktop does to show an edit to the page in view: the engine writes one appended
//! revision, PDFium (the pinned native build) opens the file with it, and the page is drawn again.
//! The engine runs as the app runs it, under the main process's supervisor, with the document
//! read by position from its file.
//!
//! ```sh
//! cargo xtask desktop-reload [--gate]   # fetches PDFium, runs this, checks the figures
//! # by hand, with PDFium in LAMINA_PDF_PDFIUM_DIR (`cargo xtask fetch-pdfium`):
//! cargo bench -p laminapdf-desktop-engine --bench reload -- [--out target/desktop-reload]
//! ```
//!
//! The files are the ones the web harness measures (`packages/viewer/perf/reload.json`: about 1, 10
//! and 100 MB of text pages with one image of seeded noise each). They are written under `--out`
//! with a recipe stamp, never committed, and written again when the recipe or the size is off.
//! Each file is opened afresh `RUNS` times in a new engine process, and the figure is the median of
//! the time from the edit call until the page in view is drawn again; the desktop has no quick
//! pass, so that is what the web calls the new pixels. The medians go to
//! `<out>/desktop-reload.json`, and the table to `<out>/reload.md`.
//!
//! The edit turns the page a quarter: the engine is built without the builder's appearance writer
//! and refuses `addContentObject` (`appearance-unavailable`). A turn writes the same appended
//! revision and reopens PDFium the same way.
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

/// The files and the baselines, shared with the web harness and the gate.
const SHAPES: &str = include_str!("../../../packages/viewer/perf/reload.json");

/// Fresh opens of each file, one edit each: the median is reported.
const RUNS: usize = 3;

/// Bumped when the way the files are written changes, so files written by the old way are replaced.
const RECIPE: &str = "v1";

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
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("reload: {message}");
            ExitCode::FAILURE
        }
    }
}

/// One file to measure, from `reload.json`.
struct Shape {
    megabytes: u64,
    pages: u32,
    side: u32,
}

/// Writes the file for `shape` to `path` unless a file written by this recipe, of about the size
/// the shape promises, is there already.
fn write_file(path: &Path, shape: &Shape) -> Result<(), String> {
    let stamp = path.with_extension("recipe");
    let recipe = format!("{RECIPE} {} {}", shape.pages, shape.side);
    // A page's noise image is side × side × 3 bytes that Flate can't shrink.
    let expected = u64::from(shape.pages) * u64::from(shape.side) * u64::from(shape.side) * 3;
    let complete = fs::metadata(path).is_ok_and(|m| m.len() >= expected);
    if complete && fs::read_to_string(&stamp).is_ok_and(|text| text == recipe) {
        return Ok(());
    }
    let mut rng = Rng::new(0x1a31_7a00 + u64::from(shape.side));
    let mut builder = Builder::new(BuildOptions::default());
    let side = shape.side;
    let mut rgba = vec![255_u8; (side * side * 4) as usize];
    for index in 0..shape.pages {
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
    if (saved.bytes.len() as u64) < expected {
        return Err(format!(
            "{}: {} bytes, fewer than the images hold",
            path.display(),
            saved.bytes.len()
        ));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, &saved.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    fs::write(&stamp, recipe).map_err(|e| format!("{}: {e}", stamp.display()))
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

fn run() -> Result<(), String> {
    let out = out_dir()?;
    let engine = PathBuf::from(env!("CARGO_BIN_EXE_laminapdf-desktop-engine"));
    let shapes: serde_json::Value = serde_json::from_str(SHAPES).map_err(|e| e.to_string())?;
    let files = shapes["files"].as_array().ok_or("reload.json has no files")?;
    let mut medians = serde_json::Map::new();
    let mut table =
        String::from("| File | Pages | Edit call (ms) | Page drawn again (ms) |\n| --- | --- | --- | --- |\n");
    for file in files {
        let shape = Shape {
            megabytes: file["megabytes"].as_u64().ok_or("a file has no megabytes")?,
            pages: file["pages"].as_u64().ok_or("a file has no pages")? as u32,
            side: file["side"].as_u64().ok_or("a file has no side")? as u32,
        };
        let path = out.join(format!("reload-{}mb.pdf", shape.megabytes));
        write_file(&path, &shape)?;
        let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
        let (mut calls, mut drawn) = (Vec::new(), Vec::new());
        for _ in 0..RUNS {
            let (call, shown) = measure(&engine, &path)?;
            calls.push(call);
            drawn.push(shown);
        }
        let (call, shown) = (median(&mut calls), median(&mut drawn));
        let _ = writeln!(
            table,
            "| {} MB ({:.1} MiB) | {} | {call:.0} | {shown:.0} |",
            shape.megabytes,
            size as f64 / 1_048_576.0,
            shape.pages,
        );
        medians.insert(shape.megabytes.to_string(), serde_json::json!(shown));
    }
    print!("{table}");
    fs::write(out.join("reload.md"), &table).map_err(|e| e.to_string())?;
    let json = serde_json::json!({ "medianMs": medians });
    fs::write(out.join("desktop-reload.json"), format!("{json:#}\n")).map_err(|e| e.to_string())?;
    Ok(())
}

/// `--out DIR` (default `target/desktop-reload`); `cargo bench` also passes `--bench`.
fn out_dir() -> Result<PathBuf, String> {
    let mut out = PathBuf::from("target/desktop-reload");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bench" => {}
            "--out" => out = PathBuf::from(args.next().ok_or("--out needs a value")?),
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    Ok(out)
}
