# E8h — XFA: detection, `RemoveXfa` and the reader's notice

| Field | Value |
| --- | --- |
| ID | `E8h` |
| Status | `ready` |
| Plan | [E8 plan §2.4](../../plans/editor/e8-form-filling.md#24-xfa) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8c](./E8c-acroform-field-model.md), [E8e](./E8e-field-widgets-ui.md) |
| ADRs | Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0021](../../decisions/0021-pdf-javascript-policy.md) |
| Branch | `feat/e8h-xfa-detection` |
| PR title | `feat(forms): detect XFA forms and say what they need` |
| Size | M (200–400) |
| PR | — |

## Goal

An XFA form is named plainly, the most cited dead end in the Hacker News research ([R3](../../research/hacker-news/pdf-users-on-hacker-news.md#r3-finish-forms-and-signatures-and-name-xfa-files-plainly)). `doc.xfa()` exposes the packets and tells static from dynamic forms. The reader says a dynamic form needs Adobe's apps, and fills a static one's fields, removing the stale XFA part on the first change so every app shows the same answers.

## Context

- [E8 plan §2.4 and §5](../../plans/editor/e8-form-filling.md#24-xfa), decision 9 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)
- [04 §3.4 XFA](../../research/pdf/04-interactive-security.md#34-xfa); ISO 32000-2 §12.7.3 (the deprecated `XFA` entry)
- **Design (Paper):** [P01](../../design/paper/forms.md#p01-fill--sign-mode)'s XFA state; the plan refines its words, which the area document records

## Scope

**In scope**

- `xfa()` → packets (stream or name/stream array) and `XfaKind::{Static, Dynamic}`, read within `Limits`; the `config` packet read with the guarded XML reader
- `RemoveXfa {}` (class `Document`)
- The reader's notice for both kinds, the first-change removal in the same transaction, and the signed-file warning instead
- `fixtures/forms/xfa-static.pdf` and `xfa-dynamic.pdf`; façade `xfa`, SDK `xfa`
- An `xfa_packets` fuzz target

**Out of scope** (and which todo covers it)

- Editing `datasets` and laying out XFA: not planned (the E8 plan, decision 9)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.17` XFA forms
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** `RemoveXfa` (class `Document`, no rewrite)
- **`SaveConstraint`:** a violation under any certification and UR3; the reader never removes XFA from a signed file
- **Web vs desktop:** this todo is the core and the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `E.17` → `read` (core, pdf, web)

## Tasks

- [ ] The detector and packet reader
- [ ] `RemoveXfa`
- [ ] The notice and the first-change rule
- [ ] Tests and the fuzz target, added to `laminapdf-model`'s regression guard list

## Acceptance criteria

- [ ] The kind agrees with pdf.js's `isPureXfa` on the pinned corpus's XFA files
- [ ] After `RemoveXfa`, pdf.js shows the AcroForm fields; undo restores the packets byte for byte
- [ ] Playwright: the dynamic notice on `xfa-dynamic.pdf`; on `xfa-static.pdf` a fill removes XFA in one undo step

## Post-merge checks

None.

## Tests

- Unit tests per packet layout; Playwright in `apps/web/e2e/`

## Docs to update

- User help: XFA forms
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Some static forms keep values only in `datasets`; their AcroForm fields look empty after `RemoveXfa`. The notice says what Lamina PDF shows, and the corpus run counts such files

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
