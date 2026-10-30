//! The `data` object of a diagnostic (ADR 0026 §1): each variant's fields under the keys its
//! `docs/errors/<code>.md` page lists. The only place document- or caller-derived strings
//! (filter, font and destination names, op-log text, sink messages) appear.
//!
//! The wrapped error enums are `#[non_exhaustive]`, so each match ends in a wildcard: a
//! variant added later gets no `data` until it is listed here.

use serde_json::{Map, Value, json};

use crate::Error;
use crate::warning::Warning;

type Data = Map<String, Value>;

fn data<const N: usize>(entries: [(&str, Value); N]) -> Data {
    entries.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// The `data` of `error`.
pub(crate) fn of_error(error: &Error) -> Data {
    match error {
        Error::Cos(e) => cos(e),
        Error::Ops(e) => ops(e),
        Error::Save(e) => save(e),
        #[cfg(feature = "text")]
        Error::Text(e) => text(e),
        #[cfg(feature = "text")]
        Error::Build(e) => build(e),
        Error::PageIndexOutOfRange { index, count } => data([("index", json!(index)), ("count", json!(count))]),
        Error::Cancelled | Error::JobFinished | Error::DocumentChanged => Data::new(),
    }
}

fn cos(e: &lamina_cos::Error) -> Data {
    use lamina_cos::Error as E;
    match e {
        E::Syntax(e) => syntax(e),
        E::LimitExceeded { limit, .. } => data([("limit", json!(limit.name()))]),
        E::Strict { warning } => data([("warning", json!(warning.code()))]),
        E::DataPending { ranges } => data([("ranges", json!(ranges))]),
        E::Type { expected, context } => data([("expected", json!(expected)), ("context", json!(context))]),
        E::InvalidStructure { kind, .. } => data([("structure", json!(kind))]),
        E::UnsupportedSecurityHandler {
            filter,
            version,
            revision,
        } => data([
            ("filter", json!(filter)),
            ("version", json!(version)),
            ("revision", json!(revision)),
        ]),
        _ => Data::new(),
    }
}

fn syntax(e: &lamina_syntax::Error) -> Data {
    use lamina_syntax::Error as E;
    match e {
        E::UnsupportedFilter(filter) => data([("filter", json!(filter))]),
        E::Type { expected, context } => data([("expected", json!(expected)), ("context", json!(context))]),
        E::InvalidStructure { kind, .. } => data([("structure", json!(kind))]),
        E::LimitExceeded { limit, .. } => data([("limit", json!(limit.name()))]),
        _ => Data::new(),
    }
}

fn ops(e: &lamina_ops::Error) -> Data {
    use lamina_ops::Error as E;
    match e {
        E::Cos(e) => cos(e),
        E::UnsupportedVersion(version) => data([("version", json!(version))]),
        E::InvalidLog(detail) => data([("detail", json!(detail))]),
        _ => Data::new(),
    }
}

fn save(e: &lamina_save::SaveError) -> Data {
    use lamina_save::{SaveError as E, SinkError};
    let names =
        |reasons: &[lamina_save::RewriteReason]| -> Vec<String> { reasons.iter().map(|r| format!("{r:?}")).collect() };
    match e {
        E::Cos(e) => cos(e),
        E::Ops(e) => ops(e),
        E::RequiresRewrite { reasons } => data([("reasons", json!(names(reasons)))]),
        E::WouldInvalidateSignatures {
            signatures,
            rewrite_reasons,
            signatures_unknown,
            ..
        } => data([
            ("signatures", json!(signatures.len())),
            ("rewrite_reasons", json!(names(rewrite_reasons))),
            ("signatures_unknown", json!(signatures_unknown)),
        ]),
        E::Sink(SinkError::Io { kind, message }) => {
            data([("kind", json!(format!("{kind:?}"))), ("message", json!(message))])
        }
        E::Sink(SinkError::Failed(message)) => data([("message", json!(message))]),
        E::Sink(SinkError::NotEmpty { held }) => data([("held", json!(held))]),
        E::NotRebased { error, .. } => {
            let inner = Error::from(error.clone());
            data([("error", json!({ "code": inner.code(), "kind": inner.kind().as_str() }))])
        }
        _ => Data::new(),
    }
}

#[cfg(feature = "text")]
fn text(e: &lamina_text::Error) -> Data {
    use lamina_text::Error as E;
    match e {
        E::InvalidFont { index } => data([("index", json!(index))]),
        E::CollectionIndex { index, count } => data([("index", json!(index)), ("count", json!(count))]),
        E::UnknownAxis { font, tag } => data([("font", json!(font)), ("tag", json!(tag.to_string()))]),
        E::InvalidAxisValue { tag } => data([("tag", json!(tag.to_string()))]),
        E::WrongDocument { object } => data([("object", json!(object))]),
        E::FontNotEmbeddable { font, fs_type } => data([("font", json!(font)), ("fs_type", json!(fs_type.bits()))]),
        E::MissingGlyph { ch, font } => data([("ch", json!(ch.to_string())), ("font", json!(font))]),
        E::GlyphOutOfRange { gid, count, font } => {
            data([("gid", json!(gid)), ("count", json!(count)), ("font", json!(font))])
        }
        E::UnknownFont { key } => data([("key", json!(key))]),
        E::Subset { font, reason } => data([("font", json!(font)), ("reason", json!(reason))]),
        E::InvalidFeature { feature } => data([("feature", json!(feature))]),
        E::InvalidTextRange { start, end, len } => {
            data([("start", json!(start)), ("end", json!(end)), ("len", json!(len))])
        }
        E::UnsupportedLayoutOption { option } => data([("option", json!(option))]),
        E::FontMismatch { font, shaped_with } => data([("font", json!(font)), ("shaped_with", json!(shaped_with))]),
        E::FontNotShapable { font } => data([("font", json!(font))]),
        E::EncodingFull { font, limit, free } => {
            data([("font", json!(font)), ("limit", json!(limit)), ("free", json!(free))])
        }
        E::Cos(e) => cos(e),
        _ => Data::new(),
    }
}

#[cfg(feature = "text")]
fn build(e: &lamina_build::Error) -> Data {
    use lamina_build::Error as E;
    match e {
        E::InvalidImage { format, reason } => {
            data([("format", json!(format!("{format:?}"))), ("reason", json!(reason))])
        }
        E::UnsupportedImage { format, feature } => {
            data([("format", json!(format!("{format:?}"))), ("feature", json!(feature))])
        }
        E::ImageTooLarge { limit } => data([("limit", json!(limit))]),
        E::ImageDataLength { expected, actual } | E::ColorComponents { expected, actual } => {
            data([("expected", json!(expected)), ("actual", json!(actual))])
        }
        E::InvalidIccProfile { reason } => data([("reason", json!(reason))]),
        E::InvalidColor { text } => data([("text", json!(text))]),
        E::InvalidGeometry { what } | E::InvalidPageLabels { what } => data([("what", json!(what))]),
        E::TextOverflow { fits, needed } => data([("fits", json!(fits)), ("needed", json!(needed))]),
        E::PageIndexOutOfRange { index, count } => data([("index", json!(index)), ("count", json!(count))]),
        E::UnknownDestination { name } => data([("name", json!(name))]),
        E::Text(e) => text(e),
        E::Cos(e) => cos(e),
        _ => Data::new(),
    }
}

/// The `data` of `warning`.
pub(crate) fn of_warning(warning: &Warning) -> Data {
    match warning {
        Warning::Document(w) => cos_warning(*w),
        #[cfg(feature = "text")]
        Warning::Font(w) => text_warning(w),
    }
}

fn cos_warning(w: lamina_cos::Warning) -> Data {
    use lamina_cos::Warning as W;
    match w {
        W::StaleGeneration { generation, .. } => data([("generation", json!(generation))]),
        W::PageCountMismatch { declared, walked, .. } => {
            data([("declared", json!(declared)), ("walked", json!(walked))])
        }
        W::InvalidBox { key, .. } | W::InvalidDate { key, .. } | W::InvalidEntry { key, .. } => {
            data([("key", json!(key))])
        }
        W::InvalidPageLabel { start } => data([("start", json!(start))]),
        W::ContentResourceMissing { offset, category, .. } => {
            data([("offset", json!(offset)), ("category", json!(category))])
        }
        W::XrefStmIgnored { offset }
        | W::ContentUnknownOperator { offset, .. }
        | W::ContentBadOperands { offset, .. }
        | W::ContentUnbalancedQ { offset, .. }
        | W::ContentUnbalancedBt { offset, .. }
        | W::ContentUnbalancedMarked { offset, .. }
        | W::ContentSyntax { offset, .. }
        | W::InlineImageUnterminated { offset, .. }
        | W::ContentPathUnpainted { offset, .. } => data([("offset", json!(offset))]),
        W::XObjectCycle { object } => data([("object", json!(object))]),
        _ => Data::new(),
    }
}

#[cfg(feature = "text")]
fn text_warning(w: &lamina_text::Warning) -> Data {
    use lamina_text::Warning as W;
    match w {
        W::ToUnicodeConflict {
            font,
            gid,
            kept,
            dropped,
        } => data([
            ("font", json!(font)),
            ("gid", json!(gid)),
            ("kept", json!(kept)),
            ("dropped", json!(dropped)),
        ]),
        W::WholeFontRebuilt { font } => data([("font", json!(font))]),
        _ => Data::new(),
    }
}
