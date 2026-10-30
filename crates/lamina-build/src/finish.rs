//! Finishing a build: the page tree (ISO 32000-2 §7.7.3.2), the catalog (§7.7.2), the Info
//! dictionary (§14.3.3) and the trailer, or new pages linked into an existing tree.

use lamina_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString, Stream};
use lamina_model::PageFrame;
use lamina_syntax::text::encode_text_string;
use lamina_text::FontRegistry;

use crate::builder::{BuildOptions, BuildState, DocFeatures};
use crate::error::Result;
use crate::meta::Metadata;
use crate::meta::xmp;
use crate::nav::dest::Targets;
use crate::nav::{labels, outline};

fn invalid(message: &'static str) -> lamina_cos::Error {
    lamina_cos::Error::InvalidStructure {
        kind: "page tree",
        message,
    }
}

fn kids(pages: &[ObjRef]) -> Object {
    Object::Array(pages.iter().map(|r| Object::Ref(*r)).collect())
}

fn count(n: usize) -> Object {
    Object::Integer(i64::try_from(n).unwrap_or(i64::MAX))
}

/// Subsets and writes every font text was drawn with.
fn write_fonts(cos: &mut CosMut<'_>, state: &mut BuildState) -> Result<()> {
    let fonts = std::mem::replace(&mut state.fonts, FontRegistry::new());
    fonts.finish_fonts(cos)?;
    Ok(())
}

/// Writes the pending link annotations with their targets resolved.
fn write_links(cos: &mut CosMut<'_>, state: &BuildState, targets: &Targets<'_>) -> Result<()> {
    for link in &state.links {
        let mut dict = link.dict.clone();
        targets.apply(&mut dict, &link.target)?;
        cos.set(link.object, Object::Dict(dict))?;
    }
    Ok(())
}

/// The new pages of an open document: fonts and links (resolved against all its pages).
pub(crate) fn finish_appended(doc: &mut lamina_model::Document, state: &mut BuildState) -> Result<()> {
    let pages: Vec<(ObjRef, PageFrame)> = {
        let pages = doc.pages()?;
        let mut out = Vec::with_capacity(pages.len());
        for page in pages.iter() {
            let r = page
                .obj_ref()
                .ok_or_else(|| invalid("a page is a direct object, so links can't point at it"))?;
            out.push((r, page.frame()?));
        }
        out
    };
    let cos = doc.cos_mut();
    let mut edit = cos.edit();
    write_fonts(&mut edit, state)?;
    write_links(
        &mut edit,
        state,
        &Targets {
            pages: &pages,
            names: None,
        },
    )?;
    Ok(())
}

/// The Info values: the metadata's, falling back to the options' producer and date.
fn merged_metadata(options: &BuildOptions, features: &DocFeatures) -> Metadata {
    let mut meta = features.metadata.clone().unwrap_or_default();
    if meta.producer.is_none() {
        meta.producer = options.producer().map(str::to_owned);
    }
    if meta.creation_date.is_none() {
        meta.creation_date = options.creation_date().copied();
    }
    meta
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
    let mut cos = doc.edit();
    write_fonts(&mut cos, state)?;
    let pages: Vec<(ObjRef, PageFrame)> = state.pages.iter().copied().zip(state.frames.iter().copied()).collect();
    let targets = Targets {
        pages: &pages,
        names: Some(&features.names),
    };
    write_links(&mut cos, state, &targets)?;

    let mut root = Dict::new();
    root.insert("Type", Name::from("Pages"));
    root.insert("Kids", kids(&state.pages));
    root.insert("Count", count(state.pages.len()));
    cos.set(state.pages_root, Object::Dict(root))?;

    // Everything after /Type, /Version and /Pages.
    let mut cat = Dict::new();
    let mut required = state.required;
    if let Some(tree) = targets.dests_tree()? {
        let mut names = Dict::new();
        names.insert("Dests", tree);
        cat.insert("Names", names);
        required = required.max((1, 2));
    }
    if let Some(o) = &features.outline {
        outline::write(&mut cos, o.root, &o.entries, &o.ids, &targets)?;
        cat.insert("Outlines", o.root);
        if o.entries.iter().any(outline::OutlineEntry::styled) {
            required = required.max((1, 4));
        }
    }
    if let Some(ranges) = &features.labels {
        cat.insert("PageLabels", labels::labels_tree(ranges, state.pages.len())?);
        required = required.max((1, 3));
    }
    if let Some(prefs) = &features.prefs {
        required = required.max(prefs.write(&mut cat));
    }
    if let Some(lang) = &features.lang {
        cat.insert("Lang", PdfString::literal(encode_text_string(lang)));
        required = required.max((1, 4));
    }
    let meta = merged_metadata(options, features);
    if features.metadata.is_some() {
        let mut d = Dict::new();
        d.insert("Type", Name::from("Metadata"));
        d.insert("Subtype", Name::from("XML"));
        let stream = Stream::new(d, xmp::packet(&meta).into_bytes());
        cat.insert("Metadata", cos.create(Object::Stream(stream))?);
        required = required.max((1, 4));
    }
    let mut head = Dict::new();
    head.insert("Type", Name::from("Catalog"));
    if required > version_of(options.version().as_str().as_bytes()).unwrap_or((1, 7)) {
        head.insert("Version", version_name(required));
    }
    head.insert("Pages", state.pages_root);
    for (key, value) in cat.iter() {
        head.insert(key.clone(), value.clone());
    }
    cos.set(catalog, Object::Dict(head))?;

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
