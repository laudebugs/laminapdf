//! `/RC` rich text through the public API: the `rich_text` fuzz target's entry over its seeds and
//! past crashes, and a few shapes other apps write.

mod common;

use common::rich_text_fuzz_entry::rich_text;
use laminapdf_ops::{RichText, TextSpan};
use laminapdf_test_fixtures::fixtures_dir;

#[test]
fn every_seed_and_regression_round_trips_through_the_subset() {
    for dir in ["../fuzz/seeds/rich_text", "regressions/rich_text"] {
        let Ok(entries) = std::fs::read_dir(fixtures_dir().join(dir)) else {
            continue;
        };
        for entry in entries {
            rich_text(&std::fs::read(entry.unwrap().path()).unwrap());
        }
    }
}

#[test]
fn nested_styles_combine() {
    let (text, simplified) = RichText::from_xhtml("<body><p><b>a <i>b</i></b></p></body>");
    let expected = RichText {
        paragraphs: vec![vec![
            TextSpan::plain("a ").with_bold(true),
            TextSpan::plain("b").with_bold(true).with_italic(true),
        ]],
    };
    assert_eq!((text, simplified), (expected, false));
}

#[test]
fn unclosed_tags_and_stray_brackets_read_as_far_as_they_go() {
    for xhtml in [
        "<body><p>open",
        "<p>a</b></i>b",
        "a > b",
        "<",
        "&#xFFFFFFFF; &bogus;",
        "",
    ] {
        rich_text(xhtml.as_bytes());
    }
}
