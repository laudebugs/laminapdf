# E2t — Signed files in the web reader: Save a copy, Details and Remove signature…

| Field | Value |
| --- | --- |
| ID | `E2t` |
| Status | `ongoing` |
| Plan | [E2 page organisation §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2i](../done/E2i-signed-gating-and-close.md) |
| ADRs | Follows [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2t-signed-file-choices` |
| PR title | `feat(web): save a copy, details and remove signature on signed files` |
| Size | M (200–400): three controls on existing surfaces, their words and their e2e tests |
| PR | — |

## Goal

Give the web reader the choices Paper draws for signed files that E2h and E2i left out. When a page op would break a signature, the user can keep the original signed, see what the signature forbids, or remove the signature on purpose, without having to go through Save. E2i proved the verdicts behind these choices; this todo puts them in front of the user.

## Context

- **Design (Paper):** [O06's certified banner](../../design/paper/edit-session-organise.md#o06-organise-mode) (Save a copy · Apply anyway… · Details), the signed-page question's Remove signature…, and [O03 Close with unsaved changes](../../design/paper/edit-session-organise.md#o03-close-with-unsaved-changes) in its Conflict state (Save becomes "Save a copy…"). The area document's As built notes list what E2h built and what waits here.
- [E2i](../done/E2i-signed-gating-and-close.md): the verdict for every page op on every signed fixture, and the e2e test of Apply anyway… and the save that removes a certification.
- The save flow (`apps/web/src/save-flow.ts`): the remove-signature question (O02) and Save a copy.

## Scope

**In scope**

- The certified banner (certification or usage rights): **Save a copy** writes a copy with the change, leaving the original signed (the O02 confirm worded for the copy); **Details** shows what the certification or usage rights forbid and which signature.
- The signed-page question: **Remove signature…** applies the op and marks the save to remove the signature, through the O02 confirm.
- O03's Conflict state: "Save a copy…" in place of Save, with the line naming the signature, and the window closing after the copy is written.
- Playwright tests of each, as a user, in Chromium, Firefox and WebKit; axe on each new surface.

**Out of scope** (and which todo covers it)

- The desktop's versions of these surfaces: [E2n](E2n-desktop-organise.md).

**Editor facts**

- **Crates and packages:** `apps/web`, `@laminapdf/ui` (words)
- **`SaveConstraint`:** `Conflict` (the choices) and `AppendOnly` (unchanged)
- **Web vs desktop:** web only; the desktop's are E2n's
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Fresh worktree; claim in the work PR
- [ ] Save a copy and Details on the banner
- [ ] Remove signature… on the signed-page question
- [ ] O03's Conflict state
- [ ] Playwright tests and axe
- [ ] The area document's As built notes

## Acceptance criteria

- [ ] On a certified file, Save a copy writes a copy with the change, without the certification, and the open file stays certified and unchanged
- [ ] Details names the signature and what it forbids, for a certification and for usage rights
- [ ] On an approval-signed file, Remove signature… applies the op, and the next save asks once and removes the signature
- [ ] Closing a document in Conflict offers Save a copy…, which writes the copy and closes; the file on disk stays signed
- [ ] axe reports no serious violations on the new surfaces

## Post-merge checks

None.

## Tests

Playwright in `apps/web/e2e/organise.e2e.test.ts` (or a new spec beside it) on E2i's fixtures: `certified-p2-outline.pdf`, `approval-organised.pdf` and `usage-rights.pdf`.

## Docs to update

- `docs/design/paper/edit-session-organise.md`: the As built notes for O03 and O06
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation): nothing to tick (E2i closed the web column); note the controls in the round's todo list

## Risks and open questions

- Save a copy on the web downloads where the browser can't write files; the original then stays as it was in the tab, and the copy is a download. The wording must say so.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
