---
"@laminapdf/viewer": patch
---

A mouse drag selects a page's text again. The layer slots above the text covered the whole page and took every press; every slot but `text` now lets the pointer through. A layer added with `addLayer` gets such a slot too, so an element in it that should be clicked, such as a link, sets `pointer-events: auto` on itself.

A selection's colour no longer hides the text it selects: the text layer (`.lamina-text-layer`) now has `mix-blend-mode: multiply`, so an opaque `::selection` tint shows under the ink.
