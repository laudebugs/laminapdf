# Q4w — The theme switch as a list with an icon per theme

| Field | Value |
| --- | --- |
| ID | `Q4w` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [W1g](./W1g-site-select.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (no inline styles; the inline script's hash) |
| Branch | `claude/dazzling-sagan-s9woz0` (with Q4v, at the owner's request) |
| PR title | `feat(docs): boxed tables with a header band, and a button that hides "On this page"` |
| Size | S–M |
| PR | [laudebugs/laminapdf#627](https://github.com/laudebugs/laminapdf/pull/627) |

## Goal

The owner asked for the docs' light and dark switch to open a list drawn by the site, not the platform's own, with an icon beside each choice. The switch was W1g's native `<select>`, whose list each browser draws its own way, with no room for icons.

## Context

- `@laminapdf/site-kit`'s `Select.astro` was the native select, and the docs' theme switch (`apps/docs/src/components/ThemeSelect.astro`, Starlight's `ThemeSelect` override) its only user.
- Starlight's theme provider fills in a native `<select>` before the first paint (`updatePickers`); a list of our own needs its own inline script for that.
- Design language §7.19 (the select field) and §7.26 (the listbox popup) give the look; ARIA's select-only combobox gives the keyboard.

## Scope

**In scope**

- `MenuSelect.astro`, `menu-select.ts` and `menu-select.css` in the site kit in place of `Select.astro` and `select.css`: a button with the value's icon and label that opens a listbox of options with icons, by the keyboard and the pointer.
- The theme switch on it, with Sun, Moon and Laptop, the stored choice shown before the first paint.
- Two Paper icons, `sun` and `moon`, by the icon package's rules, with their rows in the design language's §5.

**Out of scope**

- A select for the web reader, which has `@laminapdf/ui`'s.

## Tasks

- [x] Draw `sun` and `moon`.
- [x] Write the select and its browser tests; remove the native one.
- [x] Move the theme switch onto it, and rewrite its e2e tests.
- [x] Check it in a browser: light and dark, the keyboard, the phone menu.

## Acceptance criteria

- [x] The list opens with Light, Dark and Auto, each with its icon, and the button shows the chosen one's.
- [x] The keyboard opens, moves and chooses with the focus on the button; Escape and a click outside close it.
- [x] A choice shows at once, survives a reload with no flash and no CSP error, and the header's and the phone menu's switches stay in step.
- [x] The site kit's browser tests and the docs' e2e tests pass in Chromium, Firefox and WebKit.
- [ ] CI green.
