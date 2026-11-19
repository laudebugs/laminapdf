---
"@laminapdf/viewer": minor
---

`viewer.adoptTouch(down)` gives the viewer's gestures a touch that a drawing tool had taken. Pass the touch's `pointerdown` event, and the gestures treat it as if they had seen it start. A tool that draws with one finger calls it when a second finger lands, so the two fingers scroll or zoom. The gestures now also ignore any pointer whose `pointerdown` a tool has already handled with `preventDefault`, so a pen can draw while a finger beside it scrolls.
