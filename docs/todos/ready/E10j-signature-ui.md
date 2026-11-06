# E10j — Signing flow and signature panel UI

| Field | Value |
| --- | --- |
| ID | `E10j` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E10c](./E10c-visual-signatures.md), [E10e](./E10e-timestamps-and-tsa-proxy.md), [E10h](./E10h-revision-diff-and-attacks.md), [E10i](./E10i-platform-signers.md) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10j-signature-ui` |
| PR title | `feat(ui): signing flow and signature status panel` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Sign/certify flow (choose field or draw one, choose identity, appearance, reason/location, timestamp), a signature panel with status badges and structured results, certificate viewer, and modification warnings; E10 round close.

## Context

- **Design (Paper):** [G01 Add a signature image](../../design/paper/sign-protect-redact.md#g01-add-a-signature-image), [G02 Sign with a certificate](../../design/paper/sign-protect-redact.md#g02-sign-with-a-certificate), [G11 Phone: sign a document](../../design/paper/sign-protect-redact.md#g11-phone-sign-a-document). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.7](../../plans/ui-ux.md#57-trust-surfaces-r9r13) and [product research: personas](../../research/product/pdf-app-product-and-ux.md#what-each-persona-needs) — users confuse e-signatures and certificate signatures (Adobe forum threads, university guides): offer two separately named actions, "Add a signature image" and "Sign with a certificate", each with one plain sentence on what it proves, and a validation badge on open.
- [full-editor §E10](../../plans/full-editor.md#e10-signatures)

## Scope

**In scope**

- Sign dialog and flow
- Signature panel
- Certificate viewer
- Warnings for modified/uncovered revisions

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.03` certificate-based digital signatures; `F.04` verify signatures; `F.05` certify a document; `F.06` trusted timestamps; `F.07` inspect signer identity and certificates; `F.08` lock fields after signing; `F.09` allowed changes after certification; `F.10` signature status and modification warnings
- **Crates and packages:** `@laminapdf/ui`
- **`SaveConstraint`:** signing makes the document `AppendOnly`; adding a signature is allowed without certification and under DocMDP P2/P3, and is a violation under P1 (ADR 0007 §2); signing always runs last in a save (redact → rewrite → sign, ADR 0007 §4); `Sign` is a barrier op (ADR 0006 §5)
- **Web vs desktop:** identical apart from identity sources
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.03` `write` → `round-trip` (web, desktop) (from the status E10i set); `F.04` → `read` (web, desktop); `F.05` → `round-trip` (web, desktop); `F.06` → `round-trip` (web, desktop); `F.07` → `read` (web, desktop); `F.08` → `round-trip` (web, desktop); `F.09` → `round-trip` (web, desktop); `F.10` → `read` (web, desktop). These are the only flips of the web and desktop columns for F.04–F.10; E10e–E10h flip core and pdf

## Tasks

- [ ] Flow
- [ ] Panel
- [ ] Viewer
- [ ] e2e
- [ ] Capability check

## Acceptance criteria

- [ ] e2e: sign, reopen, see valid; modify, see modification warning
- [ ] axe zero serious violations

## Post-merge checks

None.

## Tests

- Playwright, WebdriverIO

## Docs to update

- User help: signing and validation
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
