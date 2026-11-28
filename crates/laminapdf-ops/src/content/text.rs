//! `EditText`: change the text a page already shows, in the font it is set in (ADR 0022 §5, §6;
//! E7 plan §2.2). A run is the glyphs of one extraction run (one font and size on one baseline,
//! and one marked-content sequence in a tagged page) that one text object shows. The new text is
//! encoded in the codes the font already draws, the run's show operators are written again with
//! them, and the rest of the object's line moves by the width difference.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use laminapdf_content::{
    ContentObjectId, ContentOps, ContentOwner, NotEditable, PageContentWriter, Segment, SegmentStyle, ShownGlyph,
};
use laminapdf_model::{PageId, Pt, RectPt};
use laminapdf_syntax::ObjRef;
use laminapdf_text::{EncodedGlyph, FontEncoder, Glyph, PageText, TextExtractor, is_right_to_left};
use serde::{Deserialize, Serialize};

use super::fonts::{Chain, Source, fidelity};
use super::{ObjectRef, find};
use crate::annot::AnnotFont;
use crate::blob::BlobStore;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::text_edit::TextEditReport;

/// A change in a line's width smaller than this, in points, isn't one.
pub(super) const WIDTH_EPSILON: f64 = 0.01;

/// The sine of the largest angle between two glyphs' advances that still counts as one
/// direction (about 1°).
const TURN: f64 = 0.0175;

/// How far back, in font sizes, a glyph may start from the one before it and still be in order:
/// `TJ` kerning and combining marks move back a little; a show operator that writes a word out
/// of order moves back a whole glyph.
const BACKSTEP: f64 = 0.25;

/// A run of text a page shows, named so the name lasts: the text object, as
/// [`ObjectRef`] names it, and the run's glyphs among the glyphs it shows, `start..end`
/// (0-based, `end` exclusive). On the wire:
/// `{ "object": { "ordinal": 3, "hash": "a3f1c2d4e5f60718" }, "start": 0, "end": 5 }`.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct TextRunRef {
    /// The text object.
    pub object: ObjectRef,
    /// Its first glyph in the run.
    pub start: u32,
    /// The glyph after the run's last.
    pub end: u32,
}

impl TextRunRef {
    /// The glyphs `start..end` of `object`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(object: ObjectRef, start: u32, end: u32) -> Self {
        Self { object, start, end }
    }

    /// The glyphs as indices.
    fn glyphs(self) -> Range<usize> {
        let at = |n: u32| usize::try_from(n).unwrap_or(usize::MAX);
        at(self.start)..at(self.end)
    }
}

/// Changes the text a run shows ([E7 plan] §2.2), in the font it is set in: each character
/// must be one the font already draws on the page (a subset's or a composite font's glyphs),
/// or for a simple font embedded whole or not embedded, any its encoding has. A character
/// with no glyph fails with `missing-glyph`, unless `fonts` names a font that draws it: a full
/// copy of the run's own font adds a new subset of it, any other font substitutes, and the
/// edit's report (and [`plan_edit_text`], before it is committed) names the characters and the
/// font. A run that can't be edited (`content-not-editable`) says why. The text after the run in the same text object moves by the width difference (text
/// on the line that another object places stays put); the lines below stay where
/// they were. Empty text deletes the run. Saving is an incremental update; the old text stays in
/// the file's earlier revisions.
///
/// On the wire:
/// `{ "op": "editText", "args": { "page": "p12.0", "run": { "object": { "ordinal": 3, "hash": "a3f1c2…" }, "start": 0, "end": 5 }, "text": "Hello" } }`.
///
/// [E7 plan]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e7-editing-existing-content.md
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct EditText {
    /// The page.
    #[serde(with = "crate::wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The run, as [`text_runs`] names it.
    pub run: TextRunRef,
    /// The new text.
    pub text: String,
    /// The fonts that may draw a character the run's font has no glyph for, in order: a full
    /// copy of the run's own font (its bytes, added to the session) gives a new subset, any
    /// other font substitutes. Empty when absent, so a missing glyph fails.
    ///
    /// Added in 0.7.0.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fonts: Vec<AnnotFont>,
}

impl EditText {
    /// The op that makes `run` on `page` show `text`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(page: PageId, run: TextRunRef, text: String) -> Self {
        Self {
            page,
            run,
            text,
            fonts: Vec::new(),
        }
    }

    /// The same op, allowed to draw characters the run's font lacks in `fonts`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: Vec<AnnotFont>) -> Self {
        self.fonts = fonts;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document, blobs: &BlobStore) -> Result<Effect> {
        let mut reading = Reading::of(doc, &self.page)?;
        let Prepared {
            id,
            range,
            shown,
            encoded,
            mut chain,
            mcid,
            old_text,
            report,
        } = self.prepare(doc, &mut reading, blobs)?;
        let tagged_before = reading.ir.drawn_mcids();
        let page = {
            let pages = doc.pages()?;
            let page = pages
                .by_id(&self.page)
                .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
            page.cos().clone()
        };
        let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &page)?;
        let names = chain.names(&mut writer)?;
        let segments: Vec<Segment> = encoded
            .iter()
            .map(|(source, glyphs)| {
                let segment = Segment::new(glyphs.clone());
                match source {
                    Source::Chain(at) => match names.get(at) {
                        Some(name) => segment.styled(SegmentStyle::new().with_font(name.clone())),
                        None => segment,
                    },
                    Source::Own => segment,
                }
            })
            .collect();
        reading
            .ir
            .rewrite_segments(id, &shown, range, &segments)
            .map_err(|reason| Error::ContentNotEditable { reason })?;
        // Text deleted from a text object that shows nothing else takes the object, and the
        // marked content that holds only it, with it (ADR 0022 §4).
        if self.text.is_empty() && !reading.ir.object_draws(id) {
            reading
                .ir
                .delete(id)
                .map_err(|reason| Error::ContentNotEditable { reason })?;
        }
        let gone = super::structure::gone(&tagged_before, &reading.ir.drawn_mcids());
        writer.write_parts(&reading.ir, &reading.ir.encode())?;
        chain.write(writer.cos())?;
        drop(writer);
        if let Some(tags) = super::structure::PageTags::read(doc, &self.page)? {
            let owner = mcid.filter(|m| !gone.contains(m)).and_then(|m| tags.owner(m));
            tags.untag(doc, &gone)?;
            if let Some(elem) = owner {
                super::structure::retext(doc, elem, &old_text, &self.text)?;
            }
        }
        Ok(Effect {
            pages: vec![self.page.clone()],
            removed: true,
            text_edits: vec![report],
            ..Effect::default()
        })
    }

    /// Reads the run, cuts the new text into the fonts that draw it and encodes it, without
    /// writing anything.
    fn prepare(
        &self,
        doc: &laminapdf_model::Document,
        reading: &mut Reading,
        blobs: &BlobStore,
    ) -> Result<Prepared> {
        let id = find(&reading.ir, &self.page, self.run.object)?;
        let shown_ids = reading.objects.get(&id).cloned().unwrap_or_default();
        let range = self.run.glyphs();
        let run = shown_ids
            .get(range.clone())
            .filter(|run| !run.is_empty())
            .ok_or(Error::InvalidOp("editText's glyphs aren't among the object's"))?;
        if !reading.one_show(run) {
            return Err(Error::InvalidOp("editText's glyphs aren't drawn by one show operator"));
        }
        let font = reading
            .font_of(run)
            .ok_or(Error::InvalidOp("editText's glyphs aren't in one font"))?;
        reading.load(doc, font)?;
        if let Some(reason) = reading.ir.edit_refusal(id).or_else(|| reading.refusal(run, font)) {
            return Err(Error::ContentNotEditable { reason });
        }
        let encoder = reading
            .encoders
            .get(&font)
            .ok_or(Error::InvalidOp("the font wasn't read"))?;
        let picked: Vec<&Glyph> = run.iter().filter_map(|&g| reading.text.glyphs.get(g)).collect();
        let own_name = picked.first().map_or_else(String::new, |g| g.font_name.to_string());
        let mut chain = Chain::new(&self.fonts, blobs, &own_name)?;
        let stretches = chain.cut(Some(encoder), &self.text, &own_name)?;
        let mut encoded: Vec<(Source, Vec<ShownGlyph>)> = Vec::with_capacity(stretches.len());
        for stretch in &stretches {
            let glyphs = match stretch.source {
                Source::Own => encode(encoder, &stretch.text).map_err(|ch| {
                    Error::Text(laminapdf_text::Error::MissingGlyph {
                        ch,
                        font: own_name.clone(),
                    })
                })?,
                Source::Chain(at) => chain.encode(at, &stretch.text)?,
            };
            encoded.push((stretch.source, glyphs));
        }
        let size = picked.first().map_or(0.0, |g| g.size);
        let new_width: f64 = encoded
            .iter()
            .flat_map(|(_, glyphs)| glyphs.iter())
            .map(|g| g.width / 1000.0 * size)
            .sum();
        let old_width: f64 = picked.iter().map(|g| g.advance.0.hypot(g.advance.1)).sum();
        let substitutions = chain.substitutions(&stretches);
        let report = TextEditReport {
            page: self.page.clone(),
            ordinal: self.run.object.ordinal,
            start: self.run.start,
            end: self.run.end,
            fidelity: fidelity(&substitutions, new_width - old_width),
            substitutions,
            grew_by: (new_width - old_width).max(0.0),
        };
        let shown: Vec<ShownGlyph> = shown_ids
            .iter()
            .filter_map(|&g| reading.text.glyphs.get(g))
            .map(|g| shown_glyph(encoder.glyph(g.code)))
            .collect();
        Ok(Prepared {
            id,
            range,
            shown,
            encoded,
            chain,
            mcid: picked.first().and_then(|g| g.mcid),
            old_text: joined(&picked),
            report,
        })
    }
}

/// What reading an [`EditText`] found, ready to write.
struct Prepared {
    id: ContentObjectId,
    range: Range<usize>,
    /// Every glyph the object shows, as its show operators hold them.
    shown: Vec<ShownGlyph>,
    /// The new text, each stretch in the font that draws it.
    encoded: Vec<(Source, Vec<ShownGlyph>)>,
    chain: Chain,
    mcid: Option<i64>,
    old_text: String,
    report: TextEditReport,
}

/// What `op` would do, without doing it: the characters the run's font can't draw and the fonts
/// that would draw them, and how wide the line would grow. An editor shows this before it commits
/// the edit, so a font is never changed silently.
///
/// # Errors
/// As applying the op: `unknown-page`, `content-object-stale`, `content-not-editable` and
/// `missing-glyph`.
///
/// Added in 0.7.0.
pub fn plan_edit_text(doc: &laminapdf_model::Document, op: &EditText, blobs: &BlobStore) -> Result<TextEditReport> {
    let mut reading = Reading::of(doc, &op.page)?;
    Ok(op.prepare(doc, &mut reading, blobs)?.report)
}

/// A run of text a page shows, as [`text_runs`] lists it.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ContentTextRun {
    /// Its name, which [`EditText`] takes.
    pub run: TextRunRef,
    /// The text its glyphs stand for, with a space where the pen jumps forward between two of
    /// them (a word space a `TJ` number makes): text for [`EditText`] to change.
    pub text: String,
    /// The box around its glyphs, in page space.
    pub bounds: RectPt,
    /// The font's `/BaseFont`.
    pub font_name: String,
    /// The size its glyphs are drawn at, in user space.
    pub font_size: f64,
    /// The line it is on: an index into the page's lines, in the order the content draws them.
    pub line: u32,
    /// Why it can't be edited, or `None` when it can: an editor shows it read-only, with this
    /// reason.
    pub read_only: Option<NotEditable>,
}

/// The runs of text page `page` shows, in the order the content draws them. A run is the glyphs
/// of one extraction run (one font and size on one baseline, within one marked-content sequence)
/// that one show operator draws. Text inside a Form XObject isn't listed: it belongs to the form,
/// which [`EditContent`](crate::EditContent) moves whole.
///
/// # Errors
/// `unknown-page`; errors reading the page's content or its fonts, and limits it exceeds.
///
/// Added in 0.5.0.
pub fn text_runs(doc: &laminapdf_model::Document, page: &PageId) -> Result<Vec<ContentTextRun>> {
    let mut reading = Reading::of(doc, page)?;
    let mut lines = vec![0_u32; reading.text.runs.len()];
    for (index, line) in reading.text.text_lines().iter().enumerate() {
        for run in line.runs.clone() {
            if let Some(slot) = lines.get_mut(run) {
                *slot = u32::try_from(index).unwrap_or(u32::MAX);
            }
        }
    }
    let mut pieces = Vec::new();
    for (index, run) in reading.text.runs.iter().enumerate() {
        for (id, glyphs, start) in reading.pieces(run.glyphs.clone()) {
            pieces.push((index, id, glyphs, start));
        }
    }
    let fonts: BTreeSet<ObjRef> = pieces
        .iter()
        .filter_map(|(_, _, glyphs, _)| reading.font_of(glyphs))
        .collect();
    for font in fonts {
        reading.load(doc, font)?;
    }
    let mut objects: BTreeMap<ContentObjectId, Option<NotEditable>> = BTreeMap::new();
    let mut out = Vec::with_capacity(pieces.len());
    for (index, id, glyphs, start) in pieces {
        let (Some(name), Some(run)) = (reading.ir.reference(id), reading.text.runs.get(index)) else {
            continue;
        };
        let object = *objects.entry(id).or_insert_with(|| reading.ir.edit_refusal(id));
        let read_only = object.or_else(|| match reading.font_of(&glyphs) {
            Some(font) => reading.refusal(&glyphs, font),
            None => Some(NotEditable::ContentUnreadable),
        });
        let picked: Vec<&Glyph> = glyphs.iter().filter_map(|&g| reading.text.glyphs.get(g)).collect();
        let count = u32::try_from(glyphs.len()).unwrap_or(u32::MAX);
        out.push(ContentTextRun {
            run: TextRunRef::new(ObjectRef::from(&name), start, start.saturating_add(count)),
            text: joined(&picked),
            bounds: bounds(&picked),
            font_name: run.font_name.to_string(),
            font_size: run.font_size,
            line: lines.get(index).copied().unwrap_or(0),
            read_only,
        });
    }
    Ok(out)
}

/// A page read once for its text: the content IR with each object's state and box, the
/// glyphs extraction found, and which object shows each glyph.
pub(super) struct Reading {
    pub(super) ir: ContentOps,
    pub(super) text: PageText,
    pub(super) extractor: TextExtractor,
    /// Each object's glyphs, as indices into `text.glyphs`, in the order it shows them.
    pub(super) objects: BTreeMap<ContentObjectId, Vec<usize>>,
    /// Each glyph's object and its index among the object's glyphs; `None` inside a form.
    pub(super) owners: Vec<Option<(ContentObjectId, u32)>>,
    /// Each font's encoder, built once.
    pub(super) encoders: BTreeMap<ObjRef, FontEncoder>,
    /// For each show operator the page runs (as a glyph's `show` counts them), its place among
    /// the show operators of the object that draws it.
    pub(super) local_shows: Vec<Option<usize>>,
}

impl Reading {
    pub(super) fn of(doc: &laminapdf_model::Document, page: &PageId) -> Result<Self> {
        let pages = doc.pages()?;
        let model_page = pages.by_id(page).ok_or_else(|| Error::UnknownPage(page.clone()))?;
        let mut ir = ContentOps::read_page(doc, model_page.cos(), ContentOwner::Page(page.clone()))?;
        let mut extractor = TextExtractor::new();
        let text = {
            let mut recorder = ir.recorder(model_page.frame()?);
            extractor.page_with(doc, &model_page, &mut recorder)?
        };
        let shows = ir.show_owners();
        let mut counts: BTreeMap<ContentObjectId, usize> = BTreeMap::new();
        let local_shows = shows
            .iter()
            .map(|owner| {
                owner.map(|id| {
                    let n = counts.entry(id).or_insert(0);
                    *n += 1;
                    *n - 1
                })
            })
            .collect();
        let mut objects: BTreeMap<ContentObjectId, Vec<usize>> = BTreeMap::new();
        let mut owners = Vec::with_capacity(text.glyphs.len());
        for (index, glyph) in text.glyphs.iter().enumerate() {
            let owner = usize::try_from(glyph.show)
                .ok()
                .and_then(|s| shows.get(s).copied().flatten());
            owners.push(owner.map(|id| {
                let list = objects.entry(id).or_default();
                list.push(index);
                (id, u32::try_from(list.len() - 1).unwrap_or(u32::MAX))
            }));
        }
        Ok(Self {
            ir,
            text,
            extractor,
            objects,
            owners,
            encoders: BTreeMap::new(),
            local_shows,
        })
    }

    /// The glyphs `glyphs` split by the show operator that draws them: each piece's object, its
    /// glyphs and the first one's index among the object's. Glyphs inside a form are left out.
    pub(super) fn pieces(&self, glyphs: Range<usize>) -> Vec<(ContentObjectId, Vec<usize>, u32)> {
        let mut out: Vec<(ContentObjectId, Vec<usize>, u32)> = Vec::new();
        for g in glyphs {
            let Some(Some((id, at))) = self.owners.get(g).copied() else {
                continue;
            };
            match out.last_mut() {
                Some((object, piece, first))
                    if *object == id
                        && next_of(*first, piece) == Some(at)
                        && piece.first().is_some_and(|&f| self.one_show(&[f, g])) =>
                {
                    piece.push(g);
                }
                _ => out.push((id, vec![g], at)),
            }
        }
        out
    }

    /// Whether one show operator draws every glyph of `glyphs`.
    pub(super) fn one_show(&self, glyphs: &[usize]) -> bool {
        let mut shows = glyphs.iter().map(|&g| self.text.glyphs.get(g).map(|g| g.show));
        let first = shows.next().flatten();
        first.is_some() && shows.all(|s| s == first)
    }

    /// The one font the glyphs are shown in, when it is an indirect object.
    pub(super) fn font_of(&self, glyphs: &[usize]) -> Option<ObjRef> {
        let mut fonts = glyphs.iter().map(|&g| self.text.glyphs.get(g).and_then(|g| g.font));
        let first = fonts.next()??;
        fonts.all(|f| f == Some(first)).then_some(first)
    }

    /// Builds the encoder for `font`, with every code the page shows in it, unless it is built.
    pub(super) fn load(&mut self, doc: &laminapdf_model::Document, font: ObjRef) -> Result<()> {
        if self.encoders.contains_key(&font) {
            return Ok(());
        }
        let used: Vec<_> = self
            .text
            .glyphs
            .iter()
            .filter(|g| g.font == Some(font))
            .map(|g| g.code)
            .collect();
        let encoder = self.extractor.font_encoder(doc, font, &used)?;
        self.encoders.insert(font, encoder);
        Ok(())
    }

    /// Why the glyphs `glyphs`, in `font` (loaded), can't be edited, their object aside: the
    /// font's reason, text that stands in for an `ActualText` or runs right to left, or glyphs
    /// turned against each other or written out of order (ADR 0022 §5).
    pub(super) fn refusal(&self, glyphs: &[usize], font: ObjRef) -> Option<NotEditable> {
        let encoder = self.encoders.get(&font)?;
        let picked: Vec<&Glyph> = glyphs.iter().filter_map(|&g| self.text.glyphs.get(g)).collect();
        encoder
            .refusal()
            .or_else(|| {
                picked
                    .iter()
                    .any(|g| g.actual_text.is_some())
                    .then_some(NotEditable::ActualText)
            })
            .or_else(|| {
                let rtl = picked.iter().any(|g| g.text.chars().any(is_right_to_left));
                rtl.then_some(NotEditable::RightToLeft)
            })
            .or_else(|| order_refusal(&picked))
    }
}

/// [`NotEditable::RotatedRun`] when the glyphs advance in different directions,
/// [`NotEditable::GlyphsOutOfOrder`] when one starts well before the one shown before it.
fn order_refusal(glyphs: &[&Glyph]) -> Option<NotEditable> {
    let length = |(x, y): (f64, f64)| x.hypot(y);
    let (dx, dy) = glyphs
        .iter()
        .map(|g| g.advance)
        .find(|&a| length(a) > f64::EPSILON)
        .map(|a| (a.0 / length(a), a.1 / length(a)))?;
    let mut previous: Option<f64> = None;
    let first = glyphs.first()?.origin;
    for glyph in glyphs {
        let advance = length(glyph.advance);
        if advance > f64::EPSILON && (glyph.advance.0 * dy - glyph.advance.1 * dx).abs() / advance > TURN {
            return Some(NotEditable::RotatedRun);
        }
        let along = (glyph.origin.x - first.x) * dx + (glyph.origin.y - first.y) * dy;
        if previous.is_some_and(|p| along < p - BACKSTEP * glyph.size) {
            return Some(NotEditable::GlyphsOutOfOrder);
        }
        previous = Some(along);
    }
    None
}

/// The index after a piece's last glyph, among its object's glyphs.
fn next_of(first: u32, piece: &[usize]) -> Option<u32> {
    u32::try_from(piece.len()).ok().and_then(|n| first.checked_add(n))
}

/// The smallest page-space rectangle around the glyphs' boxes.
fn bounds(glyphs: &[&Glyph]) -> RectPt {
    let points = glyphs.iter().flat_map(|g| g.quad.iter());
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        x0 = x0.min(p.x.get());
        y0 = y0.min(p.y.get());
        x1 = x1.max(p.x.get());
        y1 = y1.max(p.y.get());
    }
    if x0 > x1 {
        return RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(0.0), Pt::new(0.0));
    }
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// How far forward, in font sizes, the pen must jump between two glyphs for [`joined`] to put a
/// space between their texts.
const WORD_JUMP: f64 = 0.2;

/// The gap a space the font can't draw becomes, in thousandths of text space: a third of an
/// em, the word space TeX writes as a `TJ` number, which extraction reads as a space.
const WORD_GAP: f64 = 333.0;

/// The glyphs' text, with a space where the pen jumps forward between two of them.
pub(super) fn joined(glyphs: &[&Glyph]) -> String {
    let mut text = String::new();
    let mut previous: Option<&Glyph> = None;
    for glyph in glyphs {
        if let Some(before) = previous {
            let end = (before.origin.x + before.advance.0, before.origin.y + before.advance.1);
            let jump = (glyph.origin.x - end.0).hypot(glyph.origin.y - end.1);
            let spaced = text.ends_with(' ') || glyph.text.starts_with(' ');
            if jump > WORD_JUMP * glyph.size && !spaced {
                text.push(' ');
            }
        }
        text.push_str(&glyph.text);
        previous = Some(glyph);
    }
    text
}

/// `text` in the font's codes; a space the font can't draw becomes a [`WORD_GAP`].
///
/// # Errors
/// The first character the font can't draw.
pub(super) fn encode(encoder: &FontEncoder, text: &str) -> std::result::Result<Vec<ShownGlyph>, char> {
    if encoder.encode(" ").is_ok() {
        return Ok(encoder.encode(text)?.into_iter().map(shown_glyph).collect());
    }
    let mut out = Vec::new();
    for (index, word) in text.split(' ').enumerate() {
        if index > 0 {
            out.push(ShownGlyph::gap(WORD_GAP));
        }
        out.extend(encoder.encode(word)?.into_iter().map(shown_glyph));
    }
    Ok(out)
}

/// An encoded glyph as the IR rewrites it.
pub(super) fn shown_glyph(glyph: EncodedGlyph) -> ShownGlyph {
    ShownGlyph::new(glyph.code.value, glyph.code.len, glyph.width, glyph.word_space)
}
