//! Shared code for lamina's fuzz targets. The entry points and `exercise` are the very files
//! lamina-model's integration tests use (`crates/lamina-model/tests/common/`), and
//! lamina-content's for the `content` target (`crates/lamina-content/tests/common/`),
//! included here so a target and its regression replay (`tests/regressions.rs`) run the same
//! code.

#[path = "../../crates/lamina-model/tests/common/exercise.rs"]
pub mod exercise;
#[path = "../../crates/lamina-model/tests/common/fuzz_entry.rs"]
pub mod fuzz_entry;
#[path = "../../crates/lamina-content/tests/common/fuzz_entry.rs"]
pub mod content_entry;
/// The save targets' entry points (`ops_json`, `save_roundtrip`), shared with lamina-save's
/// `tests/regressions.rs`.
#[path = "../../crates/lamina-save/tests/common/fuzz_entry.rs"]
pub mod save_entry;
