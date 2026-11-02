//! Reader tolerance beyond the spike: repair order, generation numbers, null values in the
//! page tree and in `/Contents`, and nested references in filter parameters. Regression tests
//! for the review of PR #8.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use std::fmt::Write as _;

use common::open;
use laminapdf_cos::{ObjRef, Object, PdfString};
use laminapdf_syntax::filters;

/// One indirect object: number, generation, body.
type Obj<'a> = (u32, u16, &'a [u8]);

/// Header plus objects in the given order, with no xref at all (so opening repairs).
fn objects_only(objects: &[Obj<'_>], tail: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    for &(num, gen_nr, body) in objects {
        out.extend_from_slice(format!("{num} {gen_nr} obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    out.extend_from_slice(tail.as_bytes());
    out
}

/// Objects plus a correct classic xref (one subsection per object, each with its own gen).
fn with_xref(objects: &[Obj<'_>], trailer: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut rows = Vec::new();
    for &(num, gen_nr, body) in objects {
        rows.push((num, gen_nr, out.len()));
        out.extend_from_slice(format!("{num} {gen_nr} obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 1\n0000000000 65535 f\r\n");
    for (num, gen_nr, off) in rows {
        out.extend_from_slice(format!("{num} 1\n{off:010} {gen_nr:05} n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n{trailer}\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// An uncompressed object stream body holding `members` (`num`, serialized object).
fn object_stream(members: &[(u32, &str)]) -> Vec<u8> {
    let mut header = String::new();
    let mut body = String::new();
    for (num, obj) in members {
        write!(header, "{num} {} ", body.len()).unwrap();
        body.push_str(obj);
        body.push('\n');
    }
    let data = format!("{header}\n{body}");
    let first = header.len() + 1;
    format!(
        "<< /Type /ObjStm /N {} /First {first} /Length {} >>\nstream\n{data}\nendstream",
        members.len(),
        data.len()
    )
    .into_bytes()
}

const CATALOG: &[u8] = b"<< /Type /Catalog /Pages 2 0 R >>";
const ONE_PAGE: &[u8] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>";

mod repair_order {
    use super::*;

    #[test]
    fn compressed_object_in_later_object_stream_wins_over_lower_numbered_earlier_stream() {
        // Object stream 5 (earlier in the file) holds the old object 7; object stream 20,
        // appended by an update, holds the new one.
        let old = object_stream(&[(7, "(old)")]);
        let new = object_stream(&[(7, "(new)")]);
        let bytes = objects_only(
            &[(1, 0, CATALOG), (5, 0, &old), (20, 0, &new)],
            "trailer\n<< /Root 1 0 R >>\n",
        );
        let doc = open(&bytes).unwrap();
        assert_eq!(
            *doc.resolve(ObjRef::new(7, 0)).unwrap(),
            Object::String(PdfString::literal("new"))
        );
    }

    #[test]
    fn in_file_object_after_object_stream_wins_over_compressed_copy() {
        let stream = object_stream(&[(7, "(compressed)")]);
        let bytes = objects_only(
            &[(1, 0, CATALOG), (5, 0, &stream), (7, 0, b"(in file)")],
            "trailer\n<< /Root 1 0 R >>\n",
        );
        let doc = open(&bytes).unwrap();
        assert_eq!(
            *doc.resolve(ObjRef::new(7, 0)).unwrap(),
            Object::String(PdfString::literal("in file"))
        );
    }

    #[test]
    fn catalog_fallback_adopts_newest_catalog_in_file_order_not_highest_number() {
        // No trailer: the newest catalog (object 1, last in the file) has the one-page tree;
        // the older, higher-numbered catalog 9 points at a missing tree.
        let bytes = objects_only(
            &[
                (9, 0, b"<< /Type /Catalog /Pages 8 0 R >>"),
                (2, 0, ONE_PAGE),
                (3, 0, b"<< /Type /Page /Parent 2 0 R >>"),
                (1, 0, CATALOG),
            ],
            "",
        );
        assert_eq!(open(&bytes).unwrap().page_count(), Ok(1));
    }

    #[test]
    fn catalog_fallback_prefers_newer_in_file_catalog_over_older_compressed_one() {
        let stream = object_stream(&[(9, "<< /Type /Catalog /Pages 8 0 R >>")]);
        let bytes = objects_only(
            &[
                (5, 0, &stream),
                (2, 0, ONE_PAGE),
                (3, 0, b"<< /Type /Page /Parent 2 0 R >>"),
                (1, 0, CATALOG),
            ],
            "",
        );
        assert_eq!(open(&bytes).unwrap().page_count(), Ok(1));
    }
}

mod generations {
    use super::*;

    #[test]
    fn reference_with_stale_generation_resolves_to_null() {
        // Object 4 was freed and reused with generation 1; `4 0 R` is stale.
        let bytes = with_xref(
            &[
                (1, 0, CATALOG),
                (2, 0, b"<< /Type /Pages /Kids [] /Count 0 >>"),
                (4, 1, b"(reused)"),
            ],
            "<< /Size 5 /Root 1 0 R >>",
        );
        let doc = open(&bytes).unwrap();
        assert_eq!(*doc.resolve(ObjRef::new(4, 0)).unwrap(), Object::Null);
    }

    #[test]
    fn reference_with_matching_generation_resolves() {
        let bytes = with_xref(
            &[
                (1, 0, CATALOG),
                (2, 0, b"<< /Type /Pages /Kids [] /Count 0 >>"),
                (4, 1, b"(reused)"),
            ],
            "<< /Size 5 /Root 1 0 R >>",
        );
        let doc = open(&bytes).unwrap();
        assert_eq!(
            *doc.resolve(ObjRef::new(4, 1)).unwrap(),
            Object::String(PdfString::literal("reused"))
        );
    }

    #[test]
    fn xref_entry_whose_generation_differs_from_object_header_triggers_repair() {
        let mut bytes = with_xref(
            &[(1, 0, CATALOG), (2, 0, b"<< /Type /Pages /Kids [] /Count 0 >>")],
            "<< /Size 3 /Root 1 0 R >>",
        );
        // Same length: the xref row for object 2 now claims generation 3.
        let row = bytes.windows(7).rposition(|w| w == b"00000 n").unwrap();
        bytes[row..row + 5].copy_from_slice(b"00003");
        assert!(open(&bytes).unwrap().was_repaired());
    }

    #[test]
    fn object_stream_member_resolves_only_with_generation_zero() {
        let stream = object_stream(&[(7, "(packed)")]);
        let bytes = objects_only(&[(1, 0, CATALOG), (5, 0, &stream)], "trailer\n<< /Root 1 0 R >>\n");
        let doc = open(&bytes).unwrap();
        assert_eq!(*doc.resolve(ObjRef::new(7, 2)).unwrap(), Object::Null);
    }
}

mod page_tree_nulls {
    use super::*;

    fn tree(page: &[u8]) -> Vec<u8> {
        with_xref(
            &[
                (1, 0, CATALOG),
                (
                    2,
                    0,
                    b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] /Resources << /Font << >> >> >>",
                ),
                (3, 0, page),
            ],
            "<< /Size 4 /Root 1 0 R >>",
        )
    }

    #[test]
    fn explicit_null_media_box_keeps_inherited_value() {
        let bytes = tree(b"<< /Type /Page /Parent 2 0 R /MediaBox null >>");
        let doc = open(&bytes).unwrap();
        let inherited = Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]);
        assert_eq!(doc.pages().unwrap()[0].inherited.media_box.as_deref(), Some(&inherited));
    }

    #[test]
    fn resources_reference_to_missing_object_keeps_inherited_resources() {
        let bytes = tree(b"<< /Type /Page /Parent 2 0 R /Resources 12 0 R >>");
        let doc = open(&bytes).unwrap();
        assert!(doc.pages().unwrap()[0].resources_dict().is_some());
    }
}

mod page_contents {
    use super::*;

    fn page_with(contents: &str, extra: &[Obj<'_>]) -> Vec<u8> {
        let page = format!("<< /Type /Page /Parent 2 0 R /Contents {contents} >>");
        let mut objects: Vec<Obj<'_>> = vec![(1, 0, CATALOG), (2, 0, ONE_PAGE), (3, 0, page.as_bytes())];
        objects.extend_from_slice(extra);
        with_xref(&objects, "<< /Size 10 /Root 1 0 R >>")
    }

    #[test]
    fn contents_referencing_missing_object_is_empty() {
        let bytes = page_with("9 0 R", &[]);
        let doc = open(&bytes).unwrap();
        let page = doc.pages().unwrap().remove(0);
        assert_eq!(doc.page_content(&page), Ok(Vec::new()));
    }

    #[test]
    fn contents_array_skips_missing_elements() {
        let bytes = page_with(
            "[9 0 R 4 0 R 8 0 R]",
            &[(4, 0, b"<< /Length 2 >>\nstream\nET\nendstream")],
        );
        let doc = open(&bytes).unwrap();
        let page = doc.pages().unwrap().remove(0);
        assert_eq!(doc.page_content(&page), Ok(b"ET".to_vec()));
    }
}

mod filter_parameters {
    use super::*;

    /// A page whose content stream has `stream_dict_tail` after `/Length` and holds `data`.
    fn page_with_stream(stream_dict_tail: &str, data: &[u8], extra: &[Obj<'_>]) -> Vec<u8> {
        let mut stream = format!("<< /Length {} {stream_dict_tail} >>\nstream\n", data.len()).into_bytes();
        stream.extend_from_slice(data);
        stream.extend_from_slice(b"\nendstream");
        let mut objects: Vec<Obj<'_>> = vec![
            (1, 0, CATALOG),
            (2, 0, ONE_PAGE),
            (3, 0, b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>"),
            (4, 0, &stream),
        ];
        objects.extend_from_slice(extra);
        with_xref(&objects, "<< /Size 10 /Root 1 0 R >>")
    }

    fn content(bytes: &[u8]) -> Result<Vec<u8>, laminapdf_cos::Error> {
        let doc = open(bytes).unwrap();
        let page = doc.pages().unwrap().remove(0);
        doc.page_content(&page)
    }

    #[test]
    fn decode_parms_array_element_reference_is_resolved() {
        // One PNG row (filter type 2, Up) of 5 columns: decodes to "hello".
        let data = filters::flate_encode(b"\x02hello");
        let bytes = page_with_stream(
            "/Filter /FlateDecode /DecodeParms [6 0 R]",
            &data,
            &[(6, 0, b"<< /Predictor 12 /Columns 5 >>")],
        );
        assert_eq!(content(&bytes), Ok(b"hello".to_vec()));
    }

    #[test]
    fn filter_array_element_reference_is_resolved() {
        let data = filters::flate_encode(b"BT ET");
        let bytes = page_with_stream("/Filter [6 0 R]", &data, &[(6, 0, b"/FlateDecode")]);
        assert_eq!(content(&bytes), Ok(b"BT ET".to_vec()));
    }

    #[test]
    fn decode_parms_value_reference_inside_dictionary_is_resolved() {
        let data = filters::flate_encode(b"\x02hello");
        let bytes = page_with_stream(
            "/Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns 6 0 R >>",
            &data,
            &[(6, 0, b"5")],
        );
        assert_eq!(content(&bytes), Ok(b"hello".to_vec()));
    }

    #[test]
    fn decode_parms_reference_to_missing_object_means_no_parameters() {
        let data = filters::flate_encode(b"BT ET");
        let bytes = page_with_stream("/Filter /FlateDecode /DecodeParms [9 0 R]", &data, &[]);
        assert_eq!(content(&bytes), Ok(b"BT ET".to_vec()));
    }
}
