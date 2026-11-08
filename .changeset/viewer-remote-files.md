---
"@laminapdf/viewer": minor
---

The viewer opens remote PDFs: `client.open({ url })` makes the document worker fetch the file itself with HTTP Range requests, so its bytes never pass through the page. Lamina PDF reads the file range by range, and a render fetches the bytes it needs, so page 1 of a large file appears after a small part of it has arrived. Requests send no cookies or referrer and skip the HTTP cache. A server that ignores `Range` sends the whole file, which the viewer accepts up to 256 MiB. The new `progress` event reports how much has been read.
