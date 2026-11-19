//! Shared helpers for laminapdf-ops' integration tests: opening documents, small hand-built
//! files and overlay snapshots. Fixtures and the classic-file builder come from
//! `laminapdf-test-fixtures`.
#![allow(
    deprecated,
    reason = "these tests check that SetInfo, deprecated but still on the wire, keeps working"
)]
#![allow(
    dead_code,
    unused_imports,
    unreachable_pub,
    reason = "each test crate uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

use laminapdf_cos::{Dict, Object, OverlayEntry};

pub mod annot_fuzz_entry;
pub mod rich_text_fuzz_entry;
use laminapdf_model::{PageId, Rotation};
use laminapdf_ops::{Document, InfoPatch, Origin, RotatePage, SetInfo};
pub use laminapdf_test_fixtures::{classic, classic_with_root, fixture, fixture_text, fixtures_dir, stream_body};

pub fn open(bytes: &[u8]) -> Document {
    Document::open(bytes.to_vec(), laminapdf_cos::OpenOptions::default()).unwrap()
}

pub fn corpus(name: &str) -> Document {
    open(&fixture(&format!("corpus/{name}")))
}

/// Three pages under a `/Pages` node with `/Rotate 90` (pages 4 and 5 inherit it, page 6
/// has its own `/Rotate 180`), and an Info dictionary (object 3) with three entries.
pub fn inherited_rotate() -> Document {
    open(&classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [4 0 R 5 0 R 6 0 R] /Count 3 /Rotate 90 /MediaBox [0 0 100 100] >>",
            "<< /Title (Base) /Author (Someone) /Subject (Things) >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Type /Page /Parent 2 0 R /Rotate 180 >>",
        ],
        "/Info 3 0 R",
    ))
}

/// The overlay and the overlay's trailer, for comparing states.
pub fn snapshot(doc: &Document) -> (Vec<(u32, OverlayEntry)>, Option<Dict>) {
    (
        doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect(),
        doc.overlay().trailer().cloned(),
    )
}

pub fn page_ids(doc: &laminapdf_model::Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

pub fn rotations(doc: &laminapdf_model::Document) -> Vec<Rotation> {
    doc.pages().unwrap().iter().map(|p| p.rotate().unwrap()).collect()
}

pub fn title(doc: &laminapdf_model::Document) -> Option<String> {
    doc.info().unwrap().and_then(|i| i.title().unwrap())
}

pub fn author(doc: &laminapdf_model::Document) -> Option<String> {
    doc.info().unwrap().and_then(|i| i.author().unwrap())
}

pub fn set_title(doc: &mut Document, origin: Origin, title: Option<&str>) {
    doc.apply_all(origin, [SetInfo::patch(InfoPatch::new().title(title)).into()])
        .unwrap();
}

pub fn rotate(doc: &mut Document, page: &PageId, rotate: Rotation) {
    doc.apply_all(Origin::Local, [RotatePage::new(page.clone(), rotate).into()])
        .unwrap();
}

/// The content of the normal appearance of `name`'s first widget.
pub fn drawn(doc: &Document, name: &str) -> String {
    let widget = doc
        .form()
        .unwrap()
        .field_named(name)
        .cloned()
        .unwrap()
        .widgets
        .first()
        .unwrap()
        .id
        .as_lamina_id()
        .obj_ref()
        .unwrap();
    let resolve = |o: &Object| doc.cos().deref(o).unwrap().into_owned();
    let dict = resolve(&Object::Ref(widget));
    let normal = resolve(dict.as_dict().unwrap().get("AP").unwrap())
        .as_dict()
        .unwrap()
        .get("N")
        .cloned()
        .unwrap();
    let Object::Stream(form) = resolve(&normal) else {
        panic!("no appearance stream")
    };
    String::from_utf8_lossy(&doc.cos().decode_stream(&form).unwrap()).into_owned()
}

/// One untagged page and an XMP packet with `claim` (attributes of `rdf:Description`), such as
/// `pdfuaid:part="1"`.
pub fn claiming(claim: &str) -> Document {
    let xmp = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" {claim}/></rdf:RDF></x:xmpmeta>"#
    );
    let metadata = stream_body("/Type /Metadata /Subtype /XML", xmp.as_bytes());
    open(&classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>",
            &metadata,
        ],
        "<< /Size 5 /Root 1 0 R >>",
    ))
}

/// The XObjects the first page draws, and those its forms draw, by kind.
#[derive(Debug, Default)]
pub struct Drawn {
    /// The image XObjects' dictionaries.
    pub images: Vec<Dict>,
    /// The Form XObjects' dictionaries.
    pub forms: Vec<Dict>,
}

pub fn drawn_xobjects(doc: &Document) -> Drawn {
    fn walk(doc: &Document, resources: Option<&Dict>, out: &mut Drawn, depth: u8) {
        let Some(xobjects) = resources
            .and_then(|r| r.get("XObject"))
            .map(|x| doc.cos().deref(x).unwrap().into_owned())
        else {
            return;
        };
        for (_, x) in xobjects.as_dict().unwrap().iter() {
            let Object::Stream(s) = doc.cos().deref(x).unwrap().into_owned() else {
                continue;
            };
            if s.dict.get_name("Subtype") == Some(b"Image".as_slice()) {
                out.images.push(s.dict.clone());
            } else {
                let inner = s
                    .dict
                    .get("Resources")
                    .map(|r| doc.cos().deref(r).unwrap().into_owned());
                out.forms.push(s.dict.clone());
                if depth < 3 {
                    walk(doc, inner.as_ref().and_then(Object::as_dict), out, depth + 1);
                }
            }
        }
    }
    let mut out = Drawn::default();
    let pages = doc.pages().unwrap();
    walk(doc, pages.get(0).unwrap().resources(), &mut out, 0);
    out
}
