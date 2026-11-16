//! `cargo xtask crash symbolicate <report.json> [--symbols <dir>] [--fetch]`: a desktop crash report's
//! native frames with function names and source lines, for maintainers (todo D7d;
//! docs/guidelines/desktop-release.md §7).
//!
//! The report's `native` object, which the app fills from a minidump it processed on the user's
//! computer, lists the crashing thread's frames as a module and an offset from the module's base,
//! and the modules with their Breakpad debug IDs. The symbols of each module a frame names are read
//! from `<debug file>/<debug ID>/<debug file without .pdb>.sym`, the layout in which the release
//! workflow archives them in the private R2 bucket `lamina-pdf-symbols`: from `--symbols <dir>`
//! when it is given, else from the cache `target/crash-symbols/`, and with `--fetch` only, fetched
//! into that cache by Wrangler (`wrangler r2 object get … --remote`) with the maintainer's own
//! Cloudflare login, so xtask holds no secret and reaches the bucket only when asked. A frame whose
//! module has no symbol file prints its module and offset. A report comes from a user's computer,
//! so every string from it is printed without control characters (no terminal escapes).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use breakpad_symbols::{FrameSymbolizer, SimpleModule, SymbolFile};
use serde::Deserialize;

const USAGE: &str = "usage: cargo xtask crash symbolicate <report.json> [--symbols <dir>] [--fetch]";
/// The private bucket the desktop release workflow archives the symbols in (`archive-symbols`).
const BUCKET: &str = "lamina-pdf-symbols";
/// The main binary's name in a bundle (`productName` in tauri.conf.json) and the Cargo name its
/// symbols are archived under.
const PRODUCT_NAME: &str = "Lamina PDF";
const APP_BINARY: &str = "laminapdf-desktop";

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
struct Options {
    report: PathBuf,
    symbols: Option<PathBuf>,
    /// Whether missing symbol files may be fetched from the bucket.
    fetch: bool,
}

fn parse(args: &[String]) -> Result<Options> {
    let [cmd, report, rest @ ..] = args else { bail!(USAGE) };
    if cmd != "symbolicate" {
        bail!(USAGE);
    }
    let mut options = Options {
        report: report.into(),
        symbols: None,
        fetch: false,
    };
    let mut rest = rest.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--fetch" if !options.fetch => options.fetch = true,
            "--symbols" if options.symbols.is_none() => {
                options.symbols = Some(rest.next().context(USAGE)?.into());
            }
            _ => bail!(USAGE),
        }
    }
    Ok(options)
}

/// The part of a crash report this command reads (apps/updates/schemas/crash-report.schema.json).
#[derive(Debug, Deserialize)]
struct Report {
    native: Option<Native>,
}

/// A native crash as the app processed its minidump.
#[derive(Debug, Deserialize)]
struct Native {
    #[serde(default)]
    crash_reason: Option<String>,
    #[serde(default)]
    frames: Vec<Frame>,
    #[serde(default)]
    modules: Vec<Module>,
}

/// One frame, innermost first: an offset from its module's base.
#[derive(Debug, Deserialize)]
struct Frame {
    #[serde(default)]
    module: Option<String>,
    offset: u64,
    /// The name the app found on the computer (an exported symbol), if any.
    #[serde(default)]
    function: Option<String>,
}

/// A loaded module: its file name and Breakpad debug ID.
#[derive(Debug, Deserialize)]
struct Module {
    name: String,
    #[serde(default)]
    debug_id: Option<String>,
}

pub(crate) fn run(root: &Path, args: &[String]) -> Result<()> {
    let options = parse(args)?;
    let text = fs::read_to_string(&options.report).with_context(|| format!("reading {}", options.report.display()))?;
    let native = native_of(&text).with_context(|| options.report.display().to_string())?;
    let source = match options.symbols {
        Some(dir) => Source::Dir(dir),
        None => Source::Bucket {
            root: root.to_owned(),
            cache: root.join("target/crash-symbols"),
            fetch: options.fetch,
        },
    };
    let symbols = load_symbols(&native, &source)?;
    print!("{}", render(&native, &symbols));
    Ok(())
}

fn native_of(text: &str) -> Result<Native> {
    let report: Report = serde_json::from_str(text).context("not a crash report")?;
    report
        .native
        .context("the report has no `native` object: it describes no native crash")
}

/// Where symbol files are read from.
enum Source {
    /// A folder in the bucket's layout.
    Dir(PathBuf),
    /// The bucket, through Wrangler when `fetch`, cached under `cache`.
    Bucket { root: PathBuf, cache: PathBuf, fetch: bool },
}

impl Source {
    /// The local path of the symbol file at `key`, or `None` when there is none.
    fn find(&self, key: &str) -> Result<Option<PathBuf>> {
        match self {
            Source::Dir(dir) => {
                let path = dir.join(key);
                Ok(path.is_file().then_some(path))
            }
            Source::Bucket {
                root,
                cache,
                fetch: allowed,
            } => {
                let path = cache.join(key);
                if path.is_file() {
                    return Ok(Some(path));
                }
                if !allowed {
                    eprintln!("warning: no cached symbols at {key}; pass --fetch to download them from {BUCKET}");
                    return Ok(None);
                }
                fetch(root, key, &path)
            }
        }
    }
}

/// Fetches `key` from the bucket into `path` with the repository's Wrangler. Wrangler fails alike
/// for a missing object and a missing login, so a failure is a warning naming what it said.
fn fetch(root: &Path, key: &str, path: &Path) -> Result<Option<PathBuf>> {
    let dir = path.parent().context("a cache path has a folder")?;
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    // Written beside the cache's name, then renamed, so a failed download leaves no file behind.
    let part = path.with_extension("sym.part");
    let output = Command::new(crate::program_path("pnpm"))
        .args([
            "--filter",
            "@laminapdf/updates",
            "exec",
            "wrangler",
            "r2",
            "object",
            "get",
        ])
        .arg(format!("{BUCKET}/{key}"))
        .arg("--file")
        .arg(&part)
        .arg("--remote")
        .current_dir(root)
        .output()
        .context("running Wrangler through pnpm (run `pnpm install` first)")?;
    if !output.status.success() || !part.is_file() {
        fs::remove_file(&part).unwrap_or_default();
        let said = String::from_utf8_lossy(&output.stderr);
        let last = said
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("no message");
        eprintln!("warning: no {BUCKET}/{key} ({}): {last}", output.status);
        return Ok(None);
    }
    fs::rename(&part, path).with_context(|| format!("moving {} to {}", part.display(), path.display()))?;
    Ok(Some(path.to_owned()))
}

/// Where the symbols of the module `name` with `debug_id` are archived, in rust-minidump's
/// `<debug file>/<debug ID>/<debug file without .pdb>.sym` layout; `None` for a name or ID that
/// can't be one (the report comes from someone's computer, and the key becomes a path).
///
/// The report names a module by its file name, the symbols by its debug file: the same name on
/// macOS and Linux, but on Windows the PDB, which Cargo names with underscores. The bundled main
/// binary may carry the product name instead of the Cargo one.
fn symbol_key(name: &str, debug_id: &str) -> Option<String> {
    let safe_name = !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-' | ' '));
    let safe_id = (32..=40).contains(&debug_id.len()) && debug_id.chars().all(|c| c.is_ascii_hexdigit());
    if !(safe_name && safe_id) {
        return None;
    }
    let id = debug_id.to_ascii_uppercase();
    let (stem, windows) = match name.rsplit_once('.') {
        Some((stem, ext)) if ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("dll") => (stem, true),
        _ => (name, false),
    };
    let stem = if stem == PRODUCT_NAME { APP_BINARY } else { stem };
    Some(if windows {
        let pdb = stem.replace('-', "_");
        format!("{pdb}.pdb/{id}/{pdb}.sym")
    } else {
        format!("{stem}/{id}/{stem}.sym")
    })
}

/// The symbol files of the modules the frames name, by module name. A module with no debug ID, no
/// archived file or a file that doesn't parse is left out, with a warning.
fn load_symbols(native: &Native, source: &Source) -> Result<BTreeMap<String, SymbolFile>> {
    let mut symbols = BTreeMap::new();
    for module in &native.modules {
        let named = native
            .frames
            .iter()
            .any(|f| f.module.as_deref() == Some(module.name.as_str()));
        if !named || symbols.contains_key(&module.name) {
            continue;
        }
        let Some(key) = module.debug_id.as_deref().and_then(|id| symbol_key(&module.name, id)) else {
            eprintln!("warning: {}: no usable debug ID", module.name);
            continue;
        };
        let Some(path) = source.find(&key)? else {
            eprintln!("warning: {}: no symbols at {key}", module.name);
            continue;
        };
        match SymbolFile::from_file(&path) {
            Ok(file) => {
                symbols.insert(module.name.clone(), file);
            }
            Err(error) => eprintln!("warning: {}: {} doesn't parse: {error}", module.name, path.display()),
        }
    }
    Ok(symbols)
}

/// A frame's source location.
#[derive(Debug, Default, PartialEq, Eq)]
struct Location {
    function: String,
    file: Option<String>,
    line: Option<u32>,
}

/// What a symbol file says about one offset: the function, and the functions inlined into it
/// there, outermost first.
#[derive(Debug, Default)]
struct Lookup {
    offset: u64,
    outer: Option<Location>,
    inlined: Vec<Location>,
}

impl FrameSymbolizer for Lookup {
    fn get_instruction(&self) -> u64 {
        self.offset
    }
    fn set_function(&mut self, name: &str, _base: u64, _parameter_size: u32) {
        self.outer = Some(Location {
            function: name.to_owned(),
            ..Location::default()
        });
    }
    fn set_source_file(&mut self, file: &str, line: u32, _base: u64) {
        if let Some(outer) = &mut self.outer {
            outer.file = Some(file.to_owned());
            outer.line = Some(line);
        }
    }
    fn add_inline_frame(&mut self, name: &str, file: Option<&str>, line: Option<u32>) {
        self.inlined.push(Location {
            function: name.to_owned(),
            file: file.map(str::to_owned),
            line,
        });
    }
}

/// Looks `offset` up in `symbols`; the module's base is 0, since offsets are from it.
fn lookup(symbols: &SymbolFile, offset: u64) -> Lookup {
    let mut found = Lookup {
        offset,
        ..Lookup::default()
    };
    symbols.fill_symbol(&SimpleModule::default(), &mut found);
    found
}

/// The crash reason and the stack, one line per frame (`#n module function (file:line)`), each
/// inlined function on a line of its own above the frame it was inlined into.
fn render(native: &Native, symbols: &BTreeMap<String, SymbolFile>) -> String {
    let mut out = String::new();
    let reason = printable(native.crash_reason.as_deref().unwrap_or("not given"));
    writeln!(out, "Crash reason: {reason}").unwrap_or_default();
    for (n, frame) in native.frames.iter().enumerate() {
        let raw_module = frame.module.as_deref().unwrap_or("???");
        let module = printable(raw_module);
        let found = symbols.get(raw_module).map(|file| lookup(file, frame.offset));
        if let Some(Lookup {
            outer: Some(outer),
            inlined,
            ..
        }) = found
        {
            for location in inlined.iter().rev() {
                writeln!(out, "#{n} {module} {} [inlined]", described(location)).unwrap_or_default();
            }
            writeln!(out, "#{n} {module} {}", described(&outer)).unwrap_or_default();
        } else {
            let function = frame
                .function
                .as_deref()
                .map(|f| format!("{} ", printable(f)))
                .unwrap_or_default();
            writeln!(out, "#{n} {module} {function}+{:#x}", frame.offset).unwrap_or_default();
        }
    }
    out
}

fn described(location: &Location) -> String {
    let function = printable(&location.function);
    match (&location.file, location.line) {
        (Some(file), Some(line)) => format!("{function} ({}:{line})", printable(file)),
        (Some(file), None) => format!("{function} ({})", printable(file)),
        _ => function,
    }
}

/// `text` with every control character, a terminal escape's start among them, as `?`: a report
/// comes from a user's computer and is printed to a maintainer's terminal.
fn printable(text: &str) -> String {
    text.chars().map(|c| if c.is_control() { '?' } else { c }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A module of two functions: `engine::render`, into which `engine::draw` is inlined at
    /// 0x1010..0x1020, and `engine::main`. Hand-written in `dump_syms`'s format.
    const SYM: &str = "\
MODULE Linux x86_64 ABCDEF0123456789ABCDEF0123456789A laminapdf-desktop-engine
INFO CODE_ID EFCDAB8967452301
FILE 0 /build/crates/laminapdf-desktop-engine/src/render.rs
FILE 1 /build/crates/laminapdf-desktop-engine/src/main.rs
INLINE_ORIGIN 0 engine::draw
FUNC 1000 40 0 engine::render
INLINE 0 12 0 0 1010 10
1000 10 10 0
1010 10 30 0
1020 20 14 0
FUNC 2000 20 0 engine::main
2000 20 7 1
PUBLIC 3000 0 engine_exported
";

    const REPORT: &str = r#"{
        "schema": 1,
        "kind": "native",
        "native": {
            "crash_reason": "EXC_BAD_ACCESS / KERN_INVALID_ADDRESS",
            "frames": [
                { "module": "laminapdf-desktop-engine", "offset": 4116 },
                { "module": "laminapdf-desktop-engine", "offset": 4130 },
                { "module": "laminapdf-desktop-engine", "offset": 8196 },
                { "module": "libpdfium.dylib", "offset": 1234, "function": "FPDF_LoadPage" }
            ],
            "modules": [
                { "name": "laminapdf-desktop-engine", "version": "0.4.0", "debug_id": "ABCDEF0123456789ABCDEF0123456789A" },
                { "name": "libpdfium.dylib", "version": null, "debug_id": "0123456789ABCDEF0123456789ABCDEF0" }
            ]
        }
    }"#;

    /// A fresh, empty folder of the bucket's layout under the system's temporary folder.
    fn store(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-crash-{name}-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clearing the store");
        }
        fs::create_dir_all(&dir).expect("the store");
        dir
    }

    fn symbolicate(dir: &Path) -> String {
        let native = native_of(REPORT).expect("the sample report");
        let symbols = load_symbols(&native, &Source::Dir(dir.to_owned())).expect("loading");
        render(&native, &symbols)
    }

    #[test]
    fn symbolicate_should_name_each_frame_with_its_function_and_line() {
        let dir = store("named");
        let key = "laminapdf-desktop-engine/ABCDEF0123456789ABCDEF0123456789A/laminapdf-desktop-engine.sym";
        fs::create_dir_all(dir.join(key).parent().expect("a folder")).expect("the module's folder");
        fs::write(dir.join(key), SYM).expect("the symbol file");
        let render = "/build/crates/laminapdf-desktop-engine/src/render.rs";
        let main = "/build/crates/laminapdf-desktop-engine/src/main.rs";
        assert_eq!(
            symbolicate(&dir),
            format!(
                "Crash reason: EXC_BAD_ACCESS / KERN_INVALID_ADDRESS\n\
                 #0 laminapdf-desktop-engine engine::draw ({render}:30) [inlined]\n\
                 #0 laminapdf-desktop-engine engine::render ({render}:12)\n\
                 #1 laminapdf-desktop-engine engine::render ({render}:14)\n\
                 #2 laminapdf-desktop-engine engine::main ({main}:7)\n\
                 #3 libpdfium.dylib FPDF_LoadPage +0x4d2\n"
            )
        );
        fs::remove_dir_all(&dir).expect("clearing the store");
    }

    #[test]
    fn symbolicate_should_print_module_and_offset_when_the_symbols_are_missing() {
        let dir = store("missing");
        assert_eq!(
            symbolicate(&dir),
            "Crash reason: EXC_BAD_ACCESS / KERN_INVALID_ADDRESS\n\
             #0 laminapdf-desktop-engine +0x1014\n\
             #1 laminapdf-desktop-engine +0x1022\n\
             #2 laminapdf-desktop-engine +0x2004\n\
             #3 libpdfium.dylib FPDF_LoadPage +0x4d2\n"
        );
        fs::remove_dir_all(&dir).expect("clearing the store");
    }

    #[test]
    fn symbol_key_should_follow_the_breakpad_layout() {
        let id = "abcdef0123456789abcdef0123456789a";
        let upper = id.to_ascii_uppercase();
        assert_eq!(
            symbol_key("laminapdf-desktop-engine", id),
            Some(format!("laminapdf-desktop-engine/{upper}/laminapdf-desktop-engine.sym"))
        );
        assert_eq!(
            symbol_key("laminapdf-desktop-engine.exe", id),
            Some(format!(
                "laminapdf_desktop_engine.pdb/{upper}/laminapdf_desktop_engine.sym"
            ))
        );
        assert_eq!(
            symbol_key("Lamina PDF", id),
            Some(format!("laminapdf-desktop/{upper}/laminapdf-desktop.sym"))
        );
    }

    #[test]
    fn symbol_key_should_refuse_a_name_or_id_that_leaves_the_store() {
        let id = "ABCDEF0123456789ABCDEF0123456789A";
        assert_eq!(symbol_key("../../etc", id), None);
        assert_eq!(symbol_key("a/b", id), None);
        assert_eq!(symbol_key("engine", "../ABCDEF0123456789ABCDEF0123456789"), None);
    }

    #[test]
    fn a_report_without_a_native_crash_should_be_refused() {
        assert!(native_of(r#"{ "schema": 1, "kind": "panic" }"#).is_err());
    }

    #[test]
    fn parse_should_take_a_report_and_an_optional_folder() {
        let args = |list: &[&str]| list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            parse(&args(&["symbolicate", "r.json", "--symbols", "syms"])).expect("parses"),
            Options {
                report: "r.json".into(),
                symbols: Some("syms".into()),
                fetch: false,
            }
        );
        assert!(parse(&args(&["symbolicate"])).is_err());
    }

    #[test]
    fn parse_should_fetch_only_when_asked() {
        let args = |list: &[&str]| list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
        assert!(!parse(&args(&["symbolicate", "r.json"])).expect("parses").fetch);
        assert!(
            parse(&args(&["symbolicate", "r.json", "--fetch"]))
                .expect("parses")
                .fetch
        );
        assert!(parse(&args(&["symbolicate", "r.json", "--fetch", "--fetch"])).is_err());
    }

    #[test]
    fn printable_should_drop_terminal_escapes_from_a_report() {
        assert_eq!(printable("SIGSEGV\u{1b}]0;owned\u{7}\n"), "SIGSEGV?]0;owned??");
    }
}
