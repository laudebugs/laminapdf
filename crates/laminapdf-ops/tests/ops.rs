//! The proving ops, `SetInfo` and `RotatePage`, and transactions (ADR 0006 §1–§3).
#![allow(
    deprecated,
    reason = "SetInfo, deprecated, is used here on purpose: v1 op logs still name it"
)]
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{classic_with_root, corpus, inherited_rotate, open, page_ids, rotations, snapshot, title};
use laminapdf_cos::{Object, OverlayEntry};
use laminapdf_model::{PageId, PdfDate, Rotation};
use laminapdf_ops::{Error, Field, InfoPatch, ModificationClass, Op, Origin, RotatePage, SetInfo};

fn set_info(patch: InfoPatch) -> Op {
    SetInfo::patch(patch).into()
}

fn info_dict(doc: &laminapdf_ops::Document) -> laminapdf_cos::Dict {
    let info = doc.trailer().get("Info").unwrap();
    doc.deref(info).unwrap().as_dict().unwrap().clone()
}

mod set_info {
    use super::*;

    #[test]
    fn edits_an_indirect_info_in_place() {
        let mut doc = corpus("outline.pdf");
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().title(Some("New")))])
            .unwrap();
        assert_eq!(
            (
                title(&doc).as_deref(),
                doc.overlay().trailer().is_none(),
                doc.overlay().len()
            ),
            (Some("New"), true, 1)
        );
    }

    #[test]
    fn keeps_the_position_of_an_existing_key() {
        let mut doc = corpus("outline.pdf");
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().title(Some("New")))])
            .unwrap();
        let first = info_dict(&doc).iter().next().map(|(k, _)| k.clone());
        assert_eq!(first, Some("Title".into()));
    }

    #[test]
    fn clears_a_field_given_null() {
        let mut doc = corpus("outline.pdf");
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().author(None))])
            .unwrap();
        assert!(!info_dict(&doc).contains_key("Author"));
    }

    #[test]
    fn creates_an_indirect_info_when_there_is_none() {
        let mut doc = corpus("hello.pdf");
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().title(Some("Hello")))])
            .unwrap();
        let r = doc.trailer().get("Info").and_then(Object::as_ref).unwrap();
        assert_eq!(
            (
                title(&doc).as_deref(),
                matches!(doc.overlay().get(r.num), Some(OverlayEntry::New(_)))
            ),
            (Some("Hello"), true)
        );
    }

    #[test]
    fn promotes_a_direct_info_to_an_indirect_object_keeping_its_entries() {
        let mut doc = corpus("text-example.pdf");
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().author(Some("Me")))])
            .unwrap();
        let info = doc.trailer().get("Info").unwrap();
        assert_eq!(
            (info.as_ref().is_some(), title(&doc).as_deref()),
            (true, Some("lamina text example"))
        );
    }

    #[test]
    fn writes_mod_date_from_the_payload() {
        let mut doc = corpus("outline.pdf");
        let at: PdfDate = "D:20261029120000Z".parse().unwrap();
        doc.apply_all(Origin::Local, [SetInfo::new(InfoPatch::new(), Some(at)).into()])
            .unwrap();
        let mod_date = doc.info().unwrap().unwrap().mod_date().unwrap().unwrap();
        assert_eq!(mod_date.parsed, Some(at));
    }

    #[test]
    fn encodes_text_without_a_pdfdoc_code_as_utf16() {
        let mut doc = corpus("outline.pdf");
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().title(Some("世界")))])
            .unwrap();
        let bytes = match info_dict(&doc).get("Title") {
            Some(Object::String(s)) => s.bytes.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            (bytes.get(..2), title(&doc).as_deref()),
            (Some(&b"\xFE\xFF"[..]), Some("世界"))
        );
    }

    #[test]
    fn that_changes_nothing_leaves_the_document_clean() {
        let mut doc = corpus("outline.pdf");
        doc.apply_all(
            Origin::Local,
            [set_info(InfoPatch::new().title(Some("Outline fixture")).keywords(None))],
        )
        .unwrap();
        assert!(!doc.is_dirty() && !doc.has_unsaved_changes());
    }

    #[test]
    fn clearing_on_a_document_without_info_creates_nothing() {
        let mut doc = corpus("hello.pdf");
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().title(None))])
            .unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn is_metadata_and_incremental() {
        let op = set_info(InfoPatch::new());
        assert_eq!(
            (op.class(), op.requires_rewrite()),
            (ModificationClass::Metadata, false)
        );
    }
}

mod rotate_page {
    use super::*;

    #[test]
    fn keeps_the_page_id_and_object_number() {
        let mut doc = corpus("hello.pdf");
        let before = page_ids(&doc);
        let page = before.first().unwrap().clone();
        doc.apply_all(Origin::Local, [RotatePage::new(page, Rotation::Clockwise90).into()])
            .unwrap();
        assert_eq!(
            (page_ids(&doc), doc.overlay().iter().map(|(n, _)| n).collect::<Vec<_>>()),
            (before, vec![3])
        );
    }

    #[test]
    fn materialises_an_inherited_value_on_the_page_only() {
        let mut doc = inherited_rotate();
        let first = page_ids(&doc).first().unwrap().clone();
        doc.apply_all(Origin::Local, [RotatePage::new(first, Rotation::None).into()])
            .unwrap();
        assert_eq!(
            (
                rotations(&doc),
                doc.overlay().iter().map(|(n, _)| n).collect::<Vec<_>>()
            ),
            (vec![Rotation::None, Rotation::Clockwise90, Rotation::Half], vec![4])
        );
    }

    #[test]
    fn writes_nothing_when_the_page_already_displays_so() {
        let mut doc = inherited_rotate();
        let first = page_ids(&doc).first().unwrap().clone();
        let ((), applied) = doc
            .transact(Origin::Local, |tx| {
                tx.apply(RotatePage::new(first, Rotation::Clockwise90))?;
                Ok(())
            })
            .unwrap();
        assert!(!doc.is_dirty() && applied.touched_pages.is_empty());
    }

    /// Two pages inheriting `/Rotate 90.0` (an integral real): the first one's `/Parent`
    /// leads to the root, the second has none, so it is found by the page-tree walk.
    fn real_rotate() -> laminapdf_ops::Document {
        open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /Rotate 90.0 /MediaBox [0 0 100 100] >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /Type /Page >>",
            ],
            "",
        ))
    }

    #[test]
    fn reads_an_integral_real_rotate_through_the_parents() {
        let mut doc = real_rotate();
        let first = page_ids(&doc).first().unwrap().clone();
        let ((), applied) = doc
            .transact(Origin::Local, |tx| {
                tx.apply(RotatePage::new(first, Rotation::Clockwise90))?;
                Ok(())
            })
            .unwrap();
        assert!(!doc.is_dirty() && applied.touched_pages.is_empty());
    }

    #[test]
    fn reads_an_integral_real_rotate_through_the_page_tree_walk() {
        let mut doc = real_rotate();
        let second = page_ids(&doc).get(1).unwrap().clone();
        let ((), applied) = doc
            .transact(Origin::Local, |tx| {
                tx.apply(RotatePage::new(second, Rotation::Clockwise90))?;
                Ok(())
            })
            .unwrap();
        assert!(!doc.is_dirty() && applied.touched_pages.is_empty());
    }

    #[test]
    fn writes_nothing_when_an_own_integral_real_already_states_the_rotation() {
        let mut doc = open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 100 100] >>",
                "<< /Type /Page /Parent 2 0 R /Rotate 90.0 >>",
            ],
            "",
        ));
        let first = page_ids(&doc).first().unwrap().clone();
        let ((), applied) = doc
            .transact(Origin::Local, |tx| {
                tx.apply(RotatePage::new(first, Rotation::Clockwise90))?;
                Ok(())
            })
            .unwrap();
        assert!(!doc.is_dirty() && applied.touched_pages.is_empty());
    }

    #[test]
    fn writes_nothing_when_an_own_null_inherits_the_rotation() {
        let mut doc = open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /Rotate 90 /MediaBox [0 0 100 100] >>",
                "<< /Type /Page /Parent 2 0 R /Rotate null >>",
            ],
            "",
        ));
        let first = page_ids(&doc).first().unwrap().clone();
        let ((), applied) = doc
            .transact(Origin::Local, |tx| {
                tx.apply(RotatePage::new(first, Rotation::Clockwise90))?;
                Ok(())
            })
            .unwrap();
        assert!(!doc.is_dirty() && applied.touched_pages.is_empty());
    }

    #[test]
    fn replaces_an_own_value_in_place() {
        let mut doc = inherited_rotate();
        let third = page_ids(&doc).get(2).unwrap().clone();
        doc.apply_all(Origin::Local, [RotatePage::new(third, Rotation::Clockwise270).into()])
            .unwrap();
        let page = doc.resolve(laminapdf_cos::ObjRef::new(6, 0)).unwrap();
        let keys: Vec<_> = page.as_dict().unwrap().iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, ["Type".into(), "Parent".into(), "Rotate".into()]);
    }

    #[test]
    fn reports_the_page_as_touched() {
        let mut doc = corpus("hello.pdf");
        let page = page_ids(&doc).first().unwrap().clone();
        let applied = doc
            .apply_all(Origin::Local, [RotatePage::new(page.clone(), Rotation::Half).into()])
            .unwrap();
        assert_eq!(applied.touched_pages.into_iter().collect::<Vec<_>>(), [page]);
    }

    #[test]
    fn an_unknown_page_is_an_error() {
        let mut doc = corpus("hello.pdf");
        let page: PageId = "p99.0".parse().unwrap();
        let err = doc
            .apply_all(Origin::Local, [RotatePage::new(page.clone(), Rotation::Half).into()])
            .unwrap_err();
        assert_eq!(err, Error::UnknownPage(page));
    }

    #[test]
    fn a_direct_page_is_refused() {
        let mut doc = open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [<< /Type /Page >>] /Count 1 >>",
            ],
            "",
        ));
        let page = page_ids(&doc).first().unwrap().clone();
        let err = doc
            .apply_all(Origin::Local, [RotatePage::new(page.clone(), Rotation::Half).into()])
            .unwrap_err();
        assert_eq!(err, Error::DirectPage(page));
    }

    #[test]
    fn is_page_structure_and_incremental() {
        let op: Op = RotatePage::new("p3.0".parse().unwrap(), Rotation::None).into();
        assert_eq!(
            (op.class(), op.requires_rewrite()),
            (ModificationClass::PageStructure, false)
        );
    }
}

mod invalid_input_and_rollback {
    use super::*;

    #[test]
    fn rotate_page_fixes_an_invalid_rotate_in_strict_mode() {
        let bytes = classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /Rotate 45 >>",
            ],
            "",
        );
        let mut doc =
            laminapdf_ops::Document::open(bytes, laminapdf_cos::OpenOptions::default().with_strict(true)).unwrap();
        let page: PageId = "p3.0".parse().unwrap();
        doc.apply_all(Origin::Local, [RotatePage::new(page, Rotation::Clockwise90).into()])
            .unwrap();
        assert_eq!(rotations(&doc), [Rotation::Clockwise90]);
    }

    #[test]
    fn rotate_page_does_not_warn_about_the_value_it_replaces() {
        let mut doc = open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /Rotate 45 >>",
            ],
            "",
        ));
        let page: PageId = "p3.0".parse().unwrap();
        doc.apply_all(Origin::Local, [RotatePage::new(page, Rotation::Half).into()])
            .unwrap();
        assert_eq!(doc.warnings(), []);
    }

    #[test]
    fn rotate_page_refuses_an_object_that_is_not_a_page() {
        let mut doc = corpus("hello.pdf");
        let not_a_page: PageId = "p1.0".parse().unwrap();
        let err = doc
            .apply_all(
                Origin::Local,
                [RotatePage::new(not_a_page.clone(), Rotation::Half).into()],
            )
            .unwrap_err();
        assert_eq!(err, Error::UnknownPage(not_a_page));
    }

    #[test]
    fn rotate_page_refuses_a_page_object_outside_the_tree() {
        let mut doc = open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /Type /Page /Parent 2 0 R >>",
            ],
            "",
        ));
        let orphan: PageId = "p4.0".parse().unwrap();
        let err = doc
            .apply_all(Origin::Local, [RotatePage::new(orphan.clone(), Rotation::Half).into()])
            .unwrap_err();
        assert_eq!(err, Error::UnknownPage(orphan));
    }

    #[test]
    fn rotate_page_finds_a_page_whose_parent_entry_is_wrong() {
        let mut doc = open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 1 0 R >>",
            ],
            "",
        ));
        let page: PageId = "p3.0".parse().unwrap();
        doc.apply_all(Origin::Local, [RotatePage::new(page, Rotation::Half).into()])
            .unwrap();
        assert_eq!(rotations(&doc), [Rotation::Half]);
    }

    #[test]
    fn set_info_replaces_an_info_object_that_does_not_parse() {
        let mut doc = open(&classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [] /Count 0 >>",
                "<< /Title ( >>",
            ],
            "/Info 3 0 R",
        ));
        doc.apply_all(Origin::Local, [set_info(InfoPatch::new().title(Some("Fixed")))])
            .unwrap();
        assert_eq!(title(&doc).as_deref(), Some("Fixed"));
    }
}

mod transactions {
    use super::*;

    fn rotate_first(doc: &laminapdf_ops::Document, rotation: Rotation) -> Op {
        RotatePage::new(page_ids(doc).first().unwrap().clone(), rotation).into()
    }

    #[test]
    fn a_failing_transaction_leaves_the_overlay_as_before() {
        let mut doc = inherited_rotate();
        let op = rotate_first(&doc, Rotation::Half);
        doc.apply_all(Origin::Local, [op]).unwrap();
        let before = snapshot(&doc);
        let unknown: PageId = "p99.0".parse().unwrap();
        let result = doc.transact(Origin::Local, |tx| {
            tx.apply(SetInfo::patch(InfoPatch::new().title(Some("Changed"))))?;
            tx.apply(RotatePage::new(
                page_ids(tx.document()).get(1).unwrap().clone(),
                Rotation::None,
            ))?;
            tx.apply(RotatePage::new(unknown, Rotation::None))?;
            Ok(())
        });
        assert_eq!(
            (result.is_err(), snapshot(&doc), doc.history().undo_len()),
            (true, before, 1)
        );
    }

    #[test]
    fn an_error_returned_by_the_closure_rolls_back() {
        let mut doc = corpus("hello.pdf");
        let op = rotate_first(&doc, Rotation::Half);
        let result: laminapdf_ops::Result<((), _)> = doc.transact(Origin::Local, |tx| {
            tx.apply(op)?;
            Err(Error::NoTransaction)
        });
        assert_eq!((result.map(drop), doc.is_dirty()), (Err(Error::NoTransaction), false));
    }

    #[test]
    fn a_transaction_inside_an_open_one_is_nested_transaction() {
        let mut doc = corpus("hello.pdf");
        doc.begin(Origin::Local).unwrap();
        let err = doc.transact(Origin::Local, |_| Ok(())).unwrap_err();
        assert_eq!(
            (err, doc.begin(Origin::Local)),
            (Error::NestedTransaction, Err(Error::NestedTransaction))
        );
    }

    #[test]
    fn a_failing_savepoint_rolls_back_only_its_own_changes() {
        let mut doc = inherited_rotate();
        let op = rotate_first(&doc, Rotation::Half);
        let unknown: PageId = "p99.0".parse().unwrap();
        doc.transact(Origin::Local, |tx| {
            tx.apply(SetInfo::patch(InfoPatch::new().title(Some("Kept"))))?;
            let inner = tx.savepoint(|tx| {
                tx.apply(op)?;
                tx.apply(RotatePage::new(unknown, Rotation::None))
            });
            assert!(inner.is_err());
            Ok(())
        })
        .unwrap();
        assert_eq!(
            (title(&doc).as_deref(), rotations(&doc).first().copied()),
            (Some("Kept"), Some(Rotation::Clockwise90))
        );
    }

    #[test]
    fn a_successful_savepoint_keeps_its_changes() {
        let mut doc = inherited_rotate();
        let op = rotate_first(&doc, Rotation::Half);
        doc.transact(Origin::Local, |tx| tx.savepoint(|tx| tx.apply(op)))
            .unwrap();
        assert_eq!(rotations(&doc).first().copied(), Some(Rotation::Half));
    }

    #[test]
    fn a_caught_op_error_leaves_the_transaction_usable() {
        let mut doc = inherited_rotate();
        let op = rotate_first(&doc, Rotation::Half);
        let unknown: PageId = "p99.0".parse().unwrap();
        doc.transact(Origin::Local, |tx| {
            assert!(tx.apply(RotatePage::new(unknown, Rotation::None)).is_err());
            tx.apply(op)
        })
        .unwrap();
        assert_eq!(rotations(&doc).first().copied(), Some(Rotation::Half));
    }

    #[test]
    fn the_handle_form_commits_like_transact() {
        let mut doc = corpus("hello.pdf");
        let op = rotate_first(&doc, Rotation::Half);
        doc.begin(Origin::Local).unwrap();
        doc.apply(op).unwrap();
        let applied = doc.commit().unwrap();
        assert_eq!(
            (applied.undo.is_empty(), doc.history().undo_len(), doc.in_transaction()),
            (false, 1, false)
        );
    }

    #[test]
    fn the_handle_form_rolls_back() {
        let mut doc = corpus("hello.pdf");
        let op = rotate_first(&doc, Rotation::Half);
        doc.begin(Origin::Local).unwrap();
        doc.apply(op).unwrap();
        doc.rollback().unwrap();
        assert!(!doc.is_dirty() && !doc.history().can_undo());
    }

    #[test]
    fn apply_commit_and_rollback_need_an_open_transaction() {
        let mut doc = corpus("hello.pdf");
        let op = rotate_first(&doc, Rotation::Half);
        assert_eq!(
            (doc.apply(op).map(drop), doc.commit().map(drop), doc.rollback()),
            (
                Err(Error::NoTransaction),
                Err(Error::NoTransaction),
                Err(Error::NoTransaction)
            )
        );
    }

    #[test]
    fn the_undo_record_lists_each_changed_property() {
        let mut doc = corpus("outline.pdf");
        let applied = doc
            .apply_all(
                Origin::Local,
                [set_info(InfoPatch::new().title(Some("A")).author(None))],
            )
            .unwrap();
        let keys: Vec<_> = applied.undo.properties().map(|p| p.field).collect();
        assert_eq!(keys, [Field::Key("Title".into()), Field::Key("Author".into())]);
    }

    #[test]
    fn remote_and_system_transactions_are_not_undoable() {
        let mut doc = corpus("hello.pdf");
        let op = rotate_first(&doc, Rotation::Half);
        doc.apply_all(Origin::Remote(laminapdf_ops::PeerId::new("peer")), [op])
            .unwrap();
        common::set_title(&mut doc, Origin::System, Some("x"));
        assert_eq!(
            (doc.history().can_undo(), doc.has_unsaved_changes(), doc.is_dirty()),
            (false, false, true)
        );
    }

    #[test]
    fn edits_are_visible_to_the_cos_page_tree() {
        let mut doc = inherited_rotate();
        let op = rotate_first(&doc, Rotation::Clockwise270);
        doc.apply_all(Origin::Local, [op]).unwrap();
        let rotate = doc
            .cos()
            .pages()
            .unwrap()
            .first()
            .and_then(|p| p.inherited.rotate.clone());
        assert_eq!(rotate.as_deref(), Some(&Object::Integer(270)));
    }
}
