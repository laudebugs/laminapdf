//! The docs that ship inside the packages (L5a): after `cargo xtask publish npm-pack` and before
//! `cargo publish`, check that `@laminapdf/sdk`'s tarball and the `laminapdf` crate carry `docs/`
//! with every page, every code page, the schemas and `renames.json`, each Markdown file stamped
//! with the package's version. `apps/docs/scripts/package-docs.mjs` writes the folder.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

/// The files every package's `docs/` holds, besides one page per code.
const REQUIRED: [&str; 7] = [
    "llms.txt",
    "AGENTS.md",
    "renames.json",
    "errors/README.md",
    "pages/index.md",
    "schemas/error.schema.json",
    "schemas/renames.schema.json",
];

/// `@laminapdf/sdk`'s tarball, unpacked into a scratch folder: its `docs/` with the API reference.
pub(crate) fn check_npm_tarball(ws: &Workspace, tarball: &Path, version: &str) -> Result<()> {
    let scratch = ws.root.join("target/package-docs/npm");
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).with_context(|| format!("creating {}", scratch.display()))?;
    let tarball = tarball.to_str().context("the tarball path is not UTF-8")?;
    let into = scratch.to_str().context("the scratch path is not UTF-8")?;
    crate::run(&ws.root, "tar", &["-xzf", tarball, "-C", into])?;
    let problems = problems(&scratch.join("package/docs"), &codes(ws)?, version, true)?;
    report(&format!("{tarball}: docs/"), &problems)
}

/// The `laminapdf` crate: `docs/` on disk, and `cargo package --list` naming it (with
/// `--allow-dirty` when the generated docs aren't committed as they are).
pub(crate) fn check_crate(ws: &Workspace, version: &str, allow_dirty: bool) -> Result<()> {
    let docs = ws.root.join("crates/laminapdf/docs");
    let mut problems = problems(&docs, &codes(ws)?, version, false)?;
    let mut args = vec!["package", "--list", "-p", "laminapdf"];
    if allow_dirty {
        args.push("--allow-dirty");
    }
    let output = std::process::Command::new(crate::program_path("cargo"))
        .args(&args)
        .current_dir(&ws.root)
        .output()
        .context("running `cargo package --list`")?;
    if !output.status.success() {
        bail!(
            "`cargo package --list -p laminapdf` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    for file in REQUIRED {
        if !listing.lines().any(|line| line.trim() == format!("docs/{file}")) {
            problems.push(format!("`cargo package --list` leaves out docs/{file}"));
        }
    }
    report("crates/laminapdf/docs/", &problems)
}

fn report(what: &str, problems: &[String]) -> Result<()> {
    if problems.is_empty() {
        println!("{what}: complete, every file at its version");
        return Ok(());
    }
    bail!("{what}:\n  {}", problems.join("\n  "))
}

/// The codes with a page in `docs/errors/`.
fn codes(ws: &Workspace) -> Result<Vec<String>> {
    let dir = ws.root.join("docs/errors");
    let mut codes = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if let Some(code) = name.strip_suffix(".md").filter(|c| *c != "README") {
            codes.push(code.to_owned());
        }
    }
    Ok(codes)
}

/// What `docs` lacks: a required file, a code page, the API reference (`api`), or a Markdown file
/// whose front matter names another version.
fn problems(docs: &Path, codes: &[String], version: &str, api: bool) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let mut expected: Vec<String> = REQUIRED.iter().map(|f| (*f).to_owned()).collect();
    expected.extend(codes.iter().map(|code| format!("errors/{code}.md")));
    if api {
        expected.push("api/README.md".to_owned());
    }
    for file in &expected {
        if !docs.join(file).is_file() {
            problems.push(format!("missing {file}"));
        }
    }
    let stamp = format!("version: \"{version}\"");
    for path in markdown_files(docs)? {
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        if !front_matter_has(&text, &stamp) {
            let shown = path.strip_prefix(docs).unwrap_or(&path).display().to_string();
            problems.push(format!("{shown}: its front matter lacks `{stamp}`"));
        }
    }
    Ok(problems)
}

/// Whether `text` opens with front matter holding the line `line`.
fn front_matter_has(text: &str, line: &str) -> bool {
    let Some(rest) = text.strip_prefix("---\n") else {
        return false;
    };
    rest.split("\n---")
        .next()
        .is_some_and(|head| head.lines().any(|l| l == line))
}

fn markdown_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(files);
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(markdown_files(&path)?);
        } else if path.extension().is_some_and(|e| e == "md") {
            files.push(path);
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{front_matter_has, problems};
    use crate::tracked::TempTree;

    const STAMP: &str = "---\nversion: \"1.0.0\"\n---\n\n# x\n";

    fn complete() -> Vec<(&'static str, &'static str)> {
        vec![
            ("llms.txt", "# Lamina PDF\n"),
            ("AGENTS.md", STAMP),
            ("renames.json", "{}"),
            ("errors/README.md", STAMP),
            ("errors/wrong-password.md", STAMP),
            ("pages/index.md", STAMP),
            ("schemas/error.schema.json", "{}"),
            ("schemas/renames.schema.json", "{}"),
        ]
    }

    #[test]
    fn a_complete_folder_has_no_problems() {
        let tree = TempTree::new("package-docs-complete", &complete());
        let codes = ["wrong-password".to_owned()];
        assert_eq!(problems(&tree.0, &codes, "1.0.0", false).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn a_missing_code_page_api_or_stamp_is_a_problem() {
        let mut files = complete();
        files.retain(|(path, _)| *path != "errors/wrong-password.md");
        files.push(("pages/old.md", "---\nversion: \"0.9.0\"\n---\n"));
        let tree = TempTree::new("package-docs-gaps", &files);
        let codes = ["wrong-password".to_owned()];
        let found = problems(&tree.0, &codes, "1.0.0", true).unwrap();
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found.iter().any(|p| p.contains("errors/wrong-password.md")));
        assert!(found.iter().any(|p| p.contains("api/README.md")));
        assert!(found.iter().any(|p| p.starts_with("pages/old.md")));
    }

    #[test]
    fn the_stamp_must_be_in_the_front_matter() {
        assert!(front_matter_has(STAMP, "version: \"1.0.0\""));
        assert!(!front_matter_has("# x\n\nversion: \"1.0.0\"\n", "version: \"1.0.0\""));
    }
}
