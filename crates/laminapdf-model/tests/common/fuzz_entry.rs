//! The fuzz targets' entry points (ADR 0018 tier 9), one function per target. The fuzz
//! targets in `fuzz/fuzz_targets/` call these (through `fuzz/src/lib.rs`, which includes this
//! file), and `tests/regressions.rs` replays every saved crash input
//! (`fixtures/regressions/<target>/`) through the same function, so a regression test runs
//! exactly what the fuzzer ran.

use laminapdf_model::xmp::{MAPPINGS, Packet};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_syntax::filters::{decode, flate_decode};
use laminapdf_syntax::{DecodeBudget, Dict, Lexer, Limits, Object, Parser, Token, parse_object_with};

/// A fuzz target's entry point.
pub type Entry = fn(&[u8]);

/// The targets, by name.
pub const TARGETS: [(&str, Entry); 6] = [
    ("lex", lex),
    ("parse_object", parse_object),
    ("flate_decode", flate),
    ("open", open),
    ("page_tree", page_tree),
    ("xmp_parse", xmp_parse),
];

/// Passwords the `open` target tries after the empty one (`fuzz/seeds/open/passwords.txt`,
/// one per line, `#` comments).
const PASSWORDS: &str = include_str!("../../../../fuzz/seeds/open/passwords.txt");

/// Tokenises `data` until the end or the first error; the lexer must always advance.
pub fn lex(data: &[u8]) {
    let limits = Limits::default();
    let mut lexer = Lexer::new(data, 0).with_limits(&limits);
    loop {
        let before = lexer.pos();
        match lexer.next_token() {
            Ok(Token::Eof) | Err(_) => break,
            Ok(_) => assert!(lexer.pos() > before, "lexer did not advance at {before}"),
        }
    }
}

/// Parses `data` as a direct object (whatever parses must serialise) and as an indirect
/// object with a failing `/Length` resolver, forcing the `endstream` scan.
pub fn parse_object(data: &[u8]) {
    let limits = Limits::default();
    if let Ok(obj) = parse_object_with(data, &limits) {
        let _ = obj.to_bytes();
    }
    let _ = Parser::new(data, 0).with_limits(&limits).indirect_object(&mut |_| None);
}

/// Inflates the input and runs it through the filter pipeline with PNG predictor parameters.
/// The last four bytes are the parameters (predictor, colours, bits per component, columns)
/// and the rest is the stream, so a seed is a raw Flate stream followed by four bytes.
pub fn flate(data: &[u8]) {
    let limits = Limits::default();
    let Some((stream, params)) = data.split_last_chunk::<4>() else {
        let _ = flate_decode(data, &limits, &mut DecodeBudget::new(&limits));
        return;
    };
    let _ = flate_decode(stream, &limits, &mut DecodeBudget::new(&limits));
    let [predictor, colors, bpc, columns] = *params;
    let params: Dict = [
        ("Predictor", Object::Integer(i64::from(predictor))),
        ("Colors", Object::Integer(i64::from(colors))),
        ("BitsPerComponent", Object::Integer(i64::from(bpc))),
        ("Columns", Object::Integer(i64::from(columns))),
    ]
    .into_iter()
    .collect();
    let dict: Dict = [("Filter", Object::name("FlateDecode")), ("DecodeParms", params.into())]
        .into_iter()
        .collect();
    let _ = decode(&dict, stream, &limits, &mut DecodeBudget::new(&limits));
}

/// The parameter bytes `flate` seeds end with: predictor 1 (none), one 8-bit colour, one
/// column.
pub const FLATE_SEED_PARAMS: [u8; 4] = [1, 1, 8, 1];

/// Opens `data` with the empty password, then each listed one, and drives every read path.
pub fn open(data: &[u8]) {
    let passwords = PASSWORDS
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(str::as_bytes);
    for password in std::iter::once(&b""[..]).chain(passwords) {
        let options = OpenOptions::default().with_password(password);
        if let Ok(doc) = Document::open(data.to_vec(), options) {
            super::exercise::exercise(&doc);
            if !doc.is_encrypted() {
                break;
            }
        }
    }
}

/// Walks a fuzzed page tree: a whole file (starting `%PDF`) as is, anything else as the body
/// of the `/Pages` object (2) of a small file whose other objects are a catalog, a page and
/// a second tree node with its own page, so inheritance through an intermediate node (its
/// `/CropBox`) is exercised too.
pub fn page_tree(data: &[u8]) {
    let bytes = if data.starts_with(b"%PDF") {
        data.to_vec()
    } else {
        wrap_page_tree(data)
    };
    let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
        return;
    };
    let Ok(pages) = doc.pages() else {
        return;
    };
    for page in pages.iter() {
        let _ = page.media_box();
        let _ = page.crop_box();
        let _ = page.rotate();
        let _ = page.label();
        let id = page.id().to_string();
        let parsed: Option<laminapdf_model::PageId> = id.parse().ok();
        assert_eq!(
            parsed.and_then(|id| pages.index_of(&id)),
            Some(page.index()),
            "page IDs round-trip"
        );
    }
}

/// The small file `page_tree` wraps a non-file input in.
pub fn wrap_page_tree(body: &[u8]) -> Vec<u8> {
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        body,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90 >>",
        b"<< /Type /Pages /Parent 2 0 R /Kids [5 0 R] /Count 1 /CropBox [0 0 10 10] >>",
        b"<< /Type /Page /Parent 4 0 R >>",
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
    out.extend_from_slice(b"xref\n0 6\n0000000000 65535 f\r\n");
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// Reads `data` as an XMP packet. One that parses must write back unchanged, read every mapped
/// field, and after an edit of the title, the authors, a date and the keywords write a packet
/// that parses again with those values.
pub fn xmp_parse(data: &[u8]) {
    let Ok(mut packet) = Packet::parse(data) else {
        return;
    };
    assert_eq!(packet.write().bytes, data, "an unchanged packet writes back as it was");
    for mapping in MAPPINGS {
        let _ = packet.get(mapping.field);
    }
    let _ = (packet.pdfa_part(), packet.pdfa_conformance(), packet.pdfua_part());
    let modified = "D:20261028143000+01'00".parse().ok();
    packet.set_title(Some("A <title> & more"));
    packet.set_authors(&["Ada", "Grace"]);
    packet.set_modify_date(modified);
    packet.set_keywords(None);
    let written = packet.write();
    let edited = Packet::parse(&written.bytes).unwrap_or_else(|e| panic!("the edited packet doesn't parse: {e}"));
    assert_eq!(
        (
            edited.title(),
            edited.authors(),
            edited.modify_date(),
            edited.keywords()
        ),
        (
            Some("A <title> & more".to_owned()),
            vec!["Ada".to_owned(), "Grace".to_owned()],
            modified,
            None
        ),
        "the edited packet reads back what was set"
    );
}
