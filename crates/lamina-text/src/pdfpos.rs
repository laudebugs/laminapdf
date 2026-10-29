//! A shaped run as PDF text operations (todo B4b; research 03 §2.6–2.7, §5.3, §7.2): the
//! glyphs' codes for `TJ`, the `TJ` numbers that turn the font's widths into the shaper's
//! advances and offsets, `Ts` rises for vertical offsets, the glyph → text entries of the
//! ToUnicode CMap, and the clusters that need `/ActualText`.
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

use std::ops::Range;

use skrifa::MetadataProvider;

use crate::embed::{FontKey, FontRegistry};
use crate::error::{Error, Result};
use crate::font::{Font, GlyphId};
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
///   the rest for nothing (`<cid> <>` in ToUnicode). When reading them in drawing order
///   (right to left in an RTL run) does not give the cluster's text (Indic reordering: a
///   pre-base matra drawn before its consonant), the cluster gets `/ActualText`.
///
/// Keeping each glyph's own character keeps the font's ToUnicode CMap consistent across
/// clusters (one code, one text: ISO 32000-2 §9.10.3); extractors that ignore `/ActualText`
/// then read a reordered cluster in drawing order. A cluster also gets `/ActualText` when a
/// glyph of it already stands for another text in this font
/// ([`crate::Warning::ToUnicodeConflict`]), and the whole run does when its clusters are not
/// in visual order for its direction.
///
/// # Errors
/// [`Error::MissingGlyph`] when the run has a `.notdef` glyph (the font lacks a character of
/// its cluster); [`Error::UnknownFont`]; [`Error::InvalidTextRange`] when the run's range is
/// not in `text`; [`Error::GlyphOutOfRange`].
pub fn pdf_run(run: &ShapedRun, text: &str, fonts: &mut FontRegistry, key: FontKey) -> Result<PdfRun> {
    let run_text = sub(text, run.text_range.clone())?;
    let font = fonts.try_font(key)?.clone();
    if let Some(g) = run.glyphs.iter().find(|g| g.gid == GlyphId::NOTDEF) {
        let ch = text.get(g.cluster..).and_then(|t| t.chars().next()).unwrap_or_default();
        return Err(Error::MissingGlyph {
            ch,
            font: font.info().postscript_name.clone(),
        });
    }
    let groups = cluster_groups(run);
    let clusters: Vec<usize> = groups
        .iter()
        .filter_map(|g| run.glyphs.get(g.start).map(|g| g.cluster))
        .collect();
    let in_order = if run.direction.is_rtl() {
        clusters.windows(2).all(|w| w.first() > w.get(1))
    } else {
        clusters.windows(2).all(|w| w.first() < w.get(1))
    };
    let (cids, mut actual) = map_text(run, text, &groups, fonts, key)?;
    if !in_order {
        // The whole run is wrapped instead.
        actual.fill(None);
    }
    let (mut pieces, advance) = place(run, &font, &groups, &cids, &actual);
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

/// Records every glyph with the text it stands for; returns the CIDs (per glyph) and the
/// `ActualText` of each group that needs one.
fn map_text(
    run: &ShapedRun,
    text: &str,
    groups: &[Range<usize>],
    fonts: &mut FontRegistry,
    key: FontKey,
) -> Result<(Vec<u16>, Vec<Option<String>>)> {
    let font = fonts.try_font(key)?.clone();
    let charmap = font.font_ref()?.charmap();
    let nominal = |gid: GlyphId, cluster: &str| {
        cluster
            .chars()
            .find(|&ch| charmap.map(ch).is_some_and(|g| g.to_u32() == u32::from(gid.get())))
    };
    // Cluster boundaries: every cluster value, and the run's end.
    let mut bounds: Vec<usize> = run.glyphs.iter().map(|g| g.cluster).collect();
    bounds.push(run.text_range.end);
    bounds.sort_unstable();
    bounds.dedup();
    let cluster_text = |cluster: usize| -> &str {
        let end = bounds
            .iter()
            .copied()
            .find(|&b| b > cluster)
            .unwrap_or(run.text_range.end);
        text.get(cluster..end).unwrap_or_default()
    };

    let mut cids = Vec::with_capacity(run.glyphs.len());
    let mut actual_texts = Vec::with_capacity(groups.len());
    for group in groups {
        let glyphs = run.glyphs.get(group.clone()).unwrap_or_default();
        let cluster = glyphs.first().map(|g| cluster_text(g.cluster)).unwrap_or_default();
        let mut actual = false;
        let texts: Vec<String> = if let [_] = glyphs {
            vec![cluster.to_owned()]
        } else {
            let (texts, in_text_order) = split_cluster(run, glyphs, cluster, &nominal);
            actual = !in_text_order;
            texts
        };
        for (g, t) in glyphs.iter().zip(&texts) {
            let cid = fonts.use_glyph(key, g.gid, t)?;
            let kept = fonts.usage(key).and_then(|u| u.text().get(&cid));
            if !t.is_empty() && kept.is_some_and(|k| k != t) {
                actual = true;
            }
            cids.push(cid);
        }
        actual_texts.push(actual.then(|| cluster.to_owned()));
    }
    Ok((cids, actual_texts))
}

/// The texts the glyphs of one multi-glyph cluster stand for: a glyph that is the font's own
/// glyph for one of the cluster's characters stands for it; the characters left over go to the
/// widest other glyph (a conjunct, a reph, a contextual form), and any further glyphs stand for
/// nothing. Also returns whether reading the texts in drawing order (reversed in a
/// right-to-left run) gives the cluster's text: if not, the cluster needs `/ActualText`.
fn split_cluster(
    run: &ShapedRun,
    glyphs: &[ShapedGlyph],
    cluster: &str,
    nominal: &impl Fn(GlyphId, &str) -> Option<char>,
) -> (Vec<String>, bool) {
    let mut left: Vec<char> = cluster.chars().collect();
    let mut texts: Vec<String> = glyphs
        .iter()
        .map(|g| {
            let claimed = nominal(g.gid, cluster).and_then(|ch| {
                let at = left.iter().position(|&c| c == ch)?;
                left.remove(at);
                Some(ch)
            });
            claimed.map(String::from).unwrap_or_default()
        })
        .collect();
    if !left.is_empty() {
        // The widest glyph without text of its own (else the first glyph) takes the rest.
        let taker = glyphs
            .iter()
            .zip(&texts)
            .enumerate()
            .filter(|(_, (_, t))| t.is_empty())
            .max_by_key(|&(i, (g, _))| (g.x_advance, std::cmp::Reverse(i)))
            .map_or(0, |(i, _)| i);
        if let Some(t) = texts.get_mut(taker) {
            t.extend(left);
        }
    }
    let mut reading: Vec<&str> = texts.iter().map(String::as_str).collect();
    if run.direction.is_rtl() {
        reading.reverse();
    }
    let in_text_order = reading.concat() == cluster;
    (texts, in_text_order)
}

/// The `TJ` arrays: pieces split where `ActualText` starts or ends, segments where the rise
/// changes. Returns them and the run's advance in millionths of an em.
fn place(
    run: &ShapedRun,
    font: &Font,
    groups: &[Range<usize>],
    cids: &[u16],
    actual: &[Option<String>],
) -> (Vec<PdfPiece>, i64) {
    let upem = f64::from(font.info().units_per_em.max(1));
    let to_milli = |units: f64| round_i64(units * 1_000_000.0 / upem);
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
            let width = to_milli(f64::from(font.advance(g.gid).unwrap_or(0.0)));
            let dx = to_milli(f64::from(g.x_offset));
            let advance = to_milli(f64::from(g.x_advance));
            let rise = milli_to_f64(to_milli(f64::from(g.y_offset)));
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

/// Thousandths of an em from millionths (exact: at most three decimals).
fn milli_to_f64(milli: i64) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "positions are far below 2^52")]
    let v = milli as f64;
    v / 1000.0
}

/// `v` rounded to the nearest integer, 0 when not finite: the rounding `/W` is written with.
fn round_i64(v: f64) -> i64 {
    if !v.is_finite() {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "font units × 10^6 / upem are far inside i64"
    )]
    let r = v.round() as i64;
    r
}
