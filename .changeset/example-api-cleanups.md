---
"@laminapdf/sdk": minor
---

`Document::save` no longer takes a `CancelToken`. To stop a save from another thread, call `save_with(&options, &cancel)` instead. In TypeScript, `save()`, `saveWithReport()` and `SaveJob.step()` now return `Uint8Array<ArrayBuffer>`, which `Response` and `Blob` accept without a cast. Smaller additions: `RectPt::from_points(x, y, width, height)`, `SetInfo::patch(patch)` for an edit that records no `/ModDate`, and `new()` on `BuildOptions`, `OpenOptions`, `PngOptions`, `RetryOptions` and `TextBoxOptions`. You can now ignore the run that `Surface::text` returns: the text is drawn either way.
