//! [`Surface`]: what is drawn on a page, in page space ([ADR 0027]), written as a content
//! stream with `pdf_writer::Content` when the page is finished.
//!
//! [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md

pub(crate) mod color;
pub(crate) mod image;
pub(crate) mod path;
pub(crate) mod style;

use lamina_model::{PageFrame, RectPt};
use pdf_writer::types::{ColorSpaceOperand, LineCapStyle, LineJoinStyle};
use pdf_writer::{Content, Name};

use crate::resources::{GsKey, PageResources};
use color::Color;
use image::Image;
use path::{FillRule, Matrix, PathBuilder, Segment};
use style::{BlendMode, LineCap, LineJoin, StrokeStyle};

/// How a path is painted.
#[derive(Debug, Clone, PartialEq)]
enum Paint {
    Stroke(StrokeStyle),
    Fill(FillRule),
    FillStroke(FillRule, StrokeStyle),
    Clip(FillRule),
}

/// One drawing operation, in page space.
#[derive(Debug, Clone, PartialEq)]
enum Command {
    Save,
    Restore,
    Transform(Matrix),
    FillColor(Color),
    StrokeColor(Color),
    State(GsKey),
    Paint(Vec<Segment>, Paint),
    Image(Image, RectPt),
}

/// A page's drawing surface. Every position and length is in **page space** ([ADR 0027]):
/// points from the top-left corner of the page as displayed (its `CropBox` after `/Rotate`),
/// y down, `/UserUnit` applied. The builder maps it to user space when the page is finished,
/// so what is drawn at the top-left is at the top-left in every viewer, whatever the page's
/// boxes, rotation and unit. Text ([B5b]) puts its `y` on the baseline.
///
/// A path is built with [`Surface::path`] and used by the next [`Surface::stroke`],
/// [`Surface::fill`], [`Surface::fill_stroke`] or [`Surface::clip`]; painting without a path
/// draws nothing. A [`Surface::restore`] without a matching [`Surface::save`] is ignored and
/// a save left open is closed at the end of the page, so the content stream is always
/// balanced.
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
/// [B5b]: https://github.com/laudebugs/lamina/blob/main/docs/todos/ready/B5b-builder-text-and-document-features.md
#[derive(Debug, Default)]
pub struct Surface {
    commands: Vec<Command>,
    path: PathBuilder,
    depth: usize,
}

impl Surface {
    /// Saves the graphics state (`q`): transformations, colours, opacity and clipping until
    /// the matching [`Surface::restore`].
    pub fn save(&mut self) -> &mut Self {
        self.depth += 1;
        self.commands.push(Command::Save);
        self
    }

    /// Restores the state saved by the last open [`Surface::save`] (`Q`); ignored when none
    /// is open.
    pub fn restore(&mut self) -> &mut Self {
        if self.depth > 0 {
            self.depth -= 1;
            self.commands.push(Command::Restore);
        }
        self
    }

    /// Transforms what is drawn next (`cm`), in page space: points are mapped by `m` before
    /// they are placed on the page.
    pub fn transform(&mut self, m: Matrix) -> &mut Self {
        self.commands.push(Command::Transform(m));
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
    /// each clamped to `[0, 1]`. Equal settings share one `ExtGState` per document.
    pub fn opacity(&mut self, fill: f64, stroke: f64) -> &mut Self {
        self.commands.push(Command::State(GsKey::alpha(fill, stroke)));
        self
    }

    /// The blend mode (`/BM` in an `ExtGState`, §11.3.5).
    pub fn blend_mode(&mut self, mode: BlendMode) -> &mut Self {
        self.commands.push(Command::State(GsKey::Blend(mode)));
        self
    }

    /// Builds the path the next painting operation uses (appending to one not yet painted).
    pub fn path(&mut self, build: impl FnOnce(&mut PathBuilder)) -> &mut Self {
        build(&mut self.path);
        self
    }

    fn paint(&mut self, paint: Paint) -> &mut Self {
        let segments = self.path.take();
        if !segments.is_empty() {
            self.commands.push(Command::Paint(segments, paint));
        }
        self
    }

    /// Strokes the path (`S`).
    pub fn stroke(&mut self, style: &StrokeStyle) -> &mut Self {
        self.paint(Paint::Stroke(style.clone()))
    }

    /// Fills the path (`f` or `f*`).
    pub fn fill(&mut self, rule: FillRule) -> &mut Self {
        self.paint(Paint::Fill(rule))
    }

    /// Fills, then strokes the path (`B` or `B*`).
    pub fn fill_stroke(&mut self, rule: FillRule, style: &StrokeStyle) -> &mut Self {
        self.paint(Paint::FillStroke(rule, style.clone()))
    }

    /// Intersects the clipping region with the path (`W n` or `W* n`) until the next
    /// [`Surface::restore`].
    pub fn clip(&mut self, rule: FillRule) -> &mut Self {
        self.paint(Paint::Clip(rule))
    }

    /// Draws `image` stretched over `rect` (`Do`, §8.9.5), upright as displayed.
    pub fn image(&mut self, image: &Image, rect: RectPt) -> &mut Self {
        self.commands.push(Command::Image(image.clone(), rect));
        self
    }

    /// The content stream for a page with `frame`, registering the resources it names.
    pub(crate) fn render(&self, frame: &PageFrame, resources: &mut PageResources) -> Vec<u8> {
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
        for command in &self.commands {
            out.command(command, &from_user, resources);
        }
        for _ in 0..self.depth {
            out.content.restore_state();
        }
        out.content.finish().into_vec()
    }
}

/// What the content stream has set, to skip operators that would change nothing. `None` is
/// "not set by us" (the PDF default may still hold, but nothing is assumed).
#[derive(Debug, Clone, Default)]
struct State {
    fill: Option<Color>,
    stroke: Option<Color>,
    line: Option<StrokeStyle>,
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
            c.set_miter_limit(n(style.miter_limit.max(1.0)));
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
        let at = |p: &lamina_model::PointPt| {
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
                    let a = to_user.apply(r.x0.get(), r.y0.get());
                    let b = to_user.apply(r.x1.get(), r.y1.get());
                    let (x0, y0) = (a.0.min(b.0), a.1.min(b.1));
                    c.rect(n(x0), n(y0), n((a.0 - b.0).abs()), n((a.1 - b.1).abs()));
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
