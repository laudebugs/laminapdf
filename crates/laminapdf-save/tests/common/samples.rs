//! The edited goldens: [`legacy::hello`](super::legacy::hello) opened, changed with an op and
//! saved by the overlay writers.

use laminapdf_cos::Result;

use super::legacy::hello;

/// `fixtures/golden/incremental-set-info.pdf`: [`hello`] opened for editing, one `SetInfo`
/// (title, author, a fixed `/ModDate`), saved incrementally: the overlay writer's classic
/// section, a new Info object and a derived `/ID`.
///
/// # Errors
/// Never in practice (other failures panic: this is test support).
pub fn incremental_set_info() -> Result<Vec<u8>> {
    use laminapdf_save::Save as _;
    let mut doc = laminapdf_ops::Document::open(hello()?, laminapdf_cos::OpenOptions::default()).unwrap();
    let at = "D:20261029120000Z".parse::<laminapdf_model::PdfDate>().unwrap();
    let patch = laminapdf_ops::InfoPatch::new()
        .title(Some("Hello, Save"))
        .author(Some("lamina"));
    doc.apply_all(
        laminapdf_ops::Origin::Local,
        [laminapdf_ops::SetInfo::new(patch, Some(at)).into()],
    )
    .unwrap();
    let mut out = Vec::new();
    doc.save(
        &mut out,
        &laminapdf_save::SaveOptions::new().with_mode(laminapdf_save::SaveMode::Incremental),
    )
    .unwrap();
    Ok(out)
}

/// `fixtures/golden/full-rewrite-hello.pdf`: [`hello`] with its page rotated by 90°, written
/// by the full-rewrite writer with the defaults (numbers kept, object stream + xref stream).
///
/// # Errors
/// Never in practice (other failures panic: this is test support).
pub fn full_rewrite_hello() -> Result<Vec<u8>> {
    use laminapdf_save::Save as _;
    let mut doc = laminapdf_ops::Document::open(hello()?, laminapdf_cos::OpenOptions::default()).unwrap();
    let page = doc.pages()?.get(0).map(|p| p.id().clone()).unwrap();
    doc.apply_all(
        laminapdf_ops::Origin::Local,
        [laminapdf_ops::RotatePage::new(page, laminapdf_model::Rotation::Clockwise90).into()],
    )
    .unwrap();
    let mut out = Vec::new();
    doc.save(
        &mut out,
        &laminapdf_save::SaveOptions::new().with_mode(laminapdf_save::SaveMode::FullRewrite),
    )
    .unwrap();
    Ok(out)
}
