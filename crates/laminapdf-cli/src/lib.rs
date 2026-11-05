//! The `laminapdf` command-line tool's command layer ([ADR 0031]): the commands as functions,
//! the page-range grammar, exit codes and the CLI's errors, and the input and output they go
//! through. The `laminapdf` binary is a clap front end over [`commands`]; `laminapdf mcp`
//! will call the same functions.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

pub mod batch;
pub mod commands;
pub mod exit;
pub mod io;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod ranges;
pub mod schema;
