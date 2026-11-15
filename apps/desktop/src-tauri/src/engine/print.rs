//! Print bands from a window's engine (ADR 0008 §9): asked one at a time, and checked like tiles, since
//! the engine reads untrusted files and so its answers are untrusted too.

use laminapdf_desktop_protocol::{DocId, FromEngine, PrintBandRequest, ReqId, TileFormat, ToEngine};

use super::{Engines, engine_error, needs_password, supervisor_error, unexpected};
use crate::DesktopError;

/// The most pixels one band may hold: the engine's `RenderLimits::max_render_pixels` default.
const MAX_BAND_PIXELS: u64 = 16_777_216;

/// The widest page the engine renders: `RenderLimits::max_render_side`'s default.
const MAX_WIDTH: u32 = 16_384;

/// The tallest page in pixels: a 200-inch page at 600 dpi, far past any paper.
const MAX_HEIGHT: u32 = 120_000;

/// One band of a page, checked.
#[derive(Debug)]
pub(crate) struct PrintedBand {
    /// The whole page's width in pixels, the band's too.
    pub width: u32,
    /// The whole page's height in pixels.
    pub height: u32,
    /// The band's first row.
    pub y: u32,
    /// The band's height in rows.
    pub rows: u32,
    /// How many bands the page has.
    pub bands: u32,
    /// `width × rows × 4` bytes in the order asked for.
    pub pixels: Vec<u8>,
}

/// One band to ask for.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BandQuery {
    pub doc: DocId,
    pub page: u32,
    pub dpi: u16,
    pub band: u32,
    pub format: TileFormat,
}

impl Engines {
    /// Renders band `query.band` of a page of `window`'s document for printing. `sent` learns the
    /// request's id as it goes out, so a cancel can name it.
    pub(crate) fn print_band(
        &self,
        window: &str,
        query: BandQuery,
        sent: &mut dyn FnMut(ReqId),
    ) -> Result<PrintedBand, DesktopError> {
        self.check(window, query.doc)?;
        let engine = self.engine(window)?;
        let reply = engine.request(|req| {
            sent(req);
            ToEngine::RenderPrintBand(PrintBandRequest {
                req,
                doc: query.doc,
                page: query.page,
                dpi: query.dpi,
                band: query.band,
                format: query.format,
            })
        });
        match reply {
            Ok((
                FromEngine::PrintBand {
                    dpi,
                    width,
                    height,
                    y,
                    rows,
                    bands,
                    format,
                    ..
                },
                pixels,
            )) => {
                let band = PrintedBand {
                    width,
                    height,
                    y,
                    rows,
                    bands,
                    pixels,
                };
                if format == query.format && (1..=query.dpi).contains(&dpi) && fits(&band, query.band) {
                    Ok(band)
                } else {
                    tracing::warn!("the engine sent a print band that doesn't fit its request");
                    Err(engine_error(laminapdf_desktop_protocol::FailureKind::Protocol))
                }
            }
            Ok((FromEngine::NeedsPassword { .. }, _)) => Err(needs_password()),
            Ok((FromEngine::Error { kind, .. }, _)) => Err(engine_error(kind)),
            Ok((other, _)) => Err(unexpected(&other)),
            Err(error) => Err(supervisor_error(&error)),
        }
    }

    /// Drops `window`'s request `req`, if it still runs: the engine answers it cancelled.
    pub(crate) fn cancel_request(&self, window: &str, req: ReqId) {
        if let Some(engine) = self.windows().get(window).cloned() {
            engine.send(&ToEngine::Cancel { req });
        }
    }
}

/// Whether `band`'s numbers are possible for band `index` and its pixels hold exactly it.
fn fits(band: &PrintedBand, index: u32) -> bool {
    let pixels = u64::from(band.width) * u64::from(band.rows);
    (1..=MAX_WIDTH).contains(&band.width)
        && (1..=MAX_HEIGHT).contains(&band.height)
        && band.rows > 0
        && pixels <= MAX_BAND_PIXELS
        && u64::from(band.y) + u64::from(band.rows) <= u64::from(band.height)
        && index < band.bands
        && band.bands <= band.height
        && band.pixels.len() as u64 == pixels * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(y: u32, rows: u32, pixels: usize) -> PrintedBand {
        PrintedBand {
            width: 10,
            height: 30,
            y,
            rows,
            bands: 3,
            pixels: vec![0; pixels],
        }
    }

    #[test]
    fn a_band_whose_pixels_fill_it_should_fit() {
        assert!(fits(&band(20, 10, 10 * 10 * 4), 2));
    }

    #[test]
    fn a_band_past_the_bottom_of_its_page_should_not_fit() {
        assert!(!fits(&band(25, 10, 10 * 10 * 4), 2));
    }

    #[test]
    fn a_band_with_too_few_pixels_should_not_fit() {
        assert!(!fits(&band(0, 10, 10 * 10 * 4 - 1), 0));
    }

    #[test]
    fn a_band_numbered_past_the_pages_bands_should_not_fit() {
        assert!(!fits(&band(0, 10, 10 * 10 * 4), 3));
    }
}
