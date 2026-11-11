//! `laminapdf build`: Markdown, or a `laminapdf.document/1` JSON document ([`super::document`]),
//! into tagged pages that flow ([ADR 0032] §6–§7). Fonts and images are read through [`Io`], so
//! the MCP server reads them inside its roots; nothing is fetched.
//!
//! [ADR 0032]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0032-flowing-layout-and-document-inputs.md

use std::path::Path;

use laminapdf::markdown::{MarkdownBuildOptions, MarkdownFonts, image_targets};
use laminapdf::{
    BuildOptions, Builder, Diagnostic, ErrorCode, Font, FontHandle, Image, PageSize, PdfDate, Profile, SaveOptions,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::document::{Unnamed, build_document, is_document, parse_json};
use super::{Settings, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::{Io, STDIO};

/// `laminapdf build <input.md | input.json | -> -o OUT`.
#[derive(clap::Args, Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildArgs {
    /// The Markdown file, a `laminapdf.document/1` JSON document (`.json`), or `-` for standard
    /// input (a document when it is JSON with that `schema`).
    pub input: String,
    /// The PDF to write, or `-` for standard output.
    #[arg(short = 'o', long, value_name = "OUT")]
    pub output: String,
    /// For Markdown, a font for a role: `regular=FILE`, `bold=FILE`, `italic=FILE`,
    /// `bold-italic=FILE`, `mono=FILE`, or `fallback=FILE` (repeatable) for characters the others
    /// lack; without fonts the text is set in Helvetica and Courier: Latin only, and never
    /// embedded. For a JSON document, a font it names: `NAME=FILE`. `FILE#N` takes face N of a
    /// collection (.ttc).
    #[arg(long = "font", value_name = "ROLE=FILE|NAME=FILE")]
    #[serde(default)]
    pub fonts: Vec<String>,
    /// An image a JSON document names: `NAME=FILE` (repeatable). A name given no file is read
    /// from a file of that name in the document's folder.
    #[arg(long = "image", value_name = "NAME=FILE")]
    #[serde(default)]
    pub images: Vec<String>,
    /// The data a JSON document's placeholders read: a JSON file, or `-` for standard input.
    #[arg(long, value_name = "FILE")]
    #[serde(default)]
    pub data: Option<String>,
    /// The page size of Markdown: a4 (the default), a0 to a6, letter, legal or tabloid (a JSON
    /// document sets its own).
    #[arg(long, value_name = "SIZE")]
    #[serde(default)]
    pub page_size: Option<String>,
    /// A profile to meet: pdf-ua-1, pdf-a-2b, pdf-a-2u or pdf-a-3b (repeatable: PDF/UA-1 and
    /// one PDF/A).
    #[arg(long = "profile", value_name = "PROFILE")]
    #[serde(default)]
    pub profiles: Vec<String>,
    /// The language (a BCP 47 tag such as en-GB) when the front matter or document gives none.
    #[arg(long, value_name = "TAG")]
    #[serde(default)]
    pub lang: Option<String>,
    /// The title when the front matter or document gives none.
    #[arg(long)]
    #[serde(default)]
    pub title: Option<String>,
    /// The creation date, as PDF writes it (`20261110` or `D:20261110120000Z`); none without it,
    /// so the same input gives the same bytes.
    #[arg(long, value_name = "DATE")]
    #[serde(default)]
    pub creation_date: Option<String>,
}

/// The `laminapdf_build` MCP tool's arguments: [`BuildArgs`], with the Markdown or the
/// `laminapdf.document/1` document inline (`markdown`, `document`) or read from a path inside the
/// roots (`input`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildToolArgs {
    /// The Markdown itself; give this or `input`. Inline Markdown reads no image files: its
    /// images are left out with a warning. To include images, write the Markdown to a file in
    /// the roots and give it as `input`.
    #[serde(default)]
    pub markdown: Option<String>,
    /// A `laminapdf.document/1` document, or a template whose placeholders `data` fills; give
    /// this, `markdown` or `input`. Inline, it reads only the fonts and images `fonts` and
    /// `images` give by name.
    #[serde(default)]
    #[schemars(schema_with = "document_schema")]
    pub document: Option<serde_json::Value>,
    /// The data a document's placeholders read (`$data`, `$each`, `$item`, `$if`), for `document`
    /// or a JSON `input`.
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// A Markdown file or a JSON document (`.json`) inside the roots; give this, `markdown` or
    /// `document`. Markdown reads its images from its folder or below it; a document reads the
    /// fonts and images it names that `fonts` and `images` don't give from files of those names
    /// in its folder.
    #[serde(default)]
    pub input: Option<String>,
    /// The PDF to write inside the roots: a new file.
    pub output: String,
    /// For Markdown, fonts by role, as `ROLE=FILE`: `regular`, `bold`, `italic`, `bold-italic`,
    /// `mono` and `fallback` (repeatable, for scripts the others lack). For a document, the fonts
    /// it names, as `NAME=FILE`. Each file inside the roots; `FILE#N` takes face N of a collection.
    #[serde(default)]
    pub fonts: Vec<String>,
    /// For a document, the images it names, as `NAME=FILE`, each file inside the roots.
    #[serde(default)]
    pub images: Vec<String>,
    /// The page size of Markdown: a4 (the default), a0 to a6, letter, legal or tabloid.
    #[serde(default)]
    pub page_size: Option<String>,
    /// Profiles to meet: pdf-ua-1, pdf-a-2b, pdf-a-2u, pdf-a-3b (PDF/UA-1 and at most one PDF/A).
    #[serde(default)]
    pub profiles: Vec<String>,
    /// The language when the front matter or document gives none.
    #[serde(default)]
    pub lang: Option<String>,
    /// The title when the front matter or document gives none.
    #[serde(default)]
    pub title: Option<String>,
}

/// `document`'s schema: an object, described by `laminapdf.document/1` (an external reference,
/// which the MCP server's tool schema inlines). A template's placeholders stand where values go,
/// so a template matches `laminapdf.document/1` only once it is filled: the second branch lets it
/// through, and the build checks the filled document.
fn document_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "object",
        "anyOf": [
            { "$ref": laminapdf::flow::document::DOCUMENT_SCHEMA_URL },
            { "type": "object", "required": ["schema"] }
        ]
    })
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
    /// The data a document's placeholders read, as given: a path, or `-` (left out without).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
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

pub(super) fn usage(message: &str, argument: &str) -> CliError {
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
pub(super) fn load_font(builder: &mut Builder, given: &str, io: &mut dyn Io) -> Result<FontHandle, CliError> {
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
pub(super) fn is_local(target: &str) -> bool {
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
pub(super) fn is_missing(error: &CliError) -> bool {
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
        options = options.with_image(target, decode_image(bytes, &shown)?);
    }
    Ok(options)
}

/// An image's bytes as an [`Image`]: JPEG by its signature, else PNG; `shown` names it in an error.
pub(super) fn decode_image(bytes: Vec<u8>, shown: &str) -> Result<Image, CliError> {
    let image = if bytes.starts_with(&[0xFF, 0xD8]) {
        Image::jpeg(bytes)
    } else {
        Image::png(&bytes)
    };
    image.map_err(|e| CliError::from(laminapdf::Error::from(e)).with("input", shown))
}

/// Builds the Markdown or JSON document `args.input` names into a PDF and writes it to
/// `args.output`.
///
/// # Errors
///
/// `invalid-command-line` (a font, image, page size, profile or date, or `--data` and `--image`
/// with Markdown), `overwrite-refused`; the input, the data, a font or an image couldn't be read;
/// `invalid-document`, `template-data-missing` and `template-data-type`; `missing-glyph` and,
/// under a profile, the first rule the document breaks (`font-not-embedded` without fonts); the
/// output couldn't be written.
pub fn build(args: &BuildArgs, settings: &Settings, io: &mut dyn Io) -> Result<BuildReport, CliError> {
    build_input(args, None, settings, io)
}

/// [`build`], with the data given inline (the MCP tool's `data`) rather than by `--data`.
fn build_input(
    args: &BuildArgs,
    inline_data: Option<&serde_json::Value>,
    settings: &Settings,
    io: &mut dyn Io,
) -> Result<BuildReport, CliError> {
    if args.input == STDIO && args.data.as_deref() == Some(STDIO) {
        return Err(usage(
            "standard input is read once: give the document or the data as a file",
            STDIO,
        ));
    }
    // Checked before the work, so a refused output costs nothing.
    io.check(&args.output, settings.force)?;
    let bytes = io.read(&args.input)?;
    if is_document(&args.input, &bytes) {
        let template = parse_json(&bytes, &args.input, "document")?;
        let read = match &args.data {
            Some(path) => Some(parse_json(&io.read(path)?, path, "data")?),
            None => None,
        };
        let data = inline_data.or(read.as_ref());
        return build_document(args, &template, data, Unnamed::FromFolder, settings, io);
    }
    if let Some(given) = args.data.as_ref().or(args.images.first()) {
        return Err(usage("--data and --image are for a JSON document, not Markdown", given));
    }
    if inline_data.is_some() {
        return Err(usage("data is for a JSON document, not Markdown", &args.input));
    }
    let markdown = String::from_utf8(bytes).map_err(|_| {
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
        images: args.images,
        data: None,
        page_size: args.page_size,
        profiles: args.profiles,
        lang: args.lang,
        title: args.title,
        creation_date: None,
    };
    match (args.markdown, args.document, args.input) {
        // Inline Markdown has no folder, so it reads no image: otherwise text an agent was handed
        // could choose which picture files under the roots go into the PDF.
        (Some(markdown), None, None) => {
            if args.data.is_some() || !command.images.is_empty() {
                return Err(usage("data and images are for a document, not Markdown", "markdown"));
            }
            io.check(&command.output, settings.force)?;
            build_markdown(&command, &markdown, Images::None, settings, io)
        }
        // Likewise an inline document reads only the files given to it by name.
        (None, Some(document), None) => {
            io.check(&command.output, settings.force)?;
            build_document(&command, &document, args.data.as_ref(), Unnamed::Nowhere, settings, io)
        }
        (None, None, Some(_)) => build_input(&command, args.data.as_ref(), settings, io),
        (None, None, None) => Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "give the Markdown or the document inline (markdown, document) or as a file (input)",
        )),
        _ => Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "give one of markdown, document and input",
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
    if let Some(given) = args.data.as_ref().or(args.images.first()) {
        return Err(usage("--data and --image are for a JSON document, not Markdown", given));
    }
    let build_options = options(args, BuildOptions::new())?;
    let profiles = profile_names(&build_options);
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
    finish_build(args, builder, profiles, settings, io)
}

/// `build_options` with `args`' profiles (those it doesn't have yet) and creation date.
pub(super) fn options(args: &BuildArgs, mut build_options: BuildOptions) -> Result<BuildOptions, CliError> {
    for name in &args.profiles {
        let wanted = profile(name)?;
        if !build_options.profiles().any(|have| have == wanted) {
            build_options = build_options.with_profile(wanted);
        }
    }
    if let Some(date) = &args.creation_date {
        let date = date
            .parse::<PdfDate>()
            .map_err(|_| usage("the date is YYYYMMDD, or D:YYYYMMDDHHmmSSZ as PDF writes it", date))?;
        build_options = build_options.with_creation_date(date);
    }
    Ok(build_options)
}

/// The profiles `build_options` asks for, by their usual names, for the report.
pub(super) fn profile_names(build_options: &BuildOptions) -> Vec<String> {
    build_options.profiles().map(|p| p.as_str().to_owned()).collect()
}

/// Finishes `builder`'s PDF, writes it to `args.output` (checked before the work) unless
/// `--dry-run`, and reports it.
pub(super) fn finish_build(
    args: &BuildArgs,
    builder: Builder,
    profiles: Vec<String>,
    settings: &Settings,
    io: &mut dyn Io,
) -> Result<BuildReport, CliError> {
    settings.stop_if_cancelled()?;
    let mut doc = builder.finish()?;
    let saved = doc.save_with(&SaveOptions::default(), &settings.cancel)?;
    if !settings.dry_run {
        io.write(&args.output, &saved.bytes, settings.force)?;
    }
    Ok(BuildReport {
        schema: BuildReport::SCHEMA,
        input: args.input.clone(),
        data: args.data.clone(),
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
