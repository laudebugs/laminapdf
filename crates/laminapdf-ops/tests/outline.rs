//! The outline and named-destination ops over `fixtures/corpus/outline.pdf`: three top-level
//! bookmarks (Introduction to page 1, a chapter with two children naming `/chapter2` in the
//! catalog's `/Dests`, an appendix with one child whose `GoTo` names `(appendix)` in the
//! `/Names` tree). Each op does what it says, keeps the outline whole, and keeps a name in the
//! store it was found in.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::corpus;
use laminapdf_cos::Object;
use laminapdf_model::{OutlineItemId, OutlineTree, SiblingPosition};
use laminapdf_ops::{
    AddOutlineItem, Applied, ChildItems, DeleteNamedDestination, DeleteOutlineItem, DestinationUse, DestinationView,
    Document, ExplicitDestination, MoveOutlineItem, Op, Origin, OutlineStyle, OutlineTarget, RenameLinks,
    RenameNamedDestination, Rgb, SetNamedDestination, UpdateOutlineItem,
};

fn doc() -> Document {
    corpus("outline.pdf")
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> Applied {
    doc.apply_all(Origin::Local, [op.into()]).unwrap()
}

fn refused(doc: &mut Document, op: impl Into<Op>) -> &'static str {
    doc.apply_all(Origin::Local, [op.into()]).unwrap_err().code()
}

fn top(doc: &Document) -> Vec<OutlineItemId> {
    doc.outline().unwrap().roots().filter_map(|i| i.id().cloned()).collect()
}

fn titles(doc: &Document) -> Vec<(usize, String)> {
    doc.outline()
        .unwrap()
        .iter()
        .map(|i| (i.depth(), i.title().unwrap()))
        .collect()
}

fn item(doc: &Document, id: &OutlineItemId) -> laminapdf_cos::Dict {
    doc.resolve(id.as_lamina_id().obj_ref().unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone()
}

fn page(doc: &Document, index: usize) -> laminapdf_model::PageId {
    doc.pages().unwrap().get(index).unwrap().id().clone()
}

fn fit(doc: &Document, index: usize) -> OutlineTarget {
    OutlineTarget::Destination(ExplicitDestination::new(page(doc, index), DestinationView::Fit))
}

fn whole(doc: &Document) {
    assert!(
        !OutlineTree::read(doc).unwrap().repaired(),
        "the outline's links are whole"
    );
}

mod add {
    use super::*;

    #[test]
    fn a_new_item_and_a_child_under_it_go_where_they_say() {
        let mut doc = doc();
        let target = fit(&doc, 2);
        let added = apply(
            &mut doc,
            AddOutlineItem::new(None, SiblingPosition::First, "Summary", target),
        );
        let parent = added.created_outline_items[0].clone();
        let mut child = AddOutlineItem::new(Some(parent.clone()), SiblingPosition::Last, "Grüße", fit(&doc, 0));
        child.style = OutlineStyle::new(Some(Rgb::new(200, 0, 0)), true, false);
        apply(&mut doc, child);
        assert_eq!(
            &titles(&doc)[..2],
            &[(1, "Summary".to_owned()), (2, "Grüße".to_owned())]
        );
        let outline = doc.outline().unwrap();
        let target = outline.by_id(&parent).unwrap().target().unwrap().unwrap();
        assert_eq!(target.page_index, Some(2));
        whole(&doc);
    }

    #[test]
    fn its_target_and_style_are_written() {
        let mut doc = doc();
        let mut op = AddOutlineItem::new(
            None,
            SiblingPosition::Last,
            "Site",
            OutlineTarget::Uri("https://example.com".into()),
        );
        op.style = OutlineStyle::new(Some(Rgb::new(255, 0, 0)), true, true);
        let id = apply(&mut doc, op).created_outline_items[0].clone();
        let dict = item(&doc, &id);
        assert_eq!(dict.get("F"), Some(&Object::Integer(3)));
        let action = dict.get("A").unwrap().as_dict().unwrap();
        assert_eq!(action.get_name("S"), Some(&b"URI"[..]));
    }

    #[test]
    fn a_named_target_takes_the_form_its_store_reads() {
        let mut doc = doc();
        let in_dests = apply(
            &mut doc,
            AddOutlineItem::new(
                None,
                SiblingPosition::Last,
                "A",
                OutlineTarget::Named("chapter2".into()),
            ),
        );
        let in_tree = apply(
            &mut doc,
            AddOutlineItem::new(
                None,
                SiblingPosition::Last,
                "B",
                OutlineTarget::Named("appendix".into()),
            ),
        );
        assert!(matches!(
            item(&doc, &in_dests.created_outline_items[0]).get("Dest"),
            Some(Object::Name(_))
        ));
        assert!(matches!(
            item(&doc, &in_tree.created_outline_items[0]).get("Dest"),
            Some(Object::String(_))
        ));
    }

    #[test]
    fn an_unsafe_address_an_unknown_parent_and_an_unknown_page_are_refused() {
        let mut doc = doc();
        let js = AddOutlineItem::new(
            None,
            SiblingPosition::Last,
            "x",
            OutlineTarget::Uri("javascript:alert(1)".into()),
        );
        assert_eq!(refused(&mut doc, js), "link-uri-refused");
        let orphan = AddOutlineItem::new(Some("o99.0".parse().unwrap()), SiblingPosition::Last, "x", fit(&doc, 0));
        assert_eq!(refused(&mut doc, orphan), "unknown-outline-item");
        let nowhere =
            OutlineTarget::Destination(ExplicitDestination::new("p99.0".parse().unwrap(), DestinationView::Fit));
        assert_eq!(
            refused(&mut doc, AddOutlineItem::new(None, SiblingPosition::Last, "x", nowhere)),
            "unknown-page"
        );
        let nan = DestinationView::Xyz {
            left: Some(f64::NAN),
            top: None,
            zoom: None,
        };
        let bad = OutlineTarget::Destination(ExplicitDestination::new(page(&doc, 0), nan));
        assert_eq!(
            refused(&mut doc, AddOutlineItem::new(None, SiblingPosition::Last, "x", bad)),
            "view-not-finite"
        );
        assert!(!doc.is_dirty());
    }
}

mod update {
    use super::*;

    #[test]
    fn a_title_and_open_state_change_and_nothing_else_does() {
        let mut doc = doc();
        let appendix = top(&doc)[2].clone();
        let before = item(&doc, &appendix);
        let mut op = UpdateOutlineItem::new(appendix.clone());
        op.title = Some("Appendices".into());
        op.open = Some(true);
        apply(&mut doc, op);
        let after = item(&doc, &appendix);
        assert_eq!(
            doc.outline().unwrap().by_id(&appendix).unwrap().title().unwrap(),
            "Appendices"
        );
        assert_eq!(after.get("Count"), Some(&Object::Integer(1)));
        assert_eq!(after.get("A"), before.get("A"), "the action is kept");
        whole(&doc);
    }

    #[test]
    fn a_new_target_replaces_an_action() {
        let mut doc = doc();
        let appendix = top(&doc)[2].clone();
        let mut op = UpdateOutlineItem::new(appendix.clone());
        op.target = Some(fit(&doc, 0));
        apply(&mut doc, op);
        let dict = item(&doc, &appendix);
        assert!(dict.get("A").is_none());
        assert!(dict.get("Dest").is_some());
    }

    #[test]
    fn an_update_that_changes_nothing_leaves_no_step() {
        let mut doc = doc();
        let intro = top(&doc)[0].clone();
        let mut op = UpdateOutlineItem::new(intro);
        op.title = Some("Introduction".into());
        assert!(apply(&mut doc, op).undo.is_empty());
    }
}

mod move_and_delete {
    use super::*;

    #[test]
    fn an_item_moves_with_its_children_and_keeps_its_id() {
        let mut doc = doc();
        let [intro, chapter, _] = <[OutlineItemId; 3]>::try_from(top(&doc)).unwrap();
        apply(
            &mut doc,
            MoveOutlineItem::new(chapter.clone(), Some(intro), SiblingPosition::First),
        );
        let outline = doc.outline().unwrap();
        let moved = outline.by_id(&chapter).unwrap();
        assert_eq!(moved.depth(), 2);
        assert_eq!(moved.children().count(), 2);
        assert_eq!(top(&doc).len(), 2);
        whole(&doc);
    }

    #[test]
    fn a_move_into_its_own_branch_is_refused() {
        let mut doc = doc();
        let chapter = top(&doc)[1].clone();
        let child = doc
            .outline()
            .unwrap()
            .by_id(&chapter)
            .unwrap()
            .children()
            .next()
            .unwrap()
            .id()
            .cloned()
            .unwrap();
        let op = MoveOutlineItem::new(chapter, Some(child), SiblingPosition::Last);
        assert_eq!(refused(&mut doc, op), "outline-move-into-itself");
    }

    #[test]
    fn a_deleted_item_takes_its_children_or_leaves_them_in_its_place() {
        for (children, left) in [(ChildItems::Delete, 3), (ChildItems::Promote, 5)] {
            let mut doc = doc();
            let chapter = top(&doc)[1].clone();
            apply(&mut doc, DeleteOutlineItem::new(chapter, children));
            assert_eq!(doc.outline().unwrap().len(), left, "{children:?}");
            whole(&doc);
        }
    }
}

mod names {
    use super::*;

    fn tree_has(doc: &Document, name: &str) -> bool {
        doc.name_tree("Dests")
            .unwrap()
            .is_some_and(|t| t.get(name.as_bytes()).unwrap().is_some())
    }

    fn dests_has(doc: &Document, name: &str) -> bool {
        doc.catalog().unwrap().get("Dests").and_then(|d| match d {
            Object::Ref(r) => doc.resolve(*r).unwrap().as_dict().map(|d| d.get(name).is_some()),
            Object::Dict(d) => Some(d.get(name).is_some()),
            _ => None,
        }) == Some(true)
    }

    fn dest(doc: &Document, index: usize) -> ExplicitDestination {
        ExplicitDestination::new(page(doc, index), DestinationView::FitH { top: Some(700.0) })
    }

    #[test]
    fn a_new_name_goes_into_the_tree_and_an_existing_one_stays_where_it_is() {
        let mut doc = doc();
        let d = dest(&doc, 2);
        apply(&mut doc, SetNamedDestination::new("results", d.clone()));
        assert!(tree_has(&doc, "results"));
        apply(&mut doc, SetNamedDestination::new("chapter2", d));
        assert!(dests_has(&doc, "chapter2"));
        assert!(!tree_has(&doc, "chapter2"));
        let outline = doc.outline().unwrap();
        assert_eq!(
            outline.named_destination(b"results").unwrap().unwrap().page_index,
            Some(2)
        );
    }

    #[test]
    fn a_rename_follows_its_uses_in_their_own_form() {
        let mut doc = doc();
        let [_, chapter, appendix] = <[OutlineItemId; 3]>::try_from(top(&doc)).unwrap();
        apply(&mut doc, RenameNamedDestination::new("chapter2", "methods"));
        apply(&mut doc, RenameNamedDestination::new("appendix", "annex"));
        assert!(dests_has(&doc, "methods") && !dests_has(&doc, "chapter2"));
        assert!(tree_has(&doc, "annex") && !tree_has(&doc, "appendix"));
        assert_eq!(item(&doc, &chapter).get("Dest"), Some(&Object::name("methods")));
        let a = item(&doc, &appendix);
        let d = a.get("A").unwrap().as_dict().unwrap().get("D").unwrap().clone();
        assert!(matches!(d, Object::String(s) if s.bytes == b"annex"));
        let outline = doc.outline().unwrap();
        assert!(
            outline
                .by_id(&chapter)
                .unwrap()
                .target()
                .unwrap()
                .unwrap()
                .page_index
                .is_some()
        );
    }

    #[test]
    fn a_rename_that_keeps_its_links_leaves_them_alone() {
        let mut doc = doc();
        let chapter = top(&doc)[1].clone();
        let mut op = RenameNamedDestination::new("chapter2", "methods");
        op.links = RenameLinks::Keep;
        apply(&mut doc, op);
        assert_eq!(item(&doc, &chapter).get("Dest"), Some(&Object::name("chapter2")));
    }

    #[test]
    fn a_rename_to_a_taken_or_unknown_name_is_refused() {
        let mut doc = doc();
        assert_eq!(
            refused(&mut doc, RenameNamedDestination::new("chapter2", "appendix")),
            "named-destination-exists"
        );
        assert_eq!(
            refused(&mut doc, RenameNamedDestination::new("nope", "x")),
            "unknown-named-destination"
        );
        assert_eq!(
            refused(&mut doc, RenameNamedDestination::new("chapter2", "")),
            "empty-destination-name"
        );
    }

    #[test]
    fn a_deleted_name_reports_what_still_names_it() {
        let mut doc = doc();
        let appendix = top(&doc)[2].clone();
        let applied = apply(&mut doc, DeleteNamedDestination::new("appendix"));
        assert!(!tree_has(&doc, "appendix"));
        assert_eq!(
            applied.destination_uses,
            vec![DestinationUse::OutlineItem { item: appendix }]
        );
        assert_eq!(
            refused(&mut doc, DeleteNamedDestination::new("appendix")),
            "unknown-named-destination"
        );
    }
}

#[test]
fn every_op_is_a_document_change_saved_incrementally() {
    let ops: Vec<Op> = vec![
        AddOutlineItem::new(None, SiblingPosition::Last, "x", OutlineTarget::Named("a".into())).into(),
        UpdateOutlineItem::new("o3.0".parse().unwrap()).into(),
        MoveOutlineItem::new("o3.0".parse().unwrap(), None, SiblingPosition::First).into(),
        DeleteOutlineItem::new("o3.0".parse().unwrap(), ChildItems::Delete).into(),
        DeleteNamedDestination::new("a").into(),
        RenameNamedDestination::new("a", "b").into(),
    ];
    for op in ops {
        assert_eq!(op.class(), laminapdf_ops::ModificationClass::Document, "{op:?}");
        assert!(!op.requires_rewrite(), "{op:?}");
        assert!(op.page_changes().is_empty(), "{op:?}");
    }
}

#[test]
fn every_op_undone_leaves_the_overlay_empty() {
    let base = doc();
    let [intro, chapter, appendix] = <[OutlineItemId; 3]>::try_from(top(&base)).unwrap();
    let d = ExplicitDestination::new(
        page(&base, 1),
        DestinationView::Xyz {
            left: Some(0.0),
            top: Some(792.0),
            zoom: None,
        },
    );
    let mut update = UpdateOutlineItem::new(intro.clone());
    update.title = Some("Start".into());
    update.style = Some(OutlineStyle::new(None, true, false));
    let ops: Vec<Op> = vec![
        AddOutlineItem::new(
            Some(chapter.clone()),
            SiblingPosition::After(chapter.clone()),
            "x",
            fit(&base, 0),
        )
        .into(),
        AddOutlineItem::new(None, SiblingPosition::Before(appendix.clone()), "x", fit(&base, 0)).into(),
        update.into(),
        MoveOutlineItem::new(appendix, Some(intro), SiblingPosition::Last).into(),
        DeleteOutlineItem::new(chapter, ChildItems::Promote).into(),
        SetNamedDestination::new("new", d.clone()).into(),
        SetNamedDestination::new("chapter2", d).into(),
        RenameNamedDestination::new("appendix", "annex").into(),
        DeleteNamedDestination::new("chapter2").into(),
    ];
    for op in ops {
        let mut doc = doc();
        if let Err(e) = doc.apply_all(Origin::Local, [op.clone()]) {
            // The first add names a sibling outside the parent's children.
            assert_eq!(e.code(), "outline-not-a-sibling", "{op:?}");
            continue;
        }
        assert!(doc.is_dirty(), "{op:?}");
        doc.undo().unwrap();
        assert!(!doc.is_dirty(), "{op:?}");
    }
}

mod rename_follows_every_go_to {
    use common::classic_with_root;
    use laminapdf_model::AnnotId;

    use super::*;

    /// One page (object 3) with a link whose `GoTo` names `(x)` and runs a second `GoTo` naming it
    /// through `/Next`, a widget whose `/AA` cursor-up action names it, and a catalog `/AA`
    /// whose will-close action names it; `x` lives in the `/Names` tree.
    fn file() -> Document {
        common::open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Names << /Dests << /Names [(x) [3 0 R /Fit]] >> >> /AA << /WC << /S /GoTo /D (x) >> >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R 5 0 R] >>",
                "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /A << /S /GoTo /D (x) /Next 6 0 R >> >>",
                "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /AA << /U << /S /GoTo /D (x) >> >> >>",
                "<< /S /GoTo /D (x) >>",
            ],
            "",
        ))
    }

    fn d(doc: &Document, num: u32, path: &[&str]) -> Object {
        let mut at = (*doc.resolve(laminapdf_cos::ObjRef::new(num, 0)).unwrap()).clone();
        for key in path {
            at = at.as_dict().unwrap().get(key).unwrap().clone();
        }
        at
    }

    fn is_y(o: &Object) -> bool {
        matches!(o, Object::String(s) if s.bytes == b"y")
    }

    #[test]
    fn links_next_chains_widgets_and_the_catalog_follow() {
        let mut doc = file();
        apply(&mut doc, RenameNamedDestination::new("x", "y"));
        assert!(is_y(&d(&doc, 4, &["A", "D"])));
        assert!(is_y(&d(&doc, 6, &["D"])));
        assert!(is_y(&d(&doc, 5, &["AA", "U", "D"])));
        assert!(is_y(&d(&doc, 1, &["AA", "WC", "D"])));
    }

    #[test]
    fn a_delete_reports_each_holder_once_per_use() {
        let mut doc = file();
        let applied = apply(&mut doc, DeleteNamedDestination::new("x"));
        let annots: Vec<AnnotId> = applied
            .destination_uses
            .iter()
            .filter_map(|u| match u {
                DestinationUse::Annotation { annot, .. } => Some(annot.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            annots.len(),
            3,
            "the link, its /Next action, the widget: {:?}",
            applied.destination_uses
        );
        assert!(applied.destination_uses.contains(&DestinationUse::Catalog));
    }
}

#[test]
fn a_name_in_both_stores_is_set_where_readers_find_it() {
    let mut doc = common::open(&common::classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R /Names << /Dests << /Names [(x) [3 0 R /Fit]] >> >> /Dests << /x [3 0 R /FitH 10] >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ],
        "",
    ));
    let page = page(&doc, 0);
    apply(
        &mut doc,
        SetNamedDestination::new(
            "x",
            ExplicitDestination::new(page, DestinationView::FitH { top: Some(50.0) }),
        ),
    );
    let tree = doc.name_tree("Dests").unwrap().unwrap().get(b"x").unwrap().unwrap();
    assert_eq!(tree.as_array().unwrap().get(1), Some(&Object::name("FitH")));
}
