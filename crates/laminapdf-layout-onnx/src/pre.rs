//! The page image as Heron reads it: 640 × 640, planar RGB (all reds, then greens, then
//! blues), 8 bits per channel. The model's graph does its own scaling and normalising, so the
//! bytes go in as they are.

use laminapdf_extract::PageRaster;

use crate::SIDE;

/// One output pixel's inputs along an axis: the first input index and each input's weight.
struct Taps {
    start: usize,
    weights: Vec<f32>,
}

/// Pillow's bilinear filter along one axis: a triangle whose width grows with the reduction,
/// so a page shrunk to 640 pixels averages its strokes instead of skipping them.
fn taps(from: u32, to: u32) -> Vec<Taps> {
    let scale = f64::from(from) / f64::from(to);
    let support = scale.max(1.0);
    (0..to)
        .map(|o| {
            let centre = (f64::from(o) + 0.5) * scale;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..from"
            )]
            let (lo, hi) = (
                (centre - support).floor().clamp(0.0, f64::from(from)) as usize,
                (centre + support).ceil().clamp(0.0, f64::from(from)) as usize,
            );
            #[expect(clippy::cast_precision_loss, reason = "pixel indices are far below 2^52")]
            let raw: Vec<f64> = (lo..hi)
                .map(|i| (1.0 - ((i as f64 + 0.5 - centre) / support).abs()).max(0.0))
                .collect();
            let sum: f64 = raw.iter().sum();
            #[expect(clippy::cast_possible_truncation, reason = "weights between 0 and 1")]
            let weights = raw
                .iter()
                .map(|w| if sum > 0.0 { (w / sum) as f32 } else { 0.0 })
                .collect();
            Taps { start: lo, weights }
        })
        .collect()
}

/// `raster` resized to 640 × 640 by Pillow's bilinear filter, over white: a transparent pixel
/// reads as paper, as on screen.
pub(crate) fn chw_640(raster: &PageRaster) -> Vec<u8> {
    let side = usize::try_from(SIDE).unwrap_or(640);
    let (w, h) = (raster.width_px.max(1), raster.height_px.max(1));
    let width = usize::try_from(w).unwrap_or(1);
    let height = usize::try_from(h).unwrap_or(1);
    // Straight RGBA over white, as three planes of floats.
    let mut planes = vec![vec![255.0_f32; width * height]; 3];
    for (i, [r, g, b, a]) in raster.pixels.as_chunks::<4>().0.iter().take(width * height).enumerate() {
        let alpha = f32::from(*a) / 255.0;
        for (plane, value) in planes.iter_mut().zip([r, g, b]) {
            if let Some(v) = plane.get_mut(i) {
                *v = f32::from(*value) * alpha + 255.0 * (1.0 - alpha);
            }
        }
    }
    let (across, down) = (taps(w, SIDE), taps(h, SIDE));
    let mut out = vec![0_u8; 3 * side * side];
    let mut row = vec![0.0_f32; side * height];
    for (c, plane) in planes.iter().enumerate() {
        // Across first, into `row` (640 wide, the input's height), then down.
        for y in 0..height {
            for (x, t) in across.iter().enumerate() {
                let sum: f32 = t
                    .weights
                    .iter()
                    .enumerate()
                    .map(|(k, wt)| wt * plane.get(y * width + t.start + k).copied().unwrap_or(255.0))
                    .sum();
                if let Some(slot) = row.get_mut(y * side + x) {
                    *slot = sum;
                }
            }
        }
        for (y, t) in down.iter().enumerate() {
            for x in 0..side {
                let sum: f32 = t
                    .weights
                    .iter()
                    .enumerate()
                    .map(|(k, wt)| wt * row.get((t.start + k) * side + x).copied().unwrap_or(255.0))
                    .sum();
                if let Some(slot) = out.get_mut(c * side * side + y * side + x) {
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "clamped to 0..=255"
                    )]
                    {
                        *slot = sum.round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use laminapdf_extract::PageRaster;
    use laminapdf_model::{Pt, RectPt};

    use super::chw_640;

    fn raster(w: u32, h: u32, rgba: [u8; 4]) -> PageRaster {
        let pixels = rgba.repeat(usize::try_from(w * h).unwrap());
        PageRaster::new(
            w,
            h,
            72,
            pixels,
            RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(w.into()), Pt::new(h.into())),
        )
        .unwrap()
    }

    #[test]
    fn a_flat_colour_should_stay_flat_and_planar() {
        let out = chw_640(&raster(10, 20, [10, 20, 30, 255]));
        assert_eq!(out.len(), 3 * 640 * 640);
        assert!(out[..640 * 640].iter().all(|&v| v == 10));
        assert!(out[640 * 640..2 * 640 * 640].iter().all(|&v| v == 20));
        assert!(out[2 * 640 * 640..].iter().all(|&v| v == 30));
    }

    #[test]
    fn a_transparent_page_should_read_as_white() {
        let out = chw_640(&raster(1240, 1754, [0, 0, 0, 0]));
        assert!(out.iter().all(|&v| v == 255));
    }

    #[test]
    fn a_thin_stroke_should_survive_the_reduction() {
        // A one-pixel black column in a 1280-wide white page: a skipping resize could miss it.
        let (w, h) = (1280_u32, 64_u32);
        let mut pixels = [255_u8, 255, 255, 255].repeat(usize::try_from(w * h).unwrap());
        for y in 0..usize::try_from(h).unwrap() {
            let i = (y * 1280 + 641) * 4;
            pixels[i..i + 3].copy_from_slice(&[0, 0, 0]);
        }
        let page = RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(1280.0), Pt::new(64.0));
        let out = chw_640(&PageRaster::new(w, h, 72, pixels, page).unwrap());
        assert!(
            out[..640].iter().any(|&v| v < 200),
            "the stroke left a mark in the first row"
        );
    }
}
