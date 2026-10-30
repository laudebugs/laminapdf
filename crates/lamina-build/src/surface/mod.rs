//! [`Surface`]: what is drawn on a page, in page space ([ADR 0027]), written as a content
//! stream with `pdf_writer::Content` when the page is finished.
//!
//! [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md

pub(crate) mod color;
pub(crate) mod image;
pub(crate) mod path;
pub(crate) mod style;
pub(crate) mod text;
pub(crate) mod textbox;

use lamina_model::{PageFrame, PointPt, Pt, RectPt, UserSpaceRect};
use lamina_text::{Alignment, FontRegistry, TjElement, pdf_run};
use pdf_writer::types::{ColorSpaceOperand, LineCapStyle, LineJoinStyle, TextRenderingMode};
use pdf_writer::{Content, Name, Str, TextStr};

use crate::error::{Error, Result};
use crate::nav::LinkTarget;
use crate::nav::dest::fit_is_finite;
use crate::resources::{GsKey, PageResources};
use color::Color;
use image::Image;
use path::{FillRule, Matrix, PathBuilder, Segment};
use style::{BlendMode, LineCap, LineJoin, StrokeStyle};
use text::{Font, LinkSpec, Spec, TextCommand, TextRenderMode, TextRun, lay_out};
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
    Text(Box<TextCommand>),
    /// A link annotation over a page-space rectangle (already through the transform).
    Link(RectPt, LinkSpec),
}

/// A page's content stream and the links it asked for.
pub(crate) struct Rendered {
    pub(crate) content: Vec<u8>,
    /// Each link's rectangle in user space, in order of drawing.
    pub(crate) links: Vec<(UserSpaceRect, LinkSpec)>,
}

/// What text needs while a page is written: the document's fonts and the document, where
/// a font's object number is reserved at its first use.
pub(crate) struct TextTarget<'a> {
    pub(crate) fonts: &'a mut FontRegistry,
    pub(crate) doc: &'a lamina_cos::Document,
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
/// boxes, rotation and unit. Text ([B5b]) puts its `y` on the baseline.
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
/// use lamina_build::{BuildOptions, Builder, Color, FillRule, PageSize, StrokeStyle};
/// use lamina_model::{Pt, RectPt};
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
///         p.circle(lamina_model::PointPt::from_points(300.0, 300.0), Pt::new(50.0));
///     })
///     .stroke(&StrokeStyle::default());
/// page.finish()?;
/// let doc = b.finish()?;
/// assert_eq!(doc.pages()?.len(), 1);
/// # Ok::<(), lamina_build::Error>(())
/// ```
///
/// [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
/// [B5b]: https://github.com/laudebugs/lamina/blob/main/docs/todos/ongoing/B5b-builder-text-and-document-features.md
#[derive(Debug, Default)]
pub struct Surface {
    commands: Vec<Command>,
    depth: usize,
    /// The page-space transform in effect, and those saved by open `save`s.
    ctm: Matrix,
    saved_ctm: Vec<Matrix>,
    /// The first invalid value given, reported when the page is finished.
    error: Option<&'static str>,
}

/// A path built by [`Surface::path`], waiting to be painted. Dropping it draws nothing.
#[derive(Debug)]
#[must_use = "a path is drawn only when it is painted (fill, stroke, fill_stroke or clip)"]
pub struct PendingPath<'s> {
    surface: &'s mut Surface,
    path: PathBuilder,
}

impl<'s> PendingPath<'s> {
    fn paint(self, paint: Paint) -> &'s mut Surface {
        let Self { surface, mut path } = self;
        if !path.is_finite() {
            surface.invalid("a path coordinate is not a finite number");
        }
        if let Paint::Stroke(style) | Paint::FillStroke(_, style) = &paint
            && let Some(problem) = style.problem()
        {
            surface.invalid(problem);
        }
        let segments = path.take();
        if !segments.is_empty() {
            surface.commands.push(Command::Paint(segments, paint));
        }
        surface
    }

    /// Strokes the path (`S`).
    pub fn stroke(self, style: &StrokeStyle) -> &'s mut Surface {
        self.paint(Paint::Stroke(style.clone()))
    }

    /// Fills the path (`f` or `f*`).
    pub fn fill(self, rule: FillRule) -> &'s mut Surface {
        self.paint(Paint::Fill(rule))
    }

    /// Fills, then strokes the path (`B` or `B*`).
    pub fn fill_stroke(self, rule: FillRule, style: &StrokeStyle) -> &'s mut Surface {
        self.paint(Paint::FillStroke(rule, style.clone()))
    }

    /// Intersects the clipping region with the path (`W n` or `W* n`) until the next
    /// [`Surface::restore`].
    pub fn clip(self, rule: FillRule) -> &'s mut Surface {
        self.paint(Paint::Clip(rule))
    }
}

/// A font size must be finite and positive.
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

    pub(crate) fn text_command(&self, index: usize) -> Option<&TextCommand> {
        match self.commands.get(index) {
            Some(Command::Text(t)) => Some(t),
            _ => None,
        }
    }

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
    /// [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
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
    /// use lamina_build::{BuildOptions, Builder, PageSize, Standard14, TextBoxOptions};
    /// use lamina_model::{Pt, RectPt};
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
    /// # Ok::<(), lamina_build::Error>(())
    /// ```
    ///
    /// # Errors
    /// As [`Surface::text`]; [`Error::InvalidGeometry`] for a rectangle that isn't finite or
    /// has no width, or a line height that isn't finite and positive;
    /// [`Error::TextOverflow`] with [`Overflow::Error`].
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
        let _ = self.push_text(spec, PointPt::new(rect.x0, Pt::new(first)), leading, lines);
        Ok(TextLayout { lines: boxes, overflow })
    }

    /// A link annotation over `rect` (page space, through the current transform as an
    /// axis-aligned box): a URI, a page of this document or a named destination, with
    /// `contents` as its `/Contents` (alternate text). No border; printed with the page.
    pub fn link(&mut self, rect: RectPt, target: LinkTarget, contents: Option<String>) -> &mut Self {
        let finite = match &target {
            LinkTarget::Page { fit, .. } => fit_is_finite(fit),
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
    pub(crate) fn render(
        &self,
        frame: &PageFrame,
        resources: &mut PageResources,
        target: &mut TextTarget<'_>,
    ) -> Result<Rendered> {
        let to_user = Matrix::new(frame.to_user_space_matrix());
        // The frame maps page space to user space with a positive /UserUnit: never singular.
        let from_user = to_user.invert().unwrap_or(Matrix::IDENTITY);
        let unit = frame.user_unit();
        let mut out = Writer {
            content: Content::new(),
            to_user,
            unit,
            state: State::default(),
            saved: Vec::new(),
        };
        let mut links = Vec::new();
        for command in &self.commands {
            match command {
                Command::Link(rect, link) => links.push((frame.rect_to_user_space(rect), link.clone())),
                Command::Text(t) => {
                    out.text(t, resources, target)?;
                    if let (Some(link), Some(bounds)) = (&t.link, t.bounds()) {
                        let rect = transformed_bounds(&t.ctm, &bounds);
                        links.push((frame.rect_to_user_space(&rect), link.clone()));
                    }
                }
                _ => out.command(command, &from_user, resources),
            }
        }
        for _ in 0..self.depth {
            out.content.restore_state();
        }
        Ok(Rendered {
            content: out.content.finish().into_vec(),
            links,
        })
    }
}

/// What the content stream has set, to skip operators that would change nothing. `None` is
/// "not set by us" (the PDF default may still hold, but nothing is assumed).
#[derive(Debug, Clone, Default)]
struct State {
    fill: Option<Color>,
    stroke: Option<Color>,
    line: Option<StrokeStyle>,
    /// `Tr` (0, fill, at the start of a page: text state is part of the graphics state).
    mode: TextRenderMode,
}

struct Writer {
    content: Content,
    to_user: Matrix,
    unit: f64,
    state: State,
    saved: Vec<State>,
}

/// An operand: non-finite values (which PDF can't express) are written as 0.
#[expect(
    clippy::cast_possible_truncation,
    reason = "content-stream operands are f32 in pdf-writer; out-of-range values become 0 below"
)]
fn n(v: f64) -> f32 {
    let v = v as f32;
    if v.is_finite() { v } else { 0.0 }
}

fn matrix(m: &Matrix) -> [f32; 6] {
    m.to_array().map(n)
}

impl Writer {
    fn command(&mut self, command: &Command, from_user: &Matrix, resources: &mut PageResources) {
        match command {
            Command::Save => {
                self.saved.push(self.state.clone());
                self.content.save_state();
            }
            Command::Restore => {
                self.state = self.saved.pop().unwrap_or_default();
                self.content.restore_state();
            }
            Command::Transform(m) => {
                // Conjugated: a page-space transform, expressed in user space.
                let m = self.to_user.after(m).after(from_user);
                self.content.transform(matrix(&m));
            }
            Command::FillColor(color) => self.color(color, true, resources),
            Command::StrokeColor(color) => self.color(color, false, resources),
            Command::State(key) => {
                let name = resources.gstate(*key);
                self.content.set_parameters(Name(name.as_bytes()));
            }
            Command::Paint(segments, paint) => self.paint(segments, paint),
            // Written by `render` itself (they need the fonts or produce annotations).
            Command::Text(_) | Command::Link(..) => {}
            Command::Image(image, r) => {
                let name = resources.image(image);
                let (w, h) = (r.width().get(), r.height().get());
                // The unit square, upright: its bottom-left at the rect's bottom-left.
                let place = Matrix::new([w, 0.0, 0.0, -h, r.x0.get(), r.y1.get()]);
                self.content
                    .save_state()
                    .transform(matrix(&self.to_user.after(&place)))
                    .x_object(Name(name.as_bytes()))
                    .restore_state();
            }
        }
    }

    /// A text object: `BT`, `Tr` when it changes, `Tf`, then per line (and per shaped run) a
    /// `Tm` at the run's origin and its `TJ` arrays, with `Ts` rises and `/ActualText` spans
    /// where `pdf_run` asks for them; `ET`. A run with its own fill colour is wrapped in
    /// `q … Q`.
    fn text(&mut self, t: &TextCommand, resources: &mut PageResources, target: &mut TextTarget<'_>) -> Result<()> {
        if t.lines.iter().all(|l| l.content.is_empty()) {
            return Ok(());
        }
        let key = t.spec.font.key;
        let object = target.fonts.reference(key, target.doc)?;
        let name = resources.font(object);
        if let Some(color) = &t.fill {
            self.saved.push(self.state.clone());
            self.content.save_state();
            self.color(color, true, resources);
        }
        self.content.begin_text();
        if self.state.mode != t.mode {
            self.state.mode = t.mode;
            self.content.set_text_rendering_mode(match t.mode {
                TextRenderMode::Fill => TextRenderingMode::Fill,
                TextRenderMode::Stroke => TextRenderingMode::Stroke,
                TextRenderMode::FillStroke => TextRenderingMode::FillStroke,
                TextRenderMode::Invisible => TextRenderingMode::Invisible,
            });
        }
        let size = n(t.spec.size);
        self.content.set_font(Name(name.as_bytes()), size);
        let text = t.spec.text.as_str();
        let x0 = t.origin.x.get();
        let mut rise = 0.0_f32;
        for (i, line) in t.lines.iter().enumerate() {
            let baseline = t.baseline(i);
            if line.runs.is_empty() {
                let content = text.get(line.content.clone()).unwrap_or_default();
                if content.is_empty() {
                    continue;
                }
                let encoded = target.fonts.encode(key, content)?;
                self.text_matrix(x0 + line.x, baseline);
                self.content.show(Str(&encoded.bytes));
                continue;
            }
            for (run, x) in &line.runs {
                let pdf = pdf_run(run, text, target.fonts, key)?;
                self.text_matrix(x0 + line.x + x, baseline);
                for piece in &pdf.pieces {
                    if let Some(actual) = &piece.actual_text {
                        resources.uses_actual_text();
                        self.content
                            .begin_marked_content_with_properties(Name(b"Span"))
                            .properties()
                            .actual_text(TextStr(actual));
                    }
                    for segment in &piece.segments {
                        let wanted = n(segment.rise * t.spec.size / 1000.0);
                        if wanted.to_bits() != rise.to_bits() {
                            self.content.set_rise(wanted);
                            rise = wanted;
                        }
                        let mut show = self.content.show_positioned();
                        let mut items = show.items();
                        for element in &segment.tj {
                            match element {
                                TjElement::Codes(codes) => {
                                    items.show(Str(codes));
                                }
                                TjElement::Adjust(amount) => {
                                    items.adjust(n(*amount));
                                }
                            }
                        }
                    }
                    if piece.actual_text.is_some() {
                        self.content.end_marked_content();
                    }
                }
            }
        }
        if rise != 0.0 {
            self.content.set_rise(0.0);
        }
        self.content.end_text();
        if t.fill.is_some() {
            self.state = self.saved.pop().unwrap_or_default();
            self.content.restore_state();
        }
        Ok(())
    }

    /// `Tm` placing text space's origin at `(x, baseline)` in page space, upright as
    /// displayed (text space is y up, page space y down).
    fn text_matrix(&mut self, x: f64, baseline: f64) {
        let place = Matrix::new([1.0, 0.0, 0.0, -1.0, x, baseline]);
        self.content.set_text_matrix(matrix(&self.to_user.after(&place)));
    }

    fn color(&mut self, color: &Color, fill: bool, resources: &mut PageResources) {
        let current = if fill {
            &mut self.state.fill
        } else {
            &mut self.state.stroke
        };
        if current.as_ref() == Some(color) {
            return;
        }
        *current = Some(color.clone());
        let c = &mut self.content;
        match (color, fill) {
            (Color::Gray(g), true) => c.set_fill_gray(n(*g)),
            (Color::Gray(g), false) => c.set_stroke_gray(n(*g)),
            (Color::Rgb(r, g, b), true) => c.set_fill_rgb(n(*r), n(*g), n(*b)),
            (Color::Rgb(r, g, b), false) => c.set_stroke_rgb(n(*r), n(*g), n(*b)),
            (Color::Cmyk(cy, m, y, k), true) => c.set_fill_cmyk(n(*cy), n(*m), n(*y), n(*k)),
            (Color::Cmyk(cy, m, y, k), false) => c.set_stroke_cmyk(n(*cy), n(*m), n(*y), n(*k)),
            (Color::Icc(profile, components), fill) => {
                let name = resources.color_space(profile);
                let space = ColorSpaceOperand::Named(Name(name.as_bytes()));
                let values = components.iter().map(|v| n(*v));
                if fill {
                    c.set_fill_color_space(space).set_fill_color(values)
                } else {
                    c.set_stroke_color_space(space).set_stroke_color(values)
                }
            }
        };
    }

    fn line(&mut self, style: &StrokeStyle) {
        let unit = self.unit;
        let previous = self.state.line.replace(style.clone());
        let c = &mut self.content;
        let changed = |f: &dyn Fn(&StrokeStyle) -> bool| previous.as_ref().is_none_or(|p| !f(p));
        if changed(&|p| p.width == style.width) {
            c.set_line_width(n(style.width.get() / unit));
        }
        if changed(&|p| p.cap == style.cap) {
            c.set_line_cap(match style.cap {
                LineCap::Butt => LineCapStyle::ButtCap,
                LineCap::Round => LineCapStyle::RoundCap,
                LineCap::Square => LineCapStyle::ProjectingSquareCap,
            });
        }
        if changed(&|p| p.join == style.join) {
            c.set_line_join(match style.join {
                LineJoin::Miter => LineJoinStyle::MiterJoin,
                LineJoin::Round => LineJoinStyle::RoundJoin,
                LineJoin::Bevel => LineJoinStyle::BevelJoin,
            });
        }
        if changed(&|p| p.miter_limit.to_bits() == style.miter_limit.to_bits()) {
            c.set_miter_limit(n(style.miter_limit));
        }
        if changed(&|p| p.dash == style.dash) {
            match &style.dash {
                Some(dash) => {
                    c.set_dash_pattern(dash.array.iter().map(|d| n(d.get() / unit)), n(dash.phase.get() / unit))
                }
                None => c.set_dash_pattern([], 0.0),
            };
        }
    }

    fn paint(&mut self, segments: &[Segment], paint: &Paint) {
        if let Paint::Stroke(style) | Paint::FillStroke(_, style) = paint {
            self.line(style);
        }
        let to_user = self.to_user;
        let at = |p: &PointPt| {
            let (x, y) = to_user.apply(p.x.get(), p.y.get());
            (n(x), n(y))
        };
        let c = &mut self.content;
        for segment in segments {
            match segment {
                Segment::MoveTo(p) => {
                    let (x, y) = at(p);
                    c.move_to(x, y);
                }
                Segment::LineTo(p) => {
                    let (x, y) = at(p);
                    c.line_to(x, y);
                }
                Segment::CubicTo(c1, c2, p) => {
                    let ((x1, y1), (x2, y2), (x3, y3)) = (at(c1), at(c2), at(p));
                    c.cubic_to(x1, y1, x2, y2, x3, y3);
                }
                Segment::Close => {
                    c.close_path();
                }
                Segment::Rect(r) => {
                    // `re` starts (and leaves the current point) at its first corner: the
                    // rect's page-space top-left, so the path builder's current point holds.
                    // The frame maps page axes onto user axes, so the corners `a` and `b` span the
                    // same rectangle with signed sides whatever the rotation.
                    let a = to_user.apply(r.x0.get(), r.y0.get());
                    let b = to_user.apply(r.x1.get(), r.y1.get());
                    c.rect(n(a.0), n(a.1), n(b.0 - a.0), n(b.1 - a.1));
                }
            }
        }
        match paint {
            Paint::Stroke(_) => c.stroke(),
            Paint::Fill(FillRule::NonZero) => c.fill_nonzero(),
            Paint::Fill(FillRule::EvenOdd) => c.fill_even_odd(),
            Paint::FillStroke(FillRule::NonZero, _) => c.fill_nonzero_and_stroke(),
            Paint::FillStroke(FillRule::EvenOdd, _) => c.fill_even_odd_and_stroke(),
            Paint::Clip(FillRule::NonZero) => c.clip_nonzero().end_path(),
            Paint::Clip(FillRule::EvenOdd) => c.clip_even_odd().end_path(),
        };
    }
}
