---
id: edit-metadata-incremental
title: Edit metadata and save incrementally
checker: edit-metadata-incremental.mjs
inputs: [fixtures/golden/examples/hello.pdf => draft.pdf]
output: final.pdf
---
Open `draft.pdf`, set its document title to "Final report" and its author to "Ada Lovelace", and save the result to `final.pdf` as an incremental update, so that the bytes of `draft.pdf` stay unchanged at the start of `final.pdf`. Leave the pages as they are.
