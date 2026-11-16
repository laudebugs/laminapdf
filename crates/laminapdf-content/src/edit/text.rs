//! Rewriting the text a text object shows (ADR 0022 §5, §6; E7 plan §2.2): a range of its glyphs
//! swapped for others, re-encoded by the caller in the same font, and the rest of the line
//! moved by the width difference so nothing overlaps; the lines below stay where they were.

use std::ops::Range;

use laminapdf_syntax::{Object, PdfString, StringFormat};

use super::edits::op;
use super::{ContentObjectId, ContentOp, ContentOps, NotEditable};

/// One glyph a text object shows, as [`ContentOps::rewrite_text`] reads and writes it: its
/// character code and what moves the pen.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ShownGlyph {
    /// The character code, big-endian.
    pub code: u32,
    /// The code's length in bytes (1 to 4).
    pub len: u8,
    /// Its horizontal displacement `w0` in thousandths of text space (§9.4.4).
    pub width: f64,
    /// Whether word spacing (`Tw`) applies to it: the single-byte code 32 (§9.3.3).
    pub word_space: bool,
}

impl ShownGlyph {
    /// A glyph with code `code` of `len` bytes, `width` thousandths of text space wide.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(code: u32, len: u8, width: f64, word_space: bool) -> Self {
        Self {
            code,
            len,
            width,
            word_space,
        }
    }

    /// The code's bytes.
    fn bytes(self) -> Vec<u8> {
        let all = self.code.to_be_bytes();
        all.get(4usize.saturating_sub(usize::from(self.len))..)
            .unwrap_or_default()
            .to_vec()
    }
}

/// The text state that sizes glyphs (§9.3): `Tfs`, `Tc`, `Tw` and `Th` (as a fraction).
#[derive(Clone, Copy)]
struct Spacing {
    size: f64,
    character: f64,
    word: f64,
    scaling: f64,
}

impl Spacing {
    /// How far a glyph moves the pen, in unscaled text space (§9.4.4).
    fn advance(self, glyph: ShownGlyph) -> f64 {
        let word = if glyph.word_space { self.word } else { 0.0 };
        (glyph.width / 1000.0 * self.size + self.character + word) * self.scaling
    }

    /// How far a `TJ` number moves the pen, in unscaled text space.
    fn kern(self, number: f64) -> f64 {
        -number / 1000.0 * self.size * self.scaling
    }

    /// Follows an operator that changes the text state.
    fn update(&mut self, keyword: &[u8], operands: &[Object]) {
        let number = |i: usize| operands.get(i).and_then(Object::as_f64);
        match keyword {
            b"Tf" => self.size = number(1).unwrap_or(self.size),
            b"Tc" => self.character = number(0).unwrap_or(self.character),
            b"Tw" => self.word = number(0).unwrap_or(self.word),
            b"Tz" => self.scaling = number(0).map_or(self.scaling, |s| s / 100.0),
            b"\"" => {
                self.word = number(0).unwrap_or(self.word);
                self.character = number(1).unwrap_or(self.character);
            }
            _ => {}
        }
    }
}

/// One piece of a show operator: a glyph (its index among the object's glyphs) or a `TJ` number.
enum Piece {
    Glyph(usize),
    Kern(f64),
}

/// Whether the operator shows text.
fn shows(keyword: &[u8]) -> bool {
    matches!(keyword, b"Tj" | b"TJ" | b"'" | b"\"")
}

/// Splits `bytes` into the glyphs `glyphs[*next..]` stands for, checking each code.
fn split(bytes: &[u8], glyphs: &[ShownGlyph], next: &mut usize, out: &mut Vec<Piece>) -> Result<(), NotEditable> {
    let mut at = 0;
    while at < bytes.len() {
        let glyph = glyphs.get(*next).ok_or(NotEditable::ContentUnreadable)?;
        let len = usize::from(glyph.len);
        let code = bytes.get(at..at + len).ok_or(NotEditable::ContentUnreadable)?;
        if code != glyph.bytes().as_slice() {
            return Err(NotEditable::ContentUnreadable);
        }
        out.push(Piece::Glyph(*next));
        *next += 1;
        at += len;
    }
    Ok(())
}

/// The pieces of a show operator, in order.
fn pieces(op: &ContentOp, glyphs: &[ShownGlyph], next: &mut usize) -> Result<Vec<Piece>, NotEditable> {
    let mut out = Vec::new();
    match op.operands.last() {
        Some(Object::String(s)) => split(&s.bytes, glyphs, next, &mut out)?,
        Some(Object::Array(items)) if op.keyword[..] == *b"TJ" => {
            for item in items {
                match item {
                    Object::String(s) => split(&s.bytes, glyphs, next, &mut out)?,
                    other => out.push(Piece::Kern(other.as_f64().ok_or(NotEditable::ContentUnreadable)?)),
                }
            }
        }
        _ => return Err(NotEditable::ContentUnreadable),
    }
    Ok(out)
}

/// The string format of the operator's first string: kept for what it writes.
fn format_of(op: &ContentOp) -> StringFormat {
    let first = match op.operands.last() {
        Some(Object::String(s)) => Some(s.format),
        Some(Object::Array(items)) => items.iter().find_map(|i| match i {
            Object::String(s) => Some(s.format),
            _ => None,
        }),
        _ => None,
    };
    first.unwrap_or_default()
}

/// The operator `original` written again with `pieces`: a `TJ` stays a `TJ` (strings joined,
/// numbers kept); `Tj`, `'` and `"` keep their operator when no number is left, and become a
/// `TJ` otherwise (after the line move `'` and `"` make, written as `T*`).
fn rebuilt(original: &ContentOp, pieces: &[Piece], codes: &dyn Fn(usize) -> Vec<u8>) -> Vec<ContentOp> {
    let format = format_of(original);
    let string = |bytes: Vec<u8>| Object::String(PdfString { bytes, format });
    let mut items: Vec<Object> = Vec::new();
    let mut run: Vec<u8> = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Glyph(i) => run.extend(codes(*i)),
            Piece::Kern(n) => {
                if !run.is_empty() {
                    items.push(string(std::mem::take(&mut run)));
                }
                items.push(Object::number(*n));
            }
        }
    }
    let has_kern = items.iter().any(|i| !matches!(i, Object::String(_)));
    if !run.is_empty() || items.is_empty() {
        items.push(string(run));
    }
    let keyword = &original.keyword[..];
    if keyword == b"TJ" || has_kern {
        let mut out = Vec::new();
        match keyword {
            b"'" => out.push(op(b"T*", Vec::new())),
            b"\"" => {
                let spacing: Vec<Object> = original.operands.iter().take(2).cloned().collect();
                out.push(op(b"Tw", spacing.first().cloned().into_iter().collect()));
                out.push(op(b"Tc", spacing.get(1).cloned().into_iter().collect()));
                out.push(op(b"T*", Vec::new()));
            }
            _ => {}
        }
        out.push(op(b"TJ", vec![Object::Array(items)]));
        return out;
    }
    let mut operands: Vec<Object> = original.operands.clone();
    if let Some(last) = operands.last_mut() {
        *last = items.into_iter().next().unwrap_or_else(|| string(Vec::new()));
    }
    let mut written = original.clone();
    written.operands = operands;
    written.span = None;
    vec![written]
}

impl ContentOps {
    /// Swaps the text object's glyphs `range` for `new`, in the same font: `glyphs` is every
    /// glyph the object shows, in order, as the caller decoded them with its font (the codes
    /// must be the ones its show operators hold). `TJ` numbers inside the range go; the new
    /// glyphs go where the range started. The rest of the line moves by the width difference:
    /// glyphs later in the same show operator move by themselves, and the first `Td` or `TD`
    /// on the same line after the range takes the difference, which the next line's move takes
    /// back, so the lines below stay where they were. An empty `new` deletes the range.
    ///
    /// Returns the width difference, in unscaled text space (positive when the text grew).
    ///
    /// # Errors
    /// [`NotEditable::ClippingPath`] for text that clips (rendering modes 4–7);
    /// [`NotEditable::ContentUnreadable`] when the object isn't recorded, `range` is empty or
    /// outside `glyphs`, or the show operators' codes don't match `glyphs`.
    ///
    /// Added in 0.5.0.
    pub fn rewrite_text(
        &mut self,
        id: ContentObjectId,
        glyphs: &[ShownGlyph],
        range: Range<usize>,
        new: &[ShownGlyph],
    ) -> Result<f64, NotEditable> {
        if range.is_empty() || range.end > glyphs.len() {
            return Err(NotEditable::ContentUnreadable);
        }
        let ops = self.own_ops(id)?;
        let text = self.state_of(id)?.text;
        if text.render_mode >= 4 {
            return Err(NotEditable::ClippingPath);
        }
        let mut spacing = Spacing {
            size: text.font_size,
            character: text.char_spacing,
            word: text.word_spacing,
            scaling: text.horizontal_scaling,
        };
        let mut next = 0;
        let mut shift = 0.0;
        // After the range: whether the line's next move took the difference.
        let mut after: Option<bool> = None;
        let mut out: Vec<ContentOp> = Vec::new();
        for original in ops {
            let keyword = original.keyword.clone();
            spacing.update(&keyword, &original.operands);
            let read = if shows(&keyword) {
                Some(pieces(&original, glyphs, &mut next)?)
            } else {
                None
            };
            let touched = read.filter(|p| p.iter().any(|p| matches!(p, Piece::Glyph(i) if range.contains(i))));
            if let Some(pieces) = touched {
                let mut kept: Vec<Piece> = Vec::new();
                let mut inside = false;
                for piece in pieces {
                    match piece {
                        Piece::Glyph(i) if range.contains(&i) => {
                            if let Some(&g) = glyphs.get(i) {
                                shift -= spacing.advance(g);
                            }
                            if i == range.start {
                                shift += new.iter().map(|&g| spacing.advance(g)).sum::<f64>();
                                kept.extend((0..new.len()).map(|n| Piece::Glyph(glyphs.len() + n)));
                            }
                            inside = i + 1 < range.end;
                        }
                        Piece::Kern(n) if inside => shift -= spacing.kern(n),
                        other => kept.push(other),
                    }
                }
                let codes = |i: usize| -> Vec<u8> {
                    glyphs
                        .get(i)
                        .or_else(|| new.get(i - glyphs.len()))
                        .map(|g| g.bytes())
                        .unwrap_or_default()
                };
                out.extend(rebuilt(&original, &kept, &codes));
                if next >= range.end {
                    after = Some(false);
                }
                continue;
            }
            match (after, &keyword[..]) {
                (Some(moved), b"Td" | b"TD") if shift.abs() > f64::EPSILON => {
                    let ty = original.operands.get(1).and_then(Object::as_f64).unwrap_or(0.0);
                    let same_line = ty.abs() <= f64::EPSILON;
                    let delta = match (same_line, moved) {
                        (true, false) => Some(shift),
                        (false, true) => Some(-shift),
                        _ => None,
                    };
                    let mut written = original;
                    if let Some(delta) = delta {
                        let tx = written.operands.first().and_then(Object::as_f64).unwrap_or(0.0);
                        if let Some(first) = written.operands.first_mut() {
                            *first = Object::number(tx + delta);
                        }
                        written.span = None;
                    }
                    out.push(written);
                    after = same_line.then_some(true);
                }
                (Some(true), b"T*" | b"'" | b"\"") => {
                    out.push(op(b"Td", vec![Object::number(-shift), Object::number(0.0)]));
                    out.push(original);
                    after = None;
                }
                (Some(_), b"Tm" | b"T*" | b"'" | b"\"") => {
                    out.push(original);
                    after = None;
                }
                _ => out.push(original),
            }
        }
        if next < range.end {
            return Err(NotEditable::ContentUnreadable);
        }
        self.replace(id, out);
        Ok(shift)
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
