//! `render-golden`: the render goldens of ADR 0018 tier 8 (todo B5c). `cargo xtask
//! render-golden [--update]` runs the `laminapdf` examples, has `pdfium.py` render them with
//! PDFium, then runs this:
//!
//! ```sh
//! render-golden <pdf-dir> <pdfium-dir> <golden-dir> <report-dir> [--update]
//! ```
//!
//! For every page of every `<pdf-dir>/<example>.pdf`:
//!
//! 1. **Goldens** (72 dpi): PDFium's render (`<pdfium-dir>/72/<example>/<page>.ppm`) and
//!    hayro's are compared with `<golden-dir>/<example>/<page>-pdfium.png` and
//!    `<page>-hayro.png` (strict: [`compare::golden`]). `--update` writes them instead, and
//!    removes goldens of pages that no longer exist.
//! 2. **Parity** (150 dpi): hayro against PDFium with the hayro-parity spike's metric
//!    ([`compare::parity`]); nothing is committed at this resolution.
//!
//! Thresholds, and the known differences that loosen them for one page, are in
//! `<golden-dir>/thresholds.toml`. Every failure writes an expected | actual | diff image to
//! `<report-dir>/<example>/<page>-<check>.png`; the exit status is 1 on any failure.

mod compare;
mod hayro_render;
mod image;
mod thresholds;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use crate::image::Rgb;
use crate::thresholds::{Check, Thresholds};

/// Resolution of the committed goldens: small PNGs for the repository.
const GOLDEN_DPI: u32 = 72;
/// Resolution of the hayro-vs-PDFium parity check.
const PARITY_DPI: u32 = 150;

/// The engines with committed goldens, in file-name order.
const ENGINES: [&str; 2] = ["hayro", "pdfium"];

struct Args {
    pdfs: PathBuf,
    pdfium: PathBuf,
    golden: PathBuf,
    report: PathBuf,
    update: bool,
}

fn parse_args() -> Result<Args> {
    let mut rest: Vec<String> = std::env::args().skip(1).collect();
    let update = rest.iter().any(|a| a == "--update");
    rest.retain(|a| a != "--update");
    let [pdfs, pdfium, golden, report] = rest.as_slice() else {
        bail!("usage: render-golden <pdf-dir> <pdfium-dir> <golden-dir> <report-dir> [--update]");
    };
    Ok(Args {
        pdfs: pdfs.into(),
        pdfium: pdfium.into(),
        golden: golden.into(),
        report: report.into(),
        update,
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("render-golden: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Runs every check; `Ok(false)` when one failed.
fn run() -> Result<bool> {
    let args = parse_args()?;
    let thresholds = Thresholds::load(&args.golden.join("thresholds.toml"))?;
    let mut failures = Vec::new();
    let mut known_used = BTreeSet::new();
    let mut pages = 0;
    let mut examples = BTreeSet::new();
    for pdf in pdfs_in(&args.pdfs)? {
        let example = pdf.file_stem().context("file name")?.to_string_lossy().into_owned();
        let doc = hayro_render::Doc::load(fs::read(&pdf)?).with_context(|| pdf.display().to_string())?;
        let mut written = BTreeSet::new();
        for index in 0..doc.page_count() {
            pages += 1;
            let page = Page {
                example: &example,
                index,
            };
            for engine in ENGINES {
                let render = match engine {
                    "hayro" => doc.render(index, dpi(GOLDEN_DPI))?,
                    _ => image::read_ppm(&page.pdfium_render(&args.pdfium, GOLDEN_DPI))?,
                };
                let name = format!("{index}-{engine}.png");
                let golden = args.golden.join(&example).join(&name);
                written.insert(name);
                let check = Check::Golden(engine);
                let limits = thresholds.for_page(&example, index, &check, &mut known_used);
                if args.update {
                    image::write_png(&golden, &render)?;
                    continue;
                }
                let expected = image::read_png(&golden).with_context(|| {
                    format!("{example} page {index}: no golden; run `cargo xtask render-golden --update`")
                })?;
                let diff = compare::golden(&expected, &render, limits.delta_e);
                if let Some(problem) = limits.violated(diff.as_ref()) {
                    failures.push(format!("{example} page {index} {engine} vs golden: {problem}"));
                    page.report(&args.report, &check, &expected, &render, diff.as_ref())?;
                }
            }
            let hayro = doc.render(index, dpi(PARITY_DPI))?;
            let pdfium = image::read_ppm(&page.pdfium_render(&args.pdfium, PARITY_DPI))?;
            // The engines may round the page size differently by a pixel; compare the common part.
            let (hayro, pdfium) = compare::common_part(&hayro, &pdfium);
            let limits = thresholds.for_page(&example, index, &Check::Parity, &mut known_used);
            let diff = compare::parity(&pdfium, &hayro);
            if let Some(problem) = limits.violated(diff.as_ref()) {
                failures.push(format!(
                    "{example} page {index} hayro vs PDFium at {PARITY_DPI} dpi: {problem}"
                ));
                page.report(&args.report, &Check::Parity, &pdfium, &hayro, diff.as_ref())?;
            }
        }
        failures.extend(stale_goldens(&args.golden.join(&example), &written, args.update)?);
        examples.insert(example);
    }
    for dir in subdirs(&args.golden)? {
        if !examples.contains(&dir) {
            let path = args.golden.join(&dir);
            if args.update {
                fs::remove_dir_all(&path)?;
            } else {
                failures.push(format!(
                    "{}: goldens of an example that no longer exists",
                    path.display()
                ));
            }
        }
    }
    // Even with --update: rewriting the goldens never hides hayro and PDFium disagreeing, nor
    // a known difference that no longer applies.
    failures.extend(thresholds.unused(&known_used));
    if args.update {
        println!(
            "render goldens: wrote {pages} pages × {} engines into {}",
            ENGINES.len(),
            args.golden.display()
        );
    }
    if failures.is_empty() {
        println!(
            "render goldens: {pages} pages match in {} at {GOLDEN_DPI} dpi; hayro and PDFium agree at {PARITY_DPI} dpi",
            ENGINES.join(" and ")
        );
        Ok(true)
    } else {
        eprintln!("render goldens failed (diff images in {}):", args.report.display());
        for failure in &failures {
            eprintln!("  {failure}");
        }
        Ok(false)
    }
}

/// One page of one example.
struct Page<'a> {
    example: &'a str,
    index: usize,
}

impl Page<'_> {
    fn pdfium_render(&self, dir: &Path, dpi: u32) -> PathBuf {
        dir.join(dpi.to_string())
            .join(self.example)
            .join(format!("{}.ppm", self.index))
    }

    /// Writes expected | actual | diff for a failed check.
    fn report(
        &self,
        dir: &Path,
        check: &Check,
        expected: &Rgb,
        actual: &Rgb,
        diff: Option<&compare::Diff>,
    ) -> Result<()> {
        let path = dir
            .join(self.example)
            .join(format!("{}-{}.png", self.index, check.name()));
        image::write_png(
            &path,
            &compare::triptych(expected, actual, diff.map(|d| d.mask.as_slice())),
        )
    }
}

/// `dpi` as the renderer's scale input.
fn dpi(dpi: u32) -> f32 {
    f32::from(u16::try_from(dpi).unwrap_or(u16::MAX))
}

fn pdfs_in(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut pdfs: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        .collect();
    pdfs.sort();
    if pdfs.is_empty() {
        bail!("no PDFs in {}", dir.display());
    }
    Ok(pdfs)
}

fn subdirs(dir: &Path) -> Result<Vec<String>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(dirs)
}

/// Goldens (`*.png`) in `dir` that no page produced: removed with `--update`, failures
/// otherwise. Other entries (`.DS_Store`, folders) are not goldens and are left alone.
fn stale_goldens(dir: &Path, written: &BTreeSet<String>, update: bool) -> Result<Vec<String>> {
    let mut stale = Vec::new();
    if !dir.is_dir() {
        return Ok(stale);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if !path.is_file() || path.extension().is_none_or(|e| e != "png") {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !written.contains(&name) {
            if update {
                fs::remove_file(&path)?;
            } else {
                stale.push(format!("{}: a golden of a page that no longer exists", path.display()));
            }
        }
    }
    Ok(stale)
}
