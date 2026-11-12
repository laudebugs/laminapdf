//! Reading a document and refusing what isn't one, with the JSON Pointer of the value at fault.
//!
//! serde reports where a value is wrong, but it buffers a block tagged by `type` before reading
//! it, so its path stops at the block. When reading fails inside `blocks`, each block is read
//! again on its own, kind by kind, to find the place. Numbers serde accepts are then checked
//! against what the layout can use: a size above zero, a heading level from 1 to 6.

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{
    BlockSpec, CellSpec, CodeBlockSpec, ColumnSpec, ContentsSpec, DOCUMENT_SCHEMA, FigureSpec, FlowDocument, GroupSpec,
    ListItemSpec, ListSpec, MarginsSpec, PageSizeSpec, PageSpec, ParagraphSpec, RowSpec, RuleSpec, SpacerSpec,
    SpanSpec, StyleSpec, TableSpec, escape,
};
use crate::error::{Error, Result};

/// `invalid-document` at `pointer` for `reason`, with no suggestion.
pub(crate) fn refuse(pointer: impl Into<String>, reason: &'static str) -> Error {
    Error::InvalidDocument {
        pointer: pointer.into(),
        reason,
        did_you_mean: Vec::new(),
    }
}

impl FlowDocument {
    /// Reads `json` as a document.
    ///
    /// # Errors
    /// `invalid-document`, with the JSON Pointer of the value at fault and the `reason` in its
    /// data: an unknown key (with the closest keys the format knows in `didYouMean`), a missing
    /// key, a value of the wrong kind, a number out of range, a heading without a level from 1
    /// to 6, or a `schema` other than `laminapdf.document/1`.
    ///
    /// Added in 0.2.0.
    pub fn from_json(json: &Value) -> Result<Self> {
        if json
            .get("schema")
            .and_then(Value::as_str)
            .is_some_and(|s| s != DOCUMENT_SCHEMA)
        {
            return Err(Error::InvalidDocument {
                pointer: "/schema".to_owned(),
                reason: "unsupported-schema",
                did_you_mean: vec![DOCUMENT_SCHEMA.to_owned()],
            });
        }
        let document: Self = match serde_path_to_error::deserialize(json) {
            Ok(document) => document,
            Err(error) => {
                let at_blocks = matches!(error.path().iter().next(), Some(serde_path_to_error::Segment::Map { key }) if key == "blocks");
                if at_blocks && let Some(blocks) = json.get("blocks") {
                    check_blocks(blocks, "/blocks")?;
                }
                return Err(from_serde(&error, ""));
            }
        };
        check(&document)?;
        Ok(document)
    }
}

/// `value` read as `T`, its errors pointed from `prefix`.
fn read<T: DeserializeOwned>(value: &Value, prefix: &str) -> Result<T> {
    serde_path_to_error::deserialize(value).map_err(|e| from_serde(&e, prefix))
}

/// The kinds of block, as `type` names them.
const KINDS: [&str; 12] = [
    "paragraph",
    "heading",
    "list",
    "figure",
    "spacer",
    "pageBreak",
    "keep",
    "table",
    "contents",
    "quote",
    "codeBlock",
    "rule",
];

/// Reads each of `blocks` on its own, to point at the first fault inside one.
fn check_blocks(blocks: &Value, pointer: &str) -> Result<()> {
    let Value::Array(blocks) = blocks else {
        return Err(refuse(pointer, "wrong-type"));
    };
    for (i, block) in blocks.iter().enumerate() {
        check_block(block, &format!("{pointer}/{i}"))?;
    }
    Ok(())
}

fn check_block(block: &Value, pointer: &str) -> Result<()> {
    let Value::Object(map) = block else {
        return Err(refuse(pointer, "wrong-type"));
    };
    let Some(kind) = map.get("type") else {
        return Err(refuse(format!("{pointer}/type"), "missing-key"));
    };
    let Some(kind) = kind.as_str().filter(|k| KINDS.contains(k)) else {
        let known: Vec<String> = KINDS.iter().map(|k| (*k).to_owned()).collect();
        return Err(Error::InvalidDocument {
            pointer: format!("{pointer}/type"),
            reason: "wrong-type",
            did_you_mean: kind
                .as_str()
                .map(|k| laminapdf_syntax::did_you_mean(k, &known))
                .unwrap_or_default(),
        });
    };
    let mut rest = map.clone();
    rest.remove("type");
    let rest = Value::Object(rest);
    match kind {
        "paragraph" | "heading" => read::<ParagraphSpec>(&rest, pointer).map(drop),
        "list" => read::<ListSpec>(&rest, pointer).map(drop),
        "figure" => read::<FigureSpec>(&rest, pointer).map(drop),
        "spacer" => read::<SpacerSpec>(&rest, pointer).map(drop),
        "table" => read::<TableSpec>(&rest, pointer).map(drop),
        "contents" => read::<ContentsSpec>(&rest, pointer).map(drop),
        "codeBlock" => read::<CodeBlockSpec>(&rest, pointer).map(drop),
        // Groups hold blocks: read their own key, then each block on its own.
        "keep" | "quote" => {
            if let Some(key) = rest.as_object().and_then(|m| m.keys().find(|k| *k != "blocks")) {
                return Err(unknown_key(pointer, key, &["blocks"]));
            }
            let blocks = rest
                .get("blocks")
                .ok_or_else(|| refuse(format!("{pointer}/blocks"), "missing-key"))?;
            check_blocks(blocks, &format!("{pointer}/blocks"))
        }
        // `pageBreak` and `rule` take nothing but their `type`.
        _ => match rest.as_object().and_then(|m| m.keys().next()) {
            Some(key) => Err(unknown_key(pointer, key, &[])),
            None => Ok(()),
        },
    }
}

/// `invalid-document` for the unknown `key` in the object at `pointer`, suggesting from `known`.
fn unknown_key(pointer: &str, key: &str, known: &[&str]) -> Error {
    let known: Vec<String> = known.iter().map(|k| (*k).to_owned()).collect();
    Error::InvalidDocument {
        pointer: format!("{pointer}/{}", escape(key)),
        reason: "unknown-key",
        did_you_mean: laminapdf_syntax::did_you_mean(key, &known),
    }
}

/// The names serde lists after the last `, expected ` of `message`: the format's own keys or
/// kinds, never the document's text, which can only come before it.
fn expected(message: &str) -> Vec<String> {
    let Some((_, tail)) = message.rsplit_once(", expected ") else {
        return Vec::new();
    };
    tail.split('`').skip(1).step_by(2).map(str::to_owned).collect()
}

/// The text between `` `{prefix}` `` and the last `` `, expected ``: the key or kind the
/// document gave, which only the pointer (in `data`) carries.
fn given<'a>(message: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = message.strip_prefix(prefix)?.strip_prefix('`')?;
    let end = rest
        .rfind("`, expected ")
        .or_else(|| rest.rfind("`, there are no fields"))?;
    rest.get(..end)
}

/// `invalid-document` from serde's error, its path appended to `prefix`.
fn from_serde(error: &serde_path_to_error::Error<serde_json::Error>, prefix: &str) -> Error {
    use serde_path_to_error::Segment;
    let message = error.inner().to_string();
    let mut pointer = prefix.to_owned();
    for segment in error.path() {
        let part = match segment {
            Segment::Seq { index } => index.to_string(),
            Segment::Map { key } | Segment::Enum { variant: key } => escape(key),
            Segment::Unknown => continue,
        };
        pointer.push('/');
        pointer.push_str(&part);
    }
    if let Some(key) = given(&message, "unknown field ") {
        // serde's path ends at the key when it read the object itself, before it when it
        // buffered the object first.
        let segment = format!("/{}", escape(key));
        if !pointer.ends_with(&segment) {
            pointer.push_str(&segment);
        }
        return Error::InvalidDocument {
            pointer,
            reason: "unknown-key",
            did_you_mean: laminapdf_syntax::did_you_mean(key, &expected(&message)),
        };
    }
    if let Some(kind) = given(&message, "unknown variant ") {
        return Error::InvalidDocument {
            pointer,
            reason: "wrong-type",
            did_you_mean: laminapdf_syntax::did_you_mean(kind, &expected(&message)),
        };
    }
    if let Some(rest) = message.strip_prefix("missing field `")
        && let Some((name, _)) = rest.rsplit_once('`')
    {
        pointer.push('/');
        pointer.push_str(&escape(name));
        return refuse(pointer, "missing-key");
    }
    refuse(pointer, "wrong-type")
}

/// `out-of-range` unless `value` is finite and at least `min` (above it when `above`).
fn range(value: f64, min: f64, above: bool, pointer: impl FnOnce() -> String) -> Result<()> {
    let ok = value.is_finite() && if above { value > min } else { value >= min };
    if ok {
        Ok(())
    } else {
        Err(refuse(pointer(), "out-of-range"))
    }
}

fn positive(value: Option<f64>, pointer: impl FnOnce() -> String) -> Result<()> {
    value.map_or(Ok(()), |v| range(v, 0.0, true, pointer))
}

fn non_negative(value: Option<f64>, pointer: impl FnOnce() -> String) -> Result<()> {
    value.map_or(Ok(()), |v| range(v, 0.0, false, pointer))
}

/// What serde accepts but the layout can't use: sizes, levels and counts out of range.
fn check(document: &FlowDocument) -> Result<()> {
    for (name, style) in &document.styles {
        check_style(style, &format!("/styles/{}", escape(name)))?;
    }
    if let Some(page) = &document.page {
        check_page(page, "/page")?;
    }
    if let Some(level) = document.section_level
        && !(1..=6).contains(&level)
    {
        return Err(refuse("/sectionLevel", "out-of-range"));
    }
    check_block_list(&document.blocks, "/blocks")
}

fn check_style(style: &StyleSpec, pointer: &str) -> Result<()> {
    positive(Some(style.size), || format!("{pointer}/size"))?;
    style
        .rise
        .map_or(Ok(()), |r| range(r, f64::MIN, false, || format!("{pointer}/rise")))
}

fn check_page(page: &PageSpec, pointer: &str) -> Result<()> {
    if let PageSizeSpec::Points(size) = &page.size {
        for (i, side) in size.iter().enumerate() {
            positive(Some(*side), || format!("{pointer}/size/{i}"))?;
        }
    }
    match page.margins {
        Some(MarginsSpec::All(m)) => non_negative(Some(m), || format!("{pointer}/margins"))?,
        Some(MarginsSpec::Sides(sides)) => {
            for (i, side) in sides.iter().enumerate() {
                non_negative(Some(*side), || format!("{pointer}/margins/{i}"))?;
            }
        }
        None => {}
    }
    match &page.first {
        // The first page's settings are one level deep: a first page has no first page.
        Some(first) if first.first.is_some() => Err(refuse(format!("{pointer}/first/first"), "unknown-key")),
        Some(first) => check_page(first, &format!("{pointer}/first")),
        None => Ok(()),
    }
}

fn check_block_list(blocks: &[BlockSpec], pointer: &str) -> Result<()> {
    for (i, block) in blocks.iter().enumerate() {
        check_typed_block(block, &format!("{pointer}/{i}"))?;
    }
    Ok(())
}

fn check_typed_block(block: &BlockSpec, pointer: &str) -> Result<()> {
    match block {
        BlockSpec::Paragraph(p) if p.level.is_some() => Err(Error::InvalidDocument {
            pointer: format!("{pointer}/level"),
            reason: "unknown-key",
            did_you_mean: Vec::new(),
        }),
        BlockSpec::Paragraph(p) => check_paragraph(p, pointer),
        BlockSpec::Heading(p) => match p.level {
            Some(1..=6) => check_paragraph(p, pointer),
            Some(_) => Err(refuse(format!("{pointer}/level"), "out-of-range")),
            None => Err(refuse(format!("{pointer}/level"), "missing-key")),
        },
        BlockSpec::List(list) => check_list(list, pointer),
        BlockSpec::Figure(figure) => positive(figure.width, || format!("{pointer}/width")),
        BlockSpec::Spacer(spacer) => non_negative(Some(spacer.height), || format!("{pointer}/height")),
        BlockSpec::Keep(GroupSpec { blocks }) | BlockSpec::Quote(GroupSpec { blocks }) => {
            check_block_list(blocks, &format!("{pointer}/blocks"))
        }
        BlockSpec::Table(table) => check_table(table, pointer),
        BlockSpec::Contents(contents) => {
            if contents.levels.is_some_and(|l| !(1..=6).contains(&l)) {
                return Err(refuse(format!("{pointer}/levels"), "out-of-range"));
            }
            non_negative(contents.indent, || format!("{pointer}/indent"))
        }
        BlockSpec::PageBreak | BlockSpec::CodeBlock(_) | BlockSpec::Rule => Ok(()),
    }
}

fn check_paragraph(p: &ParagraphSpec, pointer: &str) -> Result<()> {
    positive(p.line_height, || format!("{pointer}/lineHeight"))?;
    non_negative(p.space_before, || format!("{pointer}/spaceBefore"))?;
    non_negative(p.space_after, || format!("{pointer}/spaceAfter"))?;
    for (key, count) in [("widows", p.widows), ("orphans", p.orphans)] {
        if count == Some(0) {
            return Err(refuse(format!("{pointer}/{key}"), "out-of-range"));
        }
    }
    check_spans(p.spans.as_deref(), pointer)
}

fn check_spans(spans: Option<&[SpanSpec]>, pointer: &str) -> Result<()> {
    for (i, span) in spans.into_iter().flatten().enumerate() {
        if let Some(note) = &span.note {
            check_paragraph(note, &format!("{pointer}/spans/{i}/note"))?;
        }
    }
    Ok(())
}

fn check_list(list: &ListSpec, pointer: &str) -> Result<()> {
    non_negative(list.indent, || format!("{pointer}/indent"))?;
    for (i, item) in list.items.iter().enumerate() {
        let at = format!("{pointer}/items/{i}");
        let ListItemSpec {
            spans, list: nested, ..
        } = item;
        check_spans(spans.as_deref(), &at)?;
        if let Some(nested) = nested {
            check_list(nested, &format!("{at}/list"))?;
        }
    }
    Ok(())
}

fn check_rule(rule: Option<&RuleSpec>, pointer: impl FnOnce() -> String) -> Result<()> {
    rule.map_or(Ok(()), |r| {
        non_negative(Some(r.width), || format!("{}/width", pointer()))
    })
}

fn check_table(table: &TableSpec, pointer: &str) -> Result<()> {
    for (i, column) in table.columns.iter().enumerate() {
        match *column {
            ColumnSpec::Fixed(w) | ColumnSpec::Fraction(w) => positive(Some(w), || format!("{pointer}/columns/{i}"))?,
            ColumnSpec::Auto => {}
        }
    }
    non_negative(table.padding, || format!("{pointer}/padding"))?;
    non_negative(table.space_before, || format!("{pointer}/spaceBefore"))?;
    non_negative(table.space_after, || format!("{pointer}/spaceAfter"))?;
    check_rule(table.frame.as_ref(), || format!("{pointer}/frame"))?;
    check_rule(table.header_rule.as_ref(), || format!("{pointer}/headerRule"))?;
    check_rule(table.row_rules.as_ref(), || format!("{pointer}/rowRules"))?;
    for (section, rows) in [
        ("header", &table.header),
        ("rows", &table.rows),
        ("footer", &table.footer),
    ] {
        for (r, RowSpec { cells }) in rows.iter().enumerate() {
            for (c, cell) in cells.iter().enumerate() {
                let at = format!("{pointer}/{section}/{r}/cells/{c}");
                let CellSpec { span, spans, .. } = cell;
                if *span == Some(0) {
                    return Err(refuse(format!("{at}/span"), "out-of-range"));
                }
                check_spans(spans.as_deref(), &at)?;
            }
        }
    }
    Ok(())
}
