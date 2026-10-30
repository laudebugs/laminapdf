//! Which top-level keys of `fixtures/codes.json` are groups of codes, and which of them list
//! warnings or exist only in TypeScript. One definition, included with `#[path]` by
//! `tests/common/codes.rs` (the crates' code tests), `crates/lamina/build.rs` (the hint and
//! summary tables) and `tools/xtask/src/codegen.rs` (`@lamina/pdf`'s code unions).

/// Top-level keys of `codes.json` that are not groups of codes.
pub const NOT_CODE_GROUPS: [&str; 3] = ["version", "docsUrl", "limits"];

/// Groups whose codes only `@lamina/pdf` raises: left out of the WASM hint tables.
pub const TS_ONLY_GROUPS: [&str; 1] = ["packageErrors"];

/// Whether `group` lists warnings (`warnings`, or a key ending in `Warnings`) rather than errors.
pub fn is_warning_group(group: &str) -> bool {
    group == "warnings" || group.ends_with("Warnings")
}
