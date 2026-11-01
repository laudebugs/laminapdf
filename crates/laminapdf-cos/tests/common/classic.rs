//! Hand-built files for tests, with no dependencies: shared by the integration tests of
//! `laminapdf-cos` and `laminapdf-save` (`#[path]`) and by `laminapdf-cos`'s unit tests, which can't use
//! the writer (through the `laminapdf-save` dev-dependency they would see a second `laminapdf-cos`).
#![allow(dead_code, unreachable_pub, reason = "each user takes a different subset")]

/// A classic file around object bodies numbered `1..`, with a correct xref table.
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

/// A stream object body: `dict` (without `/Length`) plus `data`, with a correct `/Length`.
pub fn stream_body(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}
