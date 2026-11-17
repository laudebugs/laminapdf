//! The structure-tree helpers the ops share (ISO 32000-2 §14.7): an element's kids read as a
//! list, the MCID a kid names on a page, an element's parent, and kids taken out of `/K`.

use laminapdf_cos::{Dict, ObjRef, Object};

use crate::annot::object_dict;
use crate::error::Result;
use crate::pages::{edit_dict, edit_object};

/// An element's `/K` as a list, an indirect array read through.
pub(crate) fn kids_of(doc: &laminapdf_model::Document, dict: &Dict) -> Result<Vec<Object>> {
    Ok(match dict.get("K") {
        Some(Object::Array(kids)) => kids.clone(),
        Some(Object::Ref(r)) => match doc.deref(&Object::Ref(*r))?.as_array() {
            Some(kids) => kids.to_vec(),
            None => vec![Object::Ref(*r)],
        },
        Some(kid) => vec![kid.clone()],
        None => Vec::new(),
    })
}

/// The MCID of marked-content kid `kid` when its content is on `page`: an integer on an
/// element whose page is `element_page`, or an `MCR` (direct or indirect) on its own `/Pg` or
/// the element's. An `MCR` into a form XObject's stream (`/Stm`) is not the page's content.
pub(crate) fn mcid_on(
    doc: &laminapdf_model::Document,
    kid: &Object,
    element_page: Option<ObjRef>,
    page: ObjRef,
) -> Result<Option<u32>> {
    let mcr = match kid {
        Object::Integer(m) => return Ok((element_page == Some(page)).then(|| u32::try_from(*m).ok()).flatten()),
        Object::Dict(d) => d.clone(),
        Object::Ref(r) => match object_dict(doc, *r)? {
            Some(d) => d,
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    let on_page = mcr.get("Pg").and_then(Object::as_ref).or(element_page) == Some(page);
    let is_mcr = mcr.get_name("Type") == Some(b"MCR".as_slice()) && mcr.get("Stm").is_none();
    Ok(if on_page && is_mcr {
        mcr.get_int("MCID").and_then(|m| u32::try_from(m).ok())
    } else {
        None
    })
}

/// `elem`'s parent when it is a structure element (not the tree root, `/Type /StructTreeRoot`).
pub(crate) fn struct_parent(doc: &laminapdf_model::Document, elem: ObjRef) -> Result<Option<ObjRef>> {
    let Some(parent) = doc
        .object(elem.num)?
        .as_dict()
        .and_then(|d| d.get("P"))
        .and_then(Object::as_ref)
    else {
        return Ok(None);
    };
    let is_root = doc.object(parent.num)?.as_dict().and_then(|d| d.get_name("Type")) == Some(b"StructTreeRoot");
    Ok((!is_root).then_some(parent))
}

/// Takes the kids `drop(kid, index)` picks out of `/K` of the dictionary object `holder` (an
/// indirect `/K` array is changed where it is); whether that left `holder` with no kids.
pub(crate) fn drop_kids(
    doc: &mut laminapdf_model::Document,
    holder: ObjRef,
    drop: impl Fn(&Object, usize) -> bool,
) -> Result<bool> {
    let keep = |items: &mut Vec<Object>| {
        let mut i = 0;
        items.retain(|kid| {
            let keep = !drop(kid, i);
            i += 1;
            keep
        });
        items.is_empty()
    };
    let indirect = match object_dict(doc, holder)?.and_then(|d| d.get("K").cloned()) {
        Some(Object::Ref(r)) if doc.deref(&Object::Ref(r))?.as_array().is_some() => Some(r),
        _ => None,
    };
    let mut emptied = false;
    if let Some(array) = indirect {
        edit_object(doc, array, |o: &mut Object| {
            if let Object::Array(items) = o {
                emptied = keep(items);
            }
        })?;
        return Ok(emptied);
    }
    edit_dict(doc, holder, |d| match d.get_mut("K") {
        Some(Object::Array(items)) => {
            emptied = keep(items);
        }
        Some(single) if drop(single, 0) => {
            d.remove("K");
            emptied = true;
        }
        _ => {}
    })?;
    Ok(emptied)
}
