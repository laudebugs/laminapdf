# U7l — The desktop pass: every desktop screen and tab dragging on macOS, Windows and Linux

| Field | Value |
| --- | --- |
| ID | `U7l` |
| Status | `ready` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7f](../done/U7f-desktop-chrome-motion.md), [U7h](../done/U7h-delight-pass.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md) (the desktop shell) |
| Branch | `test/u7l-desktop-pass` |
| PR title | `test(desktop): a pass over the desktop app's states and motion on each OS` |
| Size | S (< 200) |
| PR | — |

## Goal

Do the desktop part of U7h's by-hand pass: every desktop screen in the [screen inventory](../../design/paper/screens.md), and tab reordering and dragging out, by hand on macOS, Windows and Linux, against the plan. U7h covered the web reader and both sites; the desktop app needs U7f's tab motion merged and a machine of each OS.

## Context

- [Plan §3, §4 and §5](../../plans/motion-and-states.md#5-web-and-desktop)
- [U7f](../done/U7f-desktop-chrome-motion.md) and [U7g](../done/U7g-pointer-tab-drag.md): what the desktop chrome and tab dragging do now

## Scope

**In scope**

- Each desktop screen and the tab strip, in each OS's webview (WebKit on macOS, WebView2 on Windows, WebKitGTK on Linux), with the system's reduce-motion setting off and on
- Findings fixed when small, or written up as todos

**Out of scope**

- The web reader and the sites → [U7j](./U7j-reader-pass-fixes.md), [U7k](./U7k-sites-pass-fixes.md)

## Tasks

- [ ] macOS
- [ ] Windows
- [ ] Linux (GNOME on Wayland, where WebKitGTK misses the reduce-motion setting; U7f reads it through `gsettings`)

## Acceptance criteria

- [ ] A list in the PR of every desktop screen and tab action checked on each OS, with what was found

## Post-merge checks

None.

## Tests

- None beyond any fixes' own

## Docs to update

- Plan §7: this row

## Risks and open questions

- Needs a Windows and a Linux machine by hand; CI's desktop e2e covers only what WebDriver can drive

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
