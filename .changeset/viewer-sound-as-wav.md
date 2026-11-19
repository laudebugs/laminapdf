---
"@laminapdf/viewer": minor
---

`client.soundAsWav(id)` returns a sound comment's recording as a WAV file, read in the document's worker, so a page can play it in an `<audio>` element. Unsigned, signed, μ-law and A-law samples (every encoding PDF allows) all come out as WAV, which every browser plays. For a sound comment whose samples can't be read, the promise rejects with `sound-unavailable`.
