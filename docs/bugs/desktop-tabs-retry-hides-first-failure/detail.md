# The desktop tabs scenario's retry hides why its first attempt failed

## What we saw

On laudebugs/laminapdf#544, the full lane's `Desktop e2e (linux)` failed in `tabs.e2e.ts`, on "should move an encrypted document without asking for its password again", with `Can't call setValue on element with selector "input[aria-label="Password"]" because element wasn't found`. That is the scenario's first step after it opens the encrypted fixture. The job passed on the PR's next push with no change to the desktop app, and it passed on Linux, macOS and Windows in #529, which added the scenario.

## What the logs show

The error is the retry's, not the first attempt's: WebdriverIO's spec reporter prints the last attempt's error. The app's log (`captureBackendLogs`) shows how far the first attempt got:

| Time | App log | Step |
| --- | --- | --- |
| 15:55:55.86 | engine ready, window `doc-1` | the previous test moves a tab to a new window |
| 15:55:56.66 | engine stopped, window `doc-1` | it closes that window |
| 15:55:56.95 | engine ready, window `doc-2` | the encrypted file opened, was unlocked and moved to a new window |
| 15:55:57.30 | `doc-2` interactive | |

A passing local run logs the same windows in the same order, then `doc-2` stopping and a third window for the second move. The failing run has neither. So the first attempt opened the file, typed its password and moved it, then failed before the second move: waiting for the moved tab's pages, for its page 1 drawn sharp, or checking that no password box showed. The retry then opened the fixture again in a window where it was already open and unlocked; opening a file that is open shows its tab and asks for no password, so `setValue` found no field.

The screenshot the job keeps doesn't help. `afterTest` names it after the test's title only, so the retry's picture replaced the first attempt's: it shows the fixture open and unlocked, as the retry found it.

## Cause

The scenario's window tests weren't safe to retry. "Should move a tab to a new window" used the tabs the two tests before it left open, and "should move an encrypted document" used the window the test before it left. A failed attempt leaves documents and windows open, and the retry, built on them, fails at once with an error that names the wrong step.

Injecting a failure after the move into the first attempt reproduces CI's error exactly. With `main`'s scenario the retry fails with the same `setValue` error.

What failed in the first attempt is not known. Eight local runs of the scenarios under xvfb, five of them with the CPU loaded and three with the 1 GB `large` scenario before them, all passed.
