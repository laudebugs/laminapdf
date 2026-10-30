//! Capability tags on tests, and the results of running them.
//!
//! A Rust test claims evidence with a doc-comment line directly above it (among its other
//! doc comments and attributes):
//!
//! ```text
//! /// capability: C.08 core round-trip
//! #[test]
//! fn every_case_saves_or_refuses_as_expected_and_reopens_in_lamina() { … }
//! ```
//!
//! One test may carry several tags. Only `#[test]` functions in a crate's test targets can be
//! tagged: `src/main.rs` and `src/bin/<name>.rs` or `src/bin/<name>/…` (`--bin`), the rest of
//! `src/` (`--lib`), and `tests/<name>.rs` or `tests/<name>/…` when `tests/<name>/main.rs` exists
//! (`--test <name>`). TypeScript tests will be tagged in their name, `test("[cap C.04 pdf round-trip] …")`,
//! once a package exists to run them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};

use super::manifest::{Status, Surface};

/// The prefix of a tag line, after `///`.
const MARKER: &str = "capability:";

/// The cargo test target a tagged test is in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Target {
    Lib,
    Bin(String),
    Test(String),
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lib => f.write_str("--lib"),
            Self::Bin(name) => write!(f, "--bin {name}"),
            Self::Test(name) => write!(f, "--test {name}"),
        }
    }
}

/// A tagged test function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TestRef {
    /// The package (the directory name under `crates/`).
    pub(crate) krate: String,
    pub(crate) target: Target,
    /// The function name; the test's full name has it as one `::` segment.
    pub(crate) function: String,
    /// The source file, relative to the repository root, `/`-separated.
    pub(crate) file: String,
}

/// One `capability:` claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tag {
    pub(crate) id: String,
    pub(crate) surface: Surface,
    pub(crate) status: Status,
    pub(crate) test: TestRef,
    /// 1-based line of the tag, for messages.
    pub(crate) line: usize,
}

/// The cargo package and target of a tracked Rust file, if it can hold tagged tests. `files`
/// are every tracked Rust file, to tell a `tests/<dir>/main.rs` target from a shared helper
/// module such as `tests/common/mod.rs`. (A package with both `src/lib.rs` and `src/main.rs`
/// gets its other `src/` files counted as the library's.)
fn cargo_target(path: &Path, files: &BTreeSet<PathBuf>) -> Option<(String, Target)> {
    let parts: Vec<&str> = path
        .components()
        .map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let target = match parts.as_slice() {
        ["crates", krate, "src", "main.rs"] => Target::Bin((*krate).to_owned()),
        ["crates", _, "src", "bin", file] => Target::Bin(file.strip_suffix(".rs")?.to_owned()),
        ["crates", _, "src", "bin", dir, _, ..] => Target::Bin((*dir).to_owned()),
        ["crates", _, "src", ..] => Target::Lib,
        ["crates", _, "tests", file] => Target::Test(file.strip_suffix(".rs")?.to_owned()),
        ["crates", krate, "tests", dir, _, ..]
            if files.contains(&Path::new("crates").join(krate).join("tests").join(dir).join("main.rs")) =>
        {
            Target::Test((*dir).to_owned())
        }
        _ => return None,
    };
    Some(((*parts.get(1)?).to_owned(), target))
}

/// Every tag in one Rust source file (`files`: every tracked Rust file, see [`cargo_target`]).
/// Fails on a malformed tag, a tag that is not on a `#[test]` function, or a tag in a file
/// that is not part of a test target.
pub(crate) fn scan_rust(path: &Path, text: &str, files: &BTreeSet<PathBuf>) -> Result<Vec<Tag>> {
    let lines: Vec<&str> = text.lines().collect();
    let shown = path.display();
    let mut tags = Vec::new();
    let mut i = 0;
    while let Some(line) = lines.get(i) {
        i += 1;
        let Some(claim) = line.trim().strip_prefix("///").map(str::trim) else {
            continue;
        };
        let Some(claim) = claim.strip_prefix(MARKER) else {
            continue;
        };
        let at = format!("{shown}:{i}");
        let (id, surface, status) = parse_claim(claim).map_err(|e| anyhow::anyhow!("{at}: {e}"))?;
        let Some((krate, target)) = cargo_target(path, files) else {
            bail!(
                "{at}: capability tags go on tests in crates/<crate>/src/, tests/<name>.rs or tests/<name>/ \
                 (with a main.rs)"
            );
        };
        let Some(function) = tagged_function(lines.get(i..).unwrap_or_default()) else {
            bail!("{at}: a capability tag must be followed by a #[test] fn (only doc comments and attributes between)");
        };
        tags.push(Tag {
            id,
            surface,
            status,
            test: TestRef {
                krate,
                target,
                function,
                file: path.to_string_lossy().replace('\\', "/"),
            },
            line: i,
        });
    }
    Ok(tags)
}

/// `C.08 core round-trip` → its three parts.
fn parse_claim(claim: &str) -> Result<(String, Surface, Status)> {
    let words: Vec<&str> = claim.split_whitespace().collect();
    let [id, surface, status] = words.as_slice() else {
        bail!("a capability tag is `/// capability: <ID> <surface> <status>`, e.g. `C.08 core round-trip`");
    };
    if !super::manifest::is_id(id) {
        bail!("`{id}` is not an operation ID (`<group>.<nn>`)");
    }
    let Some(surface) = Surface::parse(surface) else {
        bail!("unknown surface `{surface}` (core, pdf, web, desktop)");
    };
    let Some(status) = Status::parse(status) else {
        bail!("unknown status `{status}` (unsupported, read, write, round-trip)");
    };
    Ok(((*id).to_owned(), surface, status))
}

/// The function a tag belongs to: skip doc comments and attributes (an attribute may span
/// lines), require a `#[test]` among them, and return the name of the `fn` that follows.
fn tagged_function(lines: &[&str]) -> Option<String> {
    let mut is_test = false;
    let mut depth = 0i32;
    for line in lines {
        let line = line.trim();
        if depth > 0 {
            depth += bracket_balance(line);
            continue;
        }
        if line.starts_with("///") {
            continue;
        }
        if line.starts_with("#[") {
            is_test |= line == "#[test]";
            depth = bracket_balance(line);
            continue;
        }
        let rest = line.strip_prefix("pub ").unwrap_or(line);
        let rest = rest.strip_prefix("async ").unwrap_or(rest);
        let name = rest.strip_prefix("fn ")?;
        let end = name.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
        return is_test.then(|| name.get(..end).unwrap_or_default().to_owned());
    }
    None
}

fn bracket_balance(line: &str) -> i32 {
    line.chars()
        .map(|c| match c {
            '[' => 1,
            ']' => -1,
            _ => 0,
        })
        .sum()
}

/// TypeScript test files that carry `[cap …]` tags. There is no TS package yet, so any tag is
/// an error rather than evidence that is silently never collected.
pub(crate) fn scan_ts(path: &Path, text: &str) -> Result<()> {
    if let Some((index, _)) = text.lines().enumerate().find(|(_, l)| l.contains("[cap ")) {
        bail!(
            "{}:{}: TypeScript capability tags are not collected yet; teach `cargo xtask capabilities` \
             to run them (Vitest `-t`) in the PR that adds the first tagged TS test",
            path.display(),
            index + 1
        );
    }
    Ok(())
}

/// Whether `path` is a TS test file that could carry tags.
pub(crate) fn is_ts_test(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let top = path.components().next().and_then(|c| c.as_os_str().to_str());
    matches!(top, Some("packages" | "apps"))
        && [".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx"]
            .iter()
            .any(|s| name.ends_with(s))
}

/// The results in libtest's default output: `test <name> ... ok|FAILED|ignored`, by full name
/// (`true` when it passed). A name is one word: libtest appends ` - should panic` (and the
/// like) after it. Interleaved stdout from tests can't forge a passing line for a test that
/// failed: libtest prints one result line per test, and a failure wins.
pub(crate) fn parse_libtest(stdout: &str) -> BTreeMap<String, bool> {
    let mut results: BTreeMap<String, bool> = BTreeMap::new();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        let Some((name, outcome)) = rest.split_once(" ... ") else {
            continue;
        };
        let Some(name) = name.split_whitespace().next() else {
            continue;
        };
        let passed = outcome.trim() == "ok";
        results
            .entry(name.to_owned())
            .and_modify(|p| *p &= passed)
            .or_insert(passed);
    }
    results
}

/// Whether the test a tag names passed: at least one test of that function ran, and all of
/// them passed. A test is the function's when its name's last segment is the function, or,
/// for `rstest` cases (`function::case_1_…`), the one before a `case_` segment. Several match
/// when the same function name is in two modules.
pub(crate) fn passed(results: &BTreeMap<String, bool>, function: &str) -> Option<bool> {
    let is_function = |name: &str| {
        let mut segments = name.rsplit("::");
        match (segments.next(), segments.next()) {
            (Some(last), _) if last == function => true,
            (Some(last), Some(parent)) => parent == function && last.starts_with("case_"),
            _ => false,
        }
    };
    let mut matched = results.iter().filter(|(name, _)| is_function(name)).peekable();
    matched.peek()?;
    Some(matched.all(|(_, ok)| *ok))
}

/// The command-line groups to run: one `cargo test` per package and target, with the function
/// names as filters.
pub(crate) fn runs(tags: &[Tag]) -> BTreeMap<(String, Target), Vec<String>> {
    let mut runs: BTreeMap<(String, Target), Vec<String>> = BTreeMap::new();
    for tag in tags {
        let names = runs
            .entry((tag.test.krate.clone(), tag.test.target.clone()))
            .or_default();
        if !names.contains(&tag.test.function) {
            names.push(tag.test.function.clone());
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "
mod pages {
    /// Rotation is read as stored.
    /// capability: C.08 core read
    #[test]
    #[expect(
        clippy::unwrap_used,
        reason = \"x\"
    )]
    fn rotate_is_read() {}

    /// capability: C.08 core round-trip
    /// capability: J.01 core round-trip
    #[test]
    fn round_trips() {}
}
";

    fn scan(path: &str, text: &str) -> Result<Vec<Tag>> {
        let files = BTreeSet::from([PathBuf::from("crates/a/tests/suite/main.rs")]);
        scan_rust(Path::new(path), text, &files)
    }

    fn target(path: &str) -> Option<Target> {
        scan(path, SOURCE).ok().map(|tags| tags[0].test.target.clone())
    }

    #[test]
    fn scan_rust_should_find_every_tag_and_its_function() {
        let tags = scan("crates/lamina-model/tests/views.rs", SOURCE).unwrap();
        let found: Vec<(&str, &str, Status)> = tags
            .iter()
            .map(|t| (t.id.as_str(), t.test.function.as_str(), t.status))
            .collect();
        assert_eq!(
            found,
            [
                ("C.08", "rotate_is_read", Status::Read),
                ("C.08", "round_trips", Status::RoundTrip),
                ("J.01", "round_trips", Status::RoundTrip)
            ]
        );
    }

    #[test]
    fn scan_rust_should_map_an_integration_test_file_to_its_target() {
        let tags = scan("crates/lamina-model/tests/views.rs", SOURCE).unwrap();
        assert_eq!(
            (tags[0].test.krate.as_str(), &tags[0].test.target),
            ("lamina-model", &Target::Test("views".to_owned()))
        );
    }

    #[test]
    fn scan_rust_should_map_a_source_file_to_the_lib_target() {
        let tags = scan("crates/lamina-ops/src/op.rs", SOURCE).unwrap();
        assert_eq!(tags[0].test.target, Target::Lib);
    }

    #[test]
    fn scan_rust_should_map_binaries_to_their_bin_target() {
        assert_eq!(
            [
                target("crates/a/src/main.rs"),
                target("crates/a/src/bin/tool.rs"),
                target("crates/a/src/bin/tool/args.rs"),
            ],
            [
                Some(Target::Bin("a".to_owned())),
                Some(Target::Bin("tool".to_owned())),
                Some(Target::Bin("tool".to_owned())),
            ]
        );
    }

    #[test]
    fn scan_rust_should_map_a_submodule_of_a_test_directory_to_its_target() {
        assert_eq!(
            target("crates/a/tests/suite/cases.rs"),
            Some(Target::Test("suite".to_owned()))
        );
    }

    #[test]
    fn scan_rust_should_reject_a_tag_on_a_helper() {
        let text = "/// capability: C.08 core read\nfn helper() {}\n";
        assert!(scan("crates/a/tests/b.rs", text).is_err());
    }

    #[test]
    fn scan_rust_should_reject_a_tag_in_a_shared_helper_module() {
        let text = "/// capability: C.08 core read\n#[test]\nfn t() {}\n";
        assert!(scan("crates/a/tests/common/mod.rs", text).is_err());
    }

    #[test]
    fn scan_rust_should_reject_a_malformed_tag() {
        let text = "/// capability: C.08 core\n#[test]\nfn t() {}\n";
        assert!(scan("crates/a/tests/b.rs", text).is_err());
    }

    #[test]
    fn scan_rust_should_reject_an_unknown_status() {
        let text = "/// capability: C.08 core edit\n#[test]\nfn t() {}\n";
        assert!(scan("crates/a/tests/b.rs", text).is_err());
    }

    #[test]
    fn scan_ts_should_reject_any_tag_for_now() {
        let text = "test(\"[cap C.04 pdf round-trip] reorders\", () => {});";
        assert!(scan_ts(Path::new("packages/pdf/src/a.test.ts"), text).is_err());
    }

    const LIBTEST: &str = "
running 3 tests
test pages::rotate_is_read ... ok
test round_trips ... FAILED
test other ... ignored
noise test fake ... ok

test result: FAILED. 1 passed; 1 failed; 1 ignored
";

    #[test]
    fn parse_libtest_should_read_each_outcome() {
        let results = parse_libtest(LIBTEST);
        assert_eq!(
            [
                passed(&results, "rotate_is_read"),
                passed(&results, "round_trips"),
                passed(&results, "other"),
                passed(&results, "missing"),
            ],
            [Some(true), Some(false), Some(false), None]
        );
    }

    #[test]
    fn parse_libtest_should_drop_the_should_panic_suffix() {
        let results = parse_libtest("test m::f - should panic ... ok\n");
        assert_eq!(passed(&results, "f"), Some(true));
    }

    #[test]
    fn passed_should_not_match_a_module_of_the_same_name() {
        let results = parse_libtest("test round_trips::other_test ... ok\n");
        assert_eq!(passed(&results, "round_trips"), None);
    }

    #[test]
    fn passed_should_match_rstest_cases() {
        let results = parse_libtest("test m::f::case_1_one ... ok\ntest m::f::case_2_two ... FAILED\n");
        assert_eq!(passed(&results, "f"), Some(false));
    }

    #[test]
    fn passed_should_require_every_matching_test_to_pass() {
        let results = parse_libtest("test a::t ... ok\ntest b::t ... FAILED\n");
        assert_eq!(passed(&results, "t"), Some(false));
    }

    #[test]
    fn runs_should_group_by_package_and_target_without_duplicates() {
        let tags = scan("crates/lamina-model/tests/views.rs", SOURCE).unwrap();
        let runs = runs(&tags);
        assert_eq!(
            runs.values().cloned().collect::<Vec<_>>(),
            [vec!["rotate_is_read".to_owned(), "round_trips".to_owned()]]
        );
    }
}
