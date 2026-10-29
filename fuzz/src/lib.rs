//! Shared helpers for lamina's fuzz targets: drive every read path of an opened document,
//! as `crates/lamina-cos/tests/common::exercise` does, plus every `lamina-model` view.
//! Errors are expected; the invariants are no panic, no hang, bounded memory (ADR 0018).

use lamina_cos::ObjRef;
use lamina_model::{Document, Result};

/// Reads everything a viewer would: the catalog, every object, every page's content and
/// fields, the outline, Info, name trees and page labels. Errors are ignored.
pub fn exercise(doc: &Document) {
    let _ = doc.catalog();
    let _ = views(doc);
    let nums: Vec<u32> = doc.object_numbers().collect();
    for num in nums {
        let _ = doc.resolve(ObjRef::new(num, 0));
    }
}

fn views(doc: &Document) -> Result<()> {
    let pages = doc.pages()?;
    for page in pages.iter() {
        let _ = doc.page_content(page.cos());
        let _ = page.media_box();
        let _ = page.crop_box();
        let _ = page.bleed_box();
        let _ = page.trim_box();
        let _ = page.art_box();
        let _ = page.rotate();
        let _ = page.user_unit();
        let _ = page.contents();
        let _ = page.annots();
        let _ = page.label();
        let _ = page.id().to_string();
    }
    let outline = doc.outline()?;
    for item in outline.iter() {
        let _ = item.title();
        let _ = item.count();
        let _ = item.action();
        let _ = item.target();
        let _ = item.children().count();
    }
    if let Some(info) = doc.info()? {
        for key in ["Title", "Author", "Subject", "Keywords", "Creator", "Producer"] {
            let _ = info.text(key);
        }
        let _ = info.creation_date();
        let _ = info.mod_date();
    }
    if let Some(dests) = doc.name_tree("Dests")? {
        let _ = dests.entries();
    }
    let _ = doc.page_labels();
    Ok(())
}
