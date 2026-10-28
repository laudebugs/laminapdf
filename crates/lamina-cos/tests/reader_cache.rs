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

/// Best-of-three wall time to open a document and walk its page tree.
fn best_open_and_walk(bytes: &[u8], expected: usize) -> Result<Duration> {
    let mut best = Duration::MAX;
    for _ in 0..3 {
        let started = Instant::now();
        let doc = open(bytes)?;
        let (count, pages) = (doc.page_count()?, doc.pages()?);
        best = best.min(started.elapsed());
        assert!(
            count == expected && pages.len() == expected,
            "{count} pages, expected {expected}"
        );
    }
    Ok(best)
}

#[test]
fn pages_should_share_resources_across_a_thousand_pages() -> Result<()> {
    let (bytes, _) = shared_resources_pdf(1000)?;
    let doc = open(&bytes)?;
    let (count, pages) = (doc.page_count()?, doc.pages()?);
    let shared: Vec<&Rc<Object>> = pages.iter().filter_map(|p| p.resources.as_ref()).collect();
    // One shared, cached `/Resources` object across every page is what keeps the walk linear.
    assert!(
        count == 1000 && shared.len() == 1000 && shared.windows(2).all(|w| matches!(w, [a, b] if Rc::ptr_eq(a, b)))
    );
    Ok(())
}

// Wall-clock scaling is too noisy for every PR (debug builds on a busy machine swing 2x+), so this
// runs on demand and in the nightly lane: `cargo test --release -p lamina-cos --test reader_cache -- --ignored`.
// 4x the pages should cost about 4x (linear), not about 16x (re-parsing shared resources per page).
#[test]
#[ignore = "timing-based; run with --release in the nightly lane"]
fn pages_should_scale_linearly_with_shared_resources() -> Result<()> {
    let (small, _) = shared_resources_pdf(1000)?;
    let (large, _) = shared_resources_pdf(4000)?;
    let (t_small, t_large) = (best_open_and_walk(&small, 1000)?, best_open_and_walk(&large, 4000)?);
    let ratio = t_large.as_secs_f64() / t_small.as_secs_f64().max(1e-6);
    assert!(
        ratio < 9.0,
        "4000 pages took {ratio:.1}x as long as 1000 ({t_large:?} vs {t_small:?})"
    );
    Ok(())
}
