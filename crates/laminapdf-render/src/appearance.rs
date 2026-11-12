//! Page appearance (ADR 0024 §5, UI/UX plan §5.5): a page drawn in Sepia, Dark or High contrast
//! instead of its own colours, apart from the UI theme.
//!
//! Neither backend can recolour as it paints: PDFium's colour scheme forces every fill to one
//! colour, which flattens charts and tinted boxes, and hayro's painting device is private. So a
//! tile is drawn as usual, then recoloured pixel by pixel, except where it shows an image or an
//! annotation:
//!
//! - images keep their pixels: each backend lists where its own parser placed them, as quads in
//!   tile pixels ([`Quad`]);
//! - annotations keep their colours: the tile is drawn without annotations, then again with them
//!   ([`Annotated`]); what they change comes from the second drawing, and so does the whole
//!   rectangle of one that changes most of it, so text under a highlight keeps its colour.
//!
//! The colour maps use integer arithmetic only, so `@laminapdf/render`'s port gives the same
//! bytes (`fixtures/appearance/colours.json` holds both to it). Text drawn as part of an image
//! keeps its colours, and so does the area an image's quad covers when the image is clipped
//! or transparent there.

use crate::request::{BYTES_PER_PIXEL, Rgba8};

/// How the page's colours are drawn. The default keeps them.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PageAppearance {
    /// The page's own colours.
    #[default]
    Original,
    /// Warm paper and brown ink, hue kept.
    Sepia,
    /// Light ink on dark paper: lightness inverted, hue kept.
    Dark,
    /// Two colours, usually the system's `CanvasText` and `Canvas`: the page's dark colours
    /// toward `ink`, its light ones toward `paper`, mid-tones pushed toward `ink`.
    HighContrast {
        /// What black becomes.
        ink: Rgba8,
        /// What white becomes.
        paper: Rgba8,
    },
}

/// An appearance's paper (what white becomes) and ink (what black becomes).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PagePalette {
    /// What white becomes.
    pub paper: Rgba8,
    /// What black becomes.
    pub ink: Rgba8,
}

/// Sepia's paper and ink: `@laminapdf/tokens`' `PAGE_APPEARANCES.sepia` (`@laminapdf/render`'s
/// tests hold both to the tokens).
///
/// Added in 0.2.0.
pub const SEPIA: PagePalette = PagePalette {
    paper: rgb(0xF4, 0xEC, 0xD8),
    ink: rgb(0x3B, 0x2F, 0x20),
};
/// Dark's paper and ink: `@laminapdf/tokens`' `PAGE_APPEARANCES.dark`.
///
/// Added in 0.2.0.
pub const DARK: PagePalette = PagePalette {
    paper: rgb(0x22, 0x21, 0x1F),
    ink: rgb(0xE6, 0xE1, 0xD8),
};

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

impl PageAppearance {
    /// Whether this appearance changes any colour.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn recolours(self) -> bool {
        !matches!(self, Self::Original)
    }

    /// The colour `rgb` (straight, sRGB) becomes; alpha is never changed.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn recolour_rgb(self, rgb: [u8; 3]) -> [u8; 3] {
        let [red, green, blue] = rgb;
        let light = luma(red, green, blue);
        match self {
            Self::Original => rgb,
            // Each channel from ink (0) to paper (255): black is ink, white is paper.
            Self::Sepia => {
                let PagePalette { paper, ink } = SEPIA;
                [
                    lerp(ink.r, paper.r, red),
                    lerp(ink.g, paper.g, green),
                    lerp(ink.b, paper.b, blue),
                ]
            }
            // The lightness from ink (black) to paper (white), plus the colour's own offset from
            // its lightness, so a blue bar stays bluish.
            Self::Dark => {
                let PagePalette { paper, ink } = DARK;
                let shift = |from: u8, to: u8, channel: u8| {
                    clamp(i32::from(lerp(from, to, light)) + i32::from(channel) - i32::from(light))
                };
                [
                    shift(ink.r, paper.r, red),
                    shift(ink.g, paper.g, green),
                    shift(ink.b, paper.b, blue),
                ]
            }
            // Squaring the lightness pushes greys toward ink, so light-grey text stays readable.
            Self::HighContrast { ink, paper } => {
                let at = div255(u32::from(light) * u32::from(light));
                [
                    lerp(ink.r, paper.r, at),
                    lerp(ink.g, paper.g, at),
                    lerp(ink.b, paper.b, at),
                ]
            }
        }
    }
}

/// Rec. 601 luma in 8.8 fixed point: `(77 r + 150 g + 29 b + 128) >> 8`, 0–255.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    let v = (77 * u32::from(r) + 150 * u32::from(g) + 29 * u32::from(b) + 128) >> 8;
    u8::try_from(v.min(255)).unwrap_or(u8::MAX)
}

/// `a` at `t` = 0 to `b` at `t` = 255, rounded.
fn lerp(a: u8, b: u8, t: u8) -> u8 {
    let t = u32::from(t);
    div255(u32::from(a) * (255 - t) + u32::from(b) * t)
}

/// `round(v / 255)` for `v` ≤ 255 × 255.
fn div255(v: u32) -> u8 {
    u8::try_from(((v + 127) / 255).min(255)).unwrap_or(u8::MAX)
}

fn clamp(v: i32) -> u8 {
    u8::try_from(v.clamp(0, 255)).unwrap_or(u8::MAX)
}

/// A quadrilateral in tile pixels: where an image's unit square or an annotation's rectangle lands.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad(pub [[f64; 2]; 4]);

impl Quad {
    /// The unit square under the affine `[a, b, c, d, e, f]` (to tile pixels).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_transform(matrix: [f64; 6]) -> Self {
        let [xx, xy, yx, yy, dx, dy] = matrix;
        let at = |x: f64, y: f64| [xx * x + yx * y + dx, xy * x + yy * y + dy];
        Self([at(0.0, 0.0), at(1.0, 0.0), at(1.0, 1.0), at(0.0, 1.0)])
    }

    /// Whether the point is inside the quad (on its edge counts), whichever way it winds.
    fn contains(&self, x: f64, y: f64) -> bool {
        let [p0, p1, p2, p3] = self.0;
        let side = |[ax, ay]: [f64; 2], [bx, by]: [f64; 2]| (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        let s = [side(p0, p1), side(p1, p2), side(p2, p3), side(p3, p0)];
        s.iter().all(|&v| v >= 0.0) || s.iter().all(|&v| v <= 0.0)
    }

    /// The quad's pixel bounds inside a `width` × `height` tile, if it touches it.
    fn bounds(&self, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let xs = self.0.map(|p| p[0]);
        let ys = self.0.map(|p| p[1]);
        let lo = |v: [f64; 4]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = |v: [f64; 4]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let (x0, x1, y0, y1) = (lo(xs), hi(xs), lo(ys), hi(ys));
        if !(x0.is_finite() && x1.is_finite() && y0.is_finite() && y1.is_finite()) {
            return None;
        }
        // Pixel centres at i + 0.5: the first and last columns whose centre can be inside.
        let first = |v: f64| (v - 0.5).ceil().max(0.0);
        let last = |v: f64, n: u32| (v - 0.5).floor().min(f64::from(n) - 1.0);
        let (cx0, cx1, cy0, cy1) = (first(x0), last(x1, width), first(y0), last(y1, height));
        if cx0 > cx1 || cy0 > cy1 {
            return None;
        }
        // In range by the clamps above: non-negative and below the tile's u32 sides.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..width and 0..height"
        )]
        Some((cx0 as u32, cx1 as u32, cy0 as u32, cy1 as u32))
    }

    /// Whether the quad covers the centre of any pixel of a `width` × `height` tile (its bounding
    /// box does, at least): a cheap test before work on what it covers.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn touches(&self, width: u32, height: u32) -> bool {
        self.bounds(width, height).is_some()
    }

    /// The indices of the pixels whose centres are inside, in a tile `width` × `height`.
    fn pixels(&self, width: u32, height: u32) -> Vec<usize> {
        let Some((x0, x1, y0, y1)) = self.bounds(width, height) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.contains(f64::from(x) + 0.5, f64::from(y) + 0.5) {
                    out.push(y as usize * width as usize + x as usize);
                }
            }
        }
        out
    }
}

/// What a recoloured tile keeps as drawn.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, Default)]
pub struct Kept<'a> {
    /// Opaque images: every pixel inside is kept.
    pub images: &'a [Quad],
    /// Images that let the page show through (a soft mask, a mask, a blend mode other than
    /// Normal): inside, only pixels that differ from `background` are kept, so the paper around
    /// a masked shape is recoloured with the page.
    pub see_through: &'a [Quad],
    /// The tile's background (`None`: transparent).
    pub background: Option<Rgba8>,
    /// The tile drawn again with its annotations.
    pub annotated: Option<Annotated<'a>>,
}

/// The same tile drawn with its annotations, and the rectangles kept whole.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
pub struct Annotated<'a> {
    /// The tile with annotations: same size as the one being recoloured.
    pub pixels: &'a [u8],
    /// In tile pixels, the rectangles of the annotations [`mostly_changed`] picked: every pixel
    /// inside comes from `pixels`, so text under a highlight keeps its colour.
    pub whole: &'a [Quad],
}

/// Which annotations change at least half of their rectangle (a highlight, a filled shape, a
/// stamp), decided once for the page: `plain` and `annotated` are the whole page drawn small,
/// without and with annotations, `width` pixels wide, and `rects` the rectangles in its pixels.
/// Deciding per tile instead would split an annotation that two tiles share.
///
/// Added in 0.2.0.
#[must_use]
pub fn mostly_changed(rects: &[Quad], plain: &[u8], annotated: &[u8], width: u32) -> Vec<bool> {
    let height = u32::try_from(plain.len() / (width as usize * BYTES_PER_PIXEL).max(1)).unwrap_or(u32::MAX);
    rects
        .iter()
        .map(|rect| {
            let inside = rect.pixels(width, height);
            let differing = inside.iter().filter(|&&i| changed(plain, annotated, i)).count();
            // Too small to cover a pixel of the small drawing: kept whole, as a highlight that
            // small would be (a large page's 256 px drawing can miss a line of text).
            inside.is_empty() || differing * 2 >= inside.len()
        })
        .collect()
}

/// Whether pixel `i` differs between the two drawings.
fn changed(plain: &[u8], annotated: &[u8], i: usize) -> bool {
    let at = i * BYTES_PER_PIXEL..(i + 1) * BYTES_PER_PIXEL;
    plain.get(at.clone()) != annotated.get(at)
}

/// What happens to each pixel of a recoloured tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fate {
    Recolour,
    /// Inside an image: kept as drawn.
    Keep,
    /// Under an annotation: taken from the annotated drawing.
    Annotation,
}

/// Recolours a tile drawn without annotations, in place: straight RGBA8 rows of `width × 4`.
///
/// Pixels of an opaque image keep their colour, and so do those of a see-through image that
/// aren't the background. With [`Kept::annotated`], its whole rectangles and every other pixel
/// the annotations change (an ink stroke, a line) come from the annotated drawing.
///
/// Added in 0.2.0.
pub fn recolour_tile(appearance: PageAppearance, width: u32, pixels: &mut [u8], kept: &Kept<'_>) {
    if !appearance.recolours() || width == 0 {
        return;
    }
    let row = width as usize * BYTES_PER_PIXEL;
    let height = u32::try_from(pixels.len() / row.max(1)).unwrap_or(u32::MAX);
    let mut fates = vec![Fate::Recolour; width as usize * height as usize];
    for i in kept.images.iter().flat_map(|quad| quad.pixels(width, height)) {
        if let Some(fate) = fates.get_mut(i) {
            *fate = Fate::Keep;
        }
    }
    let background = kept.background.map_or([0; 4], |c| [c.r, c.g, c.b, c.a]);
    for i in kept.see_through.iter().flat_map(|quad| quad.pixels(width, height)) {
        let shows_paper = pixels.get(i * BYTES_PER_PIXEL..(i + 1) * BYTES_PER_PIXEL) == Some(&background[..]);
        if let Some(fate) = fates.get_mut(i)
            && !shows_paper
        {
            *fate = Fate::Keep;
        }
    }
    let over = kept.annotated.map_or(&[][..], |a| a.pixels);
    if let Some(annotated) = kept.annotated {
        mark_annotations(&mut fates, pixels, annotated, width, height);
    }
    for (i, px) in pixels.as_chunks_mut::<BYTES_PER_PIXEL>().0.iter_mut().enumerate() {
        match fates.get(i) {
            Some(Fate::Keep) => {}
            Some(Fate::Annotation) => {
                if let Some(from) = over.get(i * BYTES_PER_PIXEL..(i + 1) * BYTES_PER_PIXEL) {
                    px.copy_from_slice(from);
                }
            }
            _ => {
                let [red, green, blue, _] = px;
                [*red, *green, *blue] = appearance.recolour_rgb([*red, *green, *blue]);
            }
        }
    }
}

/// Marks the pixels taken from the annotated drawing: the whole rectangles, and every other
/// pixel the annotations change.
fn mark_annotations(fates: &mut [Fate], plain: &[u8], annotated: Annotated<'_>, width: u32, height: u32) {
    for i in annotated.whole.iter().flat_map(|rect| rect.pixels(width, height)) {
        if let Some(fate) = fates.get_mut(i) {
            *fate = Fate::Annotation;
        }
    }
    for (i, fate) in fates.iter_mut().enumerate() {
        if changed(plain, annotated.pixels, i) {
            *fate = Fate::Annotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Annotated, DARK, Kept, PageAppearance, Quad, SEPIA, mostly_changed, recolour_tile};
    use crate::Rgba8;

    const WHITE: [u8; 3] = [255, 255, 255];
    const BLACK: [u8; 3] = [0, 0, 0];

    fn rgb(c: Rgba8) -> [u8; 3] {
        [c.r, c.g, c.b]
    }

    #[test]
    fn original_keeps_every_colour() {
        assert_eq!(PageAppearance::Original.recolour_rgb([12, 34, 56]), [12, 34, 56]);
    }

    #[test]
    fn white_becomes_paper_and_black_becomes_ink() {
        for (appearance, palette) in [(PageAppearance::Sepia, SEPIA), (PageAppearance::Dark, DARK)] {
            assert_eq!(appearance.recolour_rgb(WHITE), rgb(palette.paper), "{appearance:?}");
            assert_eq!(appearance.recolour_rgb(BLACK), rgb(palette.ink), "{appearance:?}");
        }
    }

    #[test]
    fn dark_keeps_a_colours_tint() {
        // A blue-grey bar stays bluer than it is red.
        let [r, _, b] = PageAppearance::Dark.recolour_rgb([0x4E, 0x5A, 0x63]);
        assert!(b > r);
    }

    #[test]
    fn high_contrast_pushes_grey_toward_ink() {
        let ink = Rgba8 {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        };
        let paper = Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        let hc = PageAppearance::HighContrast { ink, paper };
        assert_eq!(hc.recolour_rgb(BLACK), [255, 255, 0]);
        assert_eq!(hc.recolour_rgb(WHITE), [0, 0, 0]);
        // Mid grey lands nearer ink than paper.
        assert!(hc.recolour_rgb([128, 128, 128])[0] > 128);
    }

    #[test]
    fn image_pixels_and_alpha_are_kept() {
        // 2 × 1: the left pixel under an image quad, the right one half transparent.
        let mut pixels = [0, 0, 0, 255, 0, 0, 0, 128];
        let quad = Quad::from_transform([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let kept = Kept {
            images: &[quad],
            ..Kept::default()
        };
        recolour_tile(PageAppearance::Dark, 2, &mut pixels, &kept);
        let ink = rgb(DARK.ink);
        assert_eq!(pixels, [0, 0, 0, 255, ink[0], ink[1], ink[2], 128]);
    }

    #[test]
    fn annotation_pixels_come_from_the_annotated_drawing() {
        let mut pixels = [255, 255, 255, 255, 255, 255, 255, 255];
        let annotated = [255, 255, 0, 255, 255, 255, 255, 255];
        let annotated = Annotated {
            pixels: &annotated,
            whole: &[],
        };
        let kept = Kept {
            annotated: Some(annotated),
            ..Kept::default()
        };
        recolour_tile(PageAppearance::Dark, 2, &mut pixels, &kept);
        let paper = rgb(DARK.paper);
        assert_eq!(pixels, [255, 255, 0, 255, paper[0], paper[1], paper[2], 255]);
    }

    #[test]
    fn a_mostly_changed_annotation_keeps_its_whole_rectangle() {
        // 3 × 1 under one annotation: two pixels changed (yellow), the middle one black in both
        // drawings, like a letter under a highlight.
        let mut pixels = [255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255];
        let over = [255, 255, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255];
        let rect = Quad::from_transform([3.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let plain = pixels;
        assert_eq!(mostly_changed(&[rect], &plain, &over, 3), [true]);
        let annotated = Annotated {
            pixels: &over,
            whole: &[rect],
        };
        let kept = Kept {
            annotated: Some(annotated),
            ..Kept::default()
        };
        recolour_tile(PageAppearance::Dark, 3, &mut pixels, &kept);
        assert_eq!(pixels, over);
    }

    #[test]
    fn a_see_through_image_lets_the_paper_be_recoloured() {
        // 2 × 1 under a masked image: a white (background) pixel and a blue one.
        let mut pixels = [255, 255, 255, 255, 40, 120, 200, 255];
        let quad = Quad::from_transform([2.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let kept = Kept {
            see_through: &[quad],
            background: Some(Rgba8::WHITE),
            ..Kept::default()
        };
        recolour_tile(PageAppearance::Dark, 2, &mut pixels, &kept);
        let paper = rgb(DARK.paper);
        assert_eq!(pixels, [paper[0], paper[1], paper[2], 255, 40, 120, 200, 255]);
    }

    #[test]
    fn a_quad_outside_the_tile_keeps_nothing() {
        let mut pixels = [0, 0, 0, 255];
        let quad = Quad::from_transform([1.0, 0.0, 0.0, 1.0, 5.0, 5.0]);
        let kept = Kept {
            images: &[quad],
            ..Kept::default()
        };
        recolour_tile(PageAppearance::Sepia, 1, &mut pixels, &kept);
        assert_eq!(pixels[..3], rgb(SEPIA.ink));
    }
}
