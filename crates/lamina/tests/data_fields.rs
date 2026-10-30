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

use lamina::{Error, Warning};
use lamina_cos::Warning as W;
use lamina_syntax::{Limit, Position};

/// Codes whose page lists data keys but whose value can't be built outside its crate.
const UNBUILDABLE: [(&str, &str); 4] = [
    (
        "not-rebased",
        "SaveError::NotRebased carries a SavedOutput, whose constructor is private to lamina-save",
    ),
    // `bindingErrors`: raised by lamina-wasm, not the façade; its tests check their data keys.
    ("builder-finished", "raised by lamina-wasm"),
    ("invalid-argument", "raised by lamina-wasm"),
    ("unknown-option", "raised by lamina-wasm"),
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

/// One value of every error and warning that has data, by code.
fn samples() -> BTreeMap<&'static str, serde_json::Value> {
    use lamina_build::{Error as B, ImageFormat};
    use lamina_text::{Error as T, FsType, Tag, Warning as TW};
    let s = String::new;
    let errors: Vec<Error> = vec![
        Error::from(lamina_cos::Error::LimitExceeded {
            limit: Limit::NestingDepth,
            at: Some(Position::Object(1)),
        }),
        Error::from(lamina_cos::Error::Strict {
            warning: W::MissingHeader,
        }),
        Error::from(lamina_cos::Error::DataPending { ranges: vec![(0, 1)] }),
        Error::from(lamina_cos::Error::Type {
            expected: "dictionary",
            context: "/Kids",
        }),
        Error::from(lamina_cos::Error::InvalidStructure {
            kind: "xref stream",
            message: "bad",
        }),
        Error::from(lamina_cos::Error::UnsupportedSecurityHandler {
            filter: s(),
            version: None,
            revision: None,
        }),
        Error::from(lamina_syntax::Error::UnsupportedFilter(s())),
        Error::from(lamina_ops::Error::UnsupportedVersion(9)),
        Error::from(lamina_ops::Error::InvalidLog(s())),
        Error::from(lamina_save::SaveError::RequiresRewrite { reasons: vec![] }),
        Error::from(lamina_save::SaveError::WouldInvalidateSignatures {
            signatures: vec![],
            rewrite_reasons: vec![],
            violations: vec![],
            signatures_unknown: false,
        }),
        Error::from(lamina_save::SaveError::Sink(lamina_save::SinkError::Io {
            kind: std::io::ErrorKind::Other,
            message: s(),
        })),
        Error::from(lamina_save::SaveError::Sink(lamina_save::SinkError::NotEmpty {
            held: 1,
        })),
        Error::PageIndexOutOfRange { index: 1, count: 1 },
        Error::from(T::InvalidFont { index: 0 }),
        Error::from(T::CollectionIndex { index: 1, count: 1 }),
        Error::from(T::UnknownAxis {
            font: s(),
            tag: Tag::WEIGHT,
        }),
        Error::from(T::InvalidAxisValue { tag: Tag::WEIGHT }),
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
        Error::from(B::TextOverflow { fits: 1, needed: 2 }),
        Error::from(B::PageIndexOutOfRange { index: 1, count: 1 }),
        Error::from(B::UnknownDestination { name: s() }),
        Error::from(B::InvalidPageLabels { what: "w" }),
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
        TW::ToUnicodeConflict {
            font: s(),
            gid: 1,
            kept: s(),
            dropped: s(),
        }
        .into(),
        TW::WholeFontRebuilt { font: s() }.into(),
    ];
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
