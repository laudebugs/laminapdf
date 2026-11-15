# U7f — The desktop chrome: tabs, reduced motion and the web tells

| Field | Value |
| --- | --- |
| ID | `U7f` |
| Status | `ready` |
| Plan | [Motion and interaction states](../../plans/motion-and-states.md) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7b](./U7b-control-states-and-cursors.md), [U7c](./U7c-overlay-enter-and-exit.md) |
| ADRs | Follows 0024 (tokens in layers), 0010 (capability flags) |
| Branch | `feat/u7f-desktop-chrome-motion` |
| PR title | `feat(desktop): tab motion, Reduce motion and a window that feels native` |
| Size | M (200–600) |
| PR | — |

## Goal

Make the desktop window feel native and follow plan §5: tabs open, close and reorder with motion; the tab strip keeps its space so a second tab doesn't push the page down 34 px; chrome labels can't be selected; Settings gets Appearance and Reduce motion, and the platform reads the system's reduce-motion setting where WebKitGTK misses it (a capability flag, ADR 0010); and the webview's tells go: its context menu in chrome, the white flash on opening in dark mode, overscroll bounce, and no look for an inactive window.

## Context

- The desktop research in [plan §2.3 and §5](../../plans/motion-and-states.md#5-web-and-desktop): `use-tabs.tsx:116-118`, `settings.tsx:60-66`, `windows.rs:162-176` (no `background_color`), `file-changes.tsx:148` (`.desktop-missing` has no CSS), GTK issue 5813
- Load the `tauri-v2` skill

## Scope

**In scope**

- Tab open, close and reorder motion with the tokens
- The strip's reserved space
- `user-select: none` on chrome
- Settings › Appearance and Reduce motion; a `reduceMotion` capability read in Rust on Linux
- The window's background colour, no default context menu in chrome, `overscroll-behavior: none`, a dimmed inactive window, `.desktop-missing`'s style

**Out of scope**

- Dragging tabs → U7g
- Joining the title bar and tab strip → the desktop plan (plan §8, 2)

## Tasks

- [ ] Tab motion
- [ ] Reserved strip
- [ ] Unselectable chrome
- [ ] Settings and the reduce-motion capability
- [ ] Web tells

## Acceptance criteria

- [ ] Opening a second tab moves nothing below the strip
- [ ] On GNOME under Wayland with Reduce animations on, the app's tabs don't slide
- [ ] Right-clicking the chrome shows no Back, Refresh or Print menu
- [ ] A dark-mode window opens with no white frame

## Post-merge checks

- [ ] On macOS, Windows and Linux: open, close and reorder tabs; toggle the OS's reduce-motion setting

## Tests

- The desktop's WebdriverIO scenarios on each OS (the full lane)
- Rust tests for the capability

## Docs to update

- `apps/desktop/README.md`
- Plan §7: point this todo at `done/`

## Risks and open questions

- Reading GNOME's setting needs a D-Bus call (`org.gnome.desktop.interface enable-animations`); keep it in the main process, not the engine

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
