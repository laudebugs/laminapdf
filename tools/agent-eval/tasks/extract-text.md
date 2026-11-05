---
id: extract-text
title: Extract the text of page 1
checker: extract-text.mjs
inputs: [fixtures/corpus/text-example.pdf => sample.pdf]
output: page1.txt
---
Extract the text of the first page of `sample.pdf` in reading order and write it to `page1.txt` as UTF-8, one line of the page per line of the file.
