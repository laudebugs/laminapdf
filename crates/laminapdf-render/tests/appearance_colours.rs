//! The page appearance colour maps against `fixtures/appearance/colours.json`, the table that
//! `@laminapdf/render`'s port checks too, so both give the same bytes.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf_render::{PageAppearance, Rgba8};
use laminapdf_test_fixtures::fixture;

fn rgb(v: &serde_json::Value) -> [u8; 3] {
    let channel = |i: usize| u8::try_from(v[i].as_u64().unwrap()).unwrap();
    [channel(0), channel(1), channel(2)]
}

#[test]
fn every_sample_maps_as_the_table_says() {
    let table: serde_json::Value = serde_json::from_slice(&fixture("appearance/colours.json")).unwrap();
    let opaque = |[r, g, b]: [u8; 3]| Rgba8 { r, g, b, a: 255 };
    let high_contrast = PageAppearance::HighContrast {
        ink: opaque(rgb(&table["highContrast"]["ink"])),
        paper: opaque(rgb(&table["highContrast"]["paper"])),
    };
    let modes = [
        ("original", PageAppearance::Original),
        ("sepia", PageAppearance::Sepia),
        ("dark", PageAppearance::Dark),
        ("high-contrast", high_contrast),
    ];
    for sample in table["samples"].as_array().unwrap() {
        let input = rgb(&sample["rgb"]);
        for (name, mode) in modes {
            assert_eq!(mode.recolour_rgb(input), rgb(&sample[name]), "{name} {input:?}");
        }
    }
}
