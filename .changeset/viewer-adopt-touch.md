---
"@laminapdf/viewer": minor
---

`viewer.adoptTouch(down)` hands a touch a drawing tool took back to the viewer's gestures, as if they had seen it go down. A tool that draws with one finger calls it when a second finger lands, so the two fingers scroll or zoom. The gestures now also leave alone any pointer a tool took or ignored first (`preventDefault` on its `pointerdown`), so a pen can draw while a finger beside it scrolls.
