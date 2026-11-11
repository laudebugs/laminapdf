//! `laminapdf build` for a `laminapdf.document/1` document ([ADR 0032] §6–§7): a JSON document,
//! or a template filled from `--data`. The fonts and images it names come by name from
//! `--font NAME=FILE` and `--image NAME=FILE`, or from files of those names in its folder; all
//! are read through [`Io`], so the MCP server reads them inside its roots.
//!
//! [ADR 0032]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0032-flowing-layout-and-document-inputs.md

use std::collections::BTreeMap;
use std::path::Path;

use laminapdf::flow::document::{FlowDocument, Resources, fill};
use laminapdf::{BuildOptions, Builder, ErrorCode, Standard14};
use serde_json::Value;

use super::{
    BuildArgs, BuildReport, Settings, decode_image, finish_build, font_from_bytes, input_folder, is_local, is_missing,
    load_font, options, profile_names, set_title_and_lang, usage,
};
use crate::exit::CliError;
use crate::io::{Io, STDIO};

/// Where the fonts and images a document names come from when no `NAME=FILE` gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unnamed {
    /// Files of those names in the document's folder (the working folder for standard input).
    FromFolder,
    /// Nowhere (an inline document under MCP): it reads only the files it is given by name.
    Nowhere,
}

/// `bytes`, read from `input`, as a JSON document, or `None` for Markdown. A `.json` file is a
/// document (and must be JSON); so is standard input holding a JSON object, whatever its
/// `schema`, which the document check then reads: Markdown never parses as an object.
pub(super) fn read_document(input: &str, bytes: &[u8]) -> Result<Option<Value>, CliError> {
    if input == STDIO {
        if bytes.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'{') {
            return Ok(None);
        }
        return Ok(serde_json::from_slice::<Value>(bytes).ok().filter(Value::is_object));
    }
    let json = Path::new(input)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if json {
        parse_json(bytes, input, "document").map(Some)
    } else {
        Ok(None)
    }
}

/// `bytes` as JSON, or `input-unreadable` naming `input`.
pub(super) fn parse_json(bytes: &[u8], input: &str, what: &str) -> Result<Value, CliError> {
    serde_json::from_slice(bytes).map_err(|_| {
        CliError::new(ErrorCode::InputUnreadable, format!("the {what} isn't JSON"))
            .with("input", input)
            .with("reason", "InvalidData")
            .with_hint("the file was read, but it isn't valid JSON: check its syntax")
    })
}

/// `NAME=FILE` arguments by name; each name once.
fn named<'a>(given: &'a [String], what: &str) -> Result<BTreeMap<&'a str, &'a str>, CliError> {
    let mut out = BTreeMap::new();
    for entry in given {
        let Some((name, path)) = entry
            .split_once('=')
            .filter(|(name, path)| !name.is_empty() && !path.is_empty())
        else {
            return Err(usage(
                &format!("a {what} is NAME=FILE, the name the document gives it"),
                entry,
            ));
        };
        if out.insert(name, path).is_some() {
            return Err(usage(&format!("each {what} name is given once"), entry));
        }
    }
    Ok(out)
}

/// The fonts and images `document` names: those `args` gives by name, then, with
/// [`Unnamed::FromFolder`], files of the other names in `folder`. A name that isn't a plain file
/// name in that folder or below it, or whose file isn't there, is left for the build to report
/// (`invalid-document`, `unknown-font` or `unknown-image`); standard 14 font names need no file.
fn resources(
    args: &BuildArgs,
    document: &FlowDocument,
    builder: &mut Builder,
    unnamed: Unnamed,
    folder: &Path,
    io: &mut dyn Io,
) -> Result<Resources, CliError> {
    let fonts = named(&args.fonts, "font")?;
    let images = named(&args.images, "image")?;
    let mut resources = Resources::new();
    for (name, path) in &fonts {
        resources = resources.with_font(*name, &load_font(builder, path, io)?);
    }
    for (name, path) in &images {
        resources = resources.with_image(*name, decode_image(io.read(path)?, path)?);
    }
    if unnamed == Unnamed::Nowhere {
        return Ok(resources);
    }
    let unnamed_fonts = document
        .font_names()
        .into_iter()
        .filter(|name| !fonts.contains_key(name) && Standard14::from_base_font(name).is_none());
    for name in unnamed_fonts {
        if let Some(bytes) = read_beside(name, folder, io)? {
            resources = resources.with_font(name, &builder.font(font_from_bytes(bytes, None)?)?);
        }
    }
    for name in document
        .image_names()
        .into_iter()
        .filter(|name| !images.contains_key(name))
    {
        if let Some(bytes) = read_beside(name, folder, io)? {
            let shown = folder.join(name);
            resources = resources.with_image(name, decode_image(bytes, &shown.to_string_lossy())?);
        }
    }
    Ok(resources)
}

/// The file `name` in `folder`, when `name` names a file there or below it (no path elsewhere)
/// and the file exists.
fn read_beside(name: &str, folder: &Path, io: &mut dyn Io) -> Result<Option<Vec<u8>>, CliError> {
    if !is_local(name) {
        return Ok(None);
    }
    match io.read(&folder.join(name).to_string_lossy()) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if is_missing(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Builds `template`, filled from `data` when given, with `args`' options, and writes the PDF.
pub(super) fn build_document(
    args: &BuildArgs,
    template: &Value,
    data: Option<&Value>,
    unnamed: Unnamed,
    settings: &Settings,
    io: &mut dyn Io,
) -> Result<BuildReport, CliError> {
    if args.page_size.is_some() {
        return Err(usage(
            "a JSON document sets its own page size, in page.size",
            args.page_size.as_deref().unwrap_or_default(),
        ));
    }
    let document = match data {
        Some(data) => fill(template, data),
        None => FlowDocument::from_json(template),
    }
    .map_err(laminapdf::Error::from)?;
    let mut build_options = BuildOptions::new();
    for profile in document.profiles() {
        build_options = build_options.with_profile(profile);
    }
    let build_options = options(args, build_options)?;
    let profiles = profile_names(&build_options);
    settings.stop_if_cancelled()?;
    let mut builder = Builder::new(build_options);
    let folder = input_folder(&args.input);
    let resources = resources(args, &document, &mut builder, unnamed, folder, io)?;
    set_title_and_lang(&mut builder, args);
    builder.document(&document, &resources)?;
    finish_build(args, builder, profiles, settings, io)
}
