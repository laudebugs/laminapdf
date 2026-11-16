//! The hayro adapter through the public trait: typed errors for hostile requests and files, and
//! tile geometry that agrees with whole-page renders (tiles stitch back into the page; a view
//! rotation equals the upright render rotated). Small pages at low scale, so they run in the
//! fast lane; the render goldens are laminapdf-render-pdfium's ignored tests.
#![cfg(feature = "hayro")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    reason = "tests: a panic is a test failure; pixel counts are small"
)]

use std::sync::Arc;

use laminapdf_render::{
    ByteSource, CancelToken, HayroBackend, PixelRect, RenderBackend, RenderDocument, RenderError, RenderOpenOptions,
    RenderRequest, Rotation,
};

use laminapdf_test_fixtures::fixture;

fn open(path: &str, options: &RenderOpenOptions) -> Result<Box<dyn RenderDocument>, RenderError> {
    let source: Arc<dyn ByteSource> = Arc::new(fixture(path));
    HayroBackend::new().open(source, options)
}

/// Renders `req` into a fresh buffer.
fn render(doc: &dyn RenderDocument, req: &RenderRequest) -> (u32, u32, Vec<u8>) {
    let plan = doc.plan(req).unwrap();
    let mut out = vec![0; plan.byte_len()];
    doc.render(req, &mut out, &CancelToken::new()).unwrap();
    (plan.tile.width, plan.tile.height, out)
}

mod errors {
    use super::*;

    #[test]
    fn a_page_past_the_last_is_out_of_range() {
        let doc = open("corpus/hello.pdf", &RenderOpenOptions::new()).unwrap();
        let err = doc.page(doc.page_count()).unwrap_err();
        assert_eq!(err.code(), "page-index-out-of-range");
    }

    #[test]
    fn bytes_that_are_not_a_pdf_fail_to_open() {
        let source: Arc<dyn ByteSource> = Arc::new(b"not a pdf at all".to_vec());
        let err = HayroBackend::new()
            .open(source, &RenderOpenOptions::new())
            .err()
            .unwrap();
        assert_eq!(err.code(), "render-failed");
    }

    #[test]
    fn an_encrypted_file_without_its_password_needs_one() {
        let err = open("encrypted/r4-aes-128-user.pdf", &RenderOpenOptions::new())
            .err()
            .unwrap();
        assert_eq!(err, RenderError::PasswordRequired);
    }

    #[test]
    fn an_encrypted_file_with_the_wrong_password_is_refused() {
        let err = open(
            "encrypted/r4-aes-128-user.pdf",
            &RenderOpenOptions::new().with_password("nope"),
        )
        .err()
        .unwrap();
        assert_eq!(err, RenderError::WrongPassword);
    }

    #[test]
    fn an_encrypted_file_opens_with_its_user_password() {
        let doc = open(
            "encrypted/r4-aes-128-user.pdf",
            &RenderOpenOptions::new().with_password("user-pw"),
        )
        .unwrap();
        assert!(doc.page_count() > 0);
    }

    #[test]
    fn an_oversized_request_is_refused_before_rendering() {
        let doc = open("corpus/hello.pdf", &RenderOpenOptions::new()).unwrap();
        let mut out = Vec::new();
        let err = doc
            .render(&RenderRequest::page(0, 1000.0), &mut out, &CancelToken::new())
            .unwrap_err();
        assert_eq!(err.code(), "render-too-large");
    }

    #[test]
    fn a_wrong_buffer_size_is_a_usage_error() {
        let doc = open("corpus/hello.pdf", &RenderOpenOptions::new()).unwrap();
        let mut out = vec![0; 3];
        let err = doc
            .render(&RenderRequest::page(0, 0.25), &mut out, &CancelToken::new())
            .unwrap_err();
        assert_eq!(err.code(), "invalid-argument");
    }

    #[test]
    fn a_cancelled_token_stops_the_render() {
        let doc = open("corpus/hello.pdf", &RenderOpenOptions::new()).unwrap();
        let req = RenderRequest::page(0, 0.25);
        let mut out = vec![0; doc.plan(&req).unwrap().byte_len()];
        let cancel = CancelToken::new();
        cancel.cancel();
        assert_eq!(doc.render(&req, &mut out, &cancel).unwrap_err(), RenderError::Cancelled);
    }
}

mod geometry {
    use super::*;

    const SCALE: f32 = 0.5;

    #[test]
    fn tiles_stitch_back_into_the_whole_page() {
        let doc = open("corpus/imaging-demo.pdf", &RenderOpenOptions::new()).unwrap();
        let (w, h, whole) = render(doc.as_ref(), &RenderRequest::page(0, SCALE));
        let (tw, th) = (w.div_ceil(2), h.div_ceil(2));
        let mut stitched = vec![0u8; whole.len()];
        for (x, y) in [(0, 0), (tw, 0), (0, th), (tw, th)] {
            let clip = PixelRect {
                x,
                y,
                width: tw.min(w - x),
                height: th.min(h - y),
            };
            let (cw, ch, tile) = render(doc.as_ref(), &RenderRequest::page(0, SCALE).with_clip(clip));
            for row in 0..ch {
                let src = (row * cw * 4) as usize;
                let dst = (((y + row) * w + x) * 4) as usize;
                stitched[dst..dst + (cw * 4) as usize].copy_from_slice(&tile[src..src + (cw * 4) as usize]);
            }
        }
        assert_eq!(differing(&whole, &stitched), 0, "tiles differ from the whole page");
    }

    #[test]
    fn a_quarter_turn_equals_the_upright_render_rotated() {
        let doc = open("corpus/imaging-demo.pdf", &RenderOpenOptions::new()).unwrap();
        let (w, h, upright) = render(doc.as_ref(), &RenderRequest::page(0, SCALE));
        let (rw, rh, turned) = render(
            doc.as_ref(),
            &RenderRequest::page(0, SCALE).with_rotation(Rotation::Clockwise90),
        );
        assert_eq!((rw, rh), (h, w));
        // Clockwise: the upright pixel (x, y) lands at (h - 1 - y, x).
        let mut expected = vec![0u8; upright.len()];
        for y in 0..h {
            for x in 0..w {
                let src = ((y * w + x) * 4) as usize;
                let dst = ((x * rw + (h - 1 - y)) * 4) as usize;
                expected[dst..dst + 4].copy_from_slice(&upright[src..src + 4]);
            }
        }
        // Anti-aliasing samples a rotated grid slightly differently: allow edge pixels only.
        let share = differing(&expected, &turned) as f64 / f64::from(w * h);
        assert!(share < 0.02, "{:.2} % of pixels differ", share * 100.0);
    }

    /// Pixels whose channels differ by more than 8.
    fn differing(a: &[u8], b: &[u8]) -> usize {
        a.chunks(4)
            .zip(b.chunks(4))
            .filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
            .count()
    }
}

/// `hidden_annotations` names annotations by their place in `/Annots`: of two squares with the
/// same `/NM` (red at the left, blue at the right), only the one named is left out.
mod hidden {
    use super::*;

    /// The RGB of the pixel at (`x`, `y`) of a 300 pt page drawn at scale 1.
    fn rgb(pixels: &[u8], x: usize, y: usize) -> [u8; 3] {
        let at = (y * 300 + x) * 4;
        [pixels[at], pixels[at + 1], pixels[at + 2]]
    }

    const RED: [u8; 3] = [255, 0, 0];
    const BLUE: [u8; 3] = [0, 0, 255];
    const WHITE: [u8; 3] = [255, 255, 255];

    fn squares(hidden: &[u32]) -> ([u8; 3], [u8; 3]) {
        let doc = open("annotations/same-name.pdf", &RenderOpenOptions::new()).unwrap();
        let req = RenderRequest::page(0, 1.0).with_hidden_annotations(hidden.iter().copied());
        let (_, _, pixels) = render(doc.as_ref(), &req);
        (rgb(&pixels, 70, 150), rgb(&pixels, 230, 150))
    }

    #[test]
    fn nothing_hidden_draws_both_squares() {
        assert_eq!(squares(&[]), (RED, BLUE));
    }

    #[test]
    fn hiding_the_second_leaves_the_first_with_the_same_name() {
        assert_eq!(squares(&[1]), (RED, WHITE));
    }

    #[test]
    fn hiding_the_first_leaves_the_second_with_the_same_name() {
        assert_eq!(squares(&[0]), (WHITE, BLUE));
    }

    #[test]
    fn a_place_past_the_end_hides_nothing() {
        assert_eq!(squares(&[7]), (RED, BLUE));
    }

    #[test]
    fn a_hidden_tile_matches_the_same_part_of_the_hidden_page() {
        let doc = open("annotations/same-name.pdf", &RenderOpenOptions::new()).unwrap();
        let clip = PixelRect {
            x: 150,
            y: 90,
            width: 120,
            height: 120,
        };
        let req = RenderRequest::page(0, 1.0).with_hidden_annotations([1]);
        let (_, _, whole) = render(doc.as_ref(), &req);
        let (w, _, tile) = render(doc.as_ref(), &req.with_clip(clip));
        let row = |y: usize| &tile[y * w as usize * 4..(y + 1) * w as usize * 4];
        for y in 0..clip.height as usize {
            let from = ((clip.y as usize + y) * 300 + clip.x as usize) * 4;
            assert_eq!(row(y), &whole[from..from + w as usize * 4], "row {y}");
        }
    }
}
