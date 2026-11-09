//! Page appearance (todo U2c, ADR 0024 §5) in both backends, on `fixtures/appearance/`
//! (`tools/fixtures/appearance.py`): an opaque image keeps its pixels in every mode, a soft-masked
//! image keeps its opaque samples, annotations keep their colours while the text beside them is
//! recoloured, Dark body text reads at 7:1 or better, and every mode of every file matches the
//! backend's own golden in `fixtures/golden/render-appearance/<backend>/` (SSIM ≥ 0.99; written by
//! `cargo xtask render-golden --update`). Ignored: needs the PDFium library.
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

mod common;

use common::{APPEARANCE_FILES, APPEARANCES, appearance_dir, backends, points_to_pixels, render_appearance};
use laminapdf_render::{DARK, PageAppearance};
use laminapdf_test_fixtures::image::{contrast, read_png, ssim};

/// The pixels of a page-space rectangle (points, top-left origin) of `stem` in `appearance`.
fn area(
    backend: &dyn laminapdf_render::RenderBackend,
    stem: &str,
    appearance: PageAppearance,
    rect: [f32; 4],
    inset: u32,
) -> Vec<[u8; 4]> {
    render_appearance(backend, stem, appearance).pixels_in(points_to_pixels(rect, inset))
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn an_opaque_image_is_pixel_identical_in_every_mode() {
    // The photo fills x 30–270, y 10–160; one pixel in from its edges.
    let rect = [30.0, 10.0, 270.0, 160.0];
    for backend in backends() {
        let photo = area(backend.as_ref(), "photo", PageAppearance::Original, rect, 1);
        for (name, mode) in APPEARANCES {
            assert_eq!(
                area(backend.as_ref(), "photo", mode, rect, 1),
                photo,
                "{} {name}",
                backend.name()
            );
        }
    }
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn a_soft_masked_images_opaque_samples_bypass_recolouring() {
    // The masked disc (x 40–100, y 110–170), around its centre.
    let rect = [66.0, 136.0, 74.0, 144.0];
    for backend in backends() {
        let centre = area(backend.as_ref(), "mixed", PageAppearance::Original, rect, 0);
        assert_eq!(
            area(backend.as_ref(), "mixed", PageAppearance::Dark, rect, 0),
            centre,
            "{}",
            backend.name()
        );
    }
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn the_paper_around_a_soft_masked_image_is_recoloured() {
    // The disc's box corner (x 40–44, y 110–114): inside the image, outside the disc.
    let paper = [DARK.paper.r, DARK.paper.g, DARK.paper.b, 255];
    for backend in backends() {
        let corner = area(
            backend.as_ref(),
            "mixed",
            PageAppearance::Dark,
            [40.0, 110.0, 43.0, 113.0],
            0,
        );
        assert!(corner.iter().all(|&p| p == paper), "{}: {corner:?}", backend.name());
    }
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn annotations_keep_their_colours() {
    // The square's fill inside its border (x 200–280, y 120–170) and the highlight over its words.
    for backend in backends() {
        for rect in [[204.0, 124.0, 276.0, 166.0], [18.0, 36.0, 168.0, 54.0]] {
            let kept = area(backend.as_ref(), "annotated", PageAppearance::Original, rect, 1);
            for (name, mode) in APPEARANCES {
                let got = area(backend.as_ref(), "annotated", mode, rect, 1);
                assert_eq!(got, kept, "{} {name} {rect:?}", backend.name());
            }
        }
    }
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn text_beside_annotations_is_recoloured() {
    let rect = [18.0, 76.0, 200.0, 94.0];
    for backend in backends() {
        let original = area(backend.as_ref(), "annotated", PageAppearance::Original, rect, 0);
        assert_ne!(
            area(backend.as_ref(), "annotated", PageAppearance::Dark, rect, 0),
            original,
            "{}",
            backend.name()
        );
    }
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn dark_body_text_reads_at_seven_to_one() {
    let paper = [DARK.paper.r, DARK.paper.g, DARK.paper.b, 255];
    for backend in backends() {
        // The body line, y 40–58: its glyph pixels (any that stand out from the paper), of which
        // the brightest quarter, the strokes' cores rather than their anti-aliased edges, must
        // all reach 7:1.
        let mut glyphs: Vec<f64> = area(
            backend.as_ref(),
            "text",
            PageAppearance::Dark,
            [18.0, 38.0, 200.0, 58.0],
            0,
        )
        .into_iter()
        .map(|p| contrast(p, paper))
        .filter(|&c| c > 1.5)
        .collect();
        glyphs.sort_by(|a, b| b.total_cmp(a));
        let quarter = glyphs.get(glyphs.len() / 4).copied().unwrap_or(0.0);
        assert!(
            quarter >= 7.0,
            "{}: body text contrast {quarter:.2} at its brightest quarter",
            backend.name()
        );
    }
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn every_mode_of_every_file_matches_its_golden() {
    let mut failures = Vec::new();
    for backend in backends() {
        for stem in APPEARANCE_FILES {
            for (name, mode) in APPEARANCES {
                let path = appearance_dir(backend.name()).join(format!("{stem}-{name}.png"));
                let score = ssim(&render_appearance(backend.as_ref(), stem, mode), &read_png(&path));
                if score < 0.99 {
                    failures.push(format!("{} {stem} {name}: SSIM {score:.4}", backend.name()));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
