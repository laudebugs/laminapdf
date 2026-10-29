//! Shaping goldens (todo B4b): every case in `fixtures/shaping/cases.json` shaped with
//! HarfRust must give exactly the glyphs HarfBuzz's `hb-shape` gave (glyph IDs, UTF-8 byte
//! clusters, advances and offsets in font units), as recorded in
//! `fixtures/shaping/<script>.json` by `tools/shaping-goldens/goldens.py`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

mod common;

use lamina_text::{Direction, HarfRust, Script, ShapeOptions, Shaper};
use serde_json::{Value, json};

fn load(rel: &str) -> Value {
    serde_json::from_slice(&common::fixture(rel)).unwrap()
}

fn cases() -> Vec<Value> {
    load("shaping/cases.json")["cases"].as_array().unwrap().clone()
}

/// HarfRust's glyphs for `case`, in `hb-shape`'s JSON shape.
fn harfrust(case: &Value) -> Value {
    let font = common::font(case["font"].as_str().unwrap());
    let script: [u8; 4] = case["iso15924"].as_str().unwrap().as_bytes().try_into().unwrap();
    let mut options = ShapeOptions::new()
        .with_script(Script::new(&script))
        .with_language(case["language"].as_str().unwrap())
        .with_direction(match case["direction"].as_str().unwrap() {
            "rtl" => Direction::RightToLeft,
            _ => Direction::LeftToRight,
        });
    for feature in case["features"].as_array().unwrap() {
        options = options.with_feature(feature.as_str().unwrap().parse().unwrap());
    }
    let run = HarfRust.shape(&font, case["text"].as_str().unwrap(), &options).unwrap();
    run.glyphs
        .iter()
        .map(|g| {
            json!({
                "g": g.gid.get(), "cl": g.cluster, "ax": g.x_advance,
                "ay": g.y_advance, "dx": g.x_offset, "dy": g.y_offset,
            })
        })
        .collect()
}

/// The golden glyphs for `case`.
fn golden(case: &Value) -> Value {
    let file = load(&format!("shaping/{}.json", case["script"].as_str().unwrap()));
    file["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == case["name"])
        .map(|c| c["glyphs"].clone())
        .unwrap_or_else(|| panic!("no golden for {}; run tools/shaping-goldens/goldens.py", case["name"]))
}

#[test]
fn harfrust_matches_harfbuzz_for_every_case() {
    let mut failures = Vec::new();
    for case in cases() {
        if harfrust(&case) != golden(&case) {
            failures.push(format!("{}/{}", case["script"], case["name"]));
        }
    }
    assert!(failures.is_empty(), "differs from hb-shape: {failures:?}");
}

#[test]
fn goldens_cover_five_scripts() {
    let mut scripts: Vec<String> = cases()
        .iter()
        .map(|c| c["script"].as_str().unwrap().to_owned())
        .collect();
    scripts.dedup();
    assert_eq!(scripts, ["latin", "arabic", "hebrew", "devanagari", "cjk"]);
}

#[test]
fn goldens_were_written_with_the_pinned_harfbuzz() {
    let script = String::from_utf8(common::fixture("../tools/shaping-goldens/goldens.py")).unwrap();
    let pinned = script
        .lines()
        .find_map(|l| l.strip_prefix("HB_VERSION = "))
        .unwrap()
        .trim_matches('"');
    for case in cases() {
        let file = load(&format!("shaping/{}.json", case["script"].as_str().unwrap()));
        assert_eq!(file["harfbuzz"].as_str(), Some(pinned), "{}", case["script"]);
    }
}

#[test]
fn liga_off_disables_the_fi_ligature() {
    let all = cases();
    let find = |name: &str| all.iter().find(|c| c["name"] == name).unwrap();
    let with = golden(find("ligatures")).as_array().unwrap().len();
    let without = golden(find("ligatures-off")).as_array().unwrap().len();
    let text = find("ligatures")["text"].as_str().unwrap();
    assert_eq!(without, text.chars().count());
    assert!(with < without, "ligatures should merge glyphs: {with} vs {without}");
}

#[test]
fn shaping_is_deterministic() {
    for case in cases() {
        assert_eq!(harfrust(&case), harfrust(&case), "{}", case["name"]);
    }
}
