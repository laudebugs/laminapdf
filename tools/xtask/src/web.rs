//! `cargo xtask web build`: the reader (`apps/web`, todo C4b) built from a fresh checkout into
//! `apps/web/dist/`, as its deploys need it (todo C4d, `.github/workflows/web-deploy.yml`). It
//! builds the WebAssembly modules without running their tests (the fast lane runs those), then
//! every package the app imports, then the app.
//!
//! `cargo xtask web size`: the app shell's JavaScript against its budget (todo C7c, web-app §4):
//! every script the built `index.html` loads at startup (its scripts and module preloads),
//! minified by Vite and gzipped here, at most `apps/web/size-budget.json`'s `startupJsGzip`.
//! WebAssembly has its own budgets (`cargo xtask size`). Workers and lazy chunks are listed with
//! their sizes but not counted: web-app §4's budget is the app shell's, and the document worker
//! is engine code that runs off the main thread, beside the WebAssembly it drives.

use std::fs;

use anyhow::{Context, Result, bail};

use crate::{wasm, workspace::Workspace};

/// `cargo xtask web <command>`: the reader's build, size and release tasks.
pub(crate) fn run(ws: &Workspace, args: &[String]) -> Result<()> {
    match args {
        [cmd] if cmd == "build" => build(ws),
        [cmd] if cmd == "docs-runner" => docs_runner(ws),
        [cmd] if cmd == "size" => size(ws),
        [cmd, rest @ ..] if cmd == "next-release" => {
            let args = crate::web_release::parse_next_release(rest)?;
            crate::web_release::next_release(&ws.root, &args)
        }
        [cmd, tag] if cmd == "check-tag" => crate::web_release::check_tag(&ws.root, tag),
        _ => bail!(
            "usage: cargo xtask web build|docs-runner|size|next-release [--commit <rev>] [--since <tag>] [--notes <file>]|check-tag <tag>"
        ),
    }
}

/// `cargo xtask web build`.
pub(crate) fn build(ws: &Workspace) -> Result<()> {
    wasm::modules(ws)?;
    ws.pnpm(&["--filter", "@laminapdf/sdk", "run", "build:wasm"])?;
    packages(ws)?;
    ws.pnpm(&["--filter", "@laminapdf/web", "run", "build"])?;
    let index = ws.root.join("apps/web/dist/index.html");
    if !index.is_file() {
        bail!("the build wrote no {}", index.display());
    }
    Ok(())
}

/// `cargo xtask web docs-runner`: what the docs site's example runner (todo B7e,
/// `apps/docs/scripts/build-runner.mjs`) builds from: the WebAssembly modules, `@laminapdf/sdk`
/// and the hayro render packages. The docs workflow runs it before the docs build, so the deployed
/// pages get "Edit and run"; without it they show the examples without the button.
pub(crate) fn docs_runner(ws: &Workspace) -> Result<()> {
    wasm::modules(ws)?;
    ws.pnpm(&["--filter", "@laminapdf/sdk", "run", "build:wasm"])?;
    ws.pnpm(&["--filter", "@laminapdf/render", "run", "build"])?;
    ws.pnpm(&["--filter", "@laminapdf/render-hayro", "run", "build:wasm"])
}

/// `cargo xtask desktop-frontend`: what the desktop app's frontend imports through built `dist/`s:
/// the WebAssembly modules without their tests, `@laminapdf/sdk`, then [`packages`],
/// since its view is `@laminapdf/viewer`'s. The full lane's `desktop` job runs it before the app's
/// `typecheck:wasm` and `desktop:build`.
pub(crate) fn desktop_frontend(ws: &Workspace) -> Result<()> {
    wasm::modules(ws)?;
    ws.pnpm(&["--filter", "@laminapdf/sdk", "run", "build:wasm"])?;
    packages(ws)
}

/// Every package the reader imports through its built `dist/`, over the WebAssembly modules and
/// `@laminapdf/sdk` already built: the viewer with its render backends (`@laminapdf/render`,
/// `-pdfium`, `-hayro`), and `@laminapdf/ui` with the tokens, icons and fonts it imports. The
/// browser suites of `cargo xtask pdf-runtimes` run over the same builds.
pub(crate) fn packages(ws: &Workspace) -> Result<()> {
    ws.pnpm(&["--filter", "@laminapdf/viewer", "run", "build:wasm"])?;
    ws.pnpm(&[
        "--filter",
        "@laminapdf/tokens",
        "--filter",
        "@laminapdf/icons",
        "--filter",
        "@laminapdf/fonts",
        "--filter",
        "@laminapdf/ui",
        "run",
        "build",
    ])
}

/// Where the app shell's budget is kept.
const BUDGET_FILE: &str = "apps/web/size-budget.json";

/// `cargo xtask web size`, over the `apps/web/dist/` that `web build` wrote: a table of every
/// script (startup ones marked), and a failure when the startup scripts' gzipped total is over
/// the budget.
pub(crate) fn size(ws: &Workspace) -> Result<()> {
    let dist = ws.root.join("apps/web/dist");
    let html = fs::read_to_string(dist.join("index.html")).with_context(|| {
        format!(
            "reading {}; run `cargo xtask web build` first",
            dist.join("index.html").display()
        )
    })?;
    let budget_text =
        fs::read_to_string(ws.root.join(BUDGET_FILE)).with_context(|| format!("reading {BUDGET_FILE}"))?;
    let budget_json = serde_json::from_str::<serde_json::Value>(&budget_text)?;
    let budget = budget_json
        .get("startupJsGzip")
        .and_then(serde_json::Value::as_u64)
        .with_context(|| format!("{BUDGET_FILE} has no startupJsGzip"))?;
    let startup = startup_scripts(&html);
    if startup.is_empty() {
        bail!("{} loads no script", dist.join("index.html").display());
    }
    let mut total = 0;
    println!("{:<48} {:>10}", "startup script", "gzip bytes");
    for path in &startup {
        let file = dist.join(path.trim_start_matches('/'));
        let bytes = fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
        let gzip = wasm::gzip_len(&bytes);
        total += gzip;
        println!("{path:<48} {gzip:>10}");
    }
    println!("{:<48} {total:>10} (budget {budget})", "total");
    // The other scripts: workers (the document worker starts with the app, but runs off the main
    // thread, as engine code beside the WebAssembly) and lazy chunks. Listed, never counted.
    for (name, gzip) in other_scripts(&dist, &startup)? {
        println!("{name:<48} {gzip:>10} (not in the app shell)");
    }
    let target = budget_json.get("target").and_then(serde_json::Value::as_u64);
    let waiver = budget_json.get("waiver").and_then(serde_json::Value::as_str);
    if let (Some(target), Some(waiver)) = (target, waiver)
        && budget > target
    {
        println!("the budget is waived above web-app §4's target of {target}, until {waiver} closes");
    }
    if total > budget {
        bail!(
            "the app shell's startup JavaScript is {total} bytes gzipped, over its budget of {budget} ({BUDGET_FILE})"
        );
    }
    Ok(())
}

/// Every `.js` file under `dist/assets/` that isn't one of `startup`, with its gzipped size, by name.
fn other_scripts(dist: &std::path::Path, startup: &[String]) -> Result<Vec<(String, u64)>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dist.join("assets")).context("reading dist/assets")? {
        let path = entry?.path();
        let name = format!(
            "/assets/{}",
            path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
        );
        if path.extension().is_some_and(|e| e == "js") && !startup.contains(&name) {
            found.push((name, wasm::gzip_len(&fs::read(&path)?)));
        }
    }
    found.sort();
    Ok(found)
}

/// The scripts `html` loads at startup, in order: each `<script src>` and each
/// `<link rel="modulepreload" href>`. Lazy chunks, workers and WebAssembly are not among them.
fn startup_scripts(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        let tag_and_after = &rest[start..];
        let Some(end) = tag_and_after.find('>') else { break };
        let tag = &tag_and_after[..end];
        if tag.starts_with("<script") {
            found.extend(attribute(tag, "src"));
        } else if tag.starts_with("<link") && attribute(tag, "rel").as_deref() == Some("modulepreload") {
            found.extend(attribute(tag, "href"));
        }
        rest = &tag_and_after[end..];
    }
    found
}

/// The value of `name` in the start tag `tag`, quoted with `"`.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let at = tag.find(&key)? + key.len();
    let value = &tag[at..];
    Some(value[..value.find('"')?].to_owned())
}

#[cfg(test)]
mod tests {
    use super::startup_scripts;

    #[test]
    fn startup_scripts_are_the_page_scripts_and_module_preloads_only() {
        let html = r#"<!doctype html><html><head>
<link rel="icon" href="/favicon.svg">
<script src="/appearance.js"></script>
<script type="module" crossorigin src="/assets/index-a1.js"></script>
<link rel="modulepreload" crossorigin href="/assets/react-b2.js">
<link rel="stylesheet" crossorigin href="/assets/index-c3.css">
</head><body><div id="app"></div></body></html>"#;
        assert_eq!(
            startup_scripts(html),
            ["/appearance.js", "/assets/index-a1.js", "/assets/react-b2.js"]
        );
    }

    #[test]
    fn a_page_without_scripts_loads_none() {
        assert_eq!(
            startup_scripts("<!doctype html><p>no scripts</p>"),
            Vec::<String>::new()
        );
    }
}
