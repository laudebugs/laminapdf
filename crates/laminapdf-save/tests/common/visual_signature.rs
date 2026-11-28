//! A signature-like picture and the three ops that place it (todo E10c): an image stamp, ink
//! and an image as page content. Shared by `visual_signatures.rs` and the round-trip suite.

use laminapdf_model::{Matrix, PdfDate, UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AddContentObject, Alt, AnnotSpec, ContentId, ContentObjectSpec, Document, Ink, InkStroke, Op, Stamp,
    StampKind,
};

/// A 4x4 RGBA PNG (an ink-blue diagonal on a transparent ground), written by hand with a stored
/// deflate block so the test needs no encoder.
pub fn signature_png() -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    }
    let mut raw = Vec::new();
    for y in 0..4 {
        raw.push(0); // filter: none
        for x in 0..4 {
            raw.extend(if x == y { [20, 40, 160, 255] } else { [0, 0, 0, 0] });
        }
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &v in &raw {
        a = (a + u32::from(v)) % 65_521;
        b = (b + a) % 65_521;
    }
    let mut z = vec![0x78, 0x01, 0x01];
    z.extend((raw.len() as u16).to_le_bytes());
    z.extend((!(raw.len() as u16)).to_le_bytes());
    z.extend(&raw);
    z.extend(((b << 16) | a).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, b"IHDR", &[0, 0, 0, 4, 0, 0, 0, 4, 8, 6, 0, 0, 0]);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    png
}

fn id(s: &str) -> ContentId {
    ContentId::new(s).unwrap()
}

/// The three placements on `doc`'s first page: an image stamp, ink and an image as content.
pub fn placements(doc: &mut Document) -> [Op; 3] {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let blob = doc.add_blob(signature_png());
    let date: PdfDate = "D:20261127120000Z".parse().unwrap();
    [
        AddAnnot::new(
            page.clone(),
            AnnotSpec::Stamp(Stamp::new(
                UserSpaceRect::from_corners(72.0, 600.0, 172.0, 640.0),
                StampKind::Image(blob.clone()),
            )),
        )
        .into(),
        AddAnnot::new(
            page.clone(),
            AnnotSpec::Ink(Ink::new(vec![InkStroke::new(vec![
                UserSpacePoint::new(200.0, 600.0),
                UserSpacePoint::new(220.0, 630.0),
                UserSpacePoint::new(240.0, 600.0),
            ])])),
        )
        .into(),
        AddContentObject::new(
            id("signature"),
            page,
            ContentObjectSpec::Image { blob, crop: None },
            Matrix::new([100.0, 0.0, 0.0, 40.0, 300.0, 600.0]),
            date,
        )
        .with_alt(Alt::Text("Handwritten signature".into()))
        .into(),
    ]
}
