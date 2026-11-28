# E8h — XFA: detection, `RemoveXfa` and the reader's notice

| Field | Value |
| --- | --- |
| ID | `E8h` |
| Status | `done` |
| Plan | [E8 plan §2.4](../../plans/editor/e8-form-filling.md#24-xfa) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8c](./E8c-acroform-field-model.md), [E8e](./E8e-field-widgets-ui.md) |
| ADRs | Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0021](../../decisions/0021-pdf-javascript-policy.md) |
| Branch | `feat/e8h-xfa-detection` |
| PR title | `feat(forms): detect XFA forms and say what they need` |
| Size | M (200–400) |
| PR | [#854](https://github.com/laudebugs/laminapdf/pull/854) |

## Goal

An XFA form is named plainly, the most cited dead end in the Hacker News research ([R3](../../research/hacker-news/pdf-users-on-hacker-news.md#r3-finish-forms-and-signatures-and-name-xfa-files-plainly)). `doc.xfa()` exposes the packets and tells static from dynamic forms. The reader says a dynamic form needs Adobe's apps, and fills a static one's fields, removing the stale XFA part on the first change so every app shows the same answers.

## Context

- [E8 plan §2.4 and §5](../../plans/editor/e8-form-filling.md#24-xfa), decision 9 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)
- [04 §3.4 XFA](../../research/pdf/04-interactive-security.md#34-xfa); ISO 32000-2 §12.7.3 (the deprecated `XFA` entry)
- **Design (Paper):** [P01](../../design/paper/forms.md#p01-fill--sign-mode)'s XFA state; the plan refines its words, which the area document records

## Scope

**In scope**

- `xfa()` → packets (stream or name/stream array) and `XfaKind::{Static, Dynamic}` (dynamic on any of the plan's three signals, the catalog's `NeedsRendering` among them), read within `Limits`; the `config` packet read with the guarded XML reader
- `RemoveXfa {}` (class `Document`): drops `/XFA` and the catalog's `/NeedsRendering`
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

- [x] The detector and packet reader
- [x] `RemoveXfa`
- [x] The notice and the first-change rule
- [x] Tests and the fuzz target, added to `laminapdf-model`'s regression guard list

## Acceptance criteria

- [ ] The kind agrees with pdf.js's `isPureXfa` on the pinned corpus's XFA files (not met here: the corpus is external and runs nightly; the two fixtures agree, checked in `apps/web/e2e/forms-xfa.e2e.test.ts`, and `xfa_filled_imm1344e` and `xfa_issue14315` haven't been run; the nightly comparison is [#856](https://github.com/laudebugs/laminapdf/issues/856))
- [x] After `RemoveXfa`, pdf.js shows the AcroForm fields; undo restores the packets byte for byte
- [x] Playwright: the dynamic notice on `xfa-dynamic.pdf`; on `xfa-static.pdf` a fill removes XFA in one undo step

## Post-merge checks

None.

## Tests

- Unit tests per packet layout; Playwright in `apps/web/e2e/`

## Docs to update

- User help: XFA forms
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## As built

- `Document::xfa()` gives the packets, the kind and the `signals` that made it dynamic. An `/XFA` stream is one packet named `xdp`; the plan's §2.4 "As built (E8h)" lists the rest.
- `RemoveXfa` fails with `invalid-op` on a form without `/XFA`. The save planner holds it as class `Document`: a violation under every certification and UR3 (`crates/laminapdf-save/tests/forms.rs`).
- The reader adds `removeXfa` to a fill, a form-data import and a reset of a static, unsigned form, in the same undo step; the notice has its own Undo while the removal is the newest step. A file with a signed signature field, or one `wouldViolate([removeXfa])` reports a violation for (a certification, usage rights), keeps its XFA part and warns; so does a form whose removal the reader was refused once.
- `xfa-dynamic.pdf` carries the XDP `preamble` and `postamble` packets, as Adobe writes them; without them pdf.js can't lay the form out. pdf.js calls a form pure XFA only with `/NeedsRendering`, an XFA part and no AcroForm fields, which is narrower than the dynamic kind.
- Budgets (ADR 0016 §6, amended from CI's first run): base 1,206,200 kept (CI measured 1,205,745, headroom 455 B: [#859](https://github.com/laudebugs/laminapdf/issues/859)), extract 2,060,100, text 2,510,200, the SDK's JS 32,700.

**Deviations.**

- **Desktop half:** [#855](https://github.com/laudebugs/laminapdf/issues/855).
- **Not run locally:** the pinned corpus, CI, `/code-review`. The Playwright spec ran in Chromium, Firefox and WebKit.
- **Example pair:** `check_xfa_form` (Rust and TypeScript) with a byte-identical golden and the Check for an XFA form recipe; it draws nothing, so no render goldens.
- **Base headroom:** [#859](https://github.com/laudebugs/laminapdf/issues/859).
- **Issues from the review:** [#856](https://github.com/laudebugs/laminapdf/issues/856), [#857](https://github.com/laudebugs/laminapdf/issues/857), [#858](https://github.com/laudebugs/laminapdf/issues/858).
- **Signed-file notice:** covered by a unit test of `xfaNotice` (`apps/web/src/forms/rules.test.ts`), not by Playwright, since no signed XFA fixture exists.

## Risks and open questions

- Some static forms keep values only in `datasets`; their AcroForm fields look empty after `RemoveXfa`. The notice says what Lamina PDF shows, and [#856](https://github.com/laudebugs/laminapdf/issues/856)'s corpus comparison should count such files, as [#858](https://github.com/laudebugs/laminapdf/issues/858) asks (not done yet)

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process)) (not done: left to the PR's reviewer)
- [ ] CI green (not checked: the PR was opened without waiting for CI)
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
