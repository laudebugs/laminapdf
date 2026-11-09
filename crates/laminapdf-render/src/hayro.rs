//! The hayro adapter (feature `hayro`): ADR 0004's CI oracle and debug choice, pure Rust.
//!
//! hayro needs the whole file in memory, so `open` reads the source with [`read_all`]
//! (bounded by [`RenderLimits::max_input_bytes`]). Tiles are drawn with
//! [`hayro::render_into`] into a context the size of the tile, under a transform that scales
//! the page, applies the view rotation and shifts the tile to the origin, so no backend call
//! allocates more than one tile.
//!
//! hayro can't be paused, so the cancel token is checked before and after each call. An engine
//! panic (seen on a tiling pattern with large steps) is caught where unwinding is available and
//! reported as `render-failed`.
//!
//! Known differences from PDFium, so the nightly hayro-vs-PDFium diff (`cargo xtask render-diff`)
//! must compare with default flags:
//! hayro has no print intent (`print` is ignored) and no switch for widgets alone (it draws
//! them with the other annotations, so `form_widgets: false` is ignored too). A
//! [`RenderCache`] lives for one call: its lifetime is tied to the page borrowed from the
//! document, so it can't be kept on the document without a self-referential type.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use hayro::hayro_interpret::font::GlyphRun;
use hayro::hayro_interpret::util::{RectExt as _, TransformExt as _};
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageDrawProps, InterpreterCache,
    InterpreterSettings, SoftMask, interpret_page,
};
use hayro::hayro_syntax::object::dict::keys::{ANNOTS, F, MASK, RECT, SMASK};
use hayro::hayro_syntax::object::{Array, Dict, Rect as PdfRect};
use hayro::hayro_syntax::page::Rotation as HayroRotation;
use hayro::hayro_syntax::{Pdf, PdfData};
use hayro::kurbo::{Affine, BezPath, Rect};
use hayro::vello_cpu::color::AlphaColor;
use hayro::vello_cpu::color::palette::css::TRANSPARENT;
use hayro::vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{RenderCache, RenderSettings};
use laminapdf_cos::ByteSource;
use laminapdf_syntax::CancelToken;
use laminapdf_syntax::Rotation;

use crate::appearance::{Annotated, Kept, Quad, mostly_changed, recolour_tile};
use crate::backend::{RenderBackend, RenderDocument};
use crate::error::RenderError;
use crate::request::{RenderLimits, RenderOpenOptions, RenderPage, RenderRequest, Rgba8, TilePlan};
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
        // An in-memory source is borrowed, not copied (ADR 0004: never a second copy);
        // anything else is read whole, which hayro needs.
        let data: PdfData = if source.as_slice().is_some() {
            if source.len() > limits.max_input_bytes {
                return Err(RenderError::InputTooLarge {
                    len: source.len(),
                    limit: limits.max_input_bytes,
                });
            }
            Arc::new(InMemory(source)).into()
        } else {
            Arc::new(read_all(source.as_ref(), limits.max_input_bytes)?).into()
        };
        let password = options.password().unwrap_or("");
        let loaded =
            catch_unwind(AssertUnwindSafe(|| Pdf::new_with_password(data, password))).map_err(|_| failed("open"))?;
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
        Ok(Box::new(HayroDocument {
            pdf,
            limits,
            whole: RefCell::default(),
        }))
    }
}

/// An in-memory [`ByteSource`] as hayro's input, without a copy.
struct InMemory(Arc<dyn ByteSource>);

impl AsRef<[u8]> for InMemory {
    fn as_ref(&self) -> &[u8] {
        // Only built for sources whose `as_slice` is `Some`.
        self.0.as_slice().unwrap_or_default()
    }
}

/// A document open in hayro.
struct HayroDocument {
    pdf: Pdf,
    limits: RenderLimits,
    /// Per page, which annotations keep their whole rectangle when recoloured
    /// ([`mostly_changed`], decided once on a small drawing of the page).
    whole: RefCell<BTreeMap<u32, Vec<bool>>>,
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

impl HayroDocument {
    /// Which of `page`'s annotations keep their whole rectangle, from a drawing of the whole
    /// page at most [`DECISION_SIDE`] pixels on a side; cached per page.
    fn whole_rects(
        &self,
        page: &hayro::hayro_syntax::page::Page<'_>,
        request: &RenderRequest,
        info: &RenderPage,
    ) -> Result<Vec<bool>, RenderError> {
        if let Some(flags) = self.whole.borrow().get(&request.page) {
            return Ok(flags.clone());
        }
        let longest = info.width.max(info.height).max(1.0);
        let small = RenderRequest::page(request.page, DECISION_SIDE / longest)
            .with_flags(request.flags)
            .with_background(request.background);
        let plan = TilePlan::new(info, &small, &self.limits)?;
        let transform =
            Affine::new(plan.page_to_tile(info, Rotation::None).to_array()) * page.initial_transform(true).to_kurbo();
        let size = (
            u16::try_from(plan.tile.width).map_err(|_| failed("render"))?,
            u16::try_from(plan.tile.height).map_err(|_| failed("render"))?,
        );
        let mut without = vec![0; plan.byte_len()];
        let mut annotated = vec![0; plan.byte_len()];
        draw(page, &small, transform, size, false, &mut without)?;
        draw(page, &small, transform, size, true, &mut annotated)?;
        let flags = mostly_changed(
            &annotation_rects(page, transform),
            &without,
            &annotated,
            plan.tile.width,
        );
        self.whole.borrow_mut().insert(request.page, flags.clone());
        Ok(flags)
    }
}

/// The longest side of the page drawing [`mostly_changed`] decides on.
const DECISION_SIDE: f32 = 256.0;

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
        let transform = Affine::new(plan.page_to_tile(&info, request.rotation).to_array())
            * page.initial_transform(true).to_kurbo();
        let size = (tw, th);
        let annotations = request.flags.annotations;
        if request.appearance.recolours() {
            // Recoloured: the content alone, then (when the page has any) the annotations drawn
            // again over it, whose pixels keep their colours; images keep theirs too.
            draw(page, request, transform, size, false, out)?;
            let rects = if annotations {
                annotation_rects(page, transform)
            } else {
                Vec::new()
            };
            let (with, whole) = if rects.is_empty() {
                (None, Vec::new())
            } else {
                let mut with = vec![0; out.len()];
                draw(page, request, transform, size, true, &mut with)?;
                let flags = self.whole_rects(page, request, &info)?;
                let whole = rects.iter().zip(flags).filter(|(_, w)| *w).map(|(q, _)| *q).collect();
                (Some(with), whole)
            };
            let images = image_quads(page, transform, size)?;
            let kept = Kept {
                images: &images.opaque,
                see_through: &images.see_through,
                background: request.background,
                annotated: with.as_deref().map(|pixels| Annotated { pixels, whole: &whole }),
            };
            recolour_tile(request.appearance, plan.tile.width, out, &kept);
        } else {
            draw(page, request, transform, size, annotations, out)?;
        }
        if cancel.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        Ok(plan)
    }
}

/// Draws `page` under `transform` into the `size` tile `out`, with or without annotations.
fn draw(
    page: &hayro::hayro_syntax::page::Page<'_>,
    request: &RenderRequest,
    transform: Affine,
    (tw, th): (u16, u16),
    annotations: bool,
    out: &mut [u8],
) -> Result<(), RenderError> {
    let settings = InterpreterSettings {
        render_annotations: annotations,
        ..InterpreterSettings::default()
    };
    // Grayscale applies to the content only (as PDFium's FPDF_GRAYSCALE): draw on transparent,
    // gray it, then lay it over the background.
    let grayscale = request.flags.grayscale;
    let background = match request.background {
        Some(c) if !grayscale => AlphaColor::from_rgba8(c.r, c.g, c.b, c.a),
        _ => TRANSPARENT,
    };
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
    let under = if grayscale { request.background } else { None };
    unpremultiply_into(pixmap.data_as_u8_slice(), out, grayscale, under);
    Ok(())
}

/// Where the page's raster images land in the tile, read by interpreting the page without
/// drawing it. Stencil masks are painted in the fill colour, like text, so they are recoloured.
fn image_quads(
    page: &hayro::hayro_syntax::page::Page<'_>,
    transform: Affine,
    (tw, th): (u16, u16),
) -> Result<ImageScan, RenderError> {
    catch_unwind(AssertUnwindSafe(|| {
        let cache = InterpreterCache::new();
        let settings = InterpreterSettings {
            render_annotations: false,
            ..InterpreterSettings::default()
        };
        let bbox = Rect::new(0.0, 0.0, f64::from(tw), f64::from(th));
        let mut ctx = Context::new(transform, bbox, &cache, page.xref(), settings);
        let mut scan = ImageScan::default();
        interpret_page(page, &mut ctx, &mut scan);
        scan
    }))
    .map_err(|_| failed("image scan"))
}

/// Where each shown annotation's `/Rect` lands in the tile (hidden ones, `/F` bit 2, are left out,
/// as hayro leaves them undrawn).
fn annotation_rects(page: &hayro::hayro_syntax::page::Page<'_>, transform: Affine) -> Vec<Quad> {
    let Some(annots) = page.raw().get::<Array<'_>>(ANNOTS) else {
        return Vec::new();
    };
    annots
        .iter::<Dict<'_>>()
        .filter(|annot| annot.get::<u32>(F).unwrap_or(0) & 2 == 0)
        .filter_map(|annot| annot.get::<PdfRect>(RECT))
        .map(|rect| {
            let r = rect.to_kurbo();
            let box_ = Affine::new([r.width(), 0.0, 0.0, r.height(), r.x0, r.y0]);
            Quad::from_transform((transform * box_).as_coeffs())
        })
        .collect()
}

/// A device that only records each raster image's quad (its transform maps image pixels),
/// sorted by whether the page shows through it.
#[derive(Default)]
struct ImageScan {
    opaque: Vec<Quad>,
    see_through: Vec<Quad>,
}

impl<'a> Device<'a> for ImageScan {
    fn draw_path(&mut self, _: &BezPath, _: DrawProps<'a>, _: &DrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_glyph_run(&mut self, _: &GlyphRun<'_, 'a>, _: DrawProps<'a>, _: &DrawMode) {}
    fn draw_image(&mut self, image: Image<'a, '_>, props: ImageDrawProps<'a>) {
        if let Image::Raster(raster) = image {
            let to_unit = Affine::scale_non_uniform(f64::from(raster.width()), f64::from(raster.height()));
            let quad = Quad::from_transform((props.transform * to_unit).as_coeffs());
            let dict = raster.stream().dict();
            let masked = dict.contains_key(SMASK) || dict.contains_key(MASK);
            if masked || props.soft_mask.is_some() || props.blend_mode != BlendMode::Normal {
                self.see_through.push(quad);
            } else {
                self.opaque.push(quad);
            }
        }
    }
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

/// A page's displayed size and `/Rotate`.
fn geometry(page: &hayro::hayro_syntax::page::Page<'_>) -> RenderPage {
    let (width, height) = page.render_dimensions();
    RenderPage {
        width,
        height,
        rotate: match page.rotation() {
            HayroRotation::None => Rotation::None,
            HayroRotation::Horizontal => Rotation::Clockwise90,
            HayroRotation::Flipped => Rotation::Half,
            HayroRotation::FlippedHorizontal => Rotation::Clockwise270,
        },
    }
}

const fn failed(stage: &'static str) -> RenderError {
    RenderError::Failed { backend: NAME, stage }
}

/// hayro's premultiplied RGBA8 into straight-alpha RGBA8, same size: optionally gray (Rec. 601
/// luma, which is linear, so it applies to premultiplied channels), then laid over `under`.
fn unpremultiply_into(src: &[u8], out: &mut [u8], grayscale: bool, under: Option<Rgba8>) {
    for (&[r, g, b, a], d) in src.as_chunks::<4>().0.iter().zip(out.as_chunks_mut::<4>().0) {
        let [r, g, b] = if grayscale { [luma(r, g, b); 3] } else { [r, g, b] };
        // Source-over in premultiplied space: c + u·ua·(1 − a), alpha a + ua·(1 − a).
        let (r, g, b, a) = match under {
            Some(u) if a < 255 => {
                let rest = 255 - u32::from(a);
                let ua = u32::from(u.a);
                let over = |c: u8, uc: u8| clamp(u32::from(c) + (u32::from(uc) * ua * rest + 255 * 127) / (255 * 255));
                (
                    over(r, u.r),
                    over(g, u.g),
                    over(b, u.b),
                    clamp(u32::from(a) + (ua * rest + 127) / 255),
                )
            }
            _ => (r, g, b, a),
        };
        *d = if a == 0 {
            [0, 0, 0, 0]
        } else if a == 255 {
            [r, g, b, a]
        } else {
            [unpremultiply(r, a), unpremultiply(g, a), unpremultiply(b, a), a]
        };
    }
}

fn clamp(v: u32) -> u8 {
    u8::try_from(v.min(255)).unwrap_or(u8::MAX)
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
    use crate::Rgba8;

    #[test]
    fn unpremultiply_restores_straight_channels() {
        assert_eq!(unpremultiply(64, 128), 128);
        assert_eq!(unpremultiply(128, 128), 255);
        assert_eq!(unpremultiply(200, 128), 255);
    }

    #[test]
    fn transparent_pixels_become_zero() {
        let mut out = [9; 4];
        unpremultiply_into(&[10, 20, 30, 0], &mut out, false, None);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    #[test]
    fn grayscale_keeps_the_background_colour() {
        let sepia = Rgba8 {
            r: 255,
            g: 240,
            b: 200,
            a: 255,
        };
        let mut out = [0; 4];
        // A fully transparent content pixel: the background shows, ungrayed.
        unpremultiply_into(&[0, 0, 0, 0], &mut out, true, Some(sepia));
        assert_eq!(out, [255, 240, 200, 255]);
        // Opaque red content: gray, background hidden.
        unpremultiply_into(&[255, 0, 0, 255], &mut out, true, Some(sepia));
        assert_eq!(out, [76, 76, 76, 255]);
    }

    #[test]
    fn grayscale_uses_rec_601_luma() {
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(255, 0, 0), 76);
    }
}
