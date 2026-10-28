# E12d — Scrub non-content layers, full rewrite and verification report

| Field | Value |
| --- | --- |
| ID | `E12d` |
| Status | `ready` |
| Plan | [full-editor §E12](../../plans/full-editor.md#e12-redaction) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E11d](./E11d-sanitiser.md), [E12c](./E12c-apply-content-removal.md) |
| Blocks | [E12f](./E12f-leak-tests.md), [E18g](./E18g-leases-and-base-revisions.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e12d-scrub-and-rewrite` |
| PR title | `feat(redact): scrub metadata, annotations, fields and structure; verification report` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Complete `ApplyRedactions`: remove matching text from annotations (`/Contents`, `/RC`, popups, replies), form values and appearances, metadata (Info, XMP doc/object level), bookmarks, named destinations, page labels, file names, embedded files, JS strings, thumbnails, struct tree (`/ActualText`, `/Alt`, `/E`); force a full rewrite with no prior revisions; produce a per-layer verification report (extract, render, diff).

## Context

- [04 §10.2](../../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do) (table of layers)
- [ADR 0007](../../decisions/0007-save-planner.md) §4 order

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
- **Capability-matrix rows to flip:** `G.06` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Scrubbers
- [ ] Rewrite
- [ ] Checker
- [ ] Report UI

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] No previous revision survives (single revision, no free-list remnants holding content)
- [ ] Report is generated for every apply

## Tests

- Layer fixtures

## Docs to update

- User help
- [full-editor §E12](../../plans/full-editor.md#e12-redaction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
