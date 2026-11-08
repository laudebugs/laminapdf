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

- [Element flow plan](../../plans/element-flow.md): current API evidence, scope and shared contract.
- [PDF library scope](../../plans/pdf-library.md): full flowing layout remains deferred.
- [Creating PDFs from code](../../research/pdf/05-creating-pdfs-from-code.md): builder and layout background.
- The plan's implementation references identify the existing Rust, WebAssembly and SDK measurement paths to inspect before coding.

## Scope

**In scope**

The tasks and observable behaviors below, using existing drawing and text layout primitives.

**Out of scope**

Automatic pagination, tables, floats, transformed ink bounds and a hidden cursor in `Surface`. These require a separate proposal. The other B10 todos own the adjacent delivery steps.

## Tasks

- [ ] Settle additive accessor/helper names and document empty-result behavior before implementation.
- [ ] Reuse `TextRun::bounds()` and `TextLayout::lines` in `crates/laminapdf-build/src/surface/`; keep existing fields and return types.
- [ ] Export through the Rust facade and SDK public API; inspect the WebAssembly DTO boundary without changing its serialized shape unnecessarily.
- [ ] Preserve overflow offsets and pre-transform coordinate semantics.
- [ ] Add public API documentation, a minimal tested next-paragraph example, and the required changeset.

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

Font layout edges differ from visible ink. Use independent metric assertions and visual evidence together. Public names and feature gating must be settled before implementation; do not change existing output rectangles or hide overflow to simplify usage.

## Definition of done

- [ ] Claim this todo in the work PR's first commit from a fresh worktree: move `ready/` → `ongoing/`, fill **PR**, update links and regenerate the board.
- [ ] Implement tasks and demonstrate every acceptance criterion with recorded validation evidence.
- [ ] Follow applicable guidelines and complete code review and writing review; record findings and fixes in commits.
- [ ] Required CI is green on the final revision.
- [ ] Before merge, move this file to `done/`, update status and links, tick its plan item and regenerate the board.
