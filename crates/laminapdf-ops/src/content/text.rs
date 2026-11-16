//! `EditText`: change the text a page already shows, in the font it is set in (ADR 0022 §5, §6;
//! E7 plan §2.2). A run is the glyphs of one extraction run (one font and size on one baseline,
//! and one marked-content sequence in a tagged page) that one text object shows. The new text is
//! encoded in the codes the font already draws, the run's show operators are written again with
//! them, and the rest of the object's line moves by the width difference.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use laminapdf_content::{ContentObjectId, ContentOps, ContentOwner, NotEditable, PageContentWriter, ShownGlyph};
use laminapdf_model::{PageId, Pt, RectPt};
use laminapdf_syntax::ObjRef;
use laminapdf_text::{EncodedGlyph, FontEncoder, Glyph, PageText, TextExtractor, is_right_to_left};
use serde::{Deserialize, Serialize};

use super::{ObjectRef, find};
use crate::error::{Error, Result};
use crate::op::Effect;

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
/// with no glyph fails with `missing-glyph`; a run that can't be edited (`content-not-editable`)
/// says why. The text after the run in the same text object moves by the width difference (text
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
}

impl EditText {
    /// The op that makes `run` on `page` show `text`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(page: PageId, run: TextRunRef, text: String) -> Self {
        Self { page, run, text }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut reading = Reading::of(doc, &self.page)?;
        let id = find(&reading.ir, &self.page, self.run.object)?;
        let shown = reading.objects.get(&id).cloned().unwrap_or_default();
        let range = self.run.glyphs();
        let run = shown
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
        let new = encode(encoder, &self.text).map_err(|ch| {
            let font = run
                .first()
                .and_then(|&g| reading.text.glyphs.get(g))
                .map_or_else(String::new, |g| g.font_name.to_string());
            Error::Text(laminapdf_text::Error::MissingGlyph { ch, font })
        })?;
        let glyphs: Vec<ShownGlyph> = shown
            .iter()
            .filter_map(|&g| reading.text.glyphs.get(g))
            .map(|g| shown_glyph(encoder.glyph(g.code)))
            .collect();
        reading
            .ir
            .rewrite_text(id, &glyphs, range, &new)
            .map_err(|reason| Error::ContentNotEditable { reason })?;
        let page = {
            let pages = doc.pages()?;
            let page = pages
                .by_id(&self.page)
                .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
            page.cos().clone()
        };
        let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &page)?;
        writer.write_parts(&reading.ir, &reading.ir.encode())?;
        Ok(Effect {
            pages: vec![self.page.clone()],
            removed: true,
            ..Effect::default()
        })
    }
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
struct Reading {
    ir: ContentOps,
    text: PageText,
    extractor: TextExtractor,
    /// Each object's glyphs, as indices into `text.glyphs`, in the order it shows them.
    objects: BTreeMap<ContentObjectId, Vec<usize>>,
    /// Each glyph's object and its index among the object's glyphs; `None` inside a form.
    owners: Vec<Option<(ContentObjectId, u32)>>,
    /// Each font's encoder, built once.
    encoders: BTreeMap<ObjRef, FontEncoder>,
}

impl Reading {
    fn of(doc: &laminapdf_model::Document, page: &PageId) -> Result<Self> {
        let pages = doc.pages()?;
        let model_page = pages.by_id(page).ok_or_else(|| Error::UnknownPage(page.clone()))?;
        let mut ir = ContentOps::read_page(doc, model_page.cos(), ContentOwner::Page(page.clone()))?;
        let mut extractor = TextExtractor::new();
        let text = {
            let mut recorder = ir.recorder(model_page.frame()?);
            extractor.page_with(doc, &model_page, &mut recorder)?
        };
        let shows = ir.show_owners();
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
        })
    }

    /// The glyphs `glyphs` split by the show operator that draws them: each piece's object, its
    /// glyphs and the first one's index among the object's. Glyphs inside a form are left out.
    fn pieces(&self, glyphs: Range<usize>) -> Vec<(ContentObjectId, Vec<usize>, u32)> {
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
    fn one_show(&self, glyphs: &[usize]) -> bool {
        let mut shows = glyphs.iter().map(|&g| self.text.glyphs.get(g).map(|g| g.show));
        let first = shows.next().flatten();
        first.is_some() && shows.all(|s| s == first)
    }

    /// The one font the glyphs are shown in, when it is an indirect object.
    fn font_of(&self, glyphs: &[usize]) -> Option<ObjRef> {
        let mut fonts = glyphs.iter().map(|&g| self.text.glyphs.get(g).and_then(|g| g.font));
        let first = fonts.next()??;
        fonts.all(|f| f == Some(first)).then_some(first)
    }

    /// Builds the encoder for `font`, with every code the page shows in it, unless it is built.
    fn load(&mut self, doc: &laminapdf_model::Document, font: ObjRef) -> Result<()> {
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
    fn refusal(&self, glyphs: &[usize], font: ObjRef) -> Option<NotEditable> {
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
fn joined(glyphs: &[&Glyph]) -> String {
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
fn encode(encoder: &FontEncoder, text: &str) -> std::result::Result<Vec<ShownGlyph>, char> {
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
fn shown_glyph(glyph: EncodedGlyph) -> ShownGlyph {
    ShownGlyph::new(glyph.code.value, glyph.code.len, glyph.width, glyph.word_space)
}
