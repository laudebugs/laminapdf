//! `cargo xtask capabilities [--check]`: the test-backed capability matrix (full-editor §3, todo X2).
//!
//! The rows are `docs/capabilities/manifest.toml`, one per operation of the feature inventory
//! (`docs/research/pdf/06-feature-inventory.md` §3; the two must agree). Tests claim a status
//! for a row and surface with a tag ([`tags`]); this task builds every test target that holds
//! a tag with one `cargo test --no-run`, runs each tagged test, and rewrites
//! `docs/capabilities/README.md` from what they prove. `--check` writes nothing and fails when
//! the README differs (the full lane). Any tagged test that fails or isn't found fails both.
//!
//! The README is a function of the manifest and the tags alone once every tagged test passes,
//! so [`check_static`] (`check docs`, every PR) compares it exactly with what they generate,
//! without running anything: a docs-only edit to the manifest or the README can't leave it
//! stale. The full lane adds the proof that the tagged tests pass.

mod manifest;
mod matrix;
mod tags;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use self::manifest::Manifest;
use self::matrix::Matrix;
use self::tags::{Tag, Target};
use crate::step;
use crate::tracked::Tracked;

const MANIFEST: &str = "docs/capabilities/manifest.toml";
const README: &str = "docs/capabilities/README.md";
const INVENTORY: &str = "docs/research/pdf/06-feature-inventory.md";

/// The manifest and every tag, validated against each other and the inventory.
struct Inputs {
    manifest: Manifest,
    tags: Vec<Tag>,
}

/// Validates the manifest against the inventory and the tags against the manifest. `sources`
/// are `(repository-relative path, text)` of the files that may hold tags.
fn inputs<'a>(
    manifest: &str,
    inventory: &str,
    sources: impl IntoIterator<Item = (&'a Path, &'a str)>,
) -> Result<Inputs> {
    let manifest = Manifest::parse(manifest)?;
    let mut problems = manifest.inventory_problems(&manifest::parse_inventory(inventory)?);
    let sources: Vec<(&Path, &str)> = sources.into_iter().collect();
    let files: BTreeSet<PathBuf> = sources.iter().map(|(path, _)| path.to_path_buf()).collect();
    let mut tags = Vec::new();
    for (path, text) in sources {
        if tags::is_ts_test(path) {
            tags::scan_ts(path, text)?;
        } else {
            tags.extend(tags::scan_rust(path, text, &files)?);
        }
    }
    problems.extend(matrix::tag_problems(&manifest, &tags));
    fail_on(&problems, "the capability manifest or its tags are inconsistent")?;
    Ok(Inputs { manifest, tags })
}

/// Reads the inputs from the tracked files under `root`.
fn load(root: &Path) -> Result<Inputs> {
    let read = |path: &str| fs::read_to_string(root.join(path)).with_context(|| format!("reading {path}"));
    let tracked = Tracked::from_git(root)?;
    let mut sources = Vec::new();
    for path in tracked.files() {
        let rust = path.starts_with("crates") && path.extension().is_some_and(|x| x == "rs");
        if rust || tags::is_ts_test(path) {
            let text = fs::read_to_string(root.join(path)).with_context(|| format!("reading {}", path.display()))?;
            sources.push((path.to_path_buf(), text));
        }
    }
    inputs(
        &read(MANIFEST)?,
        &read(INVENTORY)?,
        sources.iter().map(|(p, t)| (p.as_path(), t.as_str())),
    )
}

/// The README the test results prove; fails when a tagged test failed or was not found.
/// `results` maps each (package, target) run to its libtest results.
fn evaluate(inputs: &Inputs, results: &BTreeMap<(String, Target), BTreeMap<String, bool>>) -> Result<String> {
    let mut problems = Vec::new();
    for tag in &inputs.tags {
        let test = &tag.test;
        let run = results.get(&(test.krate.clone(), test.target.clone()));
        let at = format!("{}:{} ({} {} {})", test.file, tag.line, tag.id, tag.surface, tag.status);
        match run.and_then(|r| tags::passed(r, &test.function)) {
            Some(true) => {}
            Some(false) => problems.push(format!("{at}: `{}` failed or is ignored", test.function)),
            None => problems.push(format!("{at}: `{}` did not run", test.function)),
        }
    }
    fail_on(
        &problems,
        "tagged tests did not pass, so their capability claims are not proven",
    )?;
    Ok(matrix::render(&inputs.manifest, &Matrix::from_tags(&inputs.tags)))
}

/// `cargo xtask capabilities [--check]`.
pub(crate) fn run(root: &Path, check: bool) -> Result<()> {
    let inputs = load(root)?;
    let runs = tags::runs(&inputs.tags);
    let binaries = build_tests(root, runs.keys())?;
    let mut results = BTreeMap::new();
    for ((krate, target), names) in runs {
        let Some(binary) = binaries.get(&(krate.clone(), target.clone())) else {
            continue; // reported by `evaluate` as tests that did not run
        };
        let output = run_tests(root, &krate, binary, &names)?;
        results.insert((krate, target), tags::parse_libtest(&output));
    }
    let readme = evaluate(&inputs, &results)?;
    let path = root.join(README);
    if check {
        let committed = fs::read_to_string(&path).with_context(|| format!("reading {README}"))?;
        if committed != readme {
            bail!("{README} is not what the tagged tests prove; run `cargo xtask capabilities` and commit it");
        }
        println!("capabilities: {README} matches {} passing tag(s)", inputs.tags.len());
    } else {
        fs::write(&path, readme).with_context(|| format!("writing {README}"))?;
        println!("capabilities: wrote {README} from {} passing tag(s)", inputs.tags.len());
    }
    Ok(())
}

/// What is wrong with `readme` without running a test: claims beyond what any tag could prove
/// (named one by one), else any difference from what the manifest and the tags generate
/// (names, rounds, targets, surfaces, evidence, summary).
fn static_problems(inputs: &Inputs, readme: &str) -> Vec<String> {
    let upper = Matrix::from_tags(&inputs.tags);
    let mut problems = matrix::overclaims(&inputs.manifest, readme, &upper);
    if problems.is_empty() && readme != matrix::render(&inputs.manifest, &upper) {
        problems.push(format!(
            "{README} is not what {MANIFEST} and the tags generate (a row's name, round, target or \
             surfaces, the evidence or the summary changed)"
        ));
    }
    problems
}

/// The checks that need no test run: the manifest matches the inventory, every tag names a
/// row and a surface it targets, and the README is exactly what the manifest and the tags
/// generate. Run by `cargo xtask check docs` (so also on docs-only PRs).
pub(crate) fn check_static(root: &Path) -> Result<()> {
    let inputs = load(root)?;
    let readme = fs::read_to_string(root.join(README)).with_context(|| format!("reading {README}"))?;
    fail_on(
        &static_problems(&inputs, &readme),
        &format!("{README} is stale or claims more than the tagged tests (run `cargo xtask capabilities`)"),
    )?;
    println!(
        "capabilities: {} rows, {} tag(s), README up to date",
        inputs.manifest.rows.len(),
        inputs.tags.len()
    );
    Ok(())
}

/// Builds every `(package, target)` in one `cargo test --no-run` and returns each test
/// binary, read from cargo's JSON messages (`compiler-artifact` with an `executable`). Cargo
/// may build more targets than asked (a `--test` name another selected package also has);
/// only the ones asked for are returned.
fn build_tests<'a>(
    root: &Path,
    wanted: impl IntoIterator<Item = &'a (String, Target)>,
) -> Result<BTreeMap<(String, Target), PathBuf>> {
    let wanted: BTreeSet<&(String, Target)> = wanted.into_iter().collect();
    let mut args = vec![
        "test".to_owned(),
        "--locked".to_owned(),
        "--all-features".to_owned(),
        "--no-run".to_owned(),
        "--message-format=json-render-diagnostics".to_owned(),
    ];
    let packages: BTreeSet<&str> = wanted.iter().map(|(krate, _)| krate.as_str()).collect();
    for krate in packages {
        args.extend(["-p".to_owned(), krate.to_owned()]);
    }
    let targets: BTreeSet<String> = wanted.iter().map(|(_, target)| target.to_string()).collect();
    args.extend(targets.iter().flat_map(|t| t.split(' ').map(str::to_owned)));
    step(&format!("cargo {}", args.join(" ")));
    let output = Command::new("cargo")
        .args(&args)
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("failed to start cargo")?;
    if !output.status.success() {
        bail!("`cargo {}` failed with {}", args.join(" "), output.status);
    }
    let binaries = test_binaries(&String::from_utf8_lossy(&output.stdout));
    Ok(binaries.into_iter().filter(|(key, _)| wanted.contains(key)).collect())
}

/// `(package directory name, target) → executable` from cargo's JSON messages.
fn test_binaries(messages: &str) -> BTreeMap<(String, Target), PathBuf> {
    let mut binaries = BTreeMap::new();
    for line in messages.lines() {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let text = |pointer: &str| message.pointer(pointer).and_then(Value::as_str);
        if text("/reason") != Some("compiler-artifact") {
            continue;
        }
        let (Some(executable), Some(name), Some(manifest)) =
            (text("/executable"), text("/target/name"), text("/manifest_path"))
        else {
            continue;
        };
        let Some(krate) = Path::new(manifest)
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
        else {
            continue;
        };
        let kinds: Vec<&str> = message
            .pointer("/target/kind")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let target = if kinds.contains(&"test") {
            Target::Test(name.to_owned())
        } else if kinds.contains(&"bin") {
            Target::Bin(name.to_owned())
        } else if kinds.iter().any(|k| k.ends_with("lib") || *k == "proc-macro") {
            Target::Lib
        } else {
            continue; // examples, benches
        };
        binaries.insert((krate.to_owned(), target), PathBuf::from(executable));
    }
    binaries
}

/// Runs one test binary filtered to `names`, as `cargo test` would (from the package's
/// directory); returns its stdout. A test failure is a result, not an error; a binary that
/// fails before reporting (no `test result:` line) is an error.
fn run_tests(root: &Path, krate: &str, binary: &Path, names: &[String]) -> Result<String> {
    let dir = root.join("crates").join(krate);
    step(&format!("{} {}", binary.display(), names.join(" ")));
    let output = Command::new(binary)
        .args(names)
        .current_dir(&dir)
        .env("CARGO_MANIFEST_DIR", &dir)
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("failed to start {}", binary.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    print!("{stdout}");
    if !output.status.success() && !stdout.contains("test result:") {
        bail!(
            "{} failed with {} before running tests",
            binary.display(),
            output.status
        );
    }
    Ok(stdout)
}

fn fail_on(problems: &[String], headline: &str) -> Result<()> {
    if problems.is_empty() {
        return Ok(());
    }
    for problem in problems {
        eprintln!("  {problem}");
    }
    bail!("{headline} ({} problem(s) above)", problems.len());
}

#[cfg(test)]
mod tests {
    //! The whole pipeline over a fake manifest, inventory, sources and test results.

    use super::*;

    const MANIFEST: &str = r#"
version = 1

[[group]]
id = "C"
name = "Page organisation"

[[op]]
id = "C.01"
name = "Insert blank pages"
round = "E2"

[[op]]
id = "C.02"
name = "Rotate pages"
round = "E2"
"#;

    const INVENTORY: &str = "\
## 3. Inventory

### C. Page organisation

| Operation | How | Diff. |
| --- | --- | --- |
| Insert blank pages | new page | 1 |
| Rotate pages | `/Rotate` | 1 |

## 4. Next
";

    const VIEWS: &str = "
/// capability: C.02 core read
#[test]
fn rotate_is_read() {}
";

    const SUITE: &str = "
/// capability: C.02 core round-trip
#[test]
fn suite_round_trips() {}
";

    fn fake_inputs() -> Inputs {
        inputs(
            MANIFEST,
            INVENTORY,
            [
                (Path::new("crates/lamina-model/tests/views.rs"), VIEWS),
                (Path::new(matrix::ROUND_TRIP_SUITE), SUITE),
            ],
        )
        .unwrap()
    }

    fn results(suite: &str) -> BTreeMap<(String, Target), BTreeMap<String, bool>> {
        BTreeMap::from([
            (
                ("lamina-model".to_owned(), Target::Test("views".to_owned())),
                tags::parse_libtest("test pages::rotate_is_read ... ok\n"),
            ),
            (
                ("lamina-save".to_owned(), Target::Test("roundtrip_suite".to_owned())),
                tags::parse_libtest(&format!("test tests::suite_round_trips ... {suite}\n")),
            ),
        ])
    }

    #[test]
    fn evaluate_should_render_the_best_proven_status() {
        let readme = evaluate(&fake_inputs(), &results("ok")).unwrap();
        assert!(readme.contains("| C.02 | Rotate pages | 🟢 round-trip | — | — | — | E2 round-trip |"));
    }

    #[test]
    fn evaluate_should_fail_when_a_tagged_test_fails() {
        assert!(evaluate(&fake_inputs(), &results("FAILED")).is_err());
    }

    #[test]
    fn evaluate_should_fail_when_a_tagged_test_did_not_run() {
        let mut results = results("ok");
        results.remove(&("lamina-save".to_owned(), Target::Test("roundtrip_suite".to_owned())));
        assert!(evaluate(&fake_inputs(), &results).is_err());
    }

    #[test]
    fn a_generated_readme_edited_to_claim_more_is_an_overclaim() {
        let inputs = fake_inputs();
        let readme = evaluate(&inputs, &results("ok")).unwrap().replace(
            "| C.01 | Insert blank pages | —",
            "| C.01 | Insert blank pages | 🔵 write",
        );
        let upper = Matrix::from_tags(&inputs.tags);
        assert_eq!(matrix::overclaims(&inputs.manifest, &readme, &upper).len(), 1);
    }

    #[test]
    fn static_problems_should_accept_the_generated_readme() {
        let inputs = fake_inputs();
        let readme = evaluate(&inputs, &results("ok")).unwrap();
        assert_eq!(static_problems(&inputs, &readme), Vec::<String>::new());
    }

    #[test]
    fn static_problems_should_catch_a_manifest_edit_the_readme_does_not_follow() {
        let readme = evaluate(&fake_inputs(), &results("ok")).unwrap();
        let edited = inputs(
            &MANIFEST.replacen("round = \"E2\"", "round = \"E3\"", 1),
            INVENTORY,
            [
                (Path::new("crates/lamina-model/tests/views.rs"), VIEWS),
                (Path::new(matrix::ROUND_TRIP_SUITE), SUITE),
            ],
        )
        .unwrap();
        assert_eq!(static_problems(&edited, &readme).len(), 1);
    }

    #[test]
    fn static_problems_should_catch_an_underclaim() {
        let inputs = fake_inputs();
        let readme = evaluate(&inputs, &results("ok")).unwrap().replace(
            "| C.02 | Rotate pages | 🟢 round-trip",
            "| C.02 | Rotate pages | 🟡 read",
        );
        assert_eq!(static_problems(&inputs, &readme).len(), 1);
    }

    const CARGO_MESSAGES: &str = r#"{"reason":"compiler-artifact","manifest_path":"/r/crates/lamina-save/Cargo.toml","target":{"kind":["test"],"name":"roundtrip_suite"},"executable":"/r/target/debug/deps/roundtrip_suite-1"}
{"reason":"compiler-artifact","manifest_path":"/r/crates/lamina-save/Cargo.toml","target":{"kind":["lib"],"name":"lamina_save"},"executable":"/r/target/debug/deps/lamina_save-2"}
{"reason":"compiler-artifact","manifest_path":"/r/crates/lamina-save/Cargo.toml","target":{"kind":["lib"],"name":"lamina_save"},"executable":null}
{"reason":"compiler-artifact","manifest_path":"/r/crates/x/Cargo.toml","target":{"kind":["bin"],"name":"tool"},"executable":"/r/target/debug/deps/tool-3"}
{"reason":"build-finished","success":true}"#;

    #[test]
    fn test_binaries_should_map_each_executable_to_its_package_and_target() {
        let binaries = test_binaries(CARGO_MESSAGES);
        assert_eq!(
            binaries.keys().cloned().collect::<Vec<_>>(),
            [
                ("lamina-save".to_owned(), Target::Lib),
                ("lamina-save".to_owned(), Target::Test("roundtrip_suite".to_owned())),
                ("x".to_owned(), Target::Bin("tool".to_owned())),
            ]
        );
    }

    #[test]
    fn inputs_should_reject_a_manifest_missing_an_inventory_operation() {
        let manifest = MANIFEST.replace("Insert blank pages", "Insert pages");
        assert!(inputs(&manifest, INVENTORY, []).is_err());
    }

    #[test]
    fn inputs_should_reject_a_tag_for_an_unknown_row() {
        let source = "/// capability: C.03 core read\n#[test]\nfn t() {}\n";
        assert!(inputs(MANIFEST, INVENTORY, [(Path::new("crates/a/tests/b.rs"), source)]).is_err());
    }
}
