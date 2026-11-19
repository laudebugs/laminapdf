---
"@laminapdf/viewer": minor
---

`client.soundAsWav(id)` returns a sound comment's recording as a WAV file, read in the document's worker, so a page can play it in an `<audio>` element. Raw, signed, μ-law and A-law samples all come out as WAV, which every browser plays. A sound comment whose samples can't be played fails with `sound-unavailable`.
