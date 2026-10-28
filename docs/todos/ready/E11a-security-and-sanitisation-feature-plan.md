# E11a — Security and sanitisation: feature plan

| Field | Value |
| --- | --- |
| ID | `E11a` |
| Status | `ready` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E10b](./E10b-signatures-feature-plan.md) |
| Blocks | [E11b](./E11b-encrypt-on-write.md), [E11d](./E11d-sanitiser.md), [E12a](./E12a-redaction-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e11a-security-and-sanitisation-feature-plan` |
| PR title | `docs(editor): E11 feature plan (security and sanitisation)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E11 ([full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation)): encryption on write (AES-256, `/V 5 /R 6`), passwords and permissions, removing security with authorisation, sanitisation ops with a report, public-key security (read; write behind a flag), and inspecting encryption. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [04 §7 Encryption](../../research/pdf/04-interactive-security.md#7-encryption-76)
- [library B2](../../plans/pdf-library.md#b2-document-model): decryption already implemented

## Scope

**In scope**

- The feature plan `docs/plans/editor/e11-security-and-sanitisation.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E11 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.01` document-open password; `G.02` encrypt a PDF; `G.03` permission passwords; `G.04` restrict print/copy/edit; `G.05` remove security with authorisation; `G.08` remove hidden metadata and comments; `G.09` sanitise embedded content; `G.10` remove attachments and scripts; `G.11` inspect encryption; `G.12` certificate-based security; `G.13` remove hidden layers/information
- **Crates and packages:** (planned for the round) `lamina-save` encryption on write over B2's crypt filters; public-key handler read; sanitiser ops with a report
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) `RequiresRewrite`; signed files warned that signatures will be invalidated
- **Web vs desktop:** identical; keys never leave the device
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e11-security-and-sanitisation.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`G.01`, `G.02`, `G.03`, `G.04`, `G.05`, `G.08`, `G.09`, `G.10`, `G.11`, `G.12`, `G.13`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@lamina/pdf`), and the UI tools/panels in `@lamina/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E11"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E11b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E11"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Do we ever write legacy R4/AES-128 (explicit opt-in per 04 §13 item 7) or AES-256 only?
- Open question: Honour `/P` permission bits by default in the editor (advisory; explicit override with owner password)?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
