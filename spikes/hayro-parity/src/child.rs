//! `--one`: render one document with one or both engines, page by page, and
//! report on stdout (one tab-separated line per event) so the driver can
//! attribute a crash or a hang to the page and engine that caused it.
//!
//! Protocol (fields after the tag):
//! - `pages  <hayro n|-1> <pdfium n|-1> <hayro load error> <pdfium load error>`
//! - `begin  <page> <hayro|pdfium>` before each render (page 0 = loading)
//! - `page   <page> <hayro ms> <pdfium ms> <hayro WxH> <pdfium WxH> <hayro error>
//!   <pdfium error> <ssim> <diff %> <hayro warnings>` (empty field = n/a)
//! - `done`

use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Result, anyhow};

use crate::compare;
use crate::render::{self, HayroDoc, PdfiumDoc, Rgb};

pub struct ChildArgs {
    pub file: PathBuf,
    pub dpi: f32,
    pub start: usize,
    pub hayro: bool,
    pub pdfium: bool,
    pub pdfium_lib: PathBuf,
    /// Where to write a hayro | PDFium | diff triptych for each `fail` page.
    pub fail_dir: Option<PathBuf>,
}

fn clean(s: &str) -> String {
    let s: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    s.chars().take(200).collect()
}

fn emit(fields: &[String]) {
    let mut out = std::io::stdout().lock();
    // A closed pipe means the driver gave up on us; nothing useful to do then.
    let _ = writeln!(out, "{}", fields.join("\t"));
    let _ = out.flush();
}

fn begin(page: usize, engine: &str) {
    emit(&["begin".into(), page.to_string(), engine.into()]);
}

/// Runs `f`, turning a panic into an error.
fn guarded<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(p) => {
            let msg = p
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            Err(anyhow!("panic: {msg}"))
        }
    }
}

struct Side {
    ms: Option<f64>,
    img: Option<Rgb>,
    err: String,
    warnings: Vec<String>,
}

impl Side {
    fn none(err: &str) -> Self {
        Self {
            ms: None,
            img: None,
            err: err.into(),
            warnings: Vec::new(),
        }
    }
    fn done(ms: f64, r: Result<Rgb>, warnings: Vec<String>) -> Self {
        match r {
            Ok(img) => Self {
                ms: Some(ms),
                img: Some(img),
                err: String::new(),
                warnings,
            },
            Err(e) => Self {
                ms: Some(ms),
                img: None,
                err: clean(&e.to_string()),
                warnings,
            },
        }
    }
    fn size(&self) -> String {
        self.img
            .as_ref()
            .map(|i| format!("{}x{}", i.width, i.height))
            .unwrap_or_default()
    }
}

fn timed<T>(f: impl FnOnce() -> Result<T>) -> (f64, Result<T>) {
    let t = Instant::now();
    let r = guarded(f);
    (t.elapsed().as_secs_f64() * 1000.0, r)
}

/// A one-page Helvetica document rendered once, untimed, before the real work.
/// PDFium's first text render initialises its system font mapper (≈ 0.6 s on
/// macOS), a per-process cost that would otherwise land on page 1 of every
/// document; hayro gets the same treatment (it parses its embedded fonts lazily).
const WARM_UP: &[u8] = include_bytes!("../../../fixtures/corpus/hello.pdf");

fn warm_up(args: &ChildArgs, pdfium: Option<&pdfium_render::prelude::Pdfium>) {
    if args.hayro {
        let _ = guarded(|| HayroDoc::load(WARM_UP.to_vec())?.render(0, args.dpi).map(|_| ()));
    }
    if let Some(p) = pdfium {
        let _ = PdfiumDoc::load(p, WARM_UP).and_then(|d| d.render(0, args.dpi));
    }
}

pub fn run(args: &ChildArgs) -> Result<()> {
    // Quiet the default panic hook: panics are reported through the protocol.
    std::panic::set_hook(Box::new(|_| {}));
    let bytes = std::fs::read(&args.file)?;

    let (mut hayro, mut hayro_load_err) = (None, String::new());
    if args.hayro {
        begin(0, "hayro");
        match guarded(|| HayroDoc::load(bytes.clone())) {
            Ok(d) => hayro = Some(d),
            Err(e) => hayro_load_err = clean(&e.to_string()),
        }
    }
    let pdfium = if args.pdfium {
        Some(render::bind_pdfium(&args.pdfium_lib)?)
    } else {
        None
    };
    warm_up(args, pdfium.as_ref());
    let (mut pdoc, mut pdfium_load_err) = (None, String::new());
    if let Some(p) = &pdfium {
        begin(0, "pdfium");
        match PdfiumDoc::load(p, &bytes) {
            Ok(d) => pdoc = Some(d),
            Err(e) => pdfium_load_err = clean(&e.to_string()),
        }
    }
    let hn = hayro.as_ref().map(|d| guarded(|| Ok(d.page_count())).unwrap_or(0));
    let pn = pdoc.as_ref().map(PdfiumDoc::page_count);
    let count = |n: Option<usize>| n.map_or("-1".to_owned(), |n| n.to_string());
    emit(&[
        "pages".into(),
        count(hn),
        count(pn),
        hayro_load_err.clone(),
        pdfium_load_err.clone(),
    ]);
    if hn == Some(0) && hayro_load_err.is_empty() {
        hayro_load_err = "no pages".into();
        hayro = None;
    }
    if pn == Some(0) && pdfium_load_err.is_empty() {
        pdfium_load_err = "no pages".into();
        pdoc = None;
    }

    let pages = hn.unwrap_or(0).max(pn.unwrap_or(0));
    for page in args.start.max(1)..=pages {
        let idx = page - 1;
        let h = match &hayro {
            _ if !args.hayro => Side::none(""),
            None => Side::none(&format!("load: {hayro_load_err}")),
            Some(d) => {
                begin(page, "hayro");
                let (ms, r) = timed(|| d.render(idx, args.dpi));
                match r {
                    Ok((img, w)) => Side::done(ms, Ok(img), w),
                    Err(e) => Side::done(ms, Err(e), Vec::new()),
                }
            }
        };
        let p = match &pdoc {
            _ if !args.pdfium => Side::none(""),
            None => Side::none(&format!("load: {pdfium_load_err}")),
            Some(d) => {
                begin(page, "pdfium");
                let (ms, r) = timed(|| d.render(idx, args.dpi));
                Side::done(ms, r, Vec::new())
            }
        };
        report_page(args, page, &h, &p)?;
    }
    emit(&["done".into()]);
    Ok(())
}

fn report_page(args: &ChildArgs, page: usize, h: &Side, p: &Side) -> Result<()> {
    let mut ssim = String::new();
    let mut diff = String::new();
    if let (Some(hi), Some(pi)) = (&h.img, &p.img) {
        let d = compare::compare(hi, pi);
        if let Some(d) = &d {
            ssim = format!("{:.5}", d.ssim);
            diff = format!("{:.4}", d.diff_pct);
        }
        // HAYRO_PARITY_DUMP_ALL=1 writes a triptych for every page (threshold calibration).
        let failed = std::env::var_os("HAYRO_PARITY_DUMP_ALL").is_some()
            || d.as_ref()
                .is_none_or(|d| compare::DEFAULT.classify(d.ssim, d.diff_pct) == "fail");
        if let (true, Some(dir)) = (failed, &args.fail_dir) {
            std::fs::create_dir_all(dir)?;
            let img = compare::triptych(hi, pi, d.as_ref().map(|d| d.mask.as_slice()));
            compare::write_png(&dir.join(format!("p{page:04}.png")), &img)?;
        }
    }
    let ms = |m: Option<f64>| m.map(|m| format!("{m:.2}")).unwrap_or_default();
    emit(&[
        "page".into(),
        page.to_string(),
        ms(h.ms),
        ms(p.ms),
        h.size(),
        p.size(),
        h.err.clone(),
        p.err.clone(),
        ssim,
        diff,
        clean(&h.warnings.join(";")),
    ]);
    Ok(())
}
