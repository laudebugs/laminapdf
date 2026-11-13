//! The `extract` fuzz target's entry point (ADR 0018 tier 9). `fuzz/fuzz_targets/extract.rs`
//! calls it (through `fuzz/src/lib.rs`, which includes this file), and `tests/regressions.rs`
//! replays every saved crash input (`fixtures/regressions/extract/`) through it.

use laminapdf_extract::{ExtractOptions, ReadingOrder, extract as run};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_syntax::CancelToken;

/// Extracts every page of `data` (a whole file; anything else is ignored) by layout and by
/// content order. Invariants: no panic, no hang, every block's box finite with its normalised
/// box in [0, 1], block IDs unique, no hidden run among a block's spans, and no block text
/// longer than its own spans (so nothing hidden or from elsewhere reaches it: joining lines
/// only adds spaces and removes hyphens).
pub fn extract(data: &[u8]) {
    if !data.starts_with(b"%PDF") {
        return;
    }
    let Ok(doc) = Document::open(data.to_vec(), OpenOptions::default()) else {
        return;
    };
    for order in [ReadingOrder::Layout, ReadingOrder::ContentStream] {
        let mut options = ExtractOptions::new().with_reading_order(order);
        options.drop.keep_spans = true;
        let Ok(out) = run(&doc, &options, &CancelToken::new()) else {
            continue;
        };
        let mut ids = std::collections::BTreeSet::new();
        for block in &out.blocks {
            assert!(block.bbox.is_finite(), "{}: box not finite", block.id);
            if let Some(norm) = block.bbox_norm {
                assert!(norm.iter().all(|v| (0.0..=1.0).contains(v)), "{}: {norm:?}", block.id);
            }
            assert!(ids.insert(block.id.clone()), "{}: id used twice", block.id);
            assert!(
                block.spans.iter().all(|s| !s.flags.any()),
                "{}: a flagged span",
                block.id
            );
            let visible = |t: &str| t.chars().filter(|c| !c.is_whitespace()).count();
            let own: usize = block.spans.iter().map(|s| visible(&s.text)).sum();
            assert!(visible(&block.text) <= own, "{}: text longer than its spans", block.id);
        }
    }
}

/// The filters, colour spaces and bit depths `extract_images` builds an image from.
const FILTERS: [&str; 8] = [
    "",
    "/Filter /FlateDecode",
    "/Filter /LZWDecode",
    "/Filter /ASCII85Decode",
    "/Filter /RunLengthDecode",
    "/Filter /DCTDecode",
    "/Filter /CCITTFaxDecode /DecodeParms << /K -1 /Columns 16 >>",
    "/Filter /JBIG2Decode",
];
const SPACES: [&str; 6] = [
    "/ColorSpace /DeviceGray",
    "/ColorSpace /DeviceRGB",
    "/ColorSpace /DeviceCMYK",
    "/ColorSpace [/Indexed /DeviceRGB 3 <FF000000FF000000FFFFFFFF>]",
    "/ColorSpace [/Lab << /WhitePoint [0.95 1 1.09] >>]",
    "/ImageMask true",
];
const BITS: [u8; 5] = [1, 2, 4, 8, 16];

/// A one-page file whose page draws one image: `data` its stream, its dictionary chosen by
/// `head` (filter, colour space, bits, size), with a soft mask on every other choice.
fn image_file(head: &[u8], data: &[u8]) -> Vec<u8> {
    let at = |i: usize| head.get(i).copied().map_or(0, usize::from);
    let width = at(3) % 64 + 1;
    let height = at(4) % 64 + 1;
    let smask = if at(5) % 2 == 1 { " /SMask 6 0 R" } else { "" };
    let dict = format!(
        "/Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent {} {} {}{smask}",
        BITS.get(at(2) % BITS.len()).copied().unwrap_or(8),
        SPACES.get(at(1) % SPACES.len()).copied().unwrap_or_default(),
        FILTERS.get(at(0) % FILTERS.len()).copied().unwrap_or_default(),
    );
    let content = b"q 100 0 0 100 0 0 cm /Im0 Do Q";
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>"
            .to_vec(),
        [format!("<< /Length {} >>\nstream\n", content.len()).as_bytes(), content, b"\nendstream"].concat(),
        [format!("<< {dict} /Length {} >>\nstream\n", data.len()).as_bytes(), data, b"\nendstream"].concat(),
        [
            format!("<< /Type /XObject /Subtype /Image /Width 4 /Height 4 /BitsPerComponent 8 /ColorSpace /DeviceGray /Length {} >>\nstream\n", data.len().min(16)).as_bytes(),
            data.get(..16).unwrap_or(data),
            b"\nendstream",
        ]
        .concat(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Extracts the images of `data`: a whole file as it is, anything else as the stream of one
/// image whose dictionary its first six bytes choose. Invariants: no panic, no hang, an image is
/// given at most once per object, a PNG starts with PNG's signature and holds its IHDR's size,
/// and a JPEG or JPEG 2000 is never empty.
pub fn extract_images(data: &[u8]) {
    let bytes = if data.starts_with(b"%PDF") {
        data.to_vec()
    } else {
        let (head, rest) = data.split_at(data.len().min(6));
        image_file(head, rest)
    };
    let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
        return;
    };
    let Ok(out) = laminapdf_extract::extract_images(&doc, &laminapdf_extract::ImageOptions::default()) else {
        return;
    };
    let mut seen = std::collections::BTreeSet::new();
    for image in &out.images {
        if let Some(object) = image.object {
            assert!(
                seen.insert((object, format!("{:?}", image.kind))),
                "{object:?} given twice as {:?}",
                image.kind
            );
        }
        match image.format {
            laminapdf_extract::ImageFormat::Png => {
                assert!(
                    image.data.starts_with(b"\x89PNG\r\n\x1a\n"),
                    "a PNG without its signature"
                );
                let size = image.data.get(16..24).unwrap_or_default();
                assert_eq!(size.get(..4), Some(&image.width.to_be_bytes()[..]), "IHDR's width");
                assert_eq!(size.get(4..), Some(&image.height.to_be_bytes()[..]), "IHDR's height");
            }
            _ => assert!(!image.data.is_empty(), "an empty JPEG"),
        }
    }
}
