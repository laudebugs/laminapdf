---
title: "Edit the text a page shows"
description: "List the runs of text a PDF page shows, then change or delete one in the font it is set in, with the text after it moved to fit."
version: "0.8.0"
url: "https://docs.laminapdf.com/recipes/edit-text/"
---

# Edit the text a page shows

List the runs of text a PDF page shows, then change or delete one in the font it is set in, with the text after it moved to fit.

You'll change one word a page already shows: "world" becomes "Lamina PDF". Use it to fix a typo, update a date or a price, or remove a line before you send a file on, without rebuilding the page.

First, the program lists the page's runs of text, each with its line, font and, when it can't be edited, the reason. It then picks the first run that says "world" and gives it new text:

**TypeScript**

`packages/sdk/examples/edit-text.ts`:

```ts
// Change the text a page already shows, in the font it is set in.
//
// Lists the first page's runs of text, then rewrites the first one that says "world" to say
// "Lamina PDF" instead. The new text uses the font the run was set in, so every character must
// be one that font can draw; a character it can't is refused with `missing-glyph`, and a run
// that can't be edited at all (a Type 3 font, vertical text) says why. The text after the run
// moves by the difference in width; the lines below stay where they were.
import { Document } from "@laminapdf/sdk";

/** The document with "world" rewritten on its first page. Needs `init({ extract: true })`. */
export function editText(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const runs = doc.content.textRuns(0);
  for (const run of runs) {
    const readOnly = run.readOnly ? ` (read-only: ${run.readOnly})` : "";
    console.log(`line ${run.line}: ${JSON.stringify(run.text)} in ${run.fontName} ${run.fontSize}${readOnly}`);
  }
  const run = runs.find((r) => !r.readOnly && r.text.includes("world"));
  if (!run) {
    throw new Error('no editable run says "world"');
  }
  doc.content.editText(0, run.run, run.text.replace("world", "Lamina PDF"));
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/edit_text.rs`:

````rust
//! Change the text a page already shows, in the font it is set in.
//!
//! Lists the first page's runs of text, then rewrites the first one that says "world" to say
//! "Lamina PDF" instead. The new text uses the font the run was set in, so every character must
//! be one that font can draw; a character it can't is refused with `missing-glyph`, and a run
//! that can't be edited at all (a Type 3 font, vertical text) says why. The text after the run
//! moves by the difference in width; the lines below stay where they were.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_text -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{ContentTextRun, Document, EditText, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: edit_text in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?)?;
    std::fs::write(out.join("edit_text.pdf"), edited)?;
    Ok(())
}

/// The document with "world" rewritten on its first page.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let runs = doc.text_runs(0)?;
    list(&runs);
    let run = runs
        .iter()
        .find(|r| r.read_only.is_none() && r.text.contains("world"))
        .ok_or("no editable run says \"world\"")?;
    let page = doc.page_info(0)?.id;
    let text = run.text.replace("world", "Lamina PDF");
    doc.apply(vec![EditText::new(page, run.run, text).into()])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// Prints each run: its line, text, font and, when it can't be edited, why.
fn list(runs: &[ContentTextRun]) {
    for run in runs {
        let read_only = match run.read_only {
            Some(reason) => format!(" (read-only: {})", reason.as_str()),
            None => String::new(),
        };
        println!(
            "line {}: {:?} in {} {}{read_only}",
            run.line, run.text, run.font_name, run.font_size
        );
    }
}
````

**Result:** [edit_text.pdf](https://docs.laminapdf.com/examples/pdf/edit_text.pdf)

In the result, the greeting reads "Hello, Lamina PDF!" in the same font and size. Text extraction, copy and search find the new words, since the page now shows them.

## How it works

- `content.textRuns(index)` (`text_runs` in Rust) lists the runs in the order the page draws them. A page draws its text in text objects, each a group of instructions that show glyphs. A run is the text one of those instructions shows in one font and size on one baseline; on a tagged page, a run also stays inside one tagged element. Its `text` has a space wherever the glyphs leave a word-sized gap.
- An edit names a run by its text object and its glyphs within that object. Once an edit changes the object, a name taken before that edit fails with `content-object-stale`, so an edit never lands on the wrong text.
- The new text uses the run's own font. Every character must be one the font can draw: for an embedded subset, a character the document already shows in that font; for a font that isn't embedded, any character of its encoding. Any other character fails with `missing-glyph`, and nothing changes.
- When the text grows or shrinks, the text after it in the same text object moves by the difference, and the lines below stay where they were. Another text object can place more text on the same line. That text stays put, so longer text can run into it: check the result.
- A space the font has no glyph for becomes a word-sized gap, as typesetting programs write it.
- **Undo:** one `undo()` reverts the edit.

## What to change

- **Delete a run:** give it empty text.
- **Change part of a run:** name a smaller range of its glyphs, `start` to `end`. For most Western text, one character is one glyph.
- **Read-only runs:** a run can't be edited when its font is a Type 3 font, writes vertically or has no map from its codes to text. Text that stands in for other text (`ActualText`), runs right to left, or has glyphs turned or written out of order can't be edited either. Each such run's `readOnly` says why, and editing it fails with `content-not-editable`.
- **Private content:** the old text stays in the file's earlier revisions until a full save. The edit's `stillInEarlierRevisions` says so; save with `mode: "fullRewrite"` to drop it.

Next: [edit what a page draws](../../pages/recipes/edit-objects.md), to move or delete whole objects instead.
