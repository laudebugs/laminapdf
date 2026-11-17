//! The appearance parity checks over the rendered matrix ([`crate::annots_matrix`]): each page
//! by each engine against its committed golden at [`GOLDEN_DPI`] (strict, [`compare::golden`]),
//! and pdf.js and hayro each against PDFium at [`PARITY_DPI`] (the hayro-parity spike's metric,
//! [`compare::parity`]), with the limits per engine in `annots-parity.toml`. And
//! [`compare_saved`], the corpus edit test's check that a saved file draws as its original did.
//!
//! PDFium's renders come from `pdfium.py` (`<renders>/pdfium/<dpi>/<stem>/<page>.ppm`), pdf.js's
//! from `pdfjs-render.mjs` (`<renders>/pdfjs/<dpi>/<stem>/<page>.ppm`); hayro renders here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::compare::{self, Diff};
use crate::hayro_render;
use crate::image::{self, Rgb};
use crate::settings::{GOLDEN_DPI, PARITY_DPI, as_scale};
use crate::thresholds::{Check, EngineThresholds};

/// The engines, in file-name order; PDFium is the reference of the parity checks.
pub const ENGINES: [&str; 3] = ["hayro", "pdfium", "pdfjs"];
/// The matrix file's stem, which names its renders and its known differences.
pub const MATRIX: &str = "annots-matrix";

/// Where the inputs are and what to do.
pub struct Run {
    /// The matrix PDF.
    pub matrix: PathBuf,
    /// The folder holding PDFium's and pdf.js's renders.
    pub renders: PathBuf,
    /// The committed goldens.
    pub golden: PathBuf,
    /// `annots-parity.toml`.
    pub thresholds: PathBuf,
    /// Where diff images go.
    pub report: PathBuf,
    /// Rewrite the goldens instead of comparing with them.
    pub update: bool,
}

/// The worst result of one check over every page: what its limits must allow.
#[derive(Default)]
struct Worst {
    differing: usize,
    percent: f64,
    ssim: Option<f64>,
}

impl Worst {
    fn add(&mut self, diff: Option<&Diff>) {
        if let Some(diff) = diff {
            self.differing = self.differing.max(diff.differing);
            self.percent = self.percent.max(diff.percent);
            self.ssim = Some(self.ssim.map_or(diff.ssim, |s| s.min(diff.ssim)));
        }
    }
}

/// Runs every check; returns the failures (empty when all pass). Prints each parity check's
/// result per page: the figures `annots-parity.toml`'s limits are set from.
///
/// # Errors
/// A missing render, golden or thresholds file.
pub fn run(run: &Run) -> Result<Vec<String>> {
    let thresholds = EngineThresholds::load(&run.thresholds)?;
    let doc = hayro_render::Doc::load(std::fs::read(&run.matrix)?).context("the matrix")?;
    let mut failures = Vec::new();
    let mut used = BTreeSet::new();
    let mut written = BTreeSet::new();
    let mut worst: Vec<(String, Worst)> = Vec::new();
    for page in 0..doc.page_count() {
        for engine in ENGINES {
            let render = render(&doc, &run.renders, MATRIX, engine, page, GOLDEN_DPI)?;
            let name = format!("{page}-{engine}.png");
            let golden = run.golden.join(&name);
            written.insert(name);
            let check = Check::Golden(engine);
            let limits = thresholds.for_page(MATRIX, page, &check, &mut used)?;
            if run.update {
                image::write_png(&golden, &render)?;
                continue;
            }
            let expected = image::read_png(&golden).with_context(|| {
                format!("page {page}: no {engine} golden; run `cargo xtask annots-parity --update`")
            })?;
            let diff = compare::golden(&expected, &render, limits.delta_e);
            record(&mut worst, &check, diff.as_ref());
            if let Some(problem) = limits.violated(diff.as_ref()) {
                failures.push(format!("page {page} {engine} vs golden: {problem}"));
                run.triptych(page, &check, &expected, &render, diff.as_ref())?;
            }
        }
        let pdfium = render(&doc, &run.renders, MATRIX, "pdfium", page, PARITY_DPI)?;
        for engine in ["hayro", "pdfjs"] {
            let other = render(&doc, &run.renders, MATRIX, engine, page, PARITY_DPI)?;
            // The engines may round the page size differently by a pixel; compare the common part.
            let (reference, other) = compare::common_part(&pdfium, &other);
            let check = Check::ParityOf(engine);
            let limits = thresholds.for_page(MATRIX, page, &check, &mut used)?;
            let diff = compare::parity(&reference, &other);
            if let Some(d) = &diff {
                println!("  page {page} {}: {:.3} %, SSIM {:.4}", check.name(), d.percent, d.ssim);
            }
            record(&mut worst, &check, diff.as_ref());
            if let Some(problem) = limits.violated(diff.as_ref()) {
                failures.push(format!("page {page} {engine} vs PDFium at {PARITY_DPI} dpi: {problem}"));
                run.triptych(page, &check, &reference, &other, diff.as_ref())?;
            }
        }
    }
    failures.extend(image::stale_goldens(&run.golden, &written, run.update)?);
    // Even with --update: rewriting the goldens never hides an engine disagreeing with PDFium,
    // nor a known difference that no longer applies.
    failures.extend(thresholds.unused(&used));
    for (check, w) in &worst {
        println!(
            "annots-parity: {check}: at worst {} differing pixels ({:.3} %), SSIM {:.4}",
            w.differing,
            w.percent,
            w.ssim.unwrap_or(1.0)
        );
    }
    Ok(failures)
}

fn record(worst: &mut Vec<(String, Worst)>, check: &Check, diff: Option<&Diff>) {
    let name = check.name();
    if let Some((_, w)) = worst.iter_mut().find(|(n, _)| *n == name) {
        w.add(diff);
    } else {
        let mut w = Worst::default();
        w.add(diff);
        worst.push((name, w));
    }
}

/// Page `page` of the file `stem` by `engine` at `dpi`: hayro renders `doc`; PDFium's and
/// pdf.js's renders are read from `renders`.
fn render(doc: &hayro_render::Doc, renders: &Path, stem: &str, engine: &str, page: usize, dpi: u32) -> Result<Rgb> {
    let ppm = || {
        image::read_ppm(
            &renders
                .join(engine)
                .join(dpi.to_string())
                .join(stem)
                .join(format!("{page}.ppm")),
        )
    };
    match engine {
        "hayro" => doc.render(page, as_scale(dpi)),
        "pdfium" | "pdfjs" => ppm(),
        other => bail!("no renderer named {other:?}"),
    }
}

impl Run {
    /// Writes expected | actual | diff for a failed check.
    fn triptych(&self, page: usize, check: &Check, expected: &Rgb, actual: &Rgb, diff: Option<&Diff>) -> Result<()> {
        let path = self.report.join(format!("{page}-{}.png", check.name()));
        image::write_png(
            &path,
            &compare::triptych(expected, actual, diff.map(|d| d.mask.as_slice())),
        )
    }
}

/// The corpus edit test's render check of one file: `<dir>/original.pdf` and `<dir>/saved.pdf`
/// (the original with every annotation it could move moved by 1 pt), drawn by each engine at
/// [`GOLDEN_DPI`] (PDFium's and pdf.js's renders under `<dir>`, laid out as in [`run`]), must
/// have the same pages, of the same sizes, within the `[corpus]` limits of `annots-parity.toml`.
/// A 1 pt move changes only the marks' edges, so a saved file that draws blank, loses a page or
/// breaks one fails. Returns the problems (empty when the file passes).
///
/// # Errors
/// A missing file or render of the original, or an unreadable thresholds file.
pub fn compare_saved(dir: &Path, thresholds: &Path) -> Result<Vec<String>> {
    let limits = EngineThresholds::load(thresholds)?.corpus();
    let original = hayro_render::Doc::load(std::fs::read(dir.join("original.pdf"))?).context("hayro: original")?;
    let saved = hayro_render::Doc::load(std::fs::read(dir.join("saved.pdf"))?).context("hayro: saved")?;
    if original.page_count() != saved.page_count() {
        return Ok(vec![format!(
            "hayro counts {} pages, {} before the save",
            saved.page_count(),
            original.page_count()
        )]);
    }
    let mut problems = Vec::new();
    for engine in ENGINES {
        for page in 0..original.page_count() {
            let before = render(&original, dir, "original", engine, page, GOLDEN_DPI)
                .with_context(|| format!("{engine}: page {page} of the original"))?;
            let after = match render(&saved, dir, "saved", engine, page, GOLDEN_DPI) {
                Ok(after) => after,
                Err(e) => {
                    problems.push(format!("{engine}: page {page} of the saved file: {e:#}"));
                    continue;
                }
            };
            let diff = compare::parity(&before, &after);
            if let Some(d) = &diff {
                println!("  {engine} page {page}: {:.3} %, SSIM {:.4}", d.percent, d.ssim);
            }
            if let Some(problem) = limits.violated(diff.as_ref()) {
                problems.push(format!(
                    "{engine}: page {page} draws differently after the move: {problem}"
                ));
            }
        }
        // The page count above is hayro's: an engine that drew the saved file with more pages
        // than the original shows it in how many pictures it left.
        if engine != "hayro" {
            let drawn = |stem: &str| drawn_pages(&dir.join(engine).join(GOLDEN_DPI.to_string()).join(stem));
            let (before, after) = (drawn("original")?, drawn("saved")?);
            if before != after {
                problems.push(format!(
                    "{engine}: drew {before} pages of the original and {after} of the saved file"
                ));
            }
        }
    }
    Ok(problems)
}

/// How many pages an engine drew into `dir`: its `.ppm` files.
fn drawn_pages(dir: &Path) -> Result<usize> {
    let entries = std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    let mut count = 0;
    for entry in entries {
        if entry?.path().extension().is_some_and(|e| e == "ppm") {
            count += 1;
        }
    }
    Ok(count)
}
