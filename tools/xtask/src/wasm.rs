//! `cargo xtask wasm` and `cargo xtask size` (ADR 0002 §2, ADR 0016): the two WASM modules of
//! `@laminapdf/wasm`, their tests in a wasm runtime, and the size budgets CI enforces.
//!
//! `wasm` builds `laminapdf-wasm` twice (base: no features; text: `--features text`) with the
//! `wasm-release` profile, runs `wasm-bindgen --target web` (the CLI pinned to the crate in
//! Cargo.lock), optimises with `wasm-opt` (binaryen pinned below, rustc's feature set passed
//! explicitly) keeping a named copy in `target/wasm/` for size reports, strips it into
//! `packages/wasm/dist/`, and runs the `wasm-bindgen-test` suite (Node) and the package's Node
//! smoke test, then builds `@laminapdf/sdk` over them, runs its tests (Node and Vite) and checks its
//! packaging with publint and `attw --pack` (they need `dist/`, so they run here rather than in
//! `check ts`). `size` measures what `wasm` wrote and fails above
//! `packages/wasm/size-budgets.json`, `@laminapdf/sdk`'s JS (glue + TS API, min+gzip) included.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::doctor::{self, BINARYEN_VERSION};
use crate::util::sha256_hex;
use crate::workspace::Workspace;
use crate::{wasm_glue, wasm_module};

/// The SHA-256 of each binaryen [`BINARYEN_VERSION`] release asset, from the release's `.sha256`
/// files.
const BINARYEN_ASSETS: [(&str, &str); 5] = [
    (
        "arm64-macos",
        "ad66da82ac13f163e424b1643f16c6dfcccc98b5966296b43e52d3cab04f84a8",
    ),
    (
        "x86_64-macos",
        "13a9b90be775c6389ce3d1f879cb8627bea56708ba8c122983941d53a8199b95",
    ),
    (
        "aarch64-linux",
        "89c07ea56faf38d0fbecf36ca8ec0721756716185f265b568e133d427f299bf8",
    ),
    (
        "x86_64-linux",
        "2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489",
    ),
    (
        "x86_64-windows",
        "17a2cbeac6b5693c5fbafab3838d3c65fd9c1eb38b05f5baec6c657e8c84995b",
    ),
];

/// The WASM features rustc enables by default for `wasm32-unknown-unknown` (`rustc --print cfg
/// --target wasm32-unknown-unknown`), passed to binaryen explicitly (ADR 0002 §2). Review this
/// list whenever `rust-toolchain.toml` or [`BINARYEN_VERSION`] changes.
const WASM_FEATURES: [&str; 6] = [
    "--enable-bulk-memory",
    "--enable-mutable-globals",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "--enable-reference-types",
    "--enable-multivalue",
];

/// The `wasm-opt` optimisation level. `-O1` and not `-Oz`: on these modules `-Oz` gives the
/// smallest raw file but a 6–8 % larger gzip (ADR 0016 §5 has the table), and the gzip size is
/// what users download.
const WASM_OPT_LEVEL: &str = "-O1";

const TARGET: &str = "wasm32-unknown-unknown";

/// The most memory each of Lamina PDF's modules may grow to (ADR 0016 §8): 2 GiB, which every
/// engine `@laminapdf/sdk` runs in allows a wasm32 memory. Growth past it fails an allocation,
/// the module traps, and `@laminapdf/sdk` throws `out-of-memory` and starts a fresh instance.
/// Set at link time; [`wasm_module::check`] reads it back from each module.
const MAX_MEMORY: u64 = 2 << 30;

/// The small-memory build's maximum (64 MiB): small, so `@laminapdf/sdk`'s out-of-memory test
/// reaches it with one large file.
const SMALL_MAX_MEMORY: u64 = 64 << 20;

/// One module of `@laminapdf/wasm`.
pub(crate) struct Module {
    /// `base`, `text`, `extract`, `render_hayro` or `tiff`: the files are `lamina_pdf_<name>.{js,d.ts,wasm}`.
    pub(crate) name: &'static str,
    /// The crate built (`laminapdf-wasm` for Lamina PDF's three modules).
    pub(crate) krate: &'static str,
    /// The crate's cdylib file stem in `target/wasm32-unknown-unknown/wasm-release/`.
    artefact: &'static str,
    features: &'static [&'static str],
}

impl Module {
    /// The stem of the module's files: `lamina_pdf_<name>`.
    pub(crate) fn stem(&self) -> String {
        format!("lamina_pdf_{}", self.name)
    }

    /// Whether this is one of Lamina PDF's own modules (`laminapdf-wasm`'s builds), which its
    /// `wasm-bindgen-test` suite and `@laminapdf/sdk` cover.
    fn is_lamina(&self) -> bool {
        self.krate == "laminapdf-wasm"
    }
}

/// The modules, in build order.
pub(crate) const MODULES: [Module; 5] = [
    Module {
        name: "base",
        krate: "laminapdf-wasm",
        artefact: "laminapdf_wasm",
        features: &[],
    },
    Module {
        name: "text",
        krate: "laminapdf-wasm",
        artefact: "laminapdf_wasm",
        features: &["--features", "text"],
    },
    // Base plus text extraction (B9c; ADR 0016 §4): what a reader needs, about half the text
    // module, so a viewer's text layer doesn't need fonts and shaping.
    Module {
        name: "extract",
        krate: "laminapdf-wasm",
        artefact: "laminapdf_wasm",
        features: &["--features", "extract,reflow"],
    },
    // laminapdf-render's hayro adapter (C2c, ADR 0004 §5): its own crate, wrapped by
    // @laminapdf/render-hayro; never loaded by @laminapdf/sdk.
    Module {
        name: "render_hayro",
        krate: "laminapdf-render-wasm",
        artefact: "laminapdf_render_wasm",
        features: &[],
    },
    // laminapdf-render's TIFF writer (E3m): the web reader's document worker loads it when a
    // TIFF export starts, so it is never part of the reader's startup.
    Module {
        name: "tiff",
        krate: "laminapdf-tiff-wasm",
        artefact: "laminapdf_tiff_wasm",
        features: &[],
    },
];

fn dist(ws: &Workspace) -> PathBuf {
    ws.root.join("packages/wasm/dist")
}

fn work(ws: &Workspace, module: &Module) -> PathBuf {
    // xtask's own files stay under `<root>/target` (Workspace::target_dir is cargo's).
    ws.root.join("target/wasm").join(module.name)
}

/// The named, optimised copy of `module` (for twiggy).
fn named(ws: &Workspace, module: &Module) -> PathBuf {
    work(ws, module).join(format!("{}.named.wasm", module.stem()))
}

/// `cargo xtask wasm`: every module into `packages/wasm/dist/`, then the tests that load them.
pub(crate) fn build(ws: &Workspace) -> Result<()> {
    modules(ws)?;
    build_small_memory(ws)?;
    test(ws)
}

/// Every shipped module into `packages/wasm/dist/`, without the tests: what the reader's deploys
/// build from (`cargo xtask web build`, C4d).
pub(crate) fn modules(ws: &Workspace) -> Result<()> {
    doctor::require(ws, &doctor::wasm_bindgen(ws)?)?;
    let wasm_opt = wasm_opt(ws)?;
    let out = dist(ws);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    fs::create_dir_all(&out)?;
    for module in &MODULES {
        build_module(ws, module, &wasm_opt, &out)?;
    }
    Ok(())
}

/// The `cargo` arguments that build `krate`'s cdylib: `cargo rustc` for Lamina PDF's own
/// modules, so the memory cap reaches only their link and every dependency's build stays cached.
fn cargo_args<'a>(krate: &'a str, profile: &'a str, features: &[&'a str], link: Option<&'a str>) -> Vec<&'a str> {
    let mut args = vec![if link.is_some() { "rustc" } else { "build" }, "--locked", "-p", krate];
    args.extend(["--lib", "--profile", profile, "--target", TARGET]);
    args.extend(features);
    if let Some(link) = link {
        args.extend(["--", "-C", link]);
    }
    args
}

/// The base module with a 64 MiB maximum, into `target/wasm/small-memory/` for
/// `@laminapdf/sdk`'s out-of-memory test; never shipped. The shipped module's source, profile and
/// glue: only the link argument differs, so only the final link runs again. (Not the dev profile:
/// with debug assertions, wasm-bindgen's allocator throws a JS error where release aborts.)
fn build_small_memory(ws: &Workspace) -> Result<()> {
    crate::step("WASM module `base` with a 64 MiB maximum (target/wasm/small-memory/, never shipped)");
    let link = format!("link-arg=--max-memory={SMALL_MAX_MEMORY}");
    ws.cargo(&cargo_args("laminapdf-wasm", "wasm-release", &[], Some(&link)))?;
    let out = ws.root.join("target/wasm/small-memory");
    if out.exists() {
        fs::remove_dir_all(&out)?;
    }
    fs::create_dir_all(&out)?;
    let built = ws.target_dir.join(TARGET).join("wasm-release/laminapdf_wasm.wasm");
    let (built_arg, out_arg) = (built.to_string_lossy(), out.to_string_lossy());
    crate::run(
        &ws.root,
        "wasm-bindgen",
        &[
            "--target",
            "web",
            "--experimental-reset-state-function",
            "--out-dir",
            &out_arg,
            "--out-name",
            "lamina_pdf_base",
            &built_arg,
        ],
    )?;
    // The same glue as the shipped module's, so the tests see what users get.
    let glue = out.join("lamina_pdf_base.js");
    fs::write(&glue, wasm_glue::strip_call_checks(&fs::read_to_string(&glue)?)?)?;
    let wasm = fs::read(out.join("lamina_pdf_base_bg.wasm"))?;
    wasm_module::check("small-memory", &wasm, SMALL_MAX_MEMORY)
}

fn build_module(ws: &Workspace, module: &Module, wasm_opt: &Path, out: &Path) -> Result<()> {
    crate::step(&format!("WASM module `{}`", module.name));
    let link = format!("link-arg=--max-memory={MAX_MEMORY}");
    let link = module.is_lamina().then_some(link.as_str());
    ws.cargo(&cargo_args(module.krate, "wasm-release", module.features, link))?;
    let work = work(ws, module);
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    // laminapdf-wasm's artefact has one name for its three modules: copy it before the next build.
    let built = work.join(format!("{}.wasm", module.artefact));
    fs::copy(
        ws.target_dir
            .join(TARGET)
            .join(format!("wasm-release/{}.wasm", module.artefact)),
        &built,
    )
    .with_context(|| format!("copying the {} artefact", module.krate))?;
    let stem = module.stem();
    let built_arg = built.to_string_lossy().into_owned();
    let work_arg = work.to_string_lossy().into_owned();
    let mut bindgen = vec!["--target", "web", "--keep-debug"];
    // `__wbg_reset_state`: `@laminapdf/sdk` replaces an instance with it after a trap and in
    // `reset()`; the finalizers skip objects of a replaced instance (and `wasm_glue` drops the
    // per-call checks, which the SDK makes itself).
    if module.is_lamina() {
        bindgen.push("--experimental-reset-state-function");
    }
    bindgen.extend(["--out-dir", &work_arg, "--out-name", &stem, &built_arg]);
    crate::run(&ws.root, "wasm-bindgen", &bindgen)?;
    let bindgen_wasm = work.join(format!("{stem}_bg.wasm")).to_string_lossy().into_owned();
    let named = named(ws, module).to_string_lossy().into_owned();
    let wasm_opt = wasm_opt.to_string_lossy().into_owned();
    // Optimised with the names kept (-g), then stripped: the names never change the code.
    let mut opt = vec!["-g", WASM_OPT_LEVEL];
    opt.extend(WASM_FEATURES);
    opt.extend([bindgen_wasm.as_str(), "-o", named.as_str()]);
    crate::run(&ws.root, &wasm_opt, &opt)?;
    let shipped = out.join(format!("{stem}.wasm")).to_string_lossy().into_owned();
    let mut strip = vec!["--strip-debug", "--strip-producers"];
    strip.extend(WASM_FEATURES);
    strip.extend([named.as_str(), "-o", shipped.as_str()]);
    crate::run(&ws.root, &wasm_opt, &strip)?;
    if module.is_lamina() {
        wasm_module::check(module.name, &fs::read(&shipped)?, MAX_MEMORY)?;
    }
    // The glue loads `<stem>_bg.wasm` by default; the package ships `<stem>.wasm`.
    let mut js = fs::read_to_string(work.join(format!("{stem}.js")))?;
    if module.is_lamina() {
        js = wasm_glue::strip_call_checks(&js).with_context(|| format!("{stem}.js"))?;
    }
    let from = format!("'{stem}_bg.wasm'");
    if js.matches(&from).count() != 1 {
        bail!("{stem}.js no longer names {from} exactly once: check the wasm-bindgen glue");
    }
    fs::write(
        out.join(format!("{stem}.js")),
        js.replace(&from, &format!("'{stem}.wasm'")),
    )?;
    fs::copy(work.join(format!("{stem}.d.ts")), out.join(format!("{stem}.d.ts")))?;
    Ok(())
}

/// The `wasm-bindgen-test` suite (Node) for each module's features, the package's smoke test
/// (Vitest, Node) over `dist/`, `@laminapdf/render-hayro`'s `RenderBackend` contract over
/// `lamina_pdf_render_hayro` (C2c), then `@laminapdf/sdk`: build, typecheck, tests, publint and attw.
fn test(ws: &Workspace) -> Result<()> {
    crate::step("laminapdf-wasm tests in Node (wasm-bindgen-test-runner)");
    for module in MODULES.iter().filter(|m| m.is_lamina()) {
        let mut args = vec![
            "test",
            "--locked",
            "-p",
            "laminapdf-wasm",
            "--test",
            "boundary",
            "--target",
            TARGET,
        ];
        args.extend(module.features);
        crate::run_env(
            &ws.root,
            "cargo",
            &args,
            &[("CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER", "wasm-bindgen-test-runner")],
        )?;
    }
    ws.pnpm(&["--filter", "@laminapdf/wasm", "run", "test:wasm"])?;
    // Before the SDK: its examples draw pages with @laminapdf/render-hayro, whose `dist/` its
    // typecheck and tests import.
    crate::step(
        "@laminapdf/render-hayro over lamina_pdf_render_hayro: build, typecheck, the RenderBackend contract (Node)",
    );
    ws.pnpm(&["--filter", "@laminapdf/render", "run", "build"])?;
    ws.pnpm(&["--filter", "@laminapdf/render-hayro", "run", "test:wasm"])?;
    crate::step("@laminapdf/sdk over the modules: build, typecheck, Vitest (Node, Vite), publint, attw --pack");
    ws.pnpm(&["--filter", "@laminapdf/sdk", "run", "test:wasm"])?;
    ws.pnpm(&["--filter", "@laminapdf/sdk", "run", "lint:package"])
}

/// A usable `wasm-opt`: on `PATH` at [`BINARYEN_VERSION`], else the release in
/// `target/tools/`, downloaded (and checked against its pinned SHA-256) on first use.
fn wasm_opt(ws: &Workspace) -> Result<PathBuf> {
    let on_path = doctor::wasm_opt();
    if matches!(on_path.probe(ws), doctor::Probe::Ok(_)) {
        return Ok(PathBuf::from("wasm-opt"));
    }
    let dir = ws.root.join("target/tools");
    let exe = if cfg!(windows) { "wasm-opt.exe" } else { "wasm-opt" };
    let local = dir.join(format!("binaryen-version_{BINARYEN_VERSION}/bin")).join(exe);
    if !local.is_file() {
        download_binaryen(&dir)?;
    }
    let output = Command::new(&local)
        .arg("--version")
        .output()
        .with_context(|| format!("running {}", local.display()))?;
    let version = String::from_utf8_lossy(&output.stdout);
    if !on_path.accepts(&version) {
        bail!("{} is not binaryen {BINARYEN_VERSION}: {version}", local.display());
    }
    Ok(local)
}

/// The release asset name for this host.
fn binaryen_asset() -> Result<(&'static str, &'static str)> {
    let host = match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => "arm64-macos",
        ("x86_64", "macos") => "x86_64-macos",
        ("aarch64", "linux") => "aarch64-linux",
        ("x86_64", "linux") => "x86_64-linux",
        ("x86_64", "windows") => "x86_64-windows",
        (arch, os) => bail!("no binaryen {BINARYEN_VERSION} release for {arch}-{os}: put wasm-opt on PATH"),
    };
    BINARYEN_ASSETS
        .iter()
        .find(|(name, _)| *name == host)
        .copied()
        .context("binaryen asset table")
}

fn download_binaryen(dir: &Path) -> Result<()> {
    let (host, sha256) = binaryen_asset()?;
    fs::create_dir_all(dir)?;
    let file = format!("binaryen-version_{BINARYEN_VERSION}-{host}.tar.gz");
    let url = format!("https://github.com/WebAssembly/binaryen/releases/download/version_{BINARYEN_VERSION}/{file}");
    let archive = dir.join(&file);
    let archive_arg = archive.to_string_lossy().into_owned();
    crate::run(dir, "curl", &["-fsSL", "--retry", "3", "-o", &archive_arg, &url])?;
    let found = sha256_hex(&fs::read(&archive)?);
    if found != sha256 {
        fs::remove_file(&archive)?;
        bail!("{file}: SHA-256 {found}, want {sha256}");
    }
    // The bare file name, from inside `dir`: GNU tar (Git Bash on Windows) reads `D:\…` as a
    // remote host.
    crate::run(dir, "tar", &["-xzf", &file])?;
    fs::remove_file(&archive)?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Sizes

/// Raw, gzip (level 9) and brotli (quality 11) sizes of one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sizes {
    pub(crate) raw: u64,
    pub(crate) gzip: u64,
    pub(crate) brotli: u64,
}

fn len(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

impl Sizes {
    fn of(bytes: &[u8]) -> Self {
        Self {
            raw: len(bytes.len()),
            gzip: gzip_len(bytes),
            brotli: brotli_len(bytes),
        }
    }

    fn json(self) -> Value {
        json!({ "raw": self.raw, "gzip": self.gzip, "brotli": self.brotli })
    }
}

/// The size of `bytes` as a gzip file at level 9: the deflate stream plus gzip's 18-byte
/// header and trailer (RFC 1952).
pub(crate) fn gzip_len(bytes: &[u8]) -> u64 {
    len(miniz_oxide::deflate::compress_to_vec(bytes, 9).len() + 18)
}

fn brotli_len(bytes: &[u8]) -> u64 {
    let mut out = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 11,
        lgwin: 22,
        ..Default::default()
    };
    match brotli::BrotliCompress(&mut &bytes[..], &mut out, &params) {
        Ok(_) => len(out.len()),
        Err(_) => u64::MAX,
    }
}

/// One module's files as `cargo xtask size` reads them: the `.wasm` and the minified JS glue.
#[derive(Debug)]
struct ModuleFiles {
    name: &'static str,
    wasm: Vec<u8>,
    glue: Vec<u8>,
}

/// The sizes of each module's files, one thread per module: brotli at quality 11 takes most of a
/// minute on the larger modules.
fn measure_modules(files: &[ModuleFiles]) -> BTreeMap<String, ModuleSizes> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .iter()
            .map(|file| {
                let handle = scope.spawn(|| ModuleSizes {
                    wasm: Sizes::of(&file.wasm),
                    glue: Sizes::of(&file.glue),
                });
                (file.name, handle)
            })
            .collect();
        handles
            .into_iter()
            .map(|(name, handle)| {
                // A measuring thread only panics if the compressor does; pass the panic on.
                let sizes = handle.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic));
                (name.to_owned(), sizes)
            })
            .collect()
    })
}

/// One module's measurements: the `.wasm` and the JS glue minified (esbuild).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleSizes {
    pub(crate) wasm: Sizes,
    pub(crate) glue: Sizes,
}

/// What a module may weigh, gzip bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Budget {
    pub(crate) wasm_gzip: u64,
    pub(crate) glue_gzip: u64,
}

/// Every budget violation in `measured`, one line each (empty: within budget). A module
/// without a budget is a violation too, so a new module can't ship unmeasured.
pub(crate) fn violations(measured: &BTreeMap<String, ModuleSizes>, budgets: &BTreeMap<String, Budget>) -> Vec<String> {
    let mut out = Vec::new();
    for (name, sizes) in measured {
        let Some(budget) = budgets.get(name) else {
            out.push(format!("{name}: no budget in packages/wasm/size-budgets.json"));
            continue;
        };
        if sizes.wasm.gzip > budget.wasm_gzip {
            out.push(format!(
                "{name}: lamina_pdf_{name}.wasm is {} B gzip, over its budget of {} B",
                sizes.wasm.gzip, budget.wasm_gzip
            ));
        }
        if sizes.glue.gzip > budget.glue_gzip {
            out.push(format!(
                "{name}: lamina_pdf_{name}.js (minified) is {} B gzip, over its budget of {} B",
                sizes.glue.gzip, budget.glue_gzip
            ));
        }
    }
    out
}

/// The modules that grew more than 5 % (gzip) over `baseline`: (name, baseline, now).
pub(crate) fn grown(
    measured: &BTreeMap<String, ModuleSizes>,
    baseline: &BTreeMap<String, u64>,
) -> Vec<(String, u64, u64)> {
    measured
        .iter()
        .filter_map(|(name, sizes)| {
            let base = *baseline.get(name)?;
            (sizes.wasm.gzip.saturating_mul(100) > base.saturating_mul(105))
                .then(|| (name.clone(), base, sizes.wasm.gzip))
        })
        .collect()
}

fn budgets_path(ws: &Workspace) -> PathBuf {
    ws.root.join("packages/wasm/size-budgets.json")
}

/// `size-budgets.json`: `{ "budgets": { "<module>": { "wasmGzip", "glueGzip" } }, "baseline":
/// { "<module>": { "wasmGzip", "top": [[bytes, item], …] } } }`.
fn read_budgets(ws: &Workspace) -> Result<(BTreeMap<String, Budget>, Value)> {
    let path = budgets_path(ws);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let json: Value = serde_json::from_str(&text).with_context(|| format!("{} is not JSON", path.display()))?;
    let mut budgets = BTreeMap::new();
    for (name, b) in json.get("budgets").and_then(Value::as_object).into_iter().flatten() {
        let field = |key: &str| {
            b.get(key)
                .and_then(Value::as_u64)
                .with_context(|| format!("{}: budgets.{name}.{key} must be a number", path.display()))
        };
        budgets.insert(
            name.clone(),
            Budget {
                wasm_gzip: field("wasmGzip")?,
                glue_gzip: field("glueGzip")?,
            },
        );
    }
    Ok((budgets, json))
}

/// The JS glue minified by esbuild (the package's devDependency).
fn minified(ws: &Workspace, js: &Path) -> Result<Vec<u8>> {
    ws.require_node_modules()?;
    let output = Command::new(crate::program_path("pnpm"))
        .args([
            "--filter",
            "@laminapdf/wasm",
            "exec",
            "esbuild",
            "--minify",
            "--format=esm",
            "--log-level=warning",
        ])
        .arg(js)
        .current_dir(&ws.root)
        .output()
        .context("running esbuild through pnpm")?;
    if !output.status.success() {
        bail!("esbuild failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output.stdout)
}

/// A `@laminapdf/sdk` entry (`dist/<entry>`) bundled with its imports and minified (esbuild), as a
/// page that imports it downloads it (the `.wasm` files are measured as modules; the worker
/// client's `new URL("./entry.js", …)` is not followed, so the worker file is its own entry).
fn package_js(ws: &Workspace, entry: &str) -> Result<Vec<u8>> {
    let entry = ws.root.join("packages/sdk/dist").join(entry);
    if !entry.exists() {
        bail!("{} is missing: run `cargo xtask wasm` first", entry.display());
    }
    let output = Command::new(crate::program_path("pnpm"))
        .args([
            "--filter",
            "@laminapdf/sdk",
            "exec",
            "esbuild",
            "--bundle",
            "--minify",
            "--format=esm",
            "--platform=browser",
            "--log-level=warning",
        ])
        .arg(&entry)
        .current_dir(&ws.root)
        .output()
        .context("running esbuild through pnpm")?;
    if !output.status.success() {
        bail!("esbuild failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output.stdout)
}

/// The JS budgets of `size-budgets.json`'s `packages` (`{ "<name>": { "jsGzip" } }`).
///
/// # Errors
/// A `packages` entry whose `jsGzip` is missing or not a number, named.
fn package_budgets(file: &Value) -> Result<BTreeMap<String, u64>> {
    let mut out = BTreeMap::new();
    for (name, b) in file.get("packages").and_then(Value::as_object).into_iter().flatten() {
        let gzip = b
            .get("jsGzip")
            .and_then(Value::as_u64)
            .with_context(|| format!("size-budgets.json: packages.{name}.jsGzip must be a number"))?;
        out.insert(name.clone(), gzip);
    }
    Ok(out)
}

/// The violation of `name`'s JS budget, if any (a package without a budget is one too).
pub(crate) fn package_violation(name: &str, gzip: u64, budgets: &BTreeMap<String, u64>) -> Option<String> {
    match budgets.get(name) {
        None => Some(format!(
            "{name}: no budget under packages in packages/wasm/size-budgets.json"
        )),
        Some(&budget) if gzip > budget => Some(format!(
            "{name}: its JS (minified, bundled) is {gzip} B gzip, over its budget of {budget} B"
        )),
        Some(_) => None,
    }
}

/// `@laminapdf/render-pdfium`'s `pdfium.wasm` (npm-pinned, not built here; C2c): its sizes as a
/// table row and a report entry, and the violation of `thirdParty.pdfium.wasmGzip` (C1b's
/// measured gzip size + 5 %), if any.
fn pdfium_size(
    ws: &Workspace,
    file: &Value,
    table: &mut String,
    report: &mut serde_json::Map<String, Value>,
) -> Result<Option<String>> {
    // The pin and the file come from the package, never a copy here: package.json's exact
    // version, and pdfium.wasm resolved as Node resolves it from the package (any pnpm layout).
    let manifest: Value = serde_json::from_str(&fs::read_to_string(
        ws.root.join("packages/render-pdfium/package.json"),
    )?)?;
    let version = manifest
        .pointer("/dependencies/@embedpdf~1pdfium")
        .and_then(Value::as_str)
        .context("packages/render-pdfium/package.json has no @embedpdf/pdfium dependency")?;
    let label = format!("@embedpdf/pdfium@{version}");
    let resolved = Command::new("node")
        .args([
            "-p",
            "require.resolve('@embedpdf/pdfium/pdfium.wasm', { paths: ['packages/render-pdfium'] })",
        ])
        .current_dir(&ws.root)
        .output()
        .context("running node to find pdfium.wasm")?;
    if !resolved.status.success() {
        bail!("pdfium.wasm can't be resolved from packages/render-pdfium: run `pnpm install` first");
    }
    let path = PathBuf::from(String::from_utf8_lossy(&resolved.stdout).trim());
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let sizes = Sizes::of(&bytes);
    let budget = file.pointer("/thirdParty/pdfium/wasmGzip").and_then(Value::as_u64);
    let _ = writeln!(
        table,
        "| pdfium ({label}) | {} | {} | {} | {} | — | — | — |",
        sizes.raw,
        sizes.gzip,
        sizes.brotli,
        budget.map_or("—".to_owned(), |b| b.to_string()),
    );
    report.insert(
        "pdfium".to_owned(),
        json!({ "wasm": sizes.json(), "budget": budget.map(|b| json!({ "wasmGzip": b })) }),
    );
    Ok(match budget {
        None => Some("pdfium: no budget at thirdParty.pdfium.wasmGzip in packages/wasm/size-budgets.json".to_owned()),
        Some(b) if sizes.gzip > b => Some(format!(
            "pdfium: pdfium.wasm is {} B gzip, over its budget of {b} B",
            sizes.gzip
        )),
        Some(_) => None,
    })
}

/// The JS bundles `size` measures: (budget name under `packages`, entry in `packages/sdk/dist/`,
/// table label). ADR 0016 §6: glue + TS API ≤ 20 KB min+gzip; L2a: the worker client, and the
/// worker file it starts.
const PDF_BUNDLES: [(&str, &str, &str); 3] = [
    ("@laminapdf/sdk", "index.js", "JS: glue + API"),
    ("@laminapdf/sdk/worker", "worker/index.js", "JS: worker client"),
    ("@laminapdf/sdk/worker/entry", "worker/entry.js", "JS: the worker file"),
];

/// How many items the baseline keeps and the growth diff compares.
const TOP_ITEMS: usize = 40;

/// `twiggy top` of a module's named build: the `n` largest items, (bytes, name), with names
/// made stable across builds ([`stable_name`]) and every data segment counted as one item
/// (rustc numbers them `.rodata.N` differently from build to build).
fn top_items(module: &Module, ws: &Workspace, n: usize) -> Result<Vec<(u64, String)>> {
    let output = Command::new(crate::program_path("twiggy"))
        .args(["top", "-n", "1000000", "--format", "json"])
        .arg(named(ws, module))
        .output()
        .context("running twiggy")?;
    if !output.status.success() {
        bail!("twiggy failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let items: Value = serde_json::from_slice(&output.stdout).context("twiggy's JSON")?;
    let items = items
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| Some((i.get("shallow_size")?.as_u64()?, stable_name(i.get("name")?.as_str()?))));
    Ok(largest(items, n))
}

/// The `n` largest of `items`, without the names section and twiggy's summary rows, with every
/// data segment summed into one `data segments` item.
fn largest(items: impl Iterator<Item = (u64, String)>, n: usize) -> Vec<(u64, String)> {
    let mut data = 0u64;
    let mut out: Vec<(u64, String)> = Vec::new();
    for (size, name) in items {
        if name.starts_with("data segment") {
            data += size;
        } else if !name.contains("function names")
            && !name.starts_with("custom section")
            && !name.starts_with("Σ")
            && !name.starts_with("...")
        {
            out.push((size, name));
        }
    }
    out.push((data, "data segments".to_owned()));
    out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    out.truncate(n);
    out
}

/// `name` without rustc's crate disambiguators (`laminapdf_build[19234177ad06dfd]::…` →
/// `laminapdf_build::…`), which change with every build and would make every item look new.
fn stable_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(open) = rest.find('[') {
        let (head, tail) = rest.split_at(open);
        out.push_str(head);
        let hash = tail.get(1..).and_then(|t| t.find(']').map(|end| (t, end)));
        match hash {
            Some((t, end)) if end > 0 && t.get(..end).is_some_and(|h| h.bytes().all(|b| b.is_ascii_hexdigit())) => {
                rest = t.get(end + 1..).unwrap_or_default();
            }
            _ => {
                out.push('[');
                rest = tail.get(1..).unwrap_or_default();
            }
        }
    }
    out.push_str(rest);
    out
}

/// A Markdown table of the items whose size changed between `before` and `now`.
fn top_diff(before: &[(u64, String)], now: &[(u64, String)]) -> String {
    let old: BTreeMap<&str, u64> = before.iter().map(|(s, n)| (n.as_str(), *s)).collect();
    let new: BTreeMap<&str, u64> = now.iter().map(|(s, n)| (n.as_str(), *s)).collect();
    let mut rows: Vec<(i128, &str, u64, u64)> = old
        .keys()
        .chain(new.keys())
        .map(|name| {
            let (a, b) = (old.get(name).copied().unwrap_or(0), new.get(name).copied().unwrap_or(0));
            (i128::from(b) - i128::from(a), *name, a, b)
        })
        .filter(|(d, ..)| *d != 0)
        .collect();
    rows.sort_by(|x, y| y.0.abs().cmp(&x.0.abs()).then(x.1.cmp(y.1)));
    rows.dedup_by(|x, y| x.1 == y.1);
    let mut out = String::from("| Δ bytes | before | now | item |\n| ---: | ---: | ---: | --- |\n");
    for (d, name, a, b) in rows.into_iter().take(25) {
        let name: String = name.chars().take(120).collect();
        let _ = writeln!(out, "| {d:+} | {a} | {b} | `{}` |", crate::util::md_cell(&name));
    }
    out
}

/// `cargo xtask size [--update-baseline]`: measures `packages/wasm/dist/`, writes
/// `target/size-report.json`, fails above the budgets, and reports the twiggy top-items diff of
/// any module that grew more than 5 % over the baseline (to the job summary in CI).
pub(crate) fn size(ws: &Workspace, update_baseline: bool) -> Result<()> {
    let (budgets, mut file) = read_budgets(ws)?;
    let out = dist(ws);
    let mut files = Vec::new();
    for module in &MODULES {
        let stem = module.stem();
        let wasm = out.join(format!("{stem}.wasm"));
        let wasm =
            fs::read(&wasm).with_context(|| format!("reading {}: run `cargo xtask wasm` first", wasm.display()))?;
        let glue = minified(ws, &out.join(format!("{stem}.js")))?;
        files.push(ModuleFiles {
            name: module.name,
            wasm,
            glue,
        });
    }
    let measured = measure_modules(&files);
    let baseline: BTreeMap<String, u64> = MODULES
        .iter()
        .filter_map(|m| {
            Some((
                m.name.to_owned(),
                file.pointer(&format!("/baseline/{}/wasmGzip", m.name))?.as_u64()?,
            ))
        })
        .collect();

    let mut table = String::from(
        "| module | .wasm raw | .wasm gzip | .wasm brotli | budget (gzip) | glue min | glue min+gzip | budget |\n\
         | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    let mut report = serde_json::Map::new();
    for (name, s) in &measured {
        let budget = budgets.get(name);
        let _ = writeln!(
            table,
            "| {name} | {} | {} | {} | {} | {} | {} | {} |",
            s.wasm.raw,
            s.wasm.gzip,
            s.wasm.brotli,
            budget.map_or("—".to_owned(), |b| b.wasm_gzip.to_string()),
            s.glue.raw,
            s.glue.gzip,
            budget.map_or("—".to_owned(), |b| b.glue_gzip.to_string()),
        );
        report.insert(
            name.clone(),
            json!({
                "wasm": s.wasm.json(),
                "glueMinified": s.glue.json(),
                "budget": budget.map(|b| json!({ "wasmGzip": b.wasm_gzip, "glueGzip": b.glue_gzip })),
                "baselineWasmGzip": baseline.get(name),
            }),
        );
    }
    let package_budgets = package_budgets(&file)?;
    let mut bundles = Vec::new();
    for (name, entry, label) in PDF_BUNDLES {
        let js = Sizes::of(&package_js(ws, entry)?);
        let budget = package_budgets.get(name).copied();
        let _ = writeln!(
            table,
            "| {name} ({label}) | — | — | — | — | {} | {} | {} |",
            js.raw,
            js.gzip,
            budget.map_or("—".to_owned(), |b| b.to_string()),
        );
        report.insert(
            name.to_owned(),
            json!({ "jsMinified": js.json(), "budget": budget.map(|b| json!({ "jsGzip": b })) }),
        );
        bundles.push((name, js.gzip));
    }
    let pdfium = pdfium_size(ws, &file, &mut table, &mut report)?;
    println!("{table}");
    fs::create_dir_all(ws.root.join("target"))?;
    let report_path = ws.root.join("target/size-report.json");
    fs::write(
        &report_path,
        serde_json::to_string_pretty(&Value::Object(report))? + "\n",
    )?;
    println!("wrote {}", report_path.display());
    crate::util::summary(&format!("### WASM sizes\n\n{table}\n"))?;

    report_growth(ws, &measured, &baseline, &file)?;
    if update_baseline {
        write_baseline(ws, &measured, &mut file)?;
    }

    let mut problems = violations(&measured, &budgets);
    problems.extend(
        bundles
            .iter()
            .filter_map(|&(name, gzip)| package_violation(name, gzip, &package_budgets)),
    );
    problems.extend(pdfium);
    if problems.is_empty() {
        crate::step("WASM sizes are within budget");
        Ok(())
    } else {
        bail!("over the size budget (ADR 0016 §6):\n  {}", problems.join("\n  "))
    }
}

/// Prints (and posts to the job summary) the twiggy top-items diff of every module that grew
/// more than 5 % over its baseline.
fn report_growth(
    ws: &Workspace,
    measured: &BTreeMap<String, ModuleSizes>,
    baseline: &BTreeMap<String, u64>,
    file: &Value,
) -> Result<()> {
    for (name, before, now) in grown(measured, baseline) {
        let module = MODULES.iter().find(|m| m.name == name).context("module")?;
        let message = format!("`{name}` grew from {before} B to {now} B gzip (more than 5 %).");
        match top_items(module, ws, TOP_ITEMS) {
            Ok(items) => {
                let old: Vec<(u64, String)> = file
                    .pointer(&format!("/baseline/{name}/top"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|r| Some((r.get(0)?.as_u64()?, r.get(1)?.as_str()?.to_owned())))
                    .collect();
                let diff = top_diff(&old, &items);
                println!("{message} twiggy top items, baseline → now:\n{diff}");
                crate::util::summary(&format!(
                    "#### {message}\n\ntwiggy top items, baseline → now:\n\n{diff}\n"
                ))?;
            }
            Err(e) => println!("{message} (no twiggy diff: {e:#})"),
        }
    }

    Ok(())
}

/// Records the current sizes and top items as the baseline in `size-budgets.json`.
fn write_baseline(ws: &Workspace, measured: &BTreeMap<String, ModuleSizes>, file: &mut Value) -> Result<()> {
    let mut base = serde_json::Map::new();
    for module in &MODULES {
        let top = top_items(module, ws, TOP_ITEMS)?;
        let gzip = measured.get(module.name).map_or(0, |s| s.wasm.gzip);
        base.insert(module.name.to_owned(), json!({ "wasmGzip": gzip, "top": top }));
    }
    if let Some(obj) = file.as_object_mut() {
        obj.insert("baseline".to_owned(), Value::Object(base));
    }
    let path = budgets_path(ws);
    fs::write(&path, serde_json::to_string_pretty(&file)? + "\n")?;
    ws.pnpm(&["exec", "biome", "format", "--write", &path.to_string_lossy()])?;
    println!("updated the baseline in {}", budgets_path(ws).display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        Budget, ModuleFiles, ModuleSizes, Sizes, grown, largest, measure_modules, package_budgets, package_violation,
        stable_name, top_diff, violations,
    };

    #[test]
    fn measure_modules_should_measure_each_module_like_sizes_of() {
        let file = |name, wasm: &[u8], glue: &[u8]| ModuleFiles {
            name,
            wasm: wasm.to_vec(),
            glue: glue.to_vec(),
        };
        let files = [file("a", &b"wasm a".repeat(50), b"glue a"), file("b", b"b", b"")];
        let expected: BTreeMap<String, ModuleSizes> = files
            .iter()
            .map(|f| {
                let sizes = ModuleSizes {
                    wasm: Sizes::of(&f.wasm),
                    glue: Sizes::of(&f.glue),
                };
                (f.name.to_owned(), sizes)
            })
            .collect();
        assert_eq!(measure_modules(&files), expected);
    }

    #[test]
    fn largest_sums_the_data_segments_into_one_item() {
        let items = [
            (5, ".rodata.12"),
            (7, "f"),
            (3, ".rodata.3"),
            (100, "\"function names\" subsection"),
        ]
        .map(|(s, n)| {
            (
                s,
                if n.starts_with('.') {
                    format!("data segment \"{n}\"")
                } else {
                    n.to_owned()
                },
            )
        });
        assert_eq!(
            largest(items.into_iter(), 5),
            [(8, "data segments".to_owned()), (7, "f".to_owned())]
        );
    }

    #[test]
    fn stable_name_drops_crate_disambiguators_only() {
        assert_eq!(
            stable_name("<laminapdf_build[19234177ad06dfd]::page::PageBuilder>::write [u8; 4] x[]"),
            "<laminapdf_build::page::PageBuilder>::write [u8; 4] x[]"
        );
    }

    fn sizes(gzip: u64, glue: u64) -> ModuleSizes {
        ModuleSizes {
            wasm: Sizes {
                raw: gzip * 3,
                gzip,
                brotli: gzip,
            },
            glue: Sizes {
                raw: glue * 3,
                gzip: glue,
                brotli: glue,
            },
        }
    }

    fn budgets() -> BTreeMap<String, Budget> {
        BTreeMap::from([(
            "base".to_owned(),
            Budget {
                wasm_gzip: 1000,
                glue_gzip: 100,
            },
        )])
    }

    #[test]
    fn a_module_within_budget_passes() {
        let measured = BTreeMap::from([("base".to_owned(), sizes(1000, 100))]);
        assert_eq!(violations(&measured, &budgets()), Vec::<String>::new());
    }

    #[test]
    fn a_bloated_module_fails() {
        let measured = BTreeMap::from([("base".to_owned(), sizes(1001, 100))]);
        assert_eq!(violations(&measured, &budgets()).len(), 1);
    }

    #[test]
    fn bloated_glue_fails() {
        let measured = BTreeMap::from([("base".to_owned(), sizes(900, 101))]);
        assert_eq!(violations(&measured, &budgets()).len(), 1);
    }

    #[test]
    fn a_module_without_a_budget_fails() {
        let measured = BTreeMap::from([("extract".to_owned(), sizes(1, 1))]);
        assert_eq!(violations(&measured, &budgets()).len(), 1);
    }

    #[test]
    fn a_package_over_its_js_budget_fails() {
        let budgets = BTreeMap::from([("@laminapdf/sdk".to_owned(), 20_000)]);
        assert!(package_violation("@laminapdf/sdk", 20_001, &budgets).is_some());
    }

    #[test]
    fn a_package_within_its_js_budget_passes() {
        let budgets = BTreeMap::from([("@laminapdf/sdk".to_owned(), 20_000)]);
        assert_eq!(package_violation("@laminapdf/sdk", 20_000, &budgets), None);
    }

    #[test]
    fn a_malformed_js_budget_is_an_error_naming_the_field() {
        let file = serde_json::json!({ "packages": { "@laminapdf/sdk": { "jsGzip": "20 KB" } } });
        let message = package_budgets(&file).map_or_else(|e| e.to_string(), |_| String::new());
        assert!(message.contains("packages.@laminapdf/sdk.jsGzip"), "{message}");
    }

    #[test]
    fn a_package_without_a_js_budget_fails() {
        assert!(package_violation("@laminapdf/sdk", 1, &BTreeMap::new()).is_some());
    }

    #[test]
    fn growth_over_five_percent_is_reported() {
        let measured = BTreeMap::from([("base".to_owned(), sizes(1051, 1)), ("text".to_owned(), sizes(1050, 1))]);
        let baseline = BTreeMap::from([("base".to_owned(), 1000), ("text".to_owned(), 1000)]);
        assert_eq!(grown(&measured, &baseline), [("base".to_owned(), 1000, 1051)]);
    }

    #[test]
    fn top_diff_lists_changed_items_largest_change_first() {
        let before = [(100, "a".to_owned()), (50, "b".to_owned())];
        let now = [(100, "a".to_owned()), (80, "b".to_owned()), (200, "c".to_owned())];
        let table = top_diff(&before, &now);
        let rows: Vec<&str> = table.lines().skip(2).collect();
        assert_eq!(rows, ["| +200 | 0 | 200 | `c` |", "| +30 | 50 | 80 | `b` |"]);
    }
}
