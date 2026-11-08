# B10c — Document and demonstrate element flow

| Field | Value |
| --- | --- |
| ID | `B10c` |
| Status | `ready` |
| Plan | [Element flow](../../plans/element-flow.md#delivery-steps) |
| Round / milestone | Round B10 — measured placement helpers |
| Depends on | [B10a](../ongoing/B10a-measured-text-bottom.md), [B10b](./B10b-vertical-cursor.md) |
| ADRs | Follows [0016](../../decisions/0016-package-design.md), [0018](../../decisions/0018-test-strategy.md) and [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `feat/b10c-element-flow-recipe` |
| PR title | `feat(examples): document and demonstrate element flow` |
| Size | M (paired examples, recipe and tests) |
| PR | — |

## Goal

Publish a tested report recipe that places paragraphs and an image using the cursor. Demonstrate when the helper saves work, and where callers still need explicit overflow and page-break handling.

## Context

- [Element flow plan](../../plans/element-flow.md#how-each-step-proves-it-works): the report the recipe builds and the before/after comparison it records.
- [B10a](../ongoing/B10a-measured-text-bottom.md) and [B10b](./B10b-vertical-cursor.md): the helper and cursor the examples use.
- [Example guidelines](../../guidelines/examples.md): paired Rust and TypeScript examples, public APIs only, registered with the snippet checks.

## Scope

**In scope**

- Paired Rust and TypeScript examples of a report placed with the measured bottoms (and the cursor, if B10b went ahead), including the minimal next-paragraph example.
- A docs-site recipe built from those examples.
- The before/after authoring comparison described in the plan.

**Out of scope** (and which todo covers it)

- Any change to the helper or cursor APIs: [B10a](../ongoing/B10a-measured-text-bottom.md), [B10b](./B10b-vertical-cursor.md).
- Automatic pagination, tables, floats and transformed ink bounds: no todo yet; they need a separate proposal ([plan](../../plans/element-flow.md#decisions-before-implementation)).

## Tasks

- [ ] Add paired Rust and TypeScript examples using only public APIs and existing example conventions.
- [ ] Generate a report with short and wrapped paragraphs, an empty optional section, an image, a decoration and an explicit page reset.
- [ ] Demonstrate clipping detection and an intentional caller decision; do not silently discard overflow or claim automatic pagination.
- [ ] Add a recipe to the docs source and explicit sidebar; register examples with the existing snippet/test system.
- [ ] Explain typographic versus painted bounds, point units, pre-transform coordinates, and correct TypeScript rectangle fields.
- [ ] Record the usefulness comparison described below, tick the plan tasks, and update public API references as needed.

## Acceptance criteria

- [ ] Both examples execute and show the same placements within the established numeric tolerance.
- [ ] Changing a paragraph from one line to several moves following content without changing hand-entered Y coordinates.
- [ ] Empty content adds no gap; images retain requested spacing; page reset starts at the configured top margin.
- [ ] Overflow is surfaced explicitly and no example silently treats partial text as complete.
- [ ] The recipe states that painted bounds, automatic pagination and full flow layout are outside this helper's contract.
- [ ] Review evidence includes generated PDFs, placement assertions, a rendered page and the before/after authoring comparison.

## Post-merge checks

None. All behavior can be verified before merge.

## Tests

1. Run both examples through the existing example harness and compare placement records or reference PDFs using the repository's established approach.
2. Reopen the PDFs and verify page count and text presence/order. Assert layout positions numerically; render to check text/image collisions and the explicit page reset. Extraction order alone does not prove spacing.
3. Change a report summary from one line to multiple lines, then remove an optional section. Regenerate both versions and verify following elements move by the measured height change, with no new gap for the absent section.
4. Compare the same report before and after the helper: count caller-owned Y updates and repeated bottom calculations, and list the edits needed for the text change. Keep fixture data and styles equal. Report actual counts rather than a promised percentage reduction.
5. Run the paired examples, snippet checks, docs-link/prose checks, docs-site build and required CI. Retain exact commands, results and artifact locations in the PR. Visual review supplements assertions; it does not replace them.

## Docs to update

[Element flow plan](../../plans/element-flow.md), affected public API documentation, and the recipe/examples as scoped above. Follow the [example guidelines](../../guidelines/examples.md) and [writing guidelines](../../guidelines/docs-writing.md).

## Risks and open questions

A recipe can suggest more than the helper does, so it must say plainly that overflow and page breaks stay the caller's job. The examples are byte-compared goldens, so a later change to text layout changes them too. Keep the before/after comparison to measured counts, not a promised saving.

## Definition of done

- [ ] Claim this todo in the work PR's first commit from a fresh worktree: move `ready/` → `ongoing/`, fill **PR**, update links and regenerate the board.
- [ ] Implement tasks and demonstrate every acceptance criterion with recorded validation evidence.
- [ ] Follow applicable guidelines and complete code review and writing review; record findings and fixes in commits.
- [ ] Required CI is green on the final revision.
- [ ] Before merge, move this file to `done/`, update status and links, tick its plan item and regenerate the board.
