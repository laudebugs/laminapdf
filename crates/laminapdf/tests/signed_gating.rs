//! Signed documents under every E2 page op (todo E2i, [E2 plan §6]): the verdict
//! `would_violate` gives before each op, the same verdict from the save planner after it,
//! an incremental save that leaves an approval signature's bytes alone, and the override that
//! removes the signatures an op would break.
//!
//! The fixtures are `fixtures/signed/` (pyHanko). With `LAMINA_SIGNED_GATING_OUT` set, every
//! saved file is also written to that folder, for `fixtures/signed/check_saved.py` to validate
//! in pyHanko and `pdfsig`.
//!
//! [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]
#![cfg(feature = "page-checks")]

use std::collections::BTreeSet;

use laminapdf::{
    AddAnnot, AnnotSpec, ConstraintKind, CropMargins, CropPages, DeleteAnnot, DeletePages, DocMdpPermission, Document,
    DuplicatePages, InsertBlankPages, InsertPagesFrom, LabelStyle, MergeDocuments, MergeSource, MovePages, Note,
    NoteIcon, Op, OpenOptions, OrientationMethod, PageBoxes, PageId, PageLabelRange, PageOrientation, PagePosition,
    PageSize, ReplacePages, ResizePages, Restriction, ReversePages, RotatePages, Rotation, SaveMode, SaveOptions,
    SetOrientation, SetPageBoxes, SetPageLabels, UpdateAnnot, UserSpaceRect,
};
use laminapdf_test_fixtures::fixture;

/// Every E2 op ([E2 plan §2.1]), each built against one target page.
///
/// [E2 plan §2.1]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#2-the-ops
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum E2 {
    InsertBlank,
    Delete,
    Move,
    Rotate,
    Reverse,
    Duplicate,
    InsertFrom,
    Replace,
    Merge,
    Boxes,
    Crop,
    Resize,
    Orientation,
    Labels,
}

const EVERY_OP: [E2; 14] = [
    E2::InsertBlank,
    E2::Delete,
    E2::Move,
    E2::Rotate,
    E2::Reverse,
    E2::Duplicate,
    E2::InsertFrom,
    E2::Replace,
    E2::Merge,
    E2::Boxes,
    E2::Crop,
    E2::Resize,
    E2::Orientation,
    E2::Labels,
];

impl E2 {
    /// Whether the op deletes or reshows its target page: the ops the plan's §6 lists as
    /// breaking a signed widget or a locked field on that page. Moving, reversing, inserting
    /// and duplicating leave the page as it was shown.
    const fn alters_its_page(self) -> bool {
        matches!(
            self,
            Self::Delete | Self::Replace | Self::Rotate | Self::Crop | Self::Boxes | Self::Resize | Self::Orientation
        )
    }

    /// The op on `doc` with page `target` (by index) as its target. Ops that copy pages in
    /// take them from `corpus/hello.pdf`, added to `doc` as a source.
    fn on(self, doc: &mut Document, target: usize) -> Op {
        let ids = page_ids(doc);
        let page = vec![ids[target].clone()];
        let mut source = || {
            let id = doc.add_source(fixture("corpus/hello.pdf"), None).unwrap();
            let pages = doc.source_pages(&id).unwrap();
            (id, pages)
        };
        match self {
            Self::InsertBlank => InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER).into(),
            Self::Delete => DeletePages::new(page).into(),
            // To the end, or the last page to the start, so the page always moves.
            Self::Move if target + 1 == ids.len() => MovePages::new(page, PagePosition::Start).into(),
            Self::Move => MovePages::new(page, PagePosition::End).into(),
            Self::Rotate => RotatePages::turn(page, Rotation::Clockwise90).into(),
            Self::Reverse => ReversePages::new(ids).into(),
            Self::Duplicate => DuplicatePages::new(page, PagePosition::End).into(),
            Self::InsertFrom => {
                let (id, pages) = source();
                InsertPagesFrom::new(id, pages, PagePosition::End).into()
            }
            Self::Replace => {
                let (id, pages) = source();
                ReplacePages::new(page, id, pages).into()
            }
            Self::Merge => {
                let (id, _) = source();
                MergeDocuments::new(vec![MergeSource::new(id)], PagePosition::End).into()
            }
            Self::Boxes => SetPageBoxes::new(
                page,
                PageBoxes::default().with_crop(Some(UserSpaceRect::from_corners(20.0, 20.0, 592.0, 772.0))),
            )
            .into(),
            Self::Crop => CropPages::new(page, CropMargins::all(10.0)).into(),
            Self::Resize => ResizePages::scale(page, PageSize::A4).into(),
            Self::Orientation => {
                SetOrientation::new(page, PageOrientation::Landscape, OrientationMethod::Rotate).into()
            }
            Self::Labels => {
                SetPageLabels::new(vec![PageLabelRange::new(ids[0].clone(), Some(LabelStyle::LowerRoman))]).into()
            }
        }
    }
}

/// What a fixture holds that a page op can break.
#[derive(Debug, Clone, Copy)]
enum Guard {
    /// A certification signature at this `DocMDP` level: no E2 op is permitted.
    Certified(DocMdpPermission),
    /// A usage-rights signature: its rights cover no E2 op.
    UsageRights,
    /// Approval signatures only.
    Approval,
}

struct Fixture {
    name: &'static str,
    guard: Guard,
    /// The page holding a signed signature's widget.
    signed_widget: Option<usize>,
    /// The page holding the widget of a field a `FieldMDP` lock covers.
    locked_widget: Option<usize>,
}

const FIXTURES: [Fixture; 8] = [
    Fixture {
        name: "certified-p1.pdf",
        guard: Guard::Certified(DocMdpPermission::NoChanges),
        signed_widget: Some(0),
        locked_widget: None,
    },
    Fixture {
        name: "certified-p2.pdf",
        guard: Guard::Certified(DocMdpPermission::FormFill),
        signed_widget: Some(0),
        locked_widget: None,
    },
    Fixture {
        name: "certified-p3.pdf",
        guard: Guard::Certified(DocMdpPermission::Annotate),
        signed_widget: Some(0),
        locked_widget: None,
    },
    Fixture {
        name: "certified-p2-outline.pdf",
        guard: Guard::Certified(DocMdpPermission::FormFill),
        signed_widget: Some(1),
        locked_widget: None,
    },
    Fixture {
        name: "usage-rights.pdf",
        guard: Guard::UsageRights,
        signed_widget: None,
        locked_widget: None,
    },
    Fixture {
        name: "approval.pdf",
        guard: Guard::Approval,
        signed_widget: Some(0),
        locked_widget: None,
    },
    Fixture {
        name: "approval-organised.pdf",
        guard: Guard::Approval,
        signed_widget: Some(1),
        locked_widget: None,
    },
    Fixture {
        name: "locked-page1.pdf",
        guard: Guard::Approval,
        signed_widget: Some(2),
        locked_widget: Some(0),
    },
];

impl Fixture {
    fn open(&self) -> Document {
        Document::open_bytes(fixture(&format!("signed/{}", self.name)), OpenOptions::default()).unwrap()
    }

    /// The restrictions `op` on page `target` breaks ([E2 plan §6]): every op under a
    /// certification or usage rights, and on an approval-signed file only the ops that
    /// delete or reshow the page holding a signed widget or a locked field's widget.
    ///
    /// [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
    fn expected(&self, op: E2, target: usize) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        match self.guard {
            Guard::Certified(p) => {
                out.insert(name(Restriction::DocMdp(p)));
            }
            Guard::UsageRights => {
                out.insert(name(Restriction::UsageRights));
            }
            Guard::Approval => {}
        }
        if op.alters_its_page() {
            if self.signed_widget == Some(target) {
                out.insert(name(Restriction::SignedWidget));
            }
            if self.locked_widget == Some(target) {
                out.insert(name(Restriction::FieldMdp));
            }
        }
        out
    }
}

fn name(r: Restriction) -> String {
    format!("{r:?}")
}

fn names(violations: &[laminapdf::Violation]) -> BTreeSet<String> {
    violations.iter().map(|v| name(v.restriction)).collect()
}

fn page_ids(doc: &Document) -> Vec<PageId> {
    doc.page_infos().unwrap().into_iter().map(|p| p.id).collect()
}

/// Every (fixture, op, target page) case.
fn cases() -> impl Iterator<Item = (&'static Fixture, E2, usize)> {
    FIXTURES.iter().flat_map(|f| {
        let pages = f.open().page_count().unwrap() as usize;
        EVERY_OP
            .into_iter()
            .flat_map(move |op| (0..pages).map(move |target| (f, op, target)))
    })
}

/// Writes `bytes` to `LAMINA_SIGNED_GATING_OUT`, when it is set.
fn keep(kind: &str, f: &Fixture, op: E2, target: usize, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("LAMINA_SIGNED_GATING_OUT") {
        let dir = std::path::Path::new(&dir).join(kind);
        std::fs::create_dir_all(&dir).unwrap();
        let stem = f.name.trim_end_matches(".pdf");
        std::fs::write(dir.join(format!("{stem}-{op:?}-p{}.pdf", target + 1)), bytes).unwrap();
    }
}

/// Applies `op`, and says whether it changed anything. On a one-page fixture, deleting the
/// only page is refused, and moving or reversing it changes nothing: neither leaves a change
/// to save.
fn applied(doc: &mut Document, op: Op, f: &Fixture, e2: E2, target: usize) -> bool {
    match doc.apply(vec![op]) {
        Ok(edited) => {
            assert!(
                edited.changed || matches!(e2, E2::Move | E2::Reverse),
                "{} {e2:?} on page {} changed nothing",
                f.name,
                target + 1
            );
            edited.changed
        }
        Err(e) => {
            assert!(
                doc.page_count().unwrap() == 1 && matches!(e2, E2::Delete),
                "{} {e2:?} on page {}: {e}",
                f.name,
                target + 1
            );
            false
        }
    }
}

#[test]
fn every_e2_op_on_every_signed_fixture_gets_its_verdict_before_it_applies() {
    let mut wrong = Vec::new();
    for (f, op, target) in cases() {
        let mut doc = f.open();
        let built = op.on(&mut doc, target);
        let got = names(&doc.would_violate(&built).unwrap());
        let expected = f.expected(op, target);
        if got != expected {
            wrong.push(format!(
                "{} {op:?} on page {}: {got:?}, expected {expected:?}",
                f.name,
                target + 1
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Applied, the op gets the same verdict from the save planner, and the constraint follows:
/// `Conflict` when anything is broken, `AppendOnly` (an incremental update) otherwise.
#[test]
fn the_save_planner_gives_the_same_verdict_once_the_op_applies() {
    let mut wrong = Vec::new();
    for (f, op, target) in cases() {
        let mut doc = f.open();
        let built = op.on(&mut doc, target);
        if !applied(&mut doc, built, f, op, target) {
            continue;
        }
        let plan = doc.plan(&SaveOptions::new()).unwrap();
        let expected = f.expected(op, target);
        let kind = if expected.is_empty() {
            ConstraintKind::AppendOnly
        } else {
            ConstraintKind::Conflict
        };
        let got = (plan.constraint, names(&plan.violations));
        if got != (kind, expected.clone()) {
            wrong.push(format!(
                "{} {op:?} on page {}: {got:?}, expected {:?}",
                f.name,
                target + 1,
                (kind, expected)
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// On an approval-signed file, every op that breaks nothing saves as an incremental update:
/// the signed revision's bytes come first, unchanged, so its `/ByteRange` still covers them.
#[test]
fn an_allowed_op_on_an_approval_signed_file_saves_after_the_signed_bytes() {
    for (f, op, target) in cases() {
        if !matches!(f.guard, Guard::Approval) || !f.expected(op, target).is_empty() {
            continue;
        }
        let original = fixture(&format!("signed/{}", f.name));
        let mut doc = f.open();
        let built = op.on(&mut doc, target);
        if !applied(&mut doc, built, f, op, target) {
            continue;
        }
        let saved = doc.save(&SaveOptions::new()).unwrap();
        assert_eq!(
            (
                saved.report.mode(),
                saved.report.invalidated().len(),
                saved.bytes.starts_with(&original)
            ),
            (SaveMode::Incremental, 0, true),
            "{} {op:?} on page {}",
            f.name,
            target + 1
        );
        keep("allowed", f, op, target, &saved.bytes);
    }
}

/// An op that breaks a signature saves only through the override, which removes the
/// signatures it would break: the file opens again with no signed signature left.
#[test]
fn a_refused_op_saves_only_through_the_override_that_removes_the_signatures() {
    for (f, op, target) in cases() {
        if f.expected(op, target).is_empty() {
            continue;
        }
        let mut doc = f.open();
        let built = op.on(&mut doc, target);
        if !applied(&mut doc, built, f, op, target) {
            continue;
        }
        let refused = doc.save(&SaveOptions::new()).map(|_| ()).map_err(|e| e.code());
        let saved = doc.save(&SaveOptions::new().with_invalidate_signatures(true)).unwrap();
        let again = Document::open_bytes(saved.bytes.clone(), OpenOptions::default()).unwrap();
        let left = again.plan(&SaveOptions::new()).unwrap().signatures.len();
        assert_eq!(
            (refused, left),
            (Err("would-invalidate-signatures"), 0),
            "{} {op:?} on page {}",
            f.name,
            target + 1
        );
        keep("overridden", f, op, target, &saved.bytes);
    }
}

/// The usage-rights file grants `/Annots [/Create /Delete]`: adding and deleting an
/// annotation stay inside its rights, changing one doesn't ([E4 plan §6]).
///
/// [E4 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint
#[test]
fn usage_rights_hold_each_annotation_op_to_its_own_right() {
    let f = &FIXTURES[4];
    assert_eq!(f.name, "usage-rights.pdf");
    let doc = f.open();
    let note: laminapdf::AnnotId = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    let mut update = UpdateAnnot::new(note.clone());
    update.contents = Some(Some("Changed".to_owned()));
    let ops: [(&str, Op); 3] = [
        (
            "add",
            AddAnnot::new(
                page_ids(&doc)[0].clone(),
                AnnotSpec::Note(Note::new(
                    laminapdf_model::UserSpacePoint::new(400.0, 700.0),
                    NoteIcon::Comment,
                )),
            )
            .into(),
        ),
        ("update", update.into()),
        ("delete", DeleteAnnot::new(note).into()),
    ];
    let verdicts: Vec<(&str, BTreeSet<String>)> = ops
        .iter()
        .map(|(what, op)| (*what, names(&doc.would_violate(op).unwrap())))
        .collect();
    let rights = BTreeSet::from([name(Restriction::UsageRights)]);
    assert_eq!(
        verdicts,
        [
            ("add", BTreeSet::new()),
            ("update", rights),
            ("delete", BTreeSet::new())
        ]
    );
}

/// Applied, a change the rights don't grant makes the save a conflict, as `would_violate`
/// said; a delete, which they grant, saves incrementally.
#[test]
fn usage_rights_judge_an_applied_annotation_op_by_its_own_right_at_save_time() {
    let f = &FIXTURES[4];
    let verdict = |op: fn(laminapdf::AnnotId) -> Op| {
        let mut doc = f.open();
        let note: laminapdf::AnnotId = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
        doc.apply(vec![op(note)]).unwrap();
        doc.plan(&SaveOptions::new()).unwrap().constraint
    };
    let update = |id| {
        let mut op = UpdateAnnot::new(id);
        op.contents = Some(Some("Changed".to_owned()));
        Op::from(op)
    };
    assert_eq!(
        (verdict(update), verdict(|id| DeleteAnnot::new(id).into())),
        (ConstraintKind::Conflict, ConstraintKind::AppendOnly)
    );
}
