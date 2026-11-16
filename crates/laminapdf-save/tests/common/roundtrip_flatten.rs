//! The flatten cases of the round-trip suite (todo E5e, D.14): on every plain fixture, a
//! highlight and a note with its Popup, then both flattened for printing, saved incrementally and
//! as a full rewrite. Lamina reopens each with no marks left, the same pages, and a page whose
//! content draws the two appearances; the tools compare the marks they read (`marks`: none).
#![allow(
    unreachable_pub,
    reason = "shared by the tests and the `roundtrip` example, as `tests/common` helpers are"
)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test support: a panic is a test failure"
)]

use std::path::Path;

use laminapdf_cos::OpenOptions;
use laminapdf_model::UserSpacePoint;
use laminapdf_ops::{
    AddAnnot, AnnotSpec, Document, FlattenAnnots, FlattenIntent, MarkupKind, Note, NoteIcon, Op, Origin, TextMarkup,
};
use laminapdf_save::{Save as _, SaveMode, SaveOptions};
use serde_json::json;

use super::roundtrip::{PLAIN_BUILT, PLAIN_CORPUS, Suite, facts, mark_facts};

pub fn flattened(root: &Path, suite: &mut Suite) {
    let names = PLAIN_CORPUS
        .iter()
        .map(|n| format!("corpus/{n}"))
        .chain(PLAIN_BUILT.iter().map(|n| format!("golden/{n}")));
    let p = UserSpacePoint::new;
    for name in names {
        let bytes = std::fs::read(root.join("fixtures").join(&name)).unwrap();
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let id = format!("flatten-{}-{mode:?}", name.replace('/', "_").trim_end_matches(".pdf")).to_lowercase();
            let mut doc = Document::open(bytes.clone(), OpenOptions::default())
                .unwrap()
                .with_appearances(laminapdf_build::AnnotationAppearances);
            let (pages, rotations, info, labels) = facts(&doc);
            let first = doc.pages().unwrap().get(0).unwrap().id().clone();
            let quads = vec![[p(36.0, 94.0), p(200.0, 94.0), p(36.0, 80.0), p(200.0, 80.0)]];
            let highlight = AddAnnot::new(
                first.clone(),
                AnnotSpec::TextMarkup(TextMarkup::new(MarkupKind::Highlight, quads)),
            );
            let mut note = AddAnnot::new(first, AnnotSpec::Note(Note::new(p(220.0, 94.0), NoteIcon::Comment)));
            note.common.contents = Some("Flattened".into());
            let made = doc
                .apply_all(Origin::Local, [Op::from(highlight), note.into()])
                .unwrap()
                .created_annotations;
            let expected = made.len();
            doc.apply_all(Origin::Local, [FlattenAnnots::new(made, FlattenIntent::Print).into()])
                .unwrap();
            let marks = mark_facts(&doc);
            let mut output = Vec::new();
            let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
            suite.saved += 1;
            let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
            let page = reopened.pages().unwrap().get(0).unwrap().cos().clone();
            let content = reopened.cos().page_content(&page).unwrap();
            let draws = String::from_utf8_lossy(&content).matches(" Do").count();
            if mark_facts(&reopened) != marks
                || marks.iter().any(|page| !page.is_empty())
                || draws < expected
                || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels.clone())
                || reopened.was_repaired()
            {
                suite.failures.push(format!(
                    "{id}: lamina reopened marks {:?} with {draws} drawings (repaired {}), expected none and {expected}",
                    mark_facts(&reopened),
                    reopened.was_repaired()
                ));
            }
            let file = format!("{id}.pdf");
            suite.manifest.push(json!({
                "file": &file,
                "source": name,
                "mode": format!("{:?}", report.mode()),
                "pages": pages,
                "rotations": rotations,
                "title": info[0],
                "info": null,
                "encrypted": false,
                "direct_info": matches!(reopened.trailer().get("Info"), Some(laminapdf_cos::Object::Dict(_))),
                "invalidated": 0,
                "marks": marks,
            }));
            suite.outputs.push((file, output));
        }
    }
}
