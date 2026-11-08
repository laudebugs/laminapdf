//! Hand-built classic files (a header, numbered objects, a correct xref table) for
//! tests that need exact bytes rather than what a writer produces.

use std::fmt::Write as _;

/// A classic file around object bodies numbered `1..`, with a correct xref table and the
/// trailer dictionary `trailer` written as given.
#[must_use]
pub fn classic(objects: &[&[u8]], trailer: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n{trailer}\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// [`classic`] with object 1 as the catalog. The trailer is
/// `<< /Size … /Root 1 0 R extra >>`, so `extra` adds entries such as `/Info 3 0 R`.
#[must_use]
pub fn classic_with_root(objects: &[&str], extra: &str) -> Vec<u8> {
    let bodies: Vec<&[u8]> = objects.iter().map(|o| o.as_bytes()).collect();
    let trailer = format!("<< /Size {} /Root 1 0 R {extra} >>", objects.len() + 1);
    classic(&bodies, &trailer)
}

/// A stream object body: `dict` (without `/Length`) plus `data`, with a correct `/Length`.
#[must_use]
pub fn stream_body(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// `base` with an incremental update (ISO 32000-2 §7.5.6) that writes `objects` (number and
/// body) after it, then a classic xref section and the trailer `<< trailer /Prev … >>`.
///
/// # Panics
/// When `base` has no `startxref` offset.
#[must_use]
pub fn update(base: &[u8], objects: &[(u32, &[u8])], trailer: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(base);
    let prev: usize = text
        .rsplit("startxref")
        .next()
        .and_then(|tail| tail.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap();
    let mut out = base.to_vec();
    let mut xref = String::from("xref\n0 1\n0000000000 65535 f\r\n");
    for (num, body) in objects {
        writeln!(xref, "{num} 1\n{:010} 00000 n\r", out.len()).unwrap();
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let start = out.len();
    out.extend_from_slice(xref.as_bytes());
    out.extend_from_slice(format!("trailer\n<< {trailer} /Prev {prev} >>\nstartxref\n{start}\n%%EOF\n").as_bytes());
    out
}

/// A file that holds something in each of `laminapdf reveal`'s seven categories, over two
/// revisions: the update rewrites page 1's content (object 4) without the sentence
/// "Removed draft wording." It has an Info dictionary with the custom key `/SourceModified`,
/// an XMP packet, an attached `notes.txt`, a JavaScript `/OpenAction`, a link whose action is
/// `Launch`, a `Text` annotation by "Reviewer" saying "Fix the totals", a text field `name`
/// filled with "Alice", a layer "Draft notes" off by default, and the word "classified"
/// under a black box.
#[must_use]
pub fn disclosing() -> Vec<u8> {
    let page = |removed: bool| {
        let mut content = String::from("BT /F1 12 Tf 72 720 Td (Quarterly report.) Tj ET ");
        if !removed {
            content.push_str("BT /F1 12 Tf 72 680 Td (Removed draft wording.) Tj ET ");
        }
        content.push_str("BT /F1 12 Tf 72 700 Td (classified) Tj ET 0 g 70 696 80 16 re f");
        stream_body("", content.as_bytes())
    };
    let xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"/></rdf:RDF></x:xmpmeta>"#;
    let first = page(false);
    let objects: [&[u8]; 15] = [
        b"<< /Type /Catalog /Pages 2 0 R /Metadata 7 0 R /OpenAction 8 0 R /AcroForm << /Fields [10 0 R] >> /OCProperties << /OCGs [11 0 R] /D << /OFF [11 0 R] >> >> /Names << /EmbeddedFiles 12 0 R >> >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> /Annots [9 0 R 13 0 R 10 0 R] >>",
        &first,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        b"<< /Title (Quarterly report) /Producer (Report Tool 2.1) /SourceModified (D:20240101120000Z) >>",
        &stream_body("/Type /Metadata /Subtype /XML", xmp),
        b"<< /S /JavaScript /JS (app.alert\\('opened'\\);) >>",
        b"<< /Type /Annot /Subtype /Link /Rect [72 600 200 620] /A << /S /Launch /F (setup.exe) >> >>",
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /V (Alice) /Rect [72 500 200 520] /P 3 0 R >>",
        b"<< /Type /OCG /Name (Draft notes) >>",
        b"<< /Names [(notes.txt) 14 0 R] >>",
        b"<< /Type /Annot /Subtype /Text /Rect [300 700 320 720] /T (Reviewer) /Contents (Fix the totals) >>",
        b"<< /Type /Filespec /F (notes.txt) /UF (notes.txt) /EF << /F 15 0 R >> >>",
        &stream_body("/Type /EmbeddedFile /Subtype /text#2Fplain /Params << /Size 12 >>", b"secret notes"),
    ];
    let base = classic(&objects, "<< /Size 16 /Root 1 0 R /Info 6 0 R >>");
    update(&base, &[(4, &page(true))], "/Size 16 /Root 1 0 R /Info 6 0 R")
}

#[cfg(test)]
mod tests {
    use super::{classic, classic_with_root};

    #[test]
    fn the_xref_offsets_point_at_each_object() {
        let file = classic(&[b"<< >>", b"42"], "<< /Size 3 >>");
        let text = String::from_utf8(file).unwrap();
        let second = text.find("2 0 obj").unwrap();
        assert!(text.contains(&format!("{second:010} 00000 n\r\n")), "{text}");
    }

    #[test]
    fn classic_with_root_names_object_1_the_root() {
        let file = classic_with_root(&["<< /Type /Catalog >>"], "/Info 2 0 R");
        let text = String::from_utf8(file).unwrap();
        assert!(
            text.contains("trailer\n<< /Size 2 /Root 1 0 R /Info 2 0 R >>\n"),
            "{text}"
        );
    }
}
