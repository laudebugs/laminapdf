//! `cargo xtask capabilities [--check]`: the test-backed capability matrix (full-editor §3, todo X2).
//!
//! The rows are `docs/capabilities/manifest.toml`, one per operation of the feature inventory
//! (`docs/research/pdf/06-feature-inventory.md` §3; the two must agree). Tests claim a status
//! for a row and surface with a tag ([`tags`]); this task runs every tagged test with
//! `cargo test` and rewrites `docs/capabilities/README.md` from what they prove. `--check`
//! writes nothing and fails when the README differs (the full lane). Any tagged test that fails
//! or isn't found fails both. [`check_static`] is the part that needs no test run (`check docs`).

mod manifest;
mod matrix;
mod tags;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

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
    let mut tags = Vec::new();
    for (path, text) in sources {
        if tags::is_ts_test(path) {
            tags::scan_ts(path, text)?;
        } else {
            tags.extend(tags::scan_rust(path, text)?);
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
    let mut results = BTreeMap::new();
    for ((krate, target), names) in tags::runs(&inputs.tags) {
        let output = cargo_test(root, &krate, &target, &names)?;
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

/// The checks that need no test run: the manifest matches the inventory, every tag names a
/// row and a surface it targets, and the README claims nothing more than the tags could prove
/// and lists every row once. Run by `cargo xtask check docs` (so also on docs-only PRs).
pub(crate) fn check_static(root: &Path) -> Result<()> {
    let inputs = load(root)?;
    let readme = fs::read_to_string(root.join(README)).with_context(|| format!("reading {README}"))?;
    let upper = Matrix::from_tags(&inputs.tags);
    fail_on(
        &matrix::overclaims(&inputs.manifest, &readme, &upper),
        &format!("{README} claims more than the tagged tests (run `cargo xtask capabilities`)"),
    )?;
    println!(
        "capabilities: {} rows, {} tag(s), README claims within them",
        inputs.manifest.rows.len(),
        inputs.tags.len()
    );
    Ok(())
}

/// `cargo test` for one package and target, filtered to `names`; returns its stdout. A test
/// failure is a result, not an error; a build failure (no `test result:` line) is an error.
fn cargo_test(root: &Path, krate: &str, target: &Target, names: &[String]) -> Result<String> {
    let mut args = vec!["test", "--locked", "--all-features", "-p", krate];
    match target {
        Target::Lib => args.push("--lib"),
        Target::Test(name) => args.extend(["--test", name.as_str()]),
    }
    args.push("--");
    args.extend(names.iter().map(String::as_str));
    step(&format!("cargo {}", args.join(" ")));
    let output = Command::new("cargo")
        .args(&args)
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("failed to start cargo")?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    print!("{stdout}");
    if !output.status.success() && !stdout.contains("test result:") {
        bail!(
            "`cargo {}` failed with {} before running tests",
            args.join(" "),
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
                (Path::new("crates/lamina-save/src/lib.rs"), SUITE),
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
                ("lamina-save".to_owned(), Target::Lib),
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
        results.remove(&("lamina-save".to_owned(), Target::Lib));
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
