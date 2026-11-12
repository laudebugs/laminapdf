//! Check what a PDF holds beyond its visible pages before you publish or send it: earlier
//! revisions, metadata, attachments, scripts, form values and hidden text, and redactions that
//! only covered text instead of removing it.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example check_what_a_file_hides -- <file.pdf>…
//! ```

use std::collections::BTreeSet;
use std::error::Error;

use laminapdf::extract::DropReason;
use laminapdf::reveal::{Category, Reveal};
use laminapdf::{CancelToken, Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        return Err("usage: check_what_a_file_hides <file.pdf>…".into());
    }
    for path in &paths {
        print!("{}", report(path, &what_it_hides(std::fs::read(path)?)?));
    }
    Ok(())
}

/// What a file holds beyond its visible pages.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Hidden {
    /// The kinds of thing it holds, such as `history` or `hidden-content`, in report order.
    pub(crate) categories: Vec<&'static str>,
    /// Each place text was hidden rather than removed.
    pub(crate) redactions: Vec<FailedRedaction>,
}

/// A warning that text was hidden rather than removed: a redaction that only covered it, or text an
/// earlier revision still holds.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FailedRedaction {
    /// Its warning code, such as `redaction-not-applied`.
    pub(crate) code: &'static str,
    /// The 0-based page it is on, when it is on one.
    pub(crate) page: Option<u64>,
}

/// Opens `bytes` and lists what they hold beyond the visible pages.
pub(crate) fn what_it_hides(bytes: Vec<u8>) -> Result<Hidden, laminapdf::Error> {
    let options = OpenOptions::new();
    let doc = Document::open_bytes(bytes, options.clone())?;
    // `reveal` reads every page and every earlier revision; a server would cancel it on a
    // deadline through this token.
    let reveal = doc.reveal(&options, &CancelToken::new())?;
    Ok(Hidden {
        categories: reveal.found().into_iter().map(Category::as_str).collect(),
        redactions: failed_redactions(&reveal),
    })
}

/// The redactions that only hid text, read from what `reveal` found: pages with covered text,
/// pages with redaction marks never applied, then one for each earlier revision holding words
/// the latest lost.
fn failed_redactions(reveal: &Reveal) -> Vec<FailedRedaction> {
    let found = |code, page: Option<usize>| FailedRedaction {
        code,
        page: page.and_then(|page| u64::try_from(page).ok()),
    };
    let covered: BTreeSet<usize> = reveal
        .hidden
        .iter()
        .filter(|run| run.reason == DropReason::Covered)
        .map(|run| run.page)
        .collect();
    let mut out: Vec<FailedRedaction> = covered.into_iter().map(|p| found("covered-text", Some(p))).collect();
    for (page, _marks) in reveal.disclosure.redactions() {
        out.push(found("redaction-not-applied", Some(page)));
    }
    let earlier = reveal.revisions.iter().filter(|r| r.removed_words > 0).count();
    out.extend((0..earlier).map(|_| found("text-in-earlier-revision", None)));
    out
}

/// One line for each thing `found` holds, or one saying there is nothing.
pub(crate) fn report(path: &str, found: &Hidden) -> String {
    let mut lines: Vec<String> = found.categories.iter().map(|c| format!("{path}: {c}\n")).collect();
    for FailedRedaction { code, page } in &found.redactions {
        let place = page.map_or_else(String::new, |page| format!(" on page {}", page + 1));
        lines.push(format!("{path}: {code}{place}\n"));
    }
    if lines.is_empty() {
        lines.push(format!("{path}: nothing beyond its pages\n"));
    }
    lines.concat()
}
