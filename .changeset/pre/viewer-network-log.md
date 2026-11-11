---
"@laminapdf/viewer": minor
---

The document worker now counts every network request it makes from the moment it starts: `client.networkLog()` returns them all, with the requests its Content Security Policy blocked, and the `network` and `csp-violation` events report each as it happens. `watchNetwork` does the counting and is exported for the page too: it wraps `fetch`, `XMLHttpRequest`, `WebSocket` and `sendBeacon` and reads everything else the browser loads from Resource Timing.
