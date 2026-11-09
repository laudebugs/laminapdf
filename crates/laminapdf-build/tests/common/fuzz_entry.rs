//! The `markdown` fuzz target's entry point (ADR 0018 tier 9). `fuzz/fuzz_targets/markdown.rs`
//! calls it (through `fuzz/src/lib.rs`, which includes this file), and `tests/regressions.rs`
//! replays every saved crash input (`fixtures/regressions/markdown/`) and the Markdown fixtures
//! through it.

use laminapdf_build::markdown::{MarkdownFonts, MarkdownOptions};
use laminapdf_build::{BuildOptions, Builder, Image, Warning};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_text::Font;

/// A 1 × 1 grey PNG, for an image the Markdown names `image.png`.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x3A, 0x7E, 0x9B, 0x55, 0x00, 0x00, 0x00, 0x0A, 0x49,
    0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x68, 0x00, 0x00, 0x00, 0x82, 0x00, 0x81, 0x77, 0xCD, 0x72, 0xB6, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];
const NOTO: &[u8] = include_bytes!("../../../../fixtures/fonts/NotoSans-Regular.ttf");

/// Lays `data` (read as UTF-8, invalid bytes replaced) out as Markdown twice: in the standard
/// 14 fonts, and in Noto Sans with a contents page and an image. Invariants: no panic, no hang;
/// every failure is a coded error; every warning's offset is inside the Markdown; a document
/// that builds also saves.
pub fn markdown(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    let image = Image::png(PNG).ok();
    for noto in [false, true] {
        let mut b = Builder::new(BuildOptions::new().with_id(*b"lamina-fuzz-mkdn"));
        let mut options = MarkdownOptions::new();
        if noto {
            let Ok(font) = Font::from_bytes(NOTO.to_vec()) else {
                return;
            };
            let Ok(font) = b.font(font) else { return };
            options = options.with_fonts(MarkdownFonts::new(&font)).with_contents(3);
            if let Some(image) = &image {
                options = options.with_image("image.png", image.clone());
            }
        }
        let report = match b.markdown(&text, &options) {
            Ok(report) => report,
            Err(e) => {
                assert!(!e.code().is_empty());
                continue;
            }
        };
        for w in &report.warnings {
            if let Warning::MarkdownHtmlIgnored { offset } | Warning::MarkdownImageMissing { offset } = w {
                assert!(*offset <= text.len(), "{w:?} past the end");
            }
        }
        match b.finish() {
            Ok(mut doc) => {
                let mut out = Vec::new();
                doc.save(&mut out, &SaveOptions::new()).unwrap();
            }
            Err(e) => assert!(!e.code().is_empty()),
        }
    }
}
