//! Writing a bookmark's target: `/Dest` for a page or a name, `/A` for a web address.

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::PageId;

use super::names::Stores;
use super::types::{ExplicitDestination, OutlineTarget};
use crate::error::{Error, Result};

/// Sets `dict`'s target: `/Dest` or `/A`, removing the other.
pub(super) fn write(doc: &laminapdf_model::Document, dict: &mut Dict, target: &OutlineTarget) -> Result<()> {
    match target {
        OutlineTarget::Destination(dest) => {
            dict.insert("Dest", explicit(doc, dest)?);
            dict.remove("A");
        }
        OutlineTarget::Named(name) => {
            if name.is_empty() {
                return Err(Error::EmptyDestinationName);
            }
            // A name the catalog's `/Dests` holds resolves only as a name object.
            let value = if Stores::read(doc)?.in_dests_dict(name.as_bytes()) {
                Object::name(name)
            } else {
                Object::String(PdfString::literal(name.as_bytes().to_vec()))
            };
            dict.insert("Dest", value);
            dict.remove("A");
        }
        OutlineTarget::Uri(uri) => {
            let action: Dict = [
                ("S", Object::name("URI")),
                (
                    "URI",
                    Object::String(PdfString::literal(checked_uri(uri)?.as_bytes().to_vec())),
                ),
            ]
            .into_iter()
            .collect();
            dict.insert("A", Object::Dict(action));
            dict.remove("Dest");
        }
    }
    Ok(())
}

/// `[page /XYZ …]`, with the page's object.
pub(super) fn explicit(doc: &laminapdf_model::Document, dest: &ExplicitDestination) -> Result<Object> {
    let page = page_ref(doc, &dest.page)?;
    let mut array = vec![Object::Ref(page)];
    array.extend(dest.view.to_pdf().ok_or(Error::ViewNotFinite)?);
    Ok(Object::Array(array))
}

fn page_ref(doc: &laminapdf_model::Document, id: &PageId) -> Result<ObjRef> {
    let pages = doc.pages()?;
    let page = pages.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
    page.obj_ref().ok_or_else(|| Error::DirectPage(id.clone()))
}

/// The address when it is absolute, ASCII without spaces or controls, and `http`, `https` or
/// `mailto`; a `URI` action's string is 7-bit ASCII (§12.6.4.8), and only these schemes are
/// safe for a reader to open.
pub(crate) fn checked_uri(uri: &str) -> Result<&str> {
    let printable = uri.bytes().all(|b| b.is_ascii_graphic());
    let Some((scheme, rest)) = uri.split_once(':') else {
        return Err(Error::LinkUriRefused);
    };
    let ok = match scheme.to_ascii_lowercase().as_str() {
        "http" | "https" => rest
            .strip_prefix("//")
            .is_some_and(|r| !r.is_empty() && !r.starts_with('/')),
        "mailto" => !rest.is_empty(),
        _ => false,
    };
    if printable && ok {
        Ok(uri)
    } else {
        Err(Error::LinkUriRefused)
    }
}
