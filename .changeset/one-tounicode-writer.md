---
"@laminapdf/sdk": patch
---

Text copied from a standard 14 font no longer loses half of its last character when that text runs past 256 UTF-16 units and ends with a character outside the BMP, such as an emoji. The Symbol and ZapfDingbats fonts now write a `/CapHeight` (their ascent, the rule every other font already follows), and in Rust, `Object::number` and `Object::rounded` in `laminapdf-syntax` turn a number into an integer or a real object.
