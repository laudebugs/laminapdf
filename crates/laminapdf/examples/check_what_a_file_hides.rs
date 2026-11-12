//! Check what a PDF holds beyond its visible pages before you publish or send it: earlier
//! revisions, metadata, attachments, scripts, form values and hidden text, and redactions that
//! only covered text instead of removing it.
//!
//! ```sh
//! cargo run -p laminapdf --example check_what_a_file_hides -- <file.pdf>…
//! ```

use std::error::Error;

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
    /// Each failed redaction's warning code, with the 0-based page it is on, if it has one.
    pub(crate) redactions: Vec<(&'static str, Option<u64>)>,
}

/// Opens `bytes` and lists what they hold beyond the visible pages.
pub(crate) fn what_it_hides(bytes: Vec<u8>) -> Result<Hidden, laminapdf::Error> {
    let options = OpenOptions::new();
    let doc = Document::open_bytes(bytes, options.clone())?;
    // Both calls read every page and every earlier revision; a server would cancel them on a
    // deadline through this token.
    let cancel = CancelToken::new();
    let reveal = doc.reveal(&options, &cancel)?;
    let redactions = doc.redaction_warnings(&options, &cancel)?;
    Ok(Hidden {
        categories: reveal.found().into_iter().map(laminapdf::reveal::Category::as_str).collect(),
        redactions: redactions
            .iter()
            .map(|warning| (warning.code(), warning.location().page_index))
            .collect(),
    })
}

/// One line for each thing `found` holds, or one saying there is nothing.
pub(crate) fn report(path: &str, found: &Hidden) -> String {
    let mut lines: Vec<String> = found.categories.iter().map(|c| format!("{path}: {c}\n")).collect();
    for (code, page) in &found.redactions {
        let place = page.map_or_else(String::new, |page| format!(" on page {}", page + 1));
        lines.push(format!("{path}: {code}{place}\n"));
    }
    if lines.is_empty() {
        lines.push(format!("{path}: nothing beyond its pages\n"));
    }
    lines.concat()
}
