//! Finishing a build: the page tree (ISO 32000-2 §7.7.3.2), the catalog (§7.7.2), the Info
//! dictionary (§14.3.3) and the trailer, or new pages linked into an existing tree.

use std::collections::{BTreeMap, BTreeSet};

use lamina_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString, Stream};
use lamina_model::{PageFrame, PageId};
use lamina_syntax::text::encode_text_string;
#[cfg(feature = "text")]
use lamina_text::FontRegistry;

use crate::annot::link::PendingLink;
use crate::builder::{BuildOptions, BuildState, DocFeatures};
use crate::error::Result;
use crate::meta::Metadata;
use crate::meta::xmp;
use crate::nav::dest::Targets;
use crate::nav::{LinkTarget, labels, outline};
use crate::page::count;
use crate::profile::{checks, pdfa};

fn invalid(message: &'static str) -> lamina_cos::Error {
    lamina_cos::Error::InvalidStructure {
        kind: "page tree",
        message,
    }
}

fn kids(pages: &[ObjRef]) -> Object {
    Object::Array(pages.iter().map(|r| Object::Ref(*r)).collect())
}

/// Subsets and writes every font text was drawn with.
#[cfg(feature = "text")]
fn write_fonts(cos: &mut CosMut<'_>, state: &mut BuildState) -> Result<()> {
    let fonts = std::mem::replace(&mut state.fonts, FontRegistry::new());
    state.font_warnings.extend_from_slice(fonts.warnings());
    for embedded in fonts.finish_fonts(cos)? {
        state.font_warnings.extend(embedded.warnings);
    }
    Ok(())
}

/// The pending link annotations with their targets resolved (nothing is written yet).
fn resolve_links(links: Vec<PendingLink>, targets: &Targets<'_>) -> Result<Vec<(ObjRef, Dict)>> {
    links
        .into_iter()
        .map(
            |PendingLink {
                 object,
                 mut dict,
                 target,
                 ..
             }| {
                targets.apply(&mut dict, &target)?;
                Ok((object, dict))
            },
        )
        .collect()
}

fn write_links(cos: &mut CosMut<'_>, links: Vec<(ObjRef, Dict)>) -> Result<()> {
    for (object, dict) in links {
        cos.set(object, Object::Dict(dict))?;
    }
    Ok(())
}

/// The pages links target, by index: page `i` of the document, an existing one (`existing`
/// of them) or one of the new ones in `state`. Only the targeted pages are read, so a
/// malformed page nobody links to doesn't fail the append.
fn targeted_pages(
    doc: &lamina_model::Document,
    existing: usize,
    state: &BuildState,
) -> Result<BTreeMap<u32, (ObjRef, PageFrame)>> {
    let wanted: BTreeSet<u32> = state
        .links
        .iter()
        .filter_map(|l| match l.target {
            LinkTarget::Page { index, .. } => Some(index),
            _ => None,
        })
        .collect();
    let mut out = BTreeMap::new();
    if wanted.is_empty() {
        return Ok(out);
    }
    let pages = doc.pages()?;
    for index in wanted {
        let Ok(i) = usize::try_from(index) else { continue };
        let found = if let Some(page) = (i < existing).then(|| pages.get(i)).flatten() {
            let r = page
                .obj_ref()
                .ok_or_else(|| invalid("a linked page is a direct object, so a link can't point at it"))?;
            Some((r, page.frame()?))
        } else {
            let j = i - existing.min(i);
            (i >= existing)
                .then(|| state.pages.get(j).copied().zip(state.frames.get(j).copied()))
                .flatten()
        };
        if let Some(found) = found {
            out.insert(index, found);
        }
    }
    Ok(out)
}

/// Finishes an appender: links resolved against every page of the document first (reads
/// only, so a bad target changes nothing), then the new pages linked into the tree, the
/// fonts and links written and the version raised. A write that fails rolls the document's
/// overlay back to where this call started.
pub(crate) fn finish_appended(doc: &mut lamina_model::Document, mut state: BuildState) -> Result<Vec<PageId>> {
    let existing = doc.pages()?.len();
    let targets = Targets {
        pages: targeted_pages(doc, existing, &state)?,
        count: existing + state.pages.len(),
        names: None,
    };
    let links = resolve_links(std::mem::take(&mut state.links), &targets)?;
    let mark = doc.journal_mark();
    let written = (|| -> Result<()> {
        append_pages(doc.cos_mut(), &state)?;
        let mut edit = doc.cos_mut().edit();
        #[cfg(feature = "text")]
        write_fonts(&mut edit, &mut state)?;
        write_links(&mut edit, links)?;
        raise_version(doc.cos_mut(), state.required)
    })();
    if let Err(e) = written {
        // The overlay goes back to the mark; should that fail too, the first error is the one
        // to report.
        let _ = doc.cos_mut().rollback_to(mark);
        return Err(e);
    }
    Ok(state.pages.iter().map(|r| PageId::object(*r)).collect())
}

/// The Info values: the metadata's, falling back to the options' producer and date, with
/// text XML can't hold removed (so Info and XMP say the same).
fn merged_metadata(options: &BuildOptions, features: &DocFeatures) -> Metadata {
    let mut meta = features.metadata.clone().unwrap_or_default();
    if meta.producer.is_none() {
        meta.producer = options.producer().map(str::to_owned);
    }
    if meta.creation_date.is_none() {
        meta.creation_date = options.creation_date().copied();
    }
    meta.sanitize();
    meta
}

/// Runs the profiles' checks (nothing is written yet).
fn check_profiles(options: &BuildOptions, state: &BuildState, features: &DocFeatures, meta: &Metadata) -> Result<()> {
    let conformance = options.conformance();
    if conformance.is_empty() {
        return Ok(());
    }
    checks::check(&checks::Inputs {
        conformance,
        version: version_of(options.version().as_str().as_bytes()).unwrap_or((1, 7)),
        title: meta.title.as_deref(),
        lang: features.lang.as_deref(),
        diagnostics: state.tag.diagnostics(),
        elements: state.tag.infos(),
        links: &state.links,
        device_cmyk: state.device_cmyk,
        attachments: &features.attachments,
    })?;
    #[cfg(feature = "text")]
    checks::fonts(conformance, &state.fonts, &state.font_keys)?;
    Ok(())
}

/// The catalog's `/Names` (named destinations, attached files) and `/AF`.
fn write_names(
    cos: &mut CosMut<'_>,
    cat: &mut Dict,
    targets: &Targets<'_>,
    features: &DocFeatures,
    created: Option<&lamina_model::PdfDate>,
) -> Result<()> {
    let mut names = Dict::new();
    if let Some(tree) = targets.dests_tree()? {
        names.insert("Dests", tree);
    }
    if !features.attachments.is_empty() {
        let (tree, specs) = crate::attach::write(cos, &features.attachments, created)?;
        names.insert("EmbeddedFiles", tree);
        cat.insert("AF", Object::Array(specs.into_iter().map(Object::Ref).collect()));
    }
    if !names.is_empty() {
        cat.insert("Names", names);
    }
    Ok(())
}

/// A new document's objects: fonts, links, the reserved page tree root and catalog, the
/// outline, names, page labels, Info and XMP, the trailer.
pub(crate) fn new_document(
    doc: &mut lamina_cos::Document,
    options: &BuildOptions,
    catalog: ObjRef,
    state: &mut BuildState,
    features: &DocFeatures,
) -> Result<()> {
    let targets = Targets {
        pages: (0_u32..)
            .zip(state.pages.iter().copied().zip(state.frames.iter().copied()))
            .collect(),
        count: state.pages.len(),
        names: Some(&features.names),
    };
    let conformance = options.conformance();
    let meta = merged_metadata(options, features);
    check_profiles(options, state, features, &meta)?;
    let links = resolve_links(std::mem::take(&mut state.links), &targets)?;
    let mut cos = doc.edit();
    #[cfg(feature = "text")]
    write_fonts(&mut cos, state)?;
    write_links(&mut cos, links)?;

    let mut root = Dict::new();
    root.insert("Type", Name::from("Pages"));
    root.insert("Kids", kids(&state.pages));
    root.insert("Count", count(state.pages.len()));
    cos.set(state.pages_root, Object::Dict(root))?;

    let mut required = state.required;
    if !features.names.is_empty() {
        required = required.max((1, 2));
    }
    if features
        .outline
        .as_ref()
        .is_some_and(|o| o.entries.iter().any(outline::OutlineEntry::styled))
    {
        required = required.max((1, 4));
    }
    if features.labels.is_some() {
        required = required.max((1, 3));
    }
    // PDF/UA-1 §7.1-9 (Matterhorn 07-001, 07-002): the title bar shows the title.
    let prefs = if conformance.ua {
        Some(features.prefs.unwrap_or_default().with_display_doc_title(true))
    } else {
        features.prefs
    };
    if let Some(prefs) = prefs {
        required = required.max(prefs.version());
    }
    if !features.attachments.is_empty() {
        // `/UF` (§7.11.3) is PDF 1.7.
        required = required.max((1, 7));
    }
    let xmp = features.metadata.is_some() || !conformance.is_empty();
    if features.lang.is_some() || xmp || state.tag.tagged {
        required = required.max((1, 4));
    }
    let mut cat = Dict::new();
    cat.insert("Type", Name::from("Catalog"));
    if required > version_of(options.version().as_str().as_bytes()).unwrap_or((1, 7)) {
        cat.insert("Version", version_name(required));
    }
    cat.insert("Pages", state.pages_root);
    write_names(&mut cos, &mut cat, &targets, features, meta.creation_date.as_ref())?;
    if let Some(o) = &features.outline {
        outline::write(&mut cos, o.root, &o.entries, &o.ids, &targets)?;
        cat.insert("Outlines", o.root);
    }
    if let Some(ranges) = &features.labels {
        cat.insert("PageLabels", labels::labels_tree(ranges, state.pages.len())?);
    }
    if let Some(prefs) = prefs {
        prefs.write(&mut cat);
    }
    if let Some(lang) = &features.lang {
        cat.insert("Lang", PdfString::literal(encode_text_string(lang)));
    }
    state.tag.write(&mut cos, &mut cat)?;
    if xmp {
        let mut d = Dict::new();
        d.insert("Type", Name::from("Metadata"));
        d.insert("Subtype", Name::from("XML"));
        let stream = Stream::new(d, xmp::packet(&meta, conformance).into_bytes());
        cat.insert("Metadata", cos.create(Object::Stream(stream))?);
    }
    if conformance.pdfa.is_some() {
        cat.insert("OutputIntents", pdfa::output_intents(&mut cos)?);
    }
    cos.set(catalog, Object::Dict(cat))?;

    let mut trailer = Dict::new();
    trailer.insert("Root", catalog);
    let info = meta.info_dict();
    if !info.is_empty() {
        trailer.insert("Info", cos.create(Object::Dict(info))?);
    }
    if let Some(id) = options.id() {
        let id = Object::String(PdfString::hex(id.to_vec()));
        trailer.insert("ID", vec![id.clone(), id]);
    }
    cos.set_trailer(trailer);
    Ok(())
}

/// The catalog's `/Pages`, which must be an indirect dictionary to take new kids.
pub(crate) fn page_tree_root(doc: &lamina_cos::Document) -> Result<ObjRef> {
    let catalog = doc.catalog()?;
    let root = catalog
        .get("Pages")
        .and_then(Object::as_ref)
        .ok_or_else(|| invalid("the catalog's /Pages is not an indirect reference"))?;
    match &*doc.resolve(root)? {
        Object::Dict(_) => Ok(root),
        _ => Err(invalid("the catalog's /Pages is not a dictionary").into()),
    }
}

/// Appends `state.pages` to the root node's `/Kids` (direct, or an indirect array) and adds
/// them to `/Count`.
pub(crate) fn append_pages(doc: &mut lamina_cos::Document, state: &BuildState) -> Result<()> {
    if state.pages.is_empty() {
        return Ok(());
    }
    let root_ref = state.pages_root;
    // The pages the walk finds now: /Count afterwards is these plus the new ones (a stale
    // /Count, which the reader repairs, is corrected on the way).
    let before = doc.pages()?.len();
    let Object::Dict(mut root) = (*doc.resolve(root_ref)?).clone() else {
        return Err(invalid("the catalog's /Pages is not a dictionary").into());
    };
    let new_kids = state.pages.iter().map(|r| Object::Ref(*r));
    let mut cos = doc.edit();
    match root.get("Kids") {
        Some(Object::Ref(array_ref)) => {
            let array_ref = *array_ref;
            let Some(Object::Array(mut items)) = cos.get(array_ref)?.as_deref().cloned() else {
                return Err(invalid("the root node's /Kids is not an array").into());
            };
            items.extend(new_kids);
            cos.set(array_ref, Object::Array(items))?;
        }
        Some(Object::Array(items)) => {
            let mut items = items.clone();
            items.extend(new_kids);
            root.insert("Kids", items);
        }
        None => root.insert("Kids", Object::Array(new_kids.collect())),
        Some(_) => return Err(invalid("the root node's /Kids is not an array").into()),
    }
    root.insert("Count", count(before + state.pages.len()));
    cos.set(root_ref, Object::Dict(root))?;
    Ok(())
}

/// `major.minor` from a version string (`1.7`), `None` for anything else.
pub(crate) fn version_of(text: &[u8]) -> Option<(u8, u8)> {
    let text = std::str::from_utf8(text).ok()?;
    let (major, minor) = text.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn version_name((major, minor): (u8, u8)) -> Name {
    Name::new(format!("{major}.{minor}"))
}

/// Raises an open document's version to `required` with the catalog's `/Version` (§7.7.2)
/// when neither the header nor an existing `/Version` reaches it.
pub(crate) fn raise_version(doc: &mut lamina_cos::Document, required: (u8, u8)) -> Result<()> {
    let header = doc.header_version().and_then(|v| version_of(v.as_bytes()));
    let mut catalog = doc.catalog()?;
    let declared = catalog.get_name("Version").and_then(version_of);
    let current = header.max(declared).unwrap_or((1, 0));
    if required <= current {
        return Ok(());
    }
    let Some(root) = doc.trailer().get("Root").and_then(Object::as_ref) else {
        return Err(lamina_cos::Error::InvalidStructure {
            kind: "catalog",
            message: "the catalog is not an indirect object, so /Version can't be raised",
        }
        .into());
    };
    catalog.insert("Version", version_name(required));
    doc.edit().set(root, Object::Dict(catalog))?;
    Ok(())
}
