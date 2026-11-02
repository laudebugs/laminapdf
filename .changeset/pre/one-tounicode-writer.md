---
"@laminapdf/sdk": patch
---

Text copied from a standard 14 font no longer loses half of its last character when that text runs past 256 UTF-16 units and ends with a character outside the BMP, such as an emoji. Symbol and ZapfDingbats now write a `/CapHeight`, as every other font does, and `laminapdf-syntax` adds `Object::number` and `Object::rounded`.
