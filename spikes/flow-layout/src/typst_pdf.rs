//! Experiment B: Typst embedded. `typst::compile` lays the document out and `typst-pdf` writes it
//! (through krilla); Lamina PDF then opens the finished bytes, edits them and saves incrementally.

use typst::foundations::{Datetime, Smart};
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards, Timestamp};

use crate::typst_world::MemoryWorld;

/// Lays the document out without exporting it (experiment C draws these frames itself).
pub fn layout(world: &MemoryWorld) -> Result<typst_layout::PagedDocument, String> {
    world.compile().output.map_err(|errors| format!("typst: {errors:?}"))
}

/// What one compile produced, and what it cost.
pub struct Output {
    pub pdf: Vec<u8>,
    pub pages: usize,
    pub warnings: usize,
}

/// Compiles `main` (with `files` readable by name) and exports it with a fixed identifier and
/// date, checked against PDF/UA-1 (Typst tags by default).
pub fn build(world: &MemoryWorld) -> Result<Output, String> {
    let warned = world.compile();
    let warnings = warned.warnings.len();
    let doc = warned.output.map_err(|errors| format!("typst: {errors:?}"))?;
    let options = PdfOptions {
        ident: Smart::Custom("lamina-flow-spike".to_owned()),
        timestamp: Datetime::from_ymd_hms(2026, 11, 8, 12, 0, 0).map(Timestamp::new_utc),
        standards: PdfStandards::new(&[PdfStandard::Ua_1]).map_err(|e| format!("{e:?}"))?,
        ..PdfOptions::default()
    };
    let pdf = typst_pdf::pdf(&doc, &options).map_err(|errors| format!("typst-pdf: {errors:?}"))?;
    Ok(Output { pdf, pages: doc.pages().len(), warnings })
}

/// What Lamina PDF can do with Typst's bytes: open them, count pages, set the title and save
/// that edit as an incremental update, then reopen the result.
pub struct EditCheck {
    pub pages: usize,
    pub tagged: bool,
    pub incremental_bytes: usize,
    pub reopened_title: Option<String>,
}

pub fn edit(pdf: &[u8]) -> Result<EditCheck, laminapdf::Error> {
    let mut doc = laminapdf::Document::open_bytes(pdf.to_vec(), laminapdf::OpenOptions::default())?;
    let pages = doc.page_count()? as usize;
    let tagged = doc.is_tagged()?;
    doc.apply_log(
        r#"{"v":1,"ops":[{"op":"setInfo","args":{"patch":{"title":"Edited after Typst"},"at":"D:20261108120000Z"}}]}"#,
    )?;
    let saved = doc.save(&laminapdf::SaveOptions::default())?;
    let incremental_bytes = saved.bytes.len().saturating_sub(pdf.len());
    let reopened = laminapdf::Document::open_bytes(saved.bytes, laminapdf::OpenOptions::default())?;
    let reopened_title = reopened.info()?.title;
    Ok(EditCheck { pages, tagged, incremental_bytes, reopened_title })
}
