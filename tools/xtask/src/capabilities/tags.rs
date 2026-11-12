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
//! (`--test <name>`).
//!
//! A TypeScript test claims evidence at the start of its title, one or more tags before the
//! words: `it("[cap J.01 web round-trip] [cap J.03 web round-trip] edits the title, …", …)`.
//! Only the suites in [`TS_SUITES`] may hold them, each for its own surfaces: suites a
//! required CI job runs, so a failing tagged test blocks the merge. `cargo xtask capabilities`
//! doesn't run them (they need a built app and a browser); it counts them as passing. The call
//! must be a plain `it(` or `test(`: `.skip`, `.only`, `.todo`, `.skipIf`, `.runIf`,
//! `.concurrent` and `.each` are refused, and so is any tag in a file with an unconditional
//! `describe.skip`, `describe.only` or `describe.todo`, or a focused `it.only`/`test.only`.
//! (A tag inside a `describe.skipIf`/`describe.runIf` block isn't detected; keep tags out of
//! them.) The title's string must open and close on the tag's line.

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

/// What runs a tagged test.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Runner {
    /// A cargo test target, which `cargo xtask capabilities` runs: the package (the directory
    /// name under `crates/`) and the target.
    Cargo { krate: String, target: Target },
    /// A TypeScript suite of [`TS_SUITES`], run by the CI job its `runner` names, not by
    /// `cargo xtask capabilities`.
    Suite(&'static TsSuite),
}

/// A tagged test.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TestRef {
    pub(crate) runner: Runner,
    /// A Rust test's function name (its full name has it as one `::` segment), or a TypeScript
    /// test's title without its tags.
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
                runner: Runner::Cargo { krate, target },
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

/// A TypeScript suite whose tests may carry tags: its folder (files directly in it, ending in
/// `suffix`), the surfaces its tags may claim, and the CI job that runs it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TsSuite {
    pub(crate) dir: &'static str,
    pub(crate) suffix: &'static str,
    pub(crate) surfaces: &'static [Surface],
    pub(crate) runner: &'static str,
}

/// Where TypeScript tags may go. Each suite runs in a job the merge waits on, so its tests
/// pass on every merged commit; add one only with that job.
pub(crate) const TS_SUITES: &[TsSuite] = &[TsSuite {
    dir: "apps/web/e2e",
    suffix: ".e2e.test.ts",
    surfaces: &[Surface::Web],
    runner: "the reader's e2e suite (`cargo xtask pdf-runtimes <browser>`, the fast lane's `pdf-runtimes` jobs)",
}];

/// The prefix of a tag in a TypeScript test title.
const TS_MARKER: &str = "[cap ";

/// Calls that skip, focus or run a whole block conditionally: a tagged test in a file with one
/// would not be proven on every run.
const TS_BLOCKERS: &[&str] = &[
    "describe.skip(",
    "describe.only(",
    "describe.todo(",
    "it.only(",
    "test.only(",
];

/// The suite `path` belongs to.
fn ts_suite(path: &Path) -> Option<&'static TsSuite> {
    let file = path.to_string_lossy().replace('\\', "/");
    let (dir, name) = file.rsplit_once('/')?;
    TS_SUITES.iter().find(|s| s.dir == dir && name.ends_with(s.suffix))
}

/// The places TS tags may go, for messages.
fn ts_places() -> String {
    TS_SUITES
        .iter()
        .map(|s| {
            let surfaces: Vec<&str> = s.surfaces.iter().map(|x| x.name()).collect();
            format!("{}/*{} ({})", s.dir, s.suffix, surfaces.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Every tag in one TypeScript test file. Fails on a tag outside [`TS_SUITES`], for a surface
/// its suite doesn't claim, on a call other than plain `it(`/`test(`, in a file that skips or
/// focuses (`TS_BLOCKERS`), or that isn't at the start of a one-line title.
pub(crate) fn scan_ts(path: &Path, text: &str) -> Result<Vec<Tag>> {
    let lines: Vec<&str> = text.lines().collect();
    let shown = path.display();
    let mut tags = Vec::new();
    let mut previous = "";
    for (index, line) in lines.iter().enumerate() {
        let at = format!("{shown}:{}", index + 1);
        let Some(start) = line.find(TS_MARKER) else {
            if !line.trim().is_empty() {
                previous = line;
            }
            continue;
        };
        let Some(suite) = ts_suite(path) else {
            bail!("{at}: TypeScript capability tags go only in {}", ts_places());
        };
        if let Some(blocker) = TS_BLOCKERS.iter().find(|b| text.contains(**b)) {
            bail!("{at}: the file has `{blocker}…)`, so its tagged tests don't run every time; remove it");
        }
        let before = line.get(..start).unwrap_or_default();
        let Some(quote) = before.chars().last().filter(|c| matches!(c, '"' | '\'' | '`')) else {
            bail!("{at}: a capability tag starts the test's title: it(\"[cap <ID> <surface> <status>] …\", …)");
        };
        let call = before
            .get(..before.len() - quote.len_utf8())
            .unwrap_or_default()
            .trim_end();
        let call = if call.is_empty() { previous.trim_end() } else { call };
        let callee = call
            .strip_suffix('(')
            .map(|c| {
                let begin = c
                    .rfind(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '$'))
                    .map_or(0, |i| i + 1);
                c.get(begin..).unwrap_or_default()
            })
            .unwrap_or_default();
        if callee != "it" && callee != "test" {
            bail!(
                "{at}: capability tags go on plain it(…) or test(…) calls, which run every time; found `{}`",
                if callee.is_empty() { call } else { callee }
            );
        }
        let after = line.get(start..).unwrap_or_default();
        let Some(end) = after.find(quote) else {
            bail!("{at}: keep the tagged title on one line, so the tag and the test it names stay together");
        };
        if after
            .get(end + quote.len_utf8()..)
            .unwrap_or_default()
            .trim_start()
            .starts_with('+')
        {
            bail!("{at}: keep the tagged title one string on one line, so the tag and the test it names stay together");
        }
        let mut title = after.get(..end).unwrap_or_default();
        let mut claims = Vec::new();
        while let Some(rest) = title.strip_prefix(TS_MARKER) {
            let Some((claim, tail)) = rest.split_once(']') else {
                bail!("{at}: a tag is `[cap <ID> <surface> <status>]`");
            };
            claims.push(parse_claim(claim).map_err(|e| anyhow::anyhow!("{at}: {e}"))?);
            title = tail.trim_start();
        }
        if title.contains(TS_MARKER) {
            bail!("{at}: put every tag at the start of the title, before the words");
        }
        for (id, surface, status) in claims {
            if !suite.surfaces.contains(&surface) {
                bail!(
                    "{at}: tags in {}/ claim only {}; found `{surface}`",
                    suite.dir,
                    ts_places()
                );
            }
            tags.push(Tag {
                id,
                surface,
                status,
                test: TestRef {
                    runner: Runner::Suite(suite),
                    function: title.to_owned(),
                    file: path.to_string_lossy().replace('\\', "/"),
                },
                line: index + 1,
            });
        }
        previous = line;
    }
    Ok(tags)
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
/// names as filters. TypeScript tags run in their own suites, not here.
pub(crate) fn runs(tags: &[Tag]) -> BTreeMap<(String, Target), Vec<String>> {
    let mut runs: BTreeMap<(String, Target), Vec<String>> = BTreeMap::new();
    for tag in tags {
        let Runner::Cargo { krate, target } = &tag.test.runner else {
            continue;
        };
        let names = runs.entry((krate.clone(), target.clone())).or_default();
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

    /// The cargo package and target of a Rust tag.
    fn cargo(tag: &Tag) -> (&str, &Target) {
        match &tag.test.runner {
            Runner::Cargo { krate, target } => (krate.as_str(), target),
            Runner::Suite(_) => panic!("a Rust tag runs under cargo"),
        }
    }

    fn target(path: &str) -> Option<Target> {
        scan(path, SOURCE).ok().map(|tags| cargo(&tags[0]).1.clone())
    }

    #[test]
    fn scan_rust_should_find_every_tag_and_its_function() {
        let tags = scan("crates/laminapdf-model/tests/views.rs", SOURCE).unwrap();
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
        let tags = scan("crates/laminapdf-model/tests/views.rs", SOURCE).unwrap();
        assert_eq!(cargo(&tags[0]), ("laminapdf-model", &Target::Test("views".to_owned())));
    }

    #[test]
    fn scan_rust_should_map_a_source_file_to_the_lib_target() {
        let tags = scan("crates/laminapdf-ops/src/op.rs", SOURCE).unwrap();
        assert_eq!(cargo(&tags[0]).1, &Target::Lib);
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

    const E2E: &str = "apps/web/e2e/metadata.e2e.test.ts";

    fn scan_e2e(text: &str) -> Result<Vec<Tag>> {
        scan_ts(Path::new(E2E), text)
    }

    #[test]
    fn scan_ts_should_read_one_tag_and_the_title_after_it() {
        let tags = scan_e2e("  it(\"[cap J.01 web round-trip] edits the title\", async () => {\n").unwrap();
        let found: Vec<(&str, Surface, Status, &str, usize)> = tags
            .iter()
            .map(|t| (t.id.as_str(), t.surface, t.status, t.test.function.as_str(), t.line))
            .collect();
        assert_eq!(found, [("J.01", Surface::Web, Status::RoundTrip, "edits the title", 1)]);
    }

    #[test]
    fn scan_ts_should_read_every_tag_of_a_title_on_the_line_after_its_call() {
        let text = "describe(\"metadata\", () => {\n  it(\n    '[cap J.01 web round-trip] [cap J.10 web write] removes the creator',\n    async () => {},\n  );\n});\n";
        let tags = scan_e2e(text).unwrap();
        let found: Vec<(&str, Status, &str, usize)> = tags
            .iter()
            .map(|t| (t.id.as_str(), t.status, t.test.function.as_str(), t.line))
            .collect();
        assert_eq!(
            found,
            [
                ("J.01", Status::RoundTrip, "removes the creator", 3),
                ("J.10", Status::Write, "removes the creator", 3)
            ]
        );
    }

    #[test]
    fn scan_ts_should_name_the_suite_that_runs_the_test() {
        let tags = scan_e2e("test(`[cap J.02 web read] shows the dates`, () => {});").unwrap();
        assert!(matches!(tags[0].test.runner, Runner::Suite(s) if s.dir == "apps/web/e2e"));
    }

    #[test]
    fn scan_ts_should_refuse_skipped_focused_and_conditional_calls() {
        for call in [
            "it.skip(",
            "it.only(",
            "it.todo(",
            "test.skip(",
            "it.concurrent(",
            "it.runIf(ok)(",
            "it.skipIf(no)(",
        ] {
            let text = format!("{call}\"[cap J.01 web round-trip] t\", () => {{}});");
            assert!(scan_e2e(&text).is_err(), "{call} was accepted");
        }
    }

    #[test]
    fn scan_ts_should_refuse_a_tag_in_a_file_that_skips_or_focuses_a_block() {
        let text = "describe.skip(\"later\", () => {});\nit(\"[cap J.01 web round-trip] t\", () => {});\n";
        assert!(scan_e2e(text).is_err());
    }

    #[test]
    fn scan_ts_should_refuse_a_tag_outside_the_suites_that_ci_runs() {
        for path in [
            "packages/sdk/test/a.test.ts",
            "apps/web/src/document.test.ts",
            "apps/web/e2e/smoke.test.ts",
        ] {
            let text = "it(\"[cap J.01 web round-trip] t\", () => {});";
            assert!(scan_ts(Path::new(path), text).is_err(), "{path} was accepted");
        }
    }

    #[test]
    fn scan_ts_should_refuse_a_surface_its_suite_does_not_claim() {
        assert!(scan_e2e("it(\"[cap J.01 pdf round-trip] t\", () => {});").is_err());
    }

    #[test]
    fn scan_ts_should_refuse_a_malformed_tag() {
        assert!(scan_e2e("it(\"[cap J.01 web] t\", () => {});").is_err());
        assert!(scan_e2e("it(\"[cap J.01 web edit] t\", () => {});").is_err());
    }

    #[test]
    fn scan_ts_should_refuse_a_tag_after_the_words() {
        assert!(scan_e2e("it(\"edits [cap J.01 web round-trip]\", () => {});").is_err());
    }

    #[test]
    fn scan_ts_should_refuse_a_title_that_continues_on_the_next_line() {
        assert!(scan_e2e("it(\"[cap J.01 web round-trip] edits \" +\n  \"the title\", () => {});").is_err());
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
        let tags = scan("crates/laminapdf-model/tests/views.rs", SOURCE).unwrap();
        let runs = runs(&tags);
        assert_eq!(
            runs.values().cloned().collect::<Vec<_>>(),
            [vec!["rotate_is_read".to_owned(), "round_trips".to_owned()]]
        );
    }
}
