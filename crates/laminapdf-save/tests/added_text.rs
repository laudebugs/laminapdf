//! Saving text added to pages (E6 plan §2.2 and §5, todo E6c): `AddText`'s verdict on the signed
//! fixtures (class `Content`), text in five scripts saved, reopened and read back exactly by
//! Lamina PDF and, where it is installed, `pdftotext`, `qpdf --check` clean, undo and redo that
//! give back the same bytes, and a tagged file that stays tagged.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::process::Command;

use common::fixture;
use common::save::editable;
use laminapdf_build::AnnotationAppearances;
use laminapdf_model::{PdfDate, Pt, RectPt};
use laminapdf_ops::{
    AddText, AnnotFont, ContentId, DeleteContentObject, Document, Op, Origin, TextSpan, UpdateContentObject,
    added_objects,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

fn id(s: &str) -> ContentId {
    ContentId::new(s).unwrap()
}

fn date() -> PdfDate {
    "D:20261118120000Z".parse().unwrap()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

fn first_page(doc: &Document) -> laminapdf_model::PageId {
    doc.pages().unwrap().get(0).unwrap().id().clone()
}

fn save(doc: &mut Document, mode: SaveMode) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
    bytes
}

/// A line in each of five scripts, with the font that has it: Latin with a diacritic, Arabic,
/// Hebrew, Devanagari and Chinese.
const SCRIPTS: [(&str, &str, &str); 5] = [
    ("latin", "NotoSans-Regular.ttf", "Zoë reviewed the draft twice"),
    ("arabic", "NotoSansArabic-Regular.ttf", "مرحبا بالعالم"),
    ("hebrew", "NotoSansHebrew-Regular.ttf", "שלום עולם"),
    ("devanagari", "NotoSansDevanagari-Regular.ttf", "नमस्ते दुनिया"),
    ("chinese", "NotoSansSC-Regular.otf", "你好，世界"),
];

/// The five lines added to `doc`'s first page, a line under the one before.
fn five_scripts(doc: &mut Document) -> Vec<Op> {
    let page = first_page(doc);
    SCRIPTS
        .iter()
        .enumerate()
        .map(|(i, (name, font, text))| {
            let font = AnnotFont::Font(doc.add_blob(fixture(&format!("fonts/{font}"))));
            let top = 100.0 + 40.0 * f64::from(u8::try_from(i).unwrap());
            AddText::new(
                id(name),
                page.clone(),
                rect(72.0, top, 400.0, top + 32.0),
                vec![TextSpan::plain(*text)],
                date(),
            )
            .with_fonts(vec![font], 16.0)
            .into()
        })
        .collect()
}

fn hello() -> Document {
    editable(&fixture("golden/examples/hello.pdf")).with_appearances(AnnotationAppearances)
}

/// The text Lamina PDF extracts from the first page: its runs joined by single spaces.
fn extracted(bytes: &[u8]) -> String {
    let doc = laminapdf::Document::open_bytes(bytes.to_vec(), laminapdf::OpenOptions::default()).unwrap();
    let text = doc.page_text(0, &laminapdf::extract::TextOptions::default()).unwrap();
    let runs: Vec<String> = text.runs.into_iter().map(|r| r.text).collect();
    runs.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A file of its own in the temporary folder, holding `bytes`: tests run in parallel.
fn scratch(bytes: &[u8]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lamina-added-text-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("out-{n}.pdf"));
    std::fs::write(&path, bytes).unwrap();
    path
}

/// What `pdftotext` reads from the first page, whitespace collapsed; `None` without it.
fn pdftotext(bytes: &[u8]) -> Option<String> {
    let path = scratch(bytes);
    let out = Command::new("pdftotext")
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg("1")
        .arg(&path)
        .arg("-")
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let path = scratch(bytes);
    // No qpdf on the machine: the round trip runs it.
    Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .map_or(true, |o| o.status.success())
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Restriction>) {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let page = first_page(&doc);
        let text = AddText::new(
            id("note"),
            page.clone(),
            rect(72.0, 72.0, 288.0, 120.0),
            vec![TextSpan::plain("Reviewed")],
            date(),
        );
        let ops: [Op; 3] = [
            text.into(),
            UpdateContentObject::new(id("note"), page.clone(), date()).into(),
            DeleteContentObject::new(id("note"), page).into(),
        ];
        let planner = SavePlanner::new(&doc).unwrap();
        // The three ops' verdicts are the same: text is page content in every way.
        let verdicts: Vec<Vec<Restriction>> = ops
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        assert!(verdicts.iter().all(|v| *v == verdicts[0]), "{name}: {verdicts:?}");
        (planner.constraint().kind(), verdicts[0].clone())
    }

    #[test]
    fn approval_signatures_and_a_field_lock_allow_added_text() {
        for name in ["approval.pdf", "two-signatures.pdf", "locked.pdf"] {
            assert_eq!(verdicts(name), (ConstraintKind::AppendOnly, Vec::new()), "{name}");
        }
    }

    #[test]
    fn every_certification_level_forbids_it() {
        for (name, level) in [
            ("certified-p1.pdf", DocMdpPermission::NoChanges),
            ("certified-p2.pdf", DocMdpPermission::FormFill),
            ("certified-p3.pdf", DocMdpPermission::Annotate),
        ] {
            assert_eq!(
                verdicts(name),
                (ConstraintKind::AppendOnly, vec![Restriction::DocMdp(level)]),
                "{name}"
            );
        }
    }
}

#[test]
fn five_scripts_save_in_both_modes_and_read_back_exactly() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = hello();
        let ops = five_scripts(&mut doc);
        doc.apply_all(Origin::Local, ops).unwrap();
        let bytes = save(&mut doc, mode);
        assert!(qpdf_accepts(&bytes), "{mode:?}");
        let lamina = extracted(&bytes);
        let poppler = pdftotext(&bytes);
        for (name, _, text) in SCRIPTS {
            assert!(lamina.contains(text), "{mode:?} {name}: lamina read {lamina:?}");
            if let Some(poppler) = &poppler {
                assert!(poppler.contains(text), "{mode:?} {name}: pdftotext read {poppler:?}");
            }
        }
        let reopened = editable(&bytes);
        let ids: Vec<String> = added_objects(&reopened, &first_page(&reopened))
            .unwrap()
            .iter()
            .map(|o| o.id.to_string())
            .collect();
        assert_eq!(ids, SCRIPTS.map(|(name, _, _)| name));
    }
}

/// Letters, a symbol-like Hebrew word and a Chinese phrase in one line, each from the first font
/// of the chain that has it: the fallback that A.12's symbols use.
#[test]
fn a_fallback_chain_draws_each_character_in_the_first_font_that_has_it() {
    let mut doc = hello();
    let chain = [
        "NotoSans-Regular.ttf",
        "NotoSansHebrew-Regular.ttf",
        "NotoSansSC-Regular.otf",
    ]
    .map(|f| AnnotFont::Font(doc.add_blob(fixture(&format!("fonts/{f}")))));
    let text = "Grüße שלום 世界 end";
    let op = AddText::new(
        id("mixed"),
        first_page(&doc),
        rect(72.0, 100.0, 400.0, 140.0),
        vec![TextSpan::plain(text)],
        date(),
    )
    .with_fonts(chain.to_vec(), 16.0);
    doc.apply_all(Origin::Local, [op.into()]).unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    assert!(qpdf_accepts(&bytes));
    let lamina = extracted(&bytes);
    assert!(lamina.contains(text), "lamina read {lamina:?}");
    // pdftotext puts embedding marks around the Hebrew, so it is read word by word.
    if let Some(poppler) = pdftotext(&bytes) {
        for word in text.split(' ') {
            assert!(poppler.contains(word), "pdftotext read {poppler:?}");
        }
    }
}

#[test]
fn text_added_before_a_save_is_changed_and_deleted_after_it() {
    let mut doc = hello();
    let ops = five_scripts(&mut doc);
    doc.apply_all(Origin::Local, ops).unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    let mut doc = editable(&bytes).with_appearances(AnnotationAppearances);
    let page = first_page(&doc);
    let block = laminapdf_ops::TextBlock::new(328.0, 32.0, vec![TextSpan::plain("Changed after the save")]);
    doc.apply_all(
        Origin::Local,
        [
            UpdateContentObject::new(id("latin"), page.clone(), date())
                .with_text(block)
                .into(),
            DeleteContentObject::new(id("hebrew"), page).into(),
        ],
    )
    .unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    let lamina = extracted(&bytes);
    assert!(lamina.contains("Changed after the save"), "{lamina}");
    assert!(qpdf_accepts(&bytes));
    let reopened = editable(&bytes);
    let ids: Vec<String> = added_objects(&reopened, &first_page(&reopened))
        .unwrap()
        .iter()
        .map(|o| o.id.to_string())
        .collect();
    assert_eq!(ids, ["latin", "arabic", "devanagari", "chinese"]);
}

fn with_text() -> Document {
    let mut doc = hello();
    let ops = five_scripts(&mut doc);
    doc.apply_all(Origin::Local, ops).unwrap();
    doc
}

#[test]
fn undo_leaves_the_overlay_empty_and_saves_what_the_file_was_and_redo_the_edit() {
    let mut undone = with_text();
    undone.undo().unwrap();
    assert!(undone.cos().overlay().is_empty());
    let mut untouched = hello();
    // Adding a font's bytes to a session changes nothing to save.
    assert_eq!(
        save(&mut undone, SaveMode::Incremental),
        save(&mut untouched, SaveMode::Incremental)
    );
    let mut redone = with_text();
    redone.undo().unwrap();
    redone.redo().unwrap();
    assert_eq!(
        save(&mut redone, SaveMode::Incremental),
        save(&mut with_text(), SaveMode::Incremental)
    );
}

#[test]
fn a_tagged_file_stays_tagged_and_the_text_joins_its_structure() {
    let mut doc = editable(&fixture("golden/tagged-report.pdf")).with_appearances(AnnotationAppearances);
    let page = first_page(&doc);
    let note = AddText::new(
        id("note"),
        page,
        rect(72.0, 300.0, 400.0, 340.0),
        vec![TextSpan::plain("Added to a tagged report")],
        date(),
    );
    doc.apply_all(Origin::Local, [note.into()]).unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    assert!(qpdf_accepts(&bytes));
    let reopened = laminapdf_model::Document::open(bytes.clone(), laminapdf_cos::OpenOptions::default()).unwrap();
    assert!(reopened.catalog().unwrap().get("StructTreeRoot").is_some());
    assert!(extracted(&bytes).contains("Added to a tagged report"));
}
