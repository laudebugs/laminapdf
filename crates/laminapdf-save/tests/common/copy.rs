//! Pages copied with `ObjectCopier` (todo E2p) into a document's page tree, for the copy cases
//! of the round-trip suite (`roundtrip.rs`) and the `copy_pages` fuzz target (`fuzz_entry.rs`).
//! Putting a copied page in the tree is E2c's op; until then this does it by hand: appended to
//! the root `/Pages` node.
#![allow(
    unreachable_pub,
    reason = "shared by the tests, the `roundtrip` example and the fuzzer, as `tests/common` helpers are"
)]

use laminapdf_cos::{Document, Error, Limits, ObjRef, Object, Result};
use laminapdf_model::ObjectCopier;

/// A file with a catalog (object 1) and an empty page tree (object 2).
#[must_use]
pub fn empty_file() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [] /Count 0 >>",
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 3\n0000000000 65535 f\r\n");
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// Copies `pages` of `source` into `target` and appends them to its root page-tree node.
///
/// # Errors
/// What the copier returns; `InvalidStructure` when `target`'s catalog has no indirect `/Pages`
/// dictionary with a `/Kids` array.
pub fn copy_pages_into(
    target: &mut Document,
    source: &Document,
    pages: &[&laminapdf_cos::Page],
) -> Result<Vec<ObjRef>> {
    let tree = target
        .catalog()?
        .get("Pages")
        .and_then(Object::as_ref)
        .ok_or_else(no_tree)?;
    let mut edit = target.edit();
    let copies = ObjectCopier::new(source, Limits::default()).copy_pages(&mut edit, pages)?;
    let mut node = edit.get(tree)?.and_then(|o| o.as_dict().cloned()).ok_or_else(no_tree)?;
    let mut kids = node
        .get("Kids")
        .and_then(Object::as_array)
        .ok_or_else(no_tree)?
        .to_vec();
    let count = node.get("Count").and_then(Object::as_int).unwrap_or(0);
    for &copy in &copies {
        kids.push(Object::Ref(copy));
        let mut page = edit.get(copy)?.and_then(|o| o.as_dict().cloned()).ok_or_else(no_tree)?;
        page.insert("Parent", Object::Ref(tree));
        edit.set(copy, Object::Dict(page))?;
    }
    node.insert("Kids", Object::Array(kids));
    node.insert(
        "Count",
        Object::Integer(count + i64::try_from(copies.len()).unwrap_or(0)),
    );
    edit.set(tree, Object::Dict(node))?;
    Ok(copies)
}

fn no_tree() -> Error {
    Error::InvalidStructure {
        kind: "page tree",
        message: "the target has no root /Pages node with /Kids to append to",
    }
}
