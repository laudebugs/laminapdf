---
title: "Print a comment summary"
description: "Make a new PDF that lists every comment on a reviewed document, with each page drawn small beside its comments and their numbers marked on it."
version: "0.7.0"
url: "https://docs.laminapdf.com/recipes/comment-summary/"
---

# Print a comment summary

Make a new PDF that lists every comment on a reviewed document, with each page drawn small beside its comments and their numbers marked on it.

You'll make a summary of a review: a new PDF that lists every comment, with each page drawn small beside its comments and their numbers marked on it. You need one to send a review to someone who shouldn't have to open each comment, or to print it.

`commentSummary` reads the comments and lays the summary out in the font you give it. Each comment comes with its number, author, date, status, the text it marks, its note and its replies. Here it summarises the review answered in [Answer a review](../../pages/recipes/review-comments.md):

**TypeScript**

`packages/sdk/examples/comment-summary.ts`:

```ts
// Print a summary of a review: a new PDF that lists every comment, with each page drawn small
// beside its comments and their numbers marked on it.
// The summary is tagged and conforms to PDF/UA-1, so a screen reader reads it in order. The
// reviewed file doesn't change.
import { Document, Font } from "@laminapdf/sdk";

/**
 * A summary of the comments on `bytes`, set in the font `font`, which the summary embeds. Needs
 * `init({ text: true })`.
 */
export function summariseComments(bytes: Uint8Array, font: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  using summary = doc.commentSummary({ font: Font.fromBytes(font) });
  // A page whose fonts the file doesn't embed is listed without its drawing.
  for (const warning of summary.warnings) {
    console.warn(warning.code);
  }
  return summary.save();
}
```

**Rust**

`crates/laminapdf/examples/comment_summary.rs`:

````rust
//! Print a summary of a review: a new PDF that lists every comment, with each page drawn small
//! beside its comments and their numbers marked on it.
//!
//! The summary is tagged and conforms to PDF/UA-1, so a screen reader reads it in order. The
//! reviewed file doesn't change.
//!
//! ```sh
//! cargo run -p laminapdf --example comment_summary -- reviewed.pdf font.ttf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, Font, OpenOptions, SaveOptions, SummaryOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: comment_summary reviewed.pdf font.ttf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let font = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let summary = summarise(std::fs::read(input)?, std::fs::read(font)?)?;
    std::fs::write(out.join("comment_summary.pdf"), summary)?;
    Ok(())
}

/// A summary of the comments on `bytes`, set in the font `font`, which the summary embeds.
pub(crate) fn summarise(bytes: Vec<u8>, font: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let mut summary = doc.comment_summary(&SummaryOptions::new(Font::from_bytes(font)?))?;
    // A page whose fonts the file doesn't embed is listed without its drawing.
    let mut warnings = summary.warnings();
    for warning in warnings.next_batch(warnings.remaining()) {
        eprintln!("{}", warning.code());
    }
    Ok(summary.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [comment_summary.pdf](https://docs.laminapdf.com/examples/pdf/comment_summary.pdf)

The reviewed file doesn't change. The summary is a document of its own, unsaved until you save it.

## How it works

- Comments are numbered in the order of the document: page by page, and on each page in the order the file lists them. The same numbers mark the drawn page, so a reader can match a mark to its comment.
- Each drawn page is a copy of the page itself, not a picture of it. It stays sharp at any zoom, and its text can be selected and searched.
- The copy shows what the page itself draws. Annotations and form fields aren't part of it, so a filled form's fields show blank; the numbers mark where the comments are. A layer hidden when the document opens stays hidden: what the page draws on it is left out, and drawings the page reuses from elsewhere in the file keep their layers, hidden as before.
- The summary is tagged and conforms to PDF/UA-1, so a screen reader reads it in order. Each drawn page has a description that names the page and the comments it marks.
- PDF/UA-1 needs every font embedded. A page set in a font the file only names, such as Helvetica, is listed without its drawing, and so is a page whose content can't be read to its end within the document's limits. The summary's warnings say so with `summary-page-not-drawn`.
- Replies are listed under their comment, and a reply on another page still belongs to its comment. The status is the comment's latest review status, with "Marked" when someone checked it.

## What to change

- **A list without pages:** pass `layout: "list"` (in Rust, `SummaryLayout::List`), which suits a long review.
- **Some comments only:** pass `comments` with the IDs from `commentThreads`, such as those a comments panel shows.
- **Less detail:** pass `replies: false` or `status: false`.
- **Every page:** pass `emptyPages: true` to list the pages without comments too, each saying it has none.
- **Paper:** the summary takes the size of the document's first page; pass `paper: [595.28, 841.89]` for A4, in points.
- **Title:** pass `title`. Without one, the summary is titled "Comments on" and the document's title.
- **Language:** pass `language` with the language most comments are in, so screen readers read them in that language. Pass `words` too, so the summary's own words are in it: `words: { page: "Seite {label}", kinds: { Text: "Notiz" } }` (in Rust, `SummaryOptions::with_words` with a `SummaryWords`). Each word is a template: the summary fills in the names in braces, such as `{label}`. Any word you leave out stays in English. Dates stay in ISO 8601.

Next: [flatten comments](../../pages/recipes/flatten-comments.md) to draw them into the pages instead.
