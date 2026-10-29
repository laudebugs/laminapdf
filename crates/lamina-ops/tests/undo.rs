//! Undo and redo (ADR 0006 §4) and the two dirty levels (§7).
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{author, corpus, inherited_rotate, page_ids, rotate, rotations, set_title, snapshot, title};
use lamina_model::Rotation;
use lamina_ops::{Error, InfoPatch, Origin, PeerId, SetInfo, SkipReason, Target, UndoOutcome};

fn remote() -> Origin {
    Origin::Remote(PeerId::new("peer-1"))
}

fn set_title_and_author(doc: &mut lamina_ops::Document, title: &str, author: &str) {
    doc.apply_all(
        Origin::Local,
        [SetInfo::new(InfoPatch::new().title(Some(title)).author(Some(author)), None).into()],
    )
    .unwrap();
}

mod undo {
    use super::*;

    #[test]
    fn of_set_info_on_an_indirect_info_leaves_the_overlay_empty() {
        let mut doc = corpus("outline.pdf");
        set_title_and_author(&mut doc, "A", "B");
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn of_set_info_that_created_the_info_leaves_the_overlay_empty() {
        let mut doc = corpus("hello.pdf");
        set_title(&mut doc, Origin::Local, Some("Hello"));
        doc.undo().unwrap();
        assert!(!doc.is_dirty() && doc.trailer().get("Info").is_none());
    }

    #[test]
    fn of_set_info_on_a_direct_info_leaves_the_overlay_empty() {
        let mut doc = corpus("text-example.pdf");
        set_title(&mut doc, Origin::Local, Some("Changed"));
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn of_a_removed_key_restores_the_key_order() {
        let mut doc = inherited_rotate();
        doc.apply_all(
            Origin::Local,
            [SetInfo::new(InfoPatch::new().author(None), None).into()],
        )
        .unwrap();
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn of_rotate_page_leaves_the_overlay_empty() {
        let mut doc = inherited_rotate();
        let page = page_ids(&doc).first().unwrap().clone();
        rotate(&mut doc, &page, Rotation::None);
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn reports_applied_and_the_touched_pages() {
        let mut doc = inherited_rotate();
        let page = page_ids(&doc).first().unwrap().clone();
        rotate(&mut doc, &page, Rotation::None);
        let reverted = doc.undo().unwrap().unwrap();
        assert_eq!(
            (reverted.outcome, reverted.touched_pages.into_iter().collect::<Vec<_>>()),
            (UndoOutcome::Applied, vec![page])
        );
    }

    #[test]
    fn with_nothing_to_undo_is_none() {
        let mut doc = corpus("hello.pdf");
        assert_eq!(doc.undo().unwrap(), None);
    }

    #[test]
    fn goes_back_one_transaction_at_a_time() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        set_title(&mut doc, Origin::Local, Some("Two"));
        doc.undo().unwrap();
        assert_eq!(title(&doc).as_deref(), Some("One"));
    }

    #[test]
    fn after_a_remote_change_to_the_same_property_is_partial_and_keeps_the_remote_value() {
        let mut doc = corpus("outline.pdf");
        set_title_and_author(&mut doc, "Local title", "Local author");
        set_title(&mut doc, remote(), Some("Remote title"));
        let outcome = doc.undo().unwrap().unwrap().outcome;
        let UndoOutcome::Partial { skipped } = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(
            (
                skipped.iter().map(|s| (s.key.clone(), s.reason)).collect::<Vec<_>>(),
                title(&doc).as_deref(),
                author(&doc).as_deref()
            ),
            (
                vec![(Some("Title".into()), SkipReason::Changed)],
                Some("Remote title"),
                Some("lamina")
            )
        );
    }

    #[test]
    fn when_every_property_changed_remotely_is_skipped() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("Local"));
        set_title(&mut doc, remote(), Some("Remote"));
        let outcome = doc.undo().unwrap().unwrap().outcome;
        assert!(matches!(outcome, UndoOutcome::Skipped { .. }) && title(&doc).as_deref() == Some("Remote"));
    }

    #[test]
    fn of_a_page_rotated_remotely_since_is_skipped() {
        let mut doc = inherited_rotate();
        let page = page_ids(&doc).first().unwrap().clone();
        rotate(&mut doc, &page, Rotation::Half);
        doc.apply_all(
            remote(),
            [lamina_ops::RotatePage::new(page, Rotation::Clockwise270).into()],
        )
        .unwrap();
        let outcome = doc.undo().unwrap().unwrap().outcome;
        assert!(
            matches!(&outcome, UndoOutcome::Skipped { skipped } if skipped.iter().all(|s| matches!(s.target, Target::Object(_))))
                && rotations(&doc).first() == Some(&Rotation::Clockwise270)
        );
    }

    #[test]
    fn while_a_transaction_is_open_is_nested_transaction() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        doc.begin(Origin::Local).unwrap();
        assert_eq!(
            (doc.undo().map(drop), doc.redo().map(drop)),
            (Err(Error::NestedTransaction), Err(Error::NestedTransaction))
        );
    }

    #[test]
    fn keeps_at_most_the_configured_depth() {
        let mut doc = corpus("outline.pdf");
        doc.set_undo_depth(2);
        for t in ["1", "2", "3"] {
            set_title(&mut doc, Origin::Local, Some(t));
        }
        assert_eq!(doc.history().undo_len(), 2);
    }

    #[test]
    fn defaults_to_a_depth_of_500() {
        assert_eq!(corpus("hello.pdf").history().depth(), 500);
    }
}

mod redo {
    use super::*;

    #[test]
    fn restores_the_same_overlay_as_before_undo() {
        let mut doc = corpus("hello.pdf");
        set_title(&mut doc, Origin::Local, Some("Hello"));
        let before = snapshot(&doc);
        doc.undo().unwrap();
        doc.redo().unwrap();
        assert_eq!(snapshot(&doc), before);
    }

    #[test]
    fn recreates_an_undone_creation_with_the_same_number() {
        let mut doc = corpus("hello.pdf");
        set_title(&mut doc, Origin::Local, Some("Hello"));
        let info = doc.trailer().get("Info").cloned();
        doc.undo().unwrap();
        doc.redo().unwrap();
        assert_eq!(doc.trailer().get("Info").cloned(), info);
    }

    #[test]
    fn is_cleared_by_a_new_local_transaction() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        doc.undo().unwrap();
        set_title(&mut doc, Origin::Local, Some("Two"));
        assert_eq!(doc.redo().unwrap(), None);
    }

    #[test]
    fn is_kept_across_a_remote_transaction() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        doc.undo().unwrap();
        doc.apply_all(
            remote(),
            [SetInfo::new(InfoPatch::new().subject(Some("S")), None).into()],
        )
        .unwrap();
        assert!(doc.history().can_redo());
    }

    #[test]
    fn with_nothing_to_redo_is_none() {
        let mut doc = corpus("hello.pdf");
        assert_eq!(doc.redo().unwrap(), None);
    }
}

mod coalescing {
    use super::*;

    fn type_title(doc: &mut lamina_ops::Document, key: &str, title: &str) {
        doc.transact_coalescing(key, |tx| {
            tx.apply(SetInfo::new(InfoPatch::new().title(Some(title)), None))?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn merges_transactions_with_the_same_key() {
        let mut doc = corpus("outline.pdf");
        for t in ["H", "He", "Hel"] {
            type_title(&mut doc, "title", t);
        }
        assert_eq!(doc.history().undo_len(), 1);
    }

    #[test]
    fn undo_of_a_merged_entry_restores_the_state_before_the_first() {
        let mut doc = corpus("outline.pdf");
        for t in ["H", "He", "Hel"] {
            type_title(&mut doc, "title", t);
        }
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn keeps_transactions_with_different_keys_apart() {
        let mut doc = corpus("outline.pdf");
        type_title(&mut doc, "title", "A");
        type_title(&mut doc, "other", "B");
        assert_eq!(doc.history().undo_len(), 2);
    }

    #[test]
    fn does_not_merge_into_an_entry_before_an_undo() {
        let mut doc = corpus("outline.pdf");
        type_title(&mut doc, "title", "A");
        type_title(&mut doc, "other", "B");
        doc.undo().unwrap();
        type_title(&mut doc, "title", "C");
        assert_eq!(doc.history().undo_len(), 2);
    }
}

mod dirty {
    use super::*;

    #[test]
    fn a_local_transaction_makes_both_levels_dirty() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        assert!(doc.is_dirty() && doc.has_unsaved_changes());
    }

    #[test]
    fn undo_to_the_opened_state_clears_both() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        doc.undo().unwrap();
        assert!(!doc.is_dirty() && !doc.has_unsaved_changes());
    }

    #[test]
    fn mark_saved_clears_the_semantic_level_only() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        doc.mark_saved();
        assert!(doc.is_dirty() && !doc.has_unsaved_changes());
    }

    #[test]
    fn undo_past_the_saved_point_is_unsaved_and_redo_back_to_it_is_not() {
        let mut doc = corpus("outline.pdf");
        set_title(&mut doc, Origin::Local, Some("One"));
        doc.mark_saved();
        doc.undo().unwrap();
        let after_undo = doc.has_unsaved_changes();
        doc.redo().unwrap();
        assert_eq!((after_undo, doc.has_unsaved_changes()), (true, false));
    }
}
