# B10b — Add an optional vertical cursor

| Field | Value |
| --- | --- |
| ID | `B10b` |
| Status | `ready` |
| Plan | [Element flow](../../plans/element-flow.md#delivery-steps) |
| Round / milestone | Round B10 — measured placement helpers |
| Depends on | [B10a](../ongoing/B10a-measured-text-bottom.md) |
| ADRs | Follows [0016](../../decisions/0016-package-design.md), [0018](../../decisions/0018-test-strategy.md) and [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `feat/b10b-vertical-cursor` |
| PR title | `feat(build): add an optional vertical cursor` |
| Size | M (200–400 lines; keep adapters separate if needed) |
| PR | — |

## Goal

Start this only if B10a's PR records in the plan that the measurement helper alone leaves callers needing a cursor; otherwise B10a deletes this todo.

Add an optional vertical cursor over measured bottoms and explicit rectangles. A caller can place paragraphs and images in sequence while retaining control over pages, drawing and overflow.

## Context

- [Element flow plan](../../plans/element-flow.md#optional-vertical-cursor): the cursor's transition rules, validation and coordinate system.
- [B10a](../ongoing/B10a-measured-text-bottom.md): the measured bottoms the cursor consumes, and the decision whether this todo goes ahead.
- [Surface drawing and transforms](../../../crates/laminapdf-build/src/surface/mod.rs): why the cursor stays outside `Surface` and before its transform.

## Scope

**In scope**

- A cursor type in Rust and TypeScript with the same transitions: start, advance by a measured bottom or a caller-declared rectangle plus a gap, and reset.
- Validation of positions, gaps and rectangles before any state changes.
- Exports through the `laminapdf` façade and the SDK, with the base cursor available without the text feature if B10a's decision allows it.

**Out of scope** (and which todo covers it)

- The measured bottoms themselves: [B10a](../ongoing/B10a-measured-text-bottom.md).
- Runnable examples and the docs recipe: [B10c](./B10c-element-flow-recipe.md).
- A cursor held inside `Surface`: rejected by the [plan](../../plans/element-flow.md#optional-vertical-cursor).
- Automatic pagination, tables, floats and transformed ink bounds: no todo yet; they need a separate proposal ([plan](../../plans/element-flow.md#decisions-before-implementation)).

## Tasks

- [ ] Settle cursor names and exports, and decide whether text adapters need the text feature while the base cursor remains independent.
- [ ] Implement matching Rust and TypeScript behavior outside `Surface`; do not hold a surface borrow or page ownership.
- [ ] Support initial Y, current Y, explicit reset, optional measured bottom, rectangle placement and a nonnegative gap.
- [ ] Advance with `max(currentY, bottom + gap)` only after successful validation. Reject non-finite operands/results, invalid gaps and invalid declared rectangles before mutation.
- [ ] Keep empty results unchanged, preserve caller-visible overflow and document update-after-success usage.
- [ ] Add targeted tests, public API documentation and the required changeset.

## Acceptance criteria

- [ ] Empty placements apply no gap; earlier/overlapping bounds never move the cursor backward.
- [ ] Invalid initial/reset values and invalid advancement leave valid existing state unchanged; overflow of finite `bottom + gap` is rejected.
- [ ] Images and caller-declared rectangles advance consistently with measured text.
- [ ] Callers can reset for a new page or column and draw decorations without advancing.
- [ ] A failed drawing call does not trigger cursor advancement in the documented usage; clipped text leaves its overflow visible.
- [ ] Rust and TypeScript share the same transition cases; using transforms does not silently change the cursor coordinate system.

## Post-merge checks

None. All behavior can be verified before merge.

## Tests

Use table-driven state-transition tests in the Rust builder and SDK suites. Include absent bottoms, zero and positive gaps, backwards bounds, negative Y positions, explicit resets, NaN/infinity, negative gaps, overflow of finite addition and malformed rectangle inputs.

1. Assert exact expected cursor state after every operation, including errors. Test a sequence of successful text placement, rejected advancement, an image and a page reset.
2. Make a drawing operation fail, then verify the documented draw-then-advance integration preserves the prior cursor. Test clipping separately as successful partial drawing with a retained overflow offset.
3. Run the same transition table through Rust and TypeScript; compare accepted values and rejection behavior.
4. Build a mixed text/image page with a decoration and render it. Verify measured text gaps and declared image spacing independently from cursor internals.
5. Run targeted suites, supported feature combinations (including the text-disabled base cursor if offered), API/type checks and required CI. Record commands and outcomes. Temporarily removing `max` or input validation must make the relevant transition tests fail.

## Docs to update

[Element flow plan](../../plans/element-flow.md), affected public API documentation, and the recipe/examples as scoped above. Follow the [example guidelines](../../guidelines/examples.md) and [writing guidelines](../../guidelines/docs-writing.md).

## Risks and open questions

The cursor trusts the rectangles callers declare, so a wrong image rectangle moves it wrongly; document that it measures nothing itself. Mixing measurements taken under different transforms is unsupported and must be documented, not silently corrected. Rust and TypeScript must reject the same inputs, so keep the transition table shared.

## Definition of done

- [ ] Claim this todo in the work PR's first commit from a fresh worktree: move `ready/` → `ongoing/`, fill **PR**, update links and regenerate the board.
- [ ] Implement tasks and demonstrate every acceptance criterion with recorded validation evidence.
- [ ] Follow applicable guidelines and complete code review and writing review; record findings and fixes in commits.
- [ ] Required CI is green on the final revision.
- [ ] Before merge, move this file to `done/`, update status and links, tick its plan item and regenerate the board.
