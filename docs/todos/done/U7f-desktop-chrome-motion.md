# U7f — The desktop chrome: cursors, tabs, reduced motion and the webview giveaways

| Field | Value |
| --- | --- |
| ID | `U7f` |
| Status | `done` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7b](./U7b-control-states-and-cursors.md), [U7c](./U7c-overlay-enter-and-exit.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md) (desktop shell), [0010](../../decisions/0010-platform-adapter.md) (capability flags) |
| Branch | `claude/dazzling-sagan-s9woz0` |
| PR title | `feat(desktop): tab motion, Reduce motion and a window that feels native` |
| Size | M (200–400) |
| PR | [laudebugs/laminapdf#638](https://github.com/laudebugs/laminapdf/pull/638) |

## Goal

Make the desktop window follow the plan's §5 and feel native:

- the desktop's own chrome (the home screen, recent files, Settings, file-change notices) follows the cursor rule, with labels that can't be selected
- tabs open, close and reorder with motion, and the tab strip keeps its space so a second tab doesn't push the page down
- Settings gets Reduce motion, and the platform reads the system's setting where WebKitGTK misses it (a capability flag, ADR 0010)
- the webview's giveaways go: its context menu in chrome, a white flash on opening in dark mode, overscroll bounce, and an inactive window that looks the same as an active one

## Context

- The desktop [research](../../research/product/motion-and-states/README.md) ([plan §2.3 and §5](../../plans/motion-and-states.md#5-web-and-desktop)): `use-tabs.tsx:116-118`, `settings.tsx:60-66`, `windows.rs:162-176` (no `background_color`), `app.css:40` (`pointer` on link-styled buttons), `file-changes.tsx:148` (`.desktop-missing` has no CSS), [GTK issue 5813](https://gitlab.gnome.org/GNOME/gtk/-/issues/5813)
- Load the `tauri-v2` skill

## Scope

**In scope**

- The cursor tokens and `user-select: none` on the desktop's chrome; a WebdriverIO check of its cursors
- Tab open, close and reorder motion with the tokens; the strip's reserved space
- Settings › Reduce motion; a `reduceMotion` capability read in Rust on Linux
- The window's background colour, no default context menu in chrome, `overscroll-behavior: none`, a dimmed inactive window, `.desktop-missing`'s style

**Out of scope**

- Dragging tabs → U7g
- Joining the title bar and tab strip → the desktop plan (plan §8, 2)

## Tasks

- [x] Cursors and unselectable chrome
- [x] Tab motion and the reserved strip
- [x] Settings and the reduce-motion capability
- [x] Webview giveaways

## Acceptance criteria

- [ ] The WebdriverIO cursor check passes on each OS (passes on Linux here; macOS and Windows in the full lane)
- [x] Opening a second tab moves nothing below the strip
- [ ] On GNOME under Wayland with Reduce animations on, the app's tabs don't slide (the gsettings reading is unit-tested; GNOME itself is a post-merge check)
- [x] Right-clicking the chrome shows no Back, Refresh or Print menu
- [ ] A dark-mode window opens with no white frame (the window is built hidden and shows on the ground colour; a post-merge check on each OS)

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

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **The strip shows from the first tab.** The plan's "reserves its space" and the research's two options (a reserved height, or an animated height) led here: an empty reserved band would be space with no use, and an animated height still moves the page. The design language's "hidden with one tab" changed with it.
- **No `reduceMotion` capability flag.** The system's setting changes while the app runs, and capability flags (ADR 0010) are fixed facts about a platform, so the main process answers a command (`system_motion`) and the webview sets `data-motion`, as the web reader does from its own setting. Shared UI makes no platform check either way.
