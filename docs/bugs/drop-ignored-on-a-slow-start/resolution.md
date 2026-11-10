# Resolution

`OpenDropZone` adds its listeners in a `useLayoutEffect`, which runs before the screen paints, so the window takes drops and pastes from the moment the home screen invites them.

Proof: 4 runs of `reader.e2e.test.ts` in WebKit failed on a drop or a paste every time before the change, and passed every time after it. The whole reader e2e suite then passed twice in WebKit (129 tests).
