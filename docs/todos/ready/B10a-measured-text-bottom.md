# B10a — Expose measured text bottoms

| Field | Value |
| --- | --- |
| ID | `B10a` |
| Status | `ready` |
| Plan | [Element flow](../../plans/element-flow.md#delivery-steps) |
| Round / milestone | Round B10 — measured placement helpers |
| Depends on | [B5b](../done/B5b-builder-text-and-document-features.md), [B6b](../done/B6b-lamina-pdf-package.md) |
| ADRs | Follows [0016](../../decisions/0016-package-design.md), [0018](../../decisions/0018-test-strategy.md) and [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `feat/b10a-measured-text-bottom` |
| PR title | `feat(build): expose measured text bottoms` |
| Size | M (200–400 lines; split bindings if the implementation grows) |
| PR | — |

## Goal

Expose an optional bottom position for text runs and text boxes in Rust and TypeScript. Callers can position subsequent content using actual drawn lines without repeating rectangle arithmetic.

## Context

- [Element flow plan](../../plans/element-flow.md#proposed-contract): the measured-bottom contract, including what counts as an empty result.
- [`TextRun::bounds()`](../../../crates/laminapdf-build/src/surface/text.rs) and [`TextLayout`](../../../crates/laminapdf-build/src/surface/textbox.rs): the measurements to reuse; both are taken before the surface transform.
- [WebAssembly text results](../../../crates/laminapdf-wasm/src/builder/text.rs) and [the SDK's `Surface`](../../../packages/sdk/src/builder.ts): where the TypeScript helper reads the same values, with overflow as UTF-16 offsets.
- [ADR 0027](../../decisions/0027-coordinates-units-and-page-indexing.md): top-left page coordinates in points.

## Scope

**In scope**

- An additive Rust accessor for the optional bottom of a `TextRun` and a `TextLayout`, exported through the `laminapdf` façade.
- The matching exported TypeScript helper, reading the existing result objects.
- The empty-result rule: no drawn line, or only blank lines.
- The decision whether B10b's cursor is needed, recorded in the plan.

**Out of scope** (and which todo covers it)

- The vertical cursor: [B10b](./B10b-vertical-cursor.md).
- Runnable examples and the docs recipe: [B10c](./B10c-element-flow-recipe.md).
- Automatic pagination, tables, floats and transformed ink bounds: no todo yet; they need a separate proposal ([plan](../../plans/element-flow.md#decisions-before-implementation)).

## Tasks

- [ ] Settle additive accessor/helper names and document empty-result behavior before implementation.
- [ ] Decide, from the measurement helper alone, whether callers still need the cursor, and record the answer in the plan's "Decisions before implementation". If they don't, delete B10b and drop it from B10c's **Depends on**.
- [ ] Reuse `TextRun::bounds()` and `TextLayout::lines` in `crates/laminapdf-build/src/surface/`; keep existing fields and return types.
- [ ] Export through the Rust façade and the SDK; inspect the WebAssembly DTO boundary without changing its serialized shape unnecessarily.
- [ ] Preserve overflow offsets and pre-transform coordinate semantics.
- [ ] Add public API documentation, doc tests that place a next paragraph, and the required changeset. The runnable examples are B10c's.

## Acceptance criteria

- [ ] Single-line and wrapped results equal the final line's descent edge, rather than the requested box bottom.
- [ ] Empty text, whitespace-only text, text of only line breaks and a box fitting zero lines return no bottom; trailing blank lines after drawn text have an explicit tested contract.
- [ ] Clipped text reports only drawn lines and retains overflow; TypeScript offsets remain UTF-16 indices, and a box that fits no line reports offset `0`, which the docs tell callers to test with `!== undefined`.
- [ ] Rust and TypeScript produce equivalent measurements for matching inputs within a documented tolerance.
- [ ] Existing call sites compile unchanged and measurement does not perform a second shaping/layout pass.

## Post-merge checks

None. All behavior can be verified before merge.

## Tests

Add Rust builder tests and SDK tests using existing licensed font fixtures. Cover short text, wrapping, explicit line breaks, empty input, zero fitting lines, clipping, varied font sizes and line heights, and non-identity surface transforms.

1. Compute expected positions independently from known fixture-font ascent/descent and baseline placement; do not derive every expectation by calling the helper under test.
2. Draw two paragraphs at `firstBottom + gap`, save, reopen, and inspect their text positions. Use layout metrics as the numerical oracle; extracted ink bounds may differ from font ascent/descent.
3. Render a reference page with paragraph boxes and a rule at the reported bottom. Inspect the descent edge and second paragraph placement; retain a fixture or screenshot for review.
4. Run matching Rust/SDK cases against the same font, size, text and box. Compare measurements and overflow with documented numeric tolerances.
5. Run the targeted builder and SDK suites, public API/type checks, `cargo xtask check docs`, and the required CI suite. Record exact commands and outcomes in the PR. A deliberately wrong box-bottom or first-line implementation must fail the wrapped-text case.

## Docs to update

[Element flow plan](../../plans/element-flow.md), affected public API documentation, and the recipe/examples as scoped above. Follow the [example guidelines](../../guidelines/examples.md) and [writing guidelines](../../guidelines/docs-writing.md).

## Risks and open questions

Font layout edges differ from visible ink, so check measurements against independent font metrics and a rendered page together. The TypeScript helper must read the existing WebAssembly results rather than change their serialized shape. Settle public names before implementation, and never hide overflow to simplify the helper.

## Definition of done

- [ ] Claim this todo in the work PR's first commit from a fresh worktree: move `ready/` → `ongoing/`, fill **PR**, update links and regenerate the board.
- [ ] Implement tasks and demonstrate every acceptance criterion with recorded validation evidence.
- [ ] Follow applicable guidelines and complete code review and writing review; record findings and fixes in commits.
- [ ] Required CI is green on the final revision.
- [ ] Before merge, move this file to `done/`, update status and links, tick its plan item and regenerate the board.
