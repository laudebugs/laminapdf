//! A shaped run as PDF text operations (todo B4b; research 03 §2.6–2.7, §5.3, §7.2): the
//! glyphs' codes for `TJ`, the `TJ` numbers that turn the font's widths into the shaper's
//! advances and offsets, `Ts` rises for vertical offsets, the glyph → text entries of the
//! ToUnicode CMap (one CID per glyph and text, B4d), and the clusters that need `/ActualText`.
//!
//! The arithmetic (ISO 32000-2 §9.4.4): after a glyph the pen moves by
//! `(w0 − Tj/1000) × Tfs`, with `w0` the glyph's `/W` width in thousandths of an em. The
//! shaper wants glyph *i* drawn at `pen + x_offset` and the pen then moved by `x_advance`.
//! So before each glyph a `TJ` number moves the pen by what the previous glyph still owed
//! (`x_advance − x_offset − w0`) plus this glyph's `x_offset`; after the last glyph a final
//! number settles the run's advance. Widths are the ones `/W` is written with (the same
//! rounding, to 1/1000 of a unit), so the numbers are exact: `TJ` values here have at most
//! three decimals, and computing them is integer arithmetic, deterministic on every target.
//!
//! Vertical offsets (marks above a base, cursive attachment) become the text rise `Ts`
//! (§9.3.7), which does not move the pen; the run is cut into segments of equal rise.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use icu_properties::CodePointSetData;
use icu_properties::props::DefaultIgnorableCodePoint;
use skrifa::MetadataProvider;
use skrifa::instance::Size;

use crate::embed::{FontKey, FontRegistry, milli};
use crate::error::{Error, Result};
use crate::font::GlyphId;
use crate::itemize::needs_glyph;
use crate::shape::{ShapedGlyph, ShapedRun, sub};

/// One element of a `TJ` array.
#[derive(Debug, Clone, PartialEq)]
pub enum TjElement {
    /// Glyph codes to show: two bytes per glyph (the CID, Identity-H).
    Codes(Vec<u8>),
    /// A `TJ` number in thousandths of text space: positive moves the pen left (back), so
    /// `-50` adds a twentieth of an em of space.
    Adjust(f64),
}

/// Glyphs drawn at one text rise.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PdfSegment {
    /// The rise in thousandths of the font size: set `Ts` to `rise × size / 1000` points
    /// before the segment's `TJ`, and back to 0 after the run.
    pub rise: f64,
    /// The `TJ` array.
    pub tj: Vec<TjElement>,
}

/// A stretch of the run that is either extracted through ToUnicode (`actual_text: None`) or
/// must be wrapped in `/Span << /ActualText (…) >> BDC … EMC` (ISO 32000-2 §14.9.4).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PdfPiece {
    /// The replacement text, in logical order, when ToUnicode alone can't reproduce it.
    pub actual_text: Option<String>,
    /// The piece's glyphs, in drawing order.
    pub segments: Vec<PdfSegment>,
}

/// A shaped run ready to be written: pieces in drawing order and the run's total advance.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PdfRun {
    /// The pieces, left to right.
    pub pieces: Vec<PdfPiece>,
    /// How far the pen moves over the whole run, in thousandths of the font size (the sum
    /// of the shaper's advances): `advance × size / 1000` points.
    pub advance: f64,
}

impl PdfRun {
    /// Whether some piece needs `/ActualText`.
    #[must_use]
    pub fn needs_actual_text(&self) -> bool {
        self.pieces.iter().any(|p| p.actual_text.is_some())
    }
}

/// Turns `run` (shaped from `text` with the font registered as `key`) into PDF text
/// operations, recording every glyph and the text it stands for in `fonts`.
///
/// Text mapping, per cluster (the glyphs sharing one cluster value and the characters up to
/// the next cluster):
/// * one glyph: it stands for the cluster's text (`ﬁ` → "fi", lam-alef → "لا");
/// * several glyphs: each that is the font's own glyph for one of the cluster's characters
///   stands for that character, the widest other glyph for the characters left over, and
///   the rest for nothing.
///
/// With TrueType outlines a glyph gets one CID per text it stands for (B4d, [`crate::FontUse`]),
/// so the texts are then dealt out again in reading order (drawing order, reversed in a
/// right-to-left run), each glyph keeping its share's length: a cluster the shaper
/// reordered (a pre-base matra, a reph, marks in another order) reads back as its text
/// from the codes alone, and a glyph that stands for nothing (Noto Sans Arabic's separate
/// dots, a conjunct part, the space glyph a hidden ZWJ is drawn with) gets a textless CID
/// (`<cid> <>`, a format character's code point that pdf.js skips). No `/ActualText` is
/// needed, except for a cluster holding default-ignorable characters (ZWJ, ZWNJ, the bidi
/// controls), which are never a glyph's text, and for a cluster with a glyph whose CID
/// doesn't extract as its text after all: a textless glyph once the textless CIDs are used
/// up (pdf.js would read its code), or a second text once the font's CIDs run out
/// ([`crate::Warning::ToUnicodeConflict`]).
///
/// With CFF outlines (`CIDFontType0`: one CID per glyph) B4b's mapping stays: a glyph that is
/// some character's own claims no text when it stands for nothing, and a cluster gets
/// `/ActualText` when its glyphs, read in reading order, don't spell its text, when one of
/// them stands for nothing, or when one of them already stands for another text in this font
/// ([`crate::Warning::ToUnicodeConflict`]). Either way the whole run gets `/ActualText` when
/// its clusters are not in visual order for its direction.
///
/// Everything is checked before anything is recorded: on an error, `fonts` is unchanged.
///
/// # Errors
/// [`Error::FontMismatch`] when `run` was shaped with another font than `key`'s;
/// [`Error::MissingGlyph`] for a `.notdef` glyph, naming the character the font lacks;
/// [`Error::UnknownFont`]; [`Error::InvalidTextRange`] when the run's range or a cluster is
/// not in `text`; [`Error::GlyphOutOfRange`].
pub fn pdf_run(run: &ShapedRun, text: &str, fonts: &mut FontRegistry, key: FontKey) -> Result<PdfRun> {
    let run_text = sub(text, run.text_range.clone())?;
    let font = fonts.try_font(key)?.clone();
    let info = font.info();
    check_run(run, text, info)?;
    let font_ref = font.font_ref()?;
    let charmap = font_ref.charmap();
    let groups = cluster_groups(run);
    let bounds = cluster_bounds(run);
    let cluster_text = |cluster: usize| -> &str {
        let next = bounds.partition_point(|&b| b <= cluster);
        let end = bounds.get(next).copied().unwrap_or(run.text_range.end);
        text.get(cluster..end).unwrap_or_default()
    };
    if let Some(g) = run.glyphs.iter().find(|g| g.gid == GlyphId::NOTDEF) {
        let cluster = cluster_text(g.cluster);
        let ch = cluster
            .chars()
            .find(|&ch| needs_glyph(ch) && charmap.map(ch).is_none_or(|g| g.to_u32() == 0))
            .or_else(|| cluster.chars().next())
            .unwrap_or_default();
        return Err(Error::MissingGlyph {
            ch,
            font: info.postscript_name.clone(),
        });
    }

    // Plan every glyph's claim (nothing is recorded yet).
    let mut targets: Option<BTreeSet<u32>> = None;
    let mut is_target = |gid: GlyphId| {
        targets
            .get_or_insert_with(|| charmap.mappings().map(|(_, g)| g.to_u32()).collect())
            .contains(&u32::from(gid.get()))
    };
    let mut claims: Vec<Claim<'_>> = Vec::with_capacity(run.glyphs.len());
    let mut actual: Vec<Option<String>> = Vec::with_capacity(groups.len());
    // B4d: a glyph may get one CID per text (TrueType outlines), so each glyph's text can be
    // chosen for the extractors alone; with CFF outlines it keeps its first text (B4b).
    let splits = fonts.usage(key)?.embedded().is_some_and(crate::FontUse::splits_texts);
    let mut kept = if splits {
        BTreeMap::new()
    } else {
        kept_texts(fonts, key)
    };
    for group in &groups {
        let glyphs = run.glyphs.get(group.clone()).unwrap_or_default();
        let cluster = glyphs.first().map(|g| cluster_text(g.cluster)).unwrap_or_default();
        let nominal = |gid: GlyphId| {
            cluster
                .chars()
                .find(|&ch| charmap.map(ch).is_some_and(|g| g.to_u32() == u32::from(gid.get())))
        };
        if splits && glyphs.len() == 1 {
            // What split_cluster and in_reading_order give one glyph, without their copies:
            // the cluster's visible characters, which spell it unless it has default-ignorables.
            let (text, needs_actual) = one_glyph_text(cluster);
            actual.push(needs_actual.then(|| cluster.to_owned()));
            claims.push(Claim::Text(text));
            continue;
        }
        let (texts, mut needs_actual) = split_cluster(run, glyphs, cluster, &nominal);
        if splits {
            let (texts, needs_actual) = in_reading_order(run, &texts, cluster);
            claims.extend(texts.into_iter().map(|t| Claim::Text(Cow::Owned(t))));
            actual.push(needs_actual.then(|| cluster.to_owned()));
            continue;
        }
        for (g, t) in glyphs.iter().zip(texts) {
            let claim = one_cid_claim(g.gid, t, &mut is_target, &mut kept, &mut needs_actual);
            claims.push(claim);
        }
        actual.push(needs_actual.then(|| cluster.to_owned()));
    }
    let in_order = in_visual_order(run, &groups);
    if !in_order {
        // The whole run is wrapped instead.
        actual.fill(None);
    }

    // Record. A glyph whose CID doesn't extract as its text after all (a conflict once the
    // CIDs run out, or a textless glyph without a textless slot) puts its cluster under
    // `/ActualText` (the whole run's span covers it when the run is out of order).
    let mut cids = Vec::with_capacity(run.glyphs.len());
    for (i, (g, claim)) in run.glyphs.iter().zip(&claims).enumerate() {
        cids.push(match claim {
            Claim::Text(t) => {
                let (cid, exact) = fonts.use_glyph_exact(key, g.gid, t)?;
                let group = groups.partition_point(|r| r.end <= i);
                if let (false, true, Some(slot)) = (exact, in_order, actual.get_mut(group)) {
                    slot.get_or_insert_with(|| cluster_text(g.cluster).to_owned());
                }
                cid
            }
            Claim::Nothing => fonts.glyph_cid(key, g.gid)?,
        });
    }

    let metrics = font_ref.glyph_metrics(Size::unscaled(), font.location_ref());
    let width = |gid: GlyphId| crate::embed::subset::advance_milli(&metrics, gid, info.units_per_em);
    let (mut pieces, advance) = place(run, &width, &groups, &cids, &actual);
    if !in_order {
        pieces = vec![PdfPiece {
            actual_text: Some(run_text.to_owned()),
            segments: pieces.into_iter().flat_map(|p| p.segments).collect(),
        }];
    }
    Ok(PdfRun {
        pieces,
        advance: milli_to_f64(advance),
    })
}

/// Checks that `run` was shaped with the font `info` describes and that its glyphs and
/// clusters are in range.
fn check_run(run: &ShapedRun, text: &str, info: &crate::FontInfo) -> Result<()> {
    if run.font != info.postscript_name || run.units_per_em != info.units_per_em {
        return Err(Error::FontMismatch {
            font: info.postscript_name.clone(),
            shaped_with: run.font.clone(),
        });
    }
    if let Some(g) = run.glyphs.iter().find(|g| g.gid.get() >= info.glyph_count) {
        return Err(Error::GlyphOutOfRange {
            gid: g.gid.get(),
            count: info.glyph_count,
            font: info.postscript_name.clone(),
        });
    }
    if run
        .glyphs
        .iter()
        .any(|g| !run.text_range.contains(&g.cluster) || !text.is_char_boundary(g.cluster))
    {
        return Err(Error::InvalidTextRange {
            start: run.text_range.start,
            end: run.text_range.end,
            len: text.len(),
        });
    }
    Ok(())
}

/// Whether the clusters of `groups` run left to right in a left-to-right run (right to left
/// in a right-to-left one): no reordering across clusters.
fn in_visual_order(run: &ShapedRun, groups: &[Range<usize>]) -> bool {
    let firsts: Vec<usize> = groups
        .iter()
        .filter_map(|g| run.glyphs.get(g.start).map(|g| g.cluster))
        .collect();
    if run.direction.is_rtl() {
        firsts.windows(2).all(|w| w.first() > w.get(1))
    } else {
        firsts.windows(2).all(|w| w.first() < w.get(1))
    }
}

/// A glyph's claim in a font with one CID per glyph (CFF outlines; B4b's mapping): its text;
/// for a glyph given none, nothing when it is some character's own glyph and the empty text
/// otherwise. Sets `needs_actual` when the glyph stands for nothing or already stands for
/// another text (`kept`, updated with the first text of each glyph).
fn one_cid_claim(
    gid: GlyphId,
    text: Option<String>,
    is_target: &mut impl FnMut(GlyphId) -> bool,
    kept: &mut BTreeMap<GlyphId, String>,
    needs_actual: &mut bool,
) -> Claim<'static> {
    let claim = match text {
        Some(t) => Claim::Text(Cow::Owned(t)),
        None if is_target(gid) => {
            *needs_actual = true;
            Claim::Nothing
        }
        None => {
            *needs_actual = true;
            Claim::Text(Cow::Borrowed(""))
        }
    };
    if let Claim::Text(t) = &claim {
        match kept.get(&gid) {
            Some(k) if k != t => *needs_actual = true,
            Some(_) => {}
            None => {
                kept.insert(gid, t.clone().into_owned());
            }
        }
    }
    claim
}

/// Glyph → the text it already stands for in `key`'s ToUnicode map.
fn kept_texts(fonts: &FontRegistry, key: FontKey) -> BTreeMap<GlyphId, String> {
    let Some(usage) = fonts.usage(key).ok().and_then(crate::FontUsage::embedded) else {
        return BTreeMap::new();
    };
    usage
        .glyphs()
        .into_iter()
        .filter_map(|(cid, gid)| usage.text().get(&cid).map(|t| (gid, t.clone())))
        .collect()
}

/// What a glyph is recorded as standing for.
enum Claim<'t> {
    /// This text (possibly empty) in the ToUnicode CMap.
    Text(Cow<'t, str>),
    /// Nothing: the glyph is drawn but its ToUnicode entry is left to other uses.
    Nothing,
}

/// The glyph index ranges of consecutive glyphs sharing one cluster value.
fn cluster_groups(run: &ShapedRun) -> Vec<Range<usize>> {
    let mut groups: Vec<Range<usize>> = Vec::new();
    let mut previous = None;
    for (i, g) in run.glyphs.iter().enumerate() {
        match groups.last_mut() {
            Some(group) if previous == Some(g.cluster) => group.end = i + 1,
            _ => groups.push(i..i + 1),
        }
        previous = Some(g.cluster);
    }
    groups
}

/// Every cluster value of the run, sorted and distinct.
fn cluster_bounds(run: &ShapedRun) -> Vec<usize> {
    let mut bounds: Vec<usize> = run.glyphs.iter().map(|g| g.cluster).collect();
    bounds.sort_unstable();
    bounds.dedup();
    bounds
}

/// The texts the glyphs of one cluster stand for (`None`: nothing of its own), and whether
/// the cluster needs `/ActualText` because they don't spell its text in reading order. A
/// single glyph takes the whole text; of several, a glyph that is the font's own glyph for
/// one of the cluster's characters takes it, and the characters left over go to the widest
/// other glyph (a conjunct, a reph, a contextual form). Default-ignorable characters are
/// never a glyph's text.
fn split_cluster(
    run: &ShapedRun,
    glyphs: &[ShapedGlyph],
    cluster: &str,
    nominal: &impl Fn(GlyphId) -> Option<char>,
) -> (Vec<Option<String>>, bool) {
    if let [_] = glyphs {
        let (text, needs_actual) = one_glyph_text(cluster);
        return (vec![(!text.is_empty()).then(|| text.into_owned())], needs_actual);
    }
    let visible = visible_text(cluster);
    let mut left: Vec<char> = visible.chars().collect();
    let mut texts: Vec<Option<String>> = glyphs
        .iter()
        .map(|g| {
            let ch = nominal(g.gid)?;
            let at = left.iter().position(|&c| c == ch)?;
            left.remove(at);
            Some(ch.to_string())
        })
        .collect();
    if !left.is_empty() {
        let taker = glyphs
            .iter()
            .zip(&texts)
            .enumerate()
            .filter(|(_, (_, t))| t.is_none())
            .max_by_key(|&(i, (g, _))| (g.x_advance, std::cmp::Reverse(i)))
            .map_or(0, |(i, _)| i);
        if let Some(t) = texts.get_mut(taker) {
            t.get_or_insert_with(String::new).extend(left);
        }
    }
    let reading = texts.iter().map(|t| t.as_deref().unwrap_or(""));
    let spelled = if run.direction.is_rtl() {
        spells(reading.rev(), cluster)
    } else {
        spells(reading, cluster)
    };
    (texts, !spelled)
}

/// What the one glyph of `cluster` stands for: its visible characters, and whether the cluster
/// needs `/ActualText` because they don't spell it (it holds default-ignorables; the visible
/// characters are a subsequence of it, so the lengths tell). [`split_cluster`]'s answer for one
/// glyph, and [`in_reading_order`]'s after it.
fn one_glyph_text(cluster: &str) -> (Cow<'_, str>, bool) {
    let visible = visible_text(cluster);
    let needs_actual = visible.len() != cluster.len();
    (visible, needs_actual)
}

/// `cluster` without its default-ignorable characters, borrowed when it has none.
fn visible_text(cluster: &str) -> Cow<'_, str> {
    if cluster.chars().any(is_default_ignorable) {
        Cow::Owned(cluster.chars().filter(|&ch| !is_default_ignorable(ch)).collect())
    } else {
        Cow::Borrowed(cluster)
    }
}

/// Whether `pieces`, concatenated, are `text` (without building the concatenation).
fn spells<'p>(pieces: impl Iterator<Item = &'p str>, text: &str) -> bool {
    let mut rest = text;
    for piece in pieces {
        match rest.strip_prefix(piece) {
            Some(r) => rest = r,
            None => return false,
        }
    }
    rest.is_empty()
}

/// The texts of one cluster's glyphs (drawing order) for a font whose glyphs get one CID per
/// text (B4d): the cluster's visible characters dealt out in reading order (drawing order,
/// reversed in a right-to-left run), each glyph taking as many as [`split_cluster`] gave it
/// and a glyph given none standing for nothing (`""`, a textless CID). Read in reading order
/// the glyphs then spell the cluster however the shaper reordered them (a pre-base matra, a
/// reph, marks in another order than the text), without `/ActualText`. Returns the texts and
/// whether the cluster still needs `/ActualText`: only when it holds default-ignorable
/// characters, which are no glyph's text.
fn in_reading_order(run: &ShapedRun, texts: &[Option<String>], cluster: &str) -> (Vec<String>, bool) {
    let mut sizes: Vec<usize> = texts
        .iter()
        .map(|t| t.as_deref().map_or(0, |t| t.chars().count()))
        .collect();
    if run.direction.is_rtl() {
        sizes.reverse();
    }
    let mut chars = cluster.chars().filter(|&ch| !is_default_ignorable(ch));
    let mut dealt: Vec<String> = sizes.iter().map(|&n| chars.by_ref().take(n).collect()).collect();
    // `split_cluster` gives out every visible character, so nothing is left; keep any rest
    // on the last glyph rather than lose it.
    if let Some(last) = dealt.last_mut() {
        last.extend(chars);
    }
    let reading_len: usize = dealt.iter().map(String::len).sum();
    if run.direction.is_rtl() {
        dealt.reverse();
    }
    (dealt, reading_len != cluster.len())
}

/// The `TJ` arrays: pieces split where `ActualText` starts or ends, segments where the rise
/// changes. Returns them and the run's advance in millionths of an em.
fn place(
    run: &ShapedRun,
    width: &impl Fn(GlyphId) -> i64,
    groups: &[Range<usize>],
    cids: &[u16],
    actual: &[Option<String>],
) -> (Vec<PdfPiece>, i64) {
    let to_milli = |units: i32| milli(f64::from(units), run.units_per_em);
    let mut pieces: Vec<PdfPiece> = Vec::new();
    let mut owed: i64 = 0; // how far the pen still has to move before the next glyph
    let mut total: i64 = 0;
    for (group, actual) in groups.iter().zip(actual) {
        let joins = actual.is_none() && pieces.last().is_some_and(|p| p.actual_text.is_none());
        if !joins {
            pieces.push(PdfPiece {
                actual_text: actual.clone(),
                segments: Vec::new(),
            });
        }
        let Some(piece) = pieces.last_mut() else {
            continue;
        };
        for i in group.clone() {
            let (Some(g), Some(cid)) = (run.glyphs.get(i), cids.get(i)) else {
                continue;
            };
            // The width `/W` is written with: hmtx rounded to 1/1000 of a `/W` unit.
            let width = width(g.gid);
            let dx = to_milli(g.x_offset);
            let advance = to_milli(g.x_advance);
            let rise = milli_to_f64(to_milli(g.y_offset));
            if piece.segments.last().is_none_or(|s| s.rise.to_bits() != rise.to_bits()) {
                piece.segments.push(PdfSegment { rise, tj: Vec::new() });
            }
            let Some(segment) = piece.segments.last_mut() else {
                continue;
            };
            let movement = owed + dx;
            if movement != 0 {
                segment.tj.push(TjElement::Adjust(-milli_to_f64(movement)));
            }
            match segment.tj.last_mut() {
                Some(TjElement::Codes(codes)) => codes.extend_from_slice(&cid.to_be_bytes()),
                _ => segment.tj.push(TjElement::Codes(cid.to_be_bytes().to_vec())),
            }
            owed = advance - dx - width;
            total += advance;
        }
    }
    if owed != 0 {
        let tail = pieces.last_mut().and_then(|p| p.segments.last_mut());
        if let Some(segment) = tail {
            segment.tj.push(TjElement::Adjust(-milli_to_f64(owed)));
        }
    }
    (pieces, total)
}

/// Whether `ch` is a default-ignorable code point (ZWJ, ZWNJ, the bidi controls, variation
/// selectors): invisible, never a glyph's text.
fn is_default_ignorable(ch: char) -> bool {
    // None is ASCII: skip the set lookup for them.
    !ch.is_ascii() && CodePointSetData::new::<DefaultIgnorableCodePoint>().contains(ch)
}

/// Thousandths of an em from millionths (exact: at most three decimals).
fn milli_to_f64(milli: i64) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "positions are far below 2^52")]
    let v = milli as f64;
    v / 1000.0
}
