//! What each of the fast lane's runtime suites reads (todo X12; CI caching plan §8.3), worked out
//! from the dependency graph ([`Graph`]) as the desktop's inputs are, so a pull request runs only
//! the suites its paths can reach. The reader's deploys and releases follow the `web` suite's inputs
//! too ([`web_inputs`]).

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::graph::{Graph, outermost};

/// One suite of `cargo xtask pdf-runtimes`: what it runs is in `pdf_runtimes.rs`; here is what
/// it reads.
#[derive(Debug)]
pub(crate) struct Suite {
    /// Its name, as `--suites` and the `changes` job's `suites` output give it.
    pub(crate) name: &'static str,
    /// The workspace package whose tests or build it runs.
    pub(crate) package: &'static str,
    /// That package's directory, where the walk of its `workspace:` dependencies starts.
    root: &'static str,
    /// What it reads outside the packages it reaches. These count even when they are docs by
    /// [`super::is_docs`]: the docs site builds pages from `docs/errors/` and `CHANGELOG.md`.
    extra: &'static [&'static str],
}

/// Every suite, in the order a browser leg runs them. `examples` is the one the server leg runs.
pub(crate) const SUITES: [Suite; 12] = [
    suite("examples", "@laminapdf/sdk", "packages/sdk/", &[]),
    suite(
        "render-pdfium",
        "@laminapdf/render-pdfium",
        "packages/render-pdfium/",
        &[],
    ),
    suite("render-hayro", "@laminapdf/render-hayro", "packages/render-hayro/", &[]),
    suite("viewer", "@laminapdf/viewer", "packages/viewer/", &[]),
    suite("tokens", "@laminapdf/tokens", "packages/tokens/", &[]),
    suite("fonts", "@laminapdf/fonts", "packages/fonts/", &[]),
    suite("icons", "@laminapdf/icons", "packages/icons/", &[]),
    suite("ui", "@laminapdf/ui", "packages/ui/", &[]),
    suite("site-kit", "@laminapdf/site-kit", "packages/site-kit/", &[]),
    // The capability page bundles the matrix `cargo xtask capabilities` writes.
    suite("web", "@laminapdf/web", "apps/web/", &["docs/capabilities/matrix.json"]),
    suite(
        "docs",
        "@laminapdf/docs",
        "apps/docs/",
        // The code shown on its pages (the viewer's examples among it), the error pages, the
        // changelog page, and the website's pages, which the shared header links once each exists.
        &[
            "crates/laminapdf/examples/",
            "packages/viewer/examples/",
            "docs/errors/",
            "CHANGELOG.md",
            "apps/site/src/pages/",
        ],
    ),
    suite(
        "site",
        "@laminapdf/site",
        "apps/site/",
        // What its pages read at build time (apps/site/src/data/sources.ts): the first PDF in
        // both languages, the SDK's version, the command-line examples, the install page and the
        // licence policy.
        &[
            "crates/laminapdf/examples/hello.rs",
            "packages/sdk/examples/hello.ts",
            "packages/sdk/package.json",
            "crates/laminapdf-cli/README.md",
            "apps/docs/src/content/docs/install.mdx",
            "deny.toml",
        ],
    ),
];

const fn suite(name: &'static str, package: &'static str, root: &'static str, extra: &'static [&'static str]) -> Suite {
    Suite {
        name,
        package,
        root,
        extra,
    }
}

/// The packages whose builds hold the WebAssembly the `wasm` job builds: a suite that reaches
/// either reads every crate of `wasm::MODULES`.
const WASM_PACKAGES: [&str; 2] = ["packages/wasm/", "packages/render-hayro/"];

/// What every runtime leg reads, whichever suites it runs: the manifests, lockfiles, toolchain and
/// shared configs, the fixtures the suites open, the task runner, and the workflow and its
/// actions. A file at the repository's root counts too ([`counts_everywhere`]).
const SHARED: [&str; 6] = [
    ".cargo/",
    "fixtures/",
    "tools/xtask/",
    ".github/workflows/ci.yml",
    ".github/actions/",
    ".npmrc",
];

/// Whether `path` is a file at the repository's root that isn't docs (`Cargo.toml`,
/// `pnpm-lock.yaml`, or a root config nobody thought to list): it counts for every suite and for
/// the reader.
fn is_root_file(path: &str) -> bool {
    !path.contains('/') && !super::is_docs(path)
}

/// Whether `path` reaches every suite: one of [`SHARED`], or a root file.
fn counts_everywhere(path: &str) -> bool {
    SHARED.iter().any(|s| path.starts_with(s)) || is_root_file(path)
}

/// Whether `path` lies in one of `extra` (docs or not), or isn't docs and lies in `inputs`.
fn hits(path: &str, inputs: &[String], extra: &[&str]) -> bool {
    extra.iter().any(|e| path.starts_with(e))
        || (!super::is_docs(path) && inputs.iter().any(|i| path.starts_with(i.as_str())))
}

/// The crates the WebAssembly is built from: `wasm::MODULES` and what they reach.
fn wasm_crates(graph: &Graph) -> Result<BTreeSet<String>> {
    graph.crate_dirs(crate::wasm::MODULES.iter().map(|module| module.krate))
}

/// The directories and files `suite` reads through the graph: its package and the packages it
/// reaches, and `wasm` (the crates of [`wasm_crates`]) when it reaches the WebAssembly.
fn graph_inputs_with(graph: &Graph, suite: &Suite, wasm: &BTreeSet<String>) -> Vec<String> {
    let packages = graph.package_dirs(suite.root);
    let reads_wasm = packages.iter().any(|p| WASM_PACKAGES.contains(&p.as_str()));
    let crates = wasm.iter().filter(|_| reads_wasm).cloned();
    outermost(packages.into_iter().chain(crates))
}

/// [`graph_inputs_with`], walking the WebAssembly's crates itself.
pub(crate) fn graph_inputs(graph: &Graph, suite: &Suite) -> Result<Vec<String>> {
    Ok(graph_inputs_with(graph, suite, &wasm_crates(graph)?))
}

/// The suites any of `paths` can change, in [`SUITES`]' order.
pub(crate) fn reached(graph: &Graph, paths: &[&str]) -> Result<Vec<&'static str>> {
    let wasm = wasm_crates(graph)?;
    Ok(SUITES
        .iter()
        .filter(|suite| {
            let inputs = graph_inputs_with(graph, suite, &wasm);
            paths
                .iter()
                .any(|&path| counts_everywhere(path) || hits(path, &inputs, suite.extra))
        })
        .map(|suite| suite.name)
        .collect())
}

/// The suite named `name`.
pub(crate) fn by_name(name: &str) -> Result<&'static Suite> {
    let Some(suite) = SUITES.iter().find(|s| s.name == name) else {
        let names: Vec<&str> = SUITES.iter().map(|s| s.name).collect();
        bail!("no runtime suite named {name:?}; the suites are {}", names.join(", "))
    };
    Ok(suite)
}

/// What the reader's deploys and releases depend on (`web-deploy.yml`, `cargo xtask web
/// next-release`).
#[derive(Debug)]
pub(crate) struct WebInputs {
    /// The `web` suite's graph inputs, its deploy workflow, and what every leg reads but the
    /// fixtures and the fast lane's workflow, which only its tests open.
    dirs: Vec<String>,
    /// The `web` suite's own files, docs or not.
    extra: &'static [&'static str],
}

/// The reader's [`WebInputs`].
pub(crate) fn web_inputs(graph: &Graph) -> Result<WebInputs> {
    let web = by_name("web")?;
    let shared = SHARED
        .iter()
        .filter(|&&s| s != "fixtures/" && s != ".github/workflows/ci.yml")
        .map(|&s| s.to_owned());
    let dirs = outermost(
        graph_inputs(graph, web)?
            .into_iter()
            .chain(shared)
            .chain([".github/workflows/web-deploy.yml".to_owned()]),
    );
    Ok(WebInputs { dirs, extra: web.extra })
}

/// Whether any of `paths` can change the reader as deployed: a root file, or one of `inputs`.
pub(crate) fn touches_web(paths: &[&str], inputs: &WebInputs) -> bool {
    paths
        .iter()
        .any(|&path| is_root_file(path) || hits(path, &inputs.dirs, inputs.extra))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::OnceLock;

    use super::super::graph::Graph;
    use super::{SUITES, by_name, graph_inputs, reached, touches_web, web_inputs};

    fn graph() -> &'static Graph {
        static GRAPH: OnceLock<Graph> = OnceLock::new();
        GRAPH.get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(Path::parent)
                .unwrap_or(Path::new("."));
            Graph::load(root).unwrap()
        })
    }

    fn reach(paths: &[&str]) -> Vec<&'static str> {
        reached(graph(), paths).unwrap()
    }

    #[test]
    fn every_suite_should_name_its_own_package_s_directory() {
        for suite in &SUITES {
            let manifest = std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(suite.root)
                    .join("package.json"),
            )
            .unwrap();
            let name = serde_json::from_str::<serde_json::Value>(&manifest).unwrap()["name"].clone();
            assert_eq!(name, suite.package, "{}", suite.name);
        }
    }

    #[test]
    fn a_change_to_the_cli_s_code_should_reach_no_suite() {
        assert_eq!(reach(&["crates/laminapdf-cli/src/main.rs"]), Vec::<&str>::new());
    }

    #[test]
    fn a_change_to_the_website_s_copy_should_reach_the_website_alone() {
        assert_eq!(reach(&["apps/site/src/data/copy.ts"]), ["site"]);
    }

    #[test]
    fn a_change_to_the_website_s_pages_should_reach_the_docs_site_too() {
        assert_eq!(reach(&["apps/site/src/pages/index.astro"]), ["docs", "site"]);
    }

    #[test]
    fn a_change_to_the_ui_components_should_reach_them_and_the_reader() {
        assert_eq!(reach(&["packages/ui/src/components/button.tsx"]), ["ui", "web"]);
    }

    #[test]
    fn a_change_to_a_core_crate_should_reach_every_suite_that_reads_the_webassembly() {
        assert_eq!(
            reach(&["crates/laminapdf-cos/src/lib.rs"]),
            ["examples", "render-hayro", "viewer", "web", "docs"]
        );
    }

    #[test]
    fn an_error_page_should_reach_the_docs_site_though_it_is_docs() {
        assert_eq!(reach(&["docs/errors/wrong-password.md"]), ["docs"]);
    }

    #[test]
    fn a_viewer_example_should_reach_the_docs_site_that_shows_it() {
        assert_eq!(
            reach(&["packages/viewer/examples/show-a-pdf.ts"]),
            ["viewer", "web", "docs"]
        );
    }

    #[test]
    fn the_sdk_s_first_example_should_reach_the_website_s_code_panel() {
        assert!(reach(&["packages/sdk/examples/hello.ts"]).contains(&"site"));
    }

    #[test]
    fn the_capability_matrix_should_reach_the_reader_though_it_is_docs() {
        assert_eq!(reach(&["docs/capabilities/matrix.json"]), ["web"]);
    }

    #[test]
    fn other_docs_should_reach_no_suite() {
        assert_eq!(reach(&["docs/plans/ci-caching.md", "README.md"]), Vec::<&str>::new());
    }

    #[test]
    fn the_lockfile_the_fixtures_the_task_runner_and_a_new_root_file_should_reach_every_suite() {
        for path in [
            "pnpm-lock.yaml",
            "fixtures/corpus/hello.pdf",
            "tools/xtask/src/pdf_runtimes.rs",
            ".github/actions/node-pnpm/action.yml",
            "a-new-root-file.toml",
        ] {
            assert_eq!(reach(&[path]).len(), SUITES.len(), "{path}");
        }
    }

    #[test]
    fn graph_inputs_should_hold_the_webassembly_s_crates_only_where_a_suite_reads_it() {
        let has_cos = |name: &str| {
            graph_inputs(graph(), by_name(name).unwrap())
                .unwrap()
                .iter()
                .any(|i| i == "crates/laminapdf-cos/")
        };
        assert_eq!(
            [has_cos("examples"), has_cos("render-pdfium"), has_cos("site")],
            [true, false, false]
        );
    }

    /// The first `paths:` filter of a workflow: its `- <glob>` lines.
    fn path_filter(workflow: &str) -> Vec<String> {
        let text = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../.github/workflows")
                .join(workflow),
        )
        .unwrap();
        text.lines()
            .skip_while(|l| !l.trim_start().starts_with("paths:"))
            .skip(1)
            .map(str::trim)
            .take_while(|l| l.starts_with('-') || l.starts_with('#'))
            .filter_map(|l| l.strip_prefix("- "))
            .map(|l| l.trim().to_owned())
            .collect()
    }

    /// Whether `filter` (`dir/**` or a file) holds `input` (a directory ending in `/`, or a file).
    fn covered(filter: &[String], input: &str) -> bool {
        filter.iter().any(|glob| match glob.strip_suffix("**") {
            Some(dir) => input.starts_with(dir),
            None => glob == input,
        })
    }

    /// The deploy previews of the docs site and the website build only on their path filters, so
    /// each filter must hold every package, crate and file its suite reads (todo X12).
    #[test]
    fn the_deploy_workflows_path_filters_should_hold_their_suites_inputs() {
        let mut missing = Vec::new();
        for (workflow, name) in [("docs.yml", "docs"), ("site.yml", "site")] {
            let filter = path_filter(workflow);
            let suite = by_name(name).unwrap();
            let inputs = graph_inputs(graph(), suite).unwrap();
            for input in inputs.iter().map(String::as_str).chain(suite.extra.iter().copied()) {
                if !covered(&filter, input) {
                    missing.push(format!("{workflow}: {input}"));
                }
            }
        }
        assert_eq!(missing, Vec::<String>::new());
    }

    #[test]
    fn by_name_should_refuse_an_unknown_suite() {
        assert!(by_name("gallery").is_err());
    }

    #[test]
    fn touches_web_should_follow_the_app_its_sources_build_tasks_and_root_config() {
        let inputs = web_inputs(graph()).unwrap();
        for path in [
            "apps/web/src/app.tsx",
            "packages/viewer/src/index.ts",
            "packages/ui/src/index.ts",
            "crates/laminapdf-wasm/src/lib.rs",
            "crates/laminapdf-cos/src/lib.rs",
            "tools/xtask/src/web.rs",
            "pnpm-lock.yaml",
            "Cargo.lock",
            "package.json",
            ".npmrc",
            ".cargo/config.toml",
            ".github/actions/node-pnpm/action.yml",
            ".github/workflows/web-deploy.yml",
            "a-new-root-file.toml",
            "docs/capabilities/matrix.json",
        ] {
            assert!(touches_web(&[path], &inputs), "{path}");
        }
    }

    #[test]
    fn touches_web_should_skip_docs_and_what_the_reader_never_reads() {
        let inputs = web_inputs(graph()).unwrap();
        assert!(!touches_web(
            &[
                "docs/todos/README.md",
                "README.md",
                "apps/docs/src/content/docs/index.mdx",
                "apps/site/package.json",
                "apps/gallery/src/main.tsx",
                "apps/updates/src/handler.ts",
                "crates/laminapdf-cli/src/main.rs",
                "packages/cli/package.json",
                "fixtures/corpus/hello.pdf",
                "tools/render/Cargo.toml",
                "spikes/pdf-basics/rust/src/lib.rs",
                "fuzz/fuzz_targets/lex.rs",
                ".github/workflows/docs.yml",
                ".github/workflows/ci.yml",
            ],
            &inputs
        ));
    }
}
