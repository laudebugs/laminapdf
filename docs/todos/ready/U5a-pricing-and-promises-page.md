# U5a — Pricing, promises and performance targets pages

| Field | Value |
| --- | --- |
| ID | `U5a` |
| Status | `ready` |
| Plan | [UI/UX §6 U5. Product promises](../../plans/ui-ux.md#u5-product-promises) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [B7a](./B7a-docs-site.md), [C7c](./C7c-performance-gates-and-launch.md) |
| Blocks | None |
| ADRs | Follows 0011 (hosted service, privacy, telemetry) — the pricing page must agree with it |
| Branch | `docs/u5a-pricing-and-promises` |
| PR title | `docs(site): publish pricing, product promises and performance targets` |
| Size | S (docs only) |
| PR | — |

## Goal

lamina will be priced (owner decision, 2026-10-29). The research found that users don't leave over prices as such but over surprise paywalls, changed deals and watermarks ([report §1](../../research/product/pdf-app-product-and-ux.md#competitors-fail-on-bloat-broken-deals-and-silent-damage-not-on-missing-features)). Publish the pricing the owner sets together with fair-pricing promises that answer those grievances, and the speed targets (release gates and stretch goals) next to them.

## Context

- [UI/UX plan §4 principle 10 and §5.9](../../plans/ui-ux.md#59-product-promises-r18-r19) — "priced, never hostile" and the fair-pricing rules.
- [Research: monetisation](../../research/product/pdf-app-product-and-ux.md#monetisation-the-obsidian-template) — Obsidian (free core, paid services), Stirling-PDF's paid tiers, the GoodNotes/LiquidText/PDF Expert backlash after changed deals; Acrobat Studio's AI tier. The free-core model was considered and not chosen.
- [UI/UX plan §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7) — the targets table (desktop gates: cold start < 300 ms, installer < 15 MB).
- [B7a](./B7a-docs-site.md) — the docs site these pages live on; [C7c](./C7c-performance-gates-and-launch.md) — the launch they follow.
- [Architecture §8 Q2](../../plans/architecture.md#8-open-questions-for-the-team) and [H2](./H2-accounts-and-login.md) — the hosted service and accounts, one possible way to check entitlements.

## Scope

**In scope**

- A "Pricing" page: the model and prices the owner sets (one-time, subscription or both; per platform or cross-platform), what each tier includes, trial terms if any, and what the hosted service (sync, storage) costs.
- A "Promises" section on it, owner-approved: the price and tier are shown before work starts (no paywall discovered after filling a form or editing a file); output is never watermarked without saying so first; existing purchases are honoured when the model changes; no ads; documents are never used for anything but the user's own work; the code stays MIT OR Apache-2.0.
- A "Performance" page: the release gates and stretch goals with methods, linking U5b's results when they exist.

**Out of scope** (and which todo covers it)

- Entitlement checks, licence keys, store purchases and payment in the apps → a new todo once the owner picks the model (open question 1 of the [UI/UX plan](../../plans/ui-ux.md#8-open-questions)).
- Benchmark numbers → [U5b](./U5b-reader-benchmark.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Get the pricing model, tiers and prices from the owner; draft the pages; owner approval in the PR (required).
- [ ] Check every promise against what the apps actually do (a feature behind a tier must be labelled as such before the user starts the task) and against ADR 0011 and the H-series plans; resolve contradictions before merge.
- [ ] Write the performance page from the plan's §5.6 table.
- [ ] If the pricing needs entitlement checks in the apps, add that todo to the board (with the plan link) in this PR.

## Acceptance criteria

- [ ] The owner has approved the pricing and promises text in the PR.
- [ ] No statement contradicts ADR 0011, the full-editor plan's hosted-service section or the apps' behaviour.
- [ ] `cargo xtask check docs` passes.

## Post-merge checks

None.

## Tests

None (docs only).

## Docs to update

Docs site pages; [UI/UX §8](../../plans/ui-ux.md#8-open-questions) question 1 (record the chosen model); tick the pricing item in §9.

## Risks and open questions

- A promise is hard to take back: word each one narrowly and have the owner decide.
- Pricing in app stores (if mobile or the Mac App Store come later) has its own rules on trials and upgrades; note them when those channels are decided.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
