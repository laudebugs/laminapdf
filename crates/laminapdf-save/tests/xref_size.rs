//! The trailer's `/Size` never shrinks: it covers every object number in the file, including
//! objects the reader dropped under a limit. (The reader's side of these fallbacks is in
//! `laminapdf-cos/tests/reader_fallbacks.rs`.)
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::{open, open_with};
use laminapdf_cos::{Limits, ObjRef, OpenOptions};
use laminapdf_save::XrefEol;
use laminapdf_save::test_support::incremental_update;

fn with(limits: Limits) -> OpenOptions {
    OpenOptions::default().with_limits(limits)
}

mod incremental_size {
    use super::*;

    /// Objects 1–3 plus object 9 (unreferenced) in a classic table with two subsections and
    /// an understated `/Size 4`.
    fn high_numbered_object() -> Vec<u8> {
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (num, body) in [
            (1, "<< /Type /Catalog /Pages 2 0 R >>"),
            (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            (3, "<< /Type /Page /Parent 2 0 R >>"),
            (9, "42"),
        ] {
            offsets.push(out.len());
            out.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 4\n0000000000 65535 f\r\n");
        for off in &offsets[..3] {
            out.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
        }
        out.extend_from_slice(format!("9 1\n{:010} 00000 n\r\n", offsets[3]).as_bytes());
        out.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        out
    }

    #[test]
    fn size_covers_objects_dropped_by_max_object_number() {
        let base = open_with(
            &high_numbered_object(),
            with(Limits::default().with_max_object_number(8)),
        )
        .unwrap();
        let updated = incremental_update(
            &base,
            &[(ObjRef::new(3, 0), laminapdf_cos::Object::Null)],
            XrefEol::CrLf,
        )
        .unwrap();
        assert_eq!(open(&updated).unwrap().trailer().get_int("Size"), Some(10));
    }
}
