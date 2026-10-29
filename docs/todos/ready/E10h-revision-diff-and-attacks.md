# E10h — Revision diff analysis, modification warnings and attack-catalogue tests

| Field | Value |
| --- | --- |
| ID | `E10h` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E10f](./E10f-certification-and-locking.md), [E10g](./E10g-signature-validation.md) |
| Blocks | [E10j](./E10j-signature-ui.md), [E14b](./E14b-revision-browser.md) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10h-revision-diff-and-attacks` |
| PR title | `feat(sign): revision diff analysis and signature attack tests` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Diff every revision after a signature at object level and classify changes (LTV, form fill, annotation, signature, other) under DocMDP/FieldMDP/Lock rules; flag redefinitions of anything reachable from rendering (shadow attacks), annotations added after certification (EAA), signature widgets overlaying content (SSA); and ship a regression corpus of USF/ISA/SWA/shadow/EAA/SSA files.

## Context

- **Design (Paper):** [G05 Revision diff and attack warnings](../../design/paper/sign-protect-redact.md#g05-revision-diff-and-attack-warnings). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §8.7 Incremental updates after signing](../../research/pdf/04-interactive-security.md#87-incremental-updates-after-signing)
- [04 §8.8](../../research/pdf/04-interactive-security.md#88-attacks-on-signatures-ruhr-universität-bochum-pdf-insecurity-research)
- [full-editor §8](../../plans/full-editor.md#8-risks) (external review before release)

## Scope

**In scope**

- Revision index (shared with E14b) and object diff
- Classifier. The "LTV" class (a revision that only adds `/DSS` data and/or a `/DocTimeStamp` signature) is allowed at every DocMDP level, **including P1** (ISO 32000-2 §12.8.2.2.1, 2.0 errata §12.8.5.3; [04 §8.5](../../research/pdf/04-interactive-security.md#85-pades-baseline-levels-etsi-en-319-142-1-current-v121-2024-01)); it must match the planner rule in [E10e](E10e-timestamps-and-tsa-proxy.md), so both use one shared function in `lamina-sign`. A revision that mixes LTV additions with other changes is classified by the other changes
- Render-compare of signed revision vs current as an optional deep check
- Attack corpus generated from the papers' descriptions (scripts + provenance)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.09` allowed changes after certification; `F.10` signature status and modification warnings
- **Crates and packages:** `lamina-sign`, `lamina-cos` (revisions)
- **`SaveConstraint`:** read-only
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.09` `write` → `round-trip` (core, pdf) (from the status E10f set); `F.10` → `read` (core, pdf) (web and desktop columns flip in [E10j](E10j-signature-ui.md))

## Tasks

- [ ] Diff
- [ ] Classifier
- [ ] Attack corpus
- [ ] Tests

## Acceptance criteria

- [ ] Every attack file is reported as invalid or modified
- [ ] Legitimate LTV and form-fill updates are classified as allowed; a DSS + document-timestamp revision on a DocMDP **P1** certified fixture is classified as allowed LTV, and a revision that adds DSS data plus any other object change on P1 is flagged

## Post-merge checks

None.

## Tests

- Attack corpus in `fixtures/signatures/attacks/`

## Docs to update

- Security notes
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Needs an external security review before release (plan §8)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
