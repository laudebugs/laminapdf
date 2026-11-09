//! `cargo xtask web build`: the reader (`apps/web`, todo C4b) built from a fresh checkout into
//! `apps/web/dist/`, as its deploys need it (todo C4d, `.github/workflows/web-deploy.yml`). It
//! builds the WebAssembly modules without running their tests (the fast lane runs those), then
//! every package the app imports, then the app.

use anyhow::{Result, bail};

use crate::{wasm, workspace::Workspace};

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
