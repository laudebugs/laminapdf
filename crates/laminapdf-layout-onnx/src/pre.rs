//! The page image as Heron reads it: 640 × 640, planar RGB (all reds, then greens, then
//! blues), 8 bits per channel. The model's graph does its own scaling and normalising, so the
//! bytes go in as they are.

use laminapdf_extract::PageRaster;

use crate::SIDE;

/// `raster` resized to 640 × 640 by bilinear interpolation (as the export's preprocessor
/// does), alpha dropped. A pixel missing from a short buffer reads as white.
pub(crate) fn chw_640(raster: &PageRaster) -> Vec<u8> {
    let side = usize::try_from(SIDE).unwrap_or(640);
    let (w, h) = (raster.width_px.max(1), raster.height_px.max(1));
    let width = usize::try_from(w).unwrap_or(1);
    let mut out = vec![0_u8; 3 * side * side];
    let pixel = |x: u32, y: u32, c: usize| -> f32 {
        let i = (usize::try_from(y).unwrap_or(0) * width + usize::try_from(x).unwrap_or(0)) * 4 + c;
        f32::from(raster.pixels.get(i).copied().unwrap_or(255))
    };
    let (sx, sy) = (f64::from(w) / f64::from(SIDE), f64::from(h) / f64::from(SIDE));
    for oy in 0..SIDE {
        // Pixel centres map to pixel centres, as Pillow's bilinear filter does.
        let fy = ((f64::from(oy) + 0.5) * sy - 0.5).clamp(0.0, f64::from(h - 1));
        let (y0, ty) = split(fy, h);
        for ox in 0..SIDE {
            let fx = ((f64::from(ox) + 0.5) * sx - 0.5).clamp(0.0, f64::from(w - 1));
            let (x0, tx) = split(fx, w);
            let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
            for c in 0..3 {
                let top = pixel(x0, y0, c) * (1.0 - tx) + pixel(x1, y0, c) * tx;
                let bottom = pixel(x0, y1, c) * (1.0 - tx) + pixel(x1, y1, c) * tx;
                let value = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0);
                let at = c * side * side + usize::try_from(oy).unwrap_or(0) * side + usize::try_from(ox).unwrap_or(0);
                if let Some(slot) = out.get_mut(at) {
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "clamped to 0..=255 above"
                    )]
                    {
                        *slot = value as u8;
                    }
                }
            }
        }
    }
    out
}

/// A coordinate's whole pixel and the fraction past it.
fn split(f: f64, limit: u32) -> (u32, f32) {
    let floor = f.floor();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "f is clamped to 0..limit"
    )]
    let whole = (floor as u32).min(limit.saturating_sub(1));
    #[expect(clippy::cast_possible_truncation, reason = "a fraction in 0..1")]
    let frac = (f - floor) as f32;
    (whole, frac)
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
}
