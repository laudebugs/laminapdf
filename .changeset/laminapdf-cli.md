---
"@laminapdf/sdk": minor
---

New: the `laminapdf` command-line tool (`crates/laminapdf-cli`, not published to crates.io yet), with `info`, `check`, `pages rotate`, `extract` and `explain`, versioned JSON reports with JSON Schemas, and exit codes 0–5 (ADR 0031). Its own error codes (`invalid-command-line`, `invalid-page-range`, `binary-output-to-terminal`, `unknown-code`, `overwrite-refused`, `input-unreadable`, `output-unwritable`) join every code table, so the TypeScript `ErrorCode` union lists them too. The Rust façade gains `Document::version`, `is_encrypted`, `is_tagged` and `read_every_object`, and `laminapdf::did_you_mean`.
