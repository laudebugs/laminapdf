# E11g — Sanitise dialog and report

| Field | Value |
| --- | --- |
| ID | `E11g` |
| Status | `ready` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E11d](./E11d-sanitiser.md) |
| ADRs | Follows [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e11g-sanitise-dialog` |
| PR title | `feat(ui): the Sanitise dialog and its removal report` |
| Size | M (200–600) |
| PR | — |

## Goal

The screens for E11d's sanitiser: the dialog that shows what a file hides and lets a person choose what to remove, and the report panel that lists what went. Opens the web and desktop columns of G.08, G.09, G.10 and G.13.

## Context

- **Design (Paper):** [G07 Sanitise](../../design/paper/sign-protect-redact.md#g07-sanitise). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E11 feature plan](../../plans/editor/e11-security-and-sanitisation.md) §2.3 (the ops and the report) and §5 (the signed-file rows)
- [E11d](./E11d-sanitiser.md): `Sanitise`, the strip ops and `SanitiseReport` this PR draws

## Scope

**In scope**

- The dialog: summary of what was found, the Preset select (Custom, Before sharing outside the organisation, Everything), the category checklist with counts and expandable item lists, "Earlier versions" always on, the dynamic-XFA warning, the signed-file notice, the scanning and nothing-found states
- The report panel: a section per category, a "Kept" section, the post-removal scan, **Export report…** (JSON or text) and **Copy as text**
- `security.sanitise` in the More menu and the palette, and the offer from Share and Export when the file has scripts or attachments
- Phone: the dialog and the report as large-detent sheets
- Help page: sanitising a PDF

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.08`, `G.09`, `G.10`, `G.13` (the screens)
- **Crates and packages:** `@laminapdf/ui`, `apps/web`, `apps/desktop`
- **`Op` variants:** none new
- **`SaveConstraint`:** as E11d's
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `G.08`, `G.09`, `G.10` → `round-trip` (web, desktop); `G.13` → `write` (web, desktop)

## Tasks

- [ ] The dialog with its presets, counts and states
- [ ] The report panel and its exports
- [ ] The command, menu entries, Share and Export offer and phone sheets
- [ ] Playwright and WebdriverIO scenarios; axe on each screen
- [ ] Help page

## Acceptance criteria

- [ ] Choosing a preset ticks the categories G07 shows, and the button's count follows the selection
- [ ] After removal the report matches the ops' results and the scan shows nothing left in the removed categories
- [ ] The exported report holds object numbers and counts and no document text
- [ ] The checklist, counts and expand buttons are keyboard- and screen-reader-ready (axe clean)

## Post-merge checks

None.

## Tests

- Vitest for the preset logic, counts and report panel
- Playwright and WebdriverIO: sanitise `hidden-everything.pdf`, read the report, save, reopen and run the scan

## Docs to update

- User help: sanitising a PDF
- [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) and its §10 to-dos
- `docs/capabilities/manifest.toml` statuses (regenerated via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
