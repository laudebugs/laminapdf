//! Fixture fonts for unit tests, compiled in (core crates may not read files).

use crate::font::Font;

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/", $name)).as_slice()
    };
}

/// The fixture font `name` from `fixtures/fonts/`.
pub(crate) fn font(name: &str) -> Font {
    let bytes: &'static [u8] = match name {
        "NotoSans-Regular.ttf" => fixture!("NotoSans-Regular.ttf"),
        "NotoSansArabic-Regular.ttf" => fixture!("NotoSansArabic-Regular.ttf"),
        "NotoSansHebrew-Regular.ttf" => fixture!("NotoSansHebrew-Regular.ttf"),
        "NotoSansDevanagari-Regular.ttf" => fixture!("NotoSansDevanagari-Regular.ttf"),
        other => panic!("no fixture font {other}"),
    };
    Font::from_bytes(bytes).unwrap()
}
