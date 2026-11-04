//! `laminapdf mcp` ([ADR 0031] §9): the commands as the tools of an MCP server on stdio.
//!
//! The server is a thin adapter. Each tool parses its arguments into the command's own
//! argument type, runs the same `commands::*` function the CLI runs, and returns the same report
//! (the tool's `outputSchema` is the report's schema). What makes it safe without opt-in flags:
//!
//! - **Roots:** every path goes through [`roots::Roots`]; nothing outside the `--root` folders is
//!   read or written.
//! - **Read-only by default:** the write tool is listed only with `--allow-write`, and it
//!   replaces its input only with `--allow-in-place` too.
//! - **Document text only in data:** reports keep it in their fields; errors carry the ADR 0026
//!   object, whose message and hint never quote the document; tool descriptions are fixed text.
//! - **Bounded calls:** each runs on a blocking thread with the default `Limits`, at most one per
//!   CPU at once, and with `--timeout`: when it runs out, the call's [`CancelToken`] is cancelled,
//!   and the server waits for the command to stop at its next step before it answers.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md
//! [`CancelToken`]: laminapdf::CancelToken

pub mod roots;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use laminapdf::{CancelToken, ErrorCode};
use rmcp::ServiceExt as _;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tokio::sync::Semaphore;

use crate::commands::{
    self, CheckArgs, CheckReport, ERROR_SCHEMA_ID, ExplainArgs, ExplainReport, ExtractArgs, InfoArgs, InfoReport,
    PagesReport, RotateArgs, Settings, TextReport,
};
use crate::exit::CliError;
use roots::{RootedIo, Roots};

/// `laminapdf mcp`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct McpArgs {
    /// A folder the tools may read and write; repeat for more. Relative paths start at the first.
    #[arg(long = "root", value_name = "DIR", required = true)]
    pub roots: Vec<PathBuf>,
    /// List the write tool (`laminapdf_pages_rotate`), which writes new files only.
    #[arg(long)]
    pub allow_write: bool,
    /// Let the write tool replace its input (`inPlace`); needs --allow-write.
    #[arg(long, requires = "allow_write")]
    pub allow_in_place: bool,
    /// Cut a report down to about this many bytes of JSON, marking it `"truncated": true`.
    #[arg(long, value_name = "N", default_value_t = 1 << 20)]
    pub max_output_bytes: usize,
    /// Stop a call after this many seconds; it ends as a `cancelled` tool error.
    #[arg(long, value_name = "SECS", default_value_t = 60)]
    pub timeout: u64,
}

/// What the server tells clients it is for.
const INSTRUCTIONS: &str = "Lamina PDF's command-line tools as MCP tools: read a PDF's facts, check it, \
extract its text, explain an error code, and (when the server allows writing) rotate its pages. \
Paths are files under the server's root folders.";

/// Runs the server on stdin and stdout until the client closes them.
///
/// # Errors
///
/// `invalid-command-line` for a bad `--root`; `output-unwritable` when the transport fails.
pub fn run(args: &McpArgs) -> Result<(), CliError> {
    let server = Server::new(args)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| transport_error(&e))?;
    runtime.block_on(async move {
        let running = server
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| transport_error(&e))?;
        running.waiting().await.map(|_| ()).map_err(|e| transport_error(&e))
    })
}

fn transport_error(error: &dyn std::fmt::Display) -> CliError {
    CliError::new(ErrorCode::OutputUnwritable, "the MCP transport failed").with("reason", error.to_string())
}

/// The server: its roots, its flags and the tools it lists.
#[derive(Debug, Clone)]
pub struct Server {
    roots: Arc<Roots>,
    allow_in_place: bool,
    max_output_bytes: usize,
    timeout: Duration,
    permits: Arc<Semaphore>,
    tools: Arc<Vec<Tool>>,
}

impl Server {
    /// A server for `args`.
    ///
    /// # Errors
    ///
    /// `invalid-command-line` for a bad `--root`.
    pub fn new(args: &McpArgs) -> Result<Self, CliError> {
        let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        Ok(Self {
            roots: Arc::new(Roots::new(&args.roots)?),
            allow_in_place: args.allow_in_place,
            max_output_bytes: args.max_output_bytes,
            timeout: Duration::from_secs(args.timeout),
            permits: Arc::new(Semaphore::new(cpus)),
            tools: Arc::new(tools(args.allow_write, args.allow_in_place)),
        })
    }

    /// The tools, in name order: the same list on every call and every connection.
    #[must_use]
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// Runs tool `name` with `arguments`: its report, or its error as a tool error.
    async fn call(&self, name: &str, arguments: Map<String, Value>) -> Result<CallToolResult, ErrorData> {
        if !self.tools.iter().any(|tool| tool.name == name) {
            return Err(ErrorData::invalid_params("no tool by that name", None));
        }
        let Ok(_permit) = self.permits.acquire().await else {
            return Err(ErrorData::internal_error("the server is shutting down", None));
        };
        let settings = Settings::default();
        let cancel = settings.cancel.clone();
        let (name, roots, allow_in_place) = (name.to_owned(), Arc::clone(&self.roots), self.allow_in_place);
        let job = move || run_tool(&name, arguments, settings, &roots, allow_in_place);
        let outcome = within(self.timeout, &cancel, job).await.unwrap_or_else(|| {
            let error = CliError::new(ErrorCode::Cancelled, "the call ran longer than the server's --timeout");
            Err((error, None))
        });
        Ok(match outcome {
            Ok(report) => CallToolResult::structured(truncate(report, self.max_output_bytes)),
            Err((error, input)) => CallToolResult::structured_error(error.to_line(input.as_deref())),
        })
    }
}

/// Runs `job` on a blocking thread; `None` when it runs past `timeout`. Then `cancel` stops it
/// at its next step, and this returns only once it has, so no call goes on working unanswered.
async fn within<T: Send + 'static>(
    timeout: Duration,
    cancel: &CancelToken,
    job: impl FnOnce() -> Result<T, Failure> + Send + 'static,
) -> Option<Result<T, Failure>> {
    let mut handle = tokio::task::spawn_blocking(job);
    let finished = tokio::time::timeout(timeout, &mut handle).await;
    let Ok(joined) = finished else {
        cancel.cancel();
        let _ = handle.await;
        return None;
    };
    // A job that panicked (the commands return errors instead) ends the call as cancelled.
    Some(joined.unwrap_or_else(|_| {
        Err((
            CliError::new(ErrorCode::Cancelled, "the call stopped unexpectedly"),
            None,
        ))
    }))
}

/// A tool's failure and the input it was reading, for the error line.
type Failure = (CliError, Option<String>);

/// Runs one tool on a blocking thread.
fn run_tool(
    name: &str,
    arguments: Map<String, Value>,
    mut settings: Settings,
    roots: &Roots,
    allow_in_place: bool,
) -> Result<Value, Failure> {
    let mut io = RootedIo::new(roots);
    match name {
        "laminapdf_check" => {
            let args: CheckArgs = parse(arguments)?;
            report(commands::check(&args, &settings, &mut io), Some(&args.input))
        }
        "laminapdf_explain" => {
            let args: ExplainArgs = parse(arguments)?;
            report(commands::explain(&args), None)
        }
        "laminapdf_extract" => {
            let args: ExtractArgs = parse(arguments)?;
            report(commands::extract(&args, &settings, &mut io), Some(&args.input))
        }
        "laminapdf_info" => {
            let args: InfoArgs = parse(arguments)?;
            report(commands::info(&args, &settings, &mut io), Some(&args.input))
        }
        _ => {
            let args: RotateArgs = parse(arguments)?;
            if args.in_place && !allow_in_place {
                let error = CliError::new(ErrorCode::OverwriteRefused, "the server doesn't allow --in-place")
                    .with("output", args.input.as_str());
                return Err((error, Some(args.input)));
            }
            // In place needs --force in the command; the server's flag stands for it.
            settings.force = args.in_place;
            report(commands::rotate(&args, &settings, &mut io), Some(&args.input))
        }
    }
}

/// The arguments as the command's argument type.
fn parse<T: DeserializeOwned>(arguments: Map<String, Value>) -> Result<T, Failure> {
    serde_json::from_value(Value::Object(arguments)).map_err(|e| {
        let error = CliError::new(
            ErrorCode::InvalidCommandLine,
            "the arguments don't match the tool's input schema",
        )
        .with("detail", e.to_string());
        (error, None)
    })
}

fn report<R: serde::Serialize>(result: Result<R, CliError>, input: Option<&str>) -> Result<Value, Failure> {
    let report = result.map_err(|e| (e, input.map(str::to_owned)))?;
    serde_json::to_value(report).map_err(|_| {
        (
            CliError::new(ErrorCode::OutputUnwritable, "couldn't serialise the report"),
            None,
        )
    })
}

/// `report` cut down to about `max` bytes of JSON: the last items of its largest arrays go
/// first, then the second half of its longest text, until it fits; a cut report has
/// `"truncated": true`.
fn truncate(mut report: Value, max: usize) -> Value {
    let size = |value: &Value| value.to_string().len();
    if size(&report) <= max {
        return report;
    }
    while size(&report) > max && shrink(&mut report) {}
    if let Some(map) = report.as_object_mut() {
        map.insert("truncated".to_owned(), Value::Bool(true));
    }
    report
}

/// Removes part of `value`, the largest that can shrink first: the last item of an array, or
/// the second half of a text. `false` when nothing is left to remove.
fn shrink(value: &mut Value) -> bool {
    match value {
        Value::Object(map) => {
            let mut parts: Vec<&mut Value> = map.values_mut().collect();
            parts.sort_by_key(|part| std::cmp::Reverse(part.to_string().len()));
            parts.into_iter().any(shrink)
        }
        Value::Array(items) if items.len() == 1 => items.first_mut().is_some_and(shrink) || items.pop().is_some(),
        Value::Array(items) => items.pop().is_some(),
        Value::String(text) if text.len() > 1 => {
            let mut cut = text.len() / 2;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
            true
        }
        _ => false,
    }
}

/// A JSON Schema object for `T`, self-contained: a reference to `laminapdf.error/1` (a report's
/// `warnings`) becomes a reference to a copy of its definitions in the schema's own `$defs`, so
/// a client validating `structuredContent` never has to fetch it.
fn schema<T: JsonSchema>() -> Arc<Map<String, Value>> {
    let mut schema = schemars::schema_for!(T).to_value();
    if localise(&mut schema, &format!("{ERROR_SCHEMA_ID}#/$defs/")) {
        let error: Value = serde_json::from_str(laminapdf::ERROR_SCHEMA).unwrap_or_default();
        let defs = schema.as_object_mut().and_then(|map| {
            map.entry("$defs")
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
        });
        if let (Some(defs), Some(error_defs)) = (defs, error.get("$defs").and_then(Value::as_object)) {
            for (name, definition) in error_defs {
                defs.entry(name.clone()).or_insert_with(|| definition.clone());
            }
        }
    }
    match schema {
        Value::Object(map) => Arc::new(map),
        _ => Arc::default(),
    }
}

/// Rewrites every `$ref` that starts with `prefix` to the same name under `#/$defs/`; whether
/// it rewrote one.
fn localise(value: &mut Value, prefix: &str) -> bool {
    // Every reference is visited (no short-circuit): one schema may hold several.
    let mut rewrote = false;
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                match child {
                    Value::String(target) if key == "$ref" => {
                        if let Some(name) = target.strip_prefix(prefix) {
                            *target = format!("#/$defs/{name}");
                            rewrote = true;
                        }
                    }
                    _ => rewrote |= localise(child, prefix),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrote |= localise(item, prefix);
            }
        }
        _ => {}
    }
    rewrote
}

/// One tool: `T` its arguments, `R` its report.
fn tool<T: JsonSchema, R: JsonSchema>(
    name: &'static str,
    description: &'static str,
    annotations: ToolAnnotations,
) -> Tool {
    Tool::new(name, description, schema::<T>())
        .with_raw_output_schema(schema::<R>())
        .with_annotations(annotations)
}

/// The tools, in name order. Their descriptions are fixed text, never a path or document text.
fn tools(allow_write: bool, allow_in_place: bool) -> Vec<Tool> {
    let read = || ToolAnnotations::new().read_only(true).open_world(false);
    let mut tools = vec![
        tool::<CheckArgs, CheckReport>(
            "laminapdf_check",
            "Read a whole PDF (every object, page and content stream) and report what a tolerant read had to repair.",
            read(),
        ),
        tool::<ExplainArgs, ExplainReport>(
            "laminapdf_explain",
            "The documentation page of a Lamina PDF error or warning code.",
            read(),
        ),
        tool::<ExtractArgs, TextReport>(
            "laminapdf_extract",
            "A PDF's text, page by page; `pages` takes a range such as 1-3, z (the last page) or 1-z,x2.",
            read(),
        ),
        tool::<InfoArgs, InfoReport>(
            "laminapdf_info",
            "What a PDF is: its version, pages and their sizes, file size, encryption, tagging, metadata and warnings.",
            read(),
        ),
    ];
    if allow_write {
        let (description, destructive) = if allow_in_place {
            (
                "Rotate pages of a PDF: +90 or -90 turn them, 90 sets their rotation, :RANGE names the pages. \
                 Writes `output`, a new file, or with `inPlace` replaces the input.",
                true,
            )
        } else {
            (
                "Rotate pages of a PDF: +90 or -90 turn them, 90 sets their rotation, :RANGE names the pages. \
                 Writes `output`, which must be a new file.",
                false,
            )
        };
        let annotations = ToolAnnotations::new()
            .read_only(false)
            .destructive(destructive)
            .open_world(false);
        tools.push(tool::<RotateArgs, PagesReport>(
            "laminapdf_pages_rotate",
            description,
            annotations,
        ));
    }
    tools
}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("laminapdf", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(self.tools.to_vec())))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let result = self.call(&request.name, request.arguments.unwrap_or_default()).await?;
        Ok(result.into())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_tools_should_be_in_name_order_and_read_only_by_default() {
        let names = |tools: &[Tool]| tools.iter().map(|t| t.name.to_string()).collect::<Vec<_>>();
        assert_eq!(
            names(&tools(false, false)),
            [
                "laminapdf_check",
                "laminapdf_explain",
                "laminapdf_extract",
                "laminapdf_info"
            ]
        );
        let write = tools(true, false);
        assert_eq!(names(&write).last().map(String::as_str), Some("laminapdf_pages_rotate"));
        let annotations = |tools: &[Tool]| {
            tools
                .last()
                .and_then(|t| t.annotations.clone())
                .and_then(|a| a.destructive_hint)
        };
        assert_eq!(
            (annotations(&write), annotations(&tools(true, true))),
            (Some(false), Some(true))
        );
    }

    #[test]
    fn a_job_past_the_timeout_should_be_cancelled_and_waited_for() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cancel = CancelToken::new();
        let seen = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (token, flag) = (cancel.clone(), Arc::clone(&seen));
        // A job that never ends on its own: only the timeout's cancel stops it.
        let job = move || {
            while !token.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok::<(), Failure>(())
        };
        let outcome = runtime.block_on(within(Duration::from_millis(20), &cancel, job));
        assert!(outcome.is_none());
        assert!(
            seen.load(std::sync::atomic::Ordering::SeqCst),
            "within() returned before the job stopped"
        );
        let quick = runtime.block_on(within(Duration::from_secs(5), &CancelToken::new(), || {
            Ok::<u8, Failure>(7)
        }));
        assert!(matches!(quick, Some(Ok(7))));
    }

    #[test]
    fn output_schemas_should_need_no_other_schema() {
        for tool in tools(true, true) {
            let schema = Value::Object((*tool.output_schema.clone().unwrap_or_default()).clone()).to_string();
            assert!(!schema.contains("error.schema.json#"), "{}", tool.name);
        }
        let info = schema::<InfoReport>();
        assert!(info["$defs"].get("warning").is_some() && info["$defs"].get("PageSummary").is_some());
    }

    #[test]
    fn a_report_over_the_limit_should_be_cut_and_marked() {
        let report = json!({ "schema": "laminapdf.text/1", "pages": [
            { "text": "a".repeat(400) }, { "text": "b".repeat(400) }, { "text": "c".repeat(400) }
        ] });
        let cut = truncate(report.clone(), 300);
        assert!(cut.to_string().len() <= 300, "{cut}");
        assert_eq!(
            (cut["truncated"].as_bool(), cut["schema"].as_str()),
            (Some(true), Some("laminapdf.text/1"))
        );
        assert_eq!(truncate(report.clone(), 10_000), report);
    }
}
