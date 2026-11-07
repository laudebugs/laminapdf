# B10c — Document and demonstrate element flow

| Field | Value |
| --- | --- |
| ID | `B10c` |
| Status | `ready` |
| Plan | [Element flow](../../plans/element-flow.md#delivery-steps) |
| Round / milestone | Round B10 — measured placement helpers |
| Depends on | [B10a](./B10a-measured-text-bottom.md), [B10b](./B10b-vertical-cursor.md) |
| ADRs | Follows [0016](../../decisions/0016-package-design.md), [0018](../../decisions/0018-test-strategy.md) and [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `docs/b10c-element-flow-recipe` |
| PR title | `docs(build): document and demonstrate element flow` |
| Size | M (paired examples, recipe and tests) |
| PR | — |

## Goal

Publish a tested report recipe that places paragraphs and an image using the cursor. Demonstrate when the helper saves work, and where callers still need explicit overflow and page-break handling.

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

Font layout edges differ from visible ink. Use independent metric assertions and visual evidence together. Public names and feature gating must be settled before implementation; do not change existing output rectangles or hide overflow to simplify usage.

## Definition of done

- [ ] Claim this todo in the work PR's first commit from a fresh worktree: move `ready/` → `ongoing/`, fill **PR**, update links and regenerate the board.
- [ ] Implement tasks and demonstrate every acceptance criterion with recorded validation evidence.
- [ ] Follow applicable guidelines and complete code review and writing review; record findings and fixes in commits.
- [ ] Required CI is green on the final revision.
- [ ] Before merge, move this file to `done/`, update status and links, tick its plan item and regenerate the board.
