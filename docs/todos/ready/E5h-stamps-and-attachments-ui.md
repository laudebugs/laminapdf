# E5h — Stamps and file attachments in the interface

| Field | Value |
| --- | --- |
| ID | `E5h` |
| Status | `ready` |
| Plan | [E5 plan §6](../../plans/editor/e5-review-stamps-interchange.md#6-differences-between-the-shells) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4f](./E4f-annotation-tools-ui.md), [E5a](../done/E5a-review-stamps-interchange-feature-plan.md), [E5c](./E5c-stamps-and-attachments.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0024](../../decisions/0024-app-shell-ux.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e5h-stamps-and-attachments-ui` |
| PR title | `feat(ui): the stamp picker, custom stamps and file attachments` |
| Size | M (200–400): the picker and its library, the attach-file tool, the inspector and Play |
| PR | — |

## Goal

The stamp picker of C05 (standard, dynamic and custom stamps), custom stamps kept on the device, the attach-file tool, the attachment inspector and Play for audio comments.

## Context

- **Design (Paper):** [C05 Stamps and file attachments](../../design/paper/comment-review.md#c05-stamps-and-file-attachments). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E5 plan §6 and §10](../../plans/editor/e5-review-stamps-interchange.md#10-decisions-on-the-open-questions): Play on both shells

## Scope

**In scope**

- `comment.place-stamp`, `stamps.add-custom`, `comment.attach-file`, `attachment.save`, `attachment.replace`, `annot.delete`
- Custom stamps (PNG and JPEG) in platform storage; dynamic stamps resolved from the author name and the date when placed
- The attachment inspector: save, replace, description, icon and its colour; O14's warning before saving a program
- Play through `sound_as_wav` in an `<audio>` element, on both shells; the deviation from C05 recorded in the area document

**Out of scope** (and which todo covers it)

- The desktop → [E5j](E5j-desktop-review.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.07`, `D.08`, `D.09` in the interface
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`SaveConstraint`:** the banner and gating from E1e
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.07`, `D.08` → `round-trip` (web); `D.09` → `read` (web)

## Tasks

- [ ] The picker and the custom-stamp library
- [ ] The attach-file tool and the inspector
- [ ] Play
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit: place each kind of stamp and an attachment, save, reopen in pdf.js with the stamps drawn and the file's bytes kept
- [ ] A sound from `sound.pdf` plays in each browser
- [ ] axe reports no serious violation in the picker and the inspector

## Post-merge checks

None.

## Tests

- Playwright specs in `apps/web/e2e/`
- Vitest for the stamp library

## Docs to update

- User help: stamps and attachments
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
