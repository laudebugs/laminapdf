---
"@laminapdf/viewer": minor
---

An app can show its own cursor over a page's words, such as a drawing tool's crosshair, in place of the I-beam: it sets the custom property `TEXT_CURSOR_PROPERTY` (`--lamina-text-cursor`) on the viewer's element. Each page also names how far its picture has drawn in `data-render` (`placeholder`, `low`, `sharp` or `error`), so an app can style it, for example to fade the picture in.
