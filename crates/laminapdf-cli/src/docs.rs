//! The commands that describe the CLI itself: `schema` (from the same types the commands use,
//! so agents and `laminapdf mcp` never drift from them), `completions` (clap_complete) and the
//! hidden `man-pages` that `cargo xtask cli-docs` runs (clap_mangen).

use std::path::Path;

use clap::{Args, Command};
use clap_complete::Shell;
use laminapdf::ErrorCode;
use laminapdf_cli::exit::CliError;
use laminapdf_cli::schema;
use serde_json::Value;

/// `laminapdf schema [command]`.
#[derive(Debug, Args)]
pub(crate) struct SchemaArgs {
    /// The command (check, explain, extract, info, pages-rotate); the list of schemas without one.
    command: Option<String>,
}

/// The schema `laminapdf schema` prints.
pub(crate) fn schema(args: &SchemaArgs) -> Result<Value, CliError> {
    let Some(name) = args.command.as_deref() else {
        return Ok(schema::index());
    };
    schema::command(name).ok_or_else(|| {
        let known: Vec<String> = schema::COMMANDS.iter().map(|&c| c.to_owned()).collect();
        CliError::new(ErrorCode::InvalidCommandLine, "no schema for that command")
            .with("argument", name)
            .with_did_you_mean(laminapdf::did_you_mean(name, &known))
    })
}

/// `laminapdf completions <shell>`.
#[derive(Debug, Args)]
pub(crate) struct CompletionsArgs {
    /// The shell.
    #[arg(value_enum)]
    shell: Shell,
}

/// The completion script for `args.shell`.
pub(crate) fn completions(args: &CompletionsArgs, mut command: Command) -> String {
    let mut out = Vec::new();
    clap_complete::generate(args.shell, &mut command, "laminapdf", &mut out);
    String::from_utf8_lossy(&out).into_owned()
}

/// `laminapdf man-pages <folder>`.
#[derive(Debug, Args)]
pub(crate) struct ManPagesArgs {
    /// The folder to write `laminapdf.1` and one `laminapdf-<command>.1` per command into.
    folder: String,
}

/// Writes the man pages of `command` and its subcommands into `args.folder`.
pub(crate) fn man_pages(args: &ManPagesArgs, command: &Command) -> Result<(), CliError> {
    let folder = Path::new(&args.folder);
    let unwritable = |error: std::io::Error| {
        CliError::new(ErrorCode::OutputUnwritable, "couldn't write a man page")
            .with("output", args.folder.as_str())
            .with("reason", format!("{:?}", error.kind()))
    };
    std::fs::create_dir_all(folder).map_err(unwritable)?;
    let mut command = command.clone();
    command.build();
    write_page(folder, &command, "laminapdf").map_err(unwritable)
}

/// The page of `command` as `<name>.1`, then its subcommands' as `<name>-<sub>.1`.
fn write_page(folder: &Path, command: &Command, name: &str) -> std::io::Result<()> {
    let mut page = Vec::new();
    clap_mangen::Man::new(command.clone()).title(name.to_owned()).render(&mut page)?;
    std::fs::write(folder.join(format!("{name}.1")), page)?;
    for sub in command.get_subcommands().filter(|s| !s.is_hide_set()) {
        write_page(folder, sub, &format!("{name}-{}", sub.get_name()))?;
    }
    Ok(())
}
