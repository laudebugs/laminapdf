# A tab in a code block raises `tounicode-conflict` with a CFF font

Markdown with a tab in a code block raised one `tounicode-conflict` warning per tab when the code font had CFF outlines. The builder drew each tab as a space, so the text it recorded disagreed with what it drew. The fix is in [resolution.md](resolution.md).

## What the user saw

The docs site's guide PDF (`apps/docs/scripts/guide-pdf.mjs`) builds every page's Markdown with `builder.markdown`, with Source Code Pro (`.otf`) as the mono font. It failed with 8 `tounicode-conflict` warnings. The extraction guide's text example prints `fixtures/extract/examples/invoice.txt`, whose table cells are separated by tabs, inside a code block. [laudebugs/laminapdf#461](https://github.com/laudebugs/laminapdf/pull/461) worked around it with `expandTabs` in `apps/docs/plugins/guide.mjs`, which turns tabs into spaces before the build.

## How to reproduce it

Build this Markdown with a CFF font in every role, such as `fixtures/fonts/NotoSansSC-Regular.otf`, and collect the font warnings with `Builder::finish_with_warnings`:

````markdown
```
a b
c	d
```
````

Before the fix it raised two warnings, `ToUnicodeConflict { gid: 1, kept: " ", dropped: "\t" }`: one for the tab's glyph, one for its occurrence in the code element's marked content. The same tab in a paragraph raised the same warning. The extracted text kept the tab, so `name\tqty` came back as `name	qty`, with no columns to line up under.

## The cause

A font has no glyph for a tab. The shaper (`stand_in` in `crates/laminapdf-text/src/shape/engine.rs`) shapes a tab as a space and keeps the cluster pointing at the tab, so `pdf_run` records the space glyph as standing for `"\t"`. With TrueType outlines the font gives the space glyph a second CID for the tab's text, so nothing warns. A font with CFF outlines (`CIDFontType0`) has one CID per glyph, so the space glyph keeps its first text and the second one is a `tounicode-conflict` (ADR 0005 §6, `FontUse::use_glyph`).

The flowing layout (`crates/laminapdf-build/src/flow/`) passed code-block lines and paragraph text to the shaper as written. Markdown keeps tabs in code as they are: CommonMark expands tabs only where they define block structure, with tab stops every four columns (spec §2.2), and pulldown-cmark leaves the rest of a code block's tabs in its text. JSON documents' `codeBlock` text reaches the same code.
