//! `cargo xtask corpus fetch`: the external corpora of ADR 0018 §3, fetched into the gitignored
//! `target/corpus/<source>/` at pinned commits and verified file by file against
//! `fixtures/corpus-manifest.toml` (path, size, SHA-256, plus each source's licence).
//!
//! Corpus files are never committed: only the manifest is. Each source is a sparse, shallow,
//! blob-filtered git checkout of the PDFs its `include` patterns name (plus its licence file),
//! so nothing else from those repositories is downloaded and nothing from them is run. When
//! git fails and a mirror is configured (`mirror` in the manifest or `$LAMINA_CORPUS_MIRROR`),
//! each listed file is downloaded from `<mirror>/<source>/<commit>/<path>` instead and checked
//! against the same hashes; the mirror is a copy, never the source of truth.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::workspace::Workspace;
use crate::{run, step};

/// The manifest, relative to the repository root.
pub(crate) const MANIFEST: &str = "fixtures/corpus-manifest.toml";

/// Where the corpora are checked out, relative to the repository root (gitignored);
/// `$LAMINA_CORPUS_DIR` overrides it (the nightly workflow caches a folder outside `target/`).
const CORPUS_DIR: &str = "target/corpus";

/// The manifest's header; `--update-manifest` writes it back unchanged.
const HEADER: &str = "\
# External corpora for the nightly lane (ADR 0018 §3; todo X3a). Never committed: \
`cargo xtask corpus fetch`
# checks each source out at its pinned commit into target/corpus/<name>/ (gitignored) and verifies
# every file's size and SHA-256 against this list. `cargo xtask corpus fetch --update-manifest`
# rewrites the `files` lists after a `commit` or `include` change; review the diff like code.
#
# Licences: every source is a permissively licensed repository (`licence`). Individual test PDFs
# often come from bug reports and carry no licence statement of their own, so none of them may be
# copied into fixtures/, even minimised (ADR 0018 §3): a regression found here is re-created
# synthetically.
#
# `mirror` (or $LAMINA_CORPUS_MIRROR) is an optional base URL serving <mirror>/<name>/<commit>/<path>,
# used only when git fails; the hashes here stay the source of truth.
";

/// One file of a source, as listed in the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

/// One pinned source: a git repository at a commit, narrowed by sparse-checkout patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) name: String,
    pub(crate) repo: String,
    pub(crate) commit: String,
    pub(crate) licence: String,
    pub(crate) licence_file: String,
    pub(crate) include: Vec<String>,
    pub(crate) files: Vec<Entry>,
}

/// `fixtures/corpus-manifest.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub(crate) mirror: String,
    pub(crate) sources: Vec<Source>,
}

fn string(table: &toml::Table, key: &str, what: &str) -> Result<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{what}: missing string `{key}`"))
}

fn array<'a>(table: &'a toml::Table, key: &str, what: &str) -> Result<&'a [toml::Value]> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .with_context(|| format!("{what}: missing array `{key}`"))
}

impl Manifest {
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let table: toml::Table = text.parse().context("parsing the corpus manifest")?;
        let mirror = table
            .get("mirror")
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let mut sources = Vec::new();
        for value in array(&table, "source", "manifest")? {
            let source = value.as_table().context("[[source]] must be a table")?;
            let name = string(source, "name", "[[source]]")?;
            let what = format!("source {name}");
            let include = array(source, "include", &what)?
                .iter()
                .map(|v| v.as_str().map(str::to_owned).context("include: strings only"))
                .collect::<Result<Vec<_>>>()?;
            let mut files = Vec::new();
            for file in array(source, "files", &what)? {
                let file = file.as_table().context("files: inline tables only")?;
                let size = file
                    .get("size")
                    .and_then(toml::Value::as_integer)
                    .and_then(|n| u64::try_from(n).ok())
                    .with_context(|| format!("{what}: a file without a valid `size`"))?;
                files.push(Entry {
                    path: string(file, "path", &what)?,
                    size,
                    sha256: string(file, "sha256", &what)?,
                });
            }
            sources.push(Source {
                repo: string(source, "repo", &what)?,
                commit: string(source, "commit", &what)?,
                licence: string(source, "licence", &what)?,
                licence_file: string(source, "licence_file", &what)?,
                include,
                files,
                name,
            });
        }
        Ok(Self { mirror, sources })
    }

    /// The manifest as TOML, in the layout `parse` reads (and humans review).
    pub(crate) fn render(&self) -> String {
        let mut out = String::from(HEADER);
        let _ = writeln!(out, "mirror = {}", quote(&self.mirror));
        for source in &self.sources {
            let total: u64 = source.files.iter().map(|f| f.size).sum();
            let _ = writeln!(out, "\n[[source]]");
            let _ = writeln!(out, "name = {}", quote(&source.name));
            let _ = writeln!(out, "repo = {}", quote(&source.repo));
            let _ = writeln!(out, "commit = {}", quote(&source.commit));
            let _ = writeln!(out, "licence = {}", quote(&source.licence));
            let _ = writeln!(out, "licence_file = {}", quote(&source.licence_file));
            let include: Vec<String> = source.include.iter().map(|p| quote(p)).collect();
            let _ = writeln!(out, "include = [{}]", include.join(", "));
            let _ = writeln!(out, "# {} files, {total} bytes", source.files.len());
            let _ = writeln!(out, "files = [");
            for file in &source.files {
                let _ = writeln!(
                    out,
                    "  {{ path = {}, size = {}, sha256 = {} }},",
                    quote(&file.path),
                    file.size,
                    quote(&file.sha256)
                );
            }
            let _ = writeln!(out, "]");
        }
        out
    }
}

/// `s` as a TOML basic string.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn load(ws: &Workspace) -> Result<Manifest> {
    let path = ws.root.join(MANIFEST);
    Manifest::parse(&fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?)
}

/// What `corpus fetch` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct FetchArgs {
    pub(crate) sources: Vec<String>,
    pub(crate) update_manifest: bool,
}

/// Parses `[--source <name>]… [--update-manifest]`.
pub(crate) fn parse_fetch_args(args: &[String]) -> Result<FetchArgs> {
    let mut parsed = FetchArgs::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--source" => parsed
                .sources
                .push(rest.next().context("--source needs a name")?.clone()),
            "--update-manifest" => parsed.update_manifest = true,
            other => bail!("usage: cargo xtask corpus fetch [--source <name>]… [--update-manifest]; got {other:?}"),
        }
    }
    Ok(parsed)
}

/// The sources named by `names` (all when empty); fails on an unknown name.
pub(crate) fn select<'a>(manifest: &'a Manifest, names: &[String]) -> Result<Vec<&'a Source>> {
    if let Some(unknown) = names.iter().find(|n| !manifest.sources.iter().any(|s| &s.name == *n)) {
        let known: Vec<&str> = manifest.sources.iter().map(|s| s.name.as_str()).collect();
        bail!("unknown corpus source {unknown:?}; sources: {}", known.join(", "));
    }
    Ok(manifest
        .sources
        .iter()
        .filter(|s| names.is_empty() || names.contains(&s.name))
        .collect())
}

/// `cargo xtask corpus fetch [--source <name>]… [--update-manifest]`.
pub(crate) fn fetch(ws: &Workspace, args: &FetchArgs) -> Result<()> {
    let mut manifest = load(ws)?;
    let mirror = std::env::var("LAMINA_CORPUS_MIRROR").unwrap_or_else(|_| manifest.mirror.clone());
    let names: Vec<String> = select(&manifest, &args.sources)?
        .iter()
        .map(|s| s.name.clone())
        .collect();
    let mut problems = Vec::new();
    for source in manifest.sources.iter_mut().filter(|s| names.contains(&s.name)) {
        let dir = source_dir(ws, &source.name);
        if let Err(error) = checkout(source, &dir) {
            if mirror.is_empty() || args.update_manifest {
                return Err(error.context(format!("fetching {}", source.name)));
            }
            eprintln!(
                "warning: git fetch of {} failed ({error:#}); trying the mirror",
                source.name
            );
            from_mirror(source, &dir, &mirror)?;
        }
        let actual = scan(&dir, &source.licence_file)?;
        if args.update_manifest {
            source.files = actual;
            continue;
        }
        let found = compare(&source.files, &actual);
        let total: u64 = actual.iter().map(|f| f.size).sum();
        if found.is_empty() {
            step(&format!(
                "{}: {} files ({total} bytes) match the manifest at {}",
                source.name,
                actual.len(),
                short(&source.commit)
            ));
        }
        problems.extend(found.into_iter().map(|p| format!("{}: {p}", source.name)));
    }
    if args.update_manifest {
        fs::write(ws.root.join(MANIFEST), manifest.render())?;
        step(&format!("Rewrote {MANIFEST}; review the diff"));
        return Ok(());
    }
    if !problems.is_empty() {
        let shown: Vec<&String> = problems.iter().take(40).collect();
        bail!(
            "the corpus does not match {MANIFEST} ({} problems):\n  {}",
            problems.len(),
            shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n  ")
        );
    }
    Ok(())
}

fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

pub(crate) fn source_dir(ws: &Workspace, name: &str) -> PathBuf {
    let base = std::env::var_os("LAMINA_CORPUS_DIR").map_or_else(|| ws.root.join(CORPUS_DIR), PathBuf::from);
    base.join(name)
}

fn git(dir: &Path, args: &[&str]) -> Result<()> {
    run(dir, "git", args)
}

fn git_head(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// A sparse, shallow, blob-filtered checkout of `source` at its commit in `dir`; a checkout
/// already at the commit is reused (the CI cache restores one).
fn checkout(source: &Source, dir: &Path) -> Result<()> {
    if !dir.join(".git").is_dir() {
        fs::create_dir_all(dir)?;
        git(dir, &["init", "-q"])?;
        git(dir, &["remote", "add", "origin", &source.repo])?;
    }
    git(dir, &["config", "core.sparseCheckout", "true"])?;
    let mut patterns = vec!["sparse-checkout", "set", "--no-cone"];
    patterns.extend(source.include.iter().map(String::as_str));
    patterns.push(&source.licence_file);
    git(dir, &patterns)?;
    if git_head(dir).as_deref() != Some(source.commit.as_str()) {
        git(
            dir,
            &[
                "fetch",
                "-q",
                "--depth",
                "1",
                "--filter=blob:none",
                "origin",
                &source.commit,
            ],
        )?;
        git(
            dir,
            &["-c", "advice.detachedHead=false", "checkout", "-q", "FETCH_HEAD"],
        )?;
    }
    if git_head(dir).as_deref() != Some(source.commit.as_str()) {
        bail!("{} is not at the pinned commit {}", dir.display(), source.commit);
    }
    Ok(())
}

/// Downloads every listed file from the mirror (`curl`), replacing whatever was in `dir`.
fn from_mirror(source: &Source, dir: &Path, mirror: &str) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    for file in &source.files {
        let target = dir.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let url = format!(
            "{}/{}/{}/{}",
            mirror.trim_end_matches('/'),
            source.name,
            source.commit,
            file.path
        );
        let target = target.to_string_lossy().into_owned();
        run(dir, "curl", &["-fsSL", "--retry", "3", "-o", &target, &url])?;
    }
    Ok(())
}

/// Every file under `dir` except `.git/` and the licence file, sorted by path, with its size
/// and SHA-256.
pub(crate) fn scan(dir: &Path, licence_file: &str) -> Result<Vec<Entry>> {
    let mut paths = Vec::new();
    walk(dir, dir, &mut paths)?;
    let licence = licence_file.trim_start_matches('/');
    let mut out = Vec::new();
    for relative in paths.into_iter().filter(|p| p != licence) {
        let bytes = fs::read(dir.join(&relative)).with_context(|| format!("reading {relative}"))?;
        out.push(Entry {
            size: u64::try_from(bytes.len())?,
            sha256: sha256_hex(&bytes),
            path: relative,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out)?;
        } else {
            let relative = path.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
            out.push(relative);
        }
    }
    Ok(())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Differences between the manifest's list and the files on disk: missing, changed (size or
/// SHA-256) and unlisted files.
pub(crate) fn compare(listed: &[Entry], actual: &[Entry]) -> Vec<String> {
    let on_disk: BTreeMap<&str, &Entry> = actual.iter().map(|e| (e.path.as_str(), e)).collect();
    let mut problems = Vec::new();
    for want in listed {
        match on_disk.get(want.path.as_str()) {
            None => problems.push(format!("{}: missing", want.path)),
            Some(got) if got.sha256 != want.sha256 || got.size != want.size => problems.push(format!(
                "{}: SHA-256 {} ({} bytes), manifest {} ({} bytes)",
                want.path, got.sha256, got.size, want.sha256, want.size
            )),
            Some(_) => {}
        }
    }
    for got in actual {
        if !listed.iter().any(|want| want.path == got.path) {
            problems.push(format!("{}: not in the manifest", got.path));
        }
    }
    problems
}

/// Every verified file of the selected sources as `(name, path)`, where the name is
/// `<source>/<path in the source>`; fails when a source has not been fetched or does not match.
pub(crate) fn verified_inputs(ws: &Workspace, names: &[String]) -> Result<Vec<(String, PathBuf)>> {
    let manifest = load(ws)?;
    let mut out = Vec::new();
    for source in select(&manifest, names)? {
        let dir = source_dir(ws, &source.name);
        if !dir.is_dir() {
            bail!("{} is missing: run `cargo xtask corpus fetch` first", dir.display());
        }
        let problems = compare(&source.files, &scan(&dir, &source.licence_file)?);
        if !problems.is_empty() {
            bail!(
                "{}: {} files differ from {MANIFEST} (first: {}); run `cargo xtask corpus fetch`",
                source.name,
                problems.len(),
                problems.first().map_or("", String::as_str)
            );
        }
        out.extend(
            source
                .files
                .iter()
                .map(|f| (format!("{}/{}", source.name, f.path), dir.join(&f.path))),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, sha256: &str) -> Entry {
        Entry {
            path: path.to_owned(),
            size: 3,
            sha256: sha256.to_owned(),
        }
    }

    fn manifest() -> Manifest {
        Manifest {
            mirror: String::new(),
            sources: vec![Source {
                name: "pdfjs".to_owned(),
                repo: "https://github.com/mozilla/pdf.js.git".to_owned(),
                commit: "87b369cd9d4cfe415b587b26a8d81f486db02874".to_owned(),
                licence: "Apache-2.0".to_owned(),
                licence_file: "/LICENSE".to_owned(),
                include: vec!["/test/pdfs/*.pdf".to_owned()],
                files: vec![entry("test/pdfs/a \"b\".pdf", "00"), entry("test/pdfs/c.pdf", "11")],
            }],
        }
    }

    #[test]
    fn manifest_should_parse_what_it_renders() {
        let m = manifest();
        assert_eq!(Manifest::parse(&m.render()).unwrap(), m);
    }

    #[test]
    fn manifest_should_reject_a_file_without_a_size() {
        let text = manifest().render().replace("size = 3, ", "");
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn compare_should_accept_matching_files() {
        let files = [entry("a.pdf", "00")];
        assert_eq!(compare(&files, &files), Vec::<String>::new());
    }

    #[test]
    fn compare_should_reject_a_tampered_hash() {
        let problems = compare(&[entry("a.pdf", "00")], &[entry("a.pdf", "ff")]);
        assert_eq!(problems, ["a.pdf: SHA-256 ff (3 bytes), manifest 00 (3 bytes)"]);
    }

    #[test]
    fn compare_should_report_missing_and_unlisted_files() {
        let problems = compare(&[entry("a.pdf", "00")], &[entry("b.pdf", "00")]);
        assert_eq!(problems, ["a.pdf: missing", "b.pdf: not in the manifest"]);
    }

    #[test]
    fn select_should_reject_an_unknown_source() {
        assert!(select(&manifest(), &["nope".to_owned()]).is_err());
    }

    #[test]
    fn fetch_args_should_collect_sources() {
        let args: Vec<String> = ["--source", "qpdf", "--source", "pdfjs"].map(String::from).to_vec();
        assert_eq!(parse_fetch_args(&args).unwrap().sources, ["qpdf", "pdfjs"]);
    }

    #[test]
    fn sha256_hex_should_match_the_known_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
