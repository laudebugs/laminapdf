//! `laminapdf render`: pages as PNG images, through a [`RenderBackend`] ([ADR 0004]): hayro, the
//! pure-Rust renderer, in every build, and PDFium when `LAMINA_PDF_PDFIUM_DIR` names its library.
//!
//! [ADR 0004]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0004-render-backend.md

use std::collections::BTreeSet;
use std::sync::Arc;

use clap::ValueEnum;
use laminapdf::ErrorCode;
use laminapdf_render::{ByteSource, HayroBackend, RenderBackend, RenderOpenOptions, RenderRequest};
use laminapdf_render_pdfium::PdfiumBackend;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, pages_of, schema_id};
use crate::exit::CliError;
use crate::io::Io;
use crate::template::{Fill, Template};

/// The resolution without `--dpi` or `--scale`.
pub const DEFAULT_DPI: f32 = 150.0;

/// Which engine draws the pages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Backend {
    /// hayro, the pure-Rust renderer, built in.
    #[default]
    Hayro,
    /// PDFium, loaded from the folder `LAMINA_PDF_PDFIUM_DIR` names.
    Pdfium,
}

/// `laminapdf render <input> -o <template>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderArgs {
    /// The PDF to render, or `-` for standard input.
    // The command line takes several inputs, so the front end fills this in per input.
    #[arg(skip)]
    pub input: String,
    /// Where each page goes: a path template with `{stem}`, `{name}`, `{index}` and `{n}` (the
    /// page number; `{n:03}` pads it to three digits), or `-` for standard output (one page).
    #[arg(short, long, value_name = "TEMPLATE")]
    pub output: String,
    /// The pages to render, as a range (`1-3`, `z`, `r2-z`, `1-z,x2`); every page without it.
    #[arg(long, value_name = "RANGE")]
    #[serde(default)]
    pub pages: Option<String>,
    /// Pixels per inch; 150 by default.
    #[arg(long, value_name = "N", conflicts_with = "scale")]
    #[serde(default)]
    pub dpi: Option<f32>,
    /// Pixels per point (1 is 72 dpi), instead of --dpi.
    #[arg(long, value_name = "F")]
    #[serde(default)]
    pub scale: Option<f32>,
    /// The renderer.
    #[arg(long, value_enum, default_value_t)]
    #[serde(default)]
    pub backend: Backend,
    /// The input's position among the command's inputs, for `{index}`.
    #[arg(skip)]
    #[serde(skip)]
    #[schemars(skip)]
    pub index: usize,
}

/// `laminapdf.render/1`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderReport {
    /// `laminapdf.render/1`.
    #[schemars(schema_with = "render_id")]
    pub schema: &'static str,
    /// The input as given: a path, or `-`.
    pub input: String,
    /// The renderer: `hayro` or `pdfium`.
    pub backend: Backend,
    /// Pixels per point (1 is 72 dpi).
    pub scale: f32,
    /// Whether the images were drawn but not written (`--dry-run`).
    pub dry_run: bool,
    /// The pages, in the order the range named them.
    pub pages: Vec<RenderedPage>,
}

/// One rendered page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage {
    /// The 0-based page index.
    pub index: u32,
    /// The 1-based page number.
    pub number: u32,
    /// The image's width in pixels.
    pub width: u32,
    /// The image's height in pixels.
    pub height: u32,
    /// Where it was written: a path, or `-`.
    pub output: String,
}

impl RenderReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.render/1";
}

fn render_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(RenderReport::SCHEMA)
}

/// `invalid-command-line` about the output template, with the paths in `data`.
fn clash(message: &str, output: &str) -> CliError {
    CliError::new(ErrorCode::InvalidCommandLine, message).with("output", output)
}

/// Checks `args.output` against all of a command's `inputs` before any is read: `-` takes one
/// input, and several inputs need a template that names the input and gives each a file of its
/// own.
///
/// # Errors
///
/// `invalid-command-line` for a bad template or two inputs that would write the same file.
pub fn check_outputs(args: &RenderArgs, inputs: &[String]) -> Result<(), CliError> {
    let template = Template::parse(&args.output)?;
    if inputs.len() < 2 {
        return Ok(());
    }
    if template.is_stdout() {
        return Err(clash("standard output takes one page of one input", &args.output));
    }
    let mut seen = BTreeSet::new();
    for (index, input) in inputs.iter().enumerate() {
        let path = template.expand(Fill { input, index, page: 1 });
        if !seen.insert(path.clone()) {
            return Err(clash(
                "two inputs would write the same file: put {stem}, {name} or {index} in the output template",
                &path,
            ));
        }
    }
    Ok(())
}

/// The scale `args` asks for.
fn scale(args: &RenderArgs) -> Result<f32, CliError> {
    let (value, flag) = match (args.scale, args.dpi) {
        (Some(scale), _) => (scale, "scale"),
        (None, Some(dpi)) => (dpi / 72.0, "dpi"),
        (None, None) => (DEFAULT_DPI / 72.0, "dpi"),
    };
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "the resolution must be a positive number",
        )
        .with("argument", flag))
    }
}

/// `rgba` (straight RGBA8, `width` × `height`) as a PNG file.
fn png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, CliError> {
    let failed = |e: png::EncodingError| {
        CliError::new(ErrorCode::OutputUnwritable, "couldn't encode the PNG").with("reason", e.to_string())
    };
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(failed)?;
    writer.write_image_data(rgba).map_err(failed)?;
    writer.finish().map_err(failed)?;
    Ok(out)
}

fn backend(choice: Backend) -> Result<Box<dyn RenderBackend>, CliError> {
    Ok(match choice {
        Backend::Hayro => Box::new(HayroBackend::new()),
        Backend::Pdfium => Box::new(PdfiumBackend::from_env()?),
    })
}

/// Renders `args.pages` (every page by default) of `args.input` and writes each as a PNG to
/// the path its template gives. Every path is worked out, and refused if two pages share one,
/// before anything is written.
///
/// # Errors
///
/// `invalid-command-line` for a bad template, resolution or clash; `invalid-page-range`; the
/// input couldn't be read or opened; a page couldn't be rendered; an output couldn't be written.
pub fn render(args: &RenderArgs, settings: &Settings, io: &mut dyn Io) -> Result<RenderReport, CliError> {
    let template = Template::parse(&args.output)?;
    let scale = scale(args)?;
    let data = io.read(&args.input)?;
    settings.stop_if_cancelled()?;
    let source: Arc<dyn ByteSource> = Arc::new(data);
    let mut options = RenderOpenOptions::new();
    if let Some(password) = &settings.password {
        options = options.with_password(password);
    }
    let doc = backend(args.backend)?.open(source, &options)?;
    settings.stop_if_cancelled()?;
    let numbers = pages_of(args.pages.as_deref(), doc.page_count())?;
    if template.is_stdout() && numbers.len() != 1 {
        return Err(clash(
            "standard output takes one page: name it with --pages",
            &args.output,
        ));
    }
    let mut outputs = Vec::with_capacity(numbers.len());
    let mut seen = BTreeSet::new();
    for &number in &numbers {
        let path = template.expand(Fill {
            input: &args.input,
            index: args.index,
            page: number,
        });
        if !seen.insert(path.clone()) {
            return Err(clash(
                "two pages would write the same file: put {n} in the output template",
                &path,
            ));
        }
        // Refuse an existing file (without --force) before any page is written.
        io.check(&path, settings.force)?;
        outputs.push(path);
    }
    let mut pages = Vec::with_capacity(numbers.len());
    for (number, output) in numbers.into_iter().zip(outputs) {
        settings.stop_if_cancelled()?;
        let index = number.saturating_sub(1);
        let request = RenderRequest::page(index, scale);
        let plan = doc.plan(&request)?;
        let mut rgba = vec![0; plan.byte_len()];
        doc.render(&request, &mut rgba, &settings.cancel)?;
        let (width, height) = (plan.tile.width, plan.tile.height);
        let image = png(width, height, &rgba)?;
        if !settings.dry_run {
            io.write(&output, &image, settings.force)?;
        }
        pages.push(RenderedPage {
            index,
            number,
            width,
            height,
            output,
        });
    }
    Ok(RenderReport {
        schema: RenderReport::SCHEMA,
        input: args.input.clone(),
        backend: args.backend,
        scale,
        dry_run: settings.dry_run,
        pages,
    })
}
