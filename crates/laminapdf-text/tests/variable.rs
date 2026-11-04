//! Variable-font instances and collection members (research 03 §7.9; todo B4a).
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{TestPdf, font, name_of, parse_cmap, read_fonts};
use laminapdf_test_fixtures::fixture;
use laminapdf_text::{Error, Font, Tag};

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
