//! The editable content IR ([ADR 0022]): a content owner's operators with the bytes they came
//! from, grouped into the objects an editor selects, each with its graphics state and box.
//!
//! [ADR 0022]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0022-content-stream-editing.md

mod edits;
mod encode;
mod marked;
mod parse;
mod record;
mod text;

use std::borrow::Cow;
use std::ops::Range;

use laminapdf_cos::{Document, Error, Limit, Limits, Page, Result};
use laminapdf_model::{LaminaPdfId, PageFrame, PageId, PointPt, RectPt};
use laminapdf_syntax::{Name, ObjRef, Object};

use crate::operator::Operator;
use crate::state::GraphicsState;

pub use edits::{Placement, ResourceName};
pub use encode::{EncodedContent, EncodedPart};
pub use record::Recorder;
pub use text::ShownGlyph;

/// How far outside an object's box a point still hits it, in points: thin lines and
/// hairlines have boxes too narrow to point at.
const HIT_SLOP: f64 = 2.0;

/// Whose content an IR holds: a page (its `/Contents` streams, read as one program, ISO
/// 32000-2 §7.8.2) or a Form XObject.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContentOwner {
    /// A page's content.
    Page(PageId),
    /// A Form XObject's content.
    Form(LaminaPdfId),
}

/// An object's identity within one [`ContentOps`]: allocated when the IR is built and kept
/// across edits to its owner, never reused.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentObjectId(u32);

impl ContentObjectId {
    /// The number behind the ID, unique within its IR.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// An object named so the name lasts beyond the IR (ADR 0022 §2): its owner, its ordinal among
/// the owner's objects, and a 64-bit hash of its bytes. An op log names objects this way; a
/// rebuilt IR finds the ordinal and checks the hash ([`ContentOps::find`]).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentObjectRef {
    /// Whose content the object is in.
    pub owner: ContentOwner,
    /// The object's 0-based position among its owner's objects, in content order.
    pub ordinal: u32,
    /// [`source_hash`] of the object's bytes, from its first operand through its last
    /// operator.
    pub hash: u64,
}

/// What kind of object a range of operators draws (ADR 0022 §1).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContentObjectKind {
    /// A text object, `BT` through `ET`.
    Text,
    /// A path: its construction through its painting operator, clipping included (or, for a
    /// path no operator paints, through its last segment).
    Path,
    /// An image: `Do` of an image XObject, or an inline image (`BI … EI`).
    Image,
    /// `Do` of a Form XObject.
    Form,
    /// `Do` whose XObject hasn't been resolved: the IR wasn't recorded, or the resource is
    /// missing or of another type.
    XObject,
    /// A shading painted with `sh`.
    Shading,
}

/// Why content can't be edited (ADR 0022 §5 and §7): the reason `content-not-editable` gives.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NotEditable {
    /// The text is in a Type 3 font, whose glyphs are content streams of their own.
    Type3Font,
    /// The text is written vertically (`Identity-V` and other vertical encodings).
    VerticalText,
    /// The font has neither a ToUnicode map nor a usable `cmap`, so its glyphs' text is unknown.
    NoUnicodeMap,
    /// The glyphs of one run are turned against each other.
    RotatedRun,
    /// A show operator writes its glyphs out of reading order.
    GlyphsOutOfOrder,
    /// The text stands in for other text (`/ActualText`, ISO 32000-2 §14.9.4): extraction, copy
    /// and search read that text, which an edit of the glyphs would leave behind.
    ActualText,
    /// The text is written right to left, whose glyphs a file draws in either order.
    RightToLeft,
    /// The object is inside Form XObjects nested deeper than the limit allows.
    TooDeep,
    /// The content failed to read, or the object's graphics state wasn't recorded.
    ContentUnreadable,
    /// The object clips what follows it (a `W` path, or text in a clipping render mode): moving,
    /// deleting or reordering it would change other content. Clipping is edited on its own.
    ClippingPath,
    /// Moving the object in the stacking order would change how it draws: the clip, the
    /// transparency or the font where it would go differ from where it is.
    StateNotPortable,
}

impl NotEditable {
    /// The reason as the error's data names it: `type3-font`, `vertical-text`,
    /// `no-unicode-map`, `rotated-run`, `glyphs-out-of-order`, `actual-text`, `right-to-left`, `too-deep`,
    /// `content-unreadable`, `clipping-path` or `state-not-portable`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Type3Font => "type3-font",
            Self::VerticalText => "vertical-text",
            Self::NoUnicodeMap => "no-unicode-map",
            Self::RotatedRun => "rotated-run",
            Self::GlyphsOutOfOrder => "glyphs-out-of-order",
            Self::ActualText => "actual-text",
            Self::RightToLeft => "right-to-left",
            Self::TooDeep => "too-deep",
            Self::ContentUnreadable => "content-unreadable",
            Self::ClippingPath => "clipping-path",
            Self::StateNotPortable => "state-not-portable",
        }
    }
}

/// One decoded content stream of the owner.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContentPart {
    /// The part's index in the page's `/Contents` array (0 for a single stream). A `null`
    /// element contributes no part, so this can skip numbers.
    pub contents_index: usize,
    /// The stream object, when the element is a reference.
    pub object: Option<ObjRef>,
    /// Where the part starts in [`ContentOps::source`]: the parts are joined with one `\n`,
    /// as [`Document::page_content`] joins them.
    pub start: usize,
    /// Where the part ends in [`ContentOps::source`] (exclusive).
    pub end: usize,
}

/// One operator: its operands and keyword, and where it came from.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentOp {
    keyword: Box<[u8]>,
    operands: Vec<Object>,
    /// `BI` through `EI` as written, for an inline image.
    inline_image: Option<Box<[u8]>>,
    /// The source bytes, from the end of the previous operator through this one's keyword
    /// (or `EI`); `None` for an operator an edit wrote.
    span: Option<Range<usize>>,
    /// Where its first token starts in the source (its span without the white space and
    /// comments before it); for a written operator, where it is written.
    token_start: usize,
}

impl ContentOp {
    /// An operator to write, as an edit adds it: written canonically by the encoder.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn new(operator: Operator<'_>, operands: Vec<Object>) -> Self {
        Self {
            keyword: operator.keyword().into(),
            operands,
            inline_image: None,
            span: None,
            token_start: 0,
        }
    }

    /// The operator.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn operator(&self) -> Operator<'_> {
        Operator::from_keyword(&self.keyword)
    }

    /// The operands read before it, unchecked (as [`Op::operands`](crate::Op::operands)).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn operands(&self) -> &[Object] {
        &self.operands
    }

    /// For an inline image, `BI` through `EI` as written.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn inline_image(&self) -> Option<&[u8]> {
        self.inline_image.as_deref()
    }

    /// The bytes it came from in [`ContentOps::source`]: from the end of the previous operator
    /// through its own keyword (or `EI`), so the white space and comments before it come with
    /// it. `None` for an operator an edit wrote.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn span(&self) -> Option<Range<usize>> {
        self.span.clone()
    }
}

/// A marked-content sequence (`BMC`/`BDC … EMC`, §14.6): a container around objects, not an
/// object.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MarkedSequence {
    /// The tag.
    pub tag: Name,
    /// `/MCID` from an inline property list (§14.7.5); a named property list isn't resolved.
    pub mcid: Option<i64>,
    /// The `BMC` or `BDC` operator's index in [`ContentOps::operators`].
    pub begin: usize,
    /// The `EMC` operator's index; `None` when the content leaves the sequence open.
    pub end: Option<usize>,
}

/// An object an editor selects: a range of operators with what it draws.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentObject {
    id: ContentObjectId,
    kind: ContentObjectKind,
    ops: Range<usize>,
    state: Option<Box<GraphicsState>>,
    /// The graphics state before its last operator: a painting or closing operator changes
    /// none of it, so it is the state after the object (for an object that doesn't clip).
    end_state: Option<Box<GraphicsState>>,
    bounds: Option<RectPt>,
    marked: Vec<usize>,
}

impl ContentObject {
    /// Its identity within the IR.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn id(&self) -> ContentObjectId {
        self.id
    }

    /// What it draws.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn kind(&self) -> ContentObjectKind {
        self.kind
    }

    /// Its operators: a range of [`ContentOps::operators`].
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn operators(&self) -> Range<usize> {
        self.ops.clone()
    }

    /// The graphics state in effect before its first operator: with it the object draws the
    /// same without the `q`, `cm` and colour operators before it. `None` until a [`Recorder`]
    /// has seen the content.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn state(&self) -> Option<&GraphicsState> {
        self.state.as_deref()
    }

    /// Its bounding box in page space ([ADR 0027]): what its paths, images, shadings, glyphs
    /// or form cover, before clipping. `None` until a [`Recorder`] has seen the content, after
    /// an edit rewrites the object, or when it draws nothing.
    ///
    /// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn bounds(&self) -> Option<RectPt> {
        self.bounds
    }

    /// The marked-content sequences around it, outermost first: indices into
    /// [`ContentOps::marked`].
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn marked(&self) -> &[usize] {
        &self.marked
    }
}

/// A content owner's operators grouped into objects (ADR 0022 §1), with an encoder that copies
/// every byte an edit didn't touch (§3).
///
/// Build it with [`ContentOps::read_page`] (or [`ContentOps::from_parts`]), then let a
/// [`Recorder`] see the content for each object's graphics state and box:
///
/// ```
/// use laminapdf_content::{ContentObjectKind, ContentOps, ContentOwner, ContentPart};
/// use laminapdf_cos::Limits;
///
/// let owner = ContentOwner::Page("p3.0".parse()?);
/// let content = b"q 1 0 0 RG 10 10 100 50 re S Q BT /F1 12 Tf (Hi) Tj ET".to_vec();
/// let ops = ContentOps::from_parts(owner, vec![(None, content)], &Limits::default())?;
/// let kinds: Vec<_> = ops.objects().iter().map(|o| o.kind()).collect();
/// assert_eq!(kinds, [ContentObjectKind::Path, ContentObjectKind::Text]);
/// // Nothing edited: every part stays as it was.
/// assert!(ops.encode().parts.iter().all(|part| part.bytes().is_none()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Added in 0.5.0.
#[derive(Debug, Clone)]
pub struct ContentOps {
    owner: ContentOwner,
    source: Vec<u8>,
    parts: Vec<ContentPart>,
    ops: Vec<ContentOp>,
    objects: Vec<ContentObject>,
    marked: Vec<MarkedSequence>,
    /// Source ranges edits dropped: written operators' tokens.
    removed: Vec<Range<usize>>,
    /// Per part: whether an edit touched it.
    dirty: Vec<bool>,
    /// Whether any edit has changed the operators: a [`Recorder`] then records nothing.
    edited: bool,
    /// Whether a reorder has moved an object, so the objects are no longer in ID order.
    reordered: bool,
    /// For each text-showing operator the interpreter ran, the object showing it, as a
    /// [`Recorder`] saw them; `None` for one inside a Form XObject.
    shows: Vec<Option<ContentObjectId>>,
}

impl ContentOps {
    /// Reads `page`'s `/Contents` streams into an IR, without its states and boxes (see
    /// [`Recorder`]).
    ///
    /// # Errors
    /// Decoding errors; [`Error::Type`] when `/Contents` isn't a stream or an array of
    /// streams; [`Error::LimitExceeded`] past [`Limits::max_decoded_total`] (all the page's
    /// streams together) or [`Limits::max_content_operators`].
    ///
    /// Added in 0.5.0.
    pub fn read_page(doc: &Document, page: &Page, owner: ContentOwner) -> Result<Self> {
        let limits = *doc.limits();
        let parts = parse::read_parts(doc, page, &limits)?;
        Self::build(owner, parts, &limits)
    }

    /// An IR over decoded content parts, each with its stream object when it has one, joined
    /// as one program.
    ///
    /// # Errors
    /// [`Error::LimitExceeded`] past [`Limits::max_decoded_total`] or
    /// [`Limits::max_content_operators`], or a hard limit of the tokenizer.
    ///
    /// Added in 0.5.0.
    pub fn from_parts(owner: ContentOwner, parts: Vec<(Option<ObjRef>, Vec<u8>)>, limits: &Limits) -> Result<Self> {
        let parts = parts
            .into_iter()
            .enumerate()
            .map(|(index, (object, bytes))| (index, object, bytes))
            .collect();
        Self::build(owner, parts, limits)
    }

    fn build(owner: ContentOwner, parts: Vec<parse::RawPart>, limits: &Limits) -> Result<Self> {
        let total = parts
            .iter()
            .map(|(_, _, bytes)| bytes.len())
            .fold(0usize, usize::saturating_add);
        if total > limits.max_decoded_total() {
            return Err(Error::LimitExceeded {
                limit: Limit::DecodedTotal,
                at: None,
            });
        }
        let mut source = Vec::with_capacity(total.saturating_add(parts.len()));
        let mut spans = Vec::with_capacity(parts.len());
        for (contents_index, object, bytes) in parts {
            if !spans.is_empty() {
                source.push(b'\n');
            }
            let start = source.len();
            source.extend_from_slice(&bytes);
            spans.push(ContentPart {
                contents_index,
                object,
                start,
                end: source.len(),
            });
        }
        let parsed = parse::parse(&source, limits)?;
        let dirty = vec![false; spans.len()];
        Ok(Self {
            owner,
            source,
            parts: spans,
            ops: parsed.ops,
            objects: parsed.objects,
            marked: parsed.marked,
            removed: Vec::new(),
            dirty,
            edited: false,
            reordered: false,
            shows: Vec::new(),
        })
    }

    /// Whose content this is.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn owner(&self) -> &ContentOwner {
        &self.owner
    }

    /// The content as read: every part joined with `\n`, the bytes the interpreter reads.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    /// The content streams the source joins.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn parts(&self) -> &[ContentPart] {
        &self.parts
    }

    /// Every operator, in content order.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn operators(&self) -> &[ContentOp] {
        &self.ops
    }

    /// Every object, in content order (the order they paint).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn objects(&self) -> &[ContentObject] {
        &self.objects
    }

    /// Every marked-content sequence, by where it begins.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn marked(&self) -> &[MarkedSequence] {
        &self.marked
    }

    /// The object with this ID.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn object(&self, id: ContentObjectId) -> Option<&ContentObject> {
        self.position(id).and_then(|i| self.objects.get(i))
    }

    /// The lasting name of the object with this ID, from its current bytes: an object an edit
    /// rewrote is named by the bytes the encoder writes for it.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn reference(&self, id: ContentObjectId) -> Option<ContentObjectRef> {
        let ordinal = self.position(id)?;
        let object = self.objects.get(ordinal)?;
        Some(ContentObjectRef {
            owner: self.owner.clone(),
            ordinal: u32::try_from(ordinal).ok()?,
            hash: source_hash(&self.object_bytes(object)),
        })
    }

    /// The object a lasting name names: its owner must be this IR's, its ordinal in range and
    /// its hash the object's. `None` when any of them isn't, so an op log names a stale
    /// object; the ops report that as `content-object-stale` rather than editing another one.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn find(&self, reference: &ContentObjectRef) -> Option<ContentObjectId> {
        if reference.owner != self.owner {
            return None;
        }
        let object = self.objects.get(usize::try_from(reference.ordinal).ok()?)?;
        (source_hash(&self.object_bytes(object)) == reference.hash).then_some(object.id)
    }

    /// The objects whose box holds `point` (or comes within 2 points of it), topmost first.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn hit_test(&self, point: PointPt) -> Vec<ContentObjectId> {
        let (x, y) = (point.x.get(), point.y.get());
        self.hits(|b| {
            x >= b.x0.get() - HIT_SLOP
                && x <= b.x1.get() + HIT_SLOP
                && y >= b.y0.get() - HIT_SLOP
                && y <= b.y1.get() + HIT_SLOP
        })
    }

    /// The objects whose box meets `rect`, topmost first.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn hit_test_rect(&self, rect: &RectPt) -> Vec<ContentObjectId> {
        self.hits(|b| b.x0 <= rect.x1 && rect.x0 <= b.x1 && b.y0 <= rect.y1 && rect.y0 <= b.y1)
    }

    fn hits(&self, test: impl Fn(&RectPt) -> bool) -> Vec<ContentObjectId> {
        self.objects
            .iter()
            .rev()
            .filter(|o| o.bounds.as_ref().is_some_and(&test))
            .map(|o| o.id)
            .collect()
    }

    /// Replaces the object's operators with `ops`, written canonically by the encoder; its ID,
    /// kind and graphics state stay, its box goes until it is measured again. Marked content
    /// wholly inside it goes with its operators; a `BMC`, `BDC` or `EMC` inside it that belongs
    /// to a sequence reaching outside it is kept, written after `ops`, so the content stays
    /// balanced. Returns `false` when no object has the ID.
    ///
    /// Added in 0.5.0.
    pub fn replace(&mut self, id: ContentObjectId, ops: Vec<ContentOp>) -> bool {
        let Some(index) = self.position(id) else {
            return false;
        };
        let Some(range) = self.objects.get(index).map(ContentObject::operators) else {
            return false;
        };
        // An object an edit left empty is written where its next operator starts, or at the end.
        let anchor = self.ops.get(range.start).map_or(self.source.len(), |op| op.token_start);
        self.splice(index, range, anchor, ops);
        if let Some(object) = self.objects.get_mut(index) {
            object.bounds = None;
        }
        true
    }

    /// Deletes the object: its operators go, and the marked content wholly inside it (see
    /// [`ContentOps::replace`] for marked content that reaches outside it). Returns `false`
    /// when no object has the ID.
    ///
    /// Added in 0.5.0.
    pub fn remove(&mut self, id: ContentObjectId) -> bool {
        if !self.replace(id, Vec::new()) {
            return false;
        }
        if let Some(index) = self.position(id) {
            self.objects.remove(index);
        }
        true
    }

    /// Writes the content again ([ADR 0022] §3): each part an edit touched is written from
    /// its source bytes, every operator no edit changed copied byte for byte and the rest
    /// written canonically; every other part is left as it is.
    ///
    /// [ADR 0022]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0022-content-stream-editing.md
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn encode(&self) -> EncodedContent {
        encode::encode(self, false)
    }

    /// [`ContentOps::encode`] with every part written, unchanged ones included: for a caller
    /// that writes the content as new streams whatever changed. With no edit, each part's bytes
    /// equal its source.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn encode_every_part(&self) -> EncodedContent {
        encode::encode(self, true)
    }

    /// A [`Visitor`](crate::Visitor) that records each object's graphics state, kind and box
    /// in page space as the interpreter reads this content (see [`Recorder`]).
    ///
    /// Added in 0.5.0.
    pub fn recorder(&mut self, frame: PageFrame) -> Recorder<'_> {
        // A second recording starts the show list again, so a glyph's show index never finds
        // an object of the first.
        self.shows.clear();
        Recorder::new(self, frame)
    }

    /// For each text-showing operator (`Tj`, `TJ`, `'`, `"`) the page ran, in order, the
    /// object that shows it, or `None` for one a Form XObject shows: what a [`Recorder`] saw.
    /// `laminapdf-text` numbers its glyphs by the same count, so a glyph's show index finds its
    /// object here.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn show_owners(&self) -> &[Option<ContentObjectId>] {
        &self.shows
    }

    /// The object's index in [`ContentOps::objects`]: a binary search while the objects are in
    /// ID order, a scan once a reorder has moved one.
    fn position(&self, id: ContentObjectId) -> Option<usize> {
        if self.reordered {
            self.objects.iter().position(|o| o.id == id)
        } else {
            self.objects.binary_search_by_key(&id, |o| o.id).ok()
        }
    }

    /// The object's bytes, as a lasting name hashes them: its source from its first token
    /// through its last operator, or for written operators what the encoder writes.
    fn object_bytes(&self, object: &ContentObject) -> Cow<'_, [u8]> {
        let ops = self.ops.get(object.operators()).unwrap_or_default();
        match (ops.first(), ops.last()) {
            (Some(first), Some(last)) if ops.iter().all(|op| op.span.is_some()) => {
                let end = last.span.as_ref().map_or(first.token_start, |s| s.end);
                Cow::Borrowed(self.source.get(first.token_start..end).unwrap_or_default())
            }
            _ => {
                let mut out = encode::canonical(ops);
                if out.last() == Some(&b'\n') {
                    out.pop();
                }
                Cow::Owned(out)
            }
        }
    }

    /// Replaces the operators in `range` with `new`, written at `anchor` in the source, and
    /// keeps every index into the operators in step. Marked-content operators in `range` whose
    /// sequence reaches outside it are written again after `new`, outside the object
    /// `object_index`.
    fn splice(&mut self, object_index: usize, range: Range<usize>, anchor: usize, mut new: Vec<ContentOp>) {
        self.edited = true;
        // One hole per run of source operators, from the first one's token: the white space
        // between them goes with them, the white space before the run stays.
        let mut holes: Vec<Range<usize>> = Vec::new();
        for op in self.ops.get(range.clone()).unwrap_or_default() {
            let Some(span) = &op.span else {
                continue;
            };
            match holes.last_mut() {
                Some(open) if open.end == span.start => open.end = span.end,
                _ => holes.push(op.token_start..span.end),
            }
        }
        for hole in holes {
            self.drop_source(hole);
        }
        mark_anchor(&self.parts, &mut self.dirty, anchor);
        let (start, end) = (range.start, range.end);
        let inside = |i: usize| (start..end).contains(&i);
        // The operators inside of sequences that reach outside: one end of a sequence that
        // another operator outside the range closes or opens.
        let mut carried: Vec<usize> = self
            .marked
            .iter()
            .filter_map(|seq| match (inside(seq.begin), seq.end.map(inside)) {
                (true, Some(false) | None) => Some(seq.begin),
                (false, Some(true)) => seq.end,
                _ => None,
            })
            .collect();
        carried.sort_unstable();
        let written = new.len();
        new.extend(carried.iter().filter_map(|&i| self.ops.get(i).cloned()));
        for op in &mut new {
            op.span = None;
            op.token_start = anchor;
        }
        let added = new.len();
        self.ops.splice(range, new);
        let shift = |i: usize| {
            if i >= end {
                i - (end - start) + added
            } else if let Ok(k) = carried.binary_search(&i) {
                start + written + k
            } else {
                i
            }
        };
        // Marked content wholly inside the replaced operators goes with them.
        let mut kept = Vec::with_capacity(self.marked.len());
        let mut renumber = vec![None; self.marked.len()];
        for (old, mut seq) in std::mem::take(&mut self.marked).into_iter().enumerate() {
            if inside(seq.begin) && seq.end.is_some_and(inside) {
                continue;
            }
            seq.begin = shift(seq.begin);
            seq.end = seq.end.map(shift);
            if let Some(slot) = renumber.get_mut(old) {
                *slot = Some(kept.len());
            }
            kept.push(seq);
        }
        self.marked = kept;
        for (index, object) in self.objects.iter_mut().enumerate() {
            // The replaced object by its index: an empty range can't tell it from a neighbour.
            if index == object_index {
                object.ops = start..start + written;
            } else if object.ops.start >= end {
                object.ops = shift(object.ops.start)..shift(object.ops.end);
            }
            object.marked = object
                .marked
                .iter()
                .filter_map(|&m| renumber.get(m).copied().flatten())
                .collect();
        }
    }

    /// Leaves `range` of the source out of what the encoder writes.
    fn drop_source(&mut self, range: Range<usize>) {
        mark_dirty(&self.parts, &mut self.dirty, range.clone());
        self.removed.push(range);
    }
}

/// Marks the parts `range` of the source touches as edited.
fn mark_dirty(parts: &[ContentPart], dirty: &mut [bool], range: Range<usize>) {
    for (part, flag) in parts.iter().zip(dirty.iter_mut()) {
        if range.start < part.end && part.start < range.end {
            *flag = true;
        }
    }
}

/// Marks the part written operators anchored at `anchor` go in: the one it falls in, its end
/// included, so an anchor at the end of the content still has a part.
fn mark_anchor(parts: &[ContentPart], dirty: &mut [bool], anchor: usize) {
    if let Some((_, flag)) = parts
        .iter()
        .zip(dirty.iter_mut())
        .find(|(part, _)| (part.start..=part.end).contains(&anchor))
    {
        *flag = true;
    }
}

/// The 64-bit hash a [`ContentObjectRef`] carries: FNV-1a over `bytes`. Fixed for good, since
/// op logs store it.
///
/// Added in 0.5.0.
#[must_use]
pub fn source_hash(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes
        .iter()
        .fold(OFFSET, |hash, &b| (hash ^ u64::from(b)).wrapping_mul(PRIME))
}

#[cfg(test)]
mod tests;
