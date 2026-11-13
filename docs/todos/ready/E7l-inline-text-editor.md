# E7l — Edit existing text inline in the reader (A04)

| Field | Value |
| --- | --- |
| ID | `E7l` |
| Status | `ready` |
| Plan | [E7 plan](../../plans/editor/e7-editing-existing-content.md) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6e](./E6e-content-tools-ui.md), [E7f](./E7f-font-handling-and-styling.md) |
| ADRs | Follows ADR 0022, 0024 |
| Branch | `feat/e7l-inline-text-editor` |
| PR title | `feat(ui): edit existing text in place in the reader` |
| Size | L (600–1,500) |
| PR | — |

## Goal

A04 in the web and desktop readers: the Edit text tool turns a text block into an editor over its lines; typing applies E7e's `EditText`, and the Paragraph inspector applies E7f's `SetTextStyle`, with the notices A04 draws when a font lacks letters or text can't be edited in place.

## Context

- **Design (Paper):** [A04 Edit existing text inline](../../design/paper/add-edit-content.md#a04-edit-existing-text-inline). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E7 plan](../../plans/editor/e7-editing-existing-content.md) §2 and §4
- [E6 plan §4](../../plans/editor/e6-adding-content.md#4-crate-and-package-changes): Edit mode's tools and handles, which this reuses

## Scope

**In scope**

- The Edit text tool (`tool.edit-text`, `E`), double-click with Select, and Edit text in A05's Text block inspector
- The inline editor over a text block's lines, committing one op per edit as A04's history rules say
- The Paragraph inspector: font, size, colour, style, alignment, spacing
- A04's states: original font, substituted (missing characters named), not editable in place (with the reason), reverted

**Out of scope** (and which todo covers it)

- Find and replace → [E7g](./E7g-search-and-replace.md)
- Paragraph reflow → a later round ([E7 plan §10](../../plans/editor/e7-editing-existing-content.md#10-decisions-on-the-open-questions))

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.02`, `A.03`, `A.04`, `A.05`, `A.06`
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`, `apps/desktop`
- **`Op` variants:** none new (the reader drives the core todo's ops)
- **`SaveConstraint`:** as the core todo's: `Free` → incremental; a certified file becomes `Conflict` from the first edit, with the banner naming the certifier
- **Web vs desktop:** identical but for fonts on the computer (`systemFonts`, E6h)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.02`, `A.03`, `A.04`, `A.05` → `round-trip` (web, desktop); `A.06` → `write` (web, desktop)

## Tasks

- [ ] Tool and entry points
- [ ] Inline editor
- [ ] Paragraph inspector
- [ ] Notices and states
- [ ] e2e

## Acceptance criteria

- [ ] A substitution is shown before it is committed, naming the missing characters and the substitute (A04)
- [ ] Checked in the built reader in a browser, as a user does it (CLAUDE.md), and on the desktop app
- [ ] The reader's startup scripts don't grow: everything loads with the editor's lazy chunk

## Post-merge checks

None.

## Tests

- Playwright: edit a word, delete a line, change size and colour, and edit text the subset can't draw, then save, reopen and extract
- WebdriverIO: a font on the computer completes a subset on each desktop OS

## Docs to update

- User help for the screens this adds
- [E7 plan](../../plans/editor/e7-editing-existing-content.md) §7: point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
