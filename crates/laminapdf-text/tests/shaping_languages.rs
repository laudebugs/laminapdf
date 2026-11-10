//! Language tags that aren't BCP 47 text shape like an unknown tag, never panic.
//!
//! HarfRust 0.8 compares tags byte by byte with `str` slices and panics when a cut falls inside a
//! multi-byte character. The `markdown` fuzz target found it with front matter `lang: cm` followed
//! by an invalid byte, read as U+FFFD (docs/bugs/markdown-lang-not-ascii/). The sweep puts 2-, 3-
//! and 4-byte characters at every position of tags shaped like the ones HarfRust's tables match
//! on, and checks each one shapes exactly as with no language at all. Well-formed tags still reach
//! HarfRust: the shaping goldens (`shaping_goldens.rs`) shape with `ar`, `he`, `hi`, `zh` and `en`.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use laminapdf_text::{HarfRust, ShapeOptions, ShapedRun, Shaper};

/// Tags, and prefixes of tags, that HarfRust's language tables compare against.
const TAGS: [&str; 10] = [
    "cm",
    "zh-",
    "zh-Hant",
    "cmn",
    "art-lojban",
    "x-hbo",
    "i-",
    "ja",
    "sr-Latn",
    "en",
];

/// One character of each UTF-8 length past ASCII: é (2), U+FFFD (3), 😀 (4).
const WIDE: [char; 3] = ['é', '\u{FFFD}', '😀'];

/// Every tag with every wide character inserted at every character boundary.
fn bad_tags() -> Vec<String> {
    let mut tags = Vec::new();
    for tag in TAGS {
        for wide in WIDE {
            for at in 0..=tag.len() {
                let (head, tail) = tag.split_at(at);
                tags.push(format!("{head}{wide}{tail}"));
            }
        }
    }
    tags
}

fn shape(language: Option<&str>) -> ShapedRun {
    let font = common::font("NotoSans-Regular.ttf");
    let mut options = ShapeOptions::new();
    if let Some(language) = language {
        options = options.with_language(language);
    }
    HarfRust.shape(&font, "office affine", &options).unwrap()
}

#[test]
fn a_tag_with_non_ascii_text_should_shape_like_no_tag() {
    let plain = shape(None);
    let tags = bad_tags();
    assert_eq!(tags.len(), 159);
    for tag in &tags {
        assert_eq!(shape(Some(tag)).glyphs, plain.glyphs, "language {tag:?}");
    }
}
