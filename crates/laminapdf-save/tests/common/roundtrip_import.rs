//! The import cases of the round-trip suite (todo E5d, D.13): on every plain fixture, the
//! comments of `fixtures/interchange/review.xfdf` imported as one transaction, saved
//! incrementally and as a full rewrite. Lamina reopens each with the marks it had before saving,
//! the same pages, and no repair; the tools compare the marks they read (`marks`).
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
use laminapdf_interchange::{ImportOptions, import, read};
use laminapdf_ops::Document;
use laminapdf_save::{Save as _, SaveMode, SaveOptions};
use serde_json::json;

use super::roundtrip::{PLAIN_BUILT, PLAIN_CORPUS, Suite, facts, mark_facts};

pub fn imported(root: &Path, suite: &mut Suite) {
    let xfdf = std::fs::read(root.join("fixtures/interchange/review.xfdf")).unwrap();
    let exchange = read(&xfdf, &laminapdf_syntax::Limits::default()).unwrap();
    let names = PLAIN_CORPUS
        .iter()
        .map(|n| format!("corpus/{n}"))
        .chain(PLAIN_BUILT.iter().map(|n| format!("golden/{n}")));
    for name in names {
        let bytes = std::fs::read(root.join("fixtures").join(&name)).unwrap();
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let id = format!("import-{}-{mode:?}", name.replace('/', "_").trim_end_matches(".pdf")).to_lowercase();
            let mut doc = Document::open(bytes.clone(), OpenOptions::default())
                .unwrap()
                .with_appearances(laminapdf_build::AnnotationAppearances);
            let (pages, rotations, info, labels) = facts(&doc);
            let (summary, _) = import(&mut doc, &exchange, &ImportOptions::default()).unwrap();
            let marks = mark_facts(&doc);
            let mut output = Vec::new();
            let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
            suite.saved += 1;
            let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
            if summary.added == 0
                || mark_facts(&reopened) != marks
                || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels.clone())
                || reopened.was_repaired()
            {
                suite.failures.push(format!(
                    "{id}: {} added; lamina reopened marks {:?} (repaired {}), expected {marks:?}",
                    summary.added,
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
