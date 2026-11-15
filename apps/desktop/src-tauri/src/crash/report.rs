//! The crash report, exactly as the app shows it and the update Worker receives it
//! (apps/updates/schemas/crash-report.schema.json). [`Report`] is the allow-list: it has no other
//! field, so nothing else can be sent, and every string in it is either one of a closed set or has
//! passed [`scrub`]. Its pieces come from where the crash happened: a panic's summary, how an
//! engine ended, or a webview's error.

use std::time::Duration;

use laminapdf_desktop_protocol::crash::{
    MAX_MESSAGE, NativeFrame, PanicSummary, SourceLocation, UNKNOWN_MODULE, truncate_on_char_boundary,
};
use laminapdf_desktop_protocol::{Layer, SandboxLevel};
use serde::{Deserialize, Serialize};

use super::scrub::{MAX_FUNCTION, ScriptFrame, scrub};

/// The script errors a webview report names; any other is [`OTHER_ERROR`].
const SCRIPT_ERRORS: [&str; 12] = [
    "Error",
    "TypeError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "EvalError",
    "URIError",
    "AggregateError",
    "DOMException",
    "AbortError",
    "DesktopEngineError",
    "LaminaPdfError",
];

/// A script error of another type, or a rejection with something other than an error.
const OTHER_ERROR: &str = "OtherError";

/// The report's schema version.
const SCHEMA: u8 = 1;

/// One crash report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Report {
    pub schema: u8,
    /// A random UUID v4, made for this report only and kept nowhere else.
    #[expect(clippy::struct_field_names, reason = "the schema's name for it")]
    pub report_id: String,
    pub app_version: String,
    pub channel: ReportChannel,
    pub os: Os,
    pub os_version: String,
    pub arch: Arch,
    pub process: ProcessKind,
    pub renderer: Renderer,
    pub sandbox: Vec<SandboxLayer>,
    pub uptime: Uptime,
    pub kind: CrashKind,
    /// The panic or error message, or how the engine ended; scrubbed.
    pub reason: String,
    pub location: Option<Location>,
    pub frames: Vec<Frame>,
    /// Always empty: a native crash's modules are in [`Report::native`].
    pub modules: Vec<Module>,
    /// A native crash's stack, from its minidump read on this computer (D7d); left out otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<Native>,
}

/// What a minidump says about a native crash, read on this computer: the raw dump never leaves
/// it. Every string here passed the allow-list in `native.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Native {
    /// Why it crashed, as the OS put it (`EXC_BAD_ACCESS / KERN_INVALID_ADDRESS`); scrubbed.
    pub crash_reason: String,
    /// The crashing thread's frames, innermost first, at most 64.
    pub frames: Vec<DumpFrame>,
    /// The modules the frames name, each once: the symbol archive's keys.
    pub modules: Vec<DumpModule>,
}

/// A native frame: the module's file name (or `<unknown>`) and the address within it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DumpFrame {
    pub module: String,
    pub offset: u64,
    /// Always `None` here: the computer has no symbols (`cargo xtask crash symbolicate` names it).
    pub function: Option<String>,
}

/// A module a native frame names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DumpModule {
    pub name: String,
    pub version: Option<String>,
    /// Breakpad's debug ID (32 to 40 hex digits): what `cargo xtask crash symbolicate` looks up.
    pub debug_id: String,
}

/// The channel this build follows; `dev` for a debug build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReportChannel {
    Stable,
    Beta,
    Nightly,
    Dev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Os {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Arch {
    #[serde(rename = "x86_64")]
    X86_64,
    Aarch64,
}

/// Which process crashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ProcessKind {
    Main,
    Engine,
    Webview,
}

/// What draws pages: PDFium, until the Linux renderer switch adds another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Renderer {
    Pdfium,
}

/// One layer of the engine's sandbox ([`Layer`], without Landlock's ABI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SandboxLayer {
    Off,
    NoNewPrivs,
    Seccomp,
    Landlock,
    MemoryLimit,
    Seatbelt,
    Job,
    LowIntegrity,
    RestrictedToken,
}

impl SandboxLayer {
    /// `layer`'s name in a report; `None` for a layer newer than the schema, which is left out.
    const fn of(layer: Layer) -> Option<Self> {
        Some(match layer {
            Layer::Off => Self::Off,
            Layer::NoNewPrivs => Self::NoNewPrivs,
            Layer::Seccomp => Self::Seccomp,
            Layer::Landlock { .. } => Self::Landlock,
            Layer::MemoryLimit => Self::MemoryLimit,
            Layer::Seatbelt => Self::Seatbelt,
            Layer::Job => Self::Job,
            Layer::LowIntegrity => Self::LowIntegrity,
            Layer::RestrictedToken => Self::RestrictedToken,
            _ => return None,
        })
    }
}

/// How long the app had run, coarsely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Uptime {
    #[serde(rename = "under_1m")]
    Under1m,
    #[serde(rename = "1m_to_10m")]
    To10m,
    #[serde(rename = "10m_to_1h")]
    To1h,
    #[serde(rename = "over_1h")]
    Over1h,
}

impl Uptime {
    pub(crate) fn of(running: Duration) -> Self {
        match running.as_secs() {
            0..60 => Self::Under1m,
            60..600 => Self::To10m,
            600..3600 => Self::To1h,
            _ => Self::Over1h,
        }
    }
}

/// What kind of crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CrashKind {
    /// A Rust panic.
    Panic,
    /// The engine was ended by a signal or an exception code.
    Signal,
    /// The engine exited with a failure code.
    Exit,
    /// An uncaught error in the webview.
    Error,
    /// An unhandled promise rejection in the webview.
    Rejection,
}

/// Where a panic happened, in lamina's own sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// A frame: native for Rust, a script's for the webview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum Frame {
    Native {
        module: String,
        /// The offset within `function`, as `0x…`.
        offset: String,
        function: String,
    },
    Script(ScriptFrame),
}

/// A loaded module, from a processed minidump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Module {
    pub name: String,
    pub version: Option<String>,
    pub debug_id: Option<String>,
}

/// What every report says about the app, whatever crashed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppFacts {
    pub app_version: String,
    pub channel: ReportChannel,
    pub os_version: String,
}

/// What crashed and how, before it becomes a [`Report`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Crash {
    pub process: ProcessKind,
    pub kind: CrashKind,
    /// Unscrubbed: [`Crash::report`] scrubs it.
    pub reason: String,
    pub location: Option<SourceLocation>,
    pub frames: Vec<Frame>,
    /// The engine's sandbox, when the engine was running.
    pub sandbox: Option<SandboxLevel>,
    /// The minidump's summary, for a native crash; already allow-listed.
    pub native: Option<Native>,
}

impl Crash {
    /// A panic in `process`.
    pub(crate) fn panic(process: ProcessKind, summary: PanicSummary, sandbox: Option<SandboxLevel>) -> Self {
        Self {
            process,
            kind: CrashKind::Panic,
            reason: summary.message,
            location: summary.location,
            frames: summary.frames.iter().map(native).collect(),
            sandbox,
            native: None,
        }
    }

    /// A native crash of the main process, known only from the minidump its monitor wrote: the
    /// report the next launch makes of it.
    #[cfg(unix)]
    pub(crate) fn main_native(native: Native) -> Self {
        Self {
            process: ProcessKind::Main,
            kind: CrashKind::Signal,
            reason: native.crash_reason.clone(),
            location: None,
            frames: Vec::new(),
            sandbox: None,
            native: Some(native),
        }
    }

    /// An engine that ended without a panic: `how` is the sandbox's description of its end.
    pub(crate) fn engine_exit(how: String, sandbox: Option<SandboxLevel>) -> Self {
        // `describe` in laminapdf-desktop-sandbox: "exit code: 3" or "exit status: 3" for an exit,
        // anything else (a signal, an exception code, a sandbox stop) is the OS ending it.
        let exited = how.starts_with("exit code") || how.starts_with("exit status");
        Self {
            process: ProcessKind::Engine,
            kind: if exited { CrashKind::Exit } else { CrashKind::Signal },
            reason: how,
            location: None,
            frames: Vec::new(),
            sandbox,
            native: None,
        }
    }

    /// An uncaught error named `name` (`rejection`: an unhandled rejection) in a webview, with its
    /// stack. Only the error's type is kept, from [`SCRIPT_ERRORS`]: a script's message can quote
    /// anything the page holds, a document's text included, which no scrubber can tell from code.
    /// The frames say where it was thrown.
    pub(crate) fn webview(name: &str, stack: &str, rejection: bool) -> Self {
        let reason = SCRIPT_ERRORS
            .iter()
            .find(|known| **known == name)
            .copied()
            .unwrap_or(OTHER_ERROR);
        Self {
            process: ProcessKind::Webview,
            kind: if rejection {
                CrashKind::Rejection
            } else {
                CrashKind::Error
            },
            reason: reason.to_owned(),
            location: None,
            frames: super::scrub::script_frames(stack)
                .into_iter()
                .map(Frame::Script)
                .collect(),
            sandbox: None,
            native: None,
        }
    }

    /// The report, with a new random ID and the crash's facts scrubbed.
    pub(crate) fn report(self, facts: &AppFacts, uptime: Duration) -> Report {
        let sandbox = self
            .sandbox
            .map(|level| dedup(level.layers.into_iter().filter_map(SandboxLayer::of)))
            .unwrap_or_default();
        Report {
            schema: SCHEMA,
            report_id: uuid::Uuid::new_v4().to_string(),
            app_version: facts.app_version.clone(),
            channel: facts.channel,
            os: OS,
            os_version: facts.os_version.clone(),
            arch: ARCH,
            process: self.process,
            renderer: Renderer::Pdfium,
            sandbox,
            uptime: Uptime::of(uptime),
            kind: self.kind,
            reason: truncate_on_char_boundary(scrub(&self.reason).into_owned(), MAX_MESSAGE),
            location: self.location.as_ref().and_then(ours),
            frames: self.frames,
            modules: Vec::new(),
            native: self.native,
        }
    }
}

/// A native frame, its function name scrubbed (a closure's name can't hold a path, but a
/// compromised engine's frames could hold anything).
fn native(frame: &NativeFrame) -> Frame {
    Frame::Native {
        module: module_name(&frame.module),
        offset: format!("{:#x}", frame.offset),
        function: truncate_on_char_boundary(scrub(&frame.function).into_owned(), MAX_FUNCTION),
    }
}

/// `name` if it is a plain file name the schema accepts, else `<unknown>`.
pub(super) fn module_name(name: &str) -> String {
    let plain = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-'));
    if plain {
        name.to_owned()
    } else {
        UNKNOWN_MODULE.to_owned()
    }
}

/// `at` when it is in lamina's own sources (`apps/…` or `crates/…`, relative, no `..`); else
/// `None`, since another crate's location is an absolute path on the build machine.
fn ours(at: &SourceLocation) -> Option<Location> {
    let file = at.file.replace('\\', "/");
    let segments_ok = file.split('/').all(|segment| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    });
    let ours = (file.starts_with("apps/") || file.starts_with("crates/"))
        && std::path::Path::new(&file).extension().is_some_and(|ext| ext == "rs")
        && file.len() <= 160
        && segments_ok
        && !file.split('/').any(|segment| segment.starts_with('.'))
        && file.matches('.').count() == 1;
    ours.then_some(Location {
        file,
        line: at.line,
        column: at.column,
    })
}

fn dedup(layers: impl Iterator<Item = SandboxLayer>) -> Vec<SandboxLayer> {
    let mut out: Vec<SandboxLayer> = Vec::new();
    for layer in layers {
        if !out.contains(&layer) {
            out.push(layer);
        }
    }
    out
}

const OS: Os = if cfg!(target_os = "macos") {
    Os::Macos
} else if cfg!(windows) {
    Os::Windows
} else {
    Os::Linux
};

const ARCH: Arch = if cfg!(target_arch = "aarch64") {
    Arch::Aarch64
} else {
    Arch::X86_64
};

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> AppFacts {
        AppFacts {
            app_version: "0.1.0".into(),
            channel: ReportChannel::Beta,
            os_version: "15.1".into(),
        }
    }

    fn summary(message: &str, file: &str) -> PanicSummary {
        PanicSummary {
            message: message.into(),
            location: Some(SourceLocation {
                file: file.into(),
                line: 120,
                column: 9,
            }),
            frames: vec![NativeFrame {
                module: "laminapdf-desktop-engine".into(),
                offset: 0x1a,
                function: "laminapdf_desktop_engine::worker::render".into(),
            }],
        }
    }

    /// The fixture the Worker's tests hold to the schema.
    fn fixture() -> serde_json::Value {
        fixture_named("crash-report.json")
    }

    /// The Worker's fixture `name`.
    fn fixture_named(name: &str) -> serde_json::Value {
        let path = format!("{}/../../updates/test/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn native() -> Native {
        Native {
            crash_reason: "EXC_BAD_ACCESS / KERN_INVALID_ADDRESS".into(),
            frames: vec![DumpFrame {
                module: "laminapdf-desktop-engine".into(),
                offset: 0x44_016c,
                function: None,
            }],
            modules: vec![DumpModule {
                name: "laminapdf-desktop-engine".into(),
                version: None,
                debug_id: "5887580E1F1A3CE2BEAB358DCFAA73030".into(),
            }],
        }
    }

    /// The keys of the object at `pointer` in `value`, sorted.
    fn keys_at(value: &serde_json::Value, pointer: &str) -> Vec<String> {
        let mut keys = keys(value.pointer(pointer).unwrap_or(&serde_json::Value::Null));
        keys.sort();
        keys
    }

    #[test]
    fn a_native_report_should_have_exactly_the_schemas_fields() {
        let mut crash = Crash::engine_exit("crash: signal: 11 (SIGSEGV)".into(), None);
        crash.native = Some(native());
        let built = serde_json::to_value(crash.report(&facts(), Duration::ZERO)).unwrap();
        let expected = fixture_named("crash-report-native.json");
        let pointers = ["", "/native", "/native/frames/0", "/native/modules/0"];
        let differ: Vec<&str> = pointers
            .into_iter()
            .filter(|pointer| keys_at(&built, pointer) != keys_at(&expected, pointer))
            .collect();
        assert_eq!(differ, Vec::<&str>::new());
    }

    #[test]
    fn the_native_fixture_should_read_as_a_report_with_its_stack() {
        let report: Report = serde_json::from_value(fixture_named("crash-report-native.json")).unwrap();
        assert_eq!(report.native.map(|native| native.frames.len()), Some(2));
    }

    #[test]
    fn a_report_without_a_stack_should_leave_native_out() {
        let report = Crash::engine_exit("killed".into(), None).report(&facts(), Duration::ZERO);
        assert!(serde_json::to_value(report).unwrap().get("native").is_none());
    }

    fn keys(value: &serde_json::Value) -> Vec<String> {
        value
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    }

    #[test]
    fn a_report_should_have_exactly_the_schemas_fields() {
        let report = Crash::panic(
            ProcessKind::Engine,
            summary("boom", "crates/laminapdf-desktop-engine/src/worker.rs"),
            None,
        )
        .report(&facts(), Duration::from_secs(90));
        let mut built = keys(&serde_json::to_value(&report).unwrap());
        let mut expected = keys(&fixture());
        built.sort();
        expected.sort();
        assert_eq!(built, expected);
    }

    /// The schema's `enum` at `pointer`.
    fn schema_enum(pointer: &str) -> serde_json::Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../updates/schemas/crash-report.schema.json"
        );
        let schema: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        schema.pointer(pointer).cloned().unwrap_or_default()
    }

    #[test]
    fn every_closed_field_should_take_exactly_the_schemas_values() {
        let pairs = [
            (
                "/properties/channel/enum",
                serde_json::to_value([
                    ReportChannel::Stable,
                    ReportChannel::Beta,
                    ReportChannel::Nightly,
                    ReportChannel::Dev,
                ])
                .unwrap(),
            ),
            (
                "/properties/os/enum",
                serde_json::to_value([Os::Macos, Os::Windows, Os::Linux]).unwrap(),
            ),
            (
                "/properties/arch/enum",
                serde_json::to_value([Arch::X86_64, Arch::Aarch64]).unwrap(),
            ),
            (
                "/properties/process/enum",
                serde_json::to_value([ProcessKind::Main, ProcessKind::Engine, ProcessKind::Webview]).unwrap(),
            ),
            (
                "/properties/renderer/enum",
                serde_json::to_value([Renderer::Pdfium]).unwrap(),
            ),
            (
                "/properties/sandbox/items/enum",
                serde_json::to_value([
                    SandboxLayer::Off,
                    SandboxLayer::NoNewPrivs,
                    SandboxLayer::Seccomp,
                    SandboxLayer::Landlock,
                    SandboxLayer::MemoryLimit,
                    SandboxLayer::Seatbelt,
                    SandboxLayer::Job,
                    SandboxLayer::LowIntegrity,
                    SandboxLayer::RestrictedToken,
                ])
                .unwrap(),
            ),
            (
                "/properties/uptime/enum",
                serde_json::to_value([Uptime::Under1m, Uptime::To10m, Uptime::To1h, Uptime::Over1h]).unwrap(),
            ),
            (
                "/properties/kind/enum",
                serde_json::to_value([
                    CrashKind::Panic,
                    CrashKind::Signal,
                    CrashKind::Exit,
                    CrashKind::Error,
                    CrashKind::Rejection,
                ])
                .unwrap(),
            ),
        ];
        let differ: Vec<&str> = pairs
            .into_iter()
            .filter(|(pointer, ours)| schema_enum(pointer) != *ours)
            .map(|(pointer, _)| pointer)
            .collect();
        assert_eq!(differ, Vec::<&str>::new());
    }

    #[test]
    fn a_report_should_hold_no_uuid_but_its_own_id() {
        let uuid =
            regex::Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}").unwrap();
        let report =
            Crash::panic(ProcessKind::Engine, summary("boom", "src/a.rs"), None).report(&facts(), Duration::ZERO);
        let json = serde_json::to_string(&report).unwrap();
        let found: Vec<&str> = uuid.find_iter(&json).map(|m| m.as_str()).collect();
        assert_eq!(found, [report.report_id.as_str()]);
    }

    #[test]
    fn the_fixture_should_read_as_a_report() {
        assert!(serde_json::from_value::<Report>(fixture()).is_ok());
    }

    #[test]
    fn a_panic_message_should_be_scrubbed() {
        let report = Crash::panic(ProcessKind::Main, summary("open /Users/ana/x/a.pdf", "src/a.rs"), None)
            .report(&facts(), Duration::ZERO);
        assert_eq!(report.reason, "open <path>");
    }

    #[test]
    fn a_location_outside_lamina_should_be_dropped() {
        let report = Crash::panic(
            ProcessKind::Main,
            summary("boom", "/Users/ci/.cargo/registry/src/x-1.0/src/lib.rs"),
            None,
        )
        .report(&facts(), Duration::ZERO);
        assert_eq!(report.location, None);
    }

    #[test]
    fn a_location_in_lamina_should_be_kept() {
        let report = Crash::panic(
            ProcessKind::Engine,
            summary("boom", "crates/laminapdf-desktop-engine/src/worker.rs"),
            None,
        )
        .report(&facts(), Duration::ZERO);
        assert_eq!(
            report.location.map(|at| at.file),
            Some("crates/laminapdf-desktop-engine/src/worker.rs".to_owned())
        );
    }

    #[test]
    fn an_engine_ended_by_a_signal_should_be_a_signal() {
        let crash = Crash::engine_exit("crash: signal: 11 (SIGSEGV)".into(), None);
        assert_eq!(crash.kind, CrashKind::Signal);
    }

    #[test]
    fn an_engine_that_exited_should_be_an_exit() {
        let crash = Crash::engine_exit("exit status: 3".into(), None);
        assert_eq!(crash.kind, CrashKind::Exit);
    }

    #[test]
    fn a_webview_error_should_keep_only_its_known_type() {
        let reasons = ["TypeError", "IgnorePreviousInstructions", "Error: at /a/b"]
            .map(|name| Crash::webview(name, "", false).reason);
        assert_eq!(reasons, ["TypeError", OTHER_ERROR, OTHER_ERROR]);
    }

    #[test]
    fn a_module_name_with_a_path_should_be_unknown() {
        assert_eq!(module_name("/usr/lib/libpdfium.so"), "<unknown>");
    }

    #[test]
    fn each_report_should_get_its_own_id() {
        let make = || Crash::engine_exit("killed".into(), None).report(&facts(), Duration::ZERO);
        assert_ne!(make().report_id, make().report_id);
    }

    #[test]
    fn uptime_should_be_bucketed() {
        let buckets = [0, 59, 60, 599, 600, 3599, 3600].map(|s| Uptime::of(Duration::from_secs(s)));
        assert_eq!(
            buckets,
            [
                Uptime::Under1m,
                Uptime::Under1m,
                Uptime::To10m,
                Uptime::To10m,
                Uptime::To1h,
                Uptime::To1h,
                Uptime::Over1h
            ]
        );
    }
}
