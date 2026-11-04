//! Variable-font instances and collection members (research 03 §7.9), malformed axes and
//! non-finite values included.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "tests: a panic is a test failure; values are small font units"
)]

mod common;

use common::{TestPdf, font, name_of, parse_cmap, patch_table, read_fonts};
use laminapdf_test_fixtures::fixture;
use laminapdf_text::{Error, Font, Tag};
use skrifa::MetadataProvider as _;
use skrifa::instance::{LocationRef, Size};

fn bold() -> Font {
    font("NotoSans-VF.ttf").instance(&[(Tag::WEIGHT, 700.0)]).unwrap()
}

#[test]
fn variable_font_lists_its_axes() {
    let tags: Vec<Tag> = font("NotoSans-VF.ttf").info().axes.iter().map(|a| a.tag).collect();
    assert_eq!(tags, [Tag::WEIGHT, Tag::WIDTH]);
}

#[test]
fn instance_has_a_distinct_postscript_name() {
    assert_eq!(bold().info().postscript_name, "NotoSans-Regular_wght700");
}

#[test]
fn instance_reports_its_weight() {
    assert_eq!(bold().info().weight, 700);
}

#[test]
fn instance_is_wider_than_the_default() {
    let regular = font("NotoSans-VF.ttf");
    let gid = regular.glyph_for('m').unwrap();
    assert!(bold().advance(gid).unwrap() > regular.advance(gid).unwrap());
}

#[test]
fn instance_clamps_to_the_axis_range() {
    let heavy = font("NotoSans-VF.ttf").instance(&[(Tag::WEIGHT, 5000.0)]).unwrap();
    assert_eq!(heavy.coordinates(), [(Tag::WEIGHT, 900.0)]);
}

#[test]
fn instance_of_a_static_font_is_an_unknown_axis() {
    assert_eq!(
        font("NotoSans-Regular.ttf")
            .instance(&[(Tag::WEIGHT, 700.0)])
            .unwrap_err(),
        Error::UnknownAxis {
            font: "NotoSans-Regular".to_owned(),
            tag: Tag::WEIGHT
        }
    );
}

#[test]
fn wght_700_instance_embeds_and_extracts() {
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(bold()).unwrap();
    pdf.line(key, 12.0, "Bold").unwrap();
    let f = &read_fonts(&pdf.finish().unwrap().0)[0];
    let (cmap, _) = parse_cmap(&f.to_unicode);
    assert_eq!(
        (
            name_of(&f.type0, "BaseFont").ends_with("+NotoSans-Regular_wght700"),
            cmap.values().cloned().collect::<String>()
        ),
        (true, "Bold".to_owned())
    );
}

#[test]
fn instance_subset_has_no_variation_tables() {
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(bold()).unwrap();
    pdf.line(key, 12.0, "x").unwrap();
    let program = &read_fonts(&pdf.finish().unwrap().0)[0].program;
    let parsed = skrifa::FontRef::new(program).unwrap();
    assert!(parsed.table_data(skrifa::Tag::new(b"gvar")).is_none());
}

#[test]
fn instances_of_one_font_get_different_tags() {
    let mut pdf = TestPdf::new();
    let regular = pdf.add_font(font("NotoSans-VF.ttf")).unwrap();
    let heavy = pdf.add_font(bold()).unwrap();
    pdf.line(regular, 12.0, "same").unwrap();
    pdf.line(heavy, 12.0, "same").unwrap();
    let embedded = pdf.finish().unwrap().1;
    assert_ne!(embedded[0].base_font[..6], embedded[1].base_font[..6]);
}

#[test]
fn collection_has_two_members() {
    assert_eq!(
        Font::collection_len(&fixture("fonts/NotoSans-RegularBold.ttc")).unwrap(),
        2
    );
}

#[test]
fn collection_member_1_is_bold() {
    let member = Font::from_collection(fixture("fonts/NotoSans-RegularBold.ttc"), 1).unwrap();
    assert_eq!(member.info().postscript_name, "NotoSans-Bold");
}

#[test]
fn collection_member_embeds_and_extracts() {
    let member = Font::from_collection(fixture("fonts/NotoSans-RegularBold.ttc"), 1).unwrap();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(member).unwrap();
    pdf.line(key, 12.0, "Ttc").unwrap();
    let f = &read_fonts(&pdf.finish().unwrap().0)[0];
    let (cmap, _) = parse_cmap(&f.to_unicode);
    assert_eq!(cmap.values().cloned().collect::<String>(), "Ttc");
}

#[test]
fn collection_index_past_the_end_is_refused() {
    assert_eq!(
        Font::from_collection(fixture("fonts/NotoSans-RegularBold.ttc"), 2).unwrap_err(),
        Error::CollectionIndex { index: 2, count: 2 }
    );
}

#[test]
fn single_font_is_a_collection_of_one() {
    assert_eq!(Font::collection_len(&fixture("fonts/NotoSans-Regular.ttf")).unwrap(), 1);
}

#[test]
fn cff_font_info_says_cff() {
    assert!(font("NotoSans-Regular.otf").info().is_cff);
}

fn variable_bytes() -> Vec<u8> {
    fixture("fonts/NotoSans-VF.ttf")
}

/// The VF fixture with its first `fvar` axis's minimum raised above its maximum.
fn inverted_axis() -> Vec<u8> {
    let bytes = variable_bytes();
    let fvar = common::table_offset(&bytes, *b"fvar");
    let axes = usize::from(u16::from_be_bytes([bytes[fvar + 4], bytes[fvar + 5]]));
    // Axis record: tag, minValue (Fixed 16.16), defaultValue, maxValue, flags, nameID.
    let min = 2000i32 << 16;
    patch_table(&bytes, *b"fvar", axes + 4, &min.to_be_bytes())
}

#[test]
fn malformed_axis_range_is_an_error_not_a_panic() {
    let font = Font::from_bytes(inverted_axis()).unwrap();
    let tag = font.info().axes[0].tag;
    assert_eq!(font.instance(&[(tag, 500.0)]).unwrap_err().code(), "invalid-font");
}

#[test]
fn non_finite_axis_value_is_refused() {
    let font = font("NotoSans-VF.ttf");
    assert_eq!(
        font.instance(&[(Tag::WEIGHT, f32::NAN)]).unwrap_err().code(),
        "invalid-axis-value"
    );
}

#[test]
fn infinite_axis_value_is_refused() {
    let font = font("NotoSans-VF.ttf");
    assert!(font.instance(&[(Tag::WEIGHT, f32::INFINITY)]).is_err());
}

#[test]
fn instance_bbox_is_the_union_of_the_instanced_glyph_bounds() {
    let heavy = font("NotoSans-VF.ttf").instance(&[(Tag::WEIGHT, 900.0)]).unwrap();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(heavy).unwrap();
    pdf.line(key, 12.0, "WMgj").unwrap();
    let f = &read_fonts(&pdf.finish().unwrap().0)[0];
    let program = skrifa::FontRef::new(&f.program).unwrap();
    let upem = f32::from(skrifa::raw::TableProvider::head(&program).unwrap().units_per_em());
    let metrics = program.glyph_metrics(Size::unscaled(), LocationRef::default());
    let mut bbox = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for gid in 1..u32::from(skrifa::raw::TableProvider::maxp(&program).unwrap().num_glyphs()) {
        if let Some(b) = metrics.bounds(skrifa::GlyphId::new(gid)) {
            bbox = [
                bbox[0].min(b.x_min),
                bbox[1].min(b.y_min),
                bbox[2].max(b.x_max),
                bbox[3].max(b.y_max),
            ];
        }
    }
    let expected: Vec<i64> = bbox.iter().map(|v| (v * 1000.0 / upem).round() as i64).collect();
    let written: Vec<i64> = f
        .descriptor
        .get("FontBBox")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_int().unwrap())
        .collect();
    assert_eq!(written, expected);
}

#[test]
fn instance_at_the_defaults_is_the_default_instance() {
    let regular = font("NotoSans-VF.ttf");
    let same = regular.instance(&[(Tag::WEIGHT, 400.0), (Tag::WIDTH, 100.0)]).unwrap();
    assert_eq!(
        (same.coordinates().len(), same.info().postscript_name.as_str()),
        (0, regular.info().postscript_name.as_str())
    );
}

#[test]
fn unreadable_collection_reports_the_requested_index() {
    assert_eq!(
        Font::from_collection(b"garbage".to_vec(), 3).unwrap_err(),
        Error::InvalidFont { index: 3 }
    );
}
