//! The PDFium adapter through the public trait. Every test needs the PDFium library, so all are
//! `#[ignore]`d: `cargo xtask render-golden` runs them after `fetch-pdfium` (or
//! `LAMINA_PDFIUM_DIR=… cargo test -p lamina-render-pdfium -- --ignored`).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "tests: a panic is a test failure; fixture offsets and pixel counts are small"
)]

mod common;

use std::sync::Arc;

use common::{fixture, open, pdfium, render};
use lamina_render::{
    ByteSource, CancelToken, PixelRect, RenderBackend, RenderError, RenderFlags, RenderOpenOptions, RenderRequest,
    SourceError, ViewRotation,
};

mod errors {
    use super::*;

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn a_page_past_the_last_is_out_of_range() {
        let doc = open(&pdfium(), fixture("corpus/hello.pdf"), &RenderOpenOptions::new()).unwrap();
        assert_eq!(
            doc.page(doc.page_count()).unwrap_err().code(),
            "page-index-out-of-range"
        );
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn bytes_that_are_not_a_pdf_fail_to_open() {
        let err = open(&pdfium(), b"not a pdf at all".to_vec(), &RenderOpenOptions::new())
            .err()
            .unwrap();
        assert_eq!(err.code(), "render-failed");
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn an_encrypted_file_without_its_password_needs_one() {
        let err = open(
            &pdfium(),
            fixture("encrypted/r4-aes-128-user.pdf"),
            &RenderOpenOptions::new(),
        )
        .err()
        .unwrap();
        assert_eq!(err, RenderError::PasswordRequired);
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn an_encrypted_file_with_the_wrong_password_is_refused() {
        let options = RenderOpenOptions::new().with_password("nope");
        let err = open(&pdfium(), fixture("encrypted/r4-aes-128-user.pdf"), &options)
            .err()
            .unwrap();
        assert_eq!(err, RenderError::WrongPassword);
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn an_encrypted_file_opens_with_its_user_password() {
        let options = RenderOpenOptions::new().with_password("user-pw");
        let doc = open(&pdfium(), fixture("encrypted/r4-aes-128-user.pdf"), &options).unwrap();
        assert!(render(doc.as_ref(), &RenderRequest::page(0, 0.25)).is_ok());
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn an_oversized_request_is_refused_before_rendering() {
        let doc = open(&pdfium(), fixture("corpus/hello.pdf"), &RenderOpenOptions::new()).unwrap();
        let err = doc
            .render(&RenderRequest::page(0, 1000.0), &mut [], &CancelToken::new())
            .unwrap_err();
        assert_eq!(err.code(), "render-too-large");
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn a_cancelled_token_stops_the_render() {
        let doc = open(&pdfium(), fixture("corpus/hello.pdf"), &RenderOpenOptions::new()).unwrap();
        let req = RenderRequest::page(0, 0.25);
        let mut out = vec![0; doc.plan(&req).unwrap().byte_len()];
        let cancel = CancelToken::new();
        cancel.cancel();
        assert_eq!(doc.render(&req, &mut out, &cancel).unwrap_err(), RenderError::Cancelled);
    }

    /// A source with the file's first and last 1 KiB only.
    struct HeadAndTail(Vec<u8>);

    impl ByteSource for HeadAndTail {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
            let len = self.0.len() as u64;
            let end = (offset + buf.len() as u64).min(len);
            let available = |at: u64| at < 1024 || at >= len.saturating_sub(1024);
            if !(available(offset) && available(end.saturating_sub(1))) {
                return Err(SourceError::Pending {
                    offset,
                    len: end - offset,
                });
            }
            let n = (end - offset) as usize;
            buf[..n].copy_from_slice(&self.0[offset as usize..end as usize]);
            Ok(n)
        }
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn missing_bytes_are_data_pending_with_their_ranges() {
        let bytes = fixture("corpus/text-example.pdf");
        assert!(bytes.len() > 4096, "the fixture must be larger than head + tail");
        let source: Arc<dyn ByteSource> = Arc::new(HeadAndTail(bytes));
        let outcome = pdfium()
            .open(source, &RenderOpenOptions::new())
            .and_then(|doc| render(doc.as_ref(), &RenderRequest::page(0, 0.25)).map(|_| ()));
        match outcome {
            Err(RenderError::DataPending { ranges }) => assert_ne!(ranges, Vec::new()),
            other => panic!("expected data-pending, got {:?}", other.err()),
        }
    }
}

mod geometry {
    use super::*;

    const SCALE: f32 = 0.5;

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn tiles_stitch_back_into_the_whole_page() {
        let doc = open(&pdfium(), fixture("corpus/imaging-demo.pdf"), &RenderOpenOptions::new()).unwrap();
        let whole = render(doc.as_ref(), &RenderRequest::page(0, SCALE)).unwrap();
        let (w, h) = (whole.width, whole.height);
        let (tw, th) = (w.div_ceil(2), h.div_ceil(2));
        let mut stitched = vec![0u8; whole.rgba.len()];
        for (x, y) in [(0, 0), (tw, 0), (0, th), (tw, th)] {
            let clip = PixelRect {
                x,
                y,
                width: tw.min(w - x),
                height: th.min(h - y),
            };
            let tile = render(doc.as_ref(), &RenderRequest::page(0, SCALE).with_clip(clip)).unwrap();
            for row in 0..tile.height {
                let src = (row * tile.width * 4) as usize;
                let dst = (((y + row) * w + x) * 4) as usize;
                let n = (tile.width * 4) as usize;
                stitched[dst..dst + n].copy_from_slice(&tile.rgba[src..src + n]);
            }
        }
        assert_eq!(differing(&whole.rgba, &stitched), 0, "tiles differ from the whole page");
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn a_quarter_turn_equals_the_upright_render_rotated() {
        let doc = open(&pdfium(), fixture("corpus/imaging-demo.pdf"), &RenderOpenOptions::new()).unwrap();
        let up = render(doc.as_ref(), &RenderRequest::page(0, SCALE)).unwrap();
        let turned = render(
            doc.as_ref(),
            &RenderRequest::page(0, SCALE).with_rotation(ViewRotation::Clockwise90),
        )
        .unwrap();
        assert_eq!((turned.width, turned.height), (up.height, up.width));
        let mut expected = vec![0u8; up.rgba.len()];
        for y in 0..up.height {
            for x in 0..up.width {
                let src = ((y * up.width + x) * 4) as usize;
                let dst = ((x * turned.width + (up.height - 1 - y)) * 4) as usize;
                expected[dst..dst + 4].copy_from_slice(&up.rgba[src..src + 4]);
            }
        }
        let share = differing(&expected, &turned.rgba) as f64 / f64::from(up.width * up.height);
        assert!(share < 0.02, "{:.2} % of pixels differ", share * 100.0);
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn both_backends_and_lamina_agree_on_every_page_size() {
        for file in common::corpus() {
            let bytes = fixture(&format!("corpus/{file}"));
            let lamina = lamina::Document::open_bytes(bytes.clone(), lamina::OpenOptions::default()).unwrap();
            for backend in common::backends() {
                if common::skipped(backend.name()).contains_key(&file) {
                    continue;
                }
                let doc = open(backend.as_ref(), bytes.clone(), &RenderOpenOptions::new()).unwrap();
                assert_eq!(
                    doc.page_count(),
                    lamina.page_count().unwrap(),
                    "{} {file}",
                    backend.name()
                );
                for i in 0..doc.page_count() {
                    let page = doc.page(i).unwrap();
                    let info = lamina.page_info(i).unwrap();
                    let close = |a: f32, b: f64| (f64::from(a) - b).abs() < 0.5;
                    assert!(
                        close(page.width, info.width) && close(page.height, info.height),
                        "{} {file} page {}: {}×{} vs lamina {}×{}",
                        backend.name(),
                        i + 1,
                        page.width,
                        page.height,
                        info.width,
                        info.height
                    );
                    assert_eq!(page.rotate, info.rotation, "{} {file} page {}", backend.name(), i + 1);
                }
            }
        }
    }

    /// Widgets off makes pdfium-render take its matrix path: tiles, scale and rotation must
    /// still match the form path's output (no widgets on this page, so the pixels agree).
    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn without_widgets_tiles_and_rotations_match_the_form_path() {
        let doc = open(&pdfium(), fixture("corpus/imaging-demo.pdf"), &RenderOpenOptions::new()).unwrap();
        let off = RenderFlags {
            form_widgets: false,
            ..RenderFlags::default()
        };
        for rotation in [
            ViewRotation::None,
            ViewRotation::Clockwise90,
            ViewRotation::Half,
            ViewRotation::Clockwise270,
        ] {
            let base = RenderRequest::page(0, 2.0).with_rotation(rotation);
            let with = render(doc.as_ref(), &base).unwrap();
            let clip = PixelRect {
                x: with.width / 3,
                y: with.height / 4,
                width: 256,
                height: 192,
            };
            let tile_with = render(doc.as_ref(), &base.with_clip(clip)).unwrap();
            let tile_without = render(doc.as_ref(), &base.with_clip(clip).with_flags(off)).unwrap();
            let share = differing(&tile_with.rgba, &tile_without.rgba) as f64 / f64::from(clip.width * clip.height);
            assert!(share < 0.01, "{rotation:?}: {:.2} % of the tile differs", share * 100.0);
        }
    }

    /// A source that returns at most 100 bytes per read.
    struct Dribble(Vec<u8>);

    impl ByteSource for Dribble {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
            let start = (offset as usize).min(self.0.len());
            let n = buf.len().min(100).min(self.0.len() - start);
            buf[..n].copy_from_slice(&self.0[start..start + n]);
            Ok(n)
        }
    }

    #[test]
    #[ignore = "needs the PDFium library: cargo xtask render-golden"]
    fn short_reads_render_like_a_whole_buffer() {
        let bytes = fixture("corpus/text-example.pdf");
        let whole = render(
            open(&pdfium(), bytes.clone(), &RenderOpenOptions::new())
                .unwrap()
                .as_ref(),
            &RenderRequest::page(0, 1.0),
        )
        .unwrap();
        let source: Arc<dyn ByteSource> = Arc::new(Dribble(bytes));
        let doc = pdfium().open(source, &RenderOpenOptions::new()).unwrap();
        let dribbled = render(doc.as_ref(), &RenderRequest::page(0, 1.0)).unwrap();
        assert!(whole.rgba == dribbled.rgba, "short reads changed the pixels");
    }

    fn differing(a: &[u8], b: &[u8]) -> usize {
        a.chunks(4)
            .zip(b.chunks(4))
            .filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
            .count()
    }
}
