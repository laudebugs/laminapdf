//! What a caller asks a backend to draw ([`RenderRequest`]), the bounds every backend enforces
//! ([`RenderLimits`]) and the tile geometry they share ([`TilePlan`]), ADR 0004 §1.

use laminapdf_model::Matrix;
use laminapdf_syntax::Rotation;

use crate::appearance::PageAppearance;
use crate::error::RenderError;

/// The output pixel format of every backend: RGBA8, **straight (not premultiplied) alpha**,
/// sRGB, rows packed at `width × 4` bytes. What `ImageData` and `createImageBitmap` take.
///
/// Added in 0.2.0.
pub const BYTES_PER_PIXEL: usize = 4;

/// Bounds on one render, checked before anything is allocated (`render-too-large`).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderLimits {
    /// Most pixels in one tile. Default 16,777,216 (4096 × 4096), iOS Safari's per-canvas cap.
    pub max_render_pixels: u64,
    /// Most pixels on either side of one tile. Default 16,384.
    pub max_render_side: u32,
    /// Most input bytes a backend that needs the whole file in memory (hayro) reads.
    /// Default 1 GiB.
    pub max_input_bytes: u64,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            max_render_pixels: 16_777_216,
            max_render_side: 16_384,
            max_input_bytes: 1 << 30,
        }
    }
}

/// How a backend opens a document.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default)]
pub struct RenderOpenOptions {
    password: Option<String>,
    limits: RenderLimits,
}

impl RenderOpenOptions {
    /// No password, default limits.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The password for an encrypted document. The backend uses it in `open` and keeps no
    /// copy (ADR 0004 §1): a shell that must reopen the document keeps it itself.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_password(mut self, password: &str) -> Self {
        self.password = Some(password.to_owned());
        self
    }

    /// Render limits for this document.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_limits(mut self, limits: RenderLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The password, if one was given.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// The render limits.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn limits(&self) -> &RenderLimits {
        &self.limits
    }
}

/// A rectangle in the output pixel space of the whole page at the request's scale and
/// rotation, top-left origin, y down.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// What to draw besides the page content.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent switches, each a PDFium render flag"
)]
pub struct RenderFlags {
    /// Annotation appearance streams (default on; PDFium's `FPDF_ANNOT`).
    pub annotations: bool,
    /// Form widgets, drawn read-only (default on). Widgets are annotations: they are drawn only
    /// when `annotations` is on too.
    pub form_widgets: bool,
    /// Print intent: honours annotation `Print`/`NoView` flags (PDFium's `FPDF_PRINTING`).
    pub print: bool,
    /// Grayscale output.
    pub grayscale: bool,
}

impl Default for RenderFlags {
    fn default() -> Self {
        Self {
            annotations: true,
            form_widgets: true,
            print: false,
            grayscale: false,
        }
    }
}

/// A straight-alpha sRGB colour.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha (255 = opaque).
    pub a: u8,
}

impl Rgba8 {
    /// Opaque white, the reader's page background.
    ///
    /// Added in 0.2.0.
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
}

/// One render: a page, a scale, an optional tile, a view rotation, flags, a background and the
/// page appearance.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderRequest {
    /// The page, 0-based (ADR 0027).
    pub page: u32,
    /// Output pixels per point, device pixel ratio already applied (1.0 is 72 DPI).
    pub scale: f32,
    /// The tile to draw; `None` is the whole page (still bounded by [`RenderLimits`]).
    pub clip: Option<PixelRect>,
    /// The view rotation.
    pub rotation: Rotation,
    /// What to draw.
    pub flags: RenderFlags,
    /// The background; `None` is transparent.
    pub background: Option<Rgba8>,
    /// How the page's colours are drawn (default: its own).
    pub appearance: PageAppearance,
}

impl RenderRequest {
    /// The whole of page `page` at `scale`, upright, default flags, on white.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn page(page: u32, scale: f32) -> Self {
        Self {
            page,
            scale,
            clip: None,
            rotation: Rotation::None,
            flags: RenderFlags::default(),
            background: Some(Rgba8::WHITE),
            appearance: PageAppearance::Original,
        }
    }

    /// Only the tile `clip`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_clip(mut self, clip: PixelRect) -> Self {
        self.clip = Some(clip);
        self
    }

    /// The view rotation.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// The flags.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_flags(mut self, flags: RenderFlags) -> Self {
        self.flags = flags;
        self
    }

    /// The background (`None`: transparent).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_background(mut self, background: Option<Rgba8>) -> Self {
        self.background = background;
        self
    }

    /// The page appearance.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_appearance(mut self, appearance: PageAppearance) -> Self {
        self.appearance = appearance;
        self
    }
}

/// A page as a backend reports it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderPage {
    /// Displayed width in points: the visible page box with `/Rotate` applied.
    pub width: f32,
    /// Displayed height in points.
    pub height: f32,
    /// The page's own `/Rotate`.
    pub rotate: Rotation,
}

/// The geometry of one validated request: the whole page's pixel size at the scale and view
/// rotation, and the tile inside it. Both backends render from this, so they agree on sizes.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TilePlan {
    /// The whole page's width in pixels, after the view rotation.
    pub page_width: u32,
    /// The whole page's height in pixels, after the view rotation.
    pub page_height: u32,
    /// The tile.
    pub tile: PixelRect,
}

impl TilePlan {
    /// Validates `request` against `page` and `limits`.
    ///
    /// The page's pixel size is `max(1, floor(points × scale))` per side.
    ///
    /// # Errors
    /// `invalid-argument` for a non-finite or non-positive scale, an empty tile or one outside
    /// the page, or a page too large to address at this scale; `render-too-large` above
    /// `limits`.
    ///
    /// Added in 0.2.0.
    pub fn new(page: &RenderPage, request: &RenderRequest, limits: &RenderLimits) -> Result<Self, RenderError> {
        let scale = f64::from(request.scale);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(RenderError::InvalidRequest {
                what: "scale must be finite and positive",
            });
        }
        if !(page.width.is_finite() && page.height.is_finite() && page.width > 0.0 && page.height > 0.0) {
            return Err(RenderError::InvalidRequest {
                what: "the page has no drawable area",
            });
        }
        let (w, h) = if request.rotation.is_quarter_turn() {
            (page.height, page.width)
        } else {
            (page.width, page.height)
        };
        let page_width = pixels(f64::from(w) * scale)?;
        let page_height = pixels(f64::from(h) * scale)?;
        let tile = request.clip.unwrap_or(PixelRect {
            x: 0,
            y: 0,
            width: page_width,
            height: page_height,
        });
        if tile.width == 0 || tile.height == 0 {
            return Err(RenderError::InvalidRequest {
                what: "the tile is empty",
            });
        }
        let right = u64::from(tile.x) + u64::from(tile.width);
        let bottom = u64::from(tile.y) + u64::from(tile.height);
        if right > u64::from(page_width) || bottom > u64::from(page_height) {
            return Err(RenderError::InvalidRequest {
                what: "the tile extends past the page",
            });
        }
        let (tw, th) = (u64::from(tile.width), u64::from(tile.height));
        let side = u64::from(limits.max_render_side);
        if tw > side || th > side {
            return Err(RenderError::TooLarge {
                width: tw,
                height: th,
                limit: side,
                unit: "pixels per side",
            });
        }
        if tw * th > limits.max_render_pixels {
            return Err(RenderError::TooLarge {
                width: tw,
                height: th,
                limit: limits.max_render_pixels,
                unit: "pixels",
            });
        }
        Ok(Self {
            page_width,
            page_height,
            tile,
        })
    }

    /// The bytes the tile needs: `width × height × 4`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        // Bounded by RenderLimits (u32 sides, at most max_render_pixels): fits usize on every
        // supported target only when the limit does, so saturate rather than wrap.
        usize::try_from(u64::from(self.tile.width) * u64::from(self.tile.height) * BYTES_PER_PIXEL as u64)
            .unwrap_or(usize::MAX)
    }

    /// The affine matrix from the displayed page space of `page` (points, top-left origin, y down, `/Rotate` applied:
    /// ADR 0027's page space) to this tile's pixels: scale to the page's integer pixel size,
    /// apply the view `rotation`, shift the tile to the origin. Both adapters draw with it, so
    /// their tiles agree.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn page_to_tile(&self, page: &RenderPage, rotation: Rotation) -> Matrix {
        // The page's pixel size before the view rotation.
        let (w0, h0) = if rotation.is_quarter_turn() {
            (f64::from(self.page_height), f64::from(self.page_width))
        } else {
            (f64::from(self.page_width), f64::from(self.page_height))
        };
        let (sx, sy) = (w0 / f64::from(page.width), h0 / f64::from(page.height));
        let (tx, ty) = (f64::from(self.tile.x), f64::from(self.tile.y));
        // Clockwise view rotations of the w0 × h0 bitmap, after scaling, then the tile shift.
        Matrix::new(match rotation {
            Rotation::None => [sx, 0.0, 0.0, sy, -tx, -ty],
            Rotation::Clockwise90 => [0.0, sx, -sy, 0.0, h0 - tx, -ty],
            Rotation::Half => [-sx, 0.0, 0.0, -sy, w0 - tx, h0 - ty],
            Rotation::Clockwise270 => [0.0, -sx, sy, 0.0, -tx, w0 - ty],
        })
    }

    /// Checks that `out` has exactly [`TilePlan::byte_len`] bytes.
    ///
    /// # Errors
    /// `invalid-argument` (`BufferSize`) otherwise.
    ///
    /// Added in 0.2.0.
    pub fn check_buffer(&self, out: &[u8]) -> Result<(), RenderError> {
        let expected = self.byte_len();
        if out.len() == expected {
            Ok(())
        } else {
            Err(RenderError::BufferSize {
                expected,
                actual: out.len(),
            })
        }
    }
}

/// `max(1, floor(v))` as pixels; a page wider than `i32::MAX` pixels (PDFium's limit) can't
/// be addressed at this scale.
fn pixels(v: f64) -> Result<u32, RenderError> {
    let v = v.floor().max(1.0);
    if v > f64::from(i32::MAX) {
        return Err(RenderError::InvalidRequest {
            what: "the page is too large to address at this scale",
        });
    }
    // In 1..=i32::MAX after the checks above: the cast is exact.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "range checked above"
    )]
    Ok(v as u32)
}

#[cfg(test)]
mod tests {

    use super::{PixelRect, RenderLimits, RenderPage, RenderRequest, Rotation, TilePlan};
    use crate::RenderError;

    const LETTER: RenderPage = RenderPage {
        width: 612.0,
        height: 792.0,
        rotate: Rotation::None,
    };

    mod tile_plan {
        use super::*;

        #[test]
        fn whole_page_floors_points_times_scale() {
            let plan = TilePlan::new(&LETTER, &RenderRequest::page(0, 96.0 / 72.0), &RenderLimits::default()).unwrap();
            assert_eq!((plan.page_width, plan.page_height), (816, 1056));
            assert_eq!(plan.byte_len(), 816 * 1056 * 4);
        }

        #[test]
        fn quarter_turn_swaps_the_page_size() {
            let req = RenderRequest::page(0, 1.0).with_rotation(Rotation::Clockwise90);
            let plan = TilePlan::new(&LETTER, &req, &RenderLimits::default()).unwrap();
            assert_eq!((plan.page_width, plan.page_height), (792, 612));
        }

        #[test]
        fn rejects_a_non_finite_or_non_positive_scale() {
            for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
                let err = TilePlan::new(&LETTER, &RenderRequest::page(0, scale), &RenderLimits::default()).unwrap_err();
                assert_eq!(err.code(), "invalid-argument", "{scale}");
            }
        }

        #[test]
        fn rejects_a_tile_past_the_page() {
            let req = RenderRequest::page(0, 1.0).with_clip(PixelRect {
                x: 600,
                y: 0,
                width: 13,
                height: 10,
            });
            let err = TilePlan::new(&LETTER, &req, &RenderLimits::default()).unwrap_err();
            assert_eq!(
                err,
                RenderError::InvalidRequest {
                    what: "the tile extends past the page"
                }
            );
        }

        #[test]
        fn rejects_a_page_without_area() {
            for (width, height) in [(0.0, 792.0), (612.0, -1.0), (f32::NAN, 792.0)] {
                let page = RenderPage {
                    width,
                    height,
                    rotate: Rotation::None,
                };
                let err = TilePlan::new(&page, &RenderRequest::page(0, 1.0), &RenderLimits::default()).unwrap_err();
                assert_eq!(err.code(), "invalid-argument", "{width}×{height}");
            }
        }

        #[test]
        fn rejects_an_empty_tile() {
            let req = RenderRequest::page(0, 1.0).with_clip(PixelRect {
                x: 0,
                y: 0,
                width: 0,
                height: 10,
            });
            assert_eq!(
                TilePlan::new(&LETTER, &req, &RenderLimits::default())
                    .unwrap_err()
                    .code(),
                "invalid-argument"
            );
        }

        #[test]
        fn refuses_a_tile_over_the_pixel_limit_before_allocating() {
            // A 1000× zoom of a letter page as one tile: 612,000 × 792,000 pixels.
            let err = TilePlan::new(&LETTER, &RenderRequest::page(0, 1000.0), &RenderLimits::default()).unwrap_err();
            assert_eq!(err.code(), "render-too-large");
        }

        #[test]
        fn allows_a_small_tile_of_a_huge_zoom() {
            let req = RenderRequest::page(0, 1000.0).with_clip(PixelRect {
                x: 300_000,
                y: 400_000,
                width: 512,
                height: 512,
            });
            let plan = TilePlan::new(&LETTER, &req, &RenderLimits::default()).unwrap();
            assert_eq!(plan.byte_len(), 512 * 512 * 4);
        }

        #[test]
        fn refuses_a_page_too_large_to_address() {
            let err = TilePlan::new(&LETTER, &RenderRequest::page(0, 1e9), &RenderLimits::default()).unwrap_err();
            assert_eq!(err.code(), "invalid-argument");
        }

        #[test]
        fn page_to_tile_maps_the_corners() {
            let page = RenderPage {
                width: 100.0,
                height: 50.0,
                rotate: Rotation::None,
            };
            let req = RenderRequest::page(0, 2.0);
            let plan = TilePlan::new(&page, &req, &RenderLimits::default()).unwrap();
            assert_eq!(
                plan.page_to_tile(&page, Rotation::None).apply(100.0, 50.0),
                (200.0, 100.0)
            );
            let turned = req.with_rotation(Rotation::Clockwise90);
            let plan = TilePlan::new(&page, &turned, &RenderLimits::default()).unwrap();
            // Clockwise: the top-left corner goes to the top-right of the 100 × 200 bitmap.
            assert_eq!(
                plan.page_to_tile(&page, Rotation::Clockwise90).apply(0.0, 0.0),
                (100.0, 0.0)
            );
            let shifted = req.with_clip(PixelRect {
                x: 10,
                y: 20,
                width: 5,
                height: 5,
            });
            let plan = TilePlan::new(&page, &shifted, &RenderLimits::default()).unwrap();
            assert_eq!(plan.page_to_tile(&page, Rotation::None).apply(5.0, 10.0), (0.0, 0.0));
        }

        #[test]
        fn check_buffer_wants_exactly_the_tile_bytes() {
            let plan = TilePlan::new(&LETTER, &RenderRequest::page(0, 0.1), &RenderLimits::default()).unwrap();
            assert!(plan.check_buffer(&vec![0; plan.byte_len()]).is_ok());
            assert_eq!(plan.check_buffer(&[0; 3]).unwrap_err().code(), "invalid-argument");
        }
    }
}
