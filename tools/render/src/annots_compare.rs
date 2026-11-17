//! The appearance parity checks over the rendered matrix ([`crate::annots_matrix`]): each page
//! by each engine against its committed golden at [`GOLDEN_DPI`] (strict, [`compare::golden`]),
//! and pdf.js and hayro each against PDFium at [`PARITY_DPI`] (the hayro-parity spike's metric,
//! [`compare::parity`]), with the limits per engine in `annots-parity.toml`.
//!
//! PDFium's renders come from `pdfium.py` (`<renders>/pdfium/<dpi>/annots-matrix/<page>.ppm`),
//! pdf.js's from `pdfjs-render.mjs` (`<renders>/pdfjs/<dpi>/<page>.ppm`); hayro renders here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::compare::{self, Diff};
use crate::hayro_render;
use crate::image::{self, Rgb};
use crate::thresholds::{Check, EngineThresholds, Limits};

/// Resolution of the committed goldens, as the render goldens'.
pub const GOLDEN_DPI: u32 = 72;
/// Resolution of the engine-against-PDFium checks, as the render goldens' parity.
pub const PARITY_DPI: u32 = 150;
/// The engines, in file-name order; PDFium is the reference of the parity checks.
pub const ENGINES: [&str; 3] = ["hayro", "pdfium", "pdfjs"];
/// The name of the matrix file, which names its PDFium renders and its known differences.
pub const MATRIX: &str = "annots-matrix";

/// Where the inputs are and what to do.
pub struct Run {
    pub matrix: PathBuf,
    pub renders: PathBuf,
    pub golden: PathBuf,
    pub thresholds: PathBuf,
    pub report: PathBuf,
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

/// Runs every check; returns the failures (empty when all pass).
pub fn run(run: &Run) -> Result<Vec<String>> {
    let thresholds = EngineThresholds::load(&run.thresholds)?;
    let doc = hayro_render::Doc::load(std::fs::read(&run.matrix)?).context("the matrix")?;
    let mut failures = Vec::new();
    let mut used = BTreeSet::new();
    let mut written = BTreeSet::new();
    let mut worst: Vec<(String, Worst)> = Vec::new();
    for page in 0..doc.page_count() {
        for engine in ENGINES {
            let render = run.render(&doc, engine, page, GOLDEN_DPI)?;
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
        let pdfium = run.render(&doc, "pdfium", page, PARITY_DPI)?;
        for engine in ["hayro", "pdfjs"] {
            let other = run.render(&doc, engine, page, PARITY_DPI)?;
            // The engines may round the page size differently by a pixel; compare the common part.
            let (reference, other) = compare::common_part(&pdfium, &other);
            let check = Check::ParityOf(engine);
            let limits: Limits = thresholds.for_page(MATRIX, page, &check, &mut used)?;
            let diff = compare::parity(&reference, &other);
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

impl Run {
    /// Page `page` by `engine` at `dpi`.
    fn render(&self, doc: &hayro_render::Doc, engine: &str, page: usize, dpi: u32) -> Result<Rgb> {
        let ppm = |path: PathBuf| image::read_ppm(&path);
        match engine {
            "hayro" => doc.render(page, f32::from(u16::try_from(dpi)?)),
            "pdfium" => ppm(self.ppm_path(&["pdfium", &dpi.to_string(), MATRIX], page)),
            _ => ppm(self.ppm_path(&["pdfjs", &dpi.to_string()], page)),
        }
    }

    fn ppm_path(&self, dirs: &[&str], page: usize) -> PathBuf {
        let mut path = self.renders.clone();
        path.extend(dirs);
        path.join(format!("{page}.ppm"))
    }

    /// Writes expected | actual | diff for a failed check.
    fn triptych(&self, page: usize, check: &Check, expected: &Rgb, actual: &Rgb, diff: Option<&Diff>) -> Result<()> {
        let path: &Path = &self.report.join(format!("{page}-{}.png", check.name()));
        image::write_png(
            path,
            &compare::triptych(expected, actual, diff.map(|d| d.mask.as_slice())),
        )
    }
}
