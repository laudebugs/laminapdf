---
"@laminapdf/viewer": minor
---

Each page is now a region that screen readers name and can land on. `createViewer({ pages: { name, roleDescription, focusOnJump } })` sets each page's name ("Page 4 of 120" unless the app gives its own, such as "Page iv, 4 of 120") and whether a jump moves focus to the page it lands on. Links, the outline, thumbnails and go to page are jumps; a find match leaves focus in the find field.
