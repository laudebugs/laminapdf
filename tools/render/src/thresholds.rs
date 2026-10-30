//! `fixtures/render-golden/thresholds.toml`: the limits of each check, and the known
//! differences that loosen them for one page, each with its reason (a list of exceptions
//! rather than looser global thresholds).

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::compare::Diff;

/// Which comparison: a render against its golden (per engine), or hayro against PDFium.
pub enum Check {
    Golden(&'static str),
    Parity,
}

impl Check {
    /// `golden-hayro`, `golden-pdfium` or `parity`: the name in `thresholds.toml` and in diff
    /// image names.
    pub fn name(&self) -> String {
        match self {
            Self::Golden(engine) => format!("golden-{engine}"),
            Self::Parity => "parity".to_owned(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenLimits {
    max_diff_pixels: usize,
    delta_e: f32,
    min_ssim: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParityLimits {
    max_diff_percent: f64,
    min_ssim: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    example: String,
    page: usize,
    check: String,
    max_diff_pixels: Option<usize>,
    max_diff_percent: Option<f64>,
    min_ssim: Option<f64>,
    #[expect(
        dead_code,
        reason = "documentation for reviewers; required so every exception says why"
    )]
    reason: String,
}

/// The parsed file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    golden: GoldenLimits,
    parity: ParityLimits,
    #[serde(default)]
    known: Vec<Known>,
}

/// The limits one check of one page must meet.
pub struct Limits {
    pub delta_e: f32,
    max_diff_pixels: Option<usize>,
    max_diff_percent: Option<f64>,
    min_ssim: f64,
}

impl Limits {
    /// What `diff` breaks (`None` for a size mismatch), or `None` when it is within limits.
    pub fn violated(&self, diff: Option<&Diff>) -> Option<String> {
        let Some(diff) = diff else {
            return Some("the page sizes differ".to_owned());
        };
        let too_many = self.max_diff_pixels.is_some_and(|max| diff.differing > max)
            || self.max_diff_percent.is_some_and(|max| diff.percent > max);
        (too_many || diff.ssim < self.min_ssim).then(|| {
            format!(
                "{} differing pixels ({:.3} %), SSIM {:.4} (limits: {}, SSIM ≥ {})",
                diff.differing,
                diff.percent,
                diff.ssim,
                match (self.max_diff_pixels, self.max_diff_percent) {
                    (Some(n), _) => format!("≤ {n} pixels"),
                    (None, Some(p)) => format!("≤ {p} %"),
                    (None, None) => "none".to_owned(),
                },
                self.min_ssim
            )
        })
    }
}

impl Thresholds {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// The limits for `check` of `example`'s page `page`; records which known difference
    /// applied in `used`.
    pub fn for_page(&self, example: &str, page: usize, check: &Check, used: &mut BTreeSet<usize>) -> Limits {
        let mut limits = match check {
            Check::Golden(_) => Limits {
                delta_e: self.golden.delta_e,
                max_diff_pixels: Some(self.golden.max_diff_pixels),
                max_diff_percent: None,
                min_ssim: self.golden.min_ssim,
            },
            Check::Parity => Limits {
                delta_e: 0.0,
                max_diff_pixels: None,
                max_diff_percent: Some(self.parity.max_diff_percent),
                min_ssim: self.parity.min_ssim,
            },
        };
        let name = check.name();
        for (i, known) in self.known.iter().enumerate() {
            if known.example == example && known.page == page && known.check == name {
                used.insert(i);
                limits.max_diff_pixels = known.max_diff_pixels.or(limits.max_diff_pixels);
                limits.max_diff_percent = known.max_diff_percent.or(limits.max_diff_percent);
                limits.min_ssim = known.min_ssim.unwrap_or(limits.min_ssim);
            }
        }
        limits
    }

    /// Known differences no page used: stale entries fail, so the list stays true.
    pub fn unused(&self, used: &BTreeSet<usize>) -> Vec<String> {
        self.known
            .iter()
            .enumerate()
            .filter(|(i, _)| !used.contains(i))
            .map(|(_, k)| {
                format!(
                    "thresholds.toml: known difference {} page {} {} matches no page",
                    k.example, k.page, k.check
                )
            })
            .collect()
    }
}
