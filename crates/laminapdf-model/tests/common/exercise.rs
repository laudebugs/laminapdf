//! Drives every read path of an opened document: the COS reads `laminapdf-cos`'s
//! `tests/common::exercise` covers plus every `laminapdf-model` view. Shared by the integration
//! tests and the fuzz targets (`fuzz/src/lib.rs` includes this file), so the two can't drift.
//!
//! Each view runs on its own: a failing page tree still lets the outline, Info, name trees
//! and page labels be read (and fuzzed).

use laminapdf_cos::ObjRef;
use laminapdf_model::{Document, Result};

/// Reads everything a viewer would; returns the number of calls that failed. Errors are
/// fine; the point is that nothing panics or hangs.
pub fn exercise(doc: &Document) -> usize {
    let nums: Vec<u32> = doc.object_numbers().collect();
    let objects = nums.into_iter().map(|num| doc.resolve(ObjRef::new(num, 0)).map(drop));
    [
        doc.catalog().map(drop),
        pages(doc),
        outline(doc),
        info(doc),
        doc.xmp().map(drop),
        name_trees(doc),
        doc.page_labels().map(drop),
    ]
    .into_iter()
    .chain(objects)
    .filter(Result::is_err)
    .count()
}

fn pages(doc: &Document) -> Result<()> {
    let pages = doc.pages()?;
    let mut failed = Ok(());
    for page in pages.iter() {
        let reads = [
            doc.page_content(page.cos()).map(drop),
            page.media_box().map(drop),
            page.crop_box().map(drop),
            page.bleed_box().map(drop),
            page.trim_box().map(drop),
            page.art_box().map(drop),
            page.rotate().map(drop),
            page.user_unit().map(drop),
            page.contents().map(drop),
            page.annots().map(drop),
            page.label().map(drop),
        ];
        // Content that cannot be decoded (an unsupported filter) is not a view failure.
        for read in reads.into_iter().skip(1) {
            failed = failed.and(read);
        }
        let id = page.id().to_string();
        assert_eq!(
            id.parse::<laminapdf_model::PageId>()
                .ok()
                .and_then(|id| pages.index_of(&id)),
            Some(page.index()),
            "page IDs round-trip"
        );
    }
    failed
}

fn outline(doc: &Document) -> Result<()> {
    let outline = doc.outline()?;
    let mut failed = Ok(());
    for item in outline.iter() {
        failed = failed
            .and(item.title().map(drop))
            .and(item.count().map(drop))
            .and(item.action().map(drop))
            .and(item.target().map(drop));
        let _ = item.children().count();
    }
    failed
}

fn info(doc: &Document) -> Result<()> {
    let Some(info) = doc.info()? else {
        return Ok(());
    };
    let mut failed = Ok(());
    for key in ["Title", "Author", "Subject", "Keywords", "Creator", "Producer"] {
        failed = failed.and(info.text(key).map(drop));
    }
    failed
        .and(info.creation_date().map(drop))
        .and(info.mod_date().map(drop))
}

fn name_trees(doc: &Document) -> Result<()> {
    let Some(dests) = doc.name_tree("Dests")? else {
        return Ok(());
    };
    let entries = dests.entries()?;
    if let Some((key, _)) = entries.first() {
        dests.get(key)?;
    }
    dests.get(b"\xFFmissing").map(drop)
}
