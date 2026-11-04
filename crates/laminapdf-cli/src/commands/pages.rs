//! `laminapdf pages rotate`: turns pages and saves the result, incrementally where the file
//! allows (the save planner, ADR 0007). The other page commands come with their ops.

use laminapdf::{Diagnostic, ErrorCode, Op, RotatePage, Rotation, SaveMode, SaveOptions};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, open, pages_of, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::{self, Io, STDIO};

/// `laminapdf pages rotate <input> <angle>[:<range>] (-o OUT | --in-place)`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotateArgs {
    /// The PDF to rotate pages of, or `-` for standard input.
    pub input: String,
    /// The angle and the pages: `+90` or `-90` turn from the current rotation, `90` sets it;
    /// `:1-3,x2` names the pages (every page without it).
    #[arg(allow_hyphen_values = true, value_name = "ANGLE[:RANGE]")]
    pub rotation: String,
    /// The file to write, or `-` for standard output.
    #[arg(short = 'o', long, value_name = "OUT", conflicts_with = "in_place")]
    #[serde(default)]
    pub output: Option<String>,
    /// Replace the input (needs `--force`).
    #[arg(long)]
    #[serde(default)]
    pub in_place: bool,
}

/// `laminapdf.pages/1`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagesReport {
    /// `laminapdf.pages/1`.
    #[schemars(schema_with = "pages_id")]
    pub schema: &'static str,
    /// The input as given: a path, or `-`.
    pub input: String,
    /// Where the result went: a path, or `-`.
    pub output: String,
    /// How it was saved: `incremental` (the input's bytes, then an update) or `fullRewrite`.
    pub save_mode: String,
    /// The bytes written (none with `--dry-run`).
    pub bytes: u64,
    /// The pages that turned, with their new rotation.
    pub pages: Vec<RotatedPage>,
    /// The warnings reading the document raised (ADR 0026 objects).
    #[schemars(schema_with = "warnings_schema")]
    pub warnings: Vec<Diagnostic>,
}

/// A page's new rotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RotatedPage {
    /// The 0-based page index.
    pub index: u32,
    /// The 1-based page number.
    pub number: u32,
    /// `/Rotate` now: 0, 90, 180 or 270.
    pub rotation: u16,
}

impl PagesReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.pages/1";
}

fn pages_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(PagesReport::SCHEMA)
}

/// An angle: turn by it, or set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Angle {
    Turn(i64),
    Set(i64),
}

/// Splits `<angle>[:<range>]` and reads the angle.
fn parse_rotation(text: &str) -> Result<(Angle, Option<&str>), CliError> {
    let (angle, range) = match text.split_once(':') {
        Some((angle, range)) => (angle, Some(range)),
        None => (text, None),
    };
    let (sign, digits) = match angle.as_bytes().first() {
        Some(b'+') => (Some(1), angle.get(1..)),
        Some(b'-') => (Some(-1), angle.get(1..)),
        _ => (None, Some(angle)),
    };
    let degrees = digits
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|d| d.parse::<i64>().ok())
        .filter(|d| d % 90 == 0);
    let bad_angle = || {
        CliError::new(
            ErrorCode::InvalidCommandLine,
            "the angle must be a multiple of 90: +90 or -90 turn a page, 90 sets its rotation",
        )
        .with("angle", angle)
    };
    let degrees = degrees.ok_or_else(bad_angle)?;
    let angle = match sign {
        Some(sign) => Angle::Turn(sign * degrees),
        None if degrees < 360 => Angle::Set(degrees),
        None => return Err(bad_angle()),
    };
    Ok((angle, range))
}

/// Where the result goes: `-o OUT`, or the input with `--in-place` and `--force`.
fn output_of<'a>(args: &'a RotateArgs, settings: &Settings) -> Result<&'a str, CliError> {
    match (&args.output, args.in_place) {
        (Some(output), _) => Ok(output),
        (None, true) if args.input == STDIO => Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "--in-place needs a file input, not standard input",
        )),
        (None, true) if !settings.force => Err(io::refused(&args.input)),
        (None, true) => Ok(&args.input),
        (None, false) => Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "pages rotate needs -o OUT, or --in-place with --force",
        )),
    }
}

/// Rotates the pages `args.rotation` names and writes the result.
///
/// # Errors
///
/// `invalid-command-line` (the angle, or no output), `invalid-page-range`, `overwrite-refused`;
/// the input couldn't be read, opened or saved; the output couldn't be written.
pub fn rotate(args: &RotateArgs, settings: &Settings, io: &mut dyn Io) -> Result<PagesReport, CliError> {
    let (angle, range) = parse_rotation(&args.rotation)?;
    let output = output_of(args, settings)?;
    let mut doc = open(&args.input, settings, io)?.doc;
    let infos = doc.page_infos()?;
    let mut numbers = pages_of(range, u32::try_from(infos.len()).unwrap_or(u32::MAX))?;
    let mut seen = std::collections::BTreeSet::new();
    numbers.retain(|n| seen.insert(*n));
    let mut ops = Vec::with_capacity(numbers.len());
    let mut pages = Vec::with_capacity(numbers.len());
    for number in numbers {
        let index = number.saturating_sub(1);
        let Some(info) = usize::try_from(index).ok().and_then(|i| infos.get(i)) else {
            continue;
        };
        let degrees = match angle {
            Angle::Turn(by) => i64::from(info.rotation) + by,
            Angle::Set(to) => to,
        };
        let rotation = Rotation::from_degrees(degrees).unwrap_or(Rotation::None);
        ops.push(Op::RotatePage(RotatePage::new(info.id.clone(), rotation)));
        pages.push(RotatedPage {
            index,
            number,
            rotation: rotation.degrees(),
        });
    }
    doc.apply(ops)?;
    let saved = doc.save(&SaveOptions::default())?;
    if !settings.dry_run {
        io.write(output, &saved.bytes, settings.force || args.in_place)?;
    }
    Ok(PagesReport {
        schema: PagesReport::SCHEMA,
        input: args.input.clone(),
        output: output.to_owned(),
        save_mode: match saved.report.mode() {
            SaveMode::Incremental => "incremental",
            _ => "fullRewrite",
        }
        .to_owned(),
        bytes: if settings.dry_run {
            0
        } else {
            u64::try_from(saved.bytes.len()).unwrap_or(u64::MAX)
        },
        pages,
        warnings: warnings(&doc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signed_angle_should_turn_and_a_bare_one_should_set() {
        assert_eq!(parse_rotation("+90:1-3").unwrap(), (Angle::Turn(90), Some("1-3")));
        assert_eq!(parse_rotation("-90").unwrap(), (Angle::Turn(-90), None));
        assert_eq!(parse_rotation("180:z:odd").unwrap(), (Angle::Set(180), Some("z:odd")));
    }

    use crate::io::memory::MemoryIo;

    fn io_with(name: &str, path: &str) -> MemoryIo {
        let mut io = MemoryIo::default();
        io.files.insert(name.to_owned(), laminapdf_test_fixtures::fixture(path));
        io
    }

    fn args(rotation: &str, output: Option<&str>, in_place: bool) -> RotateArgs {
        RotateArgs {
            input: "in.pdf".to_owned(),
            rotation: rotation.to_owned(),
            output: output.map(str::to_owned),
            in_place,
        }
    }

    #[test]
    fn rotate_should_turn_the_named_pages_and_write_an_incremental_update() {
        let mut io = io_with("in.pdf", "golden/links-and-outlines.pdf");
        let report = rotate(
            &args("+90:1,z,1", Some("out.pdf"), false),
            &Settings::default(),
            &mut io,
        )
        .unwrap();
        assert_eq!(
            report.pages.iter().map(|p| (p.number, p.rotation)).collect::<Vec<_>>(),
            [(1, 90), (4, 90)]
        );
        assert_eq!(report.save_mode, "incremental");
        let output = io.files.get("out.pdf").unwrap();
        assert!(output.starts_with(io.files.get("in.pdf").unwrap()));
    }

    #[test]
    fn in_place_should_need_force_and_check_before_reading() {
        let mut io = MemoryIo::default();
        let error = rotate(&args("90", None, true), &Settings::default(), &mut io).unwrap_err();
        assert_eq!(error.diagnostic().code, "overwrite-refused");
        let no_output = rotate(&args("90", None, false), &Settings::default(), &mut io).unwrap_err();
        assert_eq!(no_output.diagnostic().code, "invalid-command-line");
    }

    #[test]
    fn a_pdf_should_never_go_to_a_terminal() {
        let mut io = io_with("in.pdf", "corpus/hello.pdf");
        io.stdout_is_terminal = true;
        let error = rotate(&args("90", Some("-"), false), &Settings::default(), &mut io).unwrap_err();
        assert_eq!(error.diagnostic().code, "binary-output-to-terminal");
    }

    #[test]
    fn dry_run_should_write_nothing() {
        let mut io = io_with("in.pdf", "corpus/hello.pdf");
        let settings = Settings {
            dry_run: true,
            ..Settings::default()
        };
        let report = rotate(&args("180", Some("out.pdf"), false), &settings, &mut io).unwrap();
        assert_eq!((report.bytes, io.files.contains_key("out.pdf")), (0, false));
    }

    #[test]
    fn an_angle_off_the_quarter_turns_should_be_refused() {
        for bad in ["45", "+", "90x", "360", "-:1", ""] {
            let error = parse_rotation(bad).unwrap_err();
            assert_eq!(error.diagnostic().code, "invalid-command-line", "{bad}");
        }
    }
}
