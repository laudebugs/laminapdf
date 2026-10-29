//! Perceptual comparison of two RGB renders of the same page.
//!
//! Both images are first box-blurred 3×3 (damps sub-pixel glyph placement).
//!
//! - **SSIM** on luma (BT.601) box-downsampled 2×2, 8×8 sliding window at every
//!   pixel (computed with integral images), the usual constants K1 = 0.01,
//!   K2 = 0.03, L = 255. The page score is the mean over all windows.
//! - **Differing pixels**: the share of pixels whose CIE76 ΔE (sRGB → CIELAB,
//!   D65) is above [`DELTA_E`] and that are not explained in *both*
//!   directions: the pixel counts unless image A's colour occurs within
//!   [`DELTA_E`] in B's 3×3 neighbourhood *and* B's colour occurs within
//!   [`DELTA_E`] in A's 3×3 neighbourhood. This is the anti-aliasing tolerance:
//!   glyph edges rasterised a little differently (the two engines use
//!   different rasterisers and stem darkening) don't count; a missing glyph or
//!   1-px line, a wrong colour or a shift of more than a pixel does.
//!
//! The classification thresholds live in [`Thresholds`]; the summary reports
//! the pass rate at three settings so ADR 0004 can choose.

use crate::render::Rgb;

/// ΔE76 above which a pixel counts as different (≈ "clearly visible").
pub const DELTA_E: f32 = 10.0;

pub struct Diff {
    pub ssim: f64,
    /// Percentage (0–100) of pixels with ΔE > [`DELTA_E`].
    pub diff_pct: f64,
    /// Per-pixel mask of differing pixels (for the triage image).
    pub mask: Vec<bool>,
}

#[derive(Clone, Copy)]
pub struct Thresholds {
    pub name: &'static str,
    pub pass_ssim: f64,
    pub pass_diff_pct: f64,
    pub minor_ssim: f64,
    pub minor_diff_pct: f64,
}

/// Calibrated on a hand-checked sample (README "Thresholds"). The todo's starting
/// point (pass: SSIM ≥ 0.98 and ≤ 0.5 %) failed most visually identical text
/// pages, because SSIM on two different rasterisers of the same glyphs lands at
/// 0.95–0.98; the anti-aliasing-tolerant pixel share is the main criterion and
/// SSIM only a guard against broad low-contrast changes.
pub const DEFAULT: Thresholds = Thresholds {
    name: "default",
    pass_ssim: 0.90,
    pass_diff_pct: 0.5,
    minor_ssim: 0.85,
    minor_diff_pct: 2.0,
};
pub const STRICT: Thresholds = Thresholds {
    name: "strict",
    pass_ssim: 0.95,
    pass_diff_pct: 0.1,
    minor_ssim: 0.90,
    minor_diff_pct: 1.0,
};
pub const LENIENT: Thresholds = Thresholds {
    name: "lenient",
    pass_ssim: 0.85,
    pass_diff_pct: 1.0,
    minor_ssim: 0.80,
    minor_diff_pct: 5.0,
};
pub const ALL: [Thresholds; 3] = [STRICT, DEFAULT, LENIENT];

impl Thresholds {
    pub fn classify(&self, ssim: f64, diff_pct: f64) -> &'static str {
        if ssim >= self.pass_ssim && diff_pct <= self.pass_diff_pct {
            "pass"
        } else if ssim >= self.minor_ssim && diff_pct <= self.minor_diff_pct {
            "minor"
        } else {
            "fail"
        }
    }
}

/// Compares two same-sized images; `None` if the sizes differ.
pub fn compare(a: &Rgb, b: &Rgb) -> Option<Diff> {
    if a.width != b.width || a.height != b.height {
        return None;
    }
    let (w, h) = (a.width as usize, a.height as usize);
    let (a, b) = (&blur(a), &blur(b));
    let (la, lb) = (half_luma(a), half_luma(b));
    let ssim = ssim(&la, &lb, w / 2, h / 2);

    let to_lab = |img: &Rgb| -> Vec<[f32; 3]> { img.data.as_chunks::<3>().0.iter().map(lab).collect() };
    let (lab_a, lab_b) = (to_lab(a), to_lab(b));
    // `p` (in one image) has a match within ΔE in the 3×3 neighbourhood of (x, y) in `other`.
    let near = |p: [f32; 3], other: &[[f32; 3]], x: usize, y: usize| -> bool {
        (y.saturating_sub(1)..=(y + 1).min(h - 1))
            .any(|yy| (x.saturating_sub(1)..=(x + 1).min(w - 1)).any(|xx| delta_e(p, other[yy * w + xx]) <= DELTA_E))
    };
    let mut differing = 0usize;
    let mut mask = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let (p, q) = (lab_a[i], lab_b[i]);
            if delta_e(p, q) <= DELTA_E {
                continue;
            }
            // Anti-aliasing tolerance: a pixel is excused only if it is explained in
            // both directions (a's colour occurs near (x, y) in b, and b's colour
            // occurs near (x, y) in a). One unexplained direction makes it count:
            // otherwise a 1-px line missing from one image would be excused,
            // because the missing side's background colour sits right next to it
            // in the other image.
            if !near(p, &lab_b, x, y) || !near(q, &lab_a, x, y) {
                mask[i] = true;
                differing += 1;
            }
        }
    }
    let diff_pct = 100.0 * differing as f64 / (w * h) as f64;
    Some(Diff { ssim, diff_pct, mask })
}

/// 3×3 box blur of the RGB data (edges clamp).
fn blur(img: &Rgb) -> Rgb {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut out = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 3];
            let mut n = 0;
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let o = (yy * w + xx) * 3;
                    for (a, &v) in acc.iter_mut().zip(&img.data[o..o + 3]) {
                        *a += u32::from(v);
                    }
                    n += 1;
                }
            }
            let o = (y * w + x) * 3;
            for (c, a) in out[o..o + 3].iter_mut().zip(acc) {
                *c = (a / n) as u8;
            }
        }
    }
    Rgb {
        width: img.width,
        height: img.height,
        data: out,
    }
}

/// BT.601 luma, box-downsampled 2×2 (SSIM at half resolution damps glyph
/// anti-aliasing and sub-pixel placement noise, which both engines get "right").
fn half_luma(img: &Rgb) -> Vec<f64> {
    let (w, h) = (img.width as usize, img.height as usize);
    let y = |x: usize, yy: usize| {
        let o = (yy * w + x) * 3;
        let p = &img.data[o..o + 3];
        0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2])
    };
    let (hw, hh) = (w / 2, h / 2);
    let mut out = Vec::with_capacity(hw * hh);
    for yy in 0..hh {
        for x in 0..hw {
            out.push((y(2 * x, 2 * yy) + y(2 * x + 1, 2 * yy) + y(2 * x, 2 * yy + 1) + y(2 * x + 1, 2 * yy + 1)) / 4.0);
        }
    }
    out
}

const WIN: usize = 8;

fn ssim(a: &[f64], b: &[f64], w: usize, h: usize) -> f64 {
    if w < WIN || h < WIN {
        // Too small for a window: fall back to exact-match-or-not.
        return if a == b { 1.0 } else { 0.0 };
    }
    // Integral images of a, b, a², b², ab, (w+1)×(h+1).
    let stride = w + 1;
    let mut sums = vec![[0.0f64; 5]; stride * (h + 1)];
    for y in 0..h {
        let mut row = [0.0f64; 5];
        for x in 0..w {
            let (p, q) = (a[y * w + x], b[y * w + x]);
            for (r, v) in row.iter_mut().zip([p, q, p * p, q * q, p * q]) {
                *r += v;
            }
            let above = sums[y * stride + x + 1];
            let cell = &mut sums[(y + 1) * stride + x + 1];
            for i in 0..5 {
                cell[i] = above[i] + row[i];
            }
        }
    }
    let (c1, c2) = ((0.01f64 * 255.0).powi(2), (0.03f64 * 255.0).powi(2));
    let n = (WIN * WIN) as f64;
    let mut total = 0.0;
    let mut count = 0usize;
    for y in 0..=h - WIN {
        for x in 0..=w - WIN {
            let (tl, tr) = (sums[y * stride + x], sums[y * stride + x + WIN]);
            let (bl, br) = (sums[(y + WIN) * stride + x], sums[(y + WIN) * stride + x + WIN]);
            let s: [f64; 5] = std::array::from_fn(|i| br[i] - bl[i] - tr[i] + tl[i]);
            let (ma, mb) = (s[0] / n, s[1] / n);
            let va = (s[2] / n - ma * ma).max(0.0);
            let vb = (s[3] / n - mb * mb).max(0.0);
            let cov = s[4] / n - ma * mb;
            total += ((2.0 * ma * mb + c1) * (2.0 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            count += 1;
        }
    }
    total / count as f64
}

fn lab(p: &[u8; 3]) -> [f32; 3] {
    let lin = |c: u8| {
        let c = f32::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(p[0]), lin(p[1]), lin(p[2]));
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f32| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn delta_e(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// hayro | PDFium | diff (differing pixels red over a faded PDFium render),
/// side by side with a 4 px grey gutter. Heights may differ (size mismatch).
pub fn triptych(hayro: &Rgb, pdfium: &Rgb, mask: Option<&[bool]>) -> Rgb {
    const GAP: u32 = 4;
    let height = hayro.height.max(pdfium.height);
    let width = hayro.width + pdfium.width * 2 + GAP * 2;
    let mut data = vec![128u8; (width * height * 3) as usize];
    let mut blit = |img: &Rgb, x0: u32, f: &dyn Fn(usize, &[u8]) -> [u8; 3]| {
        for y in 0..img.height {
            for x in 0..img.width {
                let i = (y * img.width + x) as usize;
                let px = f(i, &img.data[i * 3..i * 3 + 3]);
                let o = ((y * width + x0 + x) * 3) as usize;
                data[o..o + 3].copy_from_slice(&px);
            }
        }
    };
    blit(hayro, 0, &|_, p| [p[0], p[1], p[2]]);
    blit(pdfium, hayro.width + GAP, &|_, p| [p[0], p[1], p[2]]);
    blit(pdfium, hayro.width + pdfium.width + GAP * 2, &|i, p| {
        if mask.is_some_and(|m| m.get(i).copied().unwrap_or(false)) {
            [230, 0, 0]
        } else {
            let l = (0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2])) as u8;
            let faded = 255 - (255 - l) / 4;
            [faded, faded, faded]
        }
    });
    Rgb { width, height, data }
}

pub fn write_png(path: &std::path::Path, img: &Rgb) -> anyhow::Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, img.width, img.height);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    enc.write_header()?.write_image_data(&img.data)?;
    Ok(())
}
