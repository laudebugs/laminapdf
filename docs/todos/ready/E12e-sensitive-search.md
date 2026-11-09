# E12e — Search sensitive content to redact

| Field | Value |
| --- | --- |
| ID | `E12e` |
| Status | `ready` |
| Plan | [full-editor §E12](../../plans/full-editor.md#e12-redaction) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [C5b](../ongoing/C5b-find-in-document.md), [E12b](./E12b-mark-and-review.md), [E12d](./E12d-scrub-and-rewrite.md) |
| Dependency notes | [E12b](E12b-mark-and-review.md), [E12d](E12d-scrub-and-rewrite.md) (`ApplyRedactions` complete, for the search → mark → apply → verify e2e), `C5` ([web-app §C5](../../plans/web-app.md#c5-text-selection-and-search): text layer and search) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e12e-sensitive-search` |
| PR title | `feat(redact): pattern search for sensitive content` |
| Size | M (200–600) |
| PR | — |

## Goal

Find redaction candidates by text, regex and built-in patterns (emails, phone numbers, card numbers with Luhn, IBANs, national IDs per locale), including OCR text layers when present (E13), across content, annotations, fields and metadata; mark all or selected.

## Context

- **Design (Paper):** [G09 Find sensitive content](../../design/paper/sign-protect-redact.md#g09-find-sensitive-content). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [06 §3 group G](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms)

## Scope

**In scope**

- Pattern library
- Search over all layers
- Mark-all UI

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.07` search sensitive content to redact
- **Crates and packages:** `laminapdf-text`, `@laminapdf/ui`
- **`Op` variants:** produces `MarkRedaction` ops
- **`SaveConstraint`:** as E12b
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.07` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Patterns
- [ ] Search
- [ ] UI

## Acceptance criteria

- [ ] Pattern tests with positive/negative samples
- [ ] e2e: search, mark, apply, verify absent

## Post-merge checks

None.

## Tests

- Unit, e2e

## Docs to update

- User help
- [full-editor §E12](../../plans/full-editor.md#e12-redaction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- OCR text matches depend on E13; until then the search covers existing text only

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
