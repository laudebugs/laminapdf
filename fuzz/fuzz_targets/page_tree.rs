//! Walks fuzzed page trees. A whole file (starting `%PDF`) is opened as is; anything else is
//! the body of the `/Pages` object (2) in a small file whose other objects are a catalog, a
//! page and a second tree node, so the fuzzer explores page-tree shapes directly: kids,
//! cycles, inheritance, depth, direct pages (and their path IDs).
#![no_main]

use lamina_model::{Document, OpenOptions};
use libfuzzer_sys::fuzz_target;

fn wrap(body: &[u8]) -> Vec<u8> {
    let objects: [&[u8]; 4] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        body,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90 >>",
        b"<< /Type /Pages /Kids [3 0 R 2 0 R] /Count 2 /CropBox [0 0 10 10] >>",
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 5\n0000000000 65535 f\r\n");
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

fuzz_target!(|data: &[u8]| {
    let bytes = if data.starts_with(b"%PDF") {
        data.to_vec()
    } else {
        wrap(data)
    };
    let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
        return;
    };
    if let Ok(pages) = doc.pages() {
        for page in pages.iter() {
            let _ = page.media_box();
            let _ = page.crop_box();
            let _ = page.rotate();
            let _ = page.label();
            let id = page.id().to_string();
            let parsed: lamina_model::PageId = id.parse().expect("page IDs round-trip");
            assert_eq!(pages.by_id(&parsed).map(|p| p.index()), Some(page.index()));
        }
    }
});
