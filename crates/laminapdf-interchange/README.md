# laminapdf-interchange

`laminapdf-interchange` moves a PDF's comments between apps. It reads and writes XFDF (ISO 19444-1) and FDF (ISO 32000-2 §12.7.8), the formats Acrobat and most PDF apps export comments in, and writes Markdown for notes apps. An import is applied as one transaction of ordinary edits, so a single undo takes it back.

It is part of Lamina PDF's pure core: bytes in, bytes out, with no I/O, clocks or threads, so it runs anywhere Rust or WebAssembly does.

```rust
use laminapdf_interchange::{ImportMode, ImportCommentsOptions, import, read};

let exchange = read(&xfdf_bytes, &laminapdf_syntax::Limits::default())?;
println!("{} comments, made for {:?}", exchange.comments.len(), exchange.file);
let (summary, _applied) = import(&mut doc, &exchange, &ImportCommentsOptions::new(ImportMode::Merge))?;
println!("added {}, kept {}", summary.added, summary.matched);
```

Most programs use it through the `laminapdf` crate instead: `Document::export_annotations`, `Document::preview_import` and `Document::import_annotations`.

## How it reads a file

An exchange file is read into an `Exchange`: the PDF it was made for, and each comment as its annotation dictionary's entries. XFDF itself is defined in terms of those keys, so XFDF and FDF share one model, and one table maps XFDF's attributes to PDF keys in both directions.

- **XFDF** is read with the guarded XML reader in `laminapdf_model::xml`. It refuses a DTD, which could define entities that expand without end or read other files, and every entity but XML's five. Depth, attributes and namespace declarations are bounded.
- **FDF** is PDF syntax. It is read by scanning for object headers with `laminapdf-syntax`'s tolerant parser, the way a damaged PDF is recovered, so a missing or wrong cross-reference table doesn't matter.

A file that doesn't read gives an error naming the line where reading stopped.

## How it imports

A plan is made first, without changing the document; the preview reports it and the import applies it, so the two always agree. Comments are added in their file's order, each after the comment it answers, so a reply can name a comment the same import made a moment before. Each comment gets an appearance drawn by Lamina PDF, and keeps the Popup the file places.

The modes decide what happens to the comments the file names (by `/NM`, on the same page) that are already in the document. Comments without a `/NM` are matched by the ID an export names them with.

- **Merge** (the default) keeps them as they are, and adds the file's other comments, so new replies and statuses join their threads.
- **Replace** deletes each, with its replies, and adds the file's version.
- **Add** keeps them and adds the file's as copies, renaming a copy whose `/NM` is taken.

The import leaves out what it can't make, and says why: sounds, links, redactions, kinds another app invented, stamps drawn from another app's picture, form fields, comments for pages the document doesn't have, and replies whose comment is missing. Two comments sharing a `/NM` on one page are reported. A file made for another PDF, by its `/ID` or else its name, gives a warning before anything is applied.

## Where files from other apps differ

What each app writes into XFDF varies. These differences are known, and handled as described:

- **Replies Lamina PDF makes** always write `/RT /R` and a modification date, which some apps leave out. A file round-tripped once keeps the same bytes from then on.
- **Two replies that answer each other** (a ring, which some files hold by mistake) can't be rebuilt, since each needs the other to exist first. Both are skipped, with a warning.
- **A reply to a comment's Popup** is exported as a reply to the comment, since Popups aren't exported.
- **Comments without a `/NM`** are exported with their ID as the name, so their replies can name them.
- **Appearances** in the file (`<appearance>`) are not imported: Lamina PDF draws its own, as it does for every comment it makes.
- **What the edit ops don't carry** is left out: a note's or a mark's rich text (`/RC`; a text box keeps its own), `/IT` on markup, `/RD`, and a reply's subject, colour and flags.
- **A reply that isn't a note** (a highlight with `/IRT` and `/RT /R`) is imported as a comment of its own, since a reply Lamina PDF makes is a note.
- **An attached file's dates** are written into XFDF and FDF only when the file carries them; exporting a document's attachments writes none yet.
- **A file's rich text** (`<contents-richtext>`) is written without its `<?xml …?>` declaration, which may only start a document.

Files that Acrobat exports will be added to this list as they are tested.

## License

MIT OR Apache-2.0
