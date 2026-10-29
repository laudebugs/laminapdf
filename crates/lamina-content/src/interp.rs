//! The read-only interpreter (ADR 0023): runs content streams against a graphics state and
//! reports what they draw to a [`Visitor`].

use std::collections::BTreeMap;
use std::sync::Arc;

use lamina_cos::{Dict, Document, Error, Limit, Limits, Name, ObjRef, Object, Result, Warning};

use crate::lexer::{ContentIter, IssueKind, Lexed, Op};
use crate::operator::Operator;
use crate::resources::{self, Found};
use crate::state::{Color, ColorSpace, Dash, FontRef, GraphicsState, Matrix};
use crate::visitor::{
    FillRule, FormXObject, ImageDraw, ImageSource, MarkedContent, Paint, PathPaint, PathSegment, ShadingPaint,
    TextShow, Visitor,
};

/// What an interpretation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Summary {
    /// Operators read, counting each Form XObject's every time it was drawn (the number
    /// [`Limits::max_content_operators`] bounds).
    pub operators: u64,
    /// Form XObjects drawn (each time).
    pub forms: u64,
}

/// Interprets a page's content (its `/Contents` streams, decoded and joined as one stream,
/// §7.8.2) with its resources, reporting to `visitor`.
///
/// Warnings name the page object and an offset in the joined content; those raised inside a
/// Form XObject name the form.
///
/// # Errors
/// Errors decoding the page's content ([`lamina_cos::Document::page_content`]);
/// [`Error::LimitExceeded`] past [`Limits::max_content_operators`],
/// [`Limits::max_xobject_depth`], [`Limits::max_nesting_depth`] (saved graphics states),
/// [`Limits::max_decoded_total`] (bytes interpreted, forms counted every time they are
/// drawn) or [`Limits::max_string_length`]; [`Error::Strict`] in strict mode for any warning;
/// [`Error::DataPending`] / [`Error::Source`] when the input can't be read.
///
/// # Examples
/// ```
/// use lamina_content::{TextShow, TextAdvance, Visitor, interpret_page};
/// use lamina_cos::{Dict, Object, Stream, Writer};
/// use lamina_model::{Document, OpenOptions};
///
/// // A one-page document showing "Hi".
/// let mut w = Writer::new(1, 7);
/// let pages = w.reserve();
/// let content = w.add(Stream::new(Dict::new(), b"BT /F1 12 Tf 72 700 Td (Hi) Tj ET".to_vec()));
/// let font: Dict = [("Type", Object::name("Font")), ("Subtype", Object::name("Type1")),
///     ("BaseFont", Object::name("Helvetica"))].into_iter().collect();
/// let fonts: Dict = [("F1", Object::Ref(w.add(font)))].into_iter().collect();
/// let resources: Dict = [("Font", Object::Dict(fonts))].into_iter().collect();
/// let page = w.add([("Type", Object::name("Page")), ("Parent", pages.into()),
///     ("Contents", content.into()), ("Resources", resources.into())].into_iter().collect::<Dict>());
/// w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into())]
///     .into_iter().collect::<Dict>());
/// let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// let doc = Document::open(w.finish(&[("Root", root)].into_iter().collect())?, OpenOptions::default())?;
///
/// struct Strings(Vec<Vec<u8>>);
/// impl Visitor for Strings {
///     fn show_text(&mut self, show: &TextShow<'_>) -> TextAdvance {
///         for item in show.items() {
///             if let lamina_content::TextItem::Text(bytes) = item {
///                 self.0.push(bytes.to_vec());
///             }
///         }
///         TextAdvance::default()
///     }
/// }
///
/// let pages = doc.pages()?;
/// let mut strings = Strings(Vec::new());
/// let summary = interpret_page(&doc, &pages.get(0).expect("one page"), &mut strings)?;
/// assert_eq!(strings.0, [b"Hi".to_vec()]);
/// assert_eq!(summary.operators, 5);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn interpret_page<V: Visitor + ?Sized>(
    doc: &Document,
    page: &lamina_model::Page<'_>,
    visitor: &mut V,
) -> Result<Summary> {
    let content = doc.page_content(page.cos())?;
    let resources = page.cos().resources.clone();
    let object = page.obj_ref().map(|r| r.num);
    Interpreter::new(doc, visitor).run_top(&content, resources, object)
}

/// Interprets decoded content bytes with `resources` (a resource dictionary), reporting to
/// `visitor`: for content that is not a page's, such as an appearance stream, or a test.
/// Warnings carry no object.
///
/// # Errors
/// As [`interpret_page`], without the page decoding errors.
pub fn interpret_content<V: Visitor + ?Sized>(
    doc: &Document,
    content: &[u8],
    resources: Option<&Dict>,
    visitor: &mut V,
) -> Result<Summary> {
    let resources = resources.map(|d| Arc::new(Object::Dict(d.clone())));
    Interpreter::new(doc, visitor).run_top(content, resources, None)
}

/// The state of one content stream: the page's, or one Form XObject's.
struct Frame {
    /// The object named in warnings.
    object: Option<u32>,
    /// The resource dictionary names resolve against.
    resources: Option<Arc<Object>>,
    gs: GraphicsState,
    /// Saved states (`q`).
    saved: Vec<GraphicsState>,
    in_text: bool,
    tm: Matrix,
    tlm: Matrix,
    /// Open marked-content sequences.
    marked: u64,
    /// `BX` nesting.
    compat: u64,
    /// `W`/`W*` waiting for the next painting operator.
    clip: Option<FillRule>,
}

impl Frame {
    fn new(object: Option<u32>, resources: Option<Arc<Object>>, gs: GraphicsState) -> Self {
        Self {
            object,
            resources,
            gs,
            saved: Vec::new(),
            in_text: false,
            tm: Matrix::IDENTITY,
            tlm: Matrix::IDENTITY,
            marked: 0,
            compat: 0,
            clip: None,
        }
    }
}

struct Interpreter<'d, 'v, V: ?Sized> {
    doc: &'d Document,
    visitor: &'v mut V,
    limits: Limits,
    summary: Summary,
    /// Bytes interpreted so far (forms each time they are drawn).
    bytes: u64,
    /// Forms being drawn, outermost first (cycle detection by object identity; `None` for a
    /// direct form, which can't be part of a cycle but counts as nesting).
    forms: Vec<Option<ObjRef>>,
    /// Decoded content of indirect forms, so a form drawn many times is decoded once.
    decoded: BTreeMap<ObjRef, Arc<[u8]>>,
}

fn offset_u64(offset: usize) -> u64 {
    u64::try_from(offset).unwrap_or(u64::MAX)
}

fn number(object: &Object) -> f64 {
    object.as_f64().unwrap_or_default()
}

/// The first `N` numbers of `args` (the signature check has made sure they are numbers).
fn numbers<const N: usize>(args: &[Object]) -> [f64; N] {
    let mut out = [0.0; N];
    for (slot, arg) in out.iter_mut().zip(args) {
        *slot = number(arg);
    }
    out
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a real where an integer is expected is truncated, as PDF consumers do (§7.3.3)"
)]
fn integer(object: &Object) -> i64 {
    object.as_int().unwrap_or_else(|| number(object) as i64)
}

fn name(object: &Object) -> Option<&Name> {
    match object {
        Object::Name(n) => Some(n),
        _ => None,
    }
}

fn dash(array: &[Object], phase: f64) -> Dash {
    Dash {
        array: array.iter().filter_map(Object::as_f64).collect(),
        phase,
    }
}

impl<'d, 'v, V: Visitor + ?Sized> Interpreter<'d, 'v, V> {
    fn new(doc: &'d Document, visitor: &'v mut V) -> Self {
        Self {
            doc,
            visitor,
            limits: *doc.limits(),
            summary: Summary::default(),
            bytes: 0,
            forms: Vec::new(),
            decoded: BTreeMap::new(),
        }
    }

    fn run_top(mut self, content: &[u8], resources: Option<Arc<Object>>, object: Option<u32>) -> Result<Summary> {
        let mut frame = Frame::new(object, resources, GraphicsState::default());
        self.run(&mut frame, content)?;
        Ok(self.summary)
    }

    /// Reports `warning` to the visitor and the document; fails in strict mode.
    fn warn(&mut self, warning: Warning) -> Result<()> {
        self.visitor.warning(&warning);
        self.doc.report(warning)
    }

    fn depth(&self) -> usize {
        self.forms.len()
    }

    /// Interprets one content stream in `frame`.
    fn run(&mut self, frame: &mut Frame, content: &[u8]) -> Result<()> {
        self.bytes = self.bytes.saturating_add(offset_u64(content.len()));
        if self.bytes > offset_u64(self.limits.max_decoded_total()) {
            return Err(Error::LimitExceeded {
                limit: Limit::DecodedTotal,
                at: None,
            });
        }
        for item in ContentIter::new(content).with_limits(&self.limits) {
            match item? {
                Lexed::Issue(issue) => {
                    let offset = offset_u64(issue.offset);
                    let object = frame.object;
                    self.warn(match issue.kind {
                        IssueKind::InlineImageUnterminated => Warning::InlineImageUnterminated { object, offset },
                        IssueKind::Syntax => Warning::ContentSyntax { object, offset },
                    })?;
                }
                Lexed::Op(op) => {
                    self.summary.operators += 1;
                    if self.summary.operators > self.limits.max_content_operators() {
                        return Err(Error::LimitExceeded {
                            limit: Limit::ContentOperators,
                            at: None,
                        });
                    }
                    self.visitor.operator(&op, self.depth());
                    self.execute(frame, &op)?;
                }
            }
        }
        self.finish(frame, offset_u64(content.len()))
    }

    /// Closes what a content stream left open (ADR 0023: each stream balances on its own).
    fn finish(&mut self, frame: &mut Frame, end: u64) -> Result<()> {
        let object = frame.object;
        if frame.in_text {
            frame.in_text = false;
            self.warn(Warning::ContentUnbalancedBt { object, offset: end })?;
        }
        if let Some(first) = frame.saved.drain(..).next() {
            frame.gs = first;
            self.warn(Warning::ContentUnbalancedQ { object, offset: end })?;
        }
        if frame.marked > 0 {
            for _ in 0..frame.marked {
                self.visitor.end_marked();
            }
            frame.marked = 0;
            self.warn(Warning::ContentUnbalancedMarked { object, offset: end })?;
        }
        Ok(())
    }

    fn bad_operands(&mut self, frame: &Frame, op: &Op<'_>) -> Result<()> {
        self.warn(Warning::ContentBadOperands {
            object: frame.object,
            offset: offset_u64(op.offset),
        })
    }

    fn missing(&mut self, frame: &Frame, op: &Op<'_>, category: &'static str) -> Result<()> {
        self.warn(Warning::ContentResourceMissing {
            object: frame.object,
            offset: offset_u64(op.offset),
            category,
        })
    }

    /// A text positioning or showing operator outside `BT … ET` runs anyway, with a warning.
    fn require_text(&mut self, frame: &Frame, op: &Op<'_>) -> Result<()> {
        if frame.in_text {
            return Ok(());
        }
        self.warn(Warning::ContentUnbalancedBt {
            object: frame.object,
            offset: offset_u64(op.offset),
        })
    }

    fn lookup(&self, frame: &Frame, category: &str, name: &Name) -> Result<Option<Found>> {
        resources::lookup(self.doc, frame.resources.as_deref(), category, name.as_bytes())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per Annex A operator reads best as a single table"
    )]
    fn execute(&mut self, frame: &mut Frame, op: &Op<'_>) -> Result<()> {
        if let Operator::Unknown(_) = op.operator {
            if frame.compat == 0 {
                self.warn(Warning::ContentUnknownOperator {
                    object: frame.object,
                    offset: offset_u64(op.offset),
                })?;
            }
            return Ok(());
        }
        let Some(checked) = op.operator.signature().check(&op.operands) else {
            return self.bad_operands(frame, op);
        };
        if checked.extra {
            self.bad_operands(frame, op)?;
        }
        let args = checked.operands;
        let gs = &mut frame.gs;
        match op.operator {
            // General graphics state (§8.4.4).
            Operator::LineWidth => gs.line_width = numbers::<1>(args)[0],
            Operator::LineCap => gs.line_cap = args.first().map_or(0, integer),
            Operator::LineJoin => gs.line_join = args.first().map_or(0, integer),
            Operator::MiterLimit => gs.miter_limit = numbers::<1>(args)[0],
            Operator::Dash => {
                if let [Object::Array(array), phase] = args {
                    gs.dash = dash(array, number(phase));
                }
            }
            Operator::RenderingIntent => gs.rendering_intent = args.first().and_then(name).cloned(),
            Operator::Flatness => gs.flatness = numbers::<1>(args)[0],
            Operator::ExtGState => {
                if let Some(n) = args.first().and_then(name) {
                    self.ext_gstate(frame, op, n)?;
                }
            }
            // Special graphics state.
            Operator::Save => {
                if frame.saved.len() >= self.limits.max_nesting_depth() {
                    return Err(Error::LimitExceeded {
                        limit: Limit::NestingDepth,
                        at: Some(offset_u64(op.offset)),
                    });
                }
                frame.saved.push(frame.gs.clone());
            }
            Operator::Restore => match frame.saved.pop() {
                Some(saved) => frame.gs = saved,
                None => self.warn(Warning::ContentUnbalancedQ {
                    object: frame.object,
                    offset: offset_u64(op.offset),
                })?,
            },
            Operator::Transform => {
                gs.ctm = matrix(args).multiply(&gs.ctm);
            }
            // Path construction (§8.5.2).
            Operator::MoveTo => {
                let [x, y] = numbers::<2>(args);
                self.visitor.path_segment(&PathSegment::MoveTo { x, y });
            }
            Operator::LineTo => {
                let [x, y] = numbers::<2>(args);
                self.visitor.path_segment(&PathSegment::LineTo { x, y });
            }
            Operator::CurveTo => {
                let [x1, y1, x2, y2, x3, y3] = numbers::<6>(args);
                self.visitor.path_segment(&PathSegment::CurveTo {
                    c1: Some((x1, y1)),
                    c2: Some((x2, y2)),
                    end: (x3, y3),
                });
            }
            Operator::CurveToV => {
                let [x2, y2, x3, y3] = numbers::<4>(args);
                self.visitor.path_segment(&PathSegment::CurveTo {
                    c1: None,
                    c2: Some((x2, y2)),
                    end: (x3, y3),
                });
            }
            Operator::CurveToY => {
                let [x1, y1, x3, y3] = numbers::<4>(args);
                self.visitor.path_segment(&PathSegment::CurveTo {
                    c1: Some((x1, y1)),
                    c2: None,
                    end: (x3, y3),
                });
            }
            Operator::ClosePath => self.visitor.path_segment(&PathSegment::ClosePath),
            Operator::Rectangle => {
                let [x, y, width, height] = numbers::<4>(args);
                self.visitor.path_segment(&PathSegment::Rect { x, y, width, height });
            }
            // Path painting (§8.5.3) and clipping (§8.5.4).
            Operator::Stroke => self.paint(frame, Paint::Stroke { close: false }),
            Operator::CloseStroke => self.paint(frame, Paint::Stroke { close: true }),
            Operator::Fill | Operator::FillObsolete => self.paint(frame, Paint::Fill(FillRule::NonZero)),
            Operator::FillEvenOdd => self.paint(frame, Paint::Fill(FillRule::EvenOdd)),
            Operator::FillStroke => self.paint(
                frame,
                Paint::FillStroke {
                    rule: FillRule::NonZero,
                    close: false,
                },
            ),
            Operator::FillStrokeEvenOdd => self.paint(
                frame,
                Paint::FillStroke {
                    rule: FillRule::EvenOdd,
                    close: false,
                },
            ),
            Operator::CloseFillStroke => self.paint(
                frame,
                Paint::FillStroke {
                    rule: FillRule::NonZero,
                    close: true,
                },
            ),
            Operator::CloseFillStrokeEvenOdd => self.paint(
                frame,
                Paint::FillStroke {
                    rule: FillRule::EvenOdd,
                    close: true,
                },
            ),
            Operator::EndPath => self.paint(frame, Paint::None),
            Operator::Clip => frame.clip = Some(FillRule::NonZero),
            Operator::ClipEvenOdd => frame.clip = Some(FillRule::EvenOdd),
            // Text objects (§9.4.1).
            Operator::BeginText => {
                if frame.in_text {
                    self.warn(Warning::ContentUnbalancedBt {
                        object: frame.object,
                        offset: offset_u64(op.offset),
                    })?;
                }
                frame.in_text = true;
                frame.tm = Matrix::IDENTITY;
                frame.tlm = Matrix::IDENTITY;
            }
            Operator::EndText => {
                if frame.in_text {
                    frame.in_text = false;
                } else {
                    self.warn(Warning::ContentUnbalancedBt {
                        object: frame.object,
                        offset: offset_u64(op.offset),
                    })?;
                }
            }
            // Text state (§9.3).
            Operator::CharSpacing => gs.text.char_spacing = numbers::<1>(args)[0],
            Operator::WordSpacing => gs.text.word_spacing = numbers::<1>(args)[0],
            Operator::HorizontalScaling => gs.text.horizontal_scaling = numbers::<1>(args)[0] / 100.0,
            Operator::Leading => gs.text.leading = numbers::<1>(args)[0],
            Operator::Font => {
                if let [font, size] = args
                    && let Some(n) = name(font)
                {
                    let size = number(size);
                    self.set_font(frame, op, n, size)?;
                }
            }
            Operator::RenderMode => gs.text.render_mode = args.first().map_or(0, integer),
            Operator::Rise => gs.text.rise = numbers::<1>(args)[0],
            // Text positioning (§9.4.2).
            Operator::MoveText => {
                self.require_text(frame, op)?;
                let [tx, ty] = numbers::<2>(args);
                move_text(frame, tx, ty);
            }
            Operator::MoveTextSetLeading => {
                self.require_text(frame, op)?;
                let [tx, ty] = numbers::<2>(args);
                frame.gs.text.leading = -ty;
                move_text(frame, tx, ty);
            }
            Operator::TextMatrix => {
                self.require_text(frame, op)?;
                frame.tm = matrix(args);
                frame.tlm = frame.tm;
            }
            Operator::NextLine => {
                self.require_text(frame, op)?;
                next_line(frame);
            }
            // Text showing (§9.4.3).
            Operator::ShowText | Operator::ShowTextAdjusted => {
                self.require_text(frame, op)?;
                if let Some(operand) = args.first() {
                    self.show(frame, operand);
                }
            }
            Operator::NextLineShowText => {
                self.require_text(frame, op)?;
                next_line(frame);
                if let Some(operand) = args.first() {
                    self.show(frame, operand);
                }
            }
            Operator::SpacingNextLineShowText => {
                self.require_text(frame, op)?;
                if let [word, char, string] = args {
                    frame.gs.text.word_spacing = number(word);
                    frame.gs.text.char_spacing = number(char);
                    next_line(frame);
                    self.show(frame, string);
                }
            }
            // Type 3 glyph procedures are not interpreted yet (ADR 0023): nothing to track.
            // Unknown operators returned above.
            Operator::GlyphWidth | Operator::GlyphWidthBBox | Operator::Unknown(_) => {}
            // Colour (§8.6.8).
            Operator::StrokeColorSpace | Operator::FillColorSpace => {
                if let Some(n) = args.first().and_then(name) {
                    let space = self.color_space(frame, op, n)?;
                    let color = Color::initial(space);
                    if op.operator == Operator::StrokeColorSpace {
                        frame.gs.stroke = color;
                    } else {
                        frame.gs.fill = color;
                    }
                }
            }
            Operator::StrokeColor | Operator::StrokeColorN | Operator::FillColor | Operator::FillColorN => {
                let (pattern, components) = match args.split_last() {
                    Some((Object::Name(n), rest)) => (Some(n), rest),
                    _ => (None, args),
                };
                if let Some(n) = pattern
                    && self.lookup(frame, "Pattern", n)?.is_none()
                {
                    self.missing(frame, op, "Pattern")?;
                }
                let stroke = matches!(op.operator, Operator::StrokeColor | Operator::StrokeColorN);
                let color = if stroke {
                    &mut frame.gs.stroke
                } else {
                    &mut frame.gs.fill
                };
                color.components = components.iter().map(number).collect();
                color.pattern = pattern.cloned();
            }
            Operator::StrokeGray => gs.stroke = device_color(ColorSpace::DeviceGray, args),
            Operator::FillGray => gs.fill = device_color(ColorSpace::DeviceGray, args),
            Operator::StrokeRgb => gs.stroke = device_color(ColorSpace::DeviceRgb, args),
            Operator::FillRgb => gs.fill = device_color(ColorSpace::DeviceRgb, args),
            Operator::StrokeCmyk => gs.stroke = device_color(ColorSpace::DeviceCmyk, args),
            Operator::FillCmyk => gs.fill = device_color(ColorSpace::DeviceCmyk, args),
            // Shading (§8.7.4.2).
            Operator::Shading => {
                if let Some(n) = args.first().and_then(name) {
                    match self.lookup(frame, "Shading", n)? {
                        Some(found) if found.value.as_dict().is_some() => {
                            self.visitor.paint_shading(&ShadingPaint {
                                name: n,
                                object: found.object,
                                shading: &found.value,
                                state: &frame.gs,
                            });
                        }
                        _ => self.missing(frame, op, "Shading")?,
                    }
                }
            }
            // Inline images (§8.9.7).
            Operator::BeginInlineImage => {
                if let Some(image) = &op.inline_image {
                    self.visitor.image(&ImageDraw {
                        source: ImageSource::Inline(image),
                        state: &frame.gs,
                    });
                }
            }
            Operator::InlineImageData | Operator::EndInlineImage => {
                self.warn(Warning::ContentSyntax {
                    object: frame.object,
                    offset: offset_u64(op.offset),
                })?;
            }
            // XObjects (§8.8).
            Operator::PaintXObject => {
                if let Some(n) = args.first().and_then(name) {
                    self.xobject(frame, op, n)?;
                }
            }
            // Marked content (§14.6).
            Operator::MarkPoint
            | Operator::MarkPointProperties
            | Operator::BeginMarked
            | Operator::BeginMarkedProperties => {
                self.marked(frame, op, args)?;
            }
            Operator::EndMarked => {
                if frame.marked == 0 {
                    self.warn(Warning::ContentUnbalancedMarked {
                        object: frame.object,
                        offset: offset_u64(op.offset),
                    })?;
                } else {
                    frame.marked -= 1;
                    self.visitor.end_marked();
                }
            }
            // Compatibility sections (§7.8.2).
            Operator::BeginCompatibility => frame.compat += 1,
            Operator::EndCompatibility => frame.compat = frame.compat.saturating_sub(1),
        }
        Ok(())
    }

    fn paint(&mut self, frame: &mut Frame, paint: Paint) {
        let clip = frame.clip.take();
        self.visitor.paint_path(&PathPaint {
            paint,
            clip,
            state: &frame.gs,
        });
    }

    fn show(&mut self, frame: &mut Frame, operand: &Object) {
        let advance = self.visitor.show_text(&TextShow {
            operand,
            state: &frame.gs,
            tm: frame.tm,
            tlm: frame.tlm,
            depth: self.forms.len(),
        });
        frame.tm = Matrix::translate(advance.tx, advance.ty).multiply(&frame.tm);
    }

    fn set_font(&mut self, frame: &mut Frame, op: &Op<'_>, n: &Name, size: f64) -> Result<()> {
        let found = self.lookup(frame, "Font", n)?;
        let (object, dict) = match found {
            Some(found) if found.value.as_dict().is_some() => (found.object, Some(found.value)),
            _ => {
                self.missing(frame, op, "Font")?;
                (None, None)
            }
        };
        frame.gs.text.font = Some(FontRef {
            name: Some(n.clone()),
            object,
            dict,
        });
        frame.gs.text.font_size = size;
        Ok(())
    }

    fn color_space(&mut self, frame: &Frame, op: &Op<'_>, n: &Name) -> Result<ColorSpace> {
        Ok(match n.as_bytes() {
            b"DeviceGray" => ColorSpace::DeviceGray,
            b"DeviceRGB" => ColorSpace::DeviceRgb,
            b"DeviceCMYK" => ColorSpace::DeviceCmyk,
            b"Pattern" => ColorSpace::Pattern,
            _ => {
                if self.lookup(frame, "ColorSpace", n)?.is_none() {
                    self.missing(frame, op, "ColorSpace")?;
                }
                ColorSpace::Named(n.clone())
            }
        })
    }

    /// `gs`: the `ExtGState` subset of ADR 0023 (line style, rendering intent, flatness,
    /// opacity, font).
    fn ext_gstate(&mut self, frame: &mut Frame, op: &Op<'_>, n: &Name) -> Result<()> {
        let found = self.lookup(frame, "ExtGState", n)?;
        let Some(dict) = found.as_ref().and_then(|f| f.value.as_dict()) else {
            return self.missing(frame, op, "ExtGState");
        };
        let gs = &mut frame.gs;
        for (key, value) in dict.iter() {
            match (key.as_bytes(), value) {
                (b"LW", v) if v.as_f64().is_some() => gs.line_width = number(v),
                (b"LC", v) if v.as_f64().is_some() => gs.line_cap = integer(v),
                (b"LJ", v) if v.as_f64().is_some() => gs.line_join = integer(v),
                (b"ML", v) if v.as_f64().is_some() => gs.miter_limit = number(v),
                (b"FL", v) if v.as_f64().is_some() => gs.flatness = number(v),
                (b"CA", v) if v.as_f64().is_some() => gs.stroke_alpha = number(v),
                (b"ca", v) if v.as_f64().is_some() => gs.fill_alpha = number(v),
                (b"RI", Object::Name(intent)) => gs.rendering_intent = Some(intent.clone()),
                (b"D", Object::Array(d)) => {
                    if let [Object::Array(array), phase] = d.as_slice() {
                        gs.dash = dash(array, number(phase));
                    }
                }
                (b"Font", Object::Array(font)) => {
                    if let [font, size] = font.as_slice() {
                        let dict = resources::resolve(self.doc, font)?.filter(|f| f.as_dict().is_some());
                        gs.text.font = Some(FontRef {
                            name: None,
                            object: font.as_ref(),
                            dict,
                        });
                        gs.text.font_size = number(size);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn marked(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        let Some(tag) = args.first().and_then(name) else {
            return Ok(());
        };
        let found;
        let (properties, properties_name) = match args.get(1) {
            Some(Object::Dict(d)) => (Some(d), None),
            Some(Object::Name(n)) => {
                found = self.lookup(frame, "Properties", n)?;
                let dict = found.as_ref().and_then(|f| f.value.as_dict());
                if dict.is_none() {
                    self.missing(frame, op, "Properties")?;
                }
                (dict, Some(n))
            }
            _ => (None, None),
        };
        let marked = MarkedContent {
            tag,
            properties,
            properties_name,
            mcid: properties.and_then(|p| p.get_int("MCID")),
            depth: self.forms.len(),
        };
        match op.operator {
            Operator::BeginMarked | Operator::BeginMarkedProperties => {
                frame.marked += 1;
                self.visitor.begin_marked(&marked);
            }
            _ => self.visitor.marked_point(&marked),
        }
        Ok(())
    }

    /// `Do` (§8.8): an image is reported, a form is run, a PostScript XObject is ignored.
    fn xobject(&mut self, frame: &mut Frame, op: &Op<'_>, n: &Name) -> Result<()> {
        let found = self.lookup(frame, "XObject", n)?;
        let Some(found) = found.filter(|f| matches!(*f.value, Object::Stream(_))) else {
            return self.missing(frame, op, "XObject");
        };
        let Some(dict) = found.value.as_dict() else {
            return Ok(());
        };
        match dict.get_name("Subtype") {
            Some(b"Image") => {
                self.visitor.image(&ImageDraw {
                    source: ImageSource::XObject {
                        name: n,
                        object: found.object,
                        stream: &found.value,
                    },
                    state: &frame.gs,
                });
                Ok(())
            }
            Some(b"Form") => self.form(frame, op, n, &found),
            // PostScript XObjects (§8.8.2) are ignored when rendering, and so here.
            Some(b"PS") => Ok(()),
            _ => self.missing(frame, op, "XObject"),
        }
    }

    /// Runs a Form XObject (§8.10.1): `q`, its `/Matrix` concatenated to the CTM, its content
    /// against its own resources (the parent's when it has none), `Q`.
    fn form(&mut self, frame: &Frame, op: &Op<'_>, n: &Name, found: &Found) -> Result<()> {
        let Object::Stream(stream) = &*found.value else {
            return Ok(());
        };
        if let Some(r) = found.object
            && self.forms.contains(&Some(r))
        {
            return self.warn(Warning::XObjectCycle { object: r.num });
        }
        if self.forms.len() >= self.limits.max_xobject_depth() {
            return Err(Error::LimitExceeded {
                limit: Limit::XObjectDepth,
                at: found.object.map(|r| u64::from(r.num)),
            });
        }
        let content = match found.object.and_then(|r| self.decoded.get(&r)) {
            Some(content) => Arc::clone(content),
            None => match resources::tolerant(self.doc.decode_stream(stream))? {
                Some(content) => {
                    let content: Arc<[u8]> = content.into();
                    if let Some(r) = found.object {
                        self.decoded.insert(r, Arc::clone(&content));
                    }
                    content
                }
                None => return self.missing(frame, op, "XObject"),
            },
        };
        let dict = &stream.dict;
        let matrix = dict.get("Matrix").and_then(Matrix::from_object).unwrap_or_default();
        let bbox = dict.get("BBox").and_then(|b| {
            let values: Option<Vec<f64>> = b.as_array()?.iter().map(Object::as_f64).collect();
            <[f64; 4]>::try_from(values?).ok()
        });
        let own = match dict.get("Resources") {
            Some(r) => resources::resolve(self.doc, r)?.filter(|r| r.as_dict().is_some()),
            None => None,
        };
        let resources = own.or_else(|| frame.resources.clone());
        let mut gs = frame.gs.clone();
        gs.ctm = matrix.multiply(&gs.ctm);
        let entry = gs.clone();
        let object = found.object.map(|r| r.num);
        let enter = FormXObject {
            name: n,
            object: found.object,
            dict,
            matrix,
            bbox,
            depth: self.forms.len() + 1,
            state: &entry,
        };
        self.visitor.enter_xobject(&enter);
        let mut child = Frame::new(object.or(frame.object), resources, gs);
        self.forms.push(found.object);
        self.summary.forms += 1;
        let result = self.run(&mut child, &content);
        self.forms.pop();
        self.visitor.leave_xobject(&enter);
        result
    }
}

/// The matrix of `cm` or `Tm` (six numbers, checked by the signature).
fn matrix(args: &[Object]) -> Matrix {
    Matrix::from_slice(&numbers::<6>(args)).unwrap_or_default()
}

fn move_text(frame: &mut Frame, tx: f64, ty: f64) {
    frame.tlm = Matrix::translate(tx, ty).multiply(&frame.tlm);
    frame.tm = frame.tlm;
}

fn next_line(frame: &mut Frame) {
    let leading = frame.gs.text.leading;
    move_text(frame, 0.0, -leading);
}

fn device_color(space: ColorSpace, args: &[Object]) -> Color {
    let mut color = Color::initial(space);
    color.components = args.iter().map(number).collect();
    color
}
