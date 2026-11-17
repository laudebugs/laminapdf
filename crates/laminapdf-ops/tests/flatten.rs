//! `FlattenAnnots` over `fixtures/annotations/flatten.pdf`: what each intent flattens and leaves,
//! what it refuses, the content it appends, the structure tree it edits, undo and redo, and the
//! frozen sample.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{fixture, open, snapshot};
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::Object;
use laminapdf_model::AnnotId;
use laminapdf_ops::{Applied, Document, Error, FlattenAnnots, FlattenIntent, ModificationClass, Op, Origin};

fn doc() -> Document {
    open(&fixture("annotations/flatten.pdf")).with_appearances(AnnotationAppearances)
}

fn id(text: &str) -> AnnotId {
    text.parse().unwrap()
}

fn ids(texts: &[&str]) -> Vec<AnnotId> {
    texts.iter().map(|t| id(t)).collect()
}

/// Every annotation of page 1 but the widget, in `/Annots` order.
const PAGE_ONE: [&str; 10] = [
    "a10.0", "a11.0", "a12.0", "a13.0", "a14.0", "a15.0", "a16.0", "a17.0", "a19.0", "a22.0",
];

fn flatten(d: &mut Document, annots: &[&str], intent: FlattenIntent) -> Result<Applied, Error> {
    d.apply_all(Origin::Local, [FlattenAnnots::new(ids(annots), intent).into()])
}

/// The object numbers of the annotations page `index` lists.
fn listed(d: &Document, index: usize) -> Vec<u32> {
    let pages = d.pages().unwrap();
    let page = pages.iter().nth(index).unwrap();
    d.annotation_links(&page)
        .unwrap()
        .iter()
        .filter_map(|a| a.id.as_lamina_id().obj_ref())
        .map(|r| r.num)
        .collect()
}

/// Page `index`'s content, every stream joined.
fn content(d: &Document, index: usize) -> String {
    let pages = d.pages().unwrap();
    let page = pages.iter().nth(index).unwrap();
    String::from_utf8_lossy(&d.cos().page_content(page.cos()).unwrap()).into_owned()
}

/// The objects page 1's `/XObject` resources name.
fn forms(d: &Document) -> Vec<u32> {
    let pages = d.pages().unwrap();
    let page = pages.iter().next().unwrap();
    let r = page.id().as_lamina_id().obj_ref().unwrap();
    let dict = laminapdf_cos::tolerant(d.resolve(r)).unwrap().unwrap();
    let resources = dict.as_dict().unwrap().get("Resources").cloned().unwrap();
    let resources = d.deref(&resources).unwrap().as_dict().cloned().unwrap();
    let xobjects = resources
        .get("XObject")
        .and_then(Object::as_dict)
        .cloned()
        .unwrap_or_default();
    let mut out: Vec<u32> = xobjects.iter().filter_map(|(_, v)| v.as_ref()).map(|r| r.num).collect();
    out.sort_unstable();
    out
}

#[test]
fn on_screen_it_leaves_what_is_hidden_off_or_undrawable() {
    let mut d = doc();
    let applied = flatten(&mut d, &PAGE_ONE, FlattenIntent::View).unwrap();
    // Left: hidden (12), print-only (13), a layer that is off (15), an attachment with no
    // appearance (17) and the widget (18).
    assert_eq!(
        (listed(&d, 0), applied.not_flattened),
        (vec![12, 13, 15, 17, 18], vec![id("a17.0")])
    );
}

#[test]
fn printed_it_keeps_print_only_marks_and_leaves_screen_only_ones() {
    let mut d = doc();
    flatten(&mut d, &PAGE_ONE, FlattenIntent::Print).unwrap();
    assert_eq!(listed(&d, 0), vec![12, 14, 15, 17, 18]);
}

#[test]
fn a_note_goes_with_its_popup_and_its_reply() {
    let mut d = doc();
    let applied = flatten(&mut d, &["a19.0"], FlattenIntent::View).unwrap();
    let mut also = applied.also_deleted;
    also.sort();
    assert_eq!(also, ids(&["a20.0", "a21.0"]));
}

#[test]
fn the_state_as_names_is_the_one_drawn() {
    let mut d = doc();
    flatten(&mut d, &["a11.0"], FlattenIntent::View).unwrap();
    // 33 is the stamp's `/On` appearance, 34 its `/Off`.
    assert_eq!(forms(&d), vec![33]);
}

#[test]
fn each_appearance_is_drawn_after_the_wrapped_content_as_an_artifact() {
    let mut d = doc();
    flatten(&mut d, &["a10.0", "a22.0"], FlattenIntent::View).unwrap();
    let text = content(&d, 0);
    let wrapped = text.trim_start().starts_with('q');
    let draws = text.matches(" Do").count();
    let artifacts = text.matches("/Artifact BMC").count();
    // The translucent square draws through an ExtGState with its `/CA`.
    assert_eq!((wrapped, draws, artifacts, text.contains(" gs")), (true, 2, 2, true));
}

#[test]
fn the_quarter_turned_appearance_fills_its_rect() {
    let mut d = doc();
    flatten(&mut d, &["a22.0"], FlattenIntent::View).unwrap();
    // The box [0 0 30 80], which `Do` turns by the form's [0 1 -1 0 0 0] to x −80…0, y 0…30, is
    // moved onto [200 110 280 140]: the page only shifts it.
    assert!(content(&d, 0).contains("1 0 0 1 280 110 cm"), "{}", content(&d, 0));
}

#[test]
fn a_mark_without_an_appearance_gets_one_first() {
    let mut d = doc();
    let applied = flatten(&mut d, &["a16.0"], FlattenIntent::View).unwrap();
    assert_eq!(
        (
            applied.not_flattened.is_empty(),
            listed(&d, 0).contains(&16),
            forms(&d).len()
        ),
        (true, false, 1)
    );
}

#[test]
fn without_a_generator_a_mark_without_an_appearance_stays() {
    let mut d = open(&fixture("annotations/flatten.pdf"));
    let applied = flatten(&mut d, &["a16.0"], FlattenIntent::View).unwrap();
    assert_eq!(
        (applied.not_flattened, listed(&d, 0).contains(&16)),
        (vec![id("a16.0")], true)
    );
}

#[test]
fn a_widget_is_refused() {
    let mut d = doc();
    let before = snapshot(&d);
    let err = flatten(&mut d, &["a10.0", "a18.0"], FlattenIntent::View).unwrap_err();
    assert_eq!((err.code(), snapshot(&d) == before), ("flatten-widget", true));
}

#[test]
fn an_unknown_annotation_is_refused() {
    let mut d = doc();
    let err = flatten(&mut d, &["a99.0"], FlattenIntent::View).unwrap_err();
    assert_eq!(err.code(), "unknown-annotation");
}

#[test]
fn a_tagged_annotation_leaves_the_structure_tree() {
    let mut d = doc();
    flatten(&mut d, &["a60.0"], FlattenIntent::View).unwrap();
    let elem = laminapdf_cos::tolerant(d.resolve(laminapdf_cos::ObjRef::new(71, 0)))
        .unwrap()
        .unwrap();
    let kids = elem.as_dict().unwrap().get("K").cloned().unwrap();
    // The Annot element (73) is gone; the paragraph (72) stays.
    let kids: Vec<u32> = kids
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Object::as_ref)
        .map(|r| r.num)
        .collect();
    assert_eq!((kids, listed(&d, 1)), (vec![72], Vec::new()));
}

#[test]
fn it_is_a_content_change_that_stays_in_earlier_revisions() {
    let op: Op = FlattenAnnots::new(ids(&["a10.0"]), FlattenIntent::View).into();
    let mut d = doc();
    let applied = d.apply_all(Origin::Local, [op.clone()]).unwrap();
    assert_eq!(
        (op.class(), op.requires_rewrite(), applied.still_in_earlier_revisions),
        (ModificationClass::Content, false, true)
    );
}

#[test]
fn an_id_another_one_took_is_skipped() {
    let mut d = doc();
    // The reply goes with the note, so naming it after the note is not an error.
    let applied = flatten(&mut d, &["a19.0", "a21.0"], FlattenIntent::View).unwrap();
    assert!(applied.also_deleted.contains(&id("a21.0")));
}

#[test]
fn undo_leaves_the_overlay_empty_and_redo_writes_the_same_again() {
    let mut d = doc();
    let before = snapshot(&d);
    flatten(&mut d, &PAGE_ONE, FlattenIntent::View).unwrap();
    let after = snapshot(&d);
    d.undo().unwrap();
    let undone = (snapshot(&d), d.overlay().is_empty());
    d.redo().unwrap();
    assert_eq!((undone, snapshot(&d)), ((before, true), after));
}

#[test]
fn the_frozen_sample_applies() {
    let mut d = doc();
    let op: Op = serde_json::from_slice(&fixture("ops/v1/flatten-annots.json")).unwrap();
    let applied = d.apply_all(Origin::Local, [op]).unwrap();
    assert!(!listed(&d, 0).contains(&10) && applied.still_in_earlier_revisions);
}

#[test]
fn the_wire_rejects_unknown_fields_and_intents() {
    for json in [
        r#"{ "op": "flattenAnnots", "args": { "annots": ["a10.0"], "intent": "view", "all": true } }"#,
        r#"{ "op": "flattenAnnots", "args": { "annots": ["a10.0"], "intent": "export" } }"#,
        r#"{ "op": "flattenAnnots", "args": { "annots": ["a10.0"] } }"#,
        r#"{ "op": "flattenAnnots", "args": { "annots": ["not an id"], "intent": "view" } }"#,
    ] {
        assert!(serde_json::from_str::<Op>(json).is_err(), "{json}");
    }
}

/// The edge cases' file: membership dictionaries, a blend mode, a form without `/Subtype`, locks,
/// both opacities, a reply without an appearance and a `NoRotate` square on a turned page.
fn cases() -> Document {
    open(&fixture("annotations/flatten-cases.pdf")).with_appearances(AnnotationAppearances)
}

/// The resources of `category` on page `index`, by name.
fn resources(d: &Document, index: usize, category: &str) -> laminapdf_cos::Dict {
    let pages = d.pages().unwrap();
    let page = pages.iter().nth(index).unwrap();
    let r = page.id().as_lamina_id().obj_ref().unwrap();
    let dict = laminapdf_cos::tolerant(d.resolve(r)).unwrap().unwrap();
    let resources = dict.as_dict().unwrap().get("Resources").cloned().unwrap();
    let resources = d.deref(&resources).unwrap().as_dict().cloned().unwrap();
    let entries = resources
        .get(category)
        .cloned()
        .unwrap_or(Object::Dict(laminapdf_cos::Dict::new()));
    d.deref(&entries).unwrap().as_dict().cloned().unwrap()
}

#[test]
fn a_mark_shown_by_its_membership_keeps_it_as_marked_content() {
    let mut d = cases();
    flatten(&mut d, &["a10.0"], FlattenIntent::View).unwrap();
    let properties = resources(&d, 0, "Properties");
    let (name, membership) = properties.iter().next().unwrap();
    let marked = format!("/OC /{} BDC", String::from_utf8_lossy(name.as_bytes()));
    assert_eq!(
        (membership.as_ref().map(|r| r.num), content(&d, 0).contains(&marked)),
        (Some(50), true)
    );
}

#[test]
fn a_mark_its_membership_hides_stays() {
    let mut d = cases();
    flatten(&mut d, &["a11.0"], FlattenIntent::View).unwrap();
    assert!(listed(&d, 0).contains(&11));
}

#[test]
fn a_blend_mode_is_kept() {
    let mut d = cases();
    flatten(&mut d, &["a12.0"], FlattenIntent::View).unwrap();
    let states = resources(&d, 0, "ExtGState");
    let (_, gs) = states.iter().next().unwrap();
    let gs = d.deref(gs).unwrap().as_dict().cloned().unwrap();
    assert_eq!(gs.get_name("BM"), Some(b"Multiply".as_slice()));
}

#[test]
fn both_opacities_are_kept() {
    let mut d = cases();
    flatten(&mut d, &["a17.0"], FlattenIntent::View).unwrap();
    let states = resources(&d, 0, "ExtGState");
    let (_, gs) = states.iter().next().unwrap();
    let gs = d.deref(gs).unwrap().as_dict().cloned().unwrap();
    assert_eq!(
        (
            gs.get("CA").and_then(Object::as_f64),
            gs.get("ca").and_then(Object::as_f64)
        ),
        (Some(0.5), Some(0.25))
    );
}

#[test]
fn an_appearance_without_its_subtype_is_drawn_from_a_form_copy() {
    let mut d = cases();
    flatten(&mut d, &["a13.0"], FlattenIntent::View).unwrap();
    let forms = resources(&d, 0, "XObject");
    let (_, form) = forms.iter().next().unwrap();
    let copy = form.as_ref().unwrap();
    let stream = laminapdf_cos::tolerant(d.resolve(copy)).unwrap().unwrap();
    let Object::Stream(stream) = &*stream else {
        panic!("not a stream")
    };
    assert_eq!(
        (copy.num == 33, stream.dict.get_name("Subtype")),
        (false, Some(b"Form".as_slice()))
    );
}

#[test]
fn a_locked_mark_is_refused_before_anything_is_drawn() {
    let mut d = cases();
    let before = snapshot(&d);
    let err = flatten(&mut d, &["a14.0"], FlattenIntent::View).unwrap_err();
    assert_eq!((err.code(), snapshot(&d) == before), ("annotation-locked", true));
}

#[test]
fn a_mark_with_a_locked_reply_is_refused_naming_the_reply() {
    let mut d = cases();
    let err = flatten(&mut d, &["a18.0"], FlattenIntent::View).unwrap_err();
    assert!(matches!(err, Error::AnnotationLocked(id) if id == "a19.0".parse::<AnnotId>().unwrap()));
}

#[test]
fn a_reply_without_an_appearance_is_removed_but_not_drawn() {
    let mut d = cases();
    let applied = flatten(&mut d, &["a15.0"], FlattenIntent::View).unwrap();
    assert_eq!(
        (applied.also_deleted, content(&d, 0).matches(" Do").count()),
        (vec![id("a16.0")], 1)
    );
}

#[test]
fn a_no_rotate_mark_on_a_turned_page_turns_back_about_its_upper_left_corner() {
    let mut d = cases();
    flatten(&mut d, &["a20.0"], FlattenIntent::View).unwrap();
    // The 50 × 50 box fits [20 20 70 70], then turns 90° anticlockwise about (20, 70), the
    // rectangle's upper-left corner, which the page's clockwise turn brings back upright.
    assert!(content(&d, 1).contains("0 1 -1 0 70 70 cm"), "{}", content(&d, 1));
}

#[test]
fn the_candidates_leave_out_a_mark_whose_reply_is_locked() {
    let d = cases();
    let candidates = laminapdf_ops::flatten_candidates(&d, None).unwrap();
    assert_eq!(
        candidates,
        ids(&["a10.0", "a11.0", "a12.0", "a13.0", "a15.0", "a17.0", "a20.0"])
    );
}
