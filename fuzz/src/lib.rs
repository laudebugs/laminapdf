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
/// The save targets' entry points (`ops_json`, `save_roundtrip`), shared with laminapdf-save's
/// `tests/regressions.rs`.
#[path = "../../crates/laminapdf-save/tests/common/fuzz_entry.rs"]
pub mod save_entry;
/// The `extract_text` target's entry point, shared with laminapdf-text's `tests/regressions.rs`.
#[path = "../../crates/laminapdf-text/tests/common/fuzz_entry.rs"]
pub mod text_entry;
