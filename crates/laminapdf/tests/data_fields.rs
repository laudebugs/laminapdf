//! ADR 0026 §1: the `data` of every error and warning has exactly the keys its
//! `docs/errors/<code>.md` page lists under *Data fields*.
//!
//! The façade maps other crates' `#[non_exhaustive]` enums to `data` with a wildcard arm, so a
//! variant added later would silently get no `data`. This test closes that gap from the other
//! side: every code whose page documents data keys must have a sample here, and the sample's
//! keys must be the documented ones. A new code with data fails until it is mapped and listed.
#![cfg(feature = "text")]
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test code and helpers: a panic is a test failure; one sample per variant is a long list"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use laminapdf::{Error, Warning};
use laminapdf_cos::Warning as W;
use laminapdf_syntax::{Limit, Position};

/// Codes whose page lists data keys but whose value can't be built outside its crate.
const UNBUILDABLE: [(&str, &str); 20] = [
    (
        "not-rebased",
        "SaveError::NotRebased carries a SavedOutput, whose constructor is private to laminapdf-save",
    ),
    // `bindingErrors`: raised by laminapdf-wasm, not the façade; its tests check their data keys.
    ("builder-finished", "raised by laminapdf-wasm"),
    ("invalid-argument", "raised by laminapdf-wasm"),
    ("unknown-option", "raised by laminapdf-wasm"),
    ("source-budget-exceeded", "raised by laminapdf-wasm"),
    // `packageErrors`: raised by @laminapdf/sdk; packages/sdk/test/source.test.ts checks the keys.
    ("fetch-failed", "raised by @laminapdf/sdk"),
    // packages/sdk/test/browser/worker.browser.test.ts checks these two's keys.
    ("worker-version-mismatch", "raised by @laminapdf/sdk/worker"),
    ("worker-crashed", "raised by @laminapdf/sdk/worker"),
    // packages/sdk/test/memory.test.ts and test/small-memory/oom.test.ts check these two's keys.
    ("wasm-trap", "raised by @laminapdf/sdk"),
    ("out-of-memory", "raised by @laminapdf/sdk"),
    // `renderErrors`: raised by laminapdf-render (not the façade); the fields are RenderError's.
    ("render-too-large", "raised by laminapdf-render"),
    ("render-failed", "raised by laminapdf-render"),
    ("render-backend-unavailable", "raised by laminapdf-render"),
    ("export-failed", "raised by laminapdf-render"),
    // `cliErrors`: raised by laminapdf-cli; its tests/data_fields.rs checks their keys.
    ("invalid-page-range", "raised by laminapdf-cli"),
    ("unknown-code", "raised by laminapdf-cli"),
    ("overwrite-refused", "raised by laminapdf-cli"),
    ("input-unreadable", "raised by laminapdf-cli"),
    ("output-unwritable", "raised by laminapdf-cli"),
    // Raised by `laminapdf mcp`; laminapdf-cli's tests/mcp.rs checks its key.
    ("path-outside-roots", "raised by laminapdf-cli"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The keys a page's *Data fields* section documents: the backticked names before the colon
/// of each list item, or, for a sentence, the backticked names of its first sentence outside
/// parentheses. `None.` documents none.
fn documented(code: &str) -> BTreeSet<String> {
    let page = std::fs::read_to_string(root().join(format!("docs/errors/{code}.md"))).unwrap();
    let section = page.split("## Data fields").nth(1).unwrap_or_default();
    let block: Vec<&str> = section
        .trim_start()
        .split("\n\n")
        .next()
        .unwrap_or_default()
        .lines()
        .collect();
    let mut keys = BTreeSet::new();
    if block.first().is_some_and(|l| l.starts_with("- ")) {
        for item in block.iter().filter(|l| l.starts_with("- ")) {
            keys.extend(backticked(item.split(':').next().unwrap_or_default()));
        }
    } else {
        let text = block.join(" ");
        let sentence = text.split(". ").next().unwrap_or_default();
        if !sentence.starts_with("None") {
            keys.extend(backticked(&without_parentheses(sentence)));
        }
    }
    keys
}

fn backticked(text: &str) -> Vec<String> {
    text.split('`').skip(1).step_by(2).map(str::to_owned).collect()
}

fn without_parentheses(text: &str) -> String {
    let mut depth = 0_u32;
    text.chars()
        .filter(|&c| {
            match c {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ => return depth == 0,
            }
            false
        })
        .collect()
}

/// Every code listed in `codes.json`.
fn listed_codes() -> Vec<String> {
    let text = std::fs::read_to_string(root().join("fixtures/codes.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json.as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "version" | "docsUrl" | "limits"))
        .flat_map(|(_, group)| group.as_array().unwrap().clone())
        .map(|entry| entry["code"].as_str().unwrap().to_owned())
        .collect()
}

/// One value of every error and warning that has data.
fn sample_values() -> (Vec<Error>, Vec<Warning>) {
    use laminapdf_build::{ContentKind, Error as B, ImageFormat, Profile};
    use laminapdf_text::{Error as T, FsType, OpenTypeTag, Warning as TW};
    let s = String::new;
    let errors: Vec<Error> = vec![
        Error::from(laminapdf_cos::Error::LimitExceeded {
            limit: Limit::NestingDepth,
            at: Some(Position::Object(1)),
        }),
        Error::from(laminapdf_cos::Error::Strict {
            warning: W::MissingHeader,
        }),
        Error::from(laminapdf_cos::Error::DataPending { ranges: vec![(0, 1)] }),
        Error::from(laminapdf_cos::Error::Type {
            expected: "dictionary",
            context: "/Kids",
        }),
        Error::from(laminapdf_cos::Error::InvalidStructure {
            kind: "xref stream",
            message: "bad",
        }),
        Error::from(laminapdf_cos::Error::UnsupportedSecurityHandler {
            filter: s(),
            version: None,
            revision: None,
        }),
        Error::from(laminapdf_syntax::Error::UnsupportedFilter(s())),
        Error::from(laminapdf_ops::Error::UnsupportedVersion(9)),
        Error::from(laminapdf_ops::Error::InvalidLog(s())),
        Error::from(laminapdf_ops::Error::InvalidXmpPacket(
            laminapdf_model::xmp::ParseError::Syntax { offset: 0 },
        )),
        Error::from(laminapdf_ops::Error::InvalidXmpProperty { reason: "name" }),
        Error::from(laminapdf_save::SaveError::RequiresRewrite { reasons: vec![] }),
        Error::from(laminapdf_save::SaveError::WouldInvalidateSignatures {
            signatures: vec![],
            rewrite_reasons: vec![],
            violations: vec![],
            signatures_unknown: false,
        }),
        Error::from(laminapdf_save::SaveError::Sink(laminapdf_save::SinkError::Io {
            kind: std::io::ErrorKind::Other,
            message: s(),
        })),
        Error::from(laminapdf_save::SaveError::Sink(laminapdf_save::SinkError::NotEmpty {
            held: 1,
        })),
        Error::PageIndexOutOfRange { index: 1, count: 1 },
        Error::RevisionIndexOutOfRange { index: 2, count: 2 },
        Error::from(T::InvalidFont { index: 0 }),
        Error::from(T::CollectionIndex { index: 1, count: 1 }),
        Error::from(T::UnknownAxis {
            font: s(),
            tag: OpenTypeTag::WEIGHT,
        }),
        Error::from(T::InvalidAxisValue {
            tag: OpenTypeTag::WEIGHT,
        }),
        Error::from(T::WrongDocument { object: 1 }),
        Error::from(T::FontNotEmbeddable {
            font: s(),
            fs_type: FsType::new(2),
        }),
        Error::from(T::MissingGlyph { ch: 'x', font: s() }),
        Error::from(T::GlyphOutOfRange {
            gid: 1,
            count: 1,
            font: s(),
        }),
        Error::from(T::UnknownFont { key: 1 }),
        Error::from(T::Subset { font: s(), reason: "r" }),
        Error::from(T::InvalidFeature { feature: s() }),
        Error::from(T::InvalidTextRange {
            start: 0,
            end: 1,
            len: 0,
        }),
        Error::from(T::InvalidSpans { at: 0 }),
        Error::from(T::UnsupportedLayoutOption { option: "o" }),
        Error::from(T::FontMismatch {
            font: s(),
            shaped_with: s(),
        }),
        Error::from(T::FontNotShapable { font: s() }),
        Error::from(T::EncodingFull {
            font: s(),
            limit: 1,
            free: 0,
        }),
        Error::from(B::InvalidImage {
            format: ImageFormat::Png,
            reason: "r",
        }),
        Error::from(B::UnsupportedImage {
            format: ImageFormat::Jpeg,
            feature: "f",
        }),
        Error::from(B::ImageTooLarge { limit: 1 }),
        Error::from(B::ImageDataLength { expected: 1, actual: 0 }),
        Error::from(B::InvalidIccProfile { reason: "r" }),
        Error::from(B::ColorComponents { expected: 1, actual: 0 }),
        Error::from(B::InvalidColor { text: s() }),
        Error::from(B::InvalidGeometry { what: "w" }),
        Error::from(B::FlowContentTooLarge { what: "line" }),
        Error::from(B::TextOverflow { fits: 1, needed: 2 }),
        Error::from(B::PageIndexOutOfRange { index: 1, count: 1 }),
        Error::from(B::UnknownDestination { name: s() }),
        Error::from(B::InvalidPageLabels { what: "w" }),
        Error::from(B::TagElementNotOpen),
        Error::from(B::TagElementUnclosed { count: 1 }),
        Error::from(B::ProfileVersionConflict {
            profile: Profile::PdfA2b,
        }),
        Error::from(B::MissingTitle),
        Error::from(B::MissingLang),
        Error::from(B::MissingAltText { page: 0 }),
        Error::from(B::UntaggedContent {
            page: 0,
            content: ContentKind::Path,
            count: 1,
        }),
        Error::from(B::HeadingLevelSkipped {
            page: 0,
            level: 3,
            previous: 1,
        }),
        Error::from(B::UnnumberedHeadingMisused { page: 0 }),
        Error::from(B::InvalidStructureNesting {
            page: 0,
            kind: s(),
            parent: s(),
        }),
        Error::from(B::LinkWithoutStructure { page: 0, count: 1 }),
        Error::from(B::LinkWithoutContents { page: 0 }),
        Error::from(B::FontNotEmbedded {
            profile: Profile::PdfUa1,
            font: s(),
        }),
        Error::from(B::GlyphWithoutUnicode {
            profile: Profile::PdfA2u,
            font: s(),
            gid: 1,
        }),
        Error::from(B::DeviceColorWithoutOutputIntent { page: 0 }),
        Error::from(B::EmbeddedFileForbidden {
            profile: Profile::PdfA2b,
        }),
        Error::from(B::EmbeddedFileWithoutMimeType {
            profile: Profile::PdfA3b,
            name: "data.bin".into(),
        }),
        Error::from(B::InvalidDocument {
            pointer: "/blocks/0/spaceAftr".into(),
            reason: "unknown-key",
            did_you_mean: vec!["spaceAfter".into()],
        }),
        Error::from(B::TemplateDataMissing {
            pointer: "/total".into(),
            placeholder: "$data",
        }),
        Error::from(B::TemplateDataType {
            pointer: "/items".into(),
            placeholder: "$each",
            expected: "list",
        }),
        Error::from(B::TemplateTooLarge { limit: 100_000 }),
    ];
    let warnings: Vec<Warning> = vec![
        W::XrefStmIgnored { offset: 1 }.into(),
        W::StaleGeneration {
            object: 1,
            generation: 1,
        }
        .into(),
        W::PageCountMismatch {
            object: None,
            declared: 1,
            walked: 2,
        }
        .into(),
        W::InvalidBox {
            object: None,
            page: 0,
            key: "MediaBox",
        }
        .into(),
        W::InvalidDate {
            object: None,
            key: "ModDate",
        }
        .into(),
        W::InvalidEntry {
            object: None,
            key: "Info",
        }
        .into(),
        W::InvalidPageLabel { start: 0 }.into(),
        W::XmpInvalid {
            object: None,
            offset: Some(0),
            reason: "dtd",
        }
        .into(),
        W::XmpRepaired {
            object: None,
            reason: "undeclared-prefix",
        }
        .into(),
        W::ContentUnknownOperator {
            object: None,
            offset: 0,
        }
        .into(),
        W::ContentBadOperands {
            object: None,
            offset: 0,
        }
        .into(),
        W::ContentUnbalancedQ {
            object: None,
            offset: 0,
        }
        .into(),
        W::ContentUnbalancedBt {
            object: None,
            offset: 0,
        }
        .into(),
        W::ContentUnbalancedMarked {
            object: None,
            offset: 0,
        }
        .into(),
        W::ContentSyntax {
            object: None,
            offset: 0,
        }
        .into(),
        W::ContentResourceMissing {
            object: None,
            offset: 0,
            category: "Font",
        }
        .into(),
        W::InlineImageUnterminated {
            object: None,
            offset: 0,
        }
        .into(),
        W::ContentPathUnpainted {
            object: None,
            offset: 0,
        }
        .into(),
        W::XObjectCycle { object: 1 }.into(),
        W::ExtractLimitBlocks { page: 0, limit: 1 }.into(),
        W::ExtractTableLimit { page: 0, limit: 1 }.into(),
        W::ExtractStructTreeIgnored {
            page: 0,
            reason: "coverage",
        }
        .into(),
        W::ExtractUntaggedContent { page: 0, chars: 1 }.into(),
        W::ExtractNeedsOcr {
            page: 0,
            status: "needs-ocr",
        }
        .into(),
        W::ExtractColumnOrderUncertain {
            page: 0,
            reason: "narrow-gutter",
        }
        .into(),
        TW::ToUnicodeConflict {
            font: s(),
            gid: 1,
            kept: s(),
            dropped: s(),
        }
        .into(),
        TW::WholeFontRebuilt { font: s() }.into(),
        Warning::Build(laminapdf_build::Warning::FlowKeepBroken { page: 1 }),
        Warning::Build(laminapdf_build::Warning::MarkdownHtmlIgnored { offset: 4 }),
        Warning::Build(laminapdf_build::Warning::MarkdownImageMissing { offset: 4 }),
        Warning::Redaction(laminapdf::RedactionWarning::CoveredText {
            page: 0,
            chars: 9,
            regions: 1,
        }),
        Warning::Redaction(laminapdf::RedactionWarning::RedactionNotApplied {
            page: 0,
            annotations: 2,
        }),
        Warning::Redaction(laminapdf::RedactionWarning::TextInEarlierRevision {
            revision: 0,
            words: 1,
            pages: vec![0],
        }),
        Warning::Conformance(laminapdf::ConformanceWarning::PdfaOneAuthor { authors: 2 }),
        Warning::Conformance(laminapdf::ConformanceWarning::PdfuaTitleMissing),
        Warning::Conformance(laminapdf::ConformanceWarning::PdfaInfoMismatch {
            keys: vec![laminapdf::MetadataKey::Title],
        }),
    ];
    (errors, warnings)
}

/// The data of every sample, by code.
fn samples() -> BTreeMap<&'static str, serde_json::Value> {
    let (errors, warnings) = sample_values();
    let mut out = BTreeMap::new();
    for e in errors {
        out.insert(e.code(), serde_json::to_value(e.diagnostic().data).unwrap());
    }
    for w in warnings {
        out.insert(w.code(), serde_json::to_value(w.diagnostic().data).unwrap());
    }
    out
}

#[test]
fn every_code_with_data_fields_has_a_sample_with_exactly_those_keys() {
    let samples = samples();
    let mut problems = Vec::new();
    for code in listed_codes() {
        let want = documented(&code);
        if UNBUILDABLE.iter().any(|(c, _)| *c == code) {
            continue;
        }
        let got: BTreeSet<String> = samples
            .get(code.as_str())
            .and_then(|d| d.as_object())
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        if !want.is_empty() && !samples.contains_key(code.as_str()) {
            problems.push(format!(
                "{code}: its page documents {want:?} but data_fields.rs has no sample"
            ));
        } else if got != want {
            problems.push(format!(
                "{code}: data keys {got:?}, docs/errors/{code}.md lists {want:?}"
            ));
        }
    }
    assert!(problems.is_empty(), "data vs docs/errors:\n  {}", problems.join("\n  "));
}

#[test]
fn every_sample_is_for_a_listed_code() {
    let listed = listed_codes();
    let unknown: Vec<&str> = samples()
        .into_keys()
        .filter(|c| !listed.iter().any(|l| l == c))
        .collect();
    assert!(unknown.is_empty(), "{unknown:?}");
}

/// The façade reads code and kind from the generated `ErrorCode`/`WarningCode` tables; the
/// wrapped crates compute them with their own exhaustive matches, which their tests check
/// against `codes.json`. Every error sample's kind must be the one `codes.json` lists (the
/// inner errors are opaque, ADR 0028 F9), and every warning's must be its crate's, so a stale
/// table or a wrong `codes.json` kind fails here.
#[test]
fn the_generated_tables_agree_with_the_wrapped_crates() {
    let (errors, warnings) = sample_values();
    let codes = laminapdf_test_fixtures::codes::codes_json();
    let mut wrong: Vec<String> = errors
        .iter()
        .filter(|e| laminapdf_test_fixtures::codes::listed_kind(&codes, e.code()).as_deref() != Some(e.kind().as_str()))
        .map(|e| format!("{}: façade says {:?}", e.code(), e.kind()))
        .collect();
    for w in &warnings {
        let (code, kind) = match w {
            Warning::Document(i) => (i.code(), i.kind()),
            Warning::Font(i) => (i.code(), i.kind()),
            Warning::Build(i) => (i.code(), i.kind()),
            Warning::Conformance(i) => (i.code(), i.kind()),
            _ => continue,
        };
        if (w.code(), w.kind()) != (code, kind) {
            wrong.push(format!("{code}: façade says {} {:?}", w.code(), w.kind()));
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}
