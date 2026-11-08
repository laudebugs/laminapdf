# Plan: place content after the previous element

- **Status:** Draft
- **Date:** 2026-11-07
- **Scope:** A measured bottom position and an optional vertical cursor for PDF generation.
- **Builds on:** [PDF library](pdf-library.md), [coordinate conventions](../decisions/0027-coordinates-units-and-page-indexing.md).

## Goal

Let a caller draw a paragraph, find where it ended, and place the next element below it with a chosen gap. Support Rust and TypeScript with the same coordinate and overflow rules.

Keep this separate from a full layout engine. Tables, floats, automatic pagination and cross-page paragraph continuation remain future work. The existing library plan deliberately defers those features.

## Fit with the current implementation

The builder already returns enough information for text:

| API | Measurement available today | Bottom position |
| --- | --- | --- |
| Rust `Surface::text` | `TextRun::bounds()` | `bounds.y1` |
| Rust `Surface::text_box` | `TextLayout::lines`, each with `rect` and `baseline` | Last drawn line's `rect.y1` |
| TypeScript `Surface.text` | `TextRunInfo.bounds` | `bounds.y + bounds.height` |
| TypeScript `Surface.textBox` | `TextLayout.lines`, each with `rect` and `baseline` | Last drawn line's `rect.y + rect.height` |

TypeScript output rectangles are objects with `x`, `y`, `width` and `height`. Input rectangles can use tuples; their representation must not be confused with returned measurements.

Implementation references:

- [Rust text measurements](../../crates/laminapdf-build/src/surface/text.rs).
- [Rust text layout results](../../crates/laminapdf-build/src/surface/textbox.rs).
- [Surface drawing and transforms](../../crates/laminapdf-build/src/surface/mod.rs).
- [WebAssembly measurement conversion](../../crates/laminapdf-wasm/src/builder/text.rs).
- [TypeScript drawing API](../../packages/sdk/src/builder.ts).

Coordinates start at the page's top left and Y increases downward. Text bounds describe font ascent and descent, rather than the exact painted outline of every glyph. Both measurements are taken before the surface transform: `TextRun::bounds()` says so, and `Surface::text_box` builds each `LineBox::rect` from the requested box without it, although `LineBox` calls its rect "page space".

`Surface` records drawing commands; it has no previous-element cursor. Images and shapes receive placement geometry, but do not return a shared layout result. Paths, strokes, clipping and transforms make a universal painted-bottom calculation a separate problem.

**Recommendation:** add an optional helper over measured rectangles. Keep cursor state outside `Surface` and reuse the existing text layout implementation. No new dependency or PDF serialization change is needed.

## Existing API example

This TypeScript fragment uses the current API. `surface`, `font` and `text` come from the caller; coordinates are points.

```ts
const top = 72;
const gap = 12;
const layout = surface.textBox(text, {
  font,
  size: 12,
  box: [72, top, 450, 600],
});
const lastLine = layout.lines.at(-1);
const bottom = lastLine
  ? lastLine.rect.y + lastLine.rect.height
  : undefined;
const nextY = bottom === undefined ? top : bottom + gap;
```

If `layout.overflow` is not `undefined`, some text did not fit. Test it with `!== undefined`, not for truthiness: the offset is `0` when the box fits no line. The caller must handle that before treating this paragraph as complete. The reported bottom covers only drawn lines, rather than the entire requested text box.

## Proposed contract

Names below are provisional and must be settled before implementation.

### Measured text bottom

Offer a consistent way to obtain an optional bottom from a text result. A result is empty when no line was drawn, or when every drawn line holds only whitespace or line breaks (for a text box, every `LineBox::content` range is empty). Return no position for an empty result; do not interpret its origin rectangle, or a blank line's full-height box, as real content.

For a text box, use the final drawn line's descent edge. For a text run, check its line count before using its bounds. This is a typographic layout measurement, not an ink or stroke bound.

Preserve existing result fields and return types. Evaluate an additive Rust accessor and an exported TypeScript helper; do not silently change the serialized WebAssembly result shape.

### Optional vertical cursor

A small cursor owns a current Y position and accepts a measured bottom plus an explicit nonnegative gap. It does not own the page or borrow the surface between calls.

- Start at a caller-supplied Y position.
- Advance to `max(currentY, bottom + gap)` after a nonempty placement.
- Leave the cursor unchanged for an empty result, including its gap.
- Reject non-finite positions, negative or non-finite gaps, and a non-finite `bottom + gap` before changing state.
- Allow the caller to reset the cursor when starting a page or column.
- Use points in Rust and point-valued numbers in TypeScript.

Text uses its returned measurements. Images and rectangular content use an explicit rectangle supplied by the caller. Arbitrary drawing can opt in with caller-declared layout bounds. Decorative elements need not advance the cursor.

The cursor describes the caller's layout coordinate system before surface transforms. Mixing measurements from different coordinate systems is unsupported. It does not inspect graphics state or calculate transformed painted bounds.

### Overflow and failures

A successful clipped text box may advance through its drawn lines, but its overflow offset must remain visible to the caller. It must never imply that all text was consumed.

A failed drawing call must not advance the cursor. The helper must not add pages, retry text, or discard overflow. Page-break decisions remain explicit. The measurement-only step can land before the cursor.

## Delivery steps

The following ready todos use the [todo template](../todos/TEMPLATE.md). Claim each in its own implementation PR after its dependencies are done. This draft PR records the design and work queue; it does not claim implementation work or change delivery priorities.

- [ ] [B10a](../todos/ready/B10a-measured-text-bottom.md): **Text measurement convenience.** Settle public names and empty-result semantics. Add Rust and TypeScript accessors or helpers, preserving existing APIs, with matching tests. Decide whether the cursor is needed.
- [ ] [B10b](../todos/ready/B10b-vertical-cursor.md): **Vertical cursor.** Implement a reusable cursor with matching Rust and TypeScript behavior, explicit rectangle placement and validation. Export through the Rust facade and SDK public API as appropriate. Reuse existing measurements rather than laying out text again.
- [ ] [B10c](../todos/ready/B10c-element-flow-recipe.md): **Documentation and recipe.** Add the paired examples, including the minimal next-paragraph one. Demonstrate paragraphs followed by an image, empty content, overflow handling and an explicit page reset. Keep examples runnable and align the Rust and TypeScript recipes.

Each implementation PR follows the repository's review, changeset and validation rules. No implementation is included in this plan PR.

## Acceptance and validation

- Single-line and wrapped text report the final descent edge, including mixed font metrics and line-height settings.
- Empty text, whitespace-only text, text of only line breaks and a text box that fits no lines produce no measured bottom and leave the cursor unchanged.
- Clipped text reports the last drawn line and preserves its overflow offset, including TypeScript UTF-16 offsets and an offset of `0` when no line fits.
- A gap is applied once; overlapping or earlier rectangles cannot move the cursor backward.
- Invalid numeric input and failed drawing operations leave cursor state unchanged.
- Image rectangles, caller-declared bounds and explicit page resets behave identically across Rust and TypeScript.
- Transform tests establish that cursor values stay in layout coordinates, without claiming painted page bounds.
- Existing drawing APIs and output remain compatible; the helper requires no additional layout pass.
- Tests use the existing Rust builder and SDK test suites. New public examples follow the [example guidelines](../guidelines/examples.md).

## Decisions before implementation

- Choose accessor/helper names and where the cursor is exported, following the existing public API conventions.
- Confirm whether a measurement helper plus examples solves the immediate need before adding cursor state. B10a records the answer here; B10b starts only if it says the cursor is needed, and is deleted otherwise.
- Choose whether the text-only helper is feature-gated while the rectangle cursor stays available without the text module.
- Keep automatic pagination as a separate proposal. It needs a measure-before-draw or resumable layout contract to avoid drawing before deciding that a page is full.

## Proof and usefulness

The [measurement todo](../todos/ready/B10a-measured-text-bottom.md#tests) establishes numerical correctness with independent font metrics, cross-language cases and a rendered reference page. The [cursor todo](../todos/ready/B10b-vertical-cursor.md#tests) verifies state transitions, invalid inputs and mixed-element placement. The [recipe todo](../todos/ready/B10c-element-flow-recipe.md#tests) proves the complete public workflow by generating, reopening and rendering a report.

The practical benefit is fewer caller-owned Y updates when content length changes. Compare the same report with manual measurement arithmetic and with the helper. Record actual bottom calculations, Y updates and edits needed when a summary wraps or an optional section disappears. Do not claim a speedup: the helper reuses layout results and should require no additional shaping pass.

Success means following content moves to the correct measured position without hand-adjusted coordinates, empty sections leave no extra gap, and overflow stays explicit. PDFs, placement assertions, rendered pages and exact verification commands form the review evidence. This helps invoices and reports assembled from variable text and images; it does not solve tables or automatic page breaking.

### Verification commands for implementation PRs

After installing the repository prerequisites, run these from the implementation worktree. These are future verification steps, not tests already executed for this design PR.

```sh
cargo test --locked -p laminapdf-build --features text
cargo xtask check ts
cargo xtask examples
cargo xtask check docs
cargo xtask ci
```

The TypeScript check builds and tests the package graph, including WebAssembly prerequisites. The example command generates PDFs under `target/examples-out/`, compares registered references and checks them with qpdf. Register the new examples so these checks cover them. Add text-disabled build/test commands if the cursor is exported without the text feature.

Attach or link the generated report variants and rendered pages in each implementation PR, together with the numerical assertions and command results. The design PR itself only runs todo consistency, documentation links, prose and whitespace checks.
