//! The `content` fuzz target's entry point (ADR 0018 tier 9). `fuzz/fuzz_targets/content.rs`
//! calls it (through `fuzz/src/lib.rs`, which includes this file), and `tests/regressions.rs`
//! replays every saved crash input (`fixtures/regressions/content/`) through it.

use laminapdf_content::{FormXObject, MarkedContent, Visitor, interpret_page};
use laminapdf_cos::{Dict, Object, Stream};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_save::test_support::Writer;

/// Interprets `data`: a whole file (starting `%PDF`) page by page, anything else as the
/// content of the page of [`fixture_file`], whose resources cover every resource category.
/// Invariants: no panic, no hang, and balanced visitor events, also when the interpretation
/// fails.
pub fn content(data: &[u8]) {
    let bytes = if data.starts_with(b"%PDF") {
        data.to_vec()
    } else {
        fixture_file(data)
    };
    let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
        return;
    };
    let Ok(pages) = doc.pages() else {
        return;
    };
    for page in pages.iter() {
        // Balanced whether the interpretation succeeded or stopped with an error.
        let mut check = Balance::default();
        let _ = interpret_page(&doc, &page, &mut check);
        assert_eq!(check.marked, 0, "every begin_marked is ended");
        assert_eq!(check.forms, 0, "every enter_xobject is left");
    }
}

/// Counts open marked-content sequences and forms.
#[derive(Debug, Default)]
struct Balance {
    marked: i64,
    forms: i64,
}

impl Visitor for Balance {
    fn begin_marked(&mut self, _: &MarkedContent<'_>) {
        self.marked += 1;
    }

    fn end_marked(&mut self) {
        self.marked -= 1;
        assert!(self.marked >= 0, "end_marked without begin_marked");
    }

    fn enter_xobject(&mut self, form: &FormXObject<'_>) {
        self.forms += 1;
        assert_eq!(usize::try_from(self.forms).ok(), Some(form.depth), "form depth");
    }

    fn leave_xobject(&mut self, _: &FormXObject<'_>) {
        self.forms -= 1;
    }
}

fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

/// A one-page file whose page content is `content` and whose resources hold a font `F1`, an
/// image `Im1`, forms `Fm1` (no resources of its own: it uses the page's) and `Fm2` (draws
/// itself), a graphics state `GS1` with a font, a shading `Sh1`, a pattern `P1`, a colour
/// space `CS1` and a property list `MC1`.
#[expect(clippy::too_many_lines, reason = "one object per resource category, in one place")]
pub fn fixture_file(content: &[u8]) -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let font = w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
    ]));
    let image = w.add(Stream::new(
        dict([
            ("Type", Object::name("XObject")),
            ("Subtype", Object::name("Image")),
            ("Width", 1.into()),
            ("Height", 1.into()),
            ("ColorSpace", Object::name("DeviceGray")),
            ("BitsPerComponent", 8.into()),
        ]),
        vec![0x80],
    ));
    let bbox: Object = vec![0.into(), 0.into(), 10.into(), 10.into()].into();
    let fm1 = w.add(Stream::new(
        dict([
            ("Subtype", Object::name("Form")),
            ("BBox", bbox.clone()),
            (
                "Matrix",
                vec![2.into(), 0.into(), 0.into(), 2.into(), 5.into(), 5.into()].into(),
            ),
        ]),
        b"q /Im1 Do Q BT /F1 9 Tf (x) Tj ET /P <</MCID 1>> BDC EMC".to_vec(),
    ));
    let fm2 = w.reserve();
    w.set(
        fm2,
        Stream::new(
            dict([
                ("Subtype", Object::name("Form")),
                ("BBox", bbox),
                (
                    "Resources",
                    dict([("XObject", dict([("Fm2", fm2.into())]).into())]).into(),
                ),
            ]),
            b"0 0 1 1 re f /Fm2 Do".to_vec(),
        ),
    );
    let shading = dict([
        ("ShadingType", 2.into()),
        ("ColorSpace", Object::name("DeviceRGB")),
        ("Coords", vec![0.into(), 0.into(), 1.into(), 0.into()].into()),
    ]);
    let resources = dict([
        ("Font", dict([("F1", font.into())]).into()),
        (
            "XObject",
            dict([("Im1", image.into()), ("Fm1", fm1.into()), ("Fm2", fm2.into())]).into(),
        ),
        (
            "ExtGState",
            dict([(
                "GS1",
                dict([("CA", Object::Real(0.5)), ("Font", vec![font.into(), 12.into()].into())]).into(),
            )])
            .into(),
        ),
        ("Shading", dict([("Sh1", shading.clone().into())]).into()),
        (
            "Pattern",
            dict([(
                "P1",
                dict([("PatternType", 2.into()), ("Shading", shading.into())]).into(),
            )])
            .into(),
        ),
        (
            "ColorSpace",
            dict([(
                "CS1",
                vec![Object::name("Indexed"), Object::name("DeviceRGB"), 0.into()].into(),
            )])
            .into(),
        ),
        ("Properties", dict([("MC1", dict([("MCID", 7.into())]).into())]).into()),
    ]);
    let content = w.add(Stream::new(Dict::new(), content.to_vec()));
    let page = w.add(dict([
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ("Contents", content.into()),
        ("Resources", resources.into()),
    ]));
    w.set(
        pages,
        dict([
            ("Type", Object::name("Pages")),
            ("Kids", vec![page.into()].into()),
            ("Count", 1.into()),
        ]),
    );
    let root = w.add(dict([("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    w.finish(&[("Root", root)].into_iter().collect()).unwrap_or_default()
}
