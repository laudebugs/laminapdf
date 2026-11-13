---
title: "Check what a file hides"
description: "Find what a PDF holds beyond its visible pages, from earlier revisions to hidden text, and redactions that only covered text instead of removing it."
version: "0.2.0"
url: "https://docs.laminapdf.com/guides/check-what-a-file-hides/"
---

# Check what a file hides

Find what a PDF holds beyond its visible pages, from earlier revisions to hidden text, and redactions that only covered text instead of removing it.

You'll check a PDF for what its reader can't see before you publish it, send it or accept it as redacted. A PDF keeps more than its pages show: every earlier revision's text, who made it, attached files, scripts, and text drawn invisible or under a black box.

This example lists what each file holds, and every redaction that only covered its text:

**Rust**

`crates/laminapdf/examples/check_what_a_file_hides.rs`:

````rust
//! Check what a PDF holds beyond its visible pages before you publish or send it: earlier
//! revisions, metadata, attachments, scripts, form values and hidden text, and redactions that
//! only covered text instead of removing it.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example check_what_a_file_hides -- <file.pdf>…
//! ```

use std::error::Error;

use laminapdf::reveal::Category;
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
        // The same warnings `Document::redaction_warnings` gives, read from what `reveal` found.
        redactions: reveal
            .redaction_warnings()
            .iter()
            .map(|warning| FailedRedaction {
                code: warning.code(),
                page: warning.location().page_index,
            })
            .collect(),
    })
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
````

**Output**

`fixtures/reveal/examples/check_what_a_file_hides.txt`:

```text
letter.pdf: history
letter.pdf: text-in-earlier-revision
brochure.pdf: metadata
brochure.pdf: hidden-content
```

The letter's latest revision deleted a word, but its first revision still holds it, and anyone can read that revision. The brochure has metadata and invisible text on its first page.

## What `reveal` looks for

`Document::reveal` reads every object once and sorts what it finds into seven categories. `Reveal::found` lists the ones the file has:

| Category | What it means |
| --- | --- |
| `history` | Earlier revisions, and objects nothing refers to any more |
| `metadata` | The title, author and dates in the file's two metadata stores (its document information and its XMP metadata), and metadata on single objects |
| `attachments` | Embedded files |
| `active-content` | JavaScript, actions that run on a trigger, and XFA forms |
| `annotations` | Comments and markup, with their authors |
| `forms` | Form fields that hold values |
| `hidden-content` | Hidden text, layers turned off, and redaction marks never applied |

The `Reveal` it returns also holds the details: each Info entry, each attachment's name and size, and each run of hidden text. For each earlier revision, it counts the words that revision holds and the latest one lost, and names the pages they were on.

## Redactions that didn't remove the text

A black rectangle over a name hides it on screen and in print, but the name is still in the file, and copying the text brings it back. `Document::redaction_warnings` finds three ways a redaction fails:

| Warning | What it found |
| --- | --- |
| [`covered-text`](../../errors/covered-text.md) | Text under an opaque box on the page |
| [`redaction-not-applied`](../../errors/redaction-not-applied.md) | Redaction marks that were drawn but never applied |
| [`text-in-earlier-revision`](../../errors/text-in-earlier-revision.md) | Words an earlier revision holds that the latest doesn't |

The example reads these warnings from the `Reveal` with `Reveal::redaction_warnings`, which gives the same warnings without reading the file again. `Document::redaction_warnings` would read every page and every earlier revision a second time, so call it only when the warnings are all you need.

Each warning names its page when it has one. Remove the text itself with a tool that deletes what a redaction covers, then save a [full rewrite](../../pages/reference/glossary.md#full-rewrite), so no earlier revision keeps it.

## From TypeScript and the command line

`@laminapdf/sdk` doesn't have `reveal` or the redaction warnings yet. The command line runs the same checks: `laminapdf reveal` reports each category, and `laminapdf check` reports failed redactions with its other warnings. [Inspect and check PDFs](../../pages/cli/inspect.md) shows both.

Next: [open files you don't trust](../../pages/guides/untrusted-files.md), with limits on the work an uploaded file can make Lamina PDF do.
