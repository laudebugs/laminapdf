//! Writing a page's recorded commands as a content stream: the graphics state, resources by
//! first use, tagged content and links.

use laminapdf_model::PointPt;
#[cfg(feature = "text")]
use laminapdf_text::{TjElement, pdf_run};
#[cfg(feature = "text")]
use pdf_writer::types::TextRenderingMode;
use pdf_writer::types::{ColorSpaceOperand, LineCapStyle, LineJoinStyle};
use pdf_writer::{Content, Name};
#[cfg(feature = "text")]
use pdf_writer::{Str, TextStr};

#[cfg(feature = "text")]
use super::TextTarget;
use super::color::Color;
use super::path::{FillRule, Matrix, Segment};
use super::style::{LineCap, LineJoin, StrokeStyle};
#[cfg(feature = "text")]
use super::text::{TextCommand, TextRenderMode};
use super::{Command, Paint, n};
#[cfg(feature = "text")]
use crate::error::Result;
use crate::resources::PageResources;
use crate::tag::ContentKind;
use crate::tag::mcid::Marking;

/// What the content stream has set, to skip operators that would change nothing. `None` is
/// "not set by us" (the PDF default may still hold, but nothing is assumed).
#[derive(Debug, Clone, Default)]
pub(super) struct State {
    pub(super) fill: Option<Color>,
    pub(super) stroke: Option<Color>,
    pub(super) line: Option<StrokeStyle>,
    /// `Tr` (0, fill, at the start of a page: text state is part of the graphics state).
    #[cfg(feature = "text")]
    pub(super) mode: TextRenderMode,
}

pub(super) struct Writer {
    pub(super) content: Content,
    pub(super) to_user: Matrix,
    pub(super) unit: f64,
    pub(super) state: State,
    pub(super) saved: Vec<State>,
    pub(super) marking: Marking,
}

pub(super) fn matrix(m: &Matrix) -> [f32; 6] {
    m.to_array().map(n)
}

impl Writer {
    /// A link annotation drawn now: its `OBJR` goes in the innermost `Link` element, one made
    /// for it when the innermost element is another kind.
    pub(super) fn link(&mut self) {
        let in_link = self.marking.begin_link(&mut self.content);
        self.marking.link();
        if in_link {
            self.marking.end(&mut self.content);
        }
    }

    pub(super) fn command(&mut self, command: &Command, from_user: &Matrix, resources: &mut PageResources) {
        // Every arm that draws marks first (a clip draws nothing), and `q`/`Q` close the open
        // sequence, so a new command can't be drawn outside the marking.
        match command {
            Command::Save => {
                self.marking.close(&mut self.content);
                self.saved.push(self.state.clone());
                self.content.save_state();
            }
            Command::Restore => {
                self.marking.close(&mut self.content);
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
            Command::Paint(segments, paint) => {
                if !matches!(paint, Paint::Clip(_)) {
                    self.marking.mark(&mut self.content, ContentKind::Path);
                }
                self.paint(segments, paint);
            }
            // Written by `render` itself (they need the fonts or produce annotations).
            #[cfg(feature = "text")]
            Command::Text(_) => {}
            Command::Link(..) | Command::BeginTag(_) | Command::BeginArtifact(_) | Command::EndMarked => {}
            Command::Image(image, r) => {
                self.marking.mark(&mut self.content, ContentKind::Image);
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
    #[cfg(feature = "text")]
    pub(super) fn text(
        &mut self,
        t: &TextCommand,
        resources: &mut PageResources,
        target: &mut TextTarget<'_>,
    ) -> Result<()> {
        if t.lines.iter().all(|l| l.content.is_empty()) {
            return Ok(());
        }
        self.marking.mark(&mut self.content, ContentKind::Text);
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
    #[cfg(feature = "text")]
    pub(super) fn text_matrix(&mut self, x: f64, baseline: f64) {
        let place = Matrix::new([1.0, 0.0, 0.0, -1.0, x, baseline]);
        self.content.set_text_matrix(matrix(&self.to_user.after(&place)));
    }

    pub(super) fn color(&mut self, color: &Color, fill: bool, resources: &mut PageResources) {
        let current = if fill {
            &mut self.state.fill
        } else {
            &mut self.state.stroke
        };
        if current.as_ref() == Some(color) {
            return;
        }
        *current = Some(color.clone());
        if matches!(color, Color::Cmyk(..)) {
            resources.note_device_cmyk();
        }
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

    pub(super) fn line(&mut self, style: &StrokeStyle) {
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

    pub(super) fn paint(&mut self, segments: &[Segment], paint: &Paint) {
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
