//! The two comparisons of the render goldens.
//!
//! - **Golden** (a render against its committed PNG, the same engine): every pixel whose
//!   CIE76 ΔE exceeds `delta_e` counts, with no blur and no neighbourhood, so a deliberate
//!   one-pixel-wide change shows; SSIM is a guard. The same engine at the same version is
//!   deterministic, so the default allows no differing pixel.
//! - **Parity** (hayro against PDFium at 150 dpi): the hayro-parity spike's metric
//!   (`spikes/hayro-parity/src/compare.rs`, README "Method"): both renders box-blurred 3×3; a
//!   pixel counts when its ΔE exceeds 10 and it is not explained in both directions by the
//!   other render's 3×3 neighbourhood (tolerates anti-aliasing and 1-px placement, not a
//!   missing glyph or line); SSIM on half-resolution BT.601 luma with 8×8 windows.
#![expect(
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    reason = "image arithmetic over buffers whose sizes `compare` checks once up front; \
              means of u8 values stay in 0..=255; the names are the formulas' (SSIM, CIELAB)"
)]

use std::borrow::Cow;

use crate::image::Rgb;

/// ΔE76 above which a parity pixel counts (≈ "clearly visible", the spike's value).
const PARITY_DELTA_E: f32 = 10.0;

/// What a comparison found.
#[derive(Debug, Clone)]
pub struct Diff {
    /// Mean SSIM over the page.
    pub ssim: f64,
    /// Pixels that count as different.
    pub differing: usize,
    /// Their share of the page, 0–100.
    pub percent: f64,
    /// Which pixels (for the diff image).
    pub mask: Vec<bool>,
}

/// Strict comparison of a render with its golden; `None` when the sizes differ.
pub fn golden(a: &Rgb, b: &Rgb, delta_e: f32) -> Option<Diff> {
    if (a.width, a.height) != (b.width, b.height) {
        return None;
    }
    let (w, h) = (a.width as usize, a.height as usize);
    let pixels = |img: &Rgb| -> Vec<[f32; 3]> { img.data.as_chunks::<3>().0.iter().copied().map(lab).collect() };
    let mask: Vec<bool> = pixels(a)
        .into_iter()
        .zip(pixels(b))
        .map(|(p, q)| delta_e76(p, q) > delta_e)
        .collect();
    Some(finish(mask, ssim(&half_luma(a), &half_luma(b), w / 2, h / 2)))
}

/// The spike's anti-aliasing-tolerant comparison of two engines; `None` when the sizes
/// differ.
pub fn parity(a: &Rgb, b: &Rgb) -> Option<Diff> {
    if (a.width, a.height) != (b.width, b.height) {
        return None;
    }
    let (w, h) = (a.width as usize, a.height as usize);
    let (a, b) = (&blur(a), &blur(b));
    let ssim = ssim(&half_luma(a), &half_luma(b), w / 2, h / 2);
    let to_lab = |img: &Rgb| -> Vec<[f32; 3]> { img.data.as_chunks::<3>().0.iter().copied().map(lab).collect() };
    let (lab_a, lab_b) = (to_lab(a), to_lab(b));
    // `p` (from one render) occurs within ΔE in the 3×3 neighbourhood of (x, y) in `other`.
    let near = |p: [f32; 3], other: &[[f32; 3]], x: usize, y: usize| -> bool {
        (y.saturating_sub(1)..=(y + 1).min(h - 1)).any(|yy| {
            (x.saturating_sub(1)..=(x + 1).min(w - 1)).any(|xx| delta_e76(p, other[yy * w + xx]) <= PARITY_DELTA_E)
        })
    };
    let mut mask = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let (p, q) = (lab_a[i], lab_b[i]);
            // Excused only when explained in both directions: otherwise a 1-px line missing
            // from one render would be excused by the background right next to it.
            mask[i] = delta_e76(p, q) > PARITY_DELTA_E && (!near(p, &lab_b, x, y) || !near(q, &lab_a, x, y));
        }
    }
    Some(finish(mask, ssim))
}

/// What two engines' renders of one page should be compared on: when their sizes differ by at
/// most one pixel per side (each engine floors the page size from its own page box), the common
/// top-left part; otherwise both as they are, which [`parity`] reports as a size mismatch.
/// Borrowed unless a crop is needed.
pub fn common_part<'a>(a: &'a Rgb, b: &'a Rgb) -> (Cow<'a, Rgb>, Cow<'a, Rgb>) {
    let same = (a.width, a.height) == (b.width, b.height);
    let near = a.width.abs_diff(b.width) <= 1 && a.height.abs_diff(b.height) <= 1;
    if same || !near {
        return (Cow::Borrowed(a), Cow::Borrowed(b));
    }
    let (w, h) = (a.width.min(b.width), a.height.min(b.height));
    (Cow::Owned(a.crop(w, h)), Cow::Owned(b.crop(w, h)))
}

fn finish(mask: Vec<bool>, ssim: f64) -> Diff {
    let differing = mask.iter().filter(|&&m| m).count();
    let percent = 100.0 * differing as f64 / mask.len().max(1) as f64;
    Diff {
        ssim,
        differing,
        percent,
        mask,
    }
}

/// 3×3 box blur (edges clamp).
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

/// BT.601 luma, box-downsampled 2×2.
fn half_luma(img: &Rgb) -> Vec<f64> {
    let (w, h) = (img.width as usize, img.height as usize);
    let luma = |x: usize, y: usize| {
        let o = (y * w + x) * 3;
        let p = &img.data[o..o + 3];
        0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2])
    };
    let (hw, hh) = (w / 2, h / 2);
    let mut out = Vec::with_capacity(hw * hh);
    for y in 0..hh {
        for x in 0..hw {
            out.push(
                (luma(2 * x, 2 * y) + luma(2 * x + 1, 2 * y) + luma(2 * x, 2 * y + 1) + luma(2 * x + 1, 2 * y + 1))
                    / 4.0,
            );
        }
    }
    out
}

const WIN: usize = 8;

/// Mean SSIM over every 8×8 window (integral images), K1 = 0.01, K2 = 0.03, L = 255.
fn ssim(a: &[f64], b: &[f64], w: usize, h: usize) -> f64 {
    if w < WIN || h < WIN {
        return if a == b { 1.0 } else { 0.0 };
    }
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

/// sRGB → CIELAB (D65).
fn lab(p: [u8; 3]) -> [f32; 3] {
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
        if t > 0.008_856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn delta_e76(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// expected | actual | diff (differing pixels red over a faded `actual`), side by side with
/// a 4 px grey gutter; heights may differ (a size mismatch).
pub fn triptych(expected: &Rgb, actual: &Rgb, mask: Option<&[bool]>) -> Rgb {
    const GAP: u32 = 4;
    let height = expected.height.max(actual.height);
    let width = expected.width + actual.width * 2 + GAP * 2;
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
    blit(expected, 0, &|_, p| [p[0], p[1], p[2]]);
    blit(actual, expected.width + GAP, &|_, p| [p[0], p[1], p[2]]);
    blit(actual, expected.width + actual.width + GAP * 2, &|i, p| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn page(w: u32, h: u32) -> Rgb {
        Rgb::new(w, h, vec![255; (w * h * 3) as usize]).unwrap()
    }

    fn with_vertical_line(mut img: Rgb, x: u32) -> Rgb {
        for y in 4..img.height - 4 {
            let o = ((y * img.width + x) * 3) as usize;
            img.data[o..o + 3].copy_from_slice(&[0, 0, 0]);
        }
        img
    }

    #[test]
    fn golden_counts_a_one_pixel_line() {
        let diff = golden(&page(64, 64), &with_vertical_line(page(64, 64), 20), 2.3).unwrap();
        assert_eq!(diff.differing, 56);
    }

    #[test]
    fn golden_of_identical_renders_is_clean() {
        let diff = golden(&page(32, 32), &page(32, 32), 2.3).unwrap();
        assert_eq!((diff.differing, diff.ssim), (0, 1.0));
    }

    #[test]
    fn parity_counts_a_missing_one_pixel_line() {
        let diff = parity(&page(64, 64), &with_vertical_line(page(64, 64), 20)).unwrap();
        assert!(diff.differing > 0);
    }

    #[test]
    fn parity_excuses_a_one_pixel_shift() {
        let a = with_vertical_line(page(64, 64), 20);
        let b = with_vertical_line(page(64, 64), 21);
        assert_eq!(parity(&a, &b).unwrap().differing, 0);
    }

    #[test]
    fn common_part_should_crop_a_one_pixel_difference() {
        let (a, b) = (page(10, 11), page(11, 11));
        let (a, b) = common_part(&a, &b);
        assert_eq!((a.width, b.width, a.height, b.height), (10, 10, 11, 11));
    }

    #[test]
    fn common_part_should_borrow_equal_sizes() {
        let (a, b) = (page(8, 8), page(8, 8));
        assert!(matches!(common_part(&a, &b), (Cow::Borrowed(_), Cow::Borrowed(_))));
    }

    #[test]
    fn common_part_should_leave_a_real_size_mismatch() {
        let (a, b) = (page(8, 8), page(8, 12));
        let (a, b) = common_part(&a, &b);
        assert!(parity(&a, &b).is_none());
    }

    #[test]
    fn different_sizes_do_not_compare() {
        assert!(golden(&page(8, 8), &page(8, 9), 2.3).is_none());
    }
}
