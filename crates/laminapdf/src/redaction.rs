//! What a failed redaction leaves in a file (todo T2b), for `laminapdf check`: text painted
//! over by a box (ADR 0030 §4's `covered` flag), redaction marks never applied, and words an
//! earlier revision holds that the latest one has nowhere. Each finding is a [`Warning`] with a
//! count and page indices, never the text itself.

use std::collections::BTreeMap;

use laminapdf_cos::{Location, OpenOptions};
use laminapdf_syntax::Object;

use crate::extract::{DropReason, Dropped, ExtractOptions, TextOptions};
use crate::job::CancelToken;
use crate::{Document, Error, Result, Warning};

/// One finding of [`Document::redaction_warnings`]; [`Warning::Redaction`] carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RedactionWarning {
    /// A page has text painted over by a later shape, image or annotation (`covered-text`).
    CoveredText {
        /// The 0-based page index.
        page: u32,
        /// Characters under covers, spaces left out.
        chars: u64,
        /// Covered stretches of text.
        regions: u32,
    },
    /// A page has `Redact` annotations that were never applied (`redaction-not-applied`).
    RedactionNotApplied {
        /// The 0-based page index.
        page: u32,
        /// How many.
        annotations: u32,
    },
    /// An earlier revision holds words the latest revision has nowhere
    /// (`text-in-earlier-revision`).
    TextInEarlierRevision {
        /// The revision's index, 0 for the original file.
        revision: u32,
        /// Words that revision holds more often than the latest.
        words: u64,
        /// That revision's page indices where they are.
        pages: Vec<u32>,
    },
}

impl RedactionWarning {
    /// Every code, in the order `fixtures/codes.json`'s `facadeWarnings` lists them.
    pub const CODES: &'static [&'static str] = &["covered-text", "redaction-not-applied", "text-in-earlier-revision"];

    /// The stable kebab-case code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::CoveredText { .. } => "covered-text",
            Self::RedactionNotApplied { .. } => "redaction-not-applied",
            Self::TextInEarlierRevision { .. } => "text-in-earlier-revision",
        }
    }

    /// The page, for the two page findings.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::CoveredText { page, .. } | Self::RedactionNotApplied { page, .. } => Location {
                page_index: Some(u64::from(*page)),
                ..Location::NONE
            },
            Self::TextInEarlierRevision { .. } => Location::NONE,
        }
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The words of `text`: runs of letters and digits at least two characters long.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
}

/// How often each word appears in `texts`.
fn count<'t>(texts: impl IntoIterator<Item = &'t str>) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for text in texts {
        for word in words(text) {
            *out.entry(word.to_owned()).or_insert(0) += 1;
        }
    }
    out
}

/// Every page's text, visible or not, each page's lines joined.
fn page_words(doc: &Document) -> Result<Vec<String>> {
    Ok(doc
        .page_texts(&TextOptions::default())?
        .iter()
        .map(|t| t.lines().join("\n"))
        .collect())
}

/// The words of `earlier` (pages) the latest revision holds less often: how many, and on which
/// pages of `earlier` they are.
pub(crate) fn removed(latest: &BTreeMap<String, u64>, earlier: &[String]) -> (u64, Vec<u32>) {
    let counts = count(earlier.iter().map(String::as_str));
    let gone: BTreeMap<&str, u64> = counts
        .iter()
        .filter_map(|(word, &n)| {
            let left = latest.get(word).copied().unwrap_or(0);
            (n > left).then_some((word.as_str(), n - left))
        })
        .collect();
    let pages = earlier
        .iter()
        .enumerate()
        .filter(|(_, text)| words(text).any(|w| gone.contains_key(w)))
        .map(|(i, _)| to_u32(i))
        .collect();
    (gone.values().sum(), pages)
}

/// Each earlier revision's removed words, newest first, as `(revision, words, pages)`: what
/// `text-in-earlier-revision` reports, and what a fuller report can list.
pub(crate) fn earlier_revisions(
    doc: &Document,
    options: &OpenOptions,
    cancel: &CancelToken,
) -> Result<Vec<(u32, u64, Vec<u32>)>> {
    let revisions = doc.revision_count()?;
    if revisions < 2 {
        return Ok(Vec::new());
    }
    let latest = count(page_words(doc)?.iter().map(String::as_str));
    let tolerant = options.clone().with_strict(false);
    let mut out = Vec::new();
    for revision in (0..revisions - 1).rev() {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        // An earlier revision that no longer reads tells nothing; its warnings aren't the file's.
        let Ok(earlier) = doc.open_revision(revision, tolerant.clone()) else {
            continue;
        };
        let Ok(pages) = page_words(&earlier) else {
            continue;
        };
        let (gone, on) = removed(&latest, &pages);
        if gone > 0 {
            out.push((revision, gone, on));
        }
    }
    Ok(out)
}

impl Document {
    /// How many revisions the file has: 1 for a file never updated, or one whose
    /// cross-reference table was rebuilt; each incremental update adds one (ISO 32000-2 §7.5.6).
    ///
    /// # Errors
    /// `data-pending` for a source read in windows, which has to be read whole.
    pub fn revision_count(&self) -> Result<u32> {
        Ok(to_u32(self.ops().revision_ends()?.len()))
    }

    /// Revision `index` of the file (0 is the original) opened as a document of its own: the
    /// prefix of the file that ends with that revision's `%%EOF`. The latest revision is
    /// `revision_count() - 1`, the file itself.
    ///
    /// # Errors
    /// `revision-index-out-of-range` past the last revision; what [`Document::open_bytes`]
    /// returns for the prefix.
    pub fn open_revision(&self, index: u32, options: OpenOptions) -> Result<Self> {
        let ends = self.ops().revision_ends()?;
        let count = to_u32(ends.len());
        let end = usize::try_from(index)
            .ok()
            .and_then(|i| ends.get(i))
            .and_then(|&end| usize::try_from(end).ok())
            .ok_or(Error::RevisionIndexOutOfRange { index, count })?;
        let bytes = self.ops().whole_input()?;
        let prefix = bytes.get(..end).unwrap_or(&bytes).to_vec();
        Self::open_bytes(prefix, options)
    }

    /// The text a reader can't see, page by page ([ADR 0030] §4): a `dropped` entry per hidden
    /// run, without building blocks, tables or a reading order.
    ///
    /// # Errors
    /// `cancelled`; the reading errors.
    ///
    /// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
    pub fn hidden_text(&self, cancel: &CancelToken) -> Result<Vec<Dropped>> {
        Ok(laminapdf_extract::hidden_text(
            self.ops(),
            &ExtractOptions::default(),
            cancel,
        )?)
    }

    /// What a failed redaction left in the file, as warnings: `covered-text` per page with
    /// covered text, `redaction-not-applied` per page with `Redact` annotations, and
    /// `text-in-earlier-revision` per earlier revision holding words the latest has nowhere.
    /// Earlier revisions open with `options` (its password and limits), never strictly.
    ///
    /// # Errors
    /// `cancelled` (checked between pages and revisions); the reading errors of the latest
    /// revision.
    pub fn redaction_warnings(&self, options: &OpenOptions, cancel: &CancelToken) -> Result<Vec<Warning>> {
        let mut out = Vec::new();
        let mut covered: BTreeMap<usize, (u64, u32)> = BTreeMap::new();
        for d in self.hidden_text(cancel)? {
            if d.reason == DropReason::Covered {
                let e = covered.entry(d.page).or_default();
                e.0 += u64::try_from(d.chars).unwrap_or(u64::MAX);
                e.1 = e.1.saturating_add(1);
            }
        }
        out.extend(
            covered
                .into_iter()
                .map(|(page, (chars, regions))| RedactionWarning::CoveredText {
                    page: to_u32(page),
                    chars,
                    regions,
                }),
        );
        for (page, annotations) in self.redact_annotations(cancel)? {
            out.push(RedactionWarning::RedactionNotApplied { page, annotations });
        }
        for (revision, words, pages) in earlier_revisions(self, options, cancel)? {
            out.push(RedactionWarning::TextInEarlierRevision { revision, words, pages });
        }
        Ok(out.into_iter().map(Warning::Redaction).collect())
    }

    /// Each page with `Redact` annotations (§12.5.6.23), and how many.
    fn redact_annotations(&self, cancel: &CancelToken) -> Result<Vec<(u32, u32)>> {
        let doc = self.ops();
        let mut out = Vec::new();
        for (index, page) in doc.pages()?.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let mut n = 0u32;
            for annot in laminapdf_cos::tolerant(page.annots())?.unwrap_or_default() {
                let Some(object) = laminapdf_cos::tolerant(doc.resolve(annot))? else {
                    continue;
                };
                let redact = object
                    .as_dict()
                    .and_then(|d| d.get("Subtype"))
                    .is_some_and(|s| matches!(s, Object::Name(n) if n.as_bytes() == b"Redact"));
                if redact {
                    n = n.saturating_add(1);
                }
            }
            if n > 0 {
                out.push((to_u32(index), n));
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::{RedactionWarning, count, removed, words};
    use crate::Warning;
    use crate::test_codes as codes;

    fn samples() -> Vec<RedactionWarning> {
        vec![
            RedactionWarning::CoveredText {
                page: 0,
                chars: 9,
                regions: 1,
            },
            RedactionWarning::RedactionNotApplied {
                page: 1,
                annotations: 2,
            },
            RedactionWarning::TextInEarlierRevision {
                revision: 0,
                words: 1,
                pages: vec![0],
            },
        ]
    }

    #[test]
    fn codes_matches_the_facade_warnings_of_codes_json() {
        let listed: Vec<String> = codes::codes_json()["facadeWarnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| e["code"].as_str().map(str::to_owned))
            .collect();
        let own: Vec<&str> = samples().iter().map(RedactionWarning::code).collect();
        assert_eq!(own, RedactionWarning::CODES);
        assert_eq!(listed, RedactionWarning::CODES);
    }

    #[test]
    fn every_redaction_warning_has_the_kind_codes_json_lists() {
        let warnings: Vec<Warning> = samples().into_iter().map(Warning::Redaction).collect();
        codes::assert_kinds_match(warnings.iter().map(|w| (w.code(), w.kind().as_str())));
    }

    #[test]
    fn words_are_two_letters_or_more() {
        assert_eq!(
            words("Hello, World! a 42 é").collect::<Vec<_>>(),
            ["Hello", "World", "42"]
        );
    }

    #[test]
    fn a_word_the_latest_revision_still_has_isnt_removed() {
        let latest = count(["Hello, Update"]);
        let earlier = ["Hello, World".to_owned()];
        assert_eq!(removed(&latest, &earlier), (1, vec![0]));
    }

    #[test]
    fn a_word_repeated_less_often_counts_by_how_much() {
        let latest = count(["the cat"]);
        let earlier = ["intro".to_owned(), "the cat the cat the".to_owned()];
        assert_eq!(removed(&latest, &earlier), (4, vec![0, 1]));
    }

    #[test]
    fn nothing_removed_gives_nothing() {
        let latest = count(["same words here"]);
        assert_eq!(removed(&latest, &["same words".to_owned()]), (0, vec![]));
    }
}
