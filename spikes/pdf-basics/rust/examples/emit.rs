//! Writes the writer's outputs to a directory for external validation (e.g. pypdf strict).
//! The library itself never touches the file system; this example does.
//!
//! `cargo run --example emit -- OUT_DIR`

use std::path::PathBuf;

use lamina_pdf_basics::{
    ContentBuilder, Dict, ObjRef, Object, PdfString, Stream, Writer, XrefEol, incremental_update, samples,
};

fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

/// A two-page document with a Flate content stream, an info dict and inherited MediaBox.
fn writer() -> (Writer, Dict) {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let font = w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
    ]));
    let mut kids = Vec::new();
    for text in ["Compact page one", "Compact page two"] {
        let mut c = ContentBuilder::new();
        c.begin_text()
            .font("F1", 24)
            .text_move(72, 720)
            .show_text(text)
            .end_text();
        let contents = w.add(Stream::flate(Dict::new(), c.as_bytes()));
        let resources = dict([("Font", dict([("F1", font.into())]).into())]);
        kids.push(Object::from(w.add(dict([
            ("Type", Object::name("Page")),
            ("Parent", pages.into()),
            ("Resources", resources.into()),
            ("Contents", contents.into()),
        ]))));
    }
    let count = i64::try_from(kids.len()).unwrap_or(0);
    w.set(
        pages,
        dict([
            ("Type", Object::name("Pages")),
            ("Kids", kids.into()),
            ("Count", count.into()),
            ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ]),
    );
    let root = w.add(dict([("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    let info = w.add(dict([("Producer", PdfString::literal("lamina-pdf-basics").into())]));
    (w, dict([("Root", root.into()), ("Info", info.into())]))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(std::env::args().nth(1).ok_or("usage: emit OUT_DIR")?);
    std::fs::create_dir_all(&out)?;

    let (w, trailer) = writer();
    let classic = w.finish(&trailer)?;
    let compact = w.finish_compact(&trailer)?;

    let mut c = ContentBuilder::new();
    c.begin_text()
        .font("F1", 24)
        .text_move(72, 720)
        .show_text("Hello, Update")
        .end_text();
    let page_content = Stream::new(Dict::new(), c.into_bytes());
    let hello = samples::hello()?;
    let hello_v2 = incremental_update(&hello, &[(ObjRef::new(5, 0), page_content.into())], XrefEol::CrLf)?;
    let info = dict([("Title", PdfString::literal("third revision").into())]);
    let hello_v3 = incremental_update(&hello_v2, &[(ObjRef::new(6, 0), info.into())], XrefEol::SpaceLf)?;
    // Object 8 is the /Info dictionary of the compact file.
    let new_info = dict([("Producer", PdfString::literal("lamina-pdf-basics, updated").into())]);
    let compact_v2 = incremental_update(&compact, &[(ObjRef::new(8, 0), new_info.into())], XrefEol::CrLf)?;

    for (name, bytes) in [
        ("classic.pdf", &classic),
        ("compact.pdf", &compact),
        ("hello.pdf", &hello),
        ("hello-incremental-2rev.pdf", &hello_v2),
        ("hello-incremental-3rev.pdf", &hello_v3),
        ("compact-incremental.pdf", &compact_v2),
    ] {
        std::fs::write(out.join(name), bytes)?;
        println!("{name}: {} bytes", bytes.len());
    }
    Ok(())
}
