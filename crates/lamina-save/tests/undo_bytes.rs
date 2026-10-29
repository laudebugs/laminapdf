//! Undo restores byte-identical output (todo B3c): undoing a change after a save leaves nothing
//! new to write, and the same state written twice gives the same bytes.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::save::{corpus_doc, rotate, set_title};
use lamina_model::Rotation;
use lamina_save::{Save, SaveMode, SaveOptions};

const FILES: [&str; 4] = ["hello.pdf", "hello-xrefstm.pdf", "outline.pdf", "pdflib.pdf"];

#[test]
fn an_undone_op_after_an_incremental_save_writes_the_saved_bytes() {
    for name in FILES {
        let mut doc = corpus_doc(name);
        set_title(&mut doc, "Saved");
        let mut a = Vec::new();
        doc.save(&mut a, &SaveOptions::new().with_mode(SaveMode::Incremental))
            .unwrap();
        rotate(&mut doc, 0, Rotation::Half);
        doc.undo().unwrap();
        assert!(!doc.is_dirty(), "{name}");
        let mut b = Vec::new();
        doc.save(&mut b, &SaveOptions::new()).unwrap();
        assert_eq!(a, b, "{name}");
    }
}

#[test]
fn an_undone_op_before_a_full_rewrite_writes_the_same_bytes() {
    for name in FILES {
        let full = SaveOptions::new().with_mode(SaveMode::FullRewrite);
        let mut once = corpus_doc(name);
        set_title(&mut once, "Saved");
        let mut a = Vec::new();
        once.save(&mut a, &full).unwrap();

        let mut twice = corpus_doc(name);
        set_title(&mut twice, "Saved");
        rotate(&mut twice, 0, Rotation::Half);
        twice.undo().unwrap();
        let mut b = Vec::new();
        twice.save(&mut b, &full).unwrap();
        assert_eq!(a, b, "{name}");
    }
}

#[test]
fn undoing_every_op_writes_the_base_unchanged() {
    for name in FILES {
        let mut doc = corpus_doc(name);
        let base = doc.data().unwrap().to_vec();
        set_title(&mut doc, "Gone");
        rotate(&mut doc, 0, Rotation::Clockwise90);
        doc.undo().unwrap();
        doc.undo().unwrap();
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new()).unwrap();
        assert_eq!(out, base, "{name}");
    }
}
