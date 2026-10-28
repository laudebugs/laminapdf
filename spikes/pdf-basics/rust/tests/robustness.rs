//! SPEC case 5: hostile input yields errors (or repaired documents) — never a panic or hang.
//!
//! All parser loops are bounded by the input length, so "no hang" is structural; these
//! tests additionally assert a generous wall-clock budget per scenario.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use std::time::{Duration, Instant};

use common::{Rng, corpus, exercise};
use lamina_pdf_basics::{Error, OpenOptions, filters, open, open_with, parse_object};

const BUDGET: Duration = Duration::from_secs(30);

/// Opens `bytes` and drives every read path; returns whether open succeeded.
fn survive(bytes: &[u8]) -> bool {
    match open(bytes) {
        Ok(doc) => {
            exercise(&doc);
            true
        }
        Err(_) => false,
    }
}

fn within_budget(f: impl FnOnce()) {
    let start = Instant::now();
    f();
    assert!(start.elapsed() < BUDGET, "took {:?}", start.elapsed());
}

/// A classic file around arbitrary object bodies, with a correct xref.
fn file_with(objects: &[&[u8]], trailer: &str) -> Vec<u8> {
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

#[test]
fn truncation_at_many_offsets_never_panics() {
    within_budget(|| {
        for (_, bytes) in corpus() {
            let step = (bytes.len() / 400).max(1);
            for cut in (0..bytes.len()).step_by(step) {
                survive(&bytes[..cut]);
            }
        }
    });
}

#[test]
fn random_byte_flips_never_panic() {
    within_budget(|| {
        let mut rng = Rng::new(0x1A_317A);
        for (_, bytes) in corpus() {
            for _ in 0..150 {
                let mut mutated = bytes.clone();
                for _ in 0..=rng.below(8) {
                    let at = rng.below(mutated.len());
                    mutated[at] ^= u8::try_from(rng.below(255) + 1).unwrap();
                }
                survive(&mutated);
            }
        }
    });
}

#[test]
fn random_garbage_never_panics() {
    within_budget(|| {
        let mut rng = Rng::new(7);
        for _ in 0..300 {
            let len = rng.below(2048);
            let mut bytes = b"%PDF-1.4\n".to_vec();
            bytes.extend(
                (0..len).map(|_| b"0123456789 obj endobj R << >> [ ] ( ) / stream xref trailer\n"[rng.below(60)]),
            );
            survive(&bytes);
            let _ = parse_object(&bytes);
        }
    });
}

#[test]
fn empty_input_is_an_error() {
    assert_eq!(open(b"").err(), Some(Error::NotPdf("empty input")));
}

#[test]
fn header_only_is_an_error() {
    assert!(open(b"%PDF-").is_err());
}

#[test]
fn input_without_header_or_objects_is_an_error() {
    assert_eq!(
        open(b"just some text, no PDF here").err(),
        Some(Error::NotPdf("no objects found"))
    );
}

#[test]
fn self_referencing_kids_is_a_cycle_error() {
    let bytes = file_with(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [2 0 R] /Count 1 >>",
        ],
        "<< /Size 3 /Root 1 0 R >>",
    );
    assert_eq!(open(&bytes).unwrap().page_count(), Err(Error::ReferenceCycle(2)));
}

#[test]
fn page_reachable_twice_is_a_cycle_error() {
    let bytes = file_with(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R 3 0 R] /Count 2 >>",
            b"<< /Type /Page >>",
        ],
        "<< /Size 4 /Root 1 0 R >>",
    );
    assert_eq!(open(&bytes).unwrap().page_count(), Err(Error::ReferenceCycle(3)));
}

#[test]
fn reference_cycle_is_an_error() {
    let bytes = file_with(&[b"2 0 R", b"1 0 R"], "<< /Size 3 /Root 1 0 R >>");
    assert!(matches!(open(&bytes).unwrap().catalog(), Err(Error::ReferenceCycle(_))));
}

#[test]
fn reference_cycle_without_xref_is_an_error_after_repair() {
    let bytes = b"%PDF-1.7\n1 0 obj 2 0 R endobj 2 0 obj 1 0 R endobj\ntrailer << /Root 1 0 R >>\n";
    assert!(matches!(open(bytes).unwrap().catalog(), Err(Error::ReferenceCycle(_))));
}

#[test]
fn self_referencing_stream_length_does_not_hang() {
    let bytes = file_with(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Length 2 0 R >>\nstream\nabc\nendstream",
        ],
        "<< /Size 3 /Root 1 0 R >>",
    );
    let doc = open(&bytes).unwrap();
    assert!(doc.resolve(lamina_pdf_basics::ObjRef::new(2, 0)).is_ok());
}

#[test]
fn prev_chain_loop_falls_back_to_repair() {
    let mut bytes = file_with(
        &[b"<< /Type /Catalog /Pages 2 0 R >>"],
        "<< /Size 2 /Root 1 0 R /Prev 9 >>",
    );
    let xref = bytes.windows(4).position(|w| w == b"xref").unwrap();
    let text = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("/Prev 9", &format!("/Prev {xref}"));
    bytes = text.into_bytes();
    assert!(open(&bytes).unwrap().was_repaired());
}

#[test]
fn nesting_depth_ten_thousand_is_rejected_by_parser() {
    let deep = [vec![b'['; 10_000], vec![b']'; 10_000]].concat();
    assert!(matches!(parse_object(&deep), Err(Error::TooDeep { .. })));
}

#[test]
fn nesting_depth_ten_thousand_in_a_file_is_an_error_on_resolve() {
    let deep = [vec![b'['; 10_000], vec![b']'; 10_000]].concat();
    let bytes = file_with(
        &[b"<< /Type /Catalog /Pages 2 0 R >>", &deep],
        "<< /Size 3 /Root 1 0 R >>",
    );
    let doc = open(&bytes).unwrap();
    assert!(matches!(
        doc.resolve(lamina_pdf_basics::ObjRef::new(2, 0)),
        Err(Error::TooDeep { .. })
    ));
}

#[test]
fn deeply_nested_dicts_are_rejected() {
    let deep = [b"<< /A ".repeat(10_000), b">>".repeat(10_000)].concat();
    assert!(matches!(parse_object(&deep), Err(Error::TooDeep { .. })));
}

fn bomb_file() -> Vec<u8> {
    let payload = filters::flate_encode(&vec![0u8; filters::DEFAULT_MAX_DECODED + 1]);
    let mut stream = format!("<< /Length {} /Filter /FlateDecode >>\nstream\n", payload.len()).into_bytes();
    stream.extend_from_slice(&payload);
    stream.extend_from_slice(b"\nendstream");
    file_with(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>",
            &stream,
        ],
        "<< /Size 5 /Root 1 0 R >>",
    )
}

#[test]
fn flate_bomb_over_default_limit_is_refused() {
    let bytes = bomb_file();
    let doc = open(&bytes).unwrap();
    let page = doc.pages().unwrap().remove(0);
    assert_eq!(
        doc.page_content(&page),
        Err(Error::DecompressLimit {
            limit: filters::DEFAULT_MAX_DECODED
        })
    );
}

#[test]
fn decompression_limit_is_configurable() {
    let content = filters::flate_encode(&[b' '; 5000]);
    let mut stream = format!("<< /Length {} /Filter /FlateDecode >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(&content);
    stream.extend_from_slice(b"\nendstream");
    let bytes = file_with(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>",
            &stream,
        ],
        "<< /Size 5 /Root 1 0 R >>",
    );
    let doc = open_with(&bytes, OpenOptions { max_decoded_size: 1000 }).unwrap();
    let page = doc.pages().unwrap().remove(0);
    assert_eq!(doc.page_content(&page), Err(Error::DecompressLimit { limit: 1000 }));
}

#[test]
fn xref_stream_with_absurd_widths_is_repaired_or_rejected() {
    let bytes = b"%PDF-1.5\n1 0 obj << /Type /XRef /W [9 9 9] /Size 1 /Root 1 0 R /Length 0 >> stream\n\nendstream endobj\nstartxref\n9\n%%EOF";
    survive(bytes);
}
