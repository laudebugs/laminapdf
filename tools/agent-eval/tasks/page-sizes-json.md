---
id: page-sizes-json
title: Page sizes in points as JSON
checker: page-sizes-json.mjs
inputs: [fixtures/golden/vectors.pdf => shapes.pdf]
output: sizes.json
---
Write `sizes.json` with the size of every page of `shapes.pdf`, as a viewer displays it (after any cropping and rotation), in points. Use this shape, with pages numbered from 1 and sizes rounded to two decimals:

```json
[{ "page": 1, "width": 595.28, "height": 841.89 }]
```
