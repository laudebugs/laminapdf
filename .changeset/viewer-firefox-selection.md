---
"@laminapdf/viewer": patch
---

In Firefox, selecting a page's text with the mouse works again after the user has focused a text field on the page, such as the reader's page box. A drag that leaves the viewer can now highlight your app's own text in Firefox too, as it already could in Chromium and WebKit; set `user-select: none` on text you want to keep out. Copy still writes only the page text.
