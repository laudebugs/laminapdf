# E12d — Scrub non-content layers, full rewrite and verification report

| Field | Value |
| --- | --- |
| ID | `E12d` |
| Status | `ready` |
| Plan | [full-editor §E12](../../plans/full-editor.md#e12-redaction) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E8c](./E8c-acroform-field-model.md), [E10d](./E10d-pades-signing-core.md), [E11d](./E11d-sanitiser.md), [E12c](./E12c-apply-content-removal.md) |
| Blocks | [E12e](./E12e-sensitive-search.md), [E12f](./E12f-leak-tests.md), [E18g](./E18g-leases-and-base-revisions.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e12d-scrub-and-rewrite` |
| PR title | `feat(redact): scrub metadata, annotations, fields and structure; verification report` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Complete `ApplyRedactions`: remove matching text from annotations (`/Contents`, `/RC`, popups, replies), form values and appearances, metadata (Info, XMP doc/object level), bookmarks, named destinations, page labels, file names, embedded files, JS strings, thumbnails, struct tree (`/ActualText`, `/Alt`, `/E`); force a full rewrite with no prior revisions; produce a per-layer verification report (extract, render, diff).

## Context

- [04 §10.2](../../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do) (table of layers)
- [ADR 0007](../../decisions/0007-save-planner.md) §4 order (redact → full rewrite → sign); [E10d](E10d-pades-signing-core.md) provides `Sign` for the redact-then-sign test, [E8c](E8c-acroform-field-model.md) the field model whose values and appearances are scrubbed

## Scope

**In scope**

- Scrubbers per layer
- Rewrite with GC (unreferenced objects dropped)
- Post-condition checker and report
- Apply dialog

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.06` permanent redaction
- **Crates and packages:** `lamina-ops`, `lamina-save`
- **`Op` variants:** `ApplyRedactions` (completed)
- **`SaveConstraint`:** `RequiresRewrite`: a full rewrite, every earlier revision dropped. On signed files the constraint is `Conflict`: the UI names the signatures that will be invalidated and removed (ADR 0007 §3 override); order redact → rewrite → sign
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.06` → `round-trip` (core, pdf, web, desktop): the row's only flip (E12b and E12c leave it `unsupported`). This PR removes E12c's `unstable-redaction` gate, so `ApplyRedactions` becomes public here

## Tasks

- [ ] Scrubbers
- [ ] Rewrite
- [ ] Checker
- [ ] Report UI
- [ ] Remove E12c's `unstable-redaction` feature gate: `ApplyRedactions` joins the default `Op` enum, `lamina`, `@lamina/pdf`, the JSON schema and the UI apply action; freeze its first `fixtures/ops/v1/` sample here

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected for `ApplyRedactions` (barrier, `requires_rewrite`): `RequiresRewrite` on unsigned files and `Conflict` on every signed fixture
- [ ] No previous revision survives (single revision, no free-list remnants holding content)
- [ ] Report is generated for every apply
- [ ] Redact then sign in one save (moved here from E10d): with pending `ApplyRedactions` and a pending `Sign` (E10d software signer), the planner writes the full rewrite first and the signature as an incremental revision on top (ADR 0007 §4); the output has exactly two revisions, the signature validates in pyHanko, and no redacted content is in either revision

## Post-merge checks

None.

## Tests

- Layer fixtures

## Docs to update

- User help
- [full-editor §E12](../../plans/full-editor.md#e12-redaction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
