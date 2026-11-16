# D7e — Choose which tabs to reopen after the app quit unexpectedly

| Field | Value |
| --- | --- |
| ID | `D7e` |
| Status | `ready` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D7a](../done/D7a-crash-reporting.md) |
| ADRs | Follows ADR 0008 (the session, D4d) |
| Branch | `feat/d7e-crash-recovered-tab-choice` |
| PR title | `feat(desktop): choose which tabs to reopen after a crash` |
| Size | M (300–500 lines) |
| PR | — |

## Goal

When Lamina PDF quits unexpectedly, the next launch should not reopen every tab at once: if one file causes the crash, reopening it crashes the app again. S08 shows the session's tabs as check boxes before any tab reopens, with the tab that was in front unchecked and badged "Open when Lamina PDF quit" (the drawing says "lamina"). D7a built S08's report part; this todo adds the tab choice above it.

## Context

- [S08 Crash recovered](../../design/paper/desktop-shell.md#s08-crash-recovered): the layout, the states and the accessibility notes.
- [D4d](../done/D4d-session-restore.md) and [D4f](../done/D4f-restore-notice-and-missing-marks.md): the session file, lazy restore and the restore notice.
- [D7a](../done/D7a-crash-reporting.md): `CrashReportDialog` and the question at launch; this todo puts the tab list in the same dialog.

## Scope

**In scope**

- Telling an unexpected quit from a normal one: a marker the app writes at launch and removes when it quits normally, so a native crash, which leaves no report until D7d, counts too.
- After an unexpected quit, the main window shows the dialog before any tab reopens; the other windows wait. "Reopen N tabs" restores the checked tabs in their windows; "Start without tabs" restores none and keeps the session file until the next normal quit.
- The report part as D7a built it, under the tabs, when the app may ask.

**Out of scope**

- Native crash capture: [D7d](../done/D7d-native-crash-minidumps-and-symbols.md).

## Tasks

- [ ] The unexpected-quit marker and its tests.
- [ ] The session's restore plan held back after an unexpected quit; a command that restores the chosen tabs.
- [ ] `CrashReportDialog`'s tab list, its count on the primary button, and its gallery states.
- [ ] An e2e run pair: a forced main-process panic (D7a's `e2e_crash_main_panic`), then a launch that shows the choice.

## Acceptance criteria

- [ ] After an unexpected quit, no tab reopens until the user chooses; the tab in front when the app quit starts unchecked.
- [ ] After a normal quit, the session restores as before (D4d's tests unchanged).
- [ ] The e2e run pair passes on macOS, Windows and Linux.

## Post-merge checks

None.

## Tests

Rust unit tests for the marker and the held-back plan, browser tests for the dialog's tab list, the e2e run pair.

## Docs to update

S08's "As built" notes; the desktop plan's §9 D7 to-dos.

## Risks and open questions

- A crash during start-up, before the marker is written, can't be told from a normal launch; write it first.
