//! `cargo xtask wasm` and `cargo xtask size` (ADR 0002 §2, ADR 0016): the two WASM modules of
//! `@lamina/wasm`, their tests in a wasm runtime, and the size budgets CI enforces.
//!
//! `wasm` builds `lamina-wasm` twice (base: no features; text: `--features text`) with the
//! `wasm-release` profile, runs `wasm-bindgen --target web` (the CLI pinned to the crate in
//! Cargo.lock), optimises with `wasm-opt` (binaryen pinned below, rustc's feature set passed
//! explicitly) keeping a named copy in `target/wasm/` for size reports, strips it into
//! `packages/wasm/dist/`, and runs the `wasm-bindgen-test` suite (Node) and the package's Node
//! smoke test, then builds `@lamina/pdf` over them, runs its tests (Node and Vite) and checks its
//! packaging with publint and `attw --pack` (they need `dist/`, so they run here rather than in
//! `check ts`). `size` measures what `wasm` wrote and fails above
//! `packages/wasm/size-budgets.json`, `@lamina/pdf`'s JS (glue + TS API, min+gzip) included.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::doctor::{self, BINARYEN_VERSION};
use crate::workspace::Workspace;

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

/// One module of `@lamina/wasm`.
pub(crate) struct Module {
    /// `base`, `text` or `extract`: the files are `lamina_<name>.{js,d.ts,wasm}`.
    pub(crate) name: &'static str,
    features: &'static [&'static str],
}

/// The modules, in build order.
pub(crate) const MODULES: [Module; 3] = [
    Module {
        name: "base",
        features: &[],
    },
    Module {
        name: "text",
        features: &["--features", "text"],
    },
    // Base plus text extraction (B9c; ADR 0016 §4): what a reader needs, about half the text
    // module, so a viewer's text layer doesn't need fonts and shaping.
    Module {
        name: "extract",
        features: &["--features", "extract"],
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
    work(ws, module).join(format!("lamina_{}.named.wasm", module.name))
}

/// `cargo xtask wasm`: every module into `packages/wasm/dist/`, then the tests that load them.
pub(crate) fn build(ws: &Workspace) -> Result<()> {
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
    test(ws)
}

fn build_module(ws: &Workspace, module: &Module, wasm_opt: &Path, out: &Path) -> Result<()> {
    crate::step(&format!("WASM module `{}`", module.name));
    let mut args = vec![
        "build",
        "--locked",
        "-p",
        "lamina-wasm",
        "--lib",
        "--profile",
        "wasm-release",
        "--target",
        TARGET,
    ];
    args.extend(module.features);
    ws.cargo(&args)?;
    let work = work(ws, module);
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    // The cargo artefact has one name for both modules: copy it before the next build.
    let built = work.join("lamina_wasm.wasm");
    fs::copy(ws.target_dir.join(TARGET).join("wasm-release/lamina_wasm.wasm"), &built)
        .context("copying the lamina-wasm artefact")?;
    let stem = format!("lamina_{}", module.name);
    let built_arg = built.to_string_lossy().into_owned();
    let work_arg = work.to_string_lossy().into_owned();
    crate::run(
        &ws.root,
        "wasm-bindgen",
        &[
            "--target",
            "web",
            "--keep-debug",
            "--out-dir",
            &work_arg,
            "--out-name",
            &stem,
            &built_arg,
        ],
    )?;
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
    // The glue loads `<stem>_bg.wasm` by default; the package ships `<stem>.wasm`.
    let js = fs::read_to_string(work.join(format!("{stem}.js")))?;
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
/// (Vitest, Node) over `dist/`, then `@lamina/pdf`: build, typecheck, tests, publint and attw.
fn test(ws: &Workspace) -> Result<()> {
    crate::step("lamina-wasm tests in Node (wasm-bindgen-test-runner)");
    for module in &MODULES {
        let mut args = vec![
            "test",
            "--locked",
            "-p",
            "lamina-wasm",
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
    ws.pnpm(&["--filter", "@lamina/wasm", "run", "test:wasm"])?;
    crate::step("@lamina/pdf over the modules: build, typecheck, Vitest (Node, Vite), publint, attw --pack");
    ws.pnpm(&["--filter", "@lamina/pdf", "run", "test:wasm"])?;
    ws.pnpm(&["--filter", "@lamina/pdf", "run", "lint:package"])
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
    let found = hex(&Sha256::digest(fs::read(&archive)?));
    if found != sha256 {
        fs::remove_file(&archive)?;
        bail!("{file}: SHA-256 {found}, want {sha256}");
    }
    crate::run(dir, "tar", &["-xzf", &archive_arg])?;
    fs::remove_file(&archive)?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
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
fn gzip_len(bytes: &[u8]) -> u64 {
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
                "{name}: lamina_{name}.wasm is {} B gzip, over its budget of {} B",
                sizes.wasm.gzip, budget.wasm_gzip
            ));
        }
        if sizes.glue.gzip > budget.glue_gzip {
            out.push(format!(
                "{name}: lamina_{name}.js (minified) is {} B gzip, over its budget of {} B",
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
            "@lamina/wasm",
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

/// `@lamina/pdf`'s browser entry bundled with its glue and minified (esbuild), as a page that
/// imports the whole API downloads it (the `.wasm` files are measured as modules).
fn package_js(ws: &Workspace) -> Result<Vec<u8>> {
    let entry = ws.root.join("packages/pdf/dist/index.js");
    if !entry.exists() {
        bail!("{} is missing: run `cargo xtask wasm` first", entry.display());
    }
    let output = Command::new(crate::program_path("pnpm"))
        .args([
            "--filter",
            "@lamina/pdf",
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
            "{name}: its JS (glue + API, minified) is {gzip} B gzip, over its budget of {budget} B"
        )),
        Some(_) => None,
    }
}

/// The package whose JS `size` measures (ADR 0016 §6: glue + TS API ≤ 20 KB min+gzip).
const PDF_PACKAGE: &str = "@lamina/pdf";

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

/// `name` without rustc's crate disambiguators (`lamina_build[19234177ad06dfd]::…` →
/// `lamina_build::…`), which change with every build and would make every item look new.
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
    let mut measured = BTreeMap::new();
    for module in &MODULES {
        let stem = format!("lamina_{}", module.name);
        let wasm = out.join(format!("{stem}.wasm"));
        let wasm =
            fs::read(&wasm).with_context(|| format!("reading {}: run `cargo xtask wasm` first", wasm.display()))?;
        let glue = minified(ws, &out.join(format!("{stem}.js")))?;
        measured.insert(
            module.name.to_owned(),
            ModuleSizes {
                wasm: Sizes::of(&wasm),
                glue: Sizes::of(&glue),
            },
        );
    }
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
    let pdf = Sizes::of(&package_js(ws)?);
    let package_budgets = package_budgets(&file)?;
    let pdf_budget = package_budgets.get(PDF_PACKAGE).copied();
    let _ = writeln!(
        table,
        "| {PDF_PACKAGE} (JS: glue + API) | — | — | — | — | {} | {} | {} |",
        pdf.raw,
        pdf.gzip,
        pdf_budget.map_or("—".to_owned(), |b| b.to_string()),
    );
    report.insert(
        PDF_PACKAGE.to_owned(),
        json!({ "jsMinified": pdf.json(), "budget": pdf_budget.map(|b| json!({ "jsGzip": b })) }),
    );
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
    problems.extend(package_violation(PDF_PACKAGE, pdf.gzip, &package_budgets));
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
        Budget, ModuleSizes, Sizes, grown, largest, package_budgets, package_violation, stable_name, top_diff,
        violations,
    };

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
            stable_name("<lamina_build[19234177ad06dfd]::page::PageBuilder>::write [u8; 4] x[]"),
            "<lamina_build::page::PageBuilder>::write [u8; 4] x[]"
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
        let budgets = BTreeMap::from([("@lamina/pdf".to_owned(), 20_000)]);
        assert!(package_violation("@lamina/pdf", 20_001, &budgets).is_some());
    }

    #[test]
    fn a_package_within_its_js_budget_passes() {
        let budgets = BTreeMap::from([("@lamina/pdf".to_owned(), 20_000)]);
        assert_eq!(package_violation("@lamina/pdf", 20_000, &budgets), None);
    }

    #[test]
    fn a_malformed_js_budget_is_an_error_naming_the_field() {
        let file = serde_json::json!({ "packages": { "@lamina/pdf": { "jsGzip": "20 KB" } } });
        let message = package_budgets(&file).map_or_else(|e| e.to_string(), |_| String::new());
        assert!(message.contains("packages.@lamina/pdf.jsGzip"), "{message}");
    }

    #[test]
    fn a_package_without_a_js_budget_fails() {
        assert!(package_violation("@lamina/pdf", 1, &BTreeMap::new()).is_some());
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
