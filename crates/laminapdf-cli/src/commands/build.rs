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
    /// The page size: a4 (the default), a0 to a6, letter, legal or tabloid.
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

/// The `laminapdf_build` MCP tool's arguments: [`BuildArgs`], with the Markdown inline
/// (`markdown`) or read from a path inside the roots (`input`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildToolArgs {
    /// The Markdown itself; give this or `input`. Inline Markdown reads no image files: its
    /// images are left out with a warning. To include images, write the Markdown to a file in
    /// the roots and give it as `input`.
    #[serde(default)]
    pub markdown: Option<String>,
    /// A Markdown file inside the roots; give this or `markdown`. Its images are read from its
    /// folder or below it.
    #[serde(default)]
    pub input: Option<String>,
    /// The PDF to write inside the roots: a new file.
    pub output: String,
    /// Fonts by role, as `ROLE=FILE`, each file inside the roots. The roles are `regular`, `bold`,
    /// `italic`, `bold-italic`, `mono` and `fallback` (repeatable, for scripts the others lack);
    /// `FILE#N` takes face N of a collection.
    #[serde(default)]
    pub fonts: Vec<String>,
    /// The page size: a4 (the default), a0 to a6, letter, legal or tabloid.
    #[serde(default)]
    pub page_size: Option<String>,
    /// Profiles to meet: pdf-ua-1, pdf-a-2b, pdf-a-2u, pdf-a-3b (PDF/UA-1 and at most one PDF/A).
    #[serde(default)]
    pub profiles: Vec<String>,
    /// The language when the front matter gives none.
    #[serde(default)]
    pub lang: Option<String>,
    /// The title when the front matter gives none.
    #[serde(default)]
    pub title: Option<String>,
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
    /// What the PDF leaves out or works around: raw HTML (`markdown-html-ignored`); images that
    /// are remote, outside the Markdown's folder or missing (`markdown-image-missing`); and
    /// keep-together rules it had to break.
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
    const SIZES: [(&str, PageSize); 10] = [
        ("a0", PageSize::A0),
        ("a1", PageSize::A1),
        ("a2", PageSize::A2),
        ("a3", PageSize::A3),
        ("a4", PageSize::A4),
        ("a5", PageSize::A5),
        ("a6", PageSize::A6),
        ("letter", PageSize::LETTER),
        ("legal", PageSize::LEGAL),
        ("tabloid", PageSize::TABLOID),
    ];
    let wanted = name.to_ascii_lowercase();
    SIZES
        .iter()
        .find(|(n, _)| *n == wanted)
        .map(|(_, size)| *size)
        .ok_or_else(|| usage("the page size must be a0 to a6, letter, legal or tabloid", name))
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

/// Whether the image target names a file in the Markdown's folder or below it: not on another
/// computer (never fetched), not an absolute path, not reached through `..`, and not empty or the
/// folder itself, so a Markdown file can't pull in a picture from elsewhere on the disk by its
/// path. The command line follows links as the shell does; under MCP, `RootedIo` refuses a link
/// that leads out of the roots.
fn is_local(target: &str) -> bool {
    use std::path::Component;
    let remote = target.contains("://") || target.starts_with("//") || target.starts_with("data:");
    let path = Path::new(target);
    !remote
        && path.is_relative()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        && path.components().any(|c| matches!(c, Component::Normal(_)))
}

/// Whether reading an image failed because nothing is there: the file, or a folder on its path.
fn is_missing(error: &CliError) -> bool {
    let reason = error.data().get("reason").and_then(serde_json::Value::as_str);
    matches!(reason, Some("NotFound" | "NotADirectory" | "IsADirectory"))
}

/// The images the Markdown shows, read from the input's folder (the working folder for standard
/// input). One that isn't there, or isn't a file in that folder or below it, is left out, and the
/// PDF warns `markdown-image-missing` for it (ADR 0032 §6); one that is there but can't be read or
/// decoded fails the build.
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
    for target in image_targets(markdown).into_iter().filter(|t| is_local(t)) {
        let path = folder.join(&target);
        let shown = path.to_string_lossy();
        let bytes = match io.read(&shown) {
            Ok(bytes) => bytes,
            Err(e) if is_missing(&e) => continue,
            Err(e) => return Err(e),
        };
        let image = if bytes.starts_with(&[0xFF, 0xD8]) {
            Image::jpeg(bytes)
        } else {
            Image::png(&bytes)
        };
        let image = image.map_err(|e| CliError::from(laminapdf::Error::from(e)).with("input", shown.as_ref()))?;
        options = options.with_image(target, image);
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
    let markdown = String::from_utf8(io.read(&args.input)?).map_err(|_| {
        CliError::new(ErrorCode::InputUnreadable, "the Markdown isn't UTF-8 text")
            .with("input", args.input.as_str())
            .with("reason", "InvalidData")
    })?;
    build_markdown(args, &markdown, Images::FromFolder, settings, io)
}

/// [`build`] for the MCP tool: the Markdown inline or from `input`.
///
/// # Errors
///
/// `invalid-command-line` unless exactly one of `markdown` and `input` is given; as [`build`].
pub fn build_tool(args: BuildToolArgs, settings: &Settings, io: &mut dyn Io) -> Result<BuildReport, CliError> {
    let command = BuildArgs {
        input: args.input.clone().unwrap_or_else(|| STDIO.to_owned()),
        output: args.output,
        fonts: args.fonts,
        page_size: args.page_size,
        profiles: args.profiles,
        lang: args.lang,
        title: args.title,
        creation_date: None,
    };
    match (args.markdown, args.input) {
        // Inline Markdown has no folder, so it reads no image: otherwise text an agent was handed
        // could choose which picture files under the roots go into the PDF.
        (Some(markdown), None) => {
            io.check(&command.output, settings.force)?;
            build_markdown(&command, &markdown, Images::None, settings, io)
        }
        (None, Some(_)) => build(&command, settings, io),
        (Some(_), Some(_)) => Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "give the Markdown inline (markdown) or as a file (input), not both",
        )),
        (None, None) => Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "give the Markdown inline (markdown) or as a file (input)",
        )),
    }
}

/// Where a build's images come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Images {
    /// The Markdown file's folder, or below it (the working folder for standard input).
    FromFolder,
    /// Nowhere: every image is left out with `markdown-image-missing` (inline Markdown under MCP).
    None,
}

/// Builds `markdown` with `args`' options and its images from where `images` says.
fn build_markdown(
    args: &BuildArgs,
    markdown: &str,
    images_from: Images,
    settings: &Settings,
    io: &mut dyn Io,
) -> Result<BuildReport, CliError> {
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
    settings.stop_if_cancelled()?;
    let mut builder = Builder::new(build_options);
    let mut options = MarkdownBuildOptions::new().with_page_size(size.unwrap_or(PageSize::A4));
    if let Some(fonts) = fonts(args, &mut builder, io)? {
        options = options.with_fonts(fonts);
    }
    let options = match images_from {
        Images::FromFolder => images(args, markdown, options, io)?,
        Images::None => options,
    };
    // Set first: the front matter's title and language win over them.
    if let Some(title) = &args.title {
        builder.metadata(laminapdf::Metadata::default().with_title(title.clone()));
    }
    if let Some(lang) = &args.lang {
        builder.lang(lang);
    }
    builder.markdown(markdown, &options)?;
    settings.stop_if_cancelled()?;
    let mut doc = builder.finish()?;
    let saved = doc.save_with(&SaveOptions::default(), &settings.cancel)?;
    // The output was checked before the work (build, build_tool).
    if !settings.dry_run {
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
