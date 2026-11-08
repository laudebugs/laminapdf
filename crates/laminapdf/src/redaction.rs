//! What a failed redaction leaves in a file, for `laminapdf check`: text painted
//! over by a box (ADR 0030 §4's `covered` flag), redaction marks never applied, and words an
//! earlier revision holds that the latest one has nowhere. Each finding is a [`Warning`] with a
//! count and page indices, never the text itself.

use std::collections::BTreeMap;

use laminapdf_cos::{Location, OpenOptions};
use laminapdf_syntax::Object;

use crate::extract::{DropReason, Dropped, ExtractOptions};
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

/// Every page's text, visible or not, each page's lines joined; the token is checked before
/// each page.
fn page_words(doc: &Document, cancel: &CancelToken) -> Result<Vec<String>> {
    let model = doc.ops();
    let mut extractor = laminapdf_text::TextExtractor::new();
    let mut out = Vec::new();
    for page in model.pages()?.iter() {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        out.push(extractor.page(model, &page)?.lines().join("\n"));
    }
    Ok(out)
}

/// The words of `earlier` (its pages) that `latest` (its pages) holds less often: how many, and
/// on which of `earlier`'s pages. A page counts when it holds such a word more often than the
/// latest revision's page of the same index; when pages moved and none does, every page holding
/// one counts.
pub(crate) fn removed(latest: &[String], earlier: &[String]) -> (u64, Vec<u32>) {
    let all = count(latest.iter().map(String::as_str));
    let gone: BTreeMap<String, u64> = count(earlier.iter().map(String::as_str))
        .into_iter()
        .filter_map(|(word, n)| {
            let left = all.get(&word).copied().unwrap_or(0);
            (n > left).then(|| (word, n - left))
        })
        .collect();
    let lost_on = |i: usize, text: &str| {
        let here = count([text]);
        let there = latest.get(i).map(|t| count([t.as_str()])).unwrap_or_default();
        gone.keys()
            .any(|w| here.get(w).copied().unwrap_or(0) > there.get(w).copied().unwrap_or(0))
    };
    let mut pages: Vec<u32> = earlier
        .iter()
        .enumerate()
        .filter(|(i, text)| lost_on(*i, text))
        .map(|(i, _)| to_u32(i))
        .collect();
    if pages.is_empty() {
        pages = earlier
            .iter()
            .enumerate()
            .filter(|(_, text)| words(text).any(|w| gone.contains_key(w)))
            .map(|(i, _)| to_u32(i))
            .collect();
    }
    (gone.values().sum(), pages)
}

/// Whether `error`, met reading an earlier revision, only says that revision is malformed or
/// uses something unsupported, so it can be skipped. Anything else (a password, a limit, data
/// not yet fetched, cancellation) stops the check: a revision it couldn't read might hold the
/// very text a redaction removed.
const fn skippable(error: &Error) -> bool {
    matches!(
        error.kind(),
        laminapdf_cos::ErrorKind::Input | laminapdf_cos::ErrorKind::Unsupported
    )
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
    let latest = page_words(doc, cancel)?;
    let tolerant = options.clone().with_strict(false);
    let mut out = Vec::new();
    for revision in (0..revisions - 1).rev() {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        // Its own warnings aren't the file's: only its words are compared.
        let pages = match doc
            .open_revision(revision, tolerant.clone())
            .and_then(|earlier| page_words(&earlier, cancel))
        {
            Ok(pages) => pages,
            Err(e) if skippable(&e) => continue,
            Err(e) => return Err(e),
        };
        let (gone, on) = removed(&latest, &pages);
        if gone > 0 {
            out.push((revision, gone, on));
        }
    }
    Ok(out)
}

impl Document {
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

    /// Each page with `Redact` annotations (§12.5.6.23), and how many. `/Annots` entries may
    /// be references or direct dictionaries, and `/Subtype` may be a reference too.
    fn redact_annotations(&self, cancel: &CancelToken) -> Result<Vec<(u32, u32)>> {
        let doc = self.ops();
        let is_redact = |annot: &Object| -> Result<bool> {
            let Some(annot) = laminapdf_cos::tolerant(doc.deref(annot))? else {
                return Ok(false);
            };
            let Some(subtype) = annot.as_dict().and_then(|d| d.get("Subtype")) else {
                return Ok(false);
            };
            let subtype = laminapdf_cos::tolerant(doc.deref(subtype))?;
            Ok(subtype.is_some_and(|s| matches!(&*s, Object::Name(n) if n.as_bytes() == b"Redact")))
        };
        let mut out = Vec::new();
        for (index, page) in doc.pages()?.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let Some(annots) = page.cos().dict.get("Annots") else {
                continue;
            };
            let Some(annots) = laminapdf_cos::tolerant(doc.deref(annots))? else {
                continue;
            };
            let mut n = 0u32;
            for annot in annots.as_array().unwrap_or_default() {
                if is_redact(annot)? {
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
    use super::{RedactionWarning, removed, words};
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
        let latest = ["Hello, Update".to_owned()];
        let earlier = ["Hello, World".to_owned()];
        assert_eq!(removed(&latest, &earlier), (1, vec![0]));
    }

    #[test]
    fn a_word_repeated_less_often_counts_by_how_much() {
        let latest = ["intro".to_owned(), "the cat".to_owned()];
        let earlier = ["intro".to_owned(), "the cat the cat the".to_owned()];
        assert_eq!(removed(&latest, &earlier), (3, vec![1]));
    }

    #[test]
    fn only_the_page_that_lost_a_common_word_is_listed() {
        let latest = ["the start".to_owned(), "the end".to_owned()];
        let earlier = ["the start".to_owned(), "the end the secret".to_owned()];
        assert_eq!(removed(&latest, &earlier), (2, vec![1]));
    }

    #[test]
    fn nothing_removed_gives_nothing() {
        let latest = ["same words here".to_owned()];
        assert_eq!(removed(&latest, &["same words".to_owned()]), (0, vec![]));
    }
}
