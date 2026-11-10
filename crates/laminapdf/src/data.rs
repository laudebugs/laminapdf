//! The `data` object of a diagnostic (ADR 0026 §1): each variant's fields under the keys its
//! `docs/errors/<code>.md` page lists. The only place document- or caller-derived strings
//! (filter, font and destination names, op-log text, sink messages) appear.
//!
//! The wrapped error enums are `#[non_exhaustive]`, so each match ends in a wildcard: a
//! variant added later gets no `data` until it is listed here.

use serde_json::{Map, Value, json};

use crate::Error;
use crate::error::{CoreError, Inner};
use crate::warning::Warning;

type Data = Map<String, Value>;

fn data<const N: usize>(entries: [(&str, Value); N]) -> Data {
    entries.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// The `data` of `error`.
pub(crate) fn of_error(error: &Error) -> Data {
    match error {
        Error::Core(CoreError(inner)) => match inner {
            Inner::Cos(e) => cos(e),
            Inner::Ops(e) => ops(e),
            Inner::Save(e) => save(e),
            #[cfg(feature = "extract")]
            Inner::Text(e) => text(e),
            #[cfg(feature = "build")]
            Inner::Build(e) => build(e),
            #[cfg(feature = "extract")]
            Inner::Extract(_) => Data::new(),
        },
        Error::PageIndexOutOfRange { index, count } | Error::RevisionIndexOutOfRange { index, count } => {
            data([("index", json!(index)), ("count", json!(count))])
        }
        Error::Cancelled | Error::JobFinished | Error::DocumentChanged => Data::new(),
    }
}

fn cos(e: &laminapdf_cos::Error) -> Data {
    use laminapdf_cos::Error as E;
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

fn syntax(e: &laminapdf_syntax::Error) -> Data {
    use laminapdf_syntax::Error as E;
    match e {
        E::UnsupportedFilter(filter) => data([("filter", json!(filter))]),
        E::Type { expected, context } => data([("expected", json!(expected)), ("context", json!(context))]),
        E::InvalidStructure { kind, .. } => data([("structure", json!(kind))]),
        E::LimitExceeded { limit, .. } => data([("limit", json!(limit.name()))]),
        _ => Data::new(),
    }
}

fn ops(e: &laminapdf_ops::Error) -> Data {
    use laminapdf_ops::Error as E;
    match e {
        E::Cos(e) => cos(e),
        E::UnsupportedVersion(version) => data([("version", json!(version))]),
        E::InvalidLog(detail) => data([("detail", json!(detail))]),
        _ => Data::new(),
    }
}

fn save(e: &laminapdf_save::SaveError) -> Data {
    use laminapdf_save::{SaveError as E, SinkError};
    let names = |reasons: &[laminapdf_save::RewriteReason]| -> Vec<String> {
        reasons.iter().map(|r| format!("{r:?}")).collect()
    };
    match e {
        E::Cos(e) => cos(e),
        E::Ops(e) => ops(e),
        E::RequiresRewrite { reasons } => data([("reasons", json!(names(reasons)))]),
        E::WouldInvalidateSignatures {
            signatures,
            rewrite_reasons,
            violations,
            signatures_unknown,
        } => data([
            ("signatures", Value::Array(signatures.iter().map(signature).collect())),
            ("rewrite_reasons", json!(names(rewrite_reasons))),
            (
                "violations",
                Value::Array(
                    violations
                        .iter()
                        .map(|v| {
                            json!({
                                "class": format!("{:?}", v.class),
                                "restriction": format!("{:?}", v.restriction),
                                "signature": signature(&v.signature),
                            })
                        })
                        .collect(),
                ),
            ),
            ("signatures_unknown", json!(signatures_unknown)),
        ]),
        E::Sink(SinkError::Io { kind, message }) => {
            data([("kind", json!(format!("{kind:?}"))), ("message", json!(message))])
        }
        E::Sink(SinkError::Failed(message)) => data([("message", json!(message))]),
        E::Sink(SinkError::NotEmpty { held }) => data([("held", json!(held))]),
        E::NotRebased { error, .. } => {
            let inner = Error::from(error.clone()).diagnostic();
            let mut error = json!({ "code": inner.code, "kind": inner.kind.as_str() });
            if let (Some(map), Ok(json)) = (error.as_object_mut(), serde_json::to_value(&inner))
                && let Some(location) = json.get("location")
            {
                map.insert("location".to_owned(), location.clone());
            }
            data([("error", error)])
        }
        _ => Data::new(),
    }
}

/// One signature as `would-invalidate-signatures` lists it: lamina-made values only.
fn signature(s: &laminapdf_save::SignatureInfo) -> Value {
    json!({
        "field": s.field.as_ref().map(ToString::to_string),
        "kind": format!("{:?}", s.kind),
        "value": s.value.map(|r| format!("{} {}", r.num, r.gen_nr)),
        "revision": s.revision,
        "covers": s.covers,
    })
}

#[cfg(feature = "extract")]
fn text(e: &laminapdf_text::Error) -> Data {
    use laminapdf_text::Error as E;
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
        E::InvalidSpans { at } => data([("at", json!(at))]),
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

#[cfg(feature = "build")]
fn build(e: &laminapdf_build::Error) -> Data {
    use laminapdf_build::Error as E;
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
        E::InvalidGeometry { what } | E::InvalidPageLabels { what } | E::FlowContentTooLarge { what } => {
            data([("what", json!(what))])
        }
        E::TextOverflow { fits, needed } => data([("fits", json!(fits)), ("needed", json!(needed))]),
        E::PageIndexOutOfRange { index, count } => data([("index", json!(index)), ("count", json!(count))]),
        E::UnknownDestination { name } => data([("name", json!(name))]),
        E::TagElementUnclosed { count } => data([("count", json!(count))]),
        E::ProfileVersionConflict { profile } | E::EmbeddedFileForbidden { profile } => {
            data([("profile", json!(profile.as_str()))])
        }
        E::MissingAltText { page }
        | E::UnnumberedHeadingMisused { page }
        | E::LinkWithoutContents { page }
        | E::DeviceColorWithoutOutputIntent { page } => data([("page", json!(page))]),
        E::EmbeddedFileWithoutMimeType { profile, name } => {
            data([("profile", json!(profile.as_str())), ("name", json!(name))])
        }
        E::UntaggedContent { page, content, count } => data([
            ("page", json!(page)),
            ("content", json!(content.as_str())),
            ("count", json!(count)),
        ]),
        E::HeadingLevelSkipped { page, level, previous } => data([
            ("page", json!(page)),
            ("level", json!(level)),
            ("previous", json!(previous)),
        ]),
        E::InvalidStructureNesting { page, kind, parent } => {
            data([("page", json!(page)), ("kind", json!(kind)), ("parent", json!(parent))])
        }
        E::LinkWithoutStructure { page, count } => data([("page", json!(page)), ("count", json!(count))]),
        E::FontNotEmbedded { profile, font } => data([("profile", json!(profile.as_str())), ("font", json!(font))]),
        E::GlyphWithoutUnicode { profile, font, gid } => data([
            ("profile", json!(profile.as_str())),
            ("font", json!(font)),
            ("gid", json!(gid)),
        ]),
        #[cfg(feature = "text")]
        E::Text(e) => text(e),
        E::Cos(e) => cos(e),
        _ => Data::new(),
    }
}

/// The `data` of `warning`.
pub(crate) fn of_warning(warning: &Warning) -> Data {
    match warning {
        Warning::Document(w) => cos_warning(*w),
        #[cfg(feature = "extract")]
        Warning::Font(w) => text_warning(w),
        #[cfg(feature = "extract")]
        Warning::Redaction(w) => redaction_warning(w),
        #[cfg(feature = "text")]
        Warning::Build(w) => build_warning(w),
    }
}

#[cfg(feature = "text")]
fn build_warning(w: &laminapdf_build::Warning) -> Data {
    match w {
        laminapdf_build::Warning::FlowKeepBroken { page } => data([("page", json!(page))]),
        laminapdf_build::Warning::MarkdownHtmlIgnored { offset }
        | laminapdf_build::Warning::MarkdownImageMissing { offset } => data([("offset", json!(offset))]),
        _ => Data::new(),
    }
}

#[cfg(feature = "extract")]
fn redaction_warning(w: &crate::RedactionWarning) -> Data {
    use crate::RedactionWarning as W;
    match w {
        W::CoveredText { chars, regions, .. } => data([("chars", json!(chars)), ("regions", json!(regions))]),
        W::RedactionNotApplied { annotations, .. } => data([("annotations", json!(annotations))]),
        W::TextInEarlierRevision { revision, words, pages } => data([
            ("revision", json!(revision)),
            ("words", json!(words)),
            ("pages", json!(pages)),
        ]),
    }
}

fn cos_warning(w: laminapdf_cos::Warning) -> Data {
    use laminapdf_cos::Warning as W;
    match w {
        W::StaleGeneration { generation, .. } => data([("generation", json!(generation))]),
        W::PageCountMismatch { declared, walked, .. } => {
            data([("declared", json!(declared)), ("walked", json!(walked))])
        }
        W::InvalidBox { key, .. } | W::InvalidDate { key, .. } | W::InvalidEntry { key, .. } => {
            data([("key", json!(key))])
        }
        W::InvalidPageLabel { start } => data([("start", json!(start))]),
        W::XmpInvalid {
            offset: Some(offset),
            reason,
            ..
        } => data([("offset", json!(offset)), ("reason", json!(reason))]),
        W::XmpInvalid {
            offset: None, reason, ..
        } => data([("reason", json!(reason))]),
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
        W::ExtractLimitBlocks { limit, .. } | W::ExtractTableLimit { limit, .. } => data([("limit", json!(limit))]),
        W::ExtractStructTreeIgnored { reason, .. } | W::ExtractColumnOrderUncertain { reason, .. } => {
            data([("reason", json!(reason))])
        }
        W::ExtractUntaggedContent { chars, .. } => data([("chars", json!(chars))]),
        W::ExtractNeedsOcr { status, .. } => data([("status", json!(status))]),
        _ => Data::new(),
    }
}

#[cfg(feature = "extract")]
fn text_warning(w: &laminapdf_text::Warning) -> Data {
    use laminapdf_text::Warning as W;
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
