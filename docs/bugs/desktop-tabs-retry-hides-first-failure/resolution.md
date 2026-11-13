# Resolution

The two window tests now start over. `startOver()` in `apps/desktop/e2e/specs/tabs.e2e.ts` closes every window but the one the scenarios began in and leaves that one on its start screen. "Should move a tab to a new window" then opens its three files and shows `flowing_report.pdf` on its page 5 itself, and "should move an encrypted document" opens its fixture on a start screen, so each attempt, and each retry, starts from the same state.

`afterTest` in `apps/desktop/e2e/wdio.conf.ts` adds the attempt to the screenshot's name (`…-1.png`, `…-2.png`), so a retry no longer replaces the first attempt's picture.

## Proof

- With a failure injected after the move into the first attempt, the changed scenario's retry passes and is reported as flaky, and the first attempt's screenshot is kept as `…-1.png`. The same injection into `main`'s scenario fails the retry with CI's `setValue` error.
- All 19 scenarios pass in CI's order under xvfb and a D-Bus session, `large` included.

## Left for later

The first attempt's own failure. If it comes back, the report now names its step and keeps its screenshot.
