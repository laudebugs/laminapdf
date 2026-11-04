//! The read-only interpreter (ADR 0023): runs content streams against a graphics state and
//! reports what they draw to a [`Visitor`].

mod color;
mod graphics_state;
mod path;
mod text;

use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf_cos::{Document, Error, Limit, Limits, Name, ObjRef, Object, Position, Result, Stream, Warning};

use crate::lexer::{ContentIter, IssueKind, Lexed, Op};
use crate::operator::Operator;
use crate::resources::{self, Found};
use crate::state::{ColorSpace, GraphicsState, Matrix};
use crate::visitor::{FillRule, FormXObject, ImageDraw, ImageSource, MarkedContent, Paint, PathSegment, Visitor};
use color::{Target, device_color};
use graphics_state::set_dash;
use path::{close_fill_stroke, curve_to, curve_to_y, fill_stroke, line_to, move_to, rectangle};

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
/// Errors decoding the page's content ([`laminapdf_cos::Document::page_content`]);
/// [`Error::LimitExceeded`] past [`Limits::max_content_operators`],
/// [`Limits::max_xobject_depth`], [`Limits::max_nesting_depth`] (saved graphics states),
/// [`Limits::max_decoded_total`] (bytes interpreted, forms counted every time they are
/// drawn) or [`Limits::max_string_length`]; [`Error::Strict`] in strict mode for any warning;
/// [`Error::DataPending`] / [`Error::Source`] when the input can't be read.
///
/// # Examples
/// ```
/// use laminapdf_content::{TextShow, TextAdvance, Visitor, interpret_page};
/// use laminapdf_cos::{Dict, Object, Stream};
/// use laminapdf_save::Writer;
/// use laminapdf_model::{Document, OpenOptions};
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
///             if let laminapdf_content::TextItem::Text(bytes) = item {
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
    page: &laminapdf_model::Page<'_>,
    visitor: &mut V,
) -> Result<Summary> {
    let content = doc.page_content(page.cos())?;
    let resources = page.cos().resources.clone();
    let object = page.obj_ref().map(|r| r.num);
    Interpreter::new(doc, visitor).run_top(&content, resources, object)
}

/// Interprets decoded content bytes with `resources` (a resource dictionary, shared rather
/// than copied), reporting to `visitor`: for content that is not a page's, such as an
/// appearance stream, or a test. Warnings about this content carry no object.
///
/// # Errors
/// As [`interpret_page`], without the page decoding errors.
pub fn interpret_content<V: Visitor + ?Sized>(
    doc: &Document,
    content: &[u8],
    resources: Option<Arc<Object>>,
    visitor: &mut V,
) -> Result<Summary> {
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
    /// Offset of the first segment of a path not painted yet.
    path: Option<usize>,
    /// The current point and the start of the current subpath (§8.5.2.1).
    current: Option<(f64, f64)>,
    subpath_start: Option<(f64, f64)>,
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
            path: None,
            current: None,
            subpath_start: None,
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
    /// Forms being drawn, outermost first (cycle detection by object identity).
    forms: Vec<ObjRef>,
    /// Decoded content of indirect forms, so a form drawn many times is decoded once.
    decoded: BTreeMap<ObjRef, Arc<[u8]>>,
}

fn offset_u64(offset: usize) -> u64 {
    u64::try_from(offset).unwrap_or(u64::MAX)
}

/// Where `offset` in the decoded content of `object` is (ADR 0026 §1.1); `None` for content
/// with no object (interpreted on its own, or a direct page), where no position is unambiguous.
fn in_content(object: Option<u32>, offset: u64) -> Option<Position> {
    object.map(|object| Position::InStream { object, offset })
}

/// `e` from lexing the content of `object`, its buffer offsets re-tagged as offsets in that
/// content (the syntax layer reports them as file offsets).
fn content_error(e: Error, object: Option<u32>) -> Error {
    match e {
        Error::LimitExceeded {
            limit,
            at: Some(Position::FileOffset(offset)),
        } => Error::LimitExceeded {
            limit,
            at: in_content(object, offset),
        },
        other => other,
    }
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

/// The one number of an operator that sets a single value.
fn first(args: &[Object]) -> f64 {
    args.first().map_or(0.0, number)
}

/// The one integer of an operator that sets a single value.
fn first_integer(args: &[Object]) -> i64 {
    args.first().map_or(0, integer)
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
        self.run(Frame::new(object, resources, GraphicsState::default()), content)?;
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

    /// Interprets one content stream in `frame`. Whatever the outcome, every marked-content
    /// sequence it began is ended for the visitor.
    fn run(&mut self, mut frame: Frame, content: &[u8]) -> Result<()> {
        match self.run_ops(&mut frame, content) {
            Ok(()) => self.finish(frame, offset_u64(content.len())),
            Err(e) => {
                for _ in 0..frame.marked {
                    self.visitor.end_marked();
                }
                Err(e)
            }
        }
    }

    fn run_ops(&mut self, frame: &mut Frame, content: &[u8]) -> Result<()> {
        self.bytes = self.bytes.saturating_add(offset_u64(content.len()));
        if self.bytes > offset_u64(self.limits.max_decoded_total()) {
            return Err(Error::LimitExceeded {
                limit: Limit::DecodedTotal,
                at: None,
            });
        }
        for item in ContentIter::new(content).with_limits(&self.limits) {
            match item.map_err(|e| content_error(e, frame.object))? {
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
        Ok(())
    }

    /// Closes what a content stream left open (ADR 0023: each stream balances on its own).
    /// Saved states need no restoring: the frame ends here.
    fn finish(&mut self, mut frame: Frame, end: u64) -> Result<()> {
        let object = frame.object;
        if let Some(offset) = frame.path {
            self.discard_path(&mut frame, offset)?;
        }
        if frame.in_text {
            self.warn(Warning::ContentUnbalancedBt { object, offset: end })?;
        }
        if !frame.saved.is_empty() {
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
        match op.operator {
            // General graphics state (§8.4.4).
            Operator::LineWidth => frame.gs.line_width = first(args),
            Operator::LineCap => frame.gs.line_cap = first_integer(args),
            Operator::LineJoin => frame.gs.line_join = first_integer(args),
            Operator::MiterLimit => frame.gs.miter_limit = first(args),
            Operator::Dash => set_dash(&mut frame.gs, args),
            Operator::RenderingIntent => frame.gs.rendering_intent = args.first().and_then(name).cloned(),
            Operator::Flatness => frame.gs.flatness = first(args),
            Operator::ExtGState => self.ext_gstate(frame, op, args)?,
            // Special graphics state.
            Operator::Save => self.save(frame, op)?,
            Operator::Restore => self.restore(frame, op)?,
            Operator::Transform => frame.gs.ctm = matrix(args).multiply(&frame.gs.ctm),
            // Path construction (§8.5.2).
            Operator::MoveTo => self.segment(frame, op, move_to(args)),
            Operator::LineTo => self.segment(frame, op, line_to(args)),
            Operator::CurveTo => self.segment(frame, op, curve_to(args)),
            Operator::CurveToV => self.curve_to_v(frame, op, args),
            Operator::CurveToY => self.segment(frame, op, curve_to_y(args)),
            Operator::ClosePath => self.segment(frame, op, PathSegment::ClosePath),
            Operator::Rectangle => self.segment(frame, op, rectangle(args)),
            // Path painting (§8.5.3) and clipping (§8.5.4).
            Operator::Stroke => self.paint(frame, Paint::Stroke { close: false }),
            Operator::CloseStroke => self.paint(frame, Paint::Stroke { close: true }),
            Operator::Fill | Operator::FillObsolete => self.paint(frame, Paint::Fill(FillRule::NonZero)),
            Operator::FillEvenOdd => self.paint(frame, Paint::Fill(FillRule::EvenOdd)),
            Operator::FillStroke => self.paint(frame, fill_stroke(FillRule::NonZero)),
            Operator::FillStrokeEvenOdd => self.paint(frame, fill_stroke(FillRule::EvenOdd)),
            Operator::CloseFillStroke => self.paint(frame, close_fill_stroke(FillRule::NonZero)),
            Operator::CloseFillStrokeEvenOdd => self.paint(frame, close_fill_stroke(FillRule::EvenOdd)),
            Operator::EndPath => self.paint(frame, Paint::None),
            Operator::Clip => frame.clip = Some(FillRule::NonZero),
            Operator::ClipEvenOdd => frame.clip = Some(FillRule::EvenOdd),
            // Text objects (§9.4.1).
            Operator::BeginText => self.begin_text(frame, op)?,
            Operator::EndText => self.end_text(frame, op)?,
            // Text state (§9.3).
            Operator::CharSpacing => frame.gs.text.char_spacing = first(args),
            Operator::WordSpacing => frame.gs.text.word_spacing = first(args),
            Operator::HorizontalScaling => frame.gs.text.horizontal_scaling = first(args) / 100.0,
            Operator::Leading => frame.gs.text.leading = first(args),
            Operator::Font => self.set_font(frame, op, args)?,
            Operator::RenderMode => frame.gs.text.render_mode = first_integer(args),
            Operator::Rise => frame.gs.text.rise = first(args),
            // Text positioning (§9.4.2).
            Operator::MoveText => self.move_text(frame, op, args)?,
            Operator::MoveTextSetLeading => self.move_text_set_leading(frame, op, args)?,
            Operator::TextMatrix => self.text_matrix(frame, op, args)?,
            Operator::NextLine => self.next_line(frame, op)?,
            // Text showing (§9.4.3).
            Operator::ShowText | Operator::ShowTextAdjusted => self.show_text(frame, op, args)?,
            Operator::NextLineShowText => self.next_line_show_text(frame, op, args)?,
            Operator::SpacingNextLineShowText => self.spacing_next_line_show_text(frame, op, args)?,
            // Type 3 glyph procedures are not interpreted yet (ADR 0023): nothing to track.
            // Unknown operators returned above.
            Operator::GlyphWidth | Operator::GlyphWidthBBox | Operator::Unknown(_) => {}
            // Colour (§8.6.8).
            Operator::StrokeColorSpace => self.set_color_space(frame, op, args, Target::Stroke)?,
            Operator::FillColorSpace => self.set_color_space(frame, op, args, Target::Fill)?,
            Operator::StrokeColor | Operator::StrokeColorN => self.set_color(frame, op, args, Target::Stroke)?,
            Operator::FillColor | Operator::FillColorN => self.set_color(frame, op, args, Target::Fill)?,
            Operator::StrokeGray => frame.gs.stroke = device_color(ColorSpace::DeviceGray, args),
            Operator::FillGray => frame.gs.fill = device_color(ColorSpace::DeviceGray, args),
            Operator::StrokeRgb => frame.gs.stroke = device_color(ColorSpace::DeviceRgb, args),
            Operator::FillRgb => frame.gs.fill = device_color(ColorSpace::DeviceRgb, args),
            Operator::StrokeCmyk => frame.gs.stroke = device_color(ColorSpace::DeviceCmyk, args),
            Operator::FillCmyk => frame.gs.fill = device_color(ColorSpace::DeviceCmyk, args),
            // Shading (§8.7.4.2).
            Operator::Shading => self.shading(frame, op, args)?,
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

    /// `Do` (§8.8): an image is reported, a form is run, a PostScript XObject is ignored. An
    /// XObject must be an indirect stream (§7.3.8); anything else is a missing resource.
    fn xobject(&mut self, frame: &mut Frame, op: &Op<'_>, n: &Name) -> Result<()> {
        let found = self.lookup(frame, "XObject", n)?;
        let Some((object, found)) = found.and_then(|f| Some((f.object?, f))) else {
            return self.missing(frame, op, "XObject");
        };
        let Object::Stream(stream) = &*found.value else {
            return self.missing(frame, op, "XObject");
        };
        match stream.dict.get_name("Subtype") {
            Some(b"Image") => {
                self.visitor.image(&ImageDraw {
                    source: ImageSource::XObject {
                        name: n,
                        object: Some(object),
                        stream: &found.value,
                    },
                    state: &frame.gs,
                });
                Ok(())
            }
            Some(b"Form") => self.form(frame, op, n, object, stream),
            // PostScript XObjects (§8.8.2) are ignored when rendering, and so here.
            Some(b"PS") => Ok(()),
            _ => self.missing(frame, op, "XObject"),
        }
    }

    /// Runs a Form XObject (§8.10.1): `q`, its `/Matrix` concatenated to the CTM, its content
    /// against its own resources (the parent's when it has none), `Q`.
    fn form(&mut self, frame: &Frame, op: &Op<'_>, n: &Name, object: ObjRef, stream: &Stream) -> Result<()> {
        if self.forms.contains(&object) {
            return self.warn(Warning::XObjectCycle { object: object.num });
        }
        if self.forms.len() >= self.limits.max_xobject_depth() {
            return Err(Error::LimitExceeded {
                limit: Limit::XObjectDepth,
                at: Some(Position::Object(object.num)),
            });
        }
        let content = if let Some(content) = self.decoded.get(&object) {
            Arc::clone(content)
        } else {
            let Some(content) = laminapdf_cos::tolerant(self.doc.decode_stream(stream))? else {
                return self.missing(frame, op, "XObject");
            };
            let content: Arc<[u8]> = content.into();
            self.decoded.insert(object, Arc::clone(&content));
            content
        };
        let dict = &stream.dict;
        let invalid = |key| Warning::InvalidEntry {
            object: Some(object.num),
            key,
        };
        let matrix = match dict.get("Matrix") {
            None => Matrix::IDENTITY,
            Some(m) => {
                if let Some(values) = resources::numbers(self.doc, m, 6)? {
                    Matrix::from_slice(&values).unwrap_or_default()
                } else {
                    self.warn(invalid("Matrix"))?;
                    Matrix::IDENTITY
                }
            }
        };
        let bbox = match dict.get("BBox") {
            Some(b) => resources::numbers(self.doc, b, 4)?,
            None => None,
        }
        .and_then(|b| match *b.as_slice() {
            [x0, y0, x1, y1] => Some(laminapdf_model::UserSpaceRect::from_corners(x0, y0, x1, y1)),
            _ => None,
        });
        if bbox.is_none() {
            // `/BBox` is required (§8.10.2, Table 93).
            self.warn(invalid("BBox"))?;
        }
        let own = match dict.get("Resources") {
            Some(r) => {
                let own = resources::resolve(self.doc, r)?.filter(|r| r.as_dict().is_some());
                if own.is_none() {
                    self.warn(invalid("Resources"))?;
                }
                own
            }
            None => None,
        };
        let resources = own.or_else(|| frame.resources.clone());
        let mut gs = frame.gs.clone();
        gs.ctm = matrix.multiply(&gs.ctm);
        let entry = gs.clone();
        let enter = FormXObject {
            name: n,
            object: Some(object),
            dict,
            matrix,
            bbox,
            depth: self.forms.len() + 1,
            state: &entry,
        };
        self.visitor.enter_xobject(&enter);
        self.forms.push(object);
        self.summary.forms += 1;
        let result = self.run(Frame::new(Some(object.num), resources, gs), &content);
        self.forms.pop();
        self.visitor.leave_xobject(&enter);
        result
    }
}

/// The matrix of `cm` or `Tm` (six numbers, checked by the signature).
fn matrix(args: &[Object]) -> Matrix {
    Matrix::from_slice(&numbers::<6>(args)).unwrap_or_default()
}
