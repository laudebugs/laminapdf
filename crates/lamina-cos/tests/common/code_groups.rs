//! Which top-level keys of `fixtures/codes.json` are groups of codes, and which of them list
//! warnings or exist only in TypeScript. One definition, included with `#[path]` by
//! `tests/common/codes.rs` (the crates' code tests) and `tools/xtask/src/codegen.rs` (`cargo
//! xtask gen codes`, which generates every surface's tables from `codes.json`).

/// Top-level keys of `codes.json` that are not groups of codes.
pub const NOT_CODE_GROUPS: [&str; 3] = ["version", "docsUrl", "limits"];

/// Groups whose codes only `@laminapdf/sdk` raises: left out of the Rust `ErrorCode` enum (and
/// so of the WASM).
pub const TS_ONLY_GROUPS: [&str; 1] = ["packageErrors"];

/// Whether `group` lists warnings (`warnings`, or a key ending in `Warnings`) rather than errors.
pub fn is_warning_group(group: &str) -> bool {
    group == "warnings" || group.ends_with("Warnings")
}
