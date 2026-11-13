# Spike: `content-edit` (E7a)

- **Date:** 2026-11-13
- **Status:** done. Its recommendation is recorded in [ADR 0022](../../docs/decisions/0022-content-stream-editing.md), proposed and waiting for the owner's sign-off.
- **Todo:** [E7a](../../docs/todos/ongoing/E7a-adr-0022-content-editing.md); plan [full editor §E7](../../docs/plans/full-editor.md#e7-editing-existing-content)
- **Code:** throwaway quality, outside both workspaces, never imported by product code ([code organisation §12](../../docs/guidelines/code-organization.md#12-spikes-vs-product-code)).

## Question

When Lamina PDF edits an existing page's content, should it write the whole page's operators again, or copy the operators an edit didn't touch byte for byte? And what does an editable list of operators, grouped into the objects an editor selects, cost to build?

## Answer in one paragraph

**Copy what the edit didn't touch.** Moving one object and copying the rest kept 84 % of the corpus's content bytes as they were, and 5,643 of 5,655 edited pages read back as the edit meant. Writing every operator again changed numbers on 468 of 5,967 pages, because Lamina PDF's writer rounds reals to six decimals and writes `2.0` as `2`, and made the content 6.5 % larger. The editable list itself is cheap: tokenizing and grouping ran at 67 MB/s, and the slowest page took 73 ms. The one flaw the spike found is its own: it guessed where an inline image ends from the image data, and that guess caused 30 of its 33 mismatches. The tokenizer has to report the end itself.

## Method

One standalone Cargo project with path dependencies on `laminapdf-cos`, `laminapdf-content` and `laminapdf-syntax`. For every page of the PDFs it is given, `src/main.rs`:

1. reads the page's decoded content (`Document::page_content`: the `/Contents` streams joined);
2. tokenizes it with `ContentIter` and records each operator's span: from the end of the previous operator through its own keyword, or through `EI` for an inline image;
3. groups the operators into objects: a text object (`BT … ET`), a path (construction through its painting operator), an image or form (`Do`), an inline image, a shading (`sh`);
4. **full re-serialise:** writes every operator again with `Object::write_to` and tokenizes the result, counting pages that read back identical and pages equal within 10⁻⁶ per number;
5. **span-preserving:** moves one object (the first text object, else the first object) by writing its operators again inside `q 1 0 0 1 10 0 cm … Q`, copies every other byte, and checks the result reads back as that edit.

It doesn't render anything: the comparison is between operator lists. E7j's edit corpus compares renders.

```sh
cargo xtask corpus fetch                     # the pinned external corpus, into target/corpus/
cd spikes/content-edit && cargo build --release
find ../../target/corpus/pdfjs -name '*.pdf' -print0 | xargs -0 target/release/lamina-content-edit-spike
SPIKE_DEBUG=1 …                              # also prints each mismatch on stderr
```

## Results

Apple M1 Pro, release build, the corpus at the commits `fixtures/corpus-manifest.toml` pins. Files Lamina PDF can't open (103 of 2,161, mostly deliberately broken ones) are left out.

| Source | Pages | Content | Operators | Objects (text) | Tokenize | Re-serialise: size | Identical | Within 10⁻⁶ | Verbatim after one edit | Edit reads back |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdf.js | 1,829 | 18.58 MB | 1,516,062 | 203,165 (40,734) | 59 MB/s | 102.8 % | 1,605 | 1,821 | 81.6 % | 1,696 / 1,698 |
| PDFBox | 307 | 2.65 MB | 180,442 | 65,842 (2,110) | 75 MB/s | 102.6 % | 260 | 306 | 83.2 % | 259 / 260 |
| PDFium | 351 | 0.16 MB | 12,117 | 1,696 (408) | 60 MB/s | 86.3 % | 333 | 351 | 78.7 % | 241 / 241 |
| qpdf | 3,480 | 4.63 MB | 135,752 | 19,746 (4,950) | 133 MB/s | 124.2 % | 3,301 | 3,468 | 96.3 % | 3,447 / 3,456 |
| **All** | **5,967** | **26.02 MB** | **1,844,373** | **290,449 (48,202)** | **67 MB/s** | **106.5 %** | **5,499** | **5,946** | **84.3 %** | **5,643 / 5,655** |
| Lamina PDF's fixtures | 110 | 0.47 MB | 16,667 | 1,915 (1,586) | 94 MB/s | 71.0 % | 99 | 110 | 97.4 % | 106 / 106 |

The slowest page tokenized and grouped in 73 ms (pdf.js corpus); writing every operator again took 267 ms for all 26 MB. "Edit reads back" counts numbers within 10⁻⁶, since the moved object is itself written again. Pages with no object to move (312) aren't counted there.

### What the mismatches were

- **Numbers (447 pages).** Every re-serialise difference within 10⁻⁶ comes from `format_real`: six decimals at most, and no `.0` on whole numbers. A page re-read after it holds the same values to within 5 × 10⁻⁷.
- **Inline images (30 of the 33 others).** The spike finds an inline image's end by searching for `EI` after the data and its start by searching back for `BI`. On binary data both searches can land in the wrong place. `InlineImage` reports where the data starts but not where `EI` ends, so the IR needs `Op` to carry that offset.
- **Three it didn't explain.** Two edited text objects that round-trip correctly on their own failed in their page, and one re-serialised page gained four operators. The ADR leaves them to E7c's tests and the `content` fuzz target.

## Recommendation

Build `ContentOps` with a span per operator and copy unchanged spans on encode ([ADR 0022 §1 and §3](../../docs/decisions/0022-content-stream-editing.md#decision)). Add the inline image's end offset to `Op` with it.
