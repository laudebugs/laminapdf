//! Checks what a PDF hides before you send it: redactions that left the text in the file, and
//! what it holds beyond its visible pages, such as earlier versions of itself, attachments,
//! scripts and hidden text.
//!
//! It prints each finding and exits 1 when the file holds any of the categories you refuse.
//!
//! ```sh
//! cargo run -p laminapdf --example check_what_a_file_hides -- <file.pdf>
//! ```

use std::error::Error;

use laminapdf::reveal::Category;
use laminapdf::{CancelToken, Document, OpenOptions, RedactionWarning, Warning};

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
    let names: Vec<&str> = findings.categories.iter().map(|c| c.as_str()).collect();
    println!("holds: {}", names.join(", "));
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

/// Opens `bytes` and lists its failed redactions and what it holds.
pub(crate) fn check(bytes: Vec<u8>) -> Result<Findings, laminapdf::Error> {
    // Earlier versions of the file open with the same options: its password and limits.
    let options = OpenOptions::new();
    let doc = Document::open_bytes(bytes, options.clone())?;
    // A server would cancel this token when the request it serves goes away.
    let cancel = CancelToken::new();
    let redactions = doc
        .redaction_warnings(&options, &cancel)?
        .iter()
        .map(describe)
        .collect();
    let categories = doc.reveal(&options, &cancel)?.found();
    Ok(Findings { redactions, categories })
}

/// A line about one failed redaction. Pages count from 1 here, as a reader numbers them.
fn describe(warning: &Warning) -> String {
    match warning {
        Warning::Redaction(RedactionWarning::CoveredText { page, chars, .. }) => {
            let text = count(*chars, "character");
            format!("page {}: {text} under a box, still in the file", page + 1)
        }
        Warning::Redaction(RedactionWarning::RedactionNotApplied { page, annotations }) => {
            let marks = count(u64::from(*annotations), "redaction mark");
            format!("page {}: {marks} never applied", page + 1)
        }
        Warning::Redaction(RedactionWarning::TextInEarlierRevision { revision, words, .. }) => {
            let words = count(*words, "word");
            format!("revision {revision}: {words} the latest revision doesn't hold")
        }
        // A code added in a later version: its page on the docs site says what it means.
        other => format!("{}: {}", other.code(), other.docs_url()),
    }
}

/// `n` and `noun`, plural unless `n` is 1.
fn count(n: u64, noun: &str) -> String {
    let s = if n == 1 { "" } else { "s" };
    format!("{n} {noun}{s}")
}
