//! Reader object cache: resolved objects and inherited page attributes are shared, not
//! re-parsed or copied (review finding: `resolve` re-lexed every call, `pages` cloned
//! inherited attributes per kid).

use std::rc::Rc;
use std::time::{Duration, Instant};

use lamina_cos::{Dict, ObjRef, Object, Result, Writer, open};

/// A classic file with `page_count` pages, all sharing one indirect `/Resources`.
fn shared_resources_pdf(page_count: u32) -> Result<(Vec<u8>, ObjRef)> {
    let mut w = Writer::new(1, 7);
    let catalog = w.reserve();
    let pages = w.reserve();
    let font: Dict = [("Type", Object::name("Font")), ("BaseFont", Object::name("Helvetica"))]
        .into_iter()
        .collect();
    let fonts: Dict = [("F1", Object::Dict(font))].into_iter().collect();
    let resources: Dict = [("Font", Object::Dict(fonts))].into_iter().collect();
    let resources = w.add(resources);
    let media_box = Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]);
    let kids: Vec<Object> = (0..page_count)
        .map(|_| {
            let page: Dict = [
                ("Type", Object::name("Page")),
                ("Parent", pages.into()),
                ("Resources", resources.into()),
            ]
            .into_iter()
            .collect();
            w.add(page).into()
        })
        .collect();
    let pages_dict: Dict = [
        ("Type", Object::name("Pages")),
        ("Count", page_count.into()),
        ("MediaBox", media_box),
        ("Kids", Object::Array(kids)),
    ]
    .into_iter()
    .collect();
    w.set(pages, pages_dict);
    let catalog_dict: Dict = [("Type", Object::name("Catalog")), ("Pages", pages.into())]
        .into_iter()
        .collect();
    w.set(catalog, catalog_dict);
    let trailer: Dict = [("Root", catalog)].into_iter().collect();
    Ok((w.finish(&trailer)?, resources))
}

#[test]
fn resolve_should_return_the_same_rc_when_called_twice() -> Result<()> {
    let (bytes, resources) = shared_resources_pdf(1)?;
    let doc = open(&bytes)?;
    let (first, second) = (doc.resolve(resources)?, doc.resolve(resources)?);
    assert!(Rc::ptr_eq(&first, &second));
    Ok(())
}

#[test]
fn pages_should_share_one_resources_rc_across_pages() -> Result<()> {
    let (bytes, _) = shared_resources_pdf(3)?;
    let doc = open(&bytes)?;
    let pages = doc.pages()?;
    let shared: Vec<&Rc<Object>> = pages.iter().filter_map(|p| p.resources.as_ref()).collect();
    assert!(shared.len() == 3 && shared.windows(2).all(|w| matches!(w, [a, b] if Rc::ptr_eq(a, b))));
    Ok(())
}

#[test]
fn pages_should_read_a_thousand_pages_with_shared_resources_quickly() -> Result<()> {
    let (bytes, _) = shared_resources_pdf(1000)?;
    let started = Instant::now();
    let doc = open(&bytes)?;
    let (count, pages) = (doc.page_count()?, doc.pages()?);
    let elapsed = started.elapsed();
    assert!(
        count == 1000 && pages.len() == 1000 && elapsed < Duration::from_millis(500),
        "{count} pages in {elapsed:?}"
    );
    Ok(())
}
