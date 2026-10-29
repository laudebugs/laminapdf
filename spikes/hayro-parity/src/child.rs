//! `--one`: render one document with one or both engines, page by page, and
//! report on stdout (one tab-separated line per event) so the driver can
//! attribute a crash or a hang to the page and engine that caused it.
//!
//! Protocol (fields after the tag):
//! - `pages  <hayro n|-1> <pdfium n|-1> <hayro load error> <pdfium load error> <encrypted>`
//! - `begin  <page> <hayro|pdfium>` before each render (page 0 = binding,
//!   warm-up and loading of that engine)
//! - `page   <page> <hayro ms> <pdfium ms> <hayro WxH> <pdfium WxH> <hayro error>
//!   <pdfium error> <ssim> <diff %> <class> <hayro warnings> <harness error>`
//!   (empty field = n/a; `class` is the default-threshold class of a compared
//!   page, computed here on the unrounded values)
//! - `done`

use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
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
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(200)
        .collect()
}

/// Joins warnings with `;`, truncating each one rather than the joined string.
fn join_warnings(w: &[String]) -> String {
    w.iter()
        .map(|w| {
            w.chars()
                .filter(|c| !c.is_control() && *c != ';')
                .take(80)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(";")
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

fn warm_up_hayro(dpi: f32) {
    let _ = guarded(|| HayroDoc::load(Arc::new(WARM_UP.to_vec()))?.render(0, dpi).map(|_| ()));
}

fn warm_up_pdfium(pdfium: &pdfium_render::prelude::Pdfium, dpi: f32) {
    let _ = PdfiumDoc::load(pdfium, WARM_UP).and_then(|d| d.render(0, dpi));
}

pub fn run(args: &ChildArgs) -> Result<()> {
    // Quiet the default panic hook: panics are reported through the protocol.
    std::panic::set_hook(Box::new(|_| {}));
    // Read once; hayro shares the buffer, PDFium borrows it.
    let bytes = Arc::new(std::fs::read(&args.file)?);
    let encrypted = bytes.windows(8).any(|w| w == b"/Encrypt");

    // Everything an engine does before its first page (binding, warm-up,
    // loading) runs under `begin 0 <engine>`, so a crash or hang there is
    // blamed on that engine.
    let (mut hayro, mut hayro_load_err) = (None, String::new());
    if args.hayro {
        begin(0, "hayro");
        warm_up_hayro(args.dpi);
        match guarded(|| HayroDoc::load(Arc::clone(&bytes))) {
            Ok(d) => hayro = Some(d),
            Err(e) => hayro_load_err = clean(&e.to_string()),
        }
    }
    let pdfium = if args.pdfium {
        begin(0, "pdfium");
        let p = render::bind_pdfium(&args.pdfium_lib)?;
        warm_up_pdfium(&p, args.dpi);
        Some(p)
    } else {
        None
    };
    let (mut pdoc, mut pdfium_load_err) = (None, String::new());
    if let Some(p) = &pdfium {
        match PdfiumDoc::load(p, bytes.as_slice()) {
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
        encrypted.to_string(),
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
        report_page(args, page, &h, &p);
    }
    emit(&["done".into()]);
    Ok(())
}

fn report_page(args: &ChildArgs, page: usize, h: &Side, p: &Side) {
    let (mut ssim, mut diff, mut class, mut harness) = (String::new(), String::new(), String::new(), String::new());
    if let (Some(hi), Some(pi)) = (&h.img, &p.img) {
        let d = compare::compare(hi, pi);
        class = d
            .as_ref()
            .map_or("fail", |d| compare::DEFAULT.classify(d.ssim, d.diff_pct))
            .to_owned();
        if let Some(d) = &d {
            // Full precision: the driver's sensitivity table re-classifies these.
            ssim = format!("{}", d.ssim);
            diff = format!("{}", d.diff_pct);
        }
        // HAYRO_PARITY_DUMP_ALL=1 writes a triptych for every page (threshold calibration).
        let dump = std::env::var_os("HAYRO_PARITY_DUMP_ALL").is_some() || class == "fail";
        if let (true, Some(dir)) = (dump, &args.fail_dir) {
            let img = compare::triptych(hi, pi, d.as_ref().map(|d| d.mask.as_slice()));
            let written = std::fs::create_dir_all(dir)
                .map_err(anyhow::Error::from)
                .and_then(|()| compare::write_png(&dir.join(format!("p{page:04}.png")), &img));
            if let Err(e) = written {
                harness = clean(&format!("triptych not written: {e}"));
            }
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
        class,
        join_warnings(&h.warnings),
        harness,
    ]);
}
