---
id: damaged-file-warnings
title: Repair a damaged file and list its warnings
checker: damaged-file-warnings.mjs
inputs: [fixtures/corpus/hello-broken.pdf => damaged.pdf]
output: repaired.pdf
---
`damaged.pdf` is damaged: its cross-reference data is wrong. Open it anyway, then:

1. write `warnings.json`, a JSON array of the codes of the warnings the library reported while reading the file (each code once, in the order first reported), such as `["some-code"]`;
2. save a repaired copy to `repaired.pdf` that opens without those warnings.
