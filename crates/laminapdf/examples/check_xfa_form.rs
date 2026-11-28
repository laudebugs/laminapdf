//! Check whether a PDF form is an XFA form, and make a static one show the same answers in
//! every app.
//!
//! An XFA form is an older Adobe format that only Adobe's apps lay out. A dynamic one has
//! placeholder pages and nothing to fill here; a static one has real pages and ordinary fields,
//! but Adobe's apps show the XFA part's own answers, so the part is removed.
//!
//! ```sh
//! cargo run -p laminapdf --example check_xfa_form -- form.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, RemoveXfa, SaveMode, SaveOptions, XfaKind};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: check_xfa_form form.pdf [out-dir]";
    let form = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("check_xfa_form.pdf"), check(std::fs::read(form)?)?)?;
    Ok(())
}

/// The form as it saves: without its XFA part when it is a static XFA form, unchanged otherwise.
pub(crate) fn check(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    match doc.xfa()?.map(|xfa| xfa.kind) {
        None => eprintln!("not an XFA form"),
        // Only Adobe's apps lay it out: leave it as it is.
        Some(XfaKind::Dynamic) => eprintln!("a dynamic XFA form: open it in Adobe Acrobat"),
        Some(_) => {
            eprintln!("a static XFA form: removing its XFA part");
            doc.apply(vec![RemoveXfa::new().into()])?;
        }
    }
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    Ok(doc.save(&options)?.bytes)
}
