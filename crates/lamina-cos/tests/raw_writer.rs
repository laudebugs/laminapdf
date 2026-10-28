//! SPEC case 2: parity with `examples/05-generation-approaches/raw_writer.py`.
//! Everything is compared except the Flate payload (compared decoded) and its `/Length`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::content::ContentBuilder;
use common::fixture;
use lamina_cos::{Dict, Document, ObjRef, Object, PdfString, Stream, Writer, XrefEol, open};

const TITLE: &str = "Hello from lamina";
const PARA: [&str; 2] = [
    "This one-page document was produced by hand-emitting PDF objects:",
    "a catalog, a page tree, one page, a font and a content stream.",
];

/// The decoded content exactly as the Python script builds it.
fn python_content() -> Vec<u8> {
    let mut ops = vec![
        "0.2 0.4 0.8 RG 2 w 72 600 468 120 re S".to_owned(),
        format!("BT /F2 24 Tf 90 690 Td ({TITLE}) Tj ET"),
        "BT /F1 12 Tf 14 TL 90 660 Td".to_owned(),
    ];
    ops.extend(PARA.iter().map(|l| format!("({l}) '")));
    ops.push("ET".to_owned());
    ops.join("\n").into_bytes()
}

fn content() -> Vec<u8> {
    let mut b = ContentBuilder::new();
    b.line(|l| {
        l.stroke_rgb(0.2, 0.4, 0.8)
            .line_width(2)
            .rect(72, 600, 468, 120)
            .stroke();
    });
    b.line(|l| {
        l.begin_text()
            .font("F2", 24)
            .text_move(90, 690)
            .show_text(TITLE)
            .end_text();
    });
    b.line(|l| {
        l.begin_text().font("F1", 12).leading(14).text_move(90, 660);
    });
    for line in PARA {
        b.next_line_show(line);
    }
    b.end_text();
    b.into_bytes()
}

fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

fn r(n: u32) -> Object {
    ObjRef::new(n, 0).into()
}

fn build() -> Vec<u8> {
    let mut w = Writer::new(1, 7).with_xref_eol(XrefEol::SpaceLf);
    w.add(dict([("Type", Object::name("Catalog")), ("Pages", r(2))]));
    w.add(dict([
        ("Type", Object::name("Pages")),
        ("Kids", vec![r(3)].into()),
        ("Count", 1.into()),
    ]));
    w.add(dict([
        ("Type", Object::name("Page")),
        ("Parent", r(2)),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        (
            "Resources",
            dict([("Font", dict([("F1", r(4)), ("F2", r(5))]).into())]).into(),
        ),
        ("Contents", r(6)),
    ]));
    w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
    ]));
    w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica-Bold")),
    ]));
    // `/Length` placed first so the writer keeps its position, as raw_writer.py emits it.
    w.add(Stream::flate(dict([("Length", 0.into())]), &content()));
    w.add(dict([
        ("Title", PdfString::literal(TITLE).into()),
        ("Producer", PdfString::literal("raw_writer.py").into()),
    ]));
    w.finish(&dict([("Root", r(1)), ("Info", r(7))])).unwrap()
}

fn content_stream(doc: &Document<'_>) -> Stream {
    match std::rc::Rc::unwrap_or_clone(doc.resolve(ObjRef::new(6, 0)).unwrap()) {
        Object::Stream(s) => s,
        other => panic!("object 6 is not a stream: {other:?}"),
    }
}

fn without_length(mut d: Dict) -> Dict {
    d.remove("Length");
    d
}

#[test]
fn builder_content_equals_python_content() {
    assert_eq!(
        String::from_utf8_lossy(&content()),
        String::from_utf8_lossy(&python_content())
    );
}

#[test]
fn bytes_before_the_content_stream_are_identical() {
    let (ours, theirs) = (build(), fixture("corpus/raw.pdf"));
    let cut = |b: &[u8]| b.windows(7).position(|w| w == b"6 0 obj").unwrap();
    assert_eq!(&ours[..cut(&ours)], &theirs[..cut(&theirs)]);
}

#[test]
fn content_stream_dicts_match_apart_from_length() {
    let (ours, theirs) = (build(), fixture("corpus/raw.pdf"));
    let (a, b) = (open(&ours).unwrap(), open(&theirs).unwrap());
    assert_eq!(
        without_length(content_stream(&a).dict),
        without_length(content_stream(&b).dict)
    );
}

#[test]
fn content_stream_length_is_payload_length() {
    let ours = build();
    let s = content_stream(&open(&ours).unwrap());
    assert_eq!(s.dict.get_int("Length"), Some(i64::try_from(s.data.len()).unwrap()));
}

#[test]
fn decoded_content_streams_are_identical() {
    let (ours, theirs) = (build(), fixture("corpus/raw.pdf"));
    let (a, b) = (open(&ours).unwrap(), open(&theirs).unwrap());
    assert_eq!(
        a.decode_stream(&content_stream(&a)).unwrap(),
        b.decode_stream(&content_stream(&b)).unwrap()
    );
}

#[test]
fn info_and_trailer_serialize_identically() {
    let (ours, theirs) = (build(), fixture("corpus/raw.pdf"));
    let (a, b) = (open(&ours).unwrap(), open(&theirs).unwrap());
    let info = |d: &Document<'_>| d.resolve(ObjRef::new(7, 0)).unwrap().to_bytes();
    let trailer = |d: &Document<'_>| Object::Dict(d.trailer().clone()).to_bytes();
    assert_eq!((info(&a), trailer(&a)), (info(&b), trailer(&b)));
}

#[test]
fn tail_after_content_stream_is_identical_up_to_shifted_offsets() {
    // From `endstream` on, only obj 7's xref offset and startxref differ (by the payload delta).
    let (ours, theirs) = (build(), fixture("corpus/raw.pdf"));
    let tail = |b: &[u8]| {
        let at = b.windows(9).position(|w| w == b"endstream").unwrap();
        let text = String::from_utf8_lossy(&b[at..]).into_owned();
        let lines: Vec<String> = text
            .lines()
            .map(|l| {
                if l.trim_end().ends_with(" 00000 n") {
                    "<entry>".to_owned()
                } else {
                    l.to_owned()
                }
            })
            .collect();
        lines
            .into_iter()
            .filter(|l| l.parse::<u64>().is_err())
            .collect::<Vec<_>>()
    };
    assert_eq!(tail(&ours), tail(&theirs));
}

#[test]
fn xref_entries_use_space_lf_like_the_script() {
    assert!(build().windows(20).any(|w| w == b"0000000000 65535 f \n"));
}

#[test]
fn rebuilt_file_needs_no_repair() {
    assert!(!open(&build()).unwrap().was_repaired());
}
