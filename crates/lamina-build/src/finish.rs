//! Finishing a build: the page tree (ISO 32000-2 §7.7.3.2), the catalog (§7.7.2), the Info
//! dictionary (§14.3.3) and the trailer, or new pages linked into an existing tree.

use lamina_cos::{Dict, Name, ObjRef, Object, PdfString};
use lamina_syntax::text::encode_text_string;

use crate::builder::{BuildOptions, BuildState};
use crate::error::Result;

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

/// A new document's objects: the reserved page tree root and catalog, Info, the trailer.
pub(crate) fn new_document(
    doc: &mut lamina_cos::Document,
    options: &BuildOptions,
    catalog: ObjRef,
    state: &BuildState,
) -> Result<()> {
    let mut cos = doc.edit();
    let mut root = Dict::new();
    root.insert("Type", Name::from("Pages"));
    root.insert("Kids", kids(&state.pages));
    root.insert("Count", count(state.pages.len()));
    cos.set(state.pages_root, Object::Dict(root))?;

    let mut cat = Dict::new();
    cat.insert("Type", Name::from("Catalog"));
    cat.insert("Pages", state.pages_root);
    cos.set(catalog, Object::Dict(cat))?;

    let mut trailer = Dict::new();
    trailer.insert("Root", catalog);
    let mut info = Dict::new();
    if let Some(producer) = options.producer() {
        info.insert("Producer", PdfString::literal(encode_text_string(producer)));
    }
    if let Some(date) = options.creation_date() {
        info.insert("CreationDate", PdfString::literal(date.to_string()));
    }
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
