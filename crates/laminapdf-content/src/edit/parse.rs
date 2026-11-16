//! Reading a content owner into operators with spans, grouped into objects (ADR 0022 §1).

use laminapdf_cos::{Document, Error, Limit, Limits, Page, Result};
use laminapdf_syntax::{Lexer, ObjRef, Object};

use super::{ContentObject, ContentObjectId, ContentObjectKind, ContentOp, MarkedSequence};
use crate::lexer::{ContentIter, Lexed};
use crate::operator::Operator;

/// One decoded `/Contents` element: its index in the array, its object and its bytes.
pub(super) type RawPart = (usize, Option<ObjRef>, Vec<u8>);

/// What a content owner parses into.
pub(super) struct Parsed {
    pub ops: Vec<ContentOp>,
    pub objects: Vec<ContentObject>,
    pub marked: Vec<MarkedSequence>,
}

/// Every `/Contents` element of `page` decoded, as [`Document::page_content`] reads them: a
/// `null` or free element contributes no part.
pub(super) fn read_parts(doc: &Document, page: &Page, limits: &Limits) -> Result<Vec<RawPart>> {
    let Some(contents) = page.dict.get("Contents") else {
        return Ok(Vec::new());
    };
    let resolved = doc.deref(contents)?;
    let elements = match &*resolved {
        Object::Array(items) => items.as_slice(),
        single => std::slice::from_ref(single),
    };
    let mut parts = Vec::with_capacity(elements.len());
    let mut total = 0usize;
    for (index, element) in elements.iter().enumerate() {
        let object = match element {
            Object::Ref(r) => Some(*r),
            _ if elements.len() == 1 => contents.as_ref(),
            _ => None,
        };
        let stream = doc.deref(element)?;
        let bytes = match &*stream {
            Object::Stream(s) => doc.decode_stream(s)?,
            Object::Null => continue,
            _ => {
                return Err(Error::Type {
                    expected: "stream",
                    context: "/Contents",
                });
            }
        };
        total = total.saturating_add(bytes.len());
        if total > limits.max_decoded_total() {
            return Err(Error::LimitExceeded {
                limit: Limit::DecodedTotal,
                at: None,
            });
        }
        parts.push((index, object, bytes));
    }
    Ok(parts)
}

/// Tokenizes `source` into operators with spans and groups them.
pub(super) fn parse(source: &[u8], limits: &Limits) -> Result<Parsed> {
    let mut ops = Vec::new();
    let mut previous_end = 0;
    for item in ContentIter::new(source).with_limits(limits) {
        let Lexed::Op(op) = item? else {
            continue;
        };
        if u64::try_from(ops.len()).unwrap_or(u64::MAX) >= limits.max_content_operators() {
            return Err(Error::LimitExceeded {
                limit: Limit::ContentOperators,
                at: None,
            });
        }
        let keyword = op.operator.keyword();
        let (end, inline_image) = match &op.inline_image {
            Some(image) => (image.end, source.get(op.offset..image.end).map(Box::from)),
            None => (op.offset + keyword.len(), None),
        };
        let start = previous_end.min(op.offset);
        ops.push(ContentOp {
            keyword: keyword.into(),
            operands: op.operands,
            inline_image,
            span: Some(start..end),
            token_start: token_start(source, start, op.offset),
        });
        previous_end = end;
    }
    let (objects, marked) = group(&ops);
    Ok(Parsed { ops, objects, marked })
}

/// Where the first token of the span from `start` to the keyword at `keyword` begins: past the
/// white space and comments before it, as the lexer skips them.
fn token_start(source: &[u8], start: usize, keyword: usize) -> usize {
    let mut lexer = Lexer::new(source, start);
    lexer.skip_whitespace();
    lexer.pos().min(keyword)
}

const PATH: &[Operator<'static>] = &[
    Operator::MoveTo,
    Operator::LineTo,
    Operator::CurveTo,
    Operator::CurveToV,
    Operator::CurveToY,
    Operator::ClosePath,
    Operator::Rectangle,
];

const CLIP: &[Operator<'static>] = &[Operator::Clip, Operator::ClipEvenOdd];

const PAINT: &[Operator<'static>] = &[
    Operator::Stroke,
    Operator::CloseStroke,
    Operator::Fill,
    Operator::FillObsolete,
    Operator::FillEvenOdd,
    Operator::FillStroke,
    Operator::FillStrokeEvenOdd,
    Operator::CloseFillStroke,
    Operator::CloseFillStrokeEvenOdd,
    Operator::EndPath,
];

/// Groups operators into objects, and finds the marked-content sequences around them.
fn group(ops: &[ContentOp]) -> (Vec<ContentObject>, Vec<MarkedSequence>) {
    let mut objects = Vec::new();
    let mut marked: Vec<MarkedSequence> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut i = 0;
    while let Some(op) = ops.get(i) {
        let operator = op.operator();
        let (kind, last) = match operator {
            Operator::BeginText => (ContentObjectKind::Text, text_end(ops, i)),
            o if PATH.contains(&o) => (ContentObjectKind::Path, path_end(ops, i)),
            Operator::BeginInlineImage => (ContentObjectKind::Image, i),
            Operator::PaintXObject => (ContentObjectKind::XObject, i),
            Operator::Shading => (ContentObjectKind::Shading, i),
            _ => {
                track_marked(op, i, &mut open, &mut marked);
                i += 1;
                continue;
            }
        };
        // Marked content opened and closed inside a text object belongs to the object, not
        // around it: the stack is read again after the object.
        let id = ContentObjectId(u32::try_from(objects.len()).unwrap_or(u32::MAX));
        objects.push(ContentObject {
            id,
            kind,
            ops: i..last + 1,
            state: None,
            end_state: None,
            bounds: None,
            marked: open.clone(),
        });
        for (offset, inner) in ops.get(i..=last).unwrap_or_default().iter().enumerate() {
            track_marked(inner, i + offset, &mut open, &mut marked);
        }
        i = last + 1;
    }
    (objects, marked)
}

/// Opens a marked-content sequence at a `BMC`/`BDC` (the operator at `index`) or closes the
/// innermost open one at an `EMC`; any other operator changes nothing.
fn track_marked(op: &ContentOp, index: usize, open: &mut Vec<usize>, marked: &mut Vec<MarkedSequence>) {
    match op.operator() {
        Operator::BeginMarked | Operator::BeginMarkedProperties => {
            open.push(marked.len());
            marked.push(MarkedSequence {
                tag: tag(op),
                mcid: mcid(op),
                begin: index,
                end: None,
            });
        }
        Operator::EndMarked => {
            if let Some(seq) = open.pop().and_then(|m| marked.get_mut(m)) {
                seq.end = Some(index);
            }
        }
        _ => {}
    }
}

/// The `ET` that ends the text object begun at `begin`, else the last operator.
fn text_end(ops: &[ContentOp], begin: usize) -> usize {
    ops.iter()
        .enumerate()
        .skip(begin + 1)
        .find(|(_, op)| op.operator() == Operator::EndText)
        .map_or(ops.len().saturating_sub(1), |(i, _)| i)
}

/// The painting operator that ends the path begun at `begin`, through its clipping operators;
/// a path that another operator interrupts ends at its last segment or clip.
fn path_end(ops: &[ContentOp], begin: usize) -> usize {
    let mut last = begin;
    for (i, op) in ops.iter().enumerate().skip(begin + 1) {
        let operator = op.operator();
        if PATH.contains(&operator) || CLIP.contains(&operator) {
            last = i;
        } else if PAINT.contains(&operator) {
            return i;
        } else {
            break;
        }
    }
    last
}

/// A marked-content operator's tag (its first operand).
fn tag(op: &ContentOp) -> laminapdf_syntax::Name {
    match op.operands.first() {
        Some(Object::Name(name)) => name.clone(),
        _ => laminapdf_syntax::Name::from(""),
    }
}

/// `/MCID` from a `BDC`'s inline property list.
fn mcid(op: &ContentOp) -> Option<i64> {
    match op.operands.get(1) {
        Some(Object::Dict(props)) => props.get("MCID").and_then(Object::as_int),
        _ => None,
    }
}
