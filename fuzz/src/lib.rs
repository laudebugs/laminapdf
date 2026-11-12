//! Shared code for lamina's fuzz targets. The entry points and `exercise` are the very files
//! laminapdf-model's integration tests use (`crates/laminapdf-model/tests/common/`), and
//! laminapdf-content's for the `content` target (`crates/laminapdf-content/tests/common/`),
//! included here so a target and its regression replay (`tests/regressions.rs`) run the same
//! code.

#[path = "../../crates/laminapdf-model/tests/common/exercise.rs"]
pub mod exercise;
#[path = "../../crates/laminapdf-model/tests/common/fuzz_entry.rs"]
pub mod fuzz_entry;
#[path = "../../crates/laminapdf-content/tests/common/fuzz_entry.rs"]
pub mod content_entry;
/// The `content_append_scan` target's entry point, shared with laminapdf-content's `tests/append.rs`.
#[path = "../../crates/laminapdf-content/tests/common/append_fuzz_entry.rs"]
pub mod append_entry;
/// The save targets' entry points (`ops_json`, `save_roundtrip`), shared with laminapdf-save's
/// `tests/regressions.rs`.
#[path = "../../crates/laminapdf-save/tests/common/fuzz_entry.rs"]
pub mod save_entry;
/// The `extract_text` target's entry point, shared with laminapdf-text's `tests/regressions.rs`.
#[path = "../../crates/laminapdf-text/tests/common/fuzz_entry.rs"]
pub mod text_entry;
/// The `extract` target's entry point, shared with laminapdf-extract's `tests/regressions.rs`.
#[path = "../../crates/laminapdf-extract/tests/common/fuzz_entry.rs"]
pub mod extract_entry;
/// The `markdown` target's entry point, shared with laminapdf-build's `tests/regressions.rs`.
#[path = "../../crates/laminapdf-build/tests/common/fuzz_entry.rs"]
pub mod markdown_entry;
/// The `document` target's entry point, shared with laminapdf-build's `tests/document_regressions.rs`.
#[path = "../../crates/laminapdf-build/tests/common/document_fuzz_entry.rs"]
pub mod document_entry;
/// The `engine_protocol_decode` target's entry point, shared with laminapdf-desktop-protocol's
/// `tests/regressions.rs`.
#[path = "../../crates/laminapdf-desktop-protocol/tests/common/fuzz_entry.rs"]
pub mod protocol_entry;
