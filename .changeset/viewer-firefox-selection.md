---
"@laminapdf/viewer": patch
---

In Firefox, selecting a page's text with the mouse works again after a text field on the page, such as the reader's page box, has had the focus. The viewer's root no longer sets `user-select: none`; the canvases, the other layers' slots and the hidden structure tree set it on themselves.
