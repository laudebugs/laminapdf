//! The files the pinned PDFium crashed or hung on by itself in the last run. Each such hang costs
//! the run a chunk's time limit, the file's own limit and PDFium's limit again when the blame pass
//! reruns it alone: about half an hour of the nightly for five files. PDFium is pinned and the
//! corpus files are pinned by SHA-256, so the next run with the same pin and file skips PDFium on
//! it and reruns only hayro, which is ours and changes. The report keeps the list
//! ([`FIELD`]), and the nightly restores the last report from the Actions cache.
//!
//! An entry stays at most [`RECHECK_DAYS`] days from when PDFium last failed on the file in
//! front of us, so a slow run that only looked like a hang is rendered again within a week.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::rows::{Blame, Partial};

/// The report's field: `{"pdfium": <pin>, "files": {<name>: {"sha256", "outcome", "pages", "seen"}}}`.
pub(super) const FIELD: &str = "pdfium_failures";

/// How many days an entry is trusted before PDFium renders the file again.
pub(super) const RECHECK_DAYS: u64 = 7;

/// A file PDFium crashed or hung on by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Known {
    /// The corpus manifest's SHA-256 of the file when PDFium failed on it.
    pub(super) sha256: String,
    /// `timeout` or `crash`: the both-engines pass's outcome, which decides the rows' class.
    pub(super) outcome: String,
    /// The most pages the last report gave the file, so the rows count the same.
    pub(super) pages: Option<u64>,
    /// Days since the Unix epoch when PDFium last failed on it in front of us.
    pub(super) seen: u64,
}

impl Known {
    fn json(&self) -> Value {
        json!({"sha256": self.sha256, "outcome": self.outcome, "pages": self.pages, "seen": self.seen})
    }
}

/// Today, in days since the Unix epoch.
fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400)
}

/// The entries of `report` that still hold: written under PDFium `pin`, for a file whose
/// manifest hash (`hashes`) is the same, seen less than [`RECHECK_DAYS`] days before `today`.
/// A report without the field, or under another pin, gives none.
fn select(report: &Value, pin: &str, hashes: &BTreeMap<String, String>, today: u64) -> BTreeMap<String, Known> {
    let Some(field) = report.get(FIELD) else {
        return BTreeMap::new();
    };
    if field.get("pdfium").and_then(Value::as_str) != Some(pin) {
        return BTreeMap::new();
    }
    let Some(files) = field.get("files").and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    files
        .iter()
        .filter_map(|(name, entry)| {
            let known = Known {
                sha256: entry.get("sha256")?.as_str()?.to_owned(),
                outcome: entry.get("outcome")?.as_str()?.to_owned(),
                pages: entry.get("pages").and_then(Value::as_u64),
                seen: entry.get("seen")?.as_u64()?,
            };
            let fresh = today.saturating_sub(known.seen) < RECHECK_DAYS;
            let same = hashes.get(name) == Some(&known.sha256);
            let outcome = matches!(known.outcome.as_str(), "timeout" | "crash");
            (fresh && same && outcome).then(|| (name.clone(), known))
        })
        .collect()
}

/// What the last run's report says, and what this run needs to write the next one's list.
pub(super) struct LastRun {
    /// The files PDFium is skipped on in this run.
    pub(super) skipped: BTreeMap<String, Known>,
    hashes: BTreeMap<String, String>,
    today: u64,
}

impl LastRun {
    /// [`select`] over `out/report.json`, the last run's report (none when it can't be read),
    /// for the pinned PDFium and the files among `inputs`; `hashes` are the manifest's.
    pub(super) fn read(out: &Path, hashes: BTreeMap<String, String>, inputs: &[(String, PathBuf)]) -> Self {
        let today = today();
        let mut skipped: BTreeMap<String, Known> = fs::read_to_string(out.join("report.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(|report| select(&report, crate::pdfium::TAG, &hashes, today))
            .unwrap_or_default();
        skipped.retain(|name, _| inputs.iter().any(|(n, _)| n == name));
        Self { skipped, hashes, today }
    }

    /// The report's [`FIELD`] for the next run (see [`record`]).
    pub(super) fn record(
        &self,
        blames: &BTreeMap<String, Blame>,
        facts: &serde_json::Map<String, Value>,
        partials: &BTreeMap<String, Partial>,
    ) -> Value {
        record(crate::pdfium::TAG, blames, facts, partials, self)
    }

    /// The summary's line about the files PDFium skipped, empty when it skipped none.
    pub(super) fn note(&self) -> String {
        if self.skipped.is_empty() {
            return String::new();
        }
        format!(
            "\n\n_PDFium skipped {} files it crashed or hung on alone in the last run (same PDFium, same file; \
             rendered again after {RECHECK_DAYS} days): {}._",
            self.skipped.len(),
            self.skipped.keys().map(String::as_str).collect::<Vec<_>>().join(", ")
        )
    }
}

/// What this run knows for the next one: every file PDFium crashed or hung on by itself (blamed
/// on PDFium, whatever hayro did), seen today when PDFium ran on it and carried over with its
/// old day when `last` skipped it, with the most pages any reading of it reported. A file whose
/// pages both engines finished some of is left out: skipping the both-engines pass would lose
/// those pages' rows.
fn record(
    pin: &str,
    blames: &BTreeMap<String, Blame>,
    facts: &serde_json::Map<String, Value>,
    partials: &BTreeMap<String, Partial>,
    last: &LastRun,
) -> Value {
    let files: serde_json::Map<String, Value> = blames
        .iter()
        .filter(|(_, blame)| blame.pdfium)
        .filter_map(|(name, _)| {
            let outcome = facts.get(name)?.get("outcome")?.as_str()?;
            let partial = partials.get(name);
            if !matches!(outcome, "timeout" | "crash") || partial.is_some_and(|p| !p.rows.is_empty()) {
                return None;
            }
            let known = Known {
                sha256: last.hashes.get(name)?.clone(),
                outcome: outcome.to_owned(),
                pages: partial.and_then(|p| p.pages),
                seen: last.skipped.get(name).map_or(last.today, |k| k.seen),
            };
            Some((name.clone(), known.json()))
        })
        .collect();
    json!({"pdfium": pin, "files": files})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hashes() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("pdfjs/a.pdf".to_owned(), "aa".to_owned()),
            ("pdfjs/b.pdf".to_owned(), "bb".to_owned()),
        ])
    }

    fn report(pin: &str, sha: &str, outcome: &str, seen: u64) -> Value {
        json!({FIELD: {"pdfium": pin, "files": {
            "pdfjs/a.pdf": {"sha256": sha, "outcome": outcome, "pages": 1, "seen": seen}
        }}})
    }

    #[test]
    fn select_should_keep_an_entry_with_the_same_pin_and_file() {
        let got = select(
            &report("chromium/1", "aa", "timeout", 100),
            "chromium/1",
            &hashes(),
            102,
        );
        let want = Known {
            sha256: "aa".to_owned(),
            outcome: "timeout".to_owned(),
            pages: Some(1),
            seen: 100,
        };
        assert_eq!(got, BTreeMap::from([("pdfjs/a.pdf".to_owned(), want)]));
    }

    #[test]
    fn select_should_drop_every_entry_under_another_pin() {
        assert!(
            select(
                &report("chromium/1", "aa", "timeout", 100),
                "chromium/2",
                &hashes(),
                100
            )
            .is_empty()
        );
    }

    #[test]
    fn select_should_drop_a_file_whose_content_changed() {
        assert!(
            select(
                &report("chromium/1", "a2", "timeout", 100),
                "chromium/1",
                &hashes(),
                100
            )
            .is_empty()
        );
    }

    #[test]
    fn select_should_drop_an_entry_due_for_a_recheck() {
        let report = report("chromium/1", "aa", "crash", 100);
        assert!(select(&report, "chromium/1", &hashes(), 100 + RECHECK_DAYS).is_empty());
    }

    #[test]
    fn select_should_ignore_a_report_without_the_field() {
        assert!(select(&json!({"rows": []}), "chromium/1", &hashes(), 100).is_empty());
    }

    fn blamed(hayro: bool, pdfium: bool) -> Blame {
        Blame { hayro, pdfium }
    }

    fn one_page(finished: bool) -> Partial {
        let rows = if finished {
            BTreeMap::from([(1, json!({"class": "pass"}))])
        } else {
            BTreeMap::new()
        };
        Partial { pages: Some(1), rows }
    }

    #[test]
    fn record_should_list_pdfium_failures_and_keep_a_skipped_file_s_day() {
        let blames = BTreeMap::from([
            ("pdfjs/a.pdf".to_owned(), blamed(false, true)),
            ("pdfjs/b.pdf".to_owned(), blamed(true, false)),
        ]);
        let facts = serde_json::Map::from_iter([
            ("pdfjs/a.pdf".to_owned(), json!({"outcome": "timeout"})),
            ("pdfjs/b.pdf".to_owned(), json!({"outcome": "crash"})),
        ]);
        let partials = BTreeMap::from([("pdfjs/a.pdf".to_owned(), one_page(false))]);
        let last = LastRun {
            skipped: select(&report("chromium/1", "aa", "timeout", 90), "chromium/1", &hashes(), 95),
            hashes: hashes(),
            today: 95,
        };
        let got = record("chromium/1", &blames, &facts, &partials, &last);
        assert_eq!(got, report("chromium/1", "aa", "timeout", 90)[FIELD]);
    }

    #[test]
    fn record_should_leave_out_a_file_with_pages_both_engines_finished() {
        let blames = BTreeMap::from([("pdfjs/a.pdf".to_owned(), blamed(false, true))]);
        let facts = serde_json::Map::from_iter([("pdfjs/a.pdf".to_owned(), json!({"outcome": "timeout"}))]);
        let partials = BTreeMap::from([("pdfjs/a.pdf".to_owned(), one_page(true))]);
        let last = LastRun {
            skipped: BTreeMap::new(),
            hashes: hashes(),
            today: 95,
        };
        let got = record("chromium/1", &blames, &facts, &partials, &last);
        assert_eq!(got, json!({"pdfium": "chromium/1", "files": {}}));
    }
}
