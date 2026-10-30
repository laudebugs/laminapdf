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
//! One test may carry several tags. Only `#[test]` functions in a crate's `src/` (run with
//! `--lib`) or in `tests/<name>.rs` / `tests/<name>/main.rs` (run with `--test <name>`) can
//! be tagged. TypeScript tests will be tagged in their name, `test("[cap C.04 pdf round-trip] …")`,
//! once a package exists to run them.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path};

use anyhow::{Result, bail};

use super::manifest::{Status, Surface};

/// The prefix of a tag line, after `///`.
const MARKER: &str = "capability:";

/// The cargo test target a tagged test is in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Target {
    Lib,
    Test(String),
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lib => f.write_str("--lib"),
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

/// The cargo package and target of a tracked Rust file, if it can hold tagged tests.
fn cargo_target(path: &Path) -> Option<(String, Target)> {
    let parts: Vec<&str> = path
        .components()
        .map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        ["crates", krate, "src", ..] => Some(((*krate).to_owned(), Target::Lib)),
        ["crates", krate, "tests", file] => {
            let name = file.strip_suffix(".rs")?;
            Some(((*krate).to_owned(), Target::Test(name.to_owned())))
        }
        ["crates", krate, "tests", dir, "main.rs"] => Some(((*krate).to_owned(), Target::Test((*dir).to_owned()))),
        _ => None,
    }
}

/// Every tag in one Rust source file. Fails on a malformed tag, a tag that is not on a
/// `#[test]` function, or a tag in a file that is not a test target.
pub(crate) fn scan_rust(path: &Path, text: &str) -> Result<Vec<Tag>> {
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
        let Some((krate, target)) = cargo_target(path) else {
            bail!("{at}: capability tags go on tests in crates/<crate>/src/ or crates/<crate>/tests/<name>.rs");
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
/// (`true` when it passed). Interleaved stdout from tests can't forge a passing line for a test
/// that failed: libtest prints exactly one result line per test, and a failure wins.
pub(crate) fn parse_libtest(stdout: &str) -> BTreeMap<String, bool> {
    let mut results: BTreeMap<String, bool> = BTreeMap::new();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        let Some((name, outcome)) = rest.split_once(" ... ") else {
            continue;
        };
        let passed = outcome.trim() == "ok";
        results
            .entry(name.trim().to_owned())
            .and_modify(|p| *p &= passed)
            .or_insert(passed);
    }
    results
}

/// Whether the test a tag names passed: at least one test of that function ran, and all of
/// them passed (several match when the same name is in two modules, or for `rstest` cases).
pub(crate) fn passed(results: &BTreeMap<String, bool>, function: &str) -> Option<bool> {
    let mut matched = results
        .iter()
        .filter(|(name, _)| name.split("::").any(|segment| segment == function))
        .peekable();
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
        scan_rust(Path::new(path), text)
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
