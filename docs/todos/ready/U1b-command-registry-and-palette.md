# U1b — Command registry and command palette (`@lamina/ui`)

| Field | Value |
| --- | --- |
| ID | `U1b` |
| Status | `ready` |
| Plan | [UI/UX §6 U1. Shell foundations](../../plans/ui-ux.md#u1-shell-foundations) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C4a](./C4a-ui-chrome-and-platform.md), [U1a](./U1a-adr-0024-app-shell-ux.md) |
| Blocks | [C7a](./C7a-accessibility-and-theming.md), [D4b](./D4b-native-menus.md), [U2a](./U2a-selection-surfaces-and-copy-modes.md) |
| ADRs | Implements 0024 (command registry); follows 0009, 0010 |
| Branch | `feat/u1b-command-registry-and-palette` |
| PR title | `feat(ui): add the command registry and the command palette` |
| Size | L (500–900 lines incl. tests) |
| PR | — |

## Goal

Make every reader action a registered command and give power users a Cmd/Ctrl+K palette, while menus, toolbars, context menus and the shortcut help read the same registry so labels, shortcuts and enabled state never drift between surfaces or shells.

## Context

- [UI/UX plan §5.2](../../plans/ui-ux.md#52-command-registry-and-palette-r3) and ADR 0024 (from [U1a](./U1a-adr-0024-app-shell-ux.md)).
- [Research: command palette](../../research/product/pdf-app-product-and-ux.md#a-command-palette-built-on-one-command-registry) — cmdk (MIT, React 18+, custom filter, async items, no built-in nested pages); kbar less active.
- [C4a](./C4a-ui-chrome-and-platform.md) — toolbar and `ReaderController`, whose actions become commands here.
- [TypeScript guidelines](../../guidelines/typescript.md) — named exports, strict types.

## Scope

**In scope**

- `packages/ui/src/commands/`: `Command`, `CommandContext`, `CommandRegistry` (register, unregister, lookup, `enabled` evaluation, run with typed args), argument parsers (page range `4-8`, `4,6,9-12`, page label, zoom `150%`, `fit width`).
- Shortcuts: one declaration per command with `Mod` mapping to Cmd/Ctrl, per-platform overrides, a keydown dispatcher that ignores events from text inputs, and a collision table against reserved browser/OS shortcuts.
- `CommandPalette` on cmdk: Cmd/Ctrl+K (with `preventDefault`, as Chrome and Firefox otherwise focus their search box); fuzzy ranking over title and keywords, recents first (kept locally, try/catch around storage); inline argument parsing (`go to 12`, `zoom 150`, `extract 4-8` once the editor registers it); a pushed page for commands that need arguments; shortcut chips on every row; disabled commands shown with the reason.
- Register the reader v0.1 command set from ADR 0024's appendix; C4a's toolbar buttons call commands by id.
- `useCommand(id)` and `useShortcutMap()` hooks for menus and the help dialog (C7a) and for desktop menus (D4b).

**Out of scope** (and which todo covers it)

- Shortcut help dialog UI → [C7a](./C7a-accessibility-and-theming.md) (reads `useShortcutMap()`).
- Native menus → [D4b](./D4b-native-menus.md).
- Editor commands → each E round registers its own.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Check cmdk's licence and that of its Radix dependencies; `cargo xtask npm-licenses` passes.
- [ ] Implement the registry and argument parsers with unit tests for every parser (valid, invalid, out of range, page labels).
- [ ] Implement the dispatcher and the collision test; document reserved shortcuts (browser Ctrl+F/P/S, Ctrl+W/T/N, Firefox Quick Find `/` and `'`, macOS Cmd+Q/W/H, Windows Alt+F4); the collision test covers every entry.
- [ ] Build the palette with APG combobox semantics (focus stays in the input, `aria-activedescendant`, `aria-expanded`, live count of results).
- [ ] Port C4a's toolbar actions to commands; delete duplicated handlers.
- [ ] Add a palette demo to `examples/ui-demo.tsx` with a fake viewer.

## Acceptance criteria

- [ ] Every action reachable from the toolbar is a registered command, and the palette runs each one (test enumerates the registry).
- [ ] `go to 12`, `zoom 150` and a page-label jump work from the palette in one step; invalid arguments show a message and run nothing.
- [ ] No two commands share a shortcut on any platform, and none collides with the reserved list (test).
- [ ] Keyboard-only and screen-reader checks: axe reports zero serious/critical violations with the palette open; VoiceOver announces result counts.
- [ ] Opening the palette and filtering 30 commands stays under 16 ms per keystroke on the perf harness machine.

## Post-merge checks

None.

## Tests

`packages/ui/src/commands/*.test.ts` (registry, parsers, dispatcher, collisions); `CommandPalette.test.tsx` (Vitest browser mode, three engines); axe on the demo page.

## Docs to update

`packages/ui/README.md` (how to register a command, how shells read the registry); tick the registry and palette items in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- cmdk's maintenance pace: the registry is library-independent, so the palette view can be swapped.
- Typed arguments can grow into a mini-language: keep parsers to page ranges, labels, zoom and file pickers until a mode needs more.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
