---
"@laminapdf/sdk": minor
---

Two new warning codes, `extract-table-ambiguous` and `extract-table-limit`, join the generated `WarningCode` union. In Rust, `laminapdf-extract` is now published and the `laminapdf` crate exposes it: `Document::extract`, `extract_job` and `extract_json` return blocks in reading order with tables and their cells, and an `Extraction` renders as Markdown, plain text, retrieval chunks or `laminapdf.extract/1` JSON, with a JSON Schema in `laminapdf-extract`'s `schemas/`.
