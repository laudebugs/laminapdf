//! `PageFrame::to_user_space` against fixtures/geometry/page-to-user.json, the cases
//! `@laminapdf/viewer`'s `pageToUser` reads too, so the reader maps page space as the model does.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{PageFrame, PointPt, Pt, Rotation, UserSpaceRect};
use laminapdf_test_fixtures::json;

fn number(v: &serde_json::Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn page_points_map_to_user_space_as_the_shared_cases_say() {
    let fixture = json("geometry/page-to-user.json");
    let cases = fixture["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let crop: Vec<f64> = case["crop"].as_array().unwrap().iter().map(number).collect();
        let rotate = Rotation::from_degrees(case["rotate"].as_i64().unwrap()).unwrap();
        let frame = PageFrame::new(
            UserSpaceRect::from_corners(crop[0], crop[1], crop[2], crop[3]),
            rotate,
            number(&case["userUnit"]),
        );
        let page = case["page"].as_array().unwrap();
        let user = frame.to_user_space(PointPt::new(Pt::new(number(&page[0])), Pt::new(number(&page[1]))));
        let want = case["user"].as_array().unwrap();
        assert!(
            (user.x - number(&want[0])).abs() < 1e-9 && (user.y - number(&want[1])).abs() < 1e-9,
            "{case}: got {user:?}"
        );
    }
}
