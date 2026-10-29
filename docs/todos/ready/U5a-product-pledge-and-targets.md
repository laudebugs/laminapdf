# U5a — Product pledge and performance targets page

| Field | Value |
| --- | --- |
| ID | `U5a` |
| Status | `ready` |
| Plan | [UI/UX §6 U5. Product promises](../../plans/ui-ux.md#u5-product-promises) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [B7a](./B7a-docs-site.md) |
| Blocks | None |
| ADRs | Follows 0011 (hosted service and privacy) — the pledge must agree with it |
| Branch | `docs/u5a-pledge-and-targets` |
| PR title | `docs(site): publish the product pledge and performance targets` |
| Size | S (docs only) |
| PR | — |

## Goal

Turn the research's strongest switching reason — users were burned by changed deals and paywalls — into a public, owner-approved promise: every on-device feature free forever, no account, watermark, page or task cap, and paid tiers only for services that cost money to run. Publish the speed targets (release gates and stretch goals) next to it.

## Context

- [UI/UX plan §5.9](../../plans/ui-ux.md#59-product-promises-r18-r19); [research: monetisation](../../research/product/pdf-app-product-and-ux.md#monetisation-the-obsidian-template) — Obsidian's model (Feb 2025), Stirling-PDF's paid tiers, GoodNotes/LiquidText/PDF Expert backlash.
- [UI/UX plan §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7) — the targets table.
- [B7a](./B7a-docs-site.md) — the docs site this page lives on.
- [Architecture §8 Q2](../../plans/architecture.md#8-open-questions-for-the-team) — the hosted service (accounts, storage, sync) as opt-in.

## Scope

**In scope**

- A "Promises" page: the pledge, what is and isn't free, what the hosted service charges for (to be set by the owner), the licence (MIT OR Apache-2.0) and why it can't be revoked.
- A "Performance" page: the release gates and stretch goals with methods, linking U5b's results when they exist.

**Out of scope** (and which todo covers it)

- Prices of the hosted service → owner decision with H-series todos.
- Benchmark numbers → [U5b](./U5b-reader-benchmark.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Draft the pledge text from the plan; get explicit owner approval in the PR (required).
- [ ] Check it against ADR 0011 and the H-series plans; resolve contradictions before merge.
- [ ] Write the performance page from the plan's §5.6 table.

## Acceptance criteria

- [ ] The owner has approved the pledge text in the PR.
- [ ] No statement contradicts ADR 0011 or the full-editor plan's hosted-service section.
- [ ] `cargo xtask check docs` passes.

## Post-merge checks

None.

## Tests

None (docs only).

## Docs to update

Docs site pages; answer open question 1 in [UI/UX §8](../../plans/ui-ux.md#8-open-questions); tick the pledge item in §9.

## Risks and open questions

- A pledge is hard to take back: word it narrowly (on-device features) and have the owner decide.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
