//! `cargo xtask docs-previews` (todo B7d): pictures of every page of the example goldens
//! (`fixtures/golden/examples/*.pdf`) for the docs site's Result tabs, rendered by
//! `tools/render`'s `docs-previews` (hayro, 144 dpi) into `apps/docs/public/examples/preview/`,
//! with `manifest.json`: each example's pages (pixel sizes), its PDF's size in bytes and SHA-256.
//! [`check`], part of `check docs`, compares the manifest with the PDFs, so an example that
//! changes can't ship with pictures of its old output.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::step;
use crate::util::sha256_hex;
use crate::workspace::Workspace;

/// The example goldens, relative to the repository root.
const GOLDENS: &str = "fixtures/golden/examples";
/// Where the pictures and the manifest go, relative to the repository root.
pub(crate) const PREVIEWS: &str = "apps/docs/public/examples/preview";

/// `manifest.json`: one entry per example golden, by file stem.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub examples: BTreeMap<String, Example>,
}

/// One example golden and its pictures.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Example {
    /// The PDF's SHA-256, lowercase hex.
    pub sha256: String,
    /// The PDF's size in bytes, shown beside its link.
    pub bytes: u64,
    /// Each page's picture size in pixels, in page order; page `i` is `<stem>-<i>.png`.
    pub pages: Vec<PagePicture>,
}

/// A page picture's size in pixels.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PagePicture {
    pub width: u32,
    pub height: u32,
}

/// One line the render tool prints per page.
#[derive(Deserialize)]
struct Rendered {
    example: String,
    page: usize,
    width: u32,
    height: u32,
}

/// Renders every page of every example golden and writes the manifest.
pub(crate) fn run(ws: &Workspace) -> Result<()> {
    let out = ws.root.join(PREVIEWS);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    // `cargo run` finds the binary on every system (a path would miss Windows' `.exe`).
    let manifest_path = ws.root.join(crate::render::MANIFEST);
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--locked",
            "--release",
            "--bin",
            "docs-previews",
            "--manifest-path",
        ])
        .arg(&manifest_path)
        .arg("--")
        .arg(ws.root.join(GOLDENS))
        .arg(&out)
        .current_dir(&ws.root)
        .output()
        .context("running tools/render's docs-previews")?;
    if !output.status.success() {
        bail!("docs-previews failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let mut manifest = Manifest::default();
    for line in String::from_utf8(output.stdout)?.lines() {
        let page: Rendered = serde_json::from_str(line).with_context(|| format!("reading `{line}`"))?;
        let entry = match manifest.examples.entry(page.example.clone()) {
            std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::btree_map::Entry::Vacant(e) => {
                let pdf = fs::read(pdf_path(&ws.root, &page.example))?;
                e.insert(Example {
                    sha256: sha256_hex(&pdf),
                    bytes: pdf.len() as u64,
                    pages: Vec::new(),
                })
            }
        };
        if page.page != entry.pages.len() {
            bail!("{} page {}: pages out of order", page.example, page.page);
        }
        entry.pages.push(PagePicture {
            width: page.width,
            height: page.height,
        });
    }
    let json = serde_json::to_string_pretty(&manifest)? + "\n";
    fs::write(out.join("manifest.json"), json)?;
    step(&format!(
        "Docs previews rewritten: {} examples in {PREVIEWS}; review them before committing",
        manifest.examples.len()
    ));
    Ok(())
}

/// Fails when the manifest and the example goldens disagree: a PDF changed, appeared or went.
pub(crate) fn check(root: &Path) -> Result<()> {
    let path = root.join(PREVIEWS).join("manifest.json");
    let manifest: Manifest =
        serde_json::from_str(&fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?)?;
    let problems = problems(root, &manifest)?;
    if problems.is_empty() {
        return Ok(());
    }
    bail!(
        "the docs' example pictures are out of date:\n  {}\nrun `cargo xtask docs-previews` and commit {PREVIEWS}",
        problems.join("\n  ")
    )
}

/// Each disagreement between `manifest` and the PDFs in `root`'s goldens, as a sentence.
fn problems(root: &Path, manifest: &Manifest) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let mut stems = Vec::new();
    for entry in fs::read_dir(root.join(GOLDENS))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "pdf")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            stems.push(stem.to_owned());
        }
    }
    stems.sort();
    for stem in &stems {
        match manifest.examples.get(stem) {
            None => problems.push(format!("{stem}: no pictures")),
            Some(example) => {
                let pdf = fs::read(pdf_path(root, stem))?;
                if sha256_hex(&pdf) != example.sha256 {
                    problems.push(format!("{stem}: the PDF changed since its pictures were made"));
                }
                for page in 0..example.pages.len() {
                    if !root.join(PREVIEWS).join(format!("{stem}-{page}.png")).is_file() {
                        problems.push(format!("{stem}: page {page}'s picture is missing"));
                    }
                }
            }
        }
    }
    for stem in manifest.examples.keys().filter(|s| !stems.contains(s)) {
        problems.push(format!("{stem}: pictures of a PDF that no longer exists"));
    }
    Ok(problems)
}

fn pdf_path(root: &Path, stem: &str) -> PathBuf {
    root.join(GOLDENS).join(format!("{stem}.pdf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch repository (named `name`) with one golden, `hello.pdf`, and a picture of it.
    fn repo(name: &str, pdf: &[u8], recorded: &[u8]) -> Result<(PathBuf, Manifest)> {
        let dir = std::env::temp_dir().join(format!("xtask-docs-previews-{name}-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(dir.join(GOLDENS))?;
        fs::create_dir_all(dir.join(PREVIEWS))?;
        fs::write(dir.join(GOLDENS).join("hello.pdf"), pdf)?;
        fs::write(dir.join(PREVIEWS).join("hello-0.png"), b"png")?;
        let mut manifest = Manifest::default();
        manifest.examples.insert(
            "hello".into(),
            Example {
                sha256: sha256_hex(recorded),
                bytes: 3,
                pages: vec![PagePicture { width: 1, height: 1 }],
            },
        );
        Ok((dir, manifest))
    }

    #[test]
    fn matching_pictures_have_no_problems() -> Result<()> {
        let (dir, manifest) = repo("match", b"%PDF", b"%PDF")?;
        assert_eq!(problems(&dir, &manifest)?, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn a_changed_pdf_names_its_example() -> Result<()> {
        let (dir, manifest) = repo("changed", b"%PDF-new", b"%PDF")?;
        assert_eq!(
            problems(&dir, &manifest)?,
            vec!["hello: the PDF changed since its pictures were made"]
        );
        Ok(())
    }

    #[test]
    fn a_new_pdf_and_a_missing_picture_are_problems() -> Result<()> {
        let (dir, manifest) = repo("missing", b"%PDF", b"%PDF")?;
        fs::write(dir.join(GOLDENS).join("invoice.pdf"), b"%PDF")?;
        fs::remove_file(dir.join(PREVIEWS).join("hello-0.png"))?;
        assert_eq!(
            problems(&dir, &manifest)?,
            vec!["hello: page 0's picture is missing", "invoice: no pictures"]
        );
        Ok(())
    }
}
