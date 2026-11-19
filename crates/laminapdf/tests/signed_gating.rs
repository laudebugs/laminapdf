//! Signed documents under every page-organisation op ([E2 plan §6]): the verdict
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
use std::fmt;

use laminapdf::{
    AddAnnot, AnnotId, AnnotSpec, ConstraintKind, CropMargins, CropPages, DeleteAnnot, DeletePages, DocMdpPermission,
    Document, DuplicatePages, InsertBlankPages, InsertPagesFrom, LabelStyle, MergeDocuments, MergeSource, MovePages,
    Note, NoteIcon, Op, OpenOptions, OrientationMethod, PageBoxes, PageId, PageLabelRange, PageOrientation,
    PagePosition, PageSize, ReplacePages, ResizePages, Restriction, ReversePages, RotatePages, Rotation, SaveMode,
    SaveOptions, SetOrientation, SetPageBoxes, SetPageLabels, UpdateAnnot, UserSpaceRect, Violation,
};
use laminapdf_test_fixtures::fixture;

/// Every page-organisation op ([E2 plan §2.1]), each built against one target page.
///
/// [E2 plan §2.1]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#2-the-ops
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageOp {
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

const EVERY_OP: [PageOp; 14] = [
    PageOp::InsertBlank,
    PageOp::Delete,
    PageOp::Move,
    PageOp::Rotate,
    PageOp::Reverse,
    PageOp::Duplicate,
    PageOp::InsertFrom,
    PageOp::Replace,
    PageOp::Merge,
    PageOp::Boxes,
    PageOp::Crop,
    PageOp::Resize,
    PageOp::Orientation,
    PageOp::Labels,
];

impl PageOp {
    /// Whether the op deletes or reshows its target page: the ops the plan's §6 lists as
    /// breaking a signed widget or a locked field on that page. Moving, reversing, inserting
    /// and duplicating leave the page as it was shown.
    const fn alters_its_page(self) -> bool {
        matches!(
            self,
            Self::Delete | Self::Replace | Self::Rotate | Self::Crop | Self::Boxes | Self::Resize | Self::Orientation
        )
    }

    /// Whether the op has a target page at all: adding pages at the end, merging and labelling
    /// are the same op whichever page is picked, so they run once per fixture.
    const fn has_a_target(self) -> bool {
        !matches!(self, Self::InsertBlank | Self::InsertFrom | Self::Merge | Self::Labels)
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
    /// A certification signature at this `DocMDP` level: no page op is permitted.
    Certified(DocMdpPermission),
    /// A usage-rights signature: its rights cover no page op.
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
    fn named(name: &str) -> &'static Self {
        FIXTURES.iter().find(|f| f.name == name).unwrap()
    }

    fn open(&self) -> Document {
        Document::open_bytes(self.bytes(), OpenOptions::default()).unwrap()
    }

    fn bytes(&self) -> Vec<u8> {
        fixture(&format!("signed/{}", self.name))
    }
}

/// One op on one page of one fixture.
#[derive(Clone, Copy)]
struct Case {
    fixture: &'static Fixture,
    op: PageOp,
    target: usize,
}

impl fmt::Display for Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:?} on page {}", self.fixture.name, self.op, self.target + 1)
    }
}

impl Case {
    /// Every case: each op on each page of each fixture, once for an op with no target.
    fn all() -> Vec<Self> {
        let mut out = Vec::new();
        for fixture in &FIXTURES {
            let pages = fixture.open().page_count().unwrap() as usize;
            for op in EVERY_OP {
                let targets = if op.has_a_target() { pages } else { 1 };
                out.extend((0..targets).map(|target| Self { fixture, op, target }));
            }
        }
        out
    }

    /// The fixture opened, and the op built on it.
    fn build(self) -> (Document, Op) {
        let mut doc = self.fixture.open();
        let op = self.op.on(&mut doc, self.target);
        (doc, op)
    }

    /// The restrictions the op breaks ([E2 plan §6]): every op under a certification or usage
    /// rights, and on an approval-signed file only the ops that delete or reshow the page
    /// holding a signed widget or a locked field's widget.
    ///
    /// [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
    fn expected(self) -> BTreeSet<Restriction> {
        let f = self.fixture;
        let mut out = BTreeSet::new();
        match f.guard {
            Guard::Certified(p) => {
                out.insert(Restriction::DocMdp(p));
            }
            Guard::UsageRights => {
                out.insert(Restriction::UsageRights);
            }
            Guard::Approval => {}
        }
        if self.op.alters_its_page() {
            if f.signed_widget == Some(self.target) {
                out.insert(Restriction::SignedWidget);
            }
            if f.locked_widget == Some(self.target) {
                out.insert(Restriction::FieldMdp);
            }
        }
        out
    }

    /// Applies the op, and says whether it changed anything. On a one-page fixture, deleting
    /// the only page is refused, and moving or reversing it changes nothing: neither leaves a
    /// change to save.
    fn applied(self, doc: &mut Document, op: Op) -> bool {
        match doc.apply(vec![op]) {
            Ok(edited) => {
                assert!(
                    edited.changed || matches!(self.op, PageOp::Move | PageOp::Reverse),
                    "{self} changed nothing"
                );
                edited.changed
            }
            Err(e) => {
                assert!(
                    doc.page_count().unwrap() == 1 && self.op == PageOp::Delete,
                    "{self}: {e}"
                );
                false
            }
        }
    }

    /// Writes `bytes` to `LAMINA_SIGNED_GATING_OUT`, when it is set.
    fn keep(self, kind: &str, bytes: &[u8]) {
        if let Some(dir) = std::env::var_os("LAMINA_SIGNED_GATING_OUT") {
            let dir = std::path::Path::new(&dir).join(kind);
            std::fs::create_dir_all(&dir).unwrap();
            let stem = self.fixture.name.trim_end_matches(".pdf");
            let name = format!("{stem}-{:?}-p{}.pdf", self.op, self.target + 1);
            std::fs::write(dir.join(name), bytes).unwrap();
        }
    }
}

fn restrictions(violations: &[Violation]) -> BTreeSet<Restriction> {
    violations.iter().map(|v| v.restriction).collect()
}

fn page_ids(doc: &Document) -> Vec<PageId> {
    doc.page_infos().unwrap().into_iter().map(|p| p.id).collect()
}

#[test]
fn every_page_op_on_every_signed_fixture_gets_its_verdict_before_it_applies() {
    let mut wrong = Vec::new();
    for case in Case::all() {
        let (doc, op) = case.build();
        let got = restrictions(&doc.would_violate(&op).unwrap());
        if got != case.expected() {
            wrong.push(format!("{case}: {got:?}, expected {:?}", case.expected()));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Applied, the op gets the same verdict from the save planner, and the constraint follows:
/// `Conflict` when anything is broken, `AppendOnly` (an incremental update) otherwise.
#[test]
fn the_save_planner_gives_the_same_verdict_once_the_op_applies() {
    let mut wrong = Vec::new();
    for case in Case::all() {
        let (mut doc, op) = case.build();
        if !case.applied(&mut doc, op) {
            continue;
        }
        let plan = doc.plan(&SaveOptions::new()).unwrap();
        let expected = case.expected();
        let kind = if expected.is_empty() {
            ConstraintKind::AppendOnly
        } else {
            ConstraintKind::Conflict
        };
        let got = (plan.constraint, restrictions(&plan.violations));
        if got != (kind, expected.clone()) {
            wrong.push(format!("{case}: {got:?}, expected {:?}", (kind, expected)));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// On an approval-signed file, every op that breaks nothing saves as an incremental update:
/// the signed revision's bytes come first, unchanged, so its `/ByteRange` still covers them.
#[test]
fn an_allowed_op_on_an_approval_signed_file_saves_after_the_signed_bytes() {
    for case in Case::all() {
        if !matches!(case.fixture.guard, Guard::Approval) || !case.expected().is_empty() {
            continue;
        }
        let (mut doc, op) = case.build();
        if !case.applied(&mut doc, op) {
            continue;
        }
        let saved = doc.save(&SaveOptions::new()).unwrap();
        assert_eq!(
            (
                saved.report.mode(),
                saved.report.invalidated().len(),
                saved.bytes.starts_with(&case.fixture.bytes())
            ),
            (SaveMode::Incremental, 0, true),
            "{case}"
        );
        case.keep("allowed", &saved.bytes);
    }
}

/// The named destinations of the approval file outlast every op it allows: none of them
/// deletes page 2, which holds the signature's widget, and a deleted page's destination
/// follows to the next page.
#[test]
fn an_allowed_op_keeps_the_named_destinations() {
    let fixture = Fixture::named("approval-organised.pdf");
    for case in Case::all() {
        if !std::ptr::eq(case.fixture, fixture) || !case.expected().is_empty() {
            continue;
        }
        let (mut doc, op) = case.build();
        if !case.applied(&mut doc, op) {
            continue;
        }
        let again = Document::open_bytes(doc.save(&SaveOptions::new()).unwrap().bytes, OpenOptions::default()).unwrap();
        for name in ["page-1", "page-2", "page-3"] {
            assert!(again.named_destination(name).unwrap().is_some(), "{case}: {name}");
        }
    }
}

/// An op that breaks a signature saves only through the override, which removes the
/// signatures it would break: the file opens again with no signed signature left.
#[test]
fn a_refused_op_saves_only_through_the_override_that_removes_the_signatures() {
    for case in Case::all() {
        if case.expected().is_empty() {
            continue;
        }
        let (mut doc, op) = case.build();
        if !case.applied(&mut doc, op) {
            continue;
        }
        let refused = doc.save(&SaveOptions::new()).map(|_| ()).map_err(|e| e.code());
        let saved = doc.save(&SaveOptions::new().with_invalidate_signatures(true)).unwrap();
        let again = Document::open_bytes(saved.bytes.clone(), OpenOptions::default()).unwrap();
        let left = again.plan(&SaveOptions::new()).unwrap().signatures.len();
        assert_eq!((refused, left), (Err("would-invalidate-signatures"), 0), "{case}");
        case.keep("overridden", &saved.bytes);
    }
}

/// The usage-rights file and its text note.
fn usage_rights() -> (Document, AnnotId) {
    let doc = Fixture::named("usage-rights.pdf").open();
    let note = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    (doc, note)
}

fn changed(id: AnnotId) -> Op {
    let mut op = UpdateAnnot::new(id);
    op.contents = Some(Some("Changed".to_owned()));
    op.into()
}

/// The usage-rights file grants `/Annots [/Create /Delete]`: adding and deleting an
/// annotation stay inside its rights, changing one doesn't ([E4 plan §6]).
///
/// [E4 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint
#[test]
fn usage_rights_hold_each_annotation_op_to_its_own_right() {
    let (doc, note) = usage_rights();
    let add = AddAnnot::new(
        page_ids(&doc)[0].clone(),
        AnnotSpec::Note(Note::new(
            laminapdf_model::UserSpacePoint::new(400.0, 700.0),
            NoteIcon::Comment,
        )),
    );
    let verdict = |op: Op| restrictions(&doc.would_violate(&op).unwrap());
    assert_eq!(
        [
            verdict(add.into()),
            verdict(changed(note.clone())),
            verdict(DeleteAnnot::new(note).into())
        ],
        [
            BTreeSet::new(),
            BTreeSet::from([Restriction::UsageRights]),
            BTreeSet::new()
        ]
    );
}

/// Applied, a change the rights don't grant makes the save a conflict, as `would_violate`
/// said; a delete, which they grant, saves incrementally.
#[test]
fn usage_rights_judge_an_applied_annotation_op_by_its_own_right_at_save_time() {
    let constraint = |op: fn(AnnotId) -> Op| {
        let (mut doc, note) = usage_rights();
        doc.apply(vec![op(note)]).unwrap();
        doc.plan(&SaveOptions::new()).unwrap().constraint
    };
    assert_eq!(
        (constraint(changed), constraint(|id| DeleteAnnot::new(id).into())),
        (ConstraintKind::Conflict, ConstraintKind::AppendOnly)
    );
}

/// Flattening every field it may on an approval-signed form whose signature locks `name` and
/// `agree` (todo E8p): `address` goes; the signature fields, the locked fields and the push
/// button, which has no appearance to draw, stay; and the save appends after the signed bytes,
/// for `check_saved.py` to validate in pyHanko.
#[test]
fn flattening_an_approval_signed_form_leaves_the_signature_and_the_locked_fields() {
    let bytes = fixture("signed/form-locked-include.pdf");
    let mut doc = Document::open_bytes(bytes.clone(), OpenOptions::default()).unwrap();
    doc.flatten_form(None, laminapdf::FlattenIntent::View).unwrap();
    let mut names: Vec<String> = doc.form().unwrap().fields.into_iter().map(|f| f.name).collect();
    names.sort();
    let saved = doc.save(&SaveOptions::new()).unwrap();
    assert_eq!(
        (
            names,
            saved.report.mode(),
            saved.report.invalidated().len(),
            saved.bytes.starts_with(&bytes)
        ),
        (
            ["Sig1", "agree", "name", "signature", "submit"]
                .map(str::to_owned)
                .to_vec(),
            SaveMode::Incremental,
            0,
            true
        )
    );
    if let Some(dir) = std::env::var_os("LAMINA_SIGNED_GATING_OUT") {
        let dir = std::path::Path::new(&dir).join("allowed");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("form-locked-include-FlattenForm.pdf"), &saved.bytes).unwrap();
    }
}
