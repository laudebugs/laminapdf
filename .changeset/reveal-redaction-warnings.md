---
"@laminapdf/wasm": patch
---

In Rust, `Reveal::redaction_warnings()` gives the warnings `Document::redaction_warnings` would (`covered-text`, `redaction-not-applied` and `text-in-earlier-revision`, in the same order and with the same counts) from what `Document::reveal` already found. When you need both, call `reveal` and read the warnings from its result. The file's pages and earlier revisions are then read once instead of twice.
