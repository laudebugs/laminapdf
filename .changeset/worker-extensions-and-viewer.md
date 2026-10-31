---
"@lamina/pdf": minor
---

`@lamina/pdf/worker` can be extended: `@lamina/pdf/worker/server`'s `serve({ methods })` adds methods to the same worker, called with `client.request(method, params, { priority, key })`. The worker runs higher priorities first and coalesces calls with the same key. `emit()` sends events to `client.on("event")`, and `connect({ onDiscard })` sees late answers (by default it closes their ImageBitmaps). The worker protocol is now version 2.
