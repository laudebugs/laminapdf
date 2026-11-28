//! Visual signatures (E10 plan §2.1, todo E10c): a signature picture is placed with ops that
//! already exist, so this checks their verdicts on the signed fixtures and that each survives
//! a save, in `roundtrip_suite.rs`. The ops' own suites (`stamps.rs`, `drawing.rs`,
//! `added_content.rs`) cover every other kind; here the placements are the three a signature
//! uses: an image stamp (the default), ink (a drawn signature kept as strokes) and an image as
//! page content ("flatten into the page"), the image a signature-like PNG with an alpha channel.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::editable;
use common::visual_signature::placements;
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, SavePlanner};

mod would_violate {
    use super::*;

    type Verdict = (ConstraintKind, [Vec<Restriction>; 3]);

    fn verdicts(path: &str) -> Verdict {
        let mut doc = editable(&fixture(path));
        let ops = placements(&mut doc);
        let planner = SavePlanner::new(&doc).unwrap();
        let v = |i: usize| planner.would_violate(&ops[i]).iter().map(|v| v.restriction).collect();
        (planner.constraint().kind(), [v(0), v(1), v(2)])
    }

    #[test]
    fn an_unsigned_file_allows_all_three_in_any_mode() {
        assert_eq!(
            verdicts("golden/examples/hello.pdf"),
            (ConstraintKind::Free, [vec![], vec![], vec![]])
        );
    }

    #[test]
    fn approval_signatures_and_a_field_lock_allow_all_three_append_only() {
        let none = (ConstraintKind::AppendOnly, [vec![], vec![], vec![]]);
        assert_eq!(
            [
                verdicts("signed/approval.pdf"),
                verdicts("signed/two-signatures.pdf"),
                verdicts("signed/locked.pdf")
            ],
            [none.clone(), none.clone(), none]
        );
    }

    #[test]
    fn p3_allows_the_annotations_and_forbids_the_content() {
        let content = vec![Restriction::DocMdp(DocMdpPermission::Annotate)];
        assert_eq!(
            verdicts("signed/certified-p3.pdf"),
            (ConstraintKind::AppendOnly, [vec![], vec![], content])
        );
    }

    #[test]
    fn p1_and_p2_forbid_all_three() {
        let all = |p| (ConstraintKind::AppendOnly, [0; 3].map(|_| vec![Restriction::DocMdp(p)]));
        assert_eq!(
            [verdicts("signed/certified-p1.pdf"), verdicts("signed/certified-p2.pdf")],
            [all(DocMdpPermission::NoChanges), all(DocMdpPermission::FormFill)]
        );
    }
}
