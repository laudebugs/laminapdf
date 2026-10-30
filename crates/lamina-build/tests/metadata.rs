//! Metadata (todo B5b): the Info dictionary and the XMP packet carry the same values, the
//! catalog's `/Lang` and viewer preferences, and no ambient time (the same options give the
//! same bytes).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::b5b::{self, golden_metadata};
use common::{fixture, reopen, save};
use lamina_build::{BuildOptions, Builder, Metadata, PageSize};
use lamina_cos::Object;

/// The text of the first `<tag>…</tag>` in `xml` (inside `rdf:li` for the `dc:` arrays),
/// with the five XML escapes undone.
fn xmp_value(xml: &str, tag: &str) -> Option<String> {
    let start = xml.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = start + xml[start..].find(&format!("</{tag}>"))?;
    let mut inner = &xml[start..end];
    if let Some(li) = inner.find("<rdf:li") {
        let open = li + inner[li..].find('>')? + 1;
        inner = &inner[open..open + inner[open..].find("</rdf:li>")?];
    }
    Some(
        inner
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#xA;", "\n")
            .replace("&amp;", "&"),
    )
}

fn xmp(doc: &lamina_model::Document) -> String {
    let catalog = doc.catalog().unwrap();
    let r = catalog.get("Metadata").unwrap().as_ref().unwrap();
    let Object::Stream(s) = &*doc.resolve(r).unwrap() else {
        panic!()
    };
    assert_eq!(
        (
            s.dict.get_name("Type"),
            s.dict.get_name("Subtype"),
            s.dict.contains_key("Filter")
        ),
        (Some(&b"Metadata"[..]), Some(&b"XML"[..]), false)
    );
    String::from_utf8(doc.decode_stream(s).unwrap()).unwrap()
}

/// `2026-10-29T12:00:00Z` → `D:20261029120000Z`; `…+01:00` → `D:…+01'00'`.
fn pdf_date(x: &str) -> String {
    let digits: String = x[..19].chars().filter(char::is_ascii_digit).collect();
    let zone = match &x[19..] {
        "Z" => "Z".to_owned(),
        "" => String::new(),
        z => format!("{}'{}'", &z[..3], &z[4..6]),
    };
    format!("D:{digits}{zone}")
}

#[test]
fn info_and_xmp_carry_the_same_values() {
    let doc = reopen(&fixture("golden/metadata.pdf"));
    let info = doc.info().unwrap().unwrap();
    let xml = xmp(&doc);
    let pairs = [
        (info.title().unwrap(), xmp_value(&xml, "dc:title")),
        (info.author().unwrap(), xmp_value(&xml, "dc:creator")),
        (info.subject().unwrap(), xmp_value(&xml, "dc:description")),
        (info.keywords().unwrap(), xmp_value(&xml, "pdf:Keywords")),
        (info.producer().unwrap(), xmp_value(&xml, "pdf:Producer")),
        (info.creator().unwrap(), xmp_value(&xml, "xmp:CreatorTool")),
        (
            info.creation_date().unwrap().map(|d| d.raw),
            xmp_value(&xml, "xmp:CreateDate").map(|d| pdf_date(&d)),
        ),
        (
            info.mod_date().unwrap().map(|d| d.raw),
            xmp_value(&xml, "xmp:ModifyDate").map(|d| pdf_date(&d)),
        ),
    ];
    for (from_info, from_xmp) in pairs {
        assert!(from_info.is_some());
        assert_eq!(from_info, from_xmp);
    }
    let meta = golden_metadata();
    assert_eq!(info.title().unwrap(), meta.title);
}

#[test]
fn the_catalog_has_lang_and_viewer_preferences() {
    let doc = reopen(&fixture("golden/metadata.pdf"));
    let c = doc.catalog().unwrap();
    let prefs = c.get("ViewerPreferences").unwrap().as_dict().unwrap();
    assert_eq!(
        (
            c.get("Lang"),
            prefs.get("DisplayDocTitle"),
            c.get_name("PageLayout"),
            c.get_name("PageMode")
        ),
        (
            Some(&Object::String(lamina_cos::PdfString::literal(&b"en-GB"[..]))),
            Some(&Object::Bool(true)),
            Some(&b"OneColumn"[..]),
            Some(&b"UseNone"[..])
        )
    );
}

#[test]
fn metadata_falls_back_to_the_options() {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_producer("from options")
            .with_creation_date("D:20260101000000Z".parse().unwrap()),
    );
    b.page(PageSize::A6).finish().unwrap();
    b.metadata(Metadata::default().with_title("T"));
    let doc = reopen(&save(b.finish().unwrap()));
    let info = doc.info().unwrap().unwrap();
    let xml = xmp(&doc);
    assert_eq!(
        (info.producer().unwrap(), xmp_value(&xml, "pdf:Producer")),
        (Some("from options".to_owned()), Some("from options".to_owned()))
    );
    assert_eq!(
        xmp_value(&xml, "xmp:CreateDate").as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
}

#[test]
fn unicode_and_markup_survive_both() {
    let title = "Ünïcødé — 中文 & <tags> \"quoted\"";
    let mut b = Builder::new(BuildOptions::default());
    b.page(PageSize::A6).finish().unwrap();
    b.metadata(Metadata::default().with_title(title));
    let doc = reopen(&save(b.finish().unwrap()));
    let info = doc.info().unwrap().unwrap().title().unwrap();
    assert_eq!(
        (info.as_deref(), xmp_value(&xmp(&doc), "dc:title").as_deref()),
        (Some(title), Some(title))
    );
}

#[test]
fn no_ambient_time_the_same_options_give_the_same_bytes() {
    assert_eq!(b5b::metadata(), b5b::metadata());
    assert_eq!(b5b::metadata(), fixture("golden/metadata.pdf"));
}

#[test]
fn without_metadata_no_xmp_is_written() {
    let mut b = Builder::new(BuildOptions::default());
    b.page(PageSize::A6).finish().unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    assert!(!doc.catalog().unwrap().contains_key("Metadata"));
}

#[test]
fn control_characters_are_dropped_from_info_and_xmp_alike() {
    let mut b = Builder::new(BuildOptions::default());
    b.page(PageSize::A6).finish().unwrap();
    b.metadata(Metadata::default().with_title("a\u{1}b\tc"));
    let doc = reopen(&save(b.finish().unwrap()));
    let info = doc.info().unwrap().unwrap().title().unwrap();
    let xml = xmp(&doc);
    assert_eq!(info.as_deref(), Some("ab\tc"));
    assert_eq!(xmp_value(&xml, "dc:title").map(|t| t.replace("&#x9;", "\t")), info);
}
