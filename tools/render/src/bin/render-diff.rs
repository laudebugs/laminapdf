//! `render-diff`: one reader of the nightly hayro-vs-PDFium render diff (todo X3b, ADR 0004 §6).
//! `cargo xtask render-diff` starts it through `tools/xtask/src/runner.rs`, so a crash or a hang
//! costs one file:
//!
//! ```sh
//! render-diff [--engine both|hayro|pdfium] [--dpi 96] [--pages N] [--images <dir>] [--pdfium-dir <dir>]
//!             [--rows <file.jsonl>] <list.tsv> <out.json>
//! ```
//!
//! Every `<name>\t<path>` line of the list is opened with `laminapdf-render`'s hayro adapter and
//! its PDFium adapter (the library in `--pdfium-dir`, else `LAMINA_PDF_PDFIUM_DIR`; `cargo xtask
//! fetch-pdfium`), and each page (the first
//! `N`) is rendered by both at the same DPI with the hayro-parity spike's settings (white
//! background, annotations and form widgets on, the longest side capped at 3000 px by the same
//! rule in both engines), then compared with [`compare::parity`], the spike's metric, and
//! classified ([`classify`]). A page that isn't `pass` gets a PDFium | hayro | diff image in
//! `<images>/`. The report is the runner's format: `{"tool", "version", "reports_repair",
//! "facts": {<name>: {...}}}`.
//!
//! With `--rows`, each file's header (`{"file", "header": true, "page_count", …}`, once both
//! engines opened it) and each page's row are also appended to that file as JSON lines as soon
//! as they are known, so xtask keeps the pages a process finished before it crashed or hung.
//! A panic while rendering a page is that page's error; an I/O error (reading a file, writing an
//! image) is an `io-error` row, and the run goes on.
//!
//! With `--engine hayro` or `--engine pdfium` only that engine renders and nothing is compared:
//! xtask reruns a file that crashed or hung that way, to tell which engine did it.

#[path = "../compare.rs"]
// Its own tests use the golden check too.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "shared with render-golden, which also uses the strict golden check"
    )
)]
mod compare;
#[path = "../image.rs"]
#[expect(
    dead_code,
    reason = "shared with render-golden, which also reads PPM and PNG goldens"
)]
mod image;

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use laminapdf_render::{
    ByteSource, CancelToken, HayroBackend, RenderBackend, RenderDocument, RenderOpenOptions, RenderRequest,
};
use laminapdf_render_pdfium::PdfiumBackend;
use serde_json::{Value, json};

use crate::image::Rgb;

/// The spike's resolution, and the one ADR 0004 §6's criteria are measured at first.
const DEFAULT_DPI: f32 = 96.0;
/// Longest page side in pixels; larger pages are scaled down by the same rule in both engines
/// (the spike's cap).
const MAX_SIDE: f32 = 3000.0;

/// The hayro-parity spike's default thresholds (README "Method", ADR 0004 §6): `pass` when at
/// most 0.5 % of pixels differ and SSIM ≥ 0.90, `minor` within 2 % and SSIM ≥ 0.85, else
/// `fail`. diff % is the criterion and SSIM only a guard.
const PASS: (f64, f64) = (0.5, 0.90);
const MINOR: (f64, f64) = (2.0, 0.85);

/// The class of a page both engines rendered: `pass`, `minor` or `fail` (a size mismatch is
/// `None` for `diff`, and fails).
fn classify(diff: Option<(f64, f64)>) -> &'static str {
    match diff {
        Some((percent, ssim)) if percent <= PASS.0 && ssim >= PASS.1 => "pass",
        Some((percent, ssim)) if percent <= MINOR.0 && ssim >= MINOR.1 => "minor",
        _ => "fail",
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Engines {
    Both,
    Hayro,
    Pdfium,
}

struct Args {
    engines: Engines,
    dpi: f32,
    pages: Option<u32>,
    images: Option<PathBuf>,
    pdfium_dir: Option<PathBuf>,
    rows: Option<PathBuf>,
    list: PathBuf,
    out: PathBuf,
}

const USAGE: &str = "usage: render-diff [--engine both|hayro|pdfium] [--dpi 96] [--pages N] [--images <dir>] \
                     [--pdfium-dir <dir>] [--rows <file.jsonl>] <list.tsv> <out.json>";

fn parse_args(args: &[String]) -> Result<Args> {
    let mut engines = Engines::Both;
    let mut dpi = DEFAULT_DPI;
    let mut pages = None;
    let mut images = None;
    let mut pdfium_dir = None;
    let mut rows = None;
    let mut positional = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().context(USAGE);
        match arg.as_str() {
            "--engine" => {
                engines = match value()?.as_str() {
                    "both" => Engines::Both,
                    "hayro" => Engines::Hayro,
                    "pdfium" => Engines::Pdfium,
                    other => bail!("--engine {other:?}; {USAGE}"),
                };
            }
            "--dpi" => dpi = value()?.parse().context("--dpi")?,
            "--pages" => pages = Some(value()?.parse().context("--pages")?),
            "--images" => images = Some(PathBuf::from(value()?)),
            "--pdfium-dir" => pdfium_dir = Some(PathBuf::from(value()?)),
            "--rows" => rows = Some(PathBuf::from(value()?)),
            _ => positional.push(arg.clone()),
        }
    }
    let [list, out] = positional.as_slice() else {
        bail!(USAGE);
    };
    Ok(Args {
        engines,
        dpi,
        pages,
        images,
        pdfium_dir,
        rows,
        list: list.into(),
        out: out.into(),
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args).and_then(|args| run(&args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("render-diff: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<()> {
    let hayro = (args.engines != Engines::Pdfium).then(HayroBackend::new);
    let pdfium =
        if args.engines == Engines::Hayro {
            None
        } else {
            let bound = match &args.pdfium_dir {
                Some(dir) => PdfiumBackend::bind(dir),
                None => PdfiumBackend::from_env(),
            };
            Some(bound.context(
                "binding PDFium: run `cargo xtask fetch-pdfium`, pass --pdfium-dir or set LAMINA_PDF_PDFIUM_DIR",
            )?)
        };
    let list = fs::read_to_string(&args.list).with_context(|| format!("reading {}", args.list.display()))?;
    let mut rows = match &args.rows {
        Some(path) => Some(Rows(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .with_context(|| format!("opening {}", path.display()))?,
        )),
        None => None,
    };
    let engines = Engine {
        hayro: hayro.as_ref().map(|b| b as &dyn RenderBackend),
        pdfium: pdfium.as_ref().map(|b| b as &dyn RenderBackend),
    };
    let mut facts = serde_json::Map::new();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').context("a list line without a tab")?;
        let fact = match fs::read(path) {
            Ok(bytes) => engines.file(name, bytes, args, rows.as_mut()),
            // Not the engines' doing: its own class, never blamed on either.
            Err(e) => json!({"outcome": "compared", "io_error": format!("reading {path}: {e}"), "pages": []}),
        };
        facts.insert(name.to_owned(), fact);
    }
    let version = format!(
        "hayro {} / PDFium {}",
        HayroBackend::new().version(),
        laminapdf_render_pdfium::PDFIUM_BUILD
    );
    let report = json!({
        "tool": "render-diff",
        "version": version,
        "reports_repair": false,
        "settings": {
            "dpi": args.dpi,
            "max_side": MAX_SIDE,
            "pages": args.pages,
            "pass": {"max_diff_percent": PASS.0, "min_ssim": PASS.1},
            "minor": {"max_diff_percent": MINOR.0, "min_ssim": MINOR.1},
        },
        "facts": facts,
    });
    fs::write(&args.out, serde_json::to_string(&report)? + "\n")
        .with_context(|| format!("writing {}", args.out.display()))
}

/// The `--rows` file: one JSON line per write, appended (lanes run several processes at once,
/// each file in one of them).
struct Rows(fs::File);

impl Rows {
    /// Appends `value` as one line; a failure is reported and the run goes on, since the
    /// report at the end still has every row.
    fn push(&mut self, value: &Value) {
        let line = value.to_string() + "\n";
        if let Err(e) = self.0.write_all(line.as_bytes()) {
            eprintln!("render-diff: writing a row: {e}");
        }
    }
}

/// The engines of one run (either may be off: `--engine`).
struct Engine<'a> {
    hayro: Option<&'a dyn RenderBackend>,
    pdfium: Option<&'a dyn RenderBackend>,
}

/// One engine's view of a file: its document, or why it didn't open.
type Opened = Option<Result<Box<dyn RenderDocument>, String>>;

impl Engine<'_> {
    /// Every page of one file (at most `--pages`), both engines and the comparison.
    fn file(&self, name: &str, bytes: Vec<u8>, args: &Args, mut rows: Option<&mut Rows>) -> Value {
        let source: Arc<dyn ByteSource> = Arc::new(bytes);
        // Before either engine touches the file: how many pages it has by lamina's reading, so
        // a file that crashes, hangs or opens in neither engine still counts once per page.
        let lamina_pages = catch_unwind(AssertUnwindSafe(|| {
            laminapdf_cos::Document::open(Arc::clone(&source), laminapdf_cos::OpenOptions::default())
                .and_then(|doc| doc.page_count())
                .ok()
        }))
        .ok()
        .flatten();
        if let Some(rows) = rows.as_deref_mut() {
            rows.push(&json!({"file": name, "header": true, "lamina_pages": lamina_pages}));
        }
        let open = |backend: Option<&dyn RenderBackend>| -> Opened {
            backend.map(|b| {
                b.open(Arc::clone(&source), &RenderOpenOptions::new())
                    .map_err(|e| format!("{}: {e}", e.code()))
            })
        };
        let (hayro, pdfium) = (open(self.hayro), open(self.pdfium));
        let count = |doc: &Opened| doc.as_ref().and_then(|d| d.as_ref().ok()).map(|d| d.page_count());
        let (hayro_pages, pdfium_pages) = (count(&hayro), count(&pdfium));
        let total = hayro_pages.max(pdfium_pages).unwrap_or(0);
        let pages = args.pages.map_or(total, |cap| total.min(cap));
        let error = |doc: &Opened| doc.as_ref().and_then(|d| d.as_ref().err().cloned());
        let mut fact = json!({
            "outcome": "compared",
            "page_count": total,
            "lamina_pages": lamina_pages,
            "hayro_pages": hayro_pages,
            "pdfium_pages": pdfium_pages,
            "hayro_open_error": error(&hayro),
            "pdfium_open_error": error(&pdfium),
        });
        if let Some(rows) = rows.as_deref_mut() {
            let mut header = fact.clone();
            set(&mut header, "file", json!(name));
            set(&mut header, "header", json!(true));
            rows.push(&header);
        }
        let mut done = Vec::new();
        for index in 0..pages {
            let h = hayro.as_ref().map(|d| guarded_render("hayro", d, index, args.dpi));
            let p = pdfium.as_ref().map(|d| guarded_render("PDFium", d, index, args.dpi));
            let mut row = page_row(name, index, h.as_ref(), p.as_ref(), args.images.as_deref());
            if let Some(rows) = rows.as_deref_mut() {
                set(&mut row, "file", json!(name));
                rows.push(&row);
            }
            done.push(row);
        }
        set(&mut fact, "pages", json!(done));
        fact
    }
}

/// One engine's render of one page: the bitmap and the time, or the error.
struct Rendered {
    image: Result<Rgb, String>,
    ms: f64,
}

/// Page `index` of `doc` at `dpi`, the longest side capped at [`MAX_SIDE`]; a document that
/// didn't open, or has no such page, is an error.
fn render(doc: &Result<Box<dyn RenderDocument>, String>, index: u32, dpi: f32) -> Rendered {
    let started = Instant::now();
    let image = doc.as_ref().map_err(Clone::clone).and_then(|doc| {
        let page = doc.page(index).map_err(|e| format!("{}: {e}", e.code()))?;
        let longest = page.width.max(page.height).max(1.0);
        let scale = (dpi / 72.0).min(MAX_SIDE / longest);
        let request = RenderRequest::page(index, scale);
        let plan = doc.plan(&request).map_err(|e| format!("{}: {e}", e.code()))?;
        let mut rgba = vec![0; plan.byte_len()];
        doc.render(&request, &mut rgba, &CancelToken::new())
            .map_err(|e| format!("{}: {e}", e.code()))?;
        Rgb::new(plan.tile.width, plan.tile.height, over_white(&rgba)).map_err(|e| e.to_string())
    });
    Rendered {
        image,
        ms: started.elapsed().as_secs_f64() * 1000.0,
    }
}

/// Sets `key` on a JSON object (`value` is always one here).
fn set(value: &mut Value, key: &str, to: Value) {
    if let Some(map) = value.as_object_mut() {
        map.insert(key.to_owned(), to);
    }
}

/// [`render`], with a panic turned into that page's error (the hayro adapter catches its own;
/// this covers the rest), so it costs one page instead of the file.
fn guarded_render(engine: &str, doc: &Result<Box<dyn RenderDocument>, String>, index: u32, dpi: f32) -> Rendered {
    catch_unwind(AssertUnwindSafe(|| render(doc, index, dpi))).unwrap_or_else(|_| Rendered {
        image: Err(format!("{engine} panicked rendering the page")),
        ms: 0.0,
    })
}

/// Straight-alpha RGBA laid over white, as opaque RGB (the request's background is already
/// white, so this only matters for a pixel a backend left translucent).
fn over_white(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 4 * 3);
    for &[r, g, b, a] in rgba.as_chunks::<4>().0 {
        let a = u32::from(a);
        out.extend([r, g, b].map(|c| {
            let v = (u32::from(c) * a + 255 * (255 - a) + 127) / 255;
            u8::try_from(v).unwrap_or(u8::MAX)
        }));
    }
    out
}

/// The report row of one page: each engine's size, time and error, the metric and the class.
fn page_row(
    name: &str,
    index: u32,
    hayro: Option<&Rendered>,
    pdfium: Option<&Rendered>,
    images: Option<&Path>,
) -> Value {
    let side = |r: Option<&Rendered>| -> Value {
        r.map_or(Value::Null, |r| match &r.image {
            Ok(img) => json!({"width": img.width, "height": img.height, "ms": r.ms}),
            Err(e) => json!({"error": e, "ms": r.ms}),
        })
    };
    let mut row = BTreeMap::from([
        ("page", json!(index + 1)),
        ("hayro", side(hayro)),
        ("pdfium", side(pdfium)),
    ]);
    let (Some(Rendered { image: h, .. }), Some(Rendered { image: p, .. })) = (hayro, pdfium) else {
        // One engine only (`--engine`): nothing to compare.
        return json!(row);
    };
    let class = match (h, p) {
        (Err(_), Err(_)) => "both-error",
        (Err(_), Ok(_)) => "hayro-error",
        (Ok(_), Err(_)) => "pdfium-error",
        (Ok(hayro), Ok(pdfium)) => {
            let (hayro, pdfium) = compare::common_part(hayro, pdfium);
            let diff = compare::parity(&pdfium, &hayro);
            let metric = diff.as_ref().map(|d| (d.percent, d.ssim));
            let mut class = classify(metric);
            row.insert("diff_percent", json!(metric.map(|m| m.0)));
            row.insert("ssim", json!(metric.map(|m| m.1)));
            if class != "pass"
                && let Some(dir) = images
            {
                let file = image_name(name, index);
                let triptych = compare::triptych(&pdfium, &hayro, diff.as_ref().map(|d| d.mask.as_slice()));
                match image::write_png(&dir.join(&file), &triptych) {
                    Ok(()) => {
                        row.insert("image", json!(file));
                    }
                    // The comparison stands, but the page can't be shown: its own class, never
                    // blamed on an engine.
                    Err(e) => {
                        row.insert("io_error", json!(format!("writing {file}: {e:#}")));
                        row.insert("compared_class", json!(class));
                        class = "io-error";
                    }
                }
            }
            class
        }
    };
    row.insert("class", json!(class));
    json!(row)
}

/// `<name>-<hash>-p<N>.png`: every character outside `[A-Za-z0-9._-]` replaced by `_`, and
/// the FNV-1a hash of the full name (low 32 bits), so names that flatten alike stay apart.
fn image_name(name: &str, index: u32) -> String {
    let hash = name.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe}-{:08x}-p{}.png", hash & 0xffff_ffff, index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_should_pass_within_half_a_percent_and_ssim_090() {
        assert_eq!(classify(Some((0.5, 0.90))), "pass");
    }

    #[test]
    fn classify_should_call_two_percent_minor() {
        assert_eq!(classify(Some((2.0, 0.95))), "minor");
    }

    #[test]
    fn classify_should_let_ssim_guard_a_clean_diff() {
        assert_eq!(classify(Some((0.0, 0.86))), "minor");
    }

    #[test]
    fn classify_should_fail_above_two_percent() {
        assert_eq!(classify(Some((2.01, 0.99))), "fail");
    }

    #[test]
    fn classify_should_fail_a_size_mismatch() {
        assert_eq!(classify(None), "fail");
    }

    #[test]
    fn over_white_should_keep_opaque_pixels_and_whiten_transparent_ones() {
        assert_eq!(over_white(&[10, 20, 30, 255, 0, 0, 0, 0]), [10, 20, 30, 255, 255, 255]);
    }

    #[test]
    fn image_name_should_flatten_paths() {
        assert!(image_name("pdfjs/test/pdfs/a b.pdf", 0).starts_with("pdfjs_test_pdfs_a_b.pdf-"));
    }

    #[test]
    fn image_name_should_keep_names_that_flatten_alike_apart() {
        assert_ne!(image_name("pdfjs/a b.pdf", 0), image_name("pdfjs/a_b.pdf", 0));
    }

    #[test]
    fn parse_args_should_read_the_engine_and_the_page_cap() {
        let args: Vec<String> = ["--engine", "hayro", "--pages", "3", "l.tsv", "o.json"]
            .map(String::from)
            .to_vec();
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.engines == Engines::Hayro && parsed.pages == Some(3) && parsed.out == Path::new("o.json"));
    }
}
