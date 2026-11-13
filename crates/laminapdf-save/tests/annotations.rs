//! The annotation ops (`UpdateAnnot`, `DeleteAnnot`) through a save: each mode writes a file
//! that reopens with the change and nothing else, and that qpdf accepts when it is installed.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use laminapdf_cos::OpenOptions;
use laminapdf_model::AnnotId;
use laminapdf_ops::{DeleteAnnot, Document, Offset, Op, Origin, UpdateAnnot};
use laminapdf_save::{Save as _, SaveMode, SaveOptions};

fn id(text: &str) -> AnnotId {
    text.parse().unwrap()
}

/// An annotation as the test compares it: subtype, contents and rectangle.
type Seen = (Vec<u8>, Option<String>, Option<[f64; 4]>);

/// Each page's annotations, after reopening `bytes`.
fn read_back(bytes: Vec<u8>) -> Vec<Vec<Seen>> {
    let doc = laminapdf_model::Document::open(bytes, OpenOptions::default()).unwrap();
    let pages = doc.pages().unwrap();
    pages
        .iter()
        .map(|page| {
            doc.annotations(&page)
                .unwrap()
                .into_iter()
                .map(|a| {
                    (
                        a.subtype.name().to_vec(),
                        a.contents,
                        a.rect.map(|r| [r.x0, r.y0, r.x1, r.y1]),
                    )
                })
                .collect()
        })
        .collect()
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-annots-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.pdf");
    std::fs::write(&path, bytes).unwrap();
    match std::process::Command::new("qpdf").arg("--check").arg(&path).output() {
        Ok(out) => out.status.success(),
        Err(_) => true,
    }
}

#[test]
fn an_edit_and_a_delete_save_in_both_modes_and_reopen() {
    let bytes = fixture("annotations/threads.pdf");
    let mut update = UpdateAnnot::new(id("a16.0"));
    update.contents = Some(Some("best".to_owned()));
    update.translate = Offset::new(10.0, 0.0);
    let ops: [Op; 2] = [update.into(), DeleteAnnot::new(id("a10.0")).into()];
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
        doc.apply_all(Origin::Local, ops.clone()).unwrap();
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
        assert!(qpdf_accepts(&out), "{mode:?}");
        let pages = read_back(out);
        let kinds: Vec<Vec<Vec<u8>>> = pages
            .iter()
            .map(|p| p.iter().map(|(s, _, _)| s.clone()).collect())
            .collect();
        let strike = pages[0].iter().find(|(s, _, _)| s == b"StrikeOut").unwrap();
        assert_eq!(
            (kinds, strike.1.as_deref(), strike.2),
            (
                vec![
                    vec![
                        b"Text".to_vec(),
                        b"Text".to_vec(),
                        b"StrikeOut".to_vec(),
                        b"Caret".to_vec()
                    ],
                    vec![],
                ],
                Some("best"),
                Some([30.0, 100.0, 130.0, 115.0])
            ),
            "{mode:?}"
        );
    }
}
