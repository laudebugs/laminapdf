---
"@laminapdf/sdk": patch
---

A standard 14 font's ToUnicode map no longer cuts a character in half when its text is longer than 256 UTF-16 units and ends outside the Basic Multilingual Plane, such as an emoji. The Symbol and ZapfDingbats fonts' descriptors now have a `/CapHeight`, so PDFs that use them change by a few bytes.
