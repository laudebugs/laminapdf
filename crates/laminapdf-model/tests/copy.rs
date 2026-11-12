//! `ObjectCopier` (todo E2p): what a copy carries, what it shares, what it leaves out, and the
//! limits that stop it before it writes.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_cos::{Document, Error, Limit, Limits, ObjRef, Object, OpenOptions};
use laminapdf_model::ObjectCopier;
use laminapdf_test_fixtures::{classic, classic_with_root, fixture, stream_body};

/// An empty target: a catalog and an empty page tree.
fn target() -> Document {
    let bytes = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
        ],
        "",
    );
    Document::open(bytes, OpenOptions::default()).unwrap()
}

fn open(bytes: Vec<u8>) -> Document {
    Document::open(bytes, OpenOptions::default()).unwrap()
}

/// A source whose objects are `bodies`, numbered from 1, object 1 its catalog.
fn source(bodies: &[&[u8]]) -> Document {
    open(classic(
        bodies,
        &format!("<< /Size {} /Root 1 0 R >>", bodies.len() + 1),
    ))
}

fn get(doc: &mut Document, r: ObjRef) -> Object {
    (*doc.edit().get(r).unwrap().unwrap()).clone()
}

fn dict_ref(object: &Object, key: &str) -> ObjRef {
    object.as_dict().unwrap().get(key).and_then(Object::as_ref).unwrap()
}

/// Two pages that share a font by reference and draw the same bytes from two content streams.
fn two_pages_one_font() -> Document {
    let content = stream_body("", b"BT /F1 12 Tf (Hi) Tj ET");
    source(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 200 200] >>",
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 6 0 R >>",
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 7 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        &content,
        &content,
    ])
}

#[test]
fn resources_shared_by_copied_pages_are_copied_once() {
    let source = two_pages_one_font();
    let mut target = target();
    let pages = source.pages().unwrap();
    let mut copier = ObjectCopier::new(&source, Limits::default());
    let new_pages = copier.copy_pages(&mut target.edit(), &[&pages[0], &pages[1]]).unwrap();
    // Two pages, one font, and one content stream: the second stream is the first's bytes.
    assert_eq!(target.overlay().len(), 4);
    let (first, second) = (get(&mut target, new_pages[0]), get(&mut target, new_pages[1]));
    assert_eq!(dict_ref(&first, "Contents"), dict_ref(&second, "Contents"));
    let font = |page: &Object| {
        let resources = page.as_dict().unwrap().get("Resources").unwrap().as_dict().unwrap();
        resources
            .get("Font")
            .unwrap()
            .as_dict()
            .unwrap()
            .get("F1")
            .and_then(Object::as_ref)
    };
    assert_eq!(font(&first), font(&second));
}

#[test]
fn a_second_copy_with_the_same_copier_reuses_what_the_first_copied() {
    let source = two_pages_one_font();
    let mut target = target();
    let pages = source.pages().unwrap();
    let mut copier = ObjectCopier::new(&source, Limits::default());
    copier.copy_pages(&mut target.edit(), &[&pages[0]]).unwrap();
    let before = target.overlay().len();
    copier.copy_pages(&mut target.edit(), &[&pages[1]]).unwrap();
    // Only the second page is new.
    assert_eq!(target.overlay().len(), before + 1);
}

#[test]
fn a_copied_page_drops_its_parent_and_carries_what_it_inherited() {
    let source = source(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 300 400] /Rotate 90 /Resources 4 0 R >>",
        b"<< /Type /Pages /Parent 2 0 R /Kids [5 0 R] /Count 1 /CropBox [10 10 290 390] >>",
        b"<< /ProcSet [/PDF] >>",
        b"<< /Type /Page /Parent 3 0 R /Rotate 180 >>",
    ]);
    let mut target = target();
    let pages = source.pages().unwrap();
    let new_pages = ObjectCopier::new(&source, Limits::default())
        .copy_pages(&mut target.edit(), &[&pages[0]])
        .unwrap();
    let page = get(&mut target, new_pages[0]);
    let dict = page.as_dict().unwrap();
    assert!(dict.get("Parent").is_none());
    assert_eq!(dict.get("MediaBox").and_then(Object::as_array).map(<[_]>::len), Some(4));
    assert_eq!(dict.get("CropBox").and_then(Object::as_array).map(<[_]>::len), Some(4));
    // The page's own /Rotate wins over the inherited one.
    assert_eq!(dict.get("Rotate").and_then(Object::as_int), Some(180));
    // The inherited resources were an indirect object: it is copied, as a new object.
    let resources = dict_ref(&page, "Resources");
    assert!(matches!(
        target.overlay().get(resources.num),
        Some(laminapdf_cos::OverlayEntry::New(_))
    ));
    assert!(get(&mut target, resources).as_dict().unwrap().get("ProcSet").is_some());
}

#[test]
fn links_between_pages_copied_together_point_at_the_copies_and_others_become_null() {
    let source = source(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 /MediaBox [0 0 200 200] >>",
        b"<< /Type /Page /Parent 2 0 R /Annots [6 0 R 7 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R >>",
        b"<< /Type /Page /Parent 2 0 R >>",
        b"<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /P 3 0 R /Dest [4 0 R /Fit] >>",
        b"<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /P 3 0 R /Dest [5 0 R /Fit] >>",
    ]);
    let mut target = target();
    let pages = source.pages().unwrap();
    let mut copier = ObjectCopier::new(&source, Limits::default());
    let new_pages = copier.copy_pages(&mut target.edit(), &[&pages[0], &pages[1]]).unwrap();
    let page = get(&mut target, new_pages[0]);
    let annots: Vec<ObjRef> = page
        .as_dict()
        .unwrap()
        .get("Annots")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_ref().unwrap())
        .collect();
    let dest = |doc: &mut Document, annot: ObjRef| {
        get(doc, annot)
            .as_dict()
            .unwrap()
            .get("Dest")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .clone()
    };
    // The annotation's /P is the copied page; the link to page 2 follows it.
    assert_eq!(dict_ref(&get(&mut target, annots[0]), "P"), new_pages[0]);
    assert_eq!(dest(&mut target, annots[0]), Object::Ref(new_pages[1]));
    // Page 3 was not copied: the link to it is null, and the copier lists it.
    assert_eq!(dest(&mut target, annots[1]), Object::Null);
    assert_eq!(
        copier.outside().iter().copied().collect::<Vec<_>>(),
        [ObjRef::new(5, 0)]
    );
    assert_eq!(copier.copy_of(ObjRef::new(5, 0)), Some(None));
    assert_eq!(copier.copy_of(ObjRef::new(4, 0)), Some(Some(new_pages[1])));
}

#[test]
fn a_reference_cycle_is_copied_once_and_stays_a_cycle() {
    let source = source(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>",
        b"<< /Type /Page /Parent 2 0 R /Thing 4 0 R >>",
        b"<< /Next 5 0 R >>",
        b"<< /Next 4 0 R >>",
    ]);
    let mut target = target();
    let pages = source.pages().unwrap();
    let new_pages = ObjectCopier::new(&source, Limits::default())
        .copy_pages(&mut target.edit(), &[&pages[0]])
        .unwrap();
    let a = dict_ref(&get(&mut target, new_pages[0]), "Thing");
    let b = dict_ref(&get(&mut target, a), "Next");
    assert_eq!(dict_ref(&get(&mut target, b), "Next"), a);
    assert_eq!(target.overlay().len(), 3);
}

#[test]
fn a_copy_past_the_stream_byte_limit_stops_and_leaves_the_target_unchanged() {
    let source = two_pages_one_font();
    let mut target = target();
    let pages = source.pages().unwrap();
    let limits = Limits::default().with_max_decoded_total(10);
    let err = ObjectCopier::new(&source, limits)
        .copy_pages(&mut target.edit(), &[&pages[0], &pages[1]])
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::LimitExceeded {
                limit: Limit::DecodedTotal,
                ..
            }
        ),
        "{err:?}"
    );
    assert!(!target.is_dirty());
}

#[test]
fn a_copy_past_the_nesting_limit_stops_and_leaves_the_target_unchanged() {
    let source = source(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>",
        b"<< /Type /Page /Parent 2 0 R /Deep 4 0 R >>",
        b"[[[[[[1]]]]]]",
    ]);
    let mut target = target();
    let pages = source.pages().unwrap();
    let limits = Limits::default().with_max_nesting_depth(3);
    let err = ObjectCopier::new(&source, limits)
        .copy_pages(&mut target.edit(), &[&pages[0]])
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::LimitExceeded {
                limit: Limit::NestingDepth,
                ..
            }
        ),
        "{err:?}"
    );
    assert!(!target.is_dirty());
}

#[test]
fn an_encrypted_sources_streams_are_copied_decrypted_without_the_crypt_filter() {
    for file in ["r4-aes-128-user.pdf", "crypt-filter-identity.pdf"] {
        let options = OpenOptions::default().with_password("user-pw");
        let source = Document::open(fixture(&format!("encrypted/{file}")), options)
            .or_else(|_| Document::open(fixture(&format!("encrypted/{file}")), OpenOptions::default()))
            .unwrap();
        let mut target = target();
        let pages = source.pages().unwrap();
        let new_pages = ObjectCopier::new(&source, Limits::default())
            .copy_pages(&mut target.edit(), &[&pages[0]])
            .unwrap();
        let page = get(&mut target, new_pages[0]);
        let contents = dict_ref(&page, "Contents");
        let Object::Stream(copy) = get(&mut target, contents) else {
            panic!("{file}: the contents are not a stream");
        };
        let filters = format!("{:?}", copy.dict.get("Filter"));
        assert!(!filters.contains("Crypt"), "{file}: {filters}");
        let Object::Stream(original) = &*source
            .resolve(pages[0].dict.get("Contents").unwrap().as_ref().unwrap())
            .unwrap()
        else {
            panic!("{file}: the source contents are not a stream");
        };
        assert_eq!(
            target.decode_stream(&copy).unwrap(),
            source.decode_stream(original).unwrap(),
            "{file}"
        );
    }
}
