//! The fuzz targets' entry points (ADR 0018 tier 9), one function per target. The fuzz
//! targets in `fuzz/fuzz_targets/` call these (through `fuzz/src/lib.rs`, which includes this
//! file), and `tests/regressions.rs` replays every saved crash input
//! (`fixtures/regressions/<target>/`) through the same function, so a regression test runs
//! exactly what the fuzzer ran.

use laminapdf_model::xmp::{MAPPINGS, Packet, RdfValue, XmpValue};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_syntax::filters::{decode, flate_decode};
use laminapdf_syntax::{DecodeBudget, Dict, Lexer, Limits, Object, Parser, Token, parse_object_with};

/// A fuzz target's entry point.
pub type Entry = fn(&[u8]);

/// The targets, by name.
pub const TARGETS: [(&str, Entry); 9] = [
    ("lex", lex),
    ("parse_object", parse_object),
    ("flate_decode", flate),
    ("open", open),
    ("page_tree", page_tree),
    ("form_tree", form_tree),
    ("xmp_parse", xmp_parse),
    ("af_script", af_script),
    ("xfa_packets", xfa_packets),
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

/// Reads a fuzzed form: a whole file (starting `%PDF`) as is, anything else as the body of the
/// interactive form dictionary (object 2) of a small file whose page lists three widgets, a
/// field with kids among them, so the walk, inheritance and the reconciliation with the page
/// are exercised. The form must read with each field's name unique and its ID parsing back, and
/// the tab order must list each of the page's widgets once.
pub fn form_tree(data: &[u8]) {
    let bytes = if data.starts_with(b"%PDF") {
        data.to_vec()
    } else {
        wrap_form(data)
    };
    let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
        return;
    };
    let Ok(form) = doc.form() else { return };
    let mut names = std::collections::BTreeSet::new();
    for field in &form.fields {
        assert!(
            names.insert(field.name.clone()),
            "fields that share a name are one field"
        );
        let parsed: Option<laminapdf_model::FieldId> = field.id.to_string().parse().ok();
        assert_eq!(parsed.as_ref(), Some(&field.id), "field IDs round-trip");
        let _ = field.on_states();
        // A field's actions read the same way twice, within the budget, and every selection
        // resolves against the form.
        let Ok(actions) = doc.form_actions(field) else {
            continue;
        };
        assert!(actions.len() <= 64, "actions read are bounded");
        assert_eq!(
            doc.form_actions(field).ok().as_ref(),
            Some(&actions),
            "actions read the same way twice"
        );
        for entry in &actions {
            match &entry.action {
                laminapdf_model::FormAction::ResetForm(selection) => {
                    let _ = selection.fields(&form);
                }
                laminapdf_model::FormAction::SubmitForm(submit) => {
                    let _ = submit.selection.fields(&form);
                }
                _ => {}
            }
        }
    }
    let Ok(pages) = doc.pages() else { return };
    for page in pages.iter() {
        let (Ok(order), Ok(widgets)) = (doc.tab_order(page.id()), doc.page_widgets(page.id())) else {
            continue;
        };
        let mut listed: Vec<_> = order.widgets.iter().map(|w| w.widget).collect();
        let mut all: Vec<_> = widgets.iter().map(|w| w.widget).collect();
        listed.sort_unstable();
        all.sort_unstable();
        assert_eq!(listed, all, "the tab order lists each widget once");
    }
}

/// The small file `form_tree` wraps a non-file input in: `body` is the form dictionary.
pub fn wrap_form(body: &[u8]) -> Vec<u8> {
    let objects: [&[u8]; 6] = [
        b"<< /Type /Catalog /Pages 3 0 R /AcroForm 2 0 R >>",
        body,
        b"<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] /Tabs /R /Annots [6 0 R 7 0 R 5 0 R] >>",
        b"<< /FT /Tx /T (a) /V (x) /Subtype /Widget /Rect [10 700 100 720] /P 4 0 R >>",
        b"<< /FT /Btn /Ff 32768 /T (b) /V /On /Kids [7 0 R] /Subtype /Widget /Rect [10 600 30 620] >>",
    ];
    let extra: &[u8] =
        b"<< /Parent 6 0 R /Subtype /Widget /Rect [50 600 70 620] /AS /On /AP << /N << /On 8 0 R >> >> >>";
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().chain([&extra]).enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", offsets.len() + 1).as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// Reads a fuzzed XFA part: a whole file (starting `%PDF`) as is, anything else as the one stream
/// of an `/XFA` entry in a small form, and again as each packet of an `/XFA` array (`config`,
/// `template`, `datasets`). The packets must read, the kind must be dynamic exactly when a signal
/// says so, and a second read must give the same answer.
pub fn xfa_packets(data: &[u8]) {
    let files = if data.starts_with(b"%PDF") {
        vec![data.to_vec()]
    } else {
        vec![
            wrap_xfa("/XFA 10 0 R", data),
            wrap_xfa("/XFA [(config) 10 0 R (template) 10 0 R (datasets) 10 0 R]", data),
        ]
    };
    for bytes in files {
        let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
            continue;
        };
        let Ok(read) = doc.xfa() else { continue };
        if let Some(xfa) = &read {
            assert_eq!(
                xfa.kind == laminapdf_model::XfaKind::Dynamic,
                !xfa.signals.is_empty(),
                "a form is dynamic exactly when a signal says so"
            );
            for name in ["config", "template", "datasets"] {
                let _ = xfa.packet(name);
            }
        }
        assert_eq!(doc.xfa().ok(), Some(read), "a second read gives the same answer");
    }
}

/// The small file `xfa_packets` wraps a packet in: `xfa` is the form's `/XFA` entry, and
/// object 10 is a stream holding `packet`. The form lists no field.
pub fn wrap_xfa(xfa: &str, packet: &[u8]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let objects: [(u32, Vec<u8>); 5] = [
        (1, b"<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>".to_vec()),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".to_vec()),
        (5, format!("<< /Fields [] {xfa} >>").into_bytes()),
        (
            10,
            [
                format!("<< /Length {} >>\nstream\n", packet.len()).as_bytes(),
                packet,
                b"\nendstream",
            ]
            .concat(),
        ),
    ];
    for (num, body) in &objects {
        offsets.push((*num, out.len()));
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 11\n0000000000 65535 f\r\n");
    for num in 1..11 {
        match offsets.iter().find(|(n, _)| *n == num) {
            Some((_, at)) => out.extend_from_slice(format!("{at:010} 00000 n\r\n").as_bytes()),
            None => out.extend_from_slice(b"0000000000 65535 f\r\n"),
        }
    }
    out.extend_from_slice(format!("trailer\n<< /Size 11 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// Reads `data` as an XMP packet. One that parses must write back unchanged, read every mapped
/// field, and after an edit of the title, the authors, a date and the keywords, a custom
/// property and an array, with every property declared when it claims PDF/A, write a packet that
/// parses again with those values.
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
    let custom = "http://example.com/fuzz/";
    let tags = XmpValue::Bag(vec![XmpValue::Text("a & b".into()), XmpValue::Text("<c>".into())]);
    packet
        .set_property(custom, "fz", "count", Some(XmpValue::Integer(3)))
        .unwrap_or_else(|e| panic!("a valid custom property was refused: {e}"));
    packet
        .set_property(custom, "fz", "tags", Some(tags))
        .unwrap_or_else(|e| panic!("a valid array was refused: {e}"));
    packet.declare_extensions();
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
    assert_eq!(
        edited.property(custom, "count"),
        Some(&RdfValue::Text("3".to_owned())),
        "the custom property reads back"
    );
}

/// Reads `data` as a field's keystroke, format, validate and calculate script (ADR 0021 §2): up
/// to its first NUL byte the script, after it the text typed. A script must read the same way
/// twice, and its behaviour show, check and commit the text and calculate without panicking;
/// the inventory lists the field's scripts.
pub fn af_script(data: &[u8]) {
    let mut parts = data.splitn(2, |b| *b == 0);
    let script = parts.next().unwrap_or_default();
    let typed = String::from_utf8_lossy(parts.next().unwrap_or(b"1234.5"));
    let Ok(doc) = Document::open(wrap_script(script), OpenOptions::default()) else {
        return;
    };
    let Ok(form) = doc.form() else { return };
    for field in &form.fields {
        let Ok(behaviour) = doc.field_behaviour(field) else {
            continue;
        };
        assert_eq!(
            doc.field_behaviour(field).ok().as_ref(),
            Some(&behaviour),
            "a script reads the same way twice"
        );
        let _ = behaviour.show(&typed);
        let _ = behaviour.commit(&typed);
        for b in [&behaviour.keystroke, &behaviour.format, &behaviour.validate]
            .into_iter()
            .flatten()
        {
            let _ = b.accepts(&typed);
            let _ = b.show(&typed);
        }
        if let Some(c) = &behaviour.calculate {
            let _ = c.compute(&laminapdf_model::FormValues::of(&form));
        }
    }
    let _ = doc.form_scripts();
}

/// A one-page form whose text field `a` has `script` as its `K`, `F`, `V` and `C` action, in
/// `/CO`, beside a field `b` it may read.
pub fn wrap_script(script: &[u8]) -> Vec<u8> {
    let objects: [&[u8]; 6] = [
        b"<< /Type /Catalog /Pages 3 0 R /AcroForm 2 0 R >>",
        b"<< /Fields [5 0 R 6 0 R] /CO [5 0 R] /DA (/Helv 0 Tf 0 g) >>",
        b"<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] /Annots [5 0 R 6 0 R] >>",
        b"<< /FT /Tx /T (a) /V (1) /AA << /K 7 0 R /F 7 0 R /V 7 0 R /C 7 0 R >> /Subtype /Widget /Rect [10 700 100 720] /P 4 0 R >>",
        b"<< /FT /Tx /T (b) /V (2.5) /Subtype /Widget /Rect [10 600 100 620] /P 4 0 R >>",
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    offsets.push(out.len());
    out.extend_from_slice(b"7 0 obj\n<< /S /JavaScript /JS 8 0 R >>\nendobj\n");
    offsets.push(out.len());
    out.extend_from_slice(format!("8 0 obj\n<< /Length {} >>\nstream\n", script.len()).as_bytes());
    out.extend_from_slice(script);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", offsets.len() + 1).as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}
