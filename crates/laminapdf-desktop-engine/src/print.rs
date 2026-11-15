//! Print bands (D4c, ADR 0008): a page rendered for printing, one band of rows at a time, so
//! neither process holds a whole page at print resolution. The engine picks the band height;
//! the main process asks for band 0 first and learns from the reply how many bands follow.

use laminapdf_desktop_protocol::{FailureKind, FromEngine, PrintBandRequest};
use laminapdf_render::{PixelRect, RenderDocument, RenderFlags, RenderRequest};

/// The highest resolution a print band may be asked for. The main process asks for 300 dpi (the
/// spike's measured cap); more than 600 is refused as a malformed request.
pub(crate) const MAX_PRINT_DPI: u16 = 600;

/// Pixels in one band at most: 16 MiB of RGBA, a quarter of a frame.
const BAND_PIXELS: u64 = 4 * 1024 * 1024;

/// What a failed plan answers.
pub(crate) type Failure = (FailureKind, String);

/// The render request for `band` and the reply that will carry its pixels, or why there is none.
pub(crate) fn plan(
    render: &dyn RenderDocument,
    band: &PrintBandRequest,
) -> Result<(RenderRequest, FromEngine), Failure> {
    if band.dpi == 0 || band.dpi > MAX_PRINT_DPI {
        return Err((FailureKind::Protocol, "the print resolution is out of range".into()));
    }
    let page = render.page(band.page).map_err(failure)?;
    let limits = render.limits();
    let side = limits.max_render_side;
    // A page wider than the renderer's side limit at this resolution prints at a lower one.
    let mut dpi = band.dpi.min(fit_dpi(page.width, side));
    let (width, height) = loop {
        let corner = RenderRequest::page(band.page, scale(dpi)).with_clip(PixelRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        });
        let whole = render.plan(&corner).map_err(failure)?;
        if whole.page_width <= side || dpi == 1 {
            break (whole.page_width, whole.page_height);
        }
        // `fit_dpi` rounds in f64 and the plan in f32: a pixel over the side lowers it once more.
        dpi -= 1;
    };
    let per_band = rows_per_band(width, limits.max_render_pixels);
    let bands = height.div_ceil(per_band);
    if band.band >= bands {
        return Err((FailureKind::PageOutOfRange, "the page has no such band".into()));
    }
    let y = band.band * per_band;
    let rows = per_band.min(height - y);
    let request = RenderRequest::page(band.page, scale(dpi))
        .with_clip(PixelRect {
            x: 0,
            y,
            width,
            height: rows,
        })
        .with_flags(RenderFlags {
            print: true,
            ..RenderFlags::default()
        });
    render.plan(&request).map_err(failure)?;
    let reply = FromEngine::PrintBand {
        req: band.req,
        dpi,
        width,
        height,
        y,
        rows,
        bands,
        format: band.format,
    };
    Ok((request, reply))
}

/// The payload's length for `reply`, a [`FromEngine::PrintBand`]: four bytes a pixel.
pub(crate) fn byte_len(reply: &FromEngine) -> usize {
    match reply {
        FromEngine::PrintBand { width, rows, .. } => {
            usize::try_from(u64::from(*width) * u64::from(*rows) * 4).unwrap_or(usize::MAX)
        }
        _ => 0,
    }
}

/// Swaps red and blue in place: RGBA becomes BGRA.
pub(crate) fn rgba_to_bgra(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
}

/// Pixels per point at `dpi`.
fn scale(dpi: u16) -> f32 {
    f32::from(dpi) / 72.0
}

/// The highest resolution at which a page `width_pt` wide is at most `side` pixels wide.
fn fit_dpi(width_pt: f32, side: u32) -> u16 {
    if !(width_pt.is_finite() && width_pt > 0.0) {
        return 1;
    }
    let dpi = (f64::from(side) * 72.0 / f64::from(width_pt)).floor();
    // Clamped to 1..=u16::MAX first, so the cast keeps the value.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to u16's range"
    )]
    let dpi = dpi.clamp(1.0, f64::from(u16::MAX)) as u16;
    dpi
}

/// Rows in one band of a page `width` pixels wide: as many as fit [`BAND_PIXELS`] and the
/// renderer's `max_pixels` per tile, and at least one.
fn rows_per_band(width: u32, max_pixels: u64) -> u32 {
    let budget = BAND_PIXELS.min(max_pixels);
    let rows = budget / u64::from(width.max(1));
    u32::try_from(rows.max(1)).unwrap_or(u32::MAX)
}

#[expect(clippy::needless_pass_by_value, reason = "the shape `map_err` passes")]
fn failure(error: laminapdf_render::RenderError) -> Failure {
    (crate::worker::kind_of(error.code()), error.code().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_per_band_should_keep_a_band_within_sixteen_mebibytes() {
        // A4 at 300 dpi is 2480 pixels wide: 1691 rows of it hold 4 Mi pixels.
        assert_eq!(rows_per_band(2480, u64::MAX), 1691);
    }

    #[test]
    fn rows_per_band_should_respect_the_renderers_pixel_limit() {
        assert_eq!(rows_per_band(1000, 50_000), 50);
    }

    #[test]
    fn rows_per_band_should_be_at_least_one_row() {
        assert_eq!(rows_per_band(16_384, 1), 1);
    }

    #[test]
    fn fit_dpi_should_lower_the_resolution_of_a_page_too_wide_for_the_renderer() {
        // A 200-inch banner fits 16,384 pixels at 81 dpi, not 300.
        assert_eq!(fit_dpi(200.0 * 72.0, 16_384), 81);
    }

    #[test]
    fn fit_dpi_should_answer_one_for_a_page_with_no_width() {
        assert_eq!(fit_dpi(0.0, 16_384), 1);
    }

    #[test]
    fn rgba_to_bgra_should_swap_red_and_blue_of_every_pixel() {
        let mut pixels = [1, 2, 3, 4, 5, 6, 7, 8];
        rgba_to_bgra(&mut pixels);
        assert_eq!(pixels, [3, 2, 1, 4, 7, 6, 5, 8]);
    }
}
