# D7e — Choose which tabs to reopen after the app quit unexpectedly

| Field | Value |
| --- | --- |
| ID | `D7e` |
| Status | `done` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D7a](./D7a-crash-reporting.md) |
| ADRs | Follows ADR 0008 (the session, D4d) |
| Branch | `feat/d7e-crash-recovered-tab-choice` |
| PR title | `feat(desktop): choose which tabs to reopen after a crash` |
| Size | M (300–500 lines) |
| PR | [#666](https://github.com/laudebugs/laminapdf/pull/666) |

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

- [x] The unexpected-quit marker and its tests.
- [x] The session's restore plan held back after an unexpected quit; a command that restores the chosen tabs.
- [x] `CrashReportDialog`'s tab list, its count on the primary button, and its gallery states.
- [x] An e2e run pair: a run that ends without a normal quit, then a launch that shows the choice (see Decisions: the service's kill, not a forced panic).

## Acceptance criteria

- [x] After an unexpected quit, no tab reopens until the user chooses; the tab in front when the app quit starts unchecked.
- [x] After a normal quit, the session restores as before (D4d's tests unchanged).
- [x] The e2e run pair passes on macOS, Windows and Linux (CI on #666, 0d4fa8a38).

## Post-merge checks

None.

## Tests

Rust unit tests for the marker and the held-back plan, browser tests for the dialog's tab list, the e2e run pair.

## Docs to update

S08's "As built" notes; the desktop plan's §9 D7 to-dos.

## Risks and open questions

- A crash during start-up, before the marker is written, can't be told from a normal launch; write it first.

## Decisions and deviations

- **The marker** is `running` in the config folder, written first in `setup` and removed at a normal quit, a restart and an update's install (restored if the install fails). A second launch hands its files over before `setup`, so it never touches it.
- **The front tab** is the shown tab of the window focused last. The session file now keeps that window (`focused`, optional, written again when the focus moves); a file from before it marks the first window's.
- **A dialog of its own.** The tab choice is `CrashRecoveredDialog`, with D7a's report part under the tabs; `CrashReportDialog` stays as S08's "No session to restore" state. Both share `ReportPreview` and `ReportAnswers`.
- **Always send reports** is read-only until Send is checked, with its reason as its description, rather than `aria-disabled`: react-aria's check box has no such state.
- **Closing the main window before choosing** counts as Start without tabs, so the choice never waits for the rest of the run; only a crash before the choice keeps the mark for the next launch.
- **The e2e pair** ends its first run with the service's kill, not D7a's `e2e_crash_main_panic`: the panic ends the app, but WebdriverIO then fails the run when it can't close its session. Any end but a normal quit leaves the mark, which the marker's unit tests cover. Every other run clears the mark the kill leaves before it starts.
- **Not built:** S08's "Reporting already on" line (with reporting on, D7a sends without a question), and a crash in the 2 s after a choice, before the other windows report their tabs, loses the chosen tabs.

The full list is in S08's As built note.
