//! The hayro adapter (feature `hayro`): ADR 0004's CI oracle and debug choice, pure Rust.
//!
//! hayro needs the whole file in memory, so `open` reads the source with [`read_all`]
//! (bounded by [`RenderLimits::max_input_bytes`]). Tiles are drawn with
//! [`hayro::render_into`] into a context the size of the tile, under a transform that scales
//! the page, applies the view rotation and shifts the tile to the origin, so no backend call
//! allocates more than one tile.
//!
//! hayro can't be paused, so the cancel token is checked before and after each call. An engine
//! panic (C1a saw one) is caught where unwinding is available and reported as `render-failed`.
//!
//! Known differences from PDFium, which the oracle diff (X3b) must compare with default flags:
//! hayro has no print intent (`print` is ignored) and no switch for widgets alone (it draws
//! them with the other annotations, so `form_widgets: false` is ignored too). A
//! [`RenderCache`] lives for one call: its lifetime is tied to the page borrowed from the
//! document, so it can't be kept on the document without a self-referential type.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_interpret::util::TransformExt as _;
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::page::Rotation;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::AlphaColor;
use hayro::vello_cpu::color::palette::css::TRANSPARENT;
use hayro::vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{RenderCache, RenderSettings};
use lamina_cos::ByteSource;
use lamina_syntax::CancelToken;

use crate::backend::{RenderBackend, RenderDocument};
use crate::error::RenderError;
use crate::request::{RenderLimits, RenderOpenOptions, RenderPage, RenderRequest, TilePlan};
use crate::source::read_all;

const NAME: &str = "hayro";

/// The pure-Rust renderer: hayro 0.8 with its standard 14 fonts and predefined CMaps built in.
#[derive(Debug, Clone, Copy, Default)]
pub struct HayroBackend;

impl HayroBackend {
    /// The backend.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RenderBackend for HayroBackend {
    fn name(&self) -> &'static str {
        NAME
    }

    fn version(&self) -> &'static str {
        // The workspace pins hayro exactly (`=0.8.0`).
        "0.8.0"
    }

    fn open(
        &self,
        source: Arc<dyn ByteSource>,
        options: &RenderOpenOptions,
    ) -> Result<Box<dyn RenderDocument>, RenderError> {
        let limits = *options.limits();
        let bytes = read_all(source.as_ref(), limits.max_input_bytes)?;
        let password = options.password().unwrap_or("");
        let loaded = catch_unwind(AssertUnwindSafe(|| Pdf::new_with_password(Arc::new(bytes), password)))
            .map_err(|_| failed("open"))?;
        let pdf = loaded.map_err(|e| match e {
            hayro::hayro_syntax::LoadPdfError::Decryption(hayro::hayro_syntax::DecryptionError::PasswordProtected) => {
                if options.password().is_some() {
                    RenderError::WrongPassword
                } else {
                    RenderError::PasswordRequired
                }
            }
            _ => failed("open"),
        })?;
        Ok(Box::new(HayroDocument { pdf, limits }))
    }
}

/// A document open in hayro.
struct HayroDocument {
    pdf: Pdf,
    limits: RenderLimits,
}

impl HayroDocument {
    fn count(&self) -> u32 {
        u32::try_from(self.pdf.pages().len()).unwrap_or(u32::MAX)
    }

    fn out_of_range(&self, index: u32) -> RenderError {
        RenderError::PageIndexOutOfRange {
            index,
            count: self.count(),
        }
    }
}

impl RenderDocument for HayroDocument {
    fn page_count(&self) -> u32 {
        self.count()
    }

    fn page(&self, index: u32) -> Result<RenderPage, RenderError> {
        let pages = self.pdf.pages();
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or_else(|| self.out_of_range(index))?;
        Ok(geometry(page))
    }

    fn limits(&self) -> &RenderLimits {
        &self.limits
    }

    fn render(&self, request: &RenderRequest, out: &mut [u8], cancel: &CancelToken) -> Result<TilePlan, RenderError> {
        let pages = self.pdf.pages();
        let page = usize::try_from(request.page)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or_else(|| self.out_of_range(request.page))?;
        let info = geometry(page);
        let plan = TilePlan::new(&info, request, &self.limits)?;
        plan.check_buffer(out)?;
        let (Ok(tw), Ok(th)) = (u16::try_from(plan.tile.width), u16::try_from(plan.tile.height)) else {
            return Err(RenderError::TooLarge {
                width: plan.tile.width.into(),
                height: plan.tile.height.into(),
                limit: u16::MAX.into(),
                unit: "pixels per side (hayro)",
            });
        };
        if cancel.is_cancelled() {
            return Err(RenderError::Cancelled);
        }

        // initial_transform(true) is the displayed page space (points, y down, /Rotate applied).
        let transform =
            Affine::new(plan.page_to_tile(&info, request.rotation)) * page.initial_transform(true).to_kurbo();
        let settings = InterpreterSettings {
            render_annotations: request.flags.annotations,
            ..InterpreterSettings::default()
        };
        let background = request
            .background
            .map_or(TRANSPARENT, |c| AlphaColor::from_rgba8(c.r, c.g, c.b, c.a));
        let pixmap = catch_unwind(AssertUnwindSafe(|| {
            let cache = RenderCache::new();
            let mut ctx = RenderContext::new(tw, th);
            hayro::render_into(page, &cache, &settings, &RenderSettings::default(), &mut ctx, transform);
            ctx.flush();
            let mut pixmap = Pixmap::new(tw, th);
            ctx.render_with(
                &mut pixmap,
                &mut Resources::default(),
                RasterizerSettings {
                    target_init: TargetInit::Clear(background),
                    ..RasterizerSettings::default()
                },
            );
            pixmap
        }))
        .map_err(|_| failed("render"))?;
        if cancel.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        unpremultiply_into(pixmap.data_as_u8_slice(), out, request.flags.grayscale);
        Ok(plan)
    }
}

/// A page's displayed size and `/Rotate`.
fn geometry(page: &hayro::hayro_syntax::page::Page<'_>) -> RenderPage {
    let (width, height) = page.render_dimensions();
    RenderPage {
        width,
        height,
        rotate: match page.rotation() {
            Rotation::None => 0,
            Rotation::Horizontal => 90,
            Rotation::Flipped => 180,
            Rotation::FlippedHorizontal => 270,
        },
    }
}

const fn failed(stage: &'static str) -> RenderError {
    RenderError::Failed { backend: NAME, stage }
}

/// hayro's premultiplied RGBA8 into straight-alpha RGBA8 (optionally gray), same size.
fn unpremultiply_into(src: &[u8], out: &mut [u8], grayscale: bool) {
    for (&[r, g, b, a], d) in src.as_chunks::<4>().0.iter().zip(out.as_chunks_mut::<4>().0) {
        let [r, g, b] = if a == 0 {
            [0, 0, 0]
        } else if a == 255 {
            [r, g, b]
        } else {
            [r, g, b].map(|c| unpremultiply(c, a))
        };
        let [r, g, b] = if grayscale { [luma(r, g, b); 3] } else { [r, g, b] };
        *d = [r, g, b, a];
    }
}

/// `round(c × 255 / a)`, clamped (a premultiplied channel can't exceed alpha, but rounding in
/// the rasteriser may).
fn unpremultiply(c: u8, a: u8) -> u8 {
    let v = (u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a);
    u8::try_from(v.min(255)).unwrap_or(u8::MAX)
}

/// Rec. 601 luma, rounded.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    let v = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 + 500) / 1000;
    u8::try_from(v.min(255)).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::{luma, unpremultiply, unpremultiply_into};

    #[test]
    fn unpremultiply_restores_straight_channels() {
        assert_eq!(unpremultiply(64, 128), 128);
        assert_eq!(unpremultiply(128, 128), 255);
        assert_eq!(unpremultiply(200, 128), 255);
    }

    #[test]
    fn transparent_pixels_become_zero() {
        let mut out = [9; 4];
        unpremultiply_into(&[10, 20, 30, 0], &mut out, false);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    #[test]
    fn grayscale_uses_rec_601_luma() {
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(255, 0, 0), 76);
    }
}
