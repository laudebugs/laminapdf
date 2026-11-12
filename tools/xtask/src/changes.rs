//! `cargo xtask changes`: does a branch touch code, or only docs? The full lane's first job
//! (`.github/workflows/full.yml`, ADR 0018 §2) runs it, and its suites run only for code.
//!
//! The filter is an allowlist of documentation paths ([`is_docs`], where it is defined); every
//! other path counts as code. Inverting it this way means a root config file nobody thought to list
//! (`.cargo/config.toml`, `.gitattributes`, `rust-toolchain.toml`, a new workflow) always runs
//! the suites instead of slipping through.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::tracked::git;

/// `cargo xtask changes [--base <rev>]`: lists the paths changed since the merge base of
/// `base` and `HEAD`, prints whether any of them is code, and appends `code=true|false` to
/// `$GITHUB_OUTPUT` when it is set (in GitHub Actions).
pub(crate) fn run(root: &Path, base: &str) -> Result<()> {
    let merge_base = git(root, &["merge-base", base, "HEAD"])
        .with_context(|| format!("no merge base of {base} and HEAD; fetch {base} with full history"))?;
    let merge_base = merge_base.trim();
    // --no-renames: a rename out of code (crates/x.rs → docs/x.rs) lists both of its paths.
    let diff = git(root, &["diff", "--name-only", "--no-renames", "-z", merge_base, "HEAD"])?;
    let paths: Vec<&str> = diff.split('\0').filter(|p| !p.is_empty()).collect();
    let code = touches_code(&paths);
    println!("{} paths changed since {merge_base} ({base}):", paths.len());
    for path in &paths {
        let kind = if is_docs(path) { "docs" } else { "code" };
        println!("  {kind}  {path}");
    }
    let gallery = touches_gallery(&paths);
    let web = touches_web(&paths);
    let desktop = touches_desktop(&paths);
    let render = touches_render(&paths);
    println!("code={code}");
    println!("gallery={gallery}");
    println!("web={web}");
    println!("desktop={desktop}");
    println!("render={render}");
    crate::util::github_output("code", &code.to_string())?;
    crate::util::github_output("gallery", &gallery.to_string())?;
    crate::util::github_output("web", &web.to_string())?;
    crate::util::github_output("desktop", &desktop.to_string())?;
    crate::util::github_output("render", &render.to_string())
}

/// What the desktop app's three-OS build depends on (apps/desktop, ADR 0008): the app, the
/// packages its frontend imports, the crates (the engine will be one), the manifests, lockfiles,
/// toolchains and shared configs, and the workflow and actions that build it.
const DESKTOP_INPUTS: [&str; 15] = [
    "apps/desktop/",
    "packages/",
    "crates/",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "package.json",
    ".nvmrc",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "tsconfig.base.json",
    "biome.json",
    ".github/workflows/full.yml",
    ".github/actions/linux-webkit/",
    ".github/actions/node-pnpm/",
];

/// Whether any of `paths` can change how the desktop app builds, so the full lane builds it.
pub(crate) fn touches_desktop(paths: &[&str]) -> bool {
    paths
        .iter()
        .any(|path| !is_docs(path) && DESKTOP_INPUTS.iter().any(|input| path.starts_with(input)))
}

/// What the render goldens draw from (`cargo xtask render-golden`, the full lane's macOS job; todo
/// X9b): the committed example PDFs and their pictures, the corpus and its goldens, the renderer
/// tools, every crate (the render crates read through the core ones, and the examples come from
/// the façade), the task itself, and the manifests, toolchain and workflow that build them. Wide on
/// purpose: a path left out lets a drawing change through until the next push to main.
const RENDER_INPUTS: [&str; 12] = [
    "crates/",
    "fixtures/golden/examples/",
    "fixtures/golden/render/",
    "fixtures/render-golden/",
    "fixtures/corpus/",
    "tools/render/",
    "tools/xtask/",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/",
    ".github/workflows/full.yml",
];

/// Whether any of `paths` can change what the render goldens draw, so the full lane runs them.
pub(crate) fn touches_render(paths: &[&str]) -> bool {
    paths
        .iter()
        .any(|path| !is_docs(path) && RENDER_INPUTS.iter().any(|input| path.starts_with(input)))
}

/// What the Paper gallery's screenshots and axe checks depend on (apps/gallery, todo U6f): the
/// gallery, the packages it draws, the versions pinned for them, and the command that runs its
/// tests.
const GALLERY_INPUTS: [&str; 8] = [
    "apps/gallery/",
    "packages/ui/",
    "packages/tokens/",
    "packages/icons/",
    "packages/fonts/",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "tools/xtask/src/gallery.rs",
];

/// Whether any of `paths` can change how the gallery looks, so the fast lane runs its tests.
pub(crate) fn touches_gallery(paths: &[&str]) -> bool {
    paths
        .iter()
        .any(|path| GALLERY_INPUTS.iter().any(|input| path.starts_with(input)))
}

/// Paths the reader can't depend on (todo C4d): the other apps, the tools other than xtask, the
/// spikes, the fuzz workspace and the other workflows. [`touches_web`] counts every changed path
/// outside these and outside the docs ([`is_docs`]), so a root config file or a shared action
/// nobody thought to list redeploys the reader, as [`touches_code`] does for the suites.
const OUTSIDE_WEB: [&str; 7] = [
    "apps/docs/",
    "apps/site/",
    "apps/gallery/",
    "spikes/",
    "fuzz/",
    "tools/",
    ".github/workflows/",
];

/// Paths under [`OUTSIDE_WEB`] that the reader's deploys still depend on: the build tasks and the
/// deploy workflow.
const WEB_ANYWAY: [&str; 2] = ["tools/xtask/", ".github/workflows/web-deploy.yml"];

/// Whether any of `paths` can change the reader as deployed. The preview and staging deploys
/// (`.github/workflows/web-deploy.yml`) run only when one does; the production deploy runs for
/// every `web-v*` tag.
pub(crate) fn touches_web(paths: &[&str]) -> bool {
    paths.iter().any(|path| {
        let under = |prefix: &&str| path.starts_with(prefix);
        !is_docs(path) && (WEB_ANYWAY.iter().any(under) || !OUTSIDE_WEB.iter().any(under))
    })
}

/// Old path → new path for each file renamed between `merge_base` and the working tree.
pub(crate) fn renames(root: &Path, merge_base: &str) -> Result<BTreeMap<String, String>> {
    let diff = git(
        root,
        &[
            "diff",
            "--name-status",
            "-M",
            "-l0",
            "--diff-filter=R",
            "-z",
            merge_base,
        ],
    )?;
    Ok(parse_renames(&diff))
}

/// The renames in `git diff --name-status --diff-filter=R -z` output: each is `R<score>`, the old
/// path and the new one, each ended by a NUL.
fn parse_renames(name_status: &str) -> BTreeMap<String, String> {
    let fields: Vec<&str> = name_status.split('\0').filter(|field| !field.is_empty()).collect();
    let (records, _partial) = fields.as_chunks::<3>();
    records
        .iter()
        .map(|[_, old, new]| ((*old).to_owned(), (*new).to_owned()))
        .collect()
}

/// The text of `path` at revision `rev`.
pub(crate) fn at_revision(root: &Path, rev: &str, path: &str) -> Result<String> {
    git(root, &["show", &format!("{rev}:{path}")])
}

/// True when any path is outside the docs allowlist. No paths at all is docs-only.
pub(crate) fn touches_code(paths: &[&str]) -> bool {
    paths.iter().any(|p| !is_docs(p))
}

/// The docs allowlist, the one place it is defined (ADR 0018 §2 implementation note):
///
/// - anything under `docs/`;
/// - Markdown files at the repository root (`README.md`, `CLAUDE.md`, `CHANGELOG.md`);
/// - `LICENSE*` files at the root;
/// - the GitHub issue templates (`.github/ISSUE_TEMPLATE/`) and the pull request template
///   (`.github/PULL_REQUEST_TEMPLATE.md`).
///
/// Markdown anywhere else is code: a crate README can be its rustdoc and doc tests
/// (`#![doc = include_str!("../README.md")]`), and a docs site's pages are its build input.
pub(crate) fn is_docs(path: &str) -> bool {
    let at_root = !path.contains('/');
    let is_markdown = Path::new(path)
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("md"));
    path.starts_with("docs/")
        || (at_root && (is_markdown || path.starts_with("LICENSE")))
        || path.starts_with(".github/ISSUE_TEMPLATE/")
        || path == ".github/PULL_REQUEST_TEMPLATE.md"
}

/// The default base when `--base` isn't given: `origin/main` when that ref exists (a normal
/// clone, CI), else `main` (this repository's bare clone with worktrees has no `origin/*`).
pub(crate) fn default_base(root: &Path) -> &'static str {
    pick_base(git(root, &["rev-parse", "--verify", "--quiet", "origin/main"]).is_ok())
}

fn pick_base(has_origin_main: bool) -> &'static str {
    if has_origin_main { "origin/main" } else { "main" }
}

#[cfg(test)]
mod tests {
    use super::{is_docs, pick_base, touches_code, touches_desktop, touches_gallery, touches_render, touches_web};

    #[test]
    fn touches_web_should_follow_the_app_its_sources_build_tasks_and_root_config() {
        for path in [
            "apps/web/src/app.tsx",
            "packages/viewer/src/index.ts",
            "crates/laminapdf-wasm/src/lib.rs",
            "fixtures/corpus/hello.pdf",
            "tools/xtask/src/web.rs",
            "pnpm-lock.yaml",
            "Cargo.lock",
            "package.json",
            ".npmrc",
            ".cargo/config.toml",
            ".github/actions/node-pnpm/action.yml",
            ".github/workflows/web-deploy.yml",
            "a-new-root-file.toml",
        ] {
            assert!(touches_web(&[path]), "{path}");
        }
    }

    #[test]
    fn touches_web_should_skip_docs_other_apps_other_tools_and_other_workflows() {
        assert!(!touches_web(&[
            "docs/todos/README.md",
            "README.md",
            "apps/docs/src/content/docs/index.mdx",
            "apps/site/package.json",
            "apps/gallery/src/main.tsx",
            "tools/render/Cargo.toml",
            "spikes/pdf-basics/rust/src/lib.rs",
            "fuzz/fuzz_targets/lex.rs",
            ".github/workflows/docs.yml",
        ]));
    }

    #[test]
    fn touches_gallery_should_follow_the_ui_packages_and_the_lockfile() {
        assert!(touches_gallery(&["packages/ui/src/components/button.tsx"]));
        assert!(touches_gallery(&["packages/tokens/tokens.json", "README.md"]));
        assert!(touches_gallery(&["pnpm-lock.yaml"]));
        assert!(touches_gallery(&["tools/xtask/src/gallery.rs"]));
        assert!(!touches_gallery(&[
            "crates/laminapdf/src/lib.rs",
            "packages/uikit/x.ts",
            "docs/README.md"
        ]));
    }

    #[test]
    fn touches_desktop_should_follow_the_app_its_inputs_and_its_workflow() {
        assert!(touches_desktop(&["apps/desktop/src/main.tsx"]));
        assert!(touches_desktop(&["packages/ui/src/index.ts"]));
        assert!(touches_desktop(&["Cargo.lock"]));
        assert!(touches_desktop(&[".github/actions/linux-webkit/action.yml"]));
        assert!(touches_desktop(&[".nvmrc"]));
        assert!(!touches_desktop(&[
            "apps/web/src/app.tsx",
            "docs/README.md",
            "tools/xtask/src/web.rs"
        ]));
    }

    #[test]
    fn touches_render_should_follow_the_crates_the_goldens_and_the_render_tools() {
        assert_eq!(
            [
                "crates/laminapdf-render/src/lib.rs",
                "crates/laminapdf-cos/src/xref.rs",
                "fixtures/golden/render/page-0.png",
                "fixtures/golden/examples/hello.pdf",
                "fixtures/render-golden/thresholds.toml",
                "tools/render/pdfium.py",
                "tools/xtask/src/pdfium.rs",
                "Cargo.lock",
                ".github/workflows/full.yml",
            ]
            .map(|path| touches_render(&[path])),
            [true; 9]
        );
    }

    #[test]
    fn touches_render_should_skip_the_apps_packages_docs_and_other_workflows() {
        assert!(!touches_render(&[
            "apps/web/src/app.tsx",
            "packages/ui/src/index.ts",
            "docs/README.md",
            ".github/workflows/ci.yml",
            "pnpm-lock.yaml",
        ]));
    }

    /// One example per allowlist pattern.
    const DOCS: [&str; 9] = [
        "docs/todos/README.md",
        "docs/research/notes/figure.svg",
        "README.md",
        "CLAUDE.md",
        "CHANGELOG.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        ".github/ISSUE_TEMPLATE/bug.yml",
        ".github/PULL_REQUEST_TEMPLATE.md",
    ];

    /// Root config the suites depend on, code and an unknown new root file.
    const CODE: [&str; 21] = [
        "rust-toolchain.toml",
        ".github/workflows/ci.yml",
        ".github/workflows/full.yml",
        ".github/actions/isolate-rust-home/action.yml",
        ".github/dependabot.yml",
        ".cargo/config.toml",
        ".config/nextest.toml",
        ".gitattributes",
        "biome.json",
        "tsconfig.base.json",
        ".nvmrc",
        ".npmrc",
        "Cargo.toml",
        "Cargo.lock",
        "pnpm-lock.yaml",
        "deny.toml",
        "crates/laminapdf-cos/src/lib.rs",
        "fixtures/corpus/hello.pdf",
        "tools/xtask/src/main.rs",
        "docs.toml",
        "brand-new-root-file",
    ];

    /// Markdown that can be a build input (`#![doc = include_str!("../README.md")]`, doc
    /// tests, a docs site's content) or sits outside the allowlisted folders.
    const CODE_MARKDOWN: [&str; 7] = [
        "crates/laminapdf-cos/README.md",
        "packages/sdk/README.md",
        "apps/docs/src/content/docs/index.md",
        "tools/corpus-facts/README.md",
        "fuzz/README.md",
        "fixtures/README.md",
        ".github/SECURITY.md",
    ];

    #[test]
    fn markdown_outside_the_docs_folders_is_code() {
        let docs: Vec<&str> = CODE_MARKDOWN.into_iter().filter(|p| is_docs(p)).collect();
        assert!(docs.is_empty(), "classified as docs: {docs:?}");
    }

    #[test]
    fn every_allowlist_pattern_is_docs() {
        let code: Vec<&str> = DOCS.into_iter().filter(|p| !is_docs(p)).collect();
        assert!(code.is_empty(), "classified as code: {code:?}");
    }

    #[test]
    fn root_config_and_unknown_files_are_code() {
        let docs: Vec<&str> = CODE.into_iter().filter(|p| is_docs(p)).collect();
        assert!(docs.is_empty(), "classified as docs: {docs:?}");
    }

    #[test]
    fn license_counts_only_at_the_root() {
        assert!(!is_docs("crates/laminapdf-cos/LICENSE-MIT"));
    }

    #[test]
    fn a_docs_only_change_is_not_code() {
        assert!(!touches_code(&["docs/a.md", "README.md", "LICENSE-MIT"]));
    }

    #[test]
    fn one_code_path_makes_a_change_code() {
        assert!(touches_code(&["docs/a.md", ".nvmrc"]));
    }

    #[test]
    fn the_default_base_falls_back_to_main_without_origin() {
        assert_eq!((pick_base(true), pick_base(false)), ("origin/main", "main"));
    }

    #[test]
    fn an_empty_change_is_not_code() {
        assert!(!touches_code(&[]));
    }

    #[test]
    fn parse_renames_should_map_each_old_path_to_its_new_one() {
        let renames = super::parse_renames("R100\0crates/a/x.rs\0crates/b/x.rs\0R087\0a.md\0b.md\0");
        assert_eq!(
            renames.into_iter().collect::<Vec<_>>(),
            [
                ("a.md".to_owned(), "b.md".to_owned()),
                ("crates/a/x.rs".to_owned(), "crates/b/x.rs".to_owned())
            ]
        );
    }
}
