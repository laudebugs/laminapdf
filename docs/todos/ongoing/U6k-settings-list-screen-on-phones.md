# U6k — Settings on a phone: a list screen, then each section's own screen

| Field | Value |
| --- | --- |
| ID | `U6k` |
| Status | `ongoing` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#6-round-u6-paper-in-code) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | None |
| ADRs | Follows 0024 (sheets on a phone; from [U1a](../done/U1a-adr-0024-app-shell-ux.md)) |
| Branch | `feat/u6k-settings-list-screen-on-phones` |
| PR title | `feat(ui): settings on a phone open on a list of sections` |
| Size | M (200–400 changed lines of code) |
| PR | — |

## Goal

On a phone, Settings shows its sections as a row of equal buttons above the fields, so "Appearance and accessibility" wraps onto three lines and every new section makes the row tighter. The owner chose, from three mockups, the design the Paper spec already describes: "the settings nav becomes a list screen; each block is its own screen" ([account-cloud-product.md](../../design/paper/account-cloud-product.md)). This PR makes the phone's Settings sheet open on that list, with each section's name over a one-line summary of what it is set to, and each section open as its own screen with a way back.

## Context

- [`packages/ui/src/reader/settings.tsx`](../../../packages/ui/src/reader/settings.tsx): `SettingsRegistry`, `useSettingsSection` and `SettingsDialog`.
- [`packages/ui/src/styles/settings.css`](../../../packages/ui/src/styles/settings.css): the sheet's layout, which [`settings-cut-off-on-phones`](../../bugs/settings-cut-off-on-phones/detail.md) last changed.
- The dialog becomes a sheet on a phone-wide viewport (`usePhoneLayout` in `overlays/dialog.tsx`); the sheet's CSS keys off `.paper-sheet`.

## Scope

**In scope**

- `SettingsSection.summary` and an optional `summary` argument to `useSettingsSection`.
- `SettingsDialog` on a phone: the list screen, the section screen with All settings, focus moved between them, the list again on each opening.
- Summaries for the reader's Appearance, Storage and About sections.

**Out of scope**

- The desktop rail, which stays as it is.
- The section order (About before Storage, from registration order).

## Tasks

- [ ] Add `summary` to `SettingsSection` and `useSettingsSection`.
- [ ] Give `SettingsDialog` a `data-screen` of `list` or `section`, a back button and focus handling.
- [ ] Style the phone's list rows and back button in `settings.css`; hide summaries, chevrons and the back button wider than a phone.
- [ ] Add `appearanceSummary`, the storage and About summaries, and their messages.
- [ ] Update the phone Settings browser test and the UI README.

## Acceptance criteria

- [ ] On a 360 px viewport Settings opens on the list, each row a name over its summary, with no fields shown.
- [ ] A row opens its section with its heading, and focus on All settings; All settings returns to the list with focus on that row.
- [ ] A section's options all fit inside the sheet.
- [ ] At 1280 px the list sits beside the fields with no summaries or back button.

## Post-merge checks

- [ ] On a real iPhone and Android phone: open Settings, open and leave each section.

## Tests

`packages/ui/test/browser/settings.browser.test.tsx` in Chromium, Firefox and WebKit.

## Docs to update

The UI README's Settings paragraph.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../guidelines/maintainability.md) for code, [docs writing](../guidelines/docs-writing.md) and [examples](../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
