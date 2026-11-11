# U6k — Settings on a phone: a list screen, then each section's own screen

| Field | Value |
| --- | --- |
| ID | `U6k` |
| Status | `done` |
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

- [x] Add `summary` to `SettingsSection` and `useSettingsSection`.
- [x] Give `SettingsDialog` a `data-screen` of `list` or `section`, a back button and focus handling.
- [x] Style the phone's list rows and back button in `settings.css`; hide summaries, chevrons and the back button wider than a phone.
- [x] Add `appearanceSummary`, the storage and About summaries, and their messages.
- [x] Update the phone Settings browser test and the UI README.

## Acceptance criteria

- [x] On a 360 px viewport Settings opens on the list, each row a name over its summary, with no fields shown.
- [x] A row opens its section with its heading, and focus on All settings; All settings returns to the list with focus on that row.
- [x] A section's options all fit inside the sheet.
- [x] At 1280 px the list sits beside the fields with no summaries or back button.

## Post-merge checks

- [ ] On a real iPhone and Android phone: open Settings, open and leave each section.

## Tests

`packages/ui/test/browser/settings.browser.test.tsx` in Chromium, Firefox and WebKit.

## Docs to update

The UI README's Settings paragraph.

## Risks and open questions

- The reader's startup JavaScript was 271,318 bytes gzipped before this change and 272,009 after, over the 272,000-byte waiver. The owner raised the waiver to 275,000 (`apps/web/size-budget.json`; [#380](https://github.com/laudebugs/laminapdf/issues/380) still brings it back to 200,000).

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **The hook takes an options object:** `useSettingsSection(registry, id, title, render, { shows, summary })`. The review found six positional arguments too many, with About passing an empty `shows` only to reach `summary`.
- **The phone is the Dialog's phone:** `SettingsDialog` asks `usePhoneLayout(isOpen)`, which `overlays/dialog.tsx` now exports, so a phone turned wider while Settings is open keeps both the sheet and its two screens.
- **The summary describes the row:** it is the row's `aria-describedby` on a phone and is hidden from its name, so "Appearance and accessibility" stays the name. Wider than a phone it is hidden and describes nothing.
- **Summaries:** Appearance gives the theme, the accent (none under High contrast) and single-key shortcuts ("Match system · Green · Shortcuts on"); Storage gives which of recent files and reading positions it keeps; About gives the version.
- **No current row on a phone's list:** `aria-current="page"` marks the section beside the list on wider screens only.
- **The storage section is its own hook** (`useStorageSection` in `apps/web/src/recent.tsx`): the summary pushed `useRecent` past Biome's 60-line limit.
- **Size:** the reader's startup JavaScript went from 271,318 to 272,009 bytes gzipped; the owner raised the waiver to 275,000.
