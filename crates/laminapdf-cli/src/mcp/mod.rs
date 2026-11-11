//! `laminapdf mcp` ([ADR 0031] §9): the commands as the tools of an MCP server on stdio.
//!
//! The server is a thin adapter. Each tool parses its arguments into the command's own
//! argument type, runs the same `commands::*` function the CLI runs, and returns the same report
//! (the tool's `outputSchema` is the report's schema). What makes it safe without
//! opt-in flags:
//!
//! - **Roots:** every path goes through [`roots::Roots`]; nothing outside the `--root` folders is
//!   read or written.
//! - **Read-only by default:** the write tool is listed only with `--allow-write`, and it
//!   replaces its input only with `--allow-in-place` too.
//! - **Document text only in data:** reports keep it in their fields; errors carry the ADR 0026
//!   object (as text: the `outputSchema` is the report's), whose message and hint never quote the
//!   document; tool descriptions are fixed text.
//! - **Bounded calls:** each runs on a blocking thread, at most one per CPU at once. When it
//!   runs past `--timeout`, its [`CancelToken`] stops the command at its next step. The server
//!   answers once the command has stopped, or after a short grace when it is in a step that
//!   can't be interrupted (opening a file, reading every object); that call keeps its share of
//!   the CPU limit until the step ends.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md
//! [`CancelToken`]: laminapdf::CancelToken

pub mod roots;
mod tools;
mod truncate;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use laminapdf::{CancelToken, ErrorCode};
use rmcp::ServiceExt as _;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, ServerHandler};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::commands::{
    self, BuildToolArgs, CheckArgs, ExplainArgs, ExtractArgs, Format, InfoArgs, RenderArgs, RevealArgs, RevealToolArgs,
    RotateArgs, Settings,
};
use crate::exit::CliError;
use roots::{RootedIo, Roots};

/// `laminapdf mcp`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct McpArgs {
    /// A folder the tools may read and write; repeat for more. Relative paths start at the first.
    #[arg(long = "root", value_name = "DIR", required = true)]
    pub roots: Vec<PathBuf>,
    /// List the write tools (`laminapdf_build`, `laminapdf_pages_rotate`, `laminapdf_render`), which write new files only.
    #[arg(long)]
    pub allow_write: bool,
    /// Let the write tool replace its input (`inPlace`); needs --allow-write.
    #[arg(long, requires = "allow_write")]
    pub allow_in_place: bool,
    /// Cut a report down to at most this many bytes of JSON (1 KiB or more), marking it
    /// `"truncated": true`.
    #[arg(long, value_name = "N", default_value_t = 1 << 20, value_parser = clap::value_parser!(u64).range(1024..))]
    pub max_output_bytes: u64,
    /// Stop a call after this many seconds (1 or more); it ends as a `cancelled` tool error.
    #[arg(long, value_name = "SECS", default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: u64,
}

/// What the server tells clients it is for.
const INSTRUCTIONS: &str = "Lamina PDF's command-line tools as MCP tools: read a PDF's facts, check it, \
extract its text, explain an error code, and (when the server allows writing) rotate its pages. \
Paths are files under the server's root folders.";

/// How long a cancelled call may take to stop before the server answers anyway.
const GRACE: Duration = Duration::from_secs(5);

/// Runs the server on stdin and stdout until the client closes them. Every call opens documents
/// with `settings`: the strict mode, limits and password the command line gave.
///
/// # Errors
///
/// `invalid-command-line` for a bad `--root` or `--force`; `output-unwritable` when the
/// transport fails.
pub fn run(args: &McpArgs, settings: &Settings) -> Result<(), CliError> {
    let server = Server::new(args, settings)?;
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
    settings: Settings,
    allow_in_place: bool,
    max_output_bytes: usize,
    timeout: Duration,
    permits: Arc<Semaphore>,
    tools: Arc<Vec<Tool>>,
}

impl Server {
    /// A server for `args`, opening documents with `settings`.
    ///
    /// # Errors
    ///
    /// `invalid-command-line` for a bad `--root`, or `--force`, which would let the write tool
    /// replace files the server doesn't allow it to.
    pub fn new(args: &McpArgs, settings: &Settings) -> Result<Self, CliError> {
        if settings.force {
            return Err(CliError::new(
                ErrorCode::InvalidCommandLine,
                "--force doesn't apply to laminapdf mcp: --allow-in-place lets the write tool replace its input",
            ));
        }
        let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        Ok(Self {
            roots: Arc::new(Roots::new(&args.roots)?),
            settings: settings.clone(),
            allow_in_place: args.allow_in_place,
            max_output_bytes: usize::try_from(args.max_output_bytes).unwrap_or(usize::MAX),
            timeout: Duration::from_secs(args.timeout),
            permits: Arc::new(Semaphore::new(cpus)),
            tools: Arc::new(tools::tools(args.allow_write, args.allow_in_place)),
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
        // The semaphore is never closed, so this waits for a permit and never fails.
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(|_| ErrorData::internal_error("the server is shutting down", None))?;
        // A token of the call's own: cancelling one call never stops another.
        let settings = Settings {
            cancel: CancelToken::new(),
            ..self.settings.clone()
        };
        let cancel = settings.cancel.clone();
        let (name, roots) = (name.to_owned(), Arc::clone(&self.roots));
        let (allow_in_place, max) = (self.allow_in_place, self.max_output_bytes);
        let job = move || {
            run_tool(&name, arguments, settings, &roots, allow_in_place).map(|report| truncate::truncate(report, max))
        };
        let outcome = within(self.timeout, GRACE, &cancel, permit, job)
            .await
            .unwrap_or_else(|| {
                let error = CliError::new(ErrorCode::Cancelled, "the call ran longer than the server's --timeout");
                Err((error, None))
            });
        Ok(match outcome {
            Ok(report) => CallToolResult::structured(report),
            // Text only: `structuredContent` must match the tool's `outputSchema`, which is the
            // report's, and some clients reject a call whose error object doesn't.
            Err((error, input)) => {
                CallToolResult::error(vec![ContentBlock::text(error.to_line(input.as_deref()).to_string())])
            }
        })
    }
}

/// A tool's failure and the input it was reading, for the error line.
type Failure = (CliError, Option<String>);

/// Runs `job` on a blocking thread, holding `permit`; `None` when it runs past `timeout` and
/// stops as cancelled. Past `timeout`, `cancel` stops it at its next step and this waits up to
/// `grace` for that; a job that still runs is left to finish in the background, still holding
/// `permit`. A job that finished anyway (its last step was past the cancel) keeps its result.
async fn within<T: Send + 'static>(
    timeout: Duration,
    grace: Duration,
    cancel: &CancelToken,
    permit: OwnedSemaphorePermit,
    job: impl FnOnce() -> Result<T, Failure> + Send + 'static,
) -> Option<Result<T, Failure>> {
    let mut handle = tokio::task::spawn_blocking(job);
    if let Ok(joined) = tokio::time::timeout(timeout, &mut handle).await {
        return Some(flatten(joined));
    }
    cancel.cancel();
    let Ok(joined) = tokio::time::timeout(grace, &mut handle).await else {
        tokio::spawn(async move {
            let _ = handle.await;
            drop(permit);
        });
        return None;
    };
    match flatten(joined) {
        Err((error, _)) if error.diagnostic().code == ErrorCode::Cancelled.as_str() => None,
        finished => Some(finished),
    }
}

/// A blocking job's outcome; a job that panicked (the commands return errors instead) ends the
/// call as cancelled.
fn flatten<T>(joined: Result<Result<T, Failure>, tokio::task::JoinError>) -> Result<T, Failure> {
    joined.unwrap_or_else(|_| {
        Err((
            CliError::new(ErrorCode::Cancelled, "the call stopped unexpectedly"),
            None,
        ))
    })
}

/// Runs one tool on a blocking thread.
fn run_tool(
    name: &str,
    arguments: Map<String, Value>,
    mut settings: Settings,
    roots: &Roots,
    allow_in_place: bool,
) -> Result<Value, Failure> {
    let mut io = RootedIo::new(roots);
    // Each call writes its own files: the paths one call claims never refuse another's.
    settings.claimed = Arc::default();
    match name {
        "laminapdf_build" => {
            let args: BuildToolArgs = parse(arguments)?;
            let input = args.input.clone();
            report(commands::build_tool(args, &settings, &mut io), input.as_deref())
        }
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
            if args.format == Format::Text {
                report(commands::extract(&args, &settings, &mut io), Some(&args.input))
            } else {
                report(commands::extract_document(&args, &settings, &mut io), Some(&args.input))
            }
        }
        "laminapdf_info" => {
            let args: InfoArgs = parse(arguments)?;
            report(commands::info(&args, &settings, &mut io), Some(&args.input))
        }
        "laminapdf_render" => {
            let args: RenderArgs = parse(arguments)?;
            report(commands::render(&args, &settings, &mut io), Some(&args.input))
        }
        "laminapdf_reveal" => {
            let args = RevealArgs::from(parse::<RevealToolArgs>(arguments)?);
            report(commands::reveal(&args, &settings, &mut io), Some(&args.input))
        }
        // `call` checked the name against the listed tools; the rotate tool is the one left.
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
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn cancelled() -> Failure {
        (CliError::new(ErrorCode::Cancelled, "cancelled"), None)
    }

    #[test]
    fn a_job_past_the_timeout_should_be_cancelled_and_waited_for() {
        let runtime = runtime();
        let permits = Arc::new(Semaphore::new(1));
        let cancel = CancelToken::new();
        let stopped = Arc::new(AtomicBool::new(false));
        let (token, flag) = (cancel.clone(), Arc::clone(&stopped));
        // A job that never ends on its own: only the timeout's cancel stops it.
        let job = move || {
            while !token.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            flag.store(true, Ordering::SeqCst);
            Err::<(), Failure>(cancelled())
        };
        let outcome = runtime.block_on(async {
            let permit = Arc::clone(&permits).acquire_owned().await.unwrap();
            within(Duration::from_millis(20), Duration::from_secs(5), &cancel, permit, job).await
        });
        assert!(outcome.is_none());
        assert!(
            stopped.load(Ordering::SeqCst),
            "within() returned before the job stopped"
        );
        assert_eq!(permits.available_permits(), 1);
    }

    #[test]
    fn a_job_that_ignores_the_cancel_should_be_answered_after_the_grace_and_keep_its_permit() {
        let runtime = runtime();
        let permits = Arc::new(Semaphore::new(1));
        let job = || {
            std::thread::sleep(Duration::from_millis(300));
            Ok::<u8, Failure>(1)
        };
        runtime.block_on(async {
            let permit = Arc::clone(&permits).acquire_owned().await.unwrap();
            let outcome = within(
                Duration::from_millis(10),
                Duration::from_millis(10),
                &CancelToken::new(),
                permit,
                job,
            )
            .await;
            assert!(outcome.is_none());
            assert_eq!(permits.available_permits(), 0, "the call still holds its permit");
            let _again = Arc::clone(&permits).acquire_owned().await.unwrap();
        });
    }

    #[test]
    fn a_job_that_finishes_after_the_cancel_should_keep_its_result() {
        let runtime = runtime();
        let permits = Arc::new(Semaphore::new(1));
        let job = || {
            std::thread::sleep(Duration::from_millis(50));
            Ok::<u8, Failure>(7)
        };
        let outcome = runtime.block_on(async {
            let permit = Arc::clone(&permits).acquire_owned().await.unwrap();
            within(
                Duration::from_millis(5),
                Duration::from_secs(5),
                &CancelToken::new(),
                permit,
                job,
            )
            .await
        });
        assert!(matches!(outcome, Some(Ok(7))));
    }

    #[test]
    fn force_should_be_refused() {
        let args = McpArgs {
            roots: vec![std::env::temp_dir()],
            allow_write: true,
            allow_in_place: false,
            max_output_bytes: 1 << 20,
            timeout: 60,
        };
        let settings = Settings {
            force: true,
            ..Settings::default()
        };
        let error = Server::new(&args, &settings).unwrap_err();
        assert_eq!(error.diagnostic().code, "invalid-command-line");
    }
}
