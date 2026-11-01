//! Experiment B: graph integration.
//!
//! Route 1 (krilla → bytes → lamina): see [`crate::graph::append_pages`].
//! Route 2 (pdf-writer `Chunk` in lamina's numbering space): pdf-writer lets the caller choose
//! every `Ref`, so a builder can allocate numbers from a lamina document and write typed
//! objects with pdf-writer; the chunk's bytes are then parsed back into lamina `Object`s.
//! krilla cannot do this: its `SerializeContext` allocates refs from 1 internally and
//! `ChunkContainer::finish` renumbers everything into a fresh `Pdf` (all `pub(crate)`).

use anyhow::{Result, anyhow};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_save::{XrefEol, incremental_update};
use laminapdf_syntax::{Parser, find};
use pdf_writer::{Chunk, Content, Finish, Name, Rect, Ref, Str};

use crate::graph::next_free;

/// Parses a pdf-writer chunk (`n 0 obj … endobj` repeated) into lamina objects.
pub fn parse_chunk(bytes: &[u8]) -> Result<Vec<(ObjRef, Object)>> {
    let mut out = Vec::new();
    let mut p = Parser::new(bytes, 0);
    loop {
        // Skip white-space between objects; stop at the end.
        let rest = bytes.get(p.pos()..).unwrap_or_default();
        let Some(skip) = rest.iter().position(|b| !b.is_ascii_whitespace()) else {
            break;
        };
        p = Parser::new(bytes, p.pos() + skip);
        let parsed = p.indirect_object(&mut |_| None)?;
        out.push((parsed.id, parsed.object));
        let end = find(bytes, b"endobj", p.pos()).ok_or_else(|| anyhow!("missing endobj"))?;
        p = Parser::new(bytes, end + b"endobj".len());
    }
    Ok(out)
}

/// Writes one extra page with pdf-writer, numbered directly in `base`'s space, and appends it
/// to `base` as an incremental update. Returns the new file and the number of objects.
pub fn append_page_via_chunk(base: &[u8]) -> Result<(Vec<u8>, usize)> {
    let doc = laminapdf_cos::Document::open(base.to_vec(), laminapdf_cos::OpenOptions::default())?;
    let catalog = doc.catalog()?;
    let pages_ref = catalog
        .get("Pages")
        .and_then(Object::as_ref)
        .ok_or_else(|| anyhow!("/Pages"))?;
    let mut pages = doc
        .resolve(pages_ref)?
        .as_dict()
        .cloned()
        .ok_or_else(|| anyhow!("pages"))?;

    let mut next = i32::try_from(next_free(&doc))?;
    let mut alloc = || {
        let r = Ref::new(next);
        next += 1;
        r
    };
    let (page_id, content_id, font_id, annot_id) = (alloc(), alloc(), alloc(), alloc());
    let parent = Ref::new(i32::try_from(pages_ref.num)?);

    let mut chunk = Chunk::new();
    let mut c = Content::new();
    c.begin_text()
        .set_font(Name(b"F1"), 18.0)
        .next_line(56.0, 760.0)
        .show(Str(b"Appended with pdf-writer"))
        .end_text();
    chunk.stream(content_id, &c.finish().into_vec());
    chunk.type1_font(font_id).base_font(Name(b"Helvetica"));
    let mut page = chunk.page(page_id);
    page.parent(parent)
        .media_box(Rect::new(0.0, 0.0, 595.0, 842.0))
        .contents(content_id)
        .annotations([annot_id]);
    page.resources().fonts().pair(Name(b"F1"), font_id);
    page.finish();
    let mut annot = chunk.annotation(annot_id);
    annot
        .subtype(pdf_writer::types::AnnotationType::Link)
        .rect(Rect::new(56.0, 750.0, 300.0, 780.0));
    annot
        .action()
        .action_type(pdf_writer::types::ActionType::Uri)
        .uri(Str(b"https://example.com"));
    annot.finish();

    let mut changes = parse_chunk(chunk.as_bytes())?;
    let objects = changes.len();
    // /Kids may be an indirect array; never drop existing pages.
    let kids_obj = pages.get("Kids").cloned().unwrap_or(Object::Null);
    let mut kids = doc
        .deref(&kids_obj)?
        .as_array()
        .map(<[Object]>::to_vec)
        .ok_or_else(|| anyhow!("/Pages /Kids is not an array"))?;
    kids.push(Object::Ref(ObjRef::new(u32::try_from(page_id.get())?, 0)));
    pages.insert("Count", pages.get_int("Count").unwrap_or(0) + 1);
    pages.insert("Kids", kids);
    changes.push((pages_ref, Object::Dict(pages)));
    Ok((incremental_update(&laminapdf_cos::Document::open(base.to_vec(), laminapdf_cos::OpenOptions::default())?, &changes, XrefEol::default())?, objects))
}
