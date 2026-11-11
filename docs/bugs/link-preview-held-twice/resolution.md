# Resolution

The popover listens for the browser's own `pointerenter` and `pointerleave` (`useHover` in `packages/ui/src/overlays/link-preview.tsx`) in place of React's `onPointerEnter` and `onPointerLeave`. The browser sends one `pointerenter` each time the pointer comes onto the card, wherever it comes from, so `onHold` hears each change of the pointer once. The prop's contract stays the same.

## Proof

- **A regression test that fails without the fix.** "holds once when the pointer comes from outside React, as from the viewer's links" moves the pointer onto a plain element, then onto the popover, then back. Before the fix it failed in Chromium, Firefox and WebKit, on the run and its retry, with `[ true, true, false ]`; after it, it passes in all three.
- **The probe again.** The 150 hovers that failed 149 times in Chromium passed 150 of 150 in each browser.
- **Repeated under load.** With all 8 cores busy, `link-preview.browser.test.tsx` (13 tests, both hover tests among them) ran with retries off and passed 50 of 50 times in Chromium, 50 of 50 in WebKit and 50 of 51 in Firefox. The Firefox run that failed ran no test: the browser's page crashed as it started, which Vitest reports as `VITEST_BROWSER_CONNECTION_CLOSED`.

## Left for later

The toast region (`packages/ui/src/overlays/toast-region.tsx`) also uses React's `onPointerEnter`, and could get the same double enter. It only sets a hovered state to `true`, so a second enter changes nothing, and it was left as it is.
