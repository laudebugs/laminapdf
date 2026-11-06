//! The `laminapdf` binary ([ADR 0031]): parses the command line with clap, runs the command
//! from [`laminapdf_cli::commands`], prints its report, and exits with its code.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

mod docs;
mod many;
mod render;

use std::io::{IsTerminal as _, Write as _};
use std::process::ExitCode as ProcessExit;

use anstream::ColorChoice;
use clap::{Args, CommandFactory as _, Parser, Subcommand, ValueEnum};
use laminapdf::{Diagnostic, ErrorCode};
use laminapdf_cli::commands::{
    self, CheckArgs, CheckReport, ExplainArgs, ExplainReport, ExtractArgs, ExtractedReport, Format, InfoArgs,
    InfoReport, OpenArgs, PagesReport, RenderArgs, RenderReport, RotateArgs, Settings, TextReport,
};
use laminapdf_cli::exit::{CliError, ExitCode};
use laminapdf_cli::io::{Io, ProcessIo, STDIO};
use many::Many;
use render::Streams;
use serde_json::{Value, json};

/// The environment variable a password can come from; never the command line.
const PASSWORD_VAR: &str = "LAMINA_PDF_PASSWORD";

const AFTER_HELP: &str = "\
Data goes to standard output, everything else to standard error. With --json, a report lists \
its warnings, and a failure is one JSON line on standard error with standard output empty. \
`laminapdf help exit-codes` lists the exit codes. Pages count from 1; a range is like 1-5,z \
(the last page),r2 (the second from the end),x3 (all but page 3), with :odd or :even at the end.";

/// Read, check and change PDFs.
#[derive(Debug, Parser)]
#[command(name = "laminapdf", disable_help_subcommand = true, disable_version_flag = true, after_help = AFTER_HELP)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,
    #[command(subcommand)]
    command: Option<Command>,
}

/// The flags every command takes.
#[derive(Debug, Args)]
#[expect(clippy::struct_excessive_bools, reason = "each is a command-line switch")]
struct GlobalArgs {
    /// Print the report as JSON on standard output, and a failure as one JSON line on standard error.
    #[arg(long, global = true)]
    json: bool,
    /// Print no warnings (the exit code still says 3).
    #[arg(short, long, global = true)]
    quiet: bool,
    /// Never wait for input: laminapdf never prompts, so this is accepted for scripts that pass it everywhere.
    #[arg(long, global = true)]
    no_input: bool,
    /// Replace existing files, and allow --in-place.
    #[arg(short, long, global = true)]
    force: bool,
    /// Do everything but write the output.
    #[arg(short = 'n', long, global = true)]
    dry_run: bool,
    /// When to colour the output.
    #[arg(long, global = true, value_enum, default_value_t = Color::Auto)]
    color: Color,
    /// A file holding the password for encrypted documents (or set `LAMINA_PDF_PASSWORD`).
    #[arg(long, global = true, value_name = "FILE")]
    password_file: Option<String>,
    /// Exit 0, not 3, when a command succeeds with warnings.
    #[arg(long, global = true)]
    warnings_ok: bool,
    /// Print the version (with --json, the CLI, core and schema versions).
    #[arg(short = 'V', long, global = true)]
    version: bool,
    #[command(flatten)]
    open: OpenArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// What a PDF is: version, pages, size, encryption, tagging, metadata and warnings.
    Info(Many<InfoArgs>),
    /// Read a whole PDF and report what a tolerant read had to repair (exit 3 on warnings).
    Check(Many<CheckArgs>),
    /// Change pages.
    #[command(subcommand)]
    Pages(PagesCommand),
    /// A PDF's text page by page, or its blocks in reading order as Markdown, JSON or chunks.
    Extract(Many<ExtractArgs>),
    /// Pages as PNG images.
    Render(Many<RenderArgs>),
    /// The page of an error or warning code.
    Explain(ExplainArgs),
    /// The JSON Schema of a command's arguments and output, or the list of schemas.
    Schema(docs::SchemaArgs),
    /// A shell completion script, on standard output.
    Completions(docs::CompletionsArgs),
    /// Writes a man page per command into a folder (for packaging).
    #[command(hide = true)]
    ManPages(docs::ManPagesArgs),
    /// Help on a command, or on a topic: exit-codes.
    Help(HelpArgs),
    /// Serve the commands as MCP tools on stdin and stdout, for clients that speak only MCP.
    #[cfg(feature = "mcp")]
    Mcp(laminapdf_cli::mcp::McpArgs),
    /// Serve the commands as MCP tools (this build has no MCP server).
    #[cfg(not(feature = "mcp"))]
    #[command(hide = true)]
    Mcp {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
        rest: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum PagesCommand {
    /// Turn pages: +90 or -90 from their rotation, or 90 to set it, with :RANGE for some pages.
    Rotate(RotateArgs),
}

#[derive(Debug, Args)]
struct HelpArgs {
    /// A command, or `exit-codes`.
    topic: Option<String>,
}

/// A command's outcome: its report, the warnings in it, and where it goes.
#[derive(Debug)]
struct Outcome {
    /// The report as JSON (`null` for help).
    json: Value,
    /// The report as text.
    text: String,
    /// The warnings it carries.
    warnings: Vec<Diagnostic>,
    /// Whether standard output already carries the command's data (a PDF written to `-`), so
    /// the report goes to standard error.
    stdout_taken: bool,
}

impl Outcome {
    /// A report printed on standard output.
    const fn new(json: Value, text: String, warnings: Vec<Diagnostic>) -> Self {
        Self {
            json,
            text,
            warnings,
            stdout_taken: false,
        }
    }
}

fn main() -> ProcessExit {
    let wants_json = std::env::args_os().any(|a| a == "--json");
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return parse_error(&error, wants_json),
    };
    let choice = match cli.global.color {
        Color::Auto => ColorChoice::Auto,
        Color::Always => ColorChoice::Always,
        Color::Never => ColorChoice::Never,
    };
    let mut streams = Streams::new(choice);
    let exit = run(&cli, &mut streams);
    ProcessExit::from(exit.number())
}

/// A command line clap refused: its own message as text, or an `invalid-command-line` line.
fn parse_error(error: &clap::Error, json: bool) -> ProcessExit {
    use clap::error::ErrorKind;
    if matches!(error.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
        let _ = error.print();
        return ProcessExit::SUCCESS;
    }
    let mut streams = Streams::new(ColorChoice::Auto);
    if json {
        let cli_error = CliError::new(ErrorCode::InvalidCommandLine, "the command line doesn't parse")
            .with("detail", format!("{:?}", error.kind()));
        render::error(&mut streams, &cli_error, None, true);
    } else {
        let text = error.render().to_string();
        let text = text.strip_prefix("error: ").unwrap_or(&text);
        let _ = write!(streams.err, "error[invalid-command-line]: {text}");
    }
    ProcessExit::from(ExitCode::Usage.number())
}

/// Runs the command and prints its outcome; the exit code.
fn run(cli: &Cli, streams: &mut Streams) -> ExitCode {
    let global = &cli.global;
    if global.version {
        return version(streams, global.json);
    }
    let Some(command) = &cli.command else {
        let _ = write!(streams.err, "{}", Cli::command().render_help());
        return ExitCode::Usage;
    };
    let mut io = ProcessIo::default();
    let settings = match settings(global, &mut io) {
        Ok(settings) => settings,
        Err(error) => {
            render::error(streams, &error, None, global.json);
            return error.exit_code();
        }
    };
    match command {
        Command::Info(many) => return each(streams, global, many, &settings, &mut io, InfoReport::SCHEMA, info),
        Command::Check(many) => return each(streams, global, many, &settings, &mut io, CheckReport::SCHEMA, check),
        Command::Render(many) => {
            return each(
                streams,
                global,
                many,
                &settings,
                &mut io,
                RenderReport::SCHEMA,
                render_pages,
            );
        }
        Command::Extract(many) => {
            let schema = if many.args.format == Format::Text {
                TextReport::SCHEMA
            } else {
                ExtractedReport::SCHEMA
            };
            return each(streams, global, many, &settings, &mut io, schema, extract);
        }
        _ => {}
    }
    let input = input_of(command);
    let outcome = execute(command, &settings, &mut io);
    finish(streams, global, outcome, input)
}

/// A command's arguments for one of its inputs.
trait ForInput: Clone {
    /// These arguments with `input`, the command's input number `index`, as the input.
    fn for_input(&self, input: &str, index: usize) -> Self;

    /// Refuses, before any input is read, what can't work for these `inputs` together.
    fn check_inputs(&self, inputs: &[String]) -> Result<(), CliError> {
        let _ = inputs;
        Ok(())
    }
}

macro_rules! for_input {
    ($($ty:ty),*) => {$(
        impl ForInput for $ty {
            fn for_input(&self, input: &str, _: usize) -> Self {
                let mut args = self.clone();
                input.clone_into(&mut args.input);
                args
            }
        }
    )*};
}

for_input!(InfoArgs, CheckArgs, ExtractArgs);

impl ForInput for RenderArgs {
    fn for_input(&self, input: &str, index: usize) -> Self {
        let mut args = self.clone();
        input.clone_into(&mut args.input);
        args.index = index;
        args
    }

    fn check_inputs(&self, inputs: &[String]) -> Result<(), CliError> {
        commands::check_outputs(self, inputs)
    }
}

/// Runs `one` on each input of `many`: one input prints as before, several as a batch.
fn each<A, F>(
    streams: &mut Streams,
    global: &GlobalArgs,
    many: &Many<A>,
    settings: &Settings,
    io: &mut ProcessIo,
    schema: &str,
    one: F,
) -> ExitCode
where
    A: Args + ForInput + Sync,
    F: Fn(&A, &Settings, &mut dyn Io) -> Result<Outcome, CliError> + Sync,
{
    let inputs = match many
        .expand(io)
        .and_then(|inputs| many.args.check_inputs(&inputs).map(|()| inputs))
    {
        Ok(inputs) => inputs,
        Err(error) => {
            render::error(streams, &error, None, global.json);
            return error.exit_code();
        }
    };
    // A list may hold one file or many, so a list always prints as a batch.
    if let ([input], None) = (inputs.as_slice(), &many.batch.files_from) {
        let outcome = one(&many.args.for_input(input, 0), settings, io);
        return finish(streams, global, outcome, Some(input));
    }
    many::run(streams, global, &many.batch, &inputs, schema, |index, input| {
        one(&many.args.for_input(input, index), settings, &mut ProcessIo::default())
    })
}

fn info(args: &InfoArgs, settings: &Settings, io: &mut dyn Io) -> Result<Outcome, CliError> {
    let report = commands::info(args, settings, io)?;
    let json = render::keep_fields(to_json(&report), &args.fields)?;
    Ok(Outcome::new(json, render::info_text(&report), report.warnings))
}

fn check(args: &CheckArgs, settings: &Settings, io: &mut dyn Io) -> Result<Outcome, CliError> {
    let report = commands::check(args, settings, io)?;
    Ok(Outcome::new(
        to_json(&report),
        render::check_text(&report),
        report.warnings,
    ))
}

fn render_pages(args: &RenderArgs, settings: &Settings, io: &mut dyn Io) -> Result<Outcome, CliError> {
    let report = commands::render(args, settings, io)?;
    let mut outcome = Outcome::new(to_json(&report), render::render_text(&report), Vec::new());
    // A PNG on standard output: the report goes to standard error.
    outcome.stdout_taken = report.pages.iter().any(|p| p.output == STDIO) && !settings.dry_run;
    Ok(outcome)
}

fn extract(args: &ExtractArgs, settings: &Settings, io: &mut dyn Io) -> Result<Outcome, CliError> {
    if args.format == Format::Text {
        let report = commands::extract(args, settings, io)?;
        return Ok(Outcome::new(
            to_json(&report),
            render::extract_text(&report),
            report.warnings,
        ));
    }
    let report = commands::extract_document(args, settings, io)?;
    Ok(Outcome::new(report.json, report.output, report.warnings))
}

/// Prints the outcome of a command over one input; the exit code.
fn finish(
    streams: &mut Streams,
    global: &GlobalArgs,
    outcome: Result<Outcome, CliError>,
    input: Option<&str>,
) -> ExitCode {
    match outcome {
        Ok(outcome) => match print(streams, &outcome, global) {
            Ok(()) if outcome.warnings.is_empty() || global.warnings_ok => ExitCode::Ok,
            Ok(()) => ExitCode::Warnings,
            // The data didn't arrive (a full disk, a closed pipe): not a success.
            Err(error) => {
                let error = CliError::new(ErrorCode::OutputUnwritable, "couldn't write the output")
                    .with("output", STDIO)
                    .with("reason", format!("{:?}", error.kind()));
                render::error(streams, &error, input, global.json);
                error.exit_code()
            }
        },
        Err(error) => {
            render::error(streams, &error, input, global.json);
            error.exit_code()
        }
    }
}

/// The input a command reads, for the error's `input` field.
const fn input_of(command: &Command) -> Option<&str> {
    match command {
        Command::Pages(PagesCommand::Rotate(a)) => Some(a.input.as_str()),
        _ => None,
    }
}

/// The settings, with the password from `--password-file` or `LAMINA_PDF_PASSWORD`.
fn settings(global: &GlobalArgs, io: &mut ProcessIo) -> Result<Settings, CliError> {
    let password = match &global.password_file {
        Some(file) if file == STDIO => {
            return Err(CliError::new(
                ErrorCode::InvalidCommandLine,
                "--password-file needs a file, not standard input",
            ));
        }
        Some(file) => {
            let bytes = io.read(file)?;
            let text = String::from_utf8(bytes).map_err(|_| {
                CliError::new(ErrorCode::InputUnreadable, "the password file isn't UTF-8 text")
                    .with("input", file.as_str())
                    .with("reason", "InvalidData")
            })?;
            // One line: the newline an editor or `echo` adds isn't part of the password.
            let line = text.strip_suffix('\n').unwrap_or(&text);
            Some(line.strip_suffix('\r').unwrap_or(line).to_owned())
        }
        None => std::env::var(PASSWORD_VAR).ok(),
    };
    Ok(Settings {
        open: global.open.clone(),
        password,
        force: global.force,
        dry_run: global.dry_run,
        ..Settings::default()
    })
}

fn to_json(report: &impl serde::Serialize) -> Value {
    serde_json::to_value(report).unwrap_or_default()
}

/// Runs `command`.
fn execute(command: &Command, settings: &Settings, io: &mut ProcessIo) -> Result<Outcome, CliError> {
    let outcome = |json, text, warnings, stdout_taken| Outcome {
        json,
        text,
        warnings,
        stdout_taken,
    };
    Ok(match command {
        // `run` sends these to `each`, which runs them per input.
        Command::Info(_) | Command::Check(_) | Command::Extract(_) | Command::Render(_) => {
            return Err(CliError::new(
                ErrorCode::InvalidCommandLine,
                "this command takes its inputs one by one",
            ));
        }
        Command::Pages(PagesCommand::Rotate(args)) => {
            let report = commands::rotate(args, settings, io)?;
            let taken = report.output == STDIO && !settings.dry_run;
            outcome(to_json(&report), render::pages_text(&report), report.warnings, taken)
        }
        Command::Explain(args) => {
            let report = commands::explain(args)?;
            outcome(to_json(&report), render::explain_text(&report), Vec::new(), false)
        }
        Command::Help(args) => outcome(Value::Null, help(args)?, Vec::new(), false),
        Command::Schema(args) => {
            let schema = docs::schema(args)?;
            outcome(
                schema.clone(),
                format!("{}\n", serde_json::to_string_pretty(&schema).unwrap_or_default()),
                Vec::new(),
                false,
            )
        }
        Command::Completions(args) => outcome(Value::Null, docs::completions(args, Cli::command()), Vec::new(), false),
        Command::ManPages(args) => {
            docs::man_pages(args, &Cli::command())?;
            outcome(Value::Null, String::new(), Vec::new(), false)
        }
        // Serves until the client closes standard input; the protocol had standard output.
        #[cfg(feature = "mcp")]
        Command::Mcp(args) => {
            laminapdf_cli::mcp::run(args, settings)?;
            outcome(Value::Null, String::new(), Vec::new(), true)
        }
        #[cfg(not(feature = "mcp"))]
        Command::Mcp { .. } => {
            return Err(CliError::new(
                ErrorCode::InvalidCommandLine,
                "this laminapdf was built without the MCP server (the mcp feature)",
            ));
        }
    })
}

/// `laminapdf help [topic]`.
fn help(args: &HelpArgs) -> Result<String, CliError> {
    let mut command = Cli::command();
    // Building propagates the global flags to the subcommands, so their help lists them.
    command.build();
    let Some(topic) = args.topic.as_deref() else {
        return Ok(command.render_help().to_string());
    };
    if topic == "exit-codes" {
        return Ok(render::exit_codes_text());
    }
    if let Some(sub) = command.find_subcommand_mut(topic) {
        return Ok(sub.render_help().to_string());
    }
    let mut known: Vec<String> = command.get_subcommands().map(|c| c.get_name().to_owned()).collect();
    known.push("exit-codes".to_owned());
    Err(
        CliError::new(ErrorCode::InvalidCommandLine, "no such command or help topic")
            .with("argument", topic)
            .with_did_you_mean(laminapdf::did_you_mean(topic, &known)),
    )
}

/// Prints a successful outcome: the report on standard output (standard error when standard
/// output carries the PDF), and in text mode its warnings on standard error unless `--quiet`.
/// Text for a terminal loses its control characters ([`render::for_terminal`]).
///
/// # Errors
///
/// Writing standard output failed.
fn print(streams: &mut Streams, outcome: &Outcome, global: &GlobalArgs) -> std::io::Result<()> {
    if global.json && !outcome.json.is_null() {
        let line = format!("{}\n", outcome.json);
        if outcome.stdout_taken {
            let _ = streams.err.write_all(line.as_bytes());
            return Ok(());
        }
        streams.out.write_all(line.as_bytes())?;
        return streams.out.flush();
    }
    if !global.quiet {
        render::warnings(streams, &outcome.warnings);
    }
    if outcome.stdout_taken {
        if !global.quiet {
            let _ = streams.err.write_all(render::for_terminal(&outcome.text).as_bytes());
        }
        return Ok(());
    }
    let text = if streams.out.is_terminal() {
        render::for_terminal(&outcome.text)
    } else {
        outcome.text.clone()
    };
    streams.out.write_all(text.as_bytes())?;
    streams.out.flush()
}

/// `--version`: the version, or with `--json` the CLI's, the core's and the schema ids.
fn version(streams: &mut Streams, json: bool) -> ExitCode {
    // One lockstep version for every crate (ADR 0017), so the core's is the CLI's.
    let version = env!("CARGO_PKG_VERSION");
    let text = if json {
        let schemas = [
            InfoReport::SCHEMA,
            RenderReport::SCHEMA,
            CheckReport::SCHEMA,
            PagesReport::SCHEMA,
            TextReport::SCHEMA,
            ExplainReport::SCHEMA,
            "laminapdf.error/1",
        ];
        let report = json!({ "schema": "laminapdf.version/1", "cli": version, "core": version, "schemas": schemas });
        format!("{report}\n")
    } else {
        format!("laminapdf {version}\n")
    };
    match streams
        .out
        .write_all(text.as_bytes())
        .and_then(|()| streams.out.flush())
    {
        Ok(()) => ExitCode::Ok,
        Err(_) => ExitCode::Failure,
    }
}
