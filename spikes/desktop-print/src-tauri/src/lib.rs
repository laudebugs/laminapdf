//! Spike D1b: three ways to print a PDF from a Tauri app, on each OS.
//!
//! - **A, webview:** pages rendered by PDFium at 300 dpi into a print-only DOM with named
//!   `@page` sizes (as the web reader's C6c print frame does), printed by the webview.
//! - **B, native raster:** PDFium rasters at the printer's resolution, drawn through the OS
//!   print API (AppKit, GDI, GTK).
//! - **C, native vector:** PDFKit (macOS), PDFium's `FPDF_RenderPage` onto the printer DC
//!   (Windows), CUPS with the original bytes (Linux).
//!
//! `LAMINA_SPIKE_MODE`: `interactive` (default: buttons, the OS dialogs, for a physical print),
//! `auto` (every fixture through every path into PDF files, no dialog, then exit) or
//! `dialog-<path>` (open that path's dialog for a screenshot; the script closes the app).

pub mod pdf;
mod probe;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use tauri::http::Response as HttpResponse;
use tauri::{AppHandle, Emitter, Manager, State};

pub use pdf::PageSize;

/// The pages (1-based, inclusive) the next job prints; `None` is every page. The automatic run
/// sets it for its page-range job, as a user would in the dialog.
static PAGE_RANGE: Mutex<Option<(usize, usize)>> = Mutex::new(None);

pub fn page_range() -> Option<(usize, usize)> {
    *PAGE_RANGE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The print-only DOM's resolution for path A (C6c's print dpi).
const WEBVIEW_DPI: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PrintPath {
    Webview,
    Raster,
    Vector,
}

impl PrintPath {
    const ALL: [Self; 3] = [Self::Raster, Self::Vector, Self::Webview];

    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == s)
    }

    fn name(self) -> &'static str {
        match self {
            Self::Webview => "webview",
            Self::Raster => "raster",
            Self::Vector => "vector",
        }
    }
}

pub struct Fixture {
    pub name: &'static str,
    pub path: PathBuf,
    pub bytes: Arc<Vec<u8>>,
    pub sizes: Vec<PageSize>,
}

struct App {
    fixtures: Vec<Fixture>,
    results: PathBuf,
    label: String,
    mode: String,
    /// Path A: the webview says its print DOM is ready (or why it failed).
    ready_tx: Mutex<Option<Sender<Result<(), String>>>>,
    probe: probe::Probe,
}

impl App {
    fn fixture(&self, name: &str) -> Result<&Fixture, String> {
        self.fixtures.iter().find(|f| f.name == name).ok_or_else(|| format!("no fixture {name}"))
    }
}

#[tauri::command]
fn plan(app: State<'_, App>) -> Value {
    let fixtures: Vec<Value> =
        app.fixtures.iter().map(|f| json!({ "name": f.name, "pages": f.sizes })).collect();
    json!({ "label": app.label, "mode": app.mode, "dpi": WEBVIEW_DPI, "fixtures": fixtures })
}

/// Interactive paths B and C: the OS dialog, then the job. Runs off the IPC thread.
#[tauri::command]
async fn print_native_raster(handle: AppHandle, fixture: String) -> Result<(), String> {
    run_in_background(handle, fixture, PrintPath::Raster).await
}

#[tauri::command]
async fn print_native_vector(handle: AppHandle, fixture: String) -> Result<(), String> {
    run_in_background(handle, fixture, PrintPath::Vector).await
}

/// Interactive path A from Rust (the webview's own print API, with its dialog). The page's
/// `window.print()` button is the other half of path A.
#[tauri::command]
async fn print_webview(handle: AppHandle, fixture: String) -> Result<(), String> {
    run_in_background(handle, fixture, PrintPath::Webview).await
}

async fn run_in_background(handle: AppHandle, fixture: String, path: PrintPath) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || job(&handle, &fixture, path, None).map(|_| ()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn webview_ready(app: State<'_, App>, error: Option<String>) {
    if let Some(tx) = app.ready_tx.lock().unwrap_or_else(PoisonError::into_inner).take() {
        let _ = tx.send(error.map_or(Ok(()), Err));
    }
}

#[tauri::command]
fn report(msg: String) {
    println!("[webview] {msg}");
}

/// What one print job cost.
#[derive(Default, Serialize)]
struct Metrics {
    prep_ms: u128,
    spool_ms: u128,
    main_rss_peak_mb: f64,
    web_rss_peak_mb: f64,
    out_bytes: u64,
}

/// Path A's first half: the webview builds its print DOM for `fixture` and says when every
/// image has decoded.
fn prepare_webview(handle: &AppHandle, fixture: &str) -> Result<(), String> {
    let rx: Receiver<Result<(), String>> = {
        let (tx, rx) = channel();
        *handle.state::<App>().ready_tx.lock().unwrap_or_else(PoisonError::into_inner) = Some(tx);
        rx
    };
    // The page may not be listening yet (a job right after launch): ask until it answers.
    let deadline = Instant::now() + Duration::from_secs(600);
    while Instant::now() < deadline {
        handle.emit("prepare", fixture).map_err(|e| e.to_string())?;
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(result) => return result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("the webview never built its print DOM".to_owned())
}

/// One print job. `out`: write a PDF there with no dialog; `None`: show the OS dialog.
fn job(handle: &AppHandle, name: &str, path: PrintPath, out: Option<&Path>) -> Result<Metrics, String> {
    let app = handle.state::<App>();
    let fixture = app.fixture(name)?;
    let mut m = Metrics::default();
    app.probe.start();
    let started = Instant::now();
    if path == PrintPath::Webview {
        prepare_webview(handle, name)?;
        m.prep_ms = started.elapsed().as_millis();
    }
    let spool = Instant::now();
    let result = match path {
        PrintPath::Raster => platform::print_raster(handle, fixture, out),
        PrintPath::Vector => platform::print_vector(handle, fixture, out),
        PrintPath::Webview => platform::print_webview(handle, fixture, out),
    };
    if let (Ok(()), Some(out)) = (&result, out) {
        // Some spoolers finish writing after the API returns.
        m.out_bytes = wait_for_file(out, Duration::from_secs(300))?;
    }
    m.spool_ms = spool.elapsed().as_millis();
    let sample = app.probe.stop();
    m.main_rss_peak_mb = sample.main_rss_peak_mb;
    m.web_rss_peak_mb = sample.web_rss_peak_mb;
    result.map(|()| m)
}

/// Waits until `path` exists and its size has held for half a second; returns the size.
pub fn wait_for_file(path: &Path, timeout: Duration) -> Result<u64, String> {
    let deadline = Instant::now() + timeout;
    let (mut last, mut since) = (u64::MAX, Instant::now());
    while Instant::now() < deadline {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > 0 && size == last && since.elapsed() >= Duration::from_millis(500) {
            return Ok(size);
        }
        if size != last {
            (last, since) = (size, Instant::now());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!("{} was not written in time", path.display()))
}

fn append_row(path: &Path, row: &[(&str, String)]) {
    let new = !path.exists();
    let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) else { return };
    let mut text = String::new();
    if new {
        text += &row.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(",");
        text.push('\n');
    }
    text += &row.iter().map(|(_, v)| v.replace(',', ";").replace('\n', " ")).collect::<Vec<_>>().join(",");
    text.push('\n');
    let _ = f.write_all(text.as_bytes());
}

/// `auto`: every fixture through every path into `results/out/`, one CSV row per job.
fn drive_auto(handle: &AppHandle) {
    let app = handle.state::<App>();
    let out_dir = app.results.join("out");
    let _ = std::fs::create_dir_all(&out_dir);
    let csv = app.results.join(format!("{}-jobs.csv", app.label));
    // Path A on the 50-page fixture can take minutes and gigabytes, so it runs last.
    let mut jobs: Vec<(PrintPath, &str)> =
        PrintPath::ALL.into_iter().flat_map(|p| pdf::FIXTURES.into_iter().map(move |f| (p, f))).collect();
    jobs.sort_by_key(|&(p, f)| p == PrintPath::Webview && f == "mixed-50");
    let only = std::env::var("LAMINA_SPIKE_ONLY").ok();
    for (path, fixture) in jobs {
        if only.as_deref().is_some_and(|o| o != path.name()) {
            continue;
        }
        {
            let out = out_dir.join(format!("{}-{}-{fixture}.pdf", app.label, path.name()));
            let _ = std::fs::remove_file(&out);
            println!("[auto] {} {fixture}", path.name());
            let (ok, error, m) = match job(handle, fixture, path, Some(&out)) {
                Ok(m) => (true, String::new(), m),
                Err(e) => (false, e, Metrics::default()),
            };
            if !ok {
                println!("[auto]   failed: {error}");
            }
            append_row(&csv, &[
                ("label", app.label.clone()),
                ("path", path.name().into()),
                ("fixture", fixture.into()),
                ("ok", ok.to_string()),
                ("prep_ms", m.prep_ms.to_string()),
                ("spool_ms", m.spool_ms.to_string()),
                ("main_rss_peak_mb", format!("{:.0}", m.main_rss_peak_mb)),
                ("web_rss_peak_mb", format!("{:.0}", m.web_rss_peak_mb)),
                ("out_bytes", m.out_bytes.to_string()),
                ("error", error),
            ]);
        }
    }
    // The page-range check: pages 3–7 of the 50-page fixture through path B, no dialog.
    if only.as_deref().is_none_or(|o| o == "raster") {
        let (first, last) = (3, 7);
        let fixture = "mixed-50";
        let out = out_dir.join(format!("{}-raster-{fixture}@{first}-{last}.pdf", app.label));
        let _ = std::fs::remove_file(&out);
        println!("[auto] raster {fixture} pages {first}-{last}");
        *PAGE_RANGE.lock().unwrap_or_else(PoisonError::into_inner) = Some((first, last));
        let result = job(handle, fixture, PrintPath::Raster, Some(&out));
        *PAGE_RANGE.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let (ok, error, m) = match result {
            Ok(m) => (true, String::new(), m),
            Err(e) => (false, e, Metrics::default()),
        };
        append_row(&csv, &[
            ("label", app.label.clone()),
            ("path", "raster".into()),
            ("fixture", format!("{fixture}@{first}-{last}")),
            ("ok", ok.to_string()),
            ("prep_ms", "0".into()),
            ("spool_ms", m.spool_ms.to_string()),
            ("main_rss_peak_mb", format!("{:.0}", m.main_rss_peak_mb)),
            ("web_rss_peak_mb", format!("{:.0}", m.web_rss_peak_mb)),
            ("out_bytes", m.out_bytes.to_string()),
            ("error", error),
        ]);
    }
    handle.exit(0);
}

/// `lamina://localhost/page/{fixture}/{index}` (`http://lamina.localhost/…` on Windows): one page
/// as a PNG at the print-DOM resolution.
fn page_png(handle: &AppHandle, uri_path: &str) -> Result<Vec<u8>, String> {
    let parts: Vec<&str> = uri_path.trim_start_matches('/').split('/').collect();
    let (Some(&"page"), Some(name), Some(index)) = (parts.first(), parts.get(1), parts.get(2)) else {
        return Err(format!("bad path {uri_path}"));
    };
    let index: u16 = index.parse().map_err(|_| "bad page index")?;
    let app = handle.state::<App>();
    let doc = pdf::open(&app.fixture(name)?.bytes)?;
    let raster = pdf::render(&doc, index, WEBVIEW_DPI)?;
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, raster.width, raster.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_compression(png::Compression::Fastest);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&raster.rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::disallowed_types, reason = "tauri::generate_context! builds a HashMap on some targets")]
pub fn run() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let results = std::env::var_os("LAMINA_SPIKE_RESULTS").map_or_else(|| root.join("../results"), PathBuf::from);
    let label = std::env::var("LAMINA_SPIKE_LABEL").unwrap_or_else(|_| std::env::consts::OS.to_owned());
    let mode = std::env::var("LAMINA_SPIKE_MODE").unwrap_or_else(|_| "interactive".to_owned());
    let fixtures = match load_fixtures(&root.join("../fixtures")) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("desktop-print: {e}");
            std::process::exit(2);
        }
    };
    let _ = std::fs::create_dir_all(&results);
    let start_mode = mode.clone();
    let result = tauri::Builder::default()
        .manage(App {
            fixtures,
            results,
            label,
            mode,
            ready_tx: Mutex::new(None),
            probe: probe::Probe::default(),
        })
        .register_asynchronous_uri_scheme_protocol("lamina", |ctx, request, responder| {
            let handle = ctx.app_handle().clone();
            let path = request.uri().path().to_owned();
            tauri::async_runtime::spawn_blocking(move || {
                let response = match page_png(&handle, &path) {
                    Ok(body) => HttpResponse::builder()
                        .header("Content-Type", "image/png")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(body),
                    Err(e) => HttpResponse::builder().status(500).body(e.into_bytes()),
                };
                responder.respond(response.unwrap_or_else(|_| HttpResponse::new(Vec::new())));
            });
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            if start_mode == "auto" {
                std::thread::spawn(move || drive_auto(&handle));
            } else if let Some(path) = start_mode.strip_prefix("dialog-").and_then(PrintPath::parse) {
                // The script screenshots the dialog and closes the app.
                std::thread::spawn(move || {
                    if let Err(e) = job(&handle, "mixed-50", path, None) {
                        eprintln!("desktop-print: {e}");
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            plan,
            print_webview,
            print_native_raster,
            print_native_vector,
            webview_ready,
            report
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("desktop-print: {e}");
        std::process::exit(1);
    }
}

fn load_fixtures(dir: &Path) -> Result<Vec<Fixture>, String> {
    pdf::FIXTURES
        .into_iter()
        .map(|name| {
            let path = dir.join(format!("{name}.pdf"));
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let sizes = pdf::page_sizes(&bytes)?;
            Ok(Fixture { name, path, bytes: Arc::new(bytes), sizes })
        })
        .collect()
}
