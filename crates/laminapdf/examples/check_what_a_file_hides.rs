//! Checks what a PDF hides before you send it: redactions that left the text in the file, and
//! what it holds beyond its visible pages, such as earlier versions of itself, attachments,
//! scripts and hidden text. It prints each finding, and exits 1 when the file holds any of the
//! categories you refuse.
//!
//! ```sh
//! cargo run -p laminapdf --example check_what_a_file_hides -- <file.pdf>
//! ```

use std::collections::BTreeMap;
use std::error::Error;

use laminapdf::extract::DropReason;
use laminapdf::reveal::{Category, Reveal};
use laminapdf::{CancelToken, Document, OpenOptions};

/// What a file must not hold when it leaves: its earlier versions, scripts, attachments and text
/// a reader can't see. Most files carry a title or an author, so metadata isn't refused here.
const REFUSED: [Category; 4] = [
    Category::History,
    Category::ActiveContent,
    Category::Attachments,
    Category::HiddenContent,
];

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args()
        .nth(1)
        .ok_or("usage: check_what_a_file_hides <file.pdf>")?;
    let findings = check(std::fs::read(input)?)?;
    for line in &findings.redactions {
        println!("{line}");
    }
    if !findings.categories.is_empty() {
        let names: Vec<&str> = findings.categories.iter().map(|c| c.as_str()).collect();
        println!("holds: {}", names.join(", "));
    }
    if findings.categories.iter().any(|c| REFUSED.contains(c)) || !findings.redactions.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

/// What [`check`] finds.
pub(crate) struct Findings {
    /// One line per failed redaction, without the text it failed to hide.
    pub(crate) redactions: Vec<String>,
    /// The categories the file holds beyond its pages.
    pub(crate) categories: Vec<Category>,
}

/// Opens `bytes` and lists its failed redactions and what it holds, reading the file once.
pub(crate) fn check(bytes: Vec<u8>) -> Result<Findings, laminapdf::Error> {
    // Earlier versions of the file open with the same options: its password and limits.
    let options = OpenOptions::new();
    let doc = Document::open_bytes(bytes, options.clone())?;
    // A server would cancel this token when the request it serves goes away.
    let reveal = doc.reveal(&options, &CancelToken::new())?;
    Ok(Findings {
        redactions: failed_redactions(&reveal),
        categories: reveal.found(),
    })
}

/// A line per failed redaction. Pages count from 1 here, as a reader numbers them.
fn failed_redactions(reveal: &Reveal) -> Vec<String> {
    let mut covered: BTreeMap<usize, u64> = BTreeMap::new();
    for run in reveal.hidden.iter().filter(|run| run.reason == DropReason::Covered) {
        *covered.entry(run.page).or_default() += run.chars as u64;
    }
    let mut lines: Vec<String> = covered
        .into_iter()
        .map(|(page, chars)| {
            let text = count(chars, "character");
            format!("page {}: {text} under a box, still in the file", page + 1)
        })
        .collect();
    for (page, marks) in reveal.disclosure.redactions() {
        lines.push(format!(
            "page {}: {} never applied",
            page + 1,
            count(u64::from(marks), "redaction mark")
        ));
    }
    for (index, revision) in reveal.revisions.iter().enumerate() {
        if revision.removed_words > 0 {
            let words = count(revision.removed_words, "word");
            lines.push(format!("revision {index}: {words} the latest revision doesn't hold"));
        }
    }
    lines
}

/// `n` and `noun`, plural unless `n` is 1.
fn count(n: u64, noun: &str) -> String {
    let s = if n == 1 { "" } else { "s" };
    format!("{n} {noun}{s}")
}
