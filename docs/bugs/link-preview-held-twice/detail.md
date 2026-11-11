# The link preview reported the pointer coming onto it twice

## What happened

`@laminapdf/ui`'s browser test "the popover (mouse and keyboard) › holds while the pointer is on it" failed now and then with `expected [ true, true, false ] to deeply equal [ true, false ]`: the popover called `onHold(true)` twice before `onHold(false)`. It failed on its first run and its retry in Firefox on CI (PR #412, run 42962374819), and in Chromium on a loaded machine. Branches that never touched the link preview hit it, so the fault came from main, from the link previews of #407.

A user could not see the fault. The viewer's `LinkLayer.hold` (`packages/viewer/src/layers/link-layer.ts`) only sets a flag and clears the dismiss timer when it gets `true`, so a second `true` changes nothing. Only one `false` ever arrived, and it still started the dismiss timer, so the preview neither stuck nor hid early.

## How to reproduce it

Move the pointer onto a node that React doesn't render, then onto the popover. Each time the pointer comes from such a node, in Chromium, Firefox and WebKit, React calls `onPointerEnter` twice. A probe that ran the test's hover and unhover 150 times in one page failed 149 of them in Chromium. The probe is opt-in, in `packages/ui/test/browser/link-preview.browser.test.tsx`: run `VITE_LINK_PREVIEW_PROBE=1 pnpm --filter @laminapdf/ui exec vitest run -c vitest.browser.config.ts --project chromium test/browser/link-preview.browser.test.tsx -t probe` against the component before the fix. Its first run passed, because the pointer hadn't yet entered the page; after that, each unhover left the pointer on `<body>`. A log of the native events in a failing run:

```
pointerout   target=body                 relatedTarget=div.paper-link-preview   → onHold(true)
pointerover  target=div.paper-link-preview relatedTarget=body                   → onHold(true)
pointerenter target=div.paper-link-preview relatedTarget=body
...
pointerout   target=div.paper-link-preview relatedTarget=body                   → onHold(false)
pointerleave target=div.paper-link-preview relatedTarget=body
```

The browser sent one `pointerenter` and one `pointerleave`; React reported two enters.

## What was ruled out

- **The popover remounting.** The node under the pointer after the hover was the one found before it, and only one native `pointerenter` fired.
- **The placement moving the card.** Both enters came within 0.1 ms of each other, from one pointer move at one point, with no render between them.
- **Playwright sending two moves.** One `pointermove` arrived per hover.

## Cause

React doesn't listen for `pointerenter` and `pointerleave`. It makes up `onPointerEnter` and `onPointerLeave` from `pointerout` and `pointerover` (its `EnterLeaveEventPlugin`). A `pointerout` from node A to node B reports a leave on A and an enter on B. The `pointerover` on B that follows is meant to be skipped, but React 19.3 skips it only when its `relatedTarget` is a node React rendered or a root's container. When the pointer comes from a node outside React, such as `<body>` (which holds the popover's portal but is no root), the `pointerover` isn't skipped, so React reports the enter a second time.

The test flaked because it depended on where the pointer was before it. When the browser hadn't yet placed the pointer in the test's page, the hover's `pointerover` had no `relatedTarget` and React reported one enter. When an earlier test had left the pointer over the page's `<body>`, it reported two. The probe log shows the browser placing the pointer on `<body>` by itself after a test removed the nodes under it. How soon that happens depends on the machine's load.

The reader most likely gets the same double enter, though it wasn't measured there: the viewer draws its links without React, so a pointer that moves from a link onto its preview comes from a node outside React.
