//! `render-diff`: one reader of the nightly hayro-vs-PDFium render diff (todo X3b, ADR 0004 §6).
//! `cargo xtask render-diff` starts it through `tools/xtask/src/runner.rs`, so a crash or a hang
//! costs one file:
//!
//! ```sh
//! render-diff [--engine both|hayro|pdfium] [--dpi 96] [--pages N] [--images <dir>] [--pdfium-dir <dir>] <list.tsv> <out.json>
//! ```
//!
//! Every `<name>\t<path>` line of the list is opened with `lamina-render`'s hayro adapter and
//! its PDFium adapter (the library in `--pdfium-dir`, else `LAMINA_PDFIUM_DIR`; `cargo xtask
//! fetch-pdfium`), and each page (the first
//! `N`) is rendered by both at the same DPI with the hayro-parity spike's settings (white
//! background, annotations and form widgets on, the longest side capped at 3000 px by the same
//! rule in both engines), then compared with [`compare::parity`], the spike's metric, and
//! classified ([`classify`]). A page that isn't `pass` gets a PDFium | hayro | diff image in
//! `<images>/`. The report is the runner's format: `{"tool", "version", "reports_repair",
//! "facts": {<name>: {...}}}`.
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
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use lamina_render::{
    ByteSource, CancelToken, HayroBackend, RenderBackend, RenderDocument, RenderOpenOptions, RenderRequest,
};
use lamina_render_pdfium::PdfiumBackend;
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
    list: PathBuf,
    out: PathBuf,
}

const USAGE: &str = "usage: render-diff [--engine both|hayro|pdfium] [--dpi 96] [--pages N] [--images <dir>] \
                     [--pdfium-dir <dir>] <list.tsv> <out.json>";

fn parse_args(args: &[String]) -> Result<Args> {
    let mut engines = Engines::Both;
    let mut dpi = DEFAULT_DPI;
    let mut pages = None;
    let mut images = None;
    let mut pdfium_dir = None;
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
                "binding PDFium: run `cargo xtask fetch-pdfium`, pass --pdfium-dir or set LAMINA_PDFIUM_DIR",
            )?)
        };
    let list = fs::read_to_string(&args.list).with_context(|| format!("reading {}", args.list.display()))?;
    let mut facts = serde_json::Map::new();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').context("a list line without a tab")?;
        let bytes = fs::read(path).with_context(|| format!("reading {path}"))?;
        let engines = Engine {
            hayro: hayro.as_ref().map(|b| b as &dyn RenderBackend),
            pdfium: pdfium.as_ref().map(|b| b as &dyn RenderBackend),
        };
        facts.insert(name.to_owned(), engines.file(name, bytes, args)?);
    }
    let version = format!(
        "hayro {} / PDFium {}",
        HayroBackend::new().version(),
        lamina_render_pdfium::PDFIUM_BUILD
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

/// The engines of one run (either may be off: `--engine`).
struct Engine<'a> {
    hayro: Option<&'a dyn RenderBackend>,
    pdfium: Option<&'a dyn RenderBackend>,
}

/// One engine's view of a file: its document, or why it didn't open.
type Opened = Option<Result<Box<dyn RenderDocument>, String>>;

impl Engine<'_> {
    /// Every page of one file (at most `--pages`), both engines and the comparison.
    fn file(&self, name: &str, bytes: Vec<u8>, args: &Args) -> Result<Value> {
        let source: Arc<dyn ByteSource> = Arc::new(bytes);
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
        let mut rows = Vec::new();
        for index in 0..pages {
            let h = hayro.as_ref().map(|d| render(d, index, args.dpi));
            let p = pdfium.as_ref().map(|d| render(d, index, args.dpi));
            rows.push(page_row(name, index, h.as_ref(), p.as_ref(), args.images.as_deref())?);
        }
        let error = |doc: &Opened| doc.as_ref().and_then(|d| d.as_ref().err().cloned());
        Ok(json!({
            "outcome": "compared",
            "page_count": total,
            "hayro_pages": hayro_pages,
            "pdfium_pages": pdfium_pages,
            "hayro_open_error": error(&hayro),
            "pdfium_open_error": error(&pdfium),
            "pages": rows,
        }))
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
) -> Result<Value> {
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
        return Ok(json!(row));
    };
    let class = match (h, p) {
        (Err(_), Err(_)) => "both-error",
        (Err(_), Ok(_)) => "hayro-error",
        (Ok(_), Err(_)) => "pdfium-error",
        (Ok(hayro), Ok(pdfium)) => {
            let (hayro, pdfium) = common_part(hayro, pdfium);
            let diff = compare::parity(&pdfium, &hayro);
            let metric = diff.as_ref().map(|d| (d.percent, d.ssim));
            let class = classify(metric);
            row.insert("diff_percent", json!(metric.map(|m| m.0)));
            row.insert("ssim", json!(metric.map(|m| m.1)));
            if class != "pass"
                && let Some(dir) = images
            {
                let file = image_name(name, index);
                image::write_png(
                    &dir.join(&file),
                    &compare::triptych(&pdfium, &hayro, diff.as_ref().map(|d| d.mask.as_slice())),
                )?;
                row.insert("image", json!(file));
            }
            class
        }
    };
    row.insert("class", json!(class));
    Ok(json!(row))
}

/// The engines may floor the page size differently by a pixel (each from its own page box, as
/// in the spike); then the common part is compared, as `render-golden`'s parity check does. A
/// larger difference stays, and `compare::parity` reports it as a size mismatch.
fn common_part(hayro: &Rgb, pdfium: &Rgb) -> (Rgb, Rgb) {
    let near = hayro.width.abs_diff(pdfium.width) <= 1 && hayro.height.abs_diff(pdfium.height) <= 1;
    if near {
        let (w, h) = (hayro.width.min(pdfium.width), hayro.height.min(pdfium.height));
        (hayro.crop(w, h), pdfium.crop(w, h))
    } else {
        (hayro.clone(), pdfium.clone())
    }
}

/// `<name>-p<N>.png` with every character outside `[A-Za-z0-9._-]` replaced by `_`.
fn image_name(name: &str, index: u32) -> String {
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
    format!("{safe}-p{}.png", index + 1)
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
    fn common_part_should_crop_a_one_pixel_difference() {
        let a = Rgb::new(10, 11, vec![255; 10 * 11 * 3]).unwrap();
        let b = Rgb::new(11, 11, vec![255; 11 * 11 * 3]).unwrap();
        let (a, b) = common_part(&a, &b);
        assert_eq!((a.width, b.width, a.height, b.height), (10, 10, 11, 11));
    }

    #[test]
    fn image_name_should_flatten_paths() {
        assert_eq!(
            image_name("pdfjs/test/pdfs/a b.pdf", 0),
            "pdfjs_test_pdfs_a_b.pdf-p1.png"
        );
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
