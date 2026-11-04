//! [`Surface`]: what is drawn on a page, in page space ([ADR 0027]), written as a content
//! stream with `pdf_writer::Content` when the page is finished.
//!
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md

pub(crate) mod color;
pub(crate) mod image;
pub(crate) mod path;
mod pending;
mod png;
pub(crate) mod style;
#[cfg(feature = "text")]
pub(crate) mod text;
#[cfg(feature = "text")]
pub(crate) mod textbox;
mod writer;
pub use pending::PendingPath;
use writer::Writer;

#[cfg(feature = "text")]
use laminapdf_model::PointPt;
use laminapdf_model::{PageFrame, Pt, RectPt, UserSpaceRect};
#[cfg(feature = "text")]
use laminapdf_text::{Alignment, FontRegistry};

use crate::annot::link::LinkSpec;
#[cfg(feature = "text")]
use crate::error::Error;
use crate::error::Result;
use crate::nav::LinkTarget;
use crate::nav::dest::fit_is_valid;
use crate::resources::{GsKey, PageResources};
use crate::tag::mcid::{Marking, PageMarks};
use crate::tag::{ArtifactKind, Tag, TagKind};
use color::Color;
use image::Image;
use laminapdf_model::{FillRule, Matrix};
use path::{PathBuilder, Segment};
use style::{BlendMode, StrokeStyle};
#[cfg(feature = "text")]
use text::{Font, Spec, TextCommand, TextRenderMode, TextRun, lay_out};
#[cfg(feature = "text")]
use textbox::{LineBox, Overflow, TextBoxOptions, TextLayout};

/// How a path is painted.
#[derive(Debug, Clone, PartialEq)]
enum Paint {
    Stroke(StrokeStyle),
    Fill(FillRule),
    FillStroke(FillRule, StrokeStyle),
    Clip(FillRule),
}

/// One drawing operation, in page space.
#[derive(Debug, Clone)]
enum Command {
    Save,
    Restore,
    Transform(Matrix),
    FillColor(Color),
    StrokeColor(Color),
    State(GsKey),
    Paint(Vec<Segment>, Paint),
    Image(Image, RectPt),
    #[cfg(feature = "text")]
    Text(Box<TextCommand>),
    /// A link annotation over a page-space rectangle (already through the transform).
    Link(RectPt, LinkSpec),
    /// Opens a structure element.
    BeginTag(Box<Tag>),
    /// Opens an artifact.
    BeginArtifact(ArtifactKind),
    /// Ends the innermost element or artifact.
    EndMarked,
}

/// A page's content stream, the links it asked for and its structure.
pub(crate) struct Rendered {
    pub(crate) content: Vec<u8>,
    /// Each link's rectangle in user space, in order of drawing.
    pub(crate) links: Vec<(UserSpaceRect, LinkSpec)>,
    /// The page's elements, MCIDs and untagged content.
    pub(crate) marks: PageMarks,
}

/// What text needs while a page is written: the document's fonts and the document, where
/// a font's object number is reserved at its first use.
pub(crate) struct TextTarget<'a> {
    #[cfg(feature = "text")]
    pub(crate) fonts: &'a mut FontRegistry,
    #[cfg_attr(not(feature = "text"), expect(dead_code, reason = "only text reserves font objects"))]
    pub(crate) doc: &'a laminapdf_cos::Document,
}

/// The axis-aligned box around `r` mapped by `m`.
fn transformed_bounds(m: &Matrix, r: &RectPt) -> RectPt {
    let corners = [(r.x0, r.y0), (r.x1, r.y0), (r.x0, r.y1), (r.x1, r.y1)].map(|(x, y)| m.apply(x.get(), y.get()));
    let min = |f: fn(&(f64, f64)) -> f64| corners.iter().map(f).fold(f64::INFINITY, f64::min);
    let max = |f: fn(&(f64, f64)) -> f64| corners.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
    RectPt::new(
        Pt::new(min(|c| c.0)),
        Pt::new(min(|c| c.1)),
        Pt::new(max(|c| c.0)),
        Pt::new(max(|c| c.1)),
    )
}

/// A page's drawing surface. Every position and length is in **page space** ([ADR 0027]):
/// points from the top-left corner of the page as displayed (its `CropBox` after `/Rotate`),
/// y down, `/UserUnit` applied. The builder maps it to user space when the page is finished,
/// so what is drawn at the top-left is at the top-left in every viewer, whatever the page's
/// boxes, rotation and unit. Text puts its `y` on the baseline.
///
/// [`Surface::path`] builds a path and returns a [`PendingPath`] that borrows the surface
/// until it is painted ([`PendingPath::fill`], [`PendingPath::stroke`],
/// [`PendingPath::fill_stroke`], [`PendingPath::clip`]), so no transform, colour or state
/// change can come between building a path and painting it (PDF forbids them inside a path
/// object, ISO 32000-2 §8.2). A [`Surface::restore`] without a matching [`Surface::save`] is
/// ignored and a save left open is closed at the end of the page, so the content stream is
/// always balanced.
///
/// Geometry that PDF can't express (a NaN or infinite coordinate or matrix entry, a negative
/// line width, a dash pattern that is negative or all zeros, a miter limit below 1) is not
/// written: the first such value is reported by [`PageBuilder::finish`](crate::PageBuilder::finish)
/// as [`Error::InvalidGeometry`](crate::Error::InvalidGeometry).
///
/// ```
/// use laminapdf_build::{BuildOptions, Builder, Color, FillRule, PageSize, StrokeStyle};
/// use laminapdf_model::{Pt, RectPt};
///
/// let mut b = Builder::new(BuildOptions::default());
/// let mut page = b.page(PageSize::A4);
/// page.surface()
///     .fill_color(Color::hex("#3366cc")?)
///     .path(|p| {
///         p.rect(RectPt::from_xywh(Pt::new(72.0), Pt::new(72.0), Pt::new(200.0), Pt::new(120.0)));
///     })
///     .fill(FillRule::NonZero)
///     .opacity(0.5, 0.5)
///     .path(|p| {
///         p.circle(laminapdf_model::PointPt::from_points(300.0, 300.0), Pt::new(50.0));
///     })
///     .stroke(&StrokeStyle::default());
/// page.finish()?;
/// let doc = b.finish()?;
/// assert_eq!(doc.pages()?.len(), 1);
/// # Ok::<(), laminapdf_build::Error>(())
/// ```
///
/// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
#[derive(Debug, Default)]
pub struct Surface {
    commands: Vec<Command>,
    depth: usize,
    /// The page-space transform in effect, and those saved by open `save`s.
    ctm: Matrix,
    saved_ctm: Vec<Matrix>,
    /// The first invalid value given, reported when the page is finished.
    error: Option<&'static str>,
    /// Elements and artifacts begun and not yet ended.
    marks_open: usize,
    /// Whether anything drawn opened an element or artifact.
    tagged: bool,
}

/// A font size must be finite and positive.
#[cfg(feature = "text")]
fn check_size(size: Pt) -> Result<()> {
    if size.get().is_finite() && size.get() > 0.0 {
        Ok(())
    } else {
        Err(Error::InvalidGeometry {
            what: "a font size must be a positive number",
        })
    }
}

impl Surface {
    fn invalid(&mut self, what: &'static str) {
        self.error.get_or_insert(what);
    }

    /// The first invalid value given, if any.
    pub(crate) fn error(&self) -> Option<&'static str> {
        self.error
    }

    /// Saves the graphics state (`q`): transformations, colours, opacity and clipping until
    /// the matching [`Surface::restore`].
    pub fn save(&mut self) -> &mut Self {
        self.depth += 1;
        self.saved_ctm.push(self.ctm);
        self.commands.push(Command::Save);
        self
    }

    /// Restores the state saved by the last open [`Surface::save`] (`Q`); ignored when none
    /// is open.
    pub fn restore(&mut self) -> &mut Self {
        if self.depth > 0 {
            self.depth -= 1;
            self.ctm = self.saved_ctm.pop().unwrap_or_default();
            self.commands.push(Command::Restore);
        }
        self
    }

    /// Transforms what is drawn next (`cm`), in page space: points are mapped by `m` before
    /// they are placed on the page.
    pub fn transform(&mut self, m: Matrix) -> &mut Self {
        if m.to_array().iter().all(|v| v.is_finite()) {
            self.ctm = self.ctm.after(&m);
            self.commands.push(Command::Transform(m));
        } else {
            self.invalid("a matrix entry is not a finite number");
        }
        self
    }

    /// The colour for fills.
    pub fn fill_color(&mut self, color: Color) -> &mut Self {
        self.commands.push(Command::FillColor(color));
        self
    }

    /// The colour for strokes.
    pub fn stroke_color(&mut self, color: Color) -> &mut Self {
        self.commands.push(Command::StrokeColor(color));
        self
    }

    /// Constant opacity for fills and strokes (`/ca`, `/CA` in an `ExtGState`, §11.6.4.4),
    /// each clamped to `[0, 1]` (NaN is opaque). Equal settings share one `ExtGState` per
    /// document.
    pub fn opacity(&mut self, fill: f64, stroke: f64) -> &mut Self {
        self.commands.push(Command::State(GsKey::alpha(fill, stroke)));
        self
    }

    /// The blend mode (`/BM` in an `ExtGState`, §11.3.5).
    pub fn blend_mode(&mut self, mode: BlendMode) -> &mut Self {
        self.commands.push(Command::State(GsKey::Blend(mode)));
        self
    }

    /// Builds a path; paint it with the returned [`PendingPath`].
    pub fn path(&mut self, build: impl FnOnce(&mut PathBuilder)) -> PendingPath<'_> {
        let mut path = PathBuilder::default();
        build(&mut path);
        PendingPath { surface: self, path }
    }

    /// Draws `image` stretched over `rect` (`Do`, §8.9.5), upright as displayed.
    pub fn image(&mut self, image: &Image, rect: RectPt) -> &mut Self {
        if rect.is_finite() {
            self.commands.push(Command::Image(image.clone(), rect));
        } else {
            self.invalid("an image rectangle coordinate is not a finite number");
        }
        self
    }

    /// Draws `image` over `rect` as a `Figure` structure element with `alt` as its `/Alt`
    /// (alternate description, §14.9.3). A figure without one (`None`) is drawn too, and
    /// reported by [`Builder::tagging_diagnostics`](crate::Builder::tagging_diagnostics).
    /// Inside an artifact it is part of the artifact.
    pub fn figure(&mut self, image: &Image, rect: RectPt, alt: Option<&str>) -> &mut Self {
        let tag = Tag::new(TagKind::Figure);
        self.begin_tag(match alt {
            Some(alt) => tag.with_alt(alt),
            None => tag,
        });
        self.image(image, rect);
        self.end_tag()
    }

    /// Draws what `draw` draws inside the structure element `tag` (§14.7.2): text, paths and
    /// images become its marked content (MCIDs), elements opened inside it its children, and
    /// links its `Link` children, in reading order. Elements and artifacts `draw` leaves open
    /// are ended with it. Inside an artifact, `tag` is part of the artifact.
    ///
    /// A page that uses an element or artifact makes the document tagged (see
    /// [`BuildOptions::with_tagged`](crate::BuildOptions::with_tagged)). Elements are per page:
    /// one doesn't continue on the next page.
    ///
    /// ```
    /// use laminapdf_build::{BuildOptions, Builder, PageSize, Standard14, Tag, TagKind};
    /// use laminapdf_model::{PointPt, Pt};
    ///
    /// let mut b = Builder::new(BuildOptions::default());
    /// let helvetica = b.standard_font(Standard14::Helvetica)?;
    /// let mut page = b.page(PageSize::A4);
    /// page.surface().tag(TagKind::Sect, |s| {
    ///     s.tag(Tag::new(TagKind::H1).with_lang("en"), |s| {
    ///         s.text("Tagged", &helvetica, Pt::new(24.0), PointPt::from_points(72.0, 96.0)).map(|_| ())
    ///     })
    /// })?;
    /// page.finish()?;
    /// assert!(b.tagging_diagnostics().is_empty());
    /// # Ok::<(), laminapdf_build::Error>(())
    /// ```
    ///
    /// # Errors
    /// What `draw` returns (the element is ended either way).
    pub fn tag<T>(&mut self, tag: impl Into<Tag>, draw: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let depth = self.marks_open;
        self.begin_tag(tag);
        let drawn = draw(self);
        self.end_to(depth);
        drawn
    }

    /// Draws what `draw` draws as an artifact of `kind` (§14.8.2.2): content that isn't part
    /// of the document's logical content (rules, running heads, backgrounds), outside the
    /// structure tree. Elements opened inside it are part of it.
    ///
    /// # Errors
    /// What `draw` returns.
    pub fn artifact<T>(&mut self, kind: ArtifactKind, draw: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let depth = self.marks_open;
        self.begin_artifact(kind);
        let drawn = draw(self);
        self.end_to(depth);
        drawn
    }

    /// Opens the structure element `tag` until the matching [`Surface::end_tag`]: the
    /// begin/end form of [`Surface::tag`], for bindings that can't pass a closure. One left
    /// open is ended at the end of the page.
    pub fn begin_tag(&mut self, tag: impl Into<Tag>) -> &mut Self {
        self.marks_open += 1;
        self.tagged = true;
        self.commands.push(Command::BeginTag(Box::new(tag.into())));
        self
    }

    /// Opens an artifact until the matching [`Surface::end_tag`] (see [`Surface::artifact`]).
    pub fn begin_artifact(&mut self, kind: ArtifactKind) -> &mut Self {
        self.marks_open += 1;
        self.tagged = true;
        self.commands.push(Command::BeginArtifact(kind));
        self
    }

    /// Ends the innermost open element or artifact; ignored when none is open.
    pub fn end_tag(&mut self) -> &mut Self {
        if self.marks_open > 0 {
            self.marks_open -= 1;
            self.commands.push(Command::EndMarked);
        }
        self
    }

    fn end_to(&mut self, depth: usize) {
        while self.marks_open > depth {
            self.end_tag();
        }
    }

    /// Whether anything drawn opens an element or artifact.
    pub(crate) const fn uses_tags(&self) -> bool {
        self.tagged
    }

    /// Notes that a text run became an element ([`TextRun::tag`]).
    #[cfg(feature = "text")]
    pub(crate) const fn mark_tagged(&mut self) {
        self.tagged = true;
    }

    /// Removes the last thing drawn when it is a text run, and says whether it did: a caller
    /// that draws a run and then fails to change it ([`TextRun::features`],
    /// [`TextRun::language`], [`TextRun::direction`] lay it out again) can leave nothing
    /// behind, as the bindings do.
    #[cfg(feature = "text")]
    pub fn discard_last_text(&mut self) -> bool {
        let last = matches!(self.commands.last(), Some(Command::Text(_)));
        if last {
            self.commands.pop();
        }
        last
    }

    /// Whether anything drawn is text (so finishing the page records glyphs).
    #[cfg(feature = "text")]
    pub(crate) fn has_text(&self) -> bool {
        self.commands.iter().any(|c| matches!(c, Command::Text(_)))
    }

    #[cfg(feature = "text")]
    pub(crate) fn text_command(&self, index: usize) -> Option<&TextCommand> {
        match self.commands.get(index) {
            Some(Command::Text(t)) => Some(t),
            _ => None,
        }
    }

    #[cfg(feature = "text")]
    pub(crate) fn text_command_mut(&mut self, index: usize) -> Option<&mut TextCommand> {
        match self.commands.get_mut(index) {
            Some(Command::Text(t)) => Some(t),
            _ => None,
        }
    }

    /// Draws `text` in `font` at `size` points with the first glyph's origin at `at`: **`at.y`
    /// is the baseline** ([ADR 0027] §5), `at.x` the left edge of the run. The text is shaped
    /// with HarfRust (ligatures, kerning, marks, Arabic joining, Indic reordering), its
    /// paragraphs resolved with the bidi algorithm and drawn in visual order; a line break in
    /// the text starts a new line 1.2 × `size` lower. Spaces are drawn as the font's space
    /// glyph, so extractors see them; clusters whose glyphs can't spell their text through
    /// ToUnicode get `/ActualText`. A standard 14 font draws one glyph per character, left to
    /// right, with its AFM widths (no kerning).
    ///
    /// The returned [`TextRun`] changes how the run is drawn (colour, render mode, features)
    /// and can make it a link.
    ///
    /// # Errors
    /// [`Error::InvalidGeometry`] for a size that isn't finite and positive or a position that
    /// isn't finite; [`Error::Text`] when the font has no glyph for a character
    /// (`missing-glyph`) or can't be read.
    ///
    /// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
    #[cfg(feature = "text")]
    pub fn text(&mut self, text: &str, font: &Font, size: Pt, at: PointPt) -> Result<TextRun<'_>> {
        check_size(size)?;
        if !(at.x.get().is_finite() && at.y.get().is_finite()) {
            return Err(Error::InvalidGeometry {
                what: "a text position is not a finite number",
            });
        }
        let spec = Spec {
            text: text.to_owned(),
            font: font.clone(),
            size: size.get(),
            align: Alignment::Left,
            direction: None,
            language: None,
            features: Vec::new(),
            width: None,
        };
        let lines = lay_out(&spec)?;
        Ok(self.push_text(spec, at, 1.2 * size.get(), lines))
    }

    #[cfg(feature = "text")]
    fn push_text(&mut self, spec: Spec, origin: PointPt, leading: f64, lines: Vec<text::LaidLine>) -> TextRun<'_> {
        let index = self.commands.len();
        self.commands.push(Command::Text(Box::new(TextCommand {
            spec,
            origin,
            leading,
            lines,
            fill: None,
            mode: TextRenderMode::Fill,
            link: None,
            tag: None,
            ctm: self.ctm,
        })));
        TextRun::new(self, index)
    }

    /// Lays `text` out in `rect` and draws it: lines broken greedily at Unicode line-break
    /// opportunities (UAX #14; a word wider than the box between grapheme clusters), each
    /// paragraph's direction from its first strong character unless `options` sets one,
    /// lines aligned as `options` says, the first baseline the font's ascent below the
    /// box's top and each next one `line_height × size` lower. No line is wider than the box,
    /// except one holding a single cluster wider than the box on its own, which counts as
    /// overflow.
    ///
    /// Lines that don't fit the box's height are overflow: with [`Overflow::Clip`] the lines
    /// that fit are drawn and [`TextLayout::overflow`] says where the rest starts; with
    /// [`Overflow::Error`] nothing is drawn.
    ///
    /// ```
    /// use laminapdf_build::{BuildOptions, Builder, PageSize, Standard14, TextBoxOptions};
    /// use laminapdf_model::{Pt, RectPt};
    ///
    /// let mut b = Builder::new(BuildOptions::default());
    /// let times = b.standard_font(Standard14::TimesRoman)?;
    /// let mut page = b.page(PageSize::A4);
    /// let r = RectPt::from_xywh(Pt::new(72.0), Pt::new(72.0), Pt::new(120.0), Pt::new(200.0));
    /// let layout = page.surface().text_box(
    ///     "The quick brown fox jumps over the lazy dog.",
    ///     &times,
    ///     Pt::new(12.0),
    ///     r,
    ///     &TextBoxOptions::default(),
    /// )?;
    /// assert!(layout.lines.len() > 1 && !layout.overflowed());
    /// assert!(layout.lines.iter().all(|l| l.rect.width() <= r.width()));
    /// # Ok::<(), laminapdf_build::Error>(())
    /// ```
    ///
    /// # Errors
    /// As [`Surface::text`]; [`Error::InvalidGeometry`] for a rectangle that isn't finite or
    /// has no width, or a line height that isn't finite and positive;
    /// [`Error::TextOverflow`] with [`Overflow::Error`].
    #[cfg(feature = "text")]
    pub fn text_box(
        &mut self,
        text: &str,
        font: &Font,
        size: Pt,
        rect: RectPt,
        options: &TextBoxOptions,
    ) -> Result<TextLayout> {
        check_size(size)?;
        if !rect.is_finite() || rect.width().get() <= 0.0 {
            return Err(Error::InvalidGeometry {
                what: "a text box must be finite and wider than 0",
            });
        }
        if !(options.line_height.is_finite() && options.line_height > 0.0) {
            return Err(Error::InvalidGeometry {
                what: "a line height must be a positive number",
            });
        }
        let size_pt = size.get();
        let spec = Spec {
            text: text.to_owned(),
            font: font.clone(),
            size: size_pt,
            align: options.align,
            direction: options.direction,
            language: options.language.clone(),
            features: options.features.clone(),
            width: Some(rect.width().get()),
        };
        let mut lines = lay_out(&spec)?;
        let leading = options.line_height * size_pt;
        let first = rect.y0.get() + font.ascent(size_pt);
        let descent = font.descent(size_pt);
        let limit = rect.width().get();
        let fits = |i: usize, line: &text::LaidLine| {
            #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
            let bottom = first + i as f64 * leading + descent;
            // A hair of tolerance for the f32 widths the layout measured with.
            bottom <= rect.y1.get() + 1e-9 && line.width <= limit + 1e-3
        };
        let fit = lines
            .iter()
            .enumerate()
            .position(|(i, l)| !fits(i, l))
            .unwrap_or(lines.len());
        let overflow = lines.get(fit).map(|l| l.range.start);
        if overflow.is_some() && options.overflow == Overflow::Error {
            return Err(Error::TextOverflow {
                fits: fit,
                needed: lines.len(),
            });
        }
        lines.truncate(fit);
        let x = rect.x0.get();
        let boxes = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
                let baseline = first + i as f64 * leading;
                LineBox {
                    range: l.range.clone(),
                    content: l.content.clone(),
                    rect: RectPt::new(
                        Pt::new(x + l.x),
                        Pt::new(baseline - font.ascent(size_pt)),
                        Pt::new(x + l.x + l.width),
                        Pt::new(baseline + descent),
                    ),
                    baseline: Pt::new(baseline),
                }
            })
            .collect();
        self.push_text(spec, PointPt::new(rect.x0, Pt::new(first)), leading, lines);
        Ok(TextLayout { lines: boxes, overflow })
    }

    /// A link annotation over `rect` (page space, through the current transform as an
    /// axis-aligned box): a URI, a page of this document or a named destination, with
    /// `contents` as its `/Contents` (alternate text). No border; printed with the page.
    pub fn link(&mut self, rect: RectPt, target: LinkTarget, contents: Option<String>) -> &mut Self {
        let finite = match &target {
            LinkTarget::Page { fit, .. } => fit_is_valid(fit),
            _ => true,
        };
        if rect.is_finite() && finite {
            self.commands.push(Command::Link(
                transformed_bounds(&self.ctm, &rect),
                LinkSpec { target, contents },
            ));
        } else {
            self.invalid("a link rectangle or destination coordinate is not a finite number");
        }
        self
    }

    /// The content stream for a page with `frame`, registering the resources it names.
    #[cfg_attr(
        not(feature = "text"),
        expect(clippy::unnecessary_wraps, reason = "only text can fail while a page is written")
    )]
    pub(crate) fn render(
        &self,
        frame: &PageFrame,
        resources: &mut PageResources,
        target: &mut TextTarget<'_>,
        marking: Marking,
    ) -> Result<Rendered> {
        let to_user = frame.to_user_space_matrix();
        // The frame maps page space to user space with a positive /UserUnit: never singular.
        let from_user = to_user.invert().unwrap_or(Matrix::IDENTITY);
        let unit = frame.user_unit();
        let mut out = Writer::new(to_user, unit, marking);
        // Only text writes through the target (fonts are reserved at first use).
        #[cfg(not(feature = "text"))]
        let _ = target;
        let mut links = Vec::new();
        for command in &self.commands {
            match command {
                Command::Link(rect, link) => {
                    out.link();
                    links.push((frame.rect_to_user_space(rect), link.clone()));
                }
                #[cfg(feature = "text")]
                Command::Text(t) => {
                    if let Some(tag) = &t.tag {
                        out.marking.begin_tag(&mut out.content, tag);
                    }
                    let link = t.link.as_ref().zip(t.bounds());
                    let in_link = link.is_some() && out.marking.begin_link(&mut out.content);
                    out.text(t, resources, target)?;
                    if let Some((link, bounds)) = link {
                        out.marking.link();
                        let rect = transformed_bounds(&t.ctm, &bounds);
                        links.push((frame.rect_to_user_space(&rect), link.clone()));
                    }
                    if in_link {
                        out.marking.end(&mut out.content);
                    }
                    if t.tag.is_some() {
                        out.marking.end(&mut out.content);
                    }
                }
                Command::BeginTag(tag) => out.marking.begin_tag(&mut out.content, tag),
                Command::BeginArtifact(kind) => out.marking.begin_artifact(&mut out.content, *kind),
                Command::EndMarked => out.marking.end(&mut out.content),
                _ => out.command(command, &from_user, resources),
            }
        }
        let marks = out.marking.finish(&mut out.content);
        for _ in 0..self.depth {
            out.content.restore_state();
        }
        Ok(Rendered {
            content: out.content.finish().into_vec(),
            links,
            marks,
        })
    }
}

/// An operand: non-finite values (which PDF can't express) are written as 0.
#[expect(
    clippy::cast_possible_truncation,
    reason = "content-stream operands are f32 in pdf-writer; out-of-range values become 0 below"
)]
pub(crate) fn n(v: f64) -> f32 {
    let v = v as f32;
    if v.is_finite() { v } else { 0.0 }
}
