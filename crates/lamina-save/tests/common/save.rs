//! Helpers for the tests of the overlay writers and rebase: editable documents, ops, sources
//! and sinks that misbehave, and the facts a reopened file is compared on.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use lamina_cos::{ByteSource, Object, OpenOptions, PdfString, SourceError};
use lamina_model::{PageId, Rotation};
use lamina_ops::{Document, InfoPatch, Origin, RotatePage, SetInfo};
use lamina_save::{SinkError, WriteSink};

use super::fixture;

/// An editable document over a copy of `bytes`.
pub fn editable(bytes: &[u8]) -> Document {
    Document::open(bytes.to_vec(), OpenOptions::default()).unwrap()
}

/// An editable corpus file.
pub fn corpus_doc(name: &str) -> Document {
    editable(&fixture(&format!("corpus/{name}")))
}

/// A classic file around object bodies numbered `1..`, with `/Size` and `/Root 1 0 R` added
/// to `trailer_extra` (over the shared [`super::classic::classic`]).
pub fn classic(objects: &[&str], trailer_extra: &str) -> Vec<u8> {
    let bodies: Vec<&[u8]> = objects.iter().map(|o| o.as_bytes()).collect();
    let trailer = format!("<< /Size {} /Root 1 0 R {trailer_extra} >>", objects.len() + 1);
    super::classic::classic(&bodies, &trailer)
}

/// The `/ID` used by [`two_pages`].
pub const ID0: &str = "00112233445566778899AABBCCDDEEFF";

/// Catalog 1, pages 2 with pages 3 and 4, Info 5, an orphan object 6 nothing references,
/// and an `/ID`.
pub fn two_pages() -> Vec<u8> {
    classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 200 200] >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Type /Page /Parent 2 0 R /Rotate 90 >>",
            "<< /Title (Base) >>",
            "(orphan)",
        ],
        &format!("/Info 5 0 R /ID [<{ID0}> <{ID0}>]"),
    )
}

/// Catalog 1, pages 2, page 3, an orphan 4 and an xref stream 5 (uncompressed rows,
/// `/W [1 4 2] /Index [0 6]`) that also carries stream keys a trailer must not inherit
/// (`/DL`, `/DecodeParms`).
pub fn xref_stream_base() -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (num, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R >>"),
        (4, "(orphan)"),
    ] {
        offsets.push(out.len());
        out.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref = out.len();
    offsets.push(xref);
    let mut rows = vec![0u8, 0, 0, 0, 0, 0xff, 0xff];
    for off in &offsets {
        rows.push(1);
        rows.extend_from_slice(&u32::try_from(*off).unwrap().to_be_bytes());
        rows.extend_from_slice(&[0, 0]);
    }
    out.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XRef /Size 6 /W [1 4 2] /Index [0 6] /Root 1 0 R /DL {len} \
             /DecodeParms << /Columns 7 >> /Length {len} >>\nstream\n",
            len = rows.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&rows);
    out.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// Catalog 1, pages 2, a free 3 (generation 1, head of the free list), page 4, orphan 5.
pub fn free_list_base() -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (num, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [4 0 R] /Count 1 >>"),
        (4, "<< /Type /Page /Parent 2 0 R >>"),
        (5, "(orphan)"),
    ] {
        offsets.push((num, out.len()));
        out.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 6\n0000000003 65535 f\r\n");
    for (num, off) in offsets {
        if num == 4 {
            out.extend_from_slice(b"0000000000 00001 f\r\n");
        }
        out.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

pub fn set_title(doc: &mut Document, title: &str) {
    doc.apply_all(
        Origin::Local,
        [SetInfo::new(InfoPatch::new().title(Some(title)), None).into()],
    )
    .unwrap();
}

pub fn page_ids(doc: &lamina_model::Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

pub fn rotate(doc: &mut Document, index: usize, rotation: Rotation) {
    let page = page_ids(doc).remove(index);
    doc.apply_all(Origin::Local, [RotatePage::new(page, rotation).into()])
        .unwrap();
}

/// What a test compares between the in-memory view and the reopened file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub rotations: Vec<Rotation>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub keywords: Option<String>,
}

pub fn facts(doc: &lamina_model::Document) -> Facts {
    let info = doc.info().unwrap();
    let text = |key: &str| info.as_ref().and_then(|i| i.text(key).unwrap());
    Facts {
        rotations: doc.pages().unwrap().iter().map(|p| p.rotate().unwrap()).collect(),
        title: text("Title"),
        author: text("Author"),
        keywords: text("Keywords"),
    }
}

/// The document's `/ID` strings.
pub fn ids(doc: &lamina_cos::Document) -> (Vec<u8>, Vec<u8>) {
    let id = doc.deref(doc.trailer().get("ID").unwrap()).unwrap();
    let string = |i: usize| match id.as_array().unwrap().get(i) {
        Some(Object::String(PdfString { bytes, .. })) => bytes.clone(),
        other => panic!("{other:?}"),
    };
    (string(0), string(1))
}

/// A document opened from `bytes` with default options, at the COS level.
pub fn reopen(bytes: &[u8]) -> lamina_model::Document {
    let doc = lamina_model::Document::open(bytes.to_vec(), OpenOptions::default()).unwrap();
    assert!(!doc.was_repaired(), "the output needed repair");
    assert!(doc.warnings().is_empty(), "warnings: {:?}", doc.warnings());
    doc
}

/// A source without an in-memory slice; until [`Windowed::fetch`], bytes from `missing_from`
/// on answer `Pending`.
pub struct Windowed {
    pub data: Vec<u8>,
    pub missing_from: u64,
    pub fetched: AtomicBool,
}

impl Windowed {
    /// All bytes available.
    pub fn new(data: Vec<u8>) -> Arc<Self> {
        Arc::new(Self {
            data,
            missing_from: u64::MAX,
            fetched: AtomicBool::new(true),
        })
    }

    pub fn fetch(&self) {
        self.fetched.store(true, Ordering::Relaxed);
    }
}

impl ByteSource for Windowed {
    fn len(&self) -> u64 {
        u64::try_from(self.data.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        let end = offset + u64::try_from(buf.len()).unwrap();
        if !self.fetched.load(Ordering::Relaxed) && end > self.missing_from {
            return Err(SourceError::Pending {
                offset,
                len: u64::try_from(buf.len()).unwrap(),
            });
        }
        let tail = self.data.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len());
        if let (Some(dst), Some(src)) = (buf.get_mut(..n), tail.get(..n)) {
            dst.copy_from_slice(src);
        }
        Ok(n)
    }
}

/// A sink that takes `room` bytes and then fails.
pub struct FailingSink {
    pub room: usize,
    pub taken: Vec<u8>,
}

impl FailingSink {
    pub const fn new(room: usize) -> Self {
        Self {
            room,
            taken: Vec::new(),
        }
    }
}

impl WriteSink for FailingSink {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        if self.taken.len() + bytes.len() > self.room {
            return Err(SinkError::Failed("disk full".into()));
        }
        self.taken.extend_from_slice(bytes);
        Ok(())
    }
}
