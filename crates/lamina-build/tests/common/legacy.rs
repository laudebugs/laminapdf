//! `fixtures/golden/hello.pdf` and `imaging-demo.pdf`, rebuilt byte for byte with
//! `lamina-save`'s spike-era [`Writer`] and the [`ContentBuilder`] (`content.rs`).
//!
//! These two goldens are the frozen outputs of the Python examples in research chapters 01 and
//! 02 (via the `pdf-basics` spike): a hand-wrapped page dictionary, `%` comments and fixed
//! decimals in the content, a classic xref without `/ID`. The builder (`Builder`) writes its
//! own layout and `pdf_writer::Content`, so it can't reproduce them byte for byte; they are
//! generated here, in `lamina-build`'s golden example, and `lamina-save`'s tests include this
//! file (by `#[path]`) for the documents they edit.

use lamina_cos::{Dict, ObjRef, Object, Result, Stream};
use lamina_save::{Writer, XrefEol};

use super::content::{ContentBuilder, fixed};

fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

fn name(s: &str) -> Object {
    Object::name(s)
}

fn length_only(data: Vec<u8>) -> Stream {
    Stream::new(Dict::new(), data)
}

/// `fixtures/golden/hello.pdf` (from `examples/01-file-structure/hello.py`): one page,
/// "Hello, World" in Helvetica, `CrLf` xref entries.
///
/// The page dictionary is hand-wrapped over three lines in the original, which the
/// single-space serializer cannot express, so it is written as a raw body.
///
/// # Errors
/// Never in practice; the signature mirrors [`Writer::finish`].
pub fn hello() -> Result<Vec<u8>> {
    let mut content = ContentBuilder::new();
    content
        .begin_text()
        .font("F1", 24)
        .text_move(72, 720)
        .show_text("Hello, World")
        .end_text();

    let mut w = Writer::new(1, 7).with_xref_eol(XrefEol::CrLf);
    let catalog = w.add(dict([("Type", name("Catalog")), ("Pages", ObjRef::new(2, 0).into())]));
    w.add(dict([
        ("Type", name("Pages")),
        ("Kids", vec![ObjRef::new(3, 0).into()].into()),
        ("Count", 1.into()),
    ]));
    w.add_raw(
        &b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n   /Resources << /Font << /F1 4 0 R >> >>\n   /Contents 5 0 R >>"[..],
    );
    w.add(dict([
        ("Type", name("Font")),
        ("Subtype", name("Type1")),
        ("BaseFont", name("Helvetica")),
    ]));
    w.add(length_only(content.into_bytes()));
    w.finish(&dict([("Root", catalog.into())]))
}

/// The content stream of `fixtures/golden/imaging-demo.pdf`.
#[must_use]
pub fn imaging_demo_content() -> Vec<u8> {
    let angle = 30f64.to_radians();
    let (c, s) = (angle.cos(), angle.sin());
    let mut b = ContentBuilder::new();
    b.comment("1. filled rectangle (DeviceRGB)")
        .save_state()
        .fill_rgb(fixed(0.2, 2), fixed(0.4, 2), fixed(0.8, 2))
        .rect(72, 600, 200, 120)
        .fill()
        .restore_state();
    b.comment("2. stroked Bezier circle, round joins, dashed")
        .save_state()
        .stroke_rgb(fixed(0.85, 2), fixed(0.1, 2), fixed(0.1, 2))
        .line_width(6)
        .line_join(1)
        .dash(&[18.0, 6.0], 0)
        .circle(430.0, 660.0, 70.0)
        .stroke()
        .restore_state();
    b.comment("3. axial gradient clipped to a rectangle")
        .save_state()
        .rect(72, 380, 468, 150);
    b.line(|l| {
        l.clip().end_path();
    });
    b.shade("Sh1").restore_state();
    b.comment("4. rotated square, 30 deg about its centre (306,220), 50% opacity")
        .save_state()
        .ext_gstate("GS1")
        .transform(1, 0, 0, 1, 306, 220)
        .transform(fixed(c, 6), fixed(s, 6), fixed(-s, 6), fixed(c, 6), 0, 0)
        .fill_rgb(0, 0.6, 0.3)
        .stroke_rgb(0, 0, 0)
        .line_width(2)
        .rect(-60, -60, 120, 120)
        .fill_stroke()
        .restore_state()
        .end_line();
    b.into_bytes()
}

/// `fixtures/golden/imaging-demo.pdf` (from `examples/02-imaging-model/make_imaging_demo.py`):
/// fill, dashed stroke, clipped axial shading and a transformed semi-transparent square,
/// `SpaceLf` xref entries.
///
/// # Errors
/// Never in practice; the signature mirrors [`Writer::finish`].
pub fn imaging_demo() -> Result<Vec<u8>> {
    let r = |n| Object::Ref(ObjRef::new(n, 0));
    let mut w = Writer::new(1, 7).with_xref_eol(XrefEol::SpaceLf);
    let catalog = w.add(dict([("Type", name("Catalog")), ("Pages", r(2))]));
    w.add(dict([
        ("Type", name("Pages")),
        ("Kids", vec![r(3)].into()),
        ("Count", 1.into()),
    ]));
    w.add(dict([
        ("Type", name("Page")),
        ("Parent", r(2)),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ("Contents", r(4)),
        (
            "Resources",
            dict([
                ("Shading", dict([("Sh1", r(5))]).into()),
                ("ExtGState", dict([("GS1", r(7))]).into()),
            ])
            .into(),
        ),
    ]));
    w.add(length_only(imaging_demo_content()));
    w.add(dict([
        ("ShadingType", 2.into()),
        ("ColorSpace", name("DeviceRGB")),
        ("Coords", vec![72.into(), 0.into(), 540.into(), 0.into()].into()),
        ("Function", r(6)),
        ("Extend", vec![true.into(), true.into()].into()),
    ]));
    w.add(dict([
        ("FunctionType", 2.into()),
        ("Domain", vec![0.into(), 1.into()].into()),
        ("C0", vec![1.into(), 0.85.into(), 0.1.into()].into()),
        ("C1", vec![0.5.into(), 0.into(), 0.6.into()].into()),
        ("N", 1.into()),
    ]));
    w.add(dict([
        ("Type", name("ExtGState")),
        ("ca", 0.5.into()),
        ("CA", 0.5.into()),
    ]));
    w.finish(&dict([("Root", catalog.into())]))
}
