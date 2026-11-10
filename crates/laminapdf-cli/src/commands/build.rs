//! `laminapdf build`: Markdown into tagged pages that flow ([ADR 0032] §6–§7). Fonts and images
//! are read through [`Io`], so the MCP server reads them inside its roots; nothing is fetched.
//!
//! [ADR 0032]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0032-flowing-layout-and-document-inputs.md

use std::path::Path;

use laminapdf::markdown::{MarkdownBuildOptions, MarkdownFonts, image_targets};
use laminapdf::{
    BuildOptions, Builder, Diagnostic, ErrorCode, Font, FontHandle, Image, PageSize, PdfDate, Profile, SaveOptions,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::{Io, STDIO};

/// `laminapdf build <input.md | -> -o OUT`.
#[derive(clap::Args, Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildArgs {
    /// The Markdown file, or `-` for standard input.
    pub input: String,
    /// The PDF to write, or `-` for standard output.
    #[arg(short = 'o', long, value_name = "OUT")]
    pub output: String,
    /// A font for a role: `regular=FILE`, `bold=FILE`, `italic=FILE`, `bold-italic=FILE`,
    /// `mono=FILE`, or `fallback=FILE` (repeatable) for characters the others lack; `FILE#N`
    /// takes face N of a collection (.ttc). Without fonts the text is set in Helvetica and
    /// Courier: Latin only, and never embedded.
    #[arg(long = "font", value_name = "ROLE=FILE")]
    #[serde(default)]
    pub fonts: Vec<String>,
    /// The page size: a4 (the default), a3, a5, letter or legal.
    #[arg(long, value_name = "SIZE")]
    #[serde(default)]
    pub page_size: Option<String>,
    /// A profile to meet: pdf-ua-1, pdf-a-2b, pdf-a-2u or pdf-a-3b (repeatable: PDF/UA-1 and
    /// one PDF/A).
    #[arg(long = "profile", value_name = "PROFILE")]
    #[serde(default)]
    pub profiles: Vec<String>,
    /// The language (a BCP 47 tag such as en-GB) when the front matter gives none.
    #[arg(long, value_name = "TAG")]
    #[serde(default)]
    pub lang: Option<String>,
    /// The title when the front matter gives none.
    #[arg(long)]
    #[serde(default)]
    pub title: Option<String>,
    /// The creation date, as PDF writes it (`20261110` or `D:20261110120000Z`); none without it,
    /// so the same input gives the same bytes.
    #[arg(long, value_name = "DATE")]
    #[serde(default)]
    pub creation_date: Option<String>,
}

/// `laminapdf.build/1`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildReport {
    /// `laminapdf.build/1`.
    #[schemars(schema_with = "build_id")]
    pub schema: &'static str,
    /// The input as given: a path, or `-`.
    pub input: String,
    /// Where the PDF went: a path, or `-`.
    pub output: String,
    /// The pages built.
    pub page_count: u32,
    /// The bytes written (none with `--dry-run`).
    pub bytes: u64,
    /// Whether the PDF is tagged (the flow tags every document).
    pub tagged: bool,
    /// The profiles the PDF meets, as their usual names (`PDF/UA-1`).
    pub profiles: Vec<String>,
    /// What the PDF leaves out or worked around (ADR 0026 objects): raw HTML, images that
    /// weren't found or are remote, keep rules broken.
    #[schemars(schema_with = "warnings_schema")]
    pub warnings: Vec<Diagnostic>,
}

impl BuildReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.build/1";
}

fn build_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(BuildReport::SCHEMA)
}

fn usage(message: &str, argument: &str) -> CliError {
    CliError::new(ErrorCode::InvalidCommandLine, message).with("argument", argument)
}

fn page_size(name: &str) -> Result<PageSize, CliError> {
    Ok(match name.to_ascii_lowercase().as_str() {
        "a3" => PageSize::A3,
        "a4" => PageSize::A4,
        "a5" => PageSize::A5,
        "letter" => PageSize::LETTER,
        "legal" => PageSize::LEGAL,
        _ => return Err(usage("the page size must be a3, a4, a5, letter or legal", name)),
    })
}

fn profile(name: &str) -> Result<Profile, CliError> {
    Ok(match name.to_ascii_lowercase().replace('/', "-").as_str() {
        "pdf-ua-1" => Profile::PdfUa1,
        "pdf-a-2b" => Profile::PdfA2b,
        "pdf-a-2u" => Profile::PdfA2u,
        "pdf-a-3b" => Profile::PdfA3b,
        _ => {
            return Err(usage(
                "the profile must be pdf-ua-1, pdf-a-2b, pdf-a-2u or pdf-a-3b",
                name,
            ));
        }
    })
}

/// A font from `FILE` or `FILE#N`: face `N` of a collection (`.ttc`), the first face without it.
fn load_font(builder: &mut Builder, given: &str, io: &mut dyn Io) -> Result<FontHandle, CliError> {
    let (path, face) = match given.rsplit_once('#') {
        Some((path, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => (
            path,
            Some(
                n.parse::<u32>()
                    .map_err(|_| usage("the face number is too large", given))?,
            ),
        ),
        _ => (given, None),
    };
    let bytes = io.read(path)?;
    let font = match face {
        Some(index) => Font::from_collection(bytes, index),
        None if bytes.starts_with(b"ttcf") => Font::from_collection(bytes, 0),
        None => Font::from_bytes(bytes),
    };
    Ok(builder.font(font.map_err(laminapdf::Error::from)?)?)
}

/// The fonts `--font` names, by role; `None` without any.
fn fonts(args: &BuildArgs, builder: &mut Builder, io: &mut dyn Io) -> Result<Option<MarkdownFonts>, CliError> {
    let mut roles: Vec<(&str, FontHandle)> = Vec::with_capacity(args.fonts.len());
    for given in &args.fonts {
        let Some((role, path)) = given.split_once('=') else {
            return Err(usage(
                "a font is ROLE=FILE, such as regular=NotoSans-Regular.ttf",
                given,
            ));
        };
        if !matches!(
            role,
            "regular" | "bold" | "italic" | "bold-italic" | "mono" | "fallback"
        ) {
            return Err(usage(
                "the font role must be regular, bold, italic, bold-italic, mono or fallback",
                given,
            ));
        }
        if role != "fallback" && roles.iter().any(|(r, _)| *r == role) {
            return Err(usage("each font role is given once; only fallback repeats", given));
        }
        roles.push((role, load_font(builder, path, io)?));
    }
    if roles.is_empty() {
        return Ok(None);
    }
    let Some((_, regular)) = roles.iter().find(|(r, _)| *r == "regular") else {
        return Err(usage(
            "the other font roles need a regular font: add --font regular=FILE",
            &args.fonts.join(" "),
        ));
    };
    let mut fonts = MarkdownFonts::new(regular);
    for (role, font) in &roles {
        fonts = match *role {
            "bold" => fonts.with_bold(font),
            "italic" => fonts.with_italic(font),
            "bold-italic" => fonts.with_bold_italic(font),
            "mono" => fonts.with_mono(font),
            "fallback" => fonts.with_fallback(font),
            _ => fonts,
        };
    }
    Ok(Some(fonts))
}

/// Whether the image target is on another computer, which the command never fetches.
fn is_remote(target: &str) -> bool {
    target.contains("://") || target.starts_with("//") || target.starts_with("data:")
}

/// The images the Markdown shows, read from the input's folder (the working folder for standard
/// input); remote ones are left out, and the PDF warns `markdown-image-missing` for them.
fn images(
    args: &BuildArgs,
    markdown: &str,
    mut options: MarkdownBuildOptions,
    io: &mut dyn Io,
) -> Result<MarkdownBuildOptions, CliError> {
    let folder = match args.input.as_str() {
        STDIO => Path::new(""),
        input => Path::new(input).parent().unwrap_or_else(|| Path::new("")),
    };
    for target in image_targets(markdown).into_iter().filter(|t| !is_remote(t)) {
        let path = folder.join(&target);
        let bytes = io.read(&path.to_string_lossy())?;
        let image = if bytes.starts_with(&[0xFF, 0xD8]) {
            Image::jpeg(bytes)
        } else {
            Image::png(&bytes)
        };
        options = options.with_image(
            target,
            image.map_err(|e| {
                CliError::from(laminapdf::Error::from(e)).with("input", path.to_string_lossy().as_ref())
            })?,
        );
    }
    Ok(options)
}

/// Builds the Markdown `args.input` names into a PDF and writes it to `args.output`.
///
/// # Errors
///
/// `invalid-command-line` (a font, page size, profile or date), `overwrite-refused`; the input,
/// a font or an image couldn't be read; `missing-glyph` and, under a profile, the first rule the
/// document breaks (`font-not-embedded` without `--font`); the output couldn't be written.
pub fn build(args: &BuildArgs, settings: &Settings, io: &mut dyn Io) -> Result<BuildReport, CliError> {
    // Checked before the work, so a refused output costs nothing.
    io.check(&args.output, settings.force)?;
    let mut build_options = BuildOptions::new();
    for name in &args.profiles {
        build_options = build_options.with_profile(profile(name)?);
    }
    if let Some(date) = &args.creation_date {
        let date = date
            .parse::<PdfDate>()
            .map_err(|_| usage("the date is YYYYMMDD, or D:YYYYMMDDHHmmSSZ as PDF writes it", date))?;
        build_options = build_options.with_creation_date(date);
    }
    let profiles: Vec<String> = build_options.profiles().map(|p| p.as_str().to_owned()).collect();
    let size = args.page_size.as_deref().map(page_size).transpose()?;
    let markdown = String::from_utf8(io.read(&args.input)?).map_err(|_| {
        CliError::new(ErrorCode::InputUnreadable, "the Markdown isn't UTF-8 text")
            .with("input", args.input.as_str())
            .with("reason", "InvalidData")
    })?;
    settings.stop_if_cancelled()?;
    let mut builder = Builder::new(build_options);
    let mut options = MarkdownBuildOptions::new().with_page_size(size.unwrap_or(PageSize::A4));
    if let Some(fonts) = fonts(args, &mut builder, io)? {
        options = options.with_fonts(fonts);
    }
    let options = images(args, &markdown, options, io)?;
    // Set first: the front matter's title and language win over them.
    if let Some(title) = &args.title {
        builder.metadata(laminapdf::Metadata::default().with_title(title.clone()));
    }
    if let Some(lang) = &args.lang {
        builder.lang(lang);
    }
    builder.markdown(&markdown, &options)?;
    settings.stop_if_cancelled()?;
    let mut doc = builder.finish()?;
    let saved = doc.save_with(&SaveOptions::default(), &settings.cancel)?;
    if settings.dry_run {
        io.check(&args.output, settings.force)?;
    } else {
        io.write(&args.output, &saved.bytes, settings.force)?;
    }
    Ok(BuildReport {
        schema: BuildReport::SCHEMA,
        input: args.input.clone(),
        output: args.output.clone(),
        page_count: doc.page_count()?,
        bytes: if settings.dry_run {
            0
        } else {
            u64::try_from(saved.bytes.len()).unwrap_or(u64::MAX)
        },
        tagged: doc.is_tagged()?,
        profiles,
        warnings: warnings(&doc),
    })
}
