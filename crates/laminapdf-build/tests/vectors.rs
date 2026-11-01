//! Paths, colour, opacity and the page frame, read back with Lamina PDF's reader and
//! `laminapdf-content`: what is drawn in page space lands where ADR 0027 says.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, record, reopen, save};
use laminapdf_build::{
    BuildOptions, Builder, Color, FillRule, IccProfile, Matrix, PageSize, PdfVersion, Rotation, StrokeStyle,
};
use laminapdf_cos::Object;
use laminapdf_model::{PointPt, Pt, RectPt, UserSpacePoint};

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

/// One page of `size` drawn by `draw`, saved and reopened.
fn one_page(size: PageSize, draw: impl FnOnce(&mut laminapdf_build::PageBuilder<'_>)) -> laminapdf_model::Document {
    let mut b = Builder::new(BuildOptions::default());
    let mut page = b.page(size);
    draw(&mut page);
    page.finish().unwrap();
    reopen(&save(b.finish().unwrap()))
}

#[test]
fn a_rect_from_the_top_left_lands_at_the_top_of_user_space() {
    let doc = one_page(PageSize::LETTER, |page| {
        page.surface()
            .path(|p| {
                p.rect(rect(72.0, 72.0, 200.0, 28.0));
            })
            .fill(FillRule::NonZero);
    });
    // ADR 0027 §3's worked example: page-space [72 72 272 100] is user space [72 692 272 720].
    assert_eq!(record(&doc, 0).painted[0].points, [(72.0, 720.0), (272.0, 692.0)]);
}

#[test]
fn every_rotation_maps_page_space_back_through_the_readers_frame() {
    for rotate in [
        Rotation::None,
        Rotation::Clockwise90,
        Rotation::Half,
        Rotation::Clockwise270,
    ] {
        let doc = one_page(PageSize::A5, |page| {
            page.rotate(rotate)
                .user_unit(1.5)
                .crop_box(rect(20.0, 30.0, 300.0, 400.0));
            page.surface()
                .path(|p| {
                    p.move_to(PointPt::from_points(10.0, 25.0))
                        .line_to(PointPt::from_points(100.0, 50.0));
                })
                .stroke(&StrokeStyle::default());
        });
        let frame = doc.pages().unwrap().get(0).unwrap().frame().unwrap();
        let back: Vec<(f64, f64)> = record(&doc, 0).painted[0]
            .points
            .iter()
            .map(|&(x, y)| {
                let p = frame.to_page(UserSpacePoint::new(x, y));
                ((p.x.get() * 100.0).round() / 100.0, (p.y.get() * 100.0).round() / 100.0)
            })
            .collect();
        assert_eq!(back, [(10.0, 25.0), (100.0, 50.0)], "{rotate:?}");
    }
}

#[test]
fn the_written_frame_is_the_requested_page() {
    let doc = one_page(PageSize::A4, |page| {
        page.rotate(Rotation::Clockwise270).user_unit(2.0);
    });
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let frame = page.frame().unwrap();
    assert_eq!(
        (
            frame.width().get().round(),
            frame.height().get().round(),
            page.user_unit().unwrap()
        ),
        (842.0, 595.0, 2.0)
    );
}

#[test]
fn lengths_are_physical_points_under_a_user_unit() {
    let doc = one_page(PageSize::LETTER, |page| {
        page.user_unit(2.0);
        page.surface()
            .path(|p| {
                p.move_to(PointPt::from_points(0.0, 0.0))
                    .line_to(PointPt::from_points(10.0, 0.0));
            })
            .stroke(&StrokeStyle {
                width: Pt::new(4.0),
                ..StrokeStyle::default()
            });
    });
    assert_eq!(record(&doc, 0).painted[0].line_width, 2.0);
}

#[test]
fn transforms_are_in_page_space() {
    // A unit square scaled by 10 and moved 50 pt right and down: page space [50 50 60 60].
    let doc = one_page(PageSize::LETTER, |page| {
        page.surface()
            .transform(Matrix::translate(Pt::new(50.0), Pt::new(50.0)).after(&Matrix::scale(10.0, 10.0)))
            .path(|p| {
                p.rect(rect(0.0, 0.0, 1.0, 1.0));
            })
            .fill(FillRule::NonZero);
    });
    let points = &record(&doc, 0).painted[0].points;
    let (xs, ys): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
    let span = |v: &[f64]| {
        (
            v.iter().copied().fold(f64::MAX, f64::min),
            v.iter().copied().fold(f64::MIN, f64::max),
        )
    };
    assert_eq!((span(&xs), span(&ys)), ((50.0, 60.0), (732.0, 742.0)));
}

#[test]
fn unbalanced_saves_and_restores_still_give_a_balanced_stream() {
    let doc = one_page(PageSize::A6, |page| {
        page.surface().restore().save().save().fill_color(Color::gray(0.5));
    });
    assert_eq!(record(&doc, 0).warnings, Vec::<&str>::new());
}

#[test]
fn painting_an_empty_path_draws_nothing() {
    let doc = one_page(PageSize::A6, |page| {
        page.surface()
            .path(|_| {})
            .fill(FillRule::NonZero)
            .path(|_| {})
            .stroke(&StrokeStyle::default());
    });
    assert_eq!(record(&doc, 0).painted, vec![]);
}

#[test]
fn opacity_applies_to_the_fill_and_is_written_once_per_document() {
    let mut b = Builder::new(BuildOptions::default());
    for _ in 0..2 {
        let mut page = b.page(PageSize::A6);
        page.surface()
            .opacity(0.25, 1.0)
            .path(|p| {
                p.rect(rect(10.0, 10.0, 10.0, 10.0));
            })
            .fill(FillRule::NonZero);
        page.finish().unwrap();
    }
    let doc = reopen(&save(b.finish().unwrap()));
    let refs: Vec<Object> = doc
        .pages()
        .unwrap()
        .iter()
        .map(|p| {
            let res = p.resources().unwrap();
            let gs = res.get("ExtGState").unwrap().as_dict().unwrap();
            gs.get("GS1").unwrap().clone()
        })
        .collect();
    assert_eq!(
        (refs[0] == refs[1], record(&doc, 1).painted[0].fill_alpha),
        (true, 0.25)
    );
}

#[test]
fn an_icc_colour_uses_one_iccbased_space_with_n() {
    let profile = IccProfile::new(fixture("images/gray-gamma22.icc")).unwrap();
    let doc = one_page(PageSize::A6, |page| {
        page.surface()
            .fill_color(Color::icc(&profile, &[0.5]).unwrap())
            .path(|p| {
                p.rect(rect(10.0, 10.0, 10.0, 10.0));
            })
            .fill(FillRule::NonZero);
    });
    let pages = doc.pages().unwrap();
    let res = pages.get(0).unwrap().resources().unwrap();
    let cs = res.get("ColorSpace").unwrap().as_dict().unwrap().get("CS1").unwrap();
    let Object::Ref(stream) = cs.as_array().unwrap()[1] else {
        panic!("an indirect profile")
    };
    let Object::Stream(stream) = &*doc.resolve(stream).unwrap() else {
        panic!("a stream")
    };
    assert_eq!(
        (stream.dict.get_int("N"), record(&doc, 0).painted[0].fill.clone()),
        (Some(1), vec![0.5])
    );
}

#[test]
fn an_invalid_user_unit_is_refused() {
    let mut b = Builder::new(BuildOptions::default());
    let mut page = b.page(PageSize::A4);
    page.user_unit(0.0);
    assert_eq!(page.finish().unwrap_err().code(), "invalid-geometry");
}

#[test]
fn a_page_without_area_is_refused() {
    let mut b = Builder::new(BuildOptions::default());
    let page = b.page(PageSize::new(Pt::new(0.0), Pt::new(100.0)));
    assert_eq!(page.finish().unwrap_err().code(), "invalid-geometry");
}

#[test]
fn a_dropped_page_is_not_written() {
    let mut b = Builder::new(BuildOptions::default());
    drop(b.page(PageSize::A4));
    b.page(PageSize::A5).finish().unwrap();
    assert_eq!(b.finish().unwrap().pages().unwrap().len(), 1);
}

#[test]
fn the_header_has_the_requested_version_and_the_trailer_the_given_id() {
    let mut b = Builder::new(BuildOptions::default().with_version(PdfVersion::V2_0).with_id([9; 16]));
    b.page(PageSize::A6).finish().unwrap();
    let bytes = save(b.finish().unwrap());
    let doc = reopen(&bytes);
    let id = doc.trailer().get("ID").unwrap().as_array().unwrap()[0].clone();
    assert_eq!(
        (&bytes[..8], id),
        (
            &b"%PDF-2.0"[..],
            Object::String(laminapdf_cos::PdfString::hex(vec![9; 16]))
        )
    );
}

#[test]
fn a_new_document_saves_as_a_full_rewrite_with_the_default_options() {
    use laminapdf_save::Save as _;
    let mut b = Builder::new(BuildOptions::default());
    b.page(PageSize::A6).finish().unwrap();
    let mut doc = b.finish().unwrap();
    let mut out = Vec::new();
    let report = doc.save(&mut out, &laminapdf_save::SaveOptions::default()).unwrap();
    assert_eq!(
        (report.mode(), reopen(&out).pages().unwrap().len()),
        (laminapdf_save::SaveMode::FullRewrite, 1)
    );
}

/// `page.finish()`'s error code after `draw`.
fn refused(draw: impl FnOnce(&mut laminapdf_build::Surface)) -> &'static str {
    let mut b = Builder::new(BuildOptions::default());
    let mut page = b.page(PageSize::A6);
    draw(page.surface());
    page.finish().unwrap_err().code()
}

#[test]
fn a_nan_coordinate_is_refused_when_the_page_is_finished() {
    let code = refused(|s| {
        s.path(|p| {
            p.move_to(PointPt::from_points(f64::NAN, 1.0))
                .line_to(PointPt::from_points(2.0, 2.0));
        })
        .fill(FillRule::NonZero);
    });
    assert_eq!(code, "invalid-geometry");
}

#[test]
fn a_negative_line_width_is_refused() {
    let code = refused(|s| {
        s.path(|p| {
            p.rect(rect(1.0, 1.0, 5.0, 5.0));
        })
        .stroke(&StrokeStyle {
            width: Pt::new(-2.0),
            ..StrokeStyle::default()
        });
    });
    assert_eq!(code, "invalid-geometry");
}

#[test]
fn an_infinite_matrix_or_image_rect_is_refused() {
    let image = laminapdf_build::Image::rgba(1, 1, &[0, 0, 0, 255]).unwrap();
    let matrix = refused(|s| {
        s.transform(Matrix::scale(f64::INFINITY, 1.0));
    });
    let image = refused(|s| {
        s.image(&image, rect(0.0, f64::NAN, 1.0, 1.0));
    });
    assert_eq!((matrix, image), ("invalid-geometry", "invalid-geometry"));
}

#[test]
fn after_a_rect_the_current_point_is_its_top_left_on_every_rotation() {
    // `re` then `l`: the line starts where `re` left the current point, which must be the
    // rect's page-space top-left (10, 20), whatever the rotation.
    for rotate in [
        Rotation::None,
        Rotation::Clockwise90,
        Rotation::Half,
        Rotation::Clockwise270,
    ] {
        let doc = one_page(PageSize::A5, |page| {
            page.rotate(rotate);
            page.surface()
                .path(|p| {
                    p.rect(rect(10.0, 20.0, 30.0, 40.0))
                        .line_to(PointPt::from_points(100.0, 100.0));
                })
                .stroke(&StrokeStyle::default());
        });
        let frame = doc.pages().unwrap().get(0).unwrap().frame().unwrap();
        let first = record(&doc, 0).painted[0].points[0];
        let p = frame.to_page(UserSpacePoint::new(first.0, first.1));
        assert_eq!((p.x.get().round(), p.y.get().round()), (10.0, 20.0), "{rotate:?}");
    }
}

#[test]
fn features_newer_than_the_version_raise_the_catalog_version() {
    let mut b = Builder::new(BuildOptions::default().with_version(PdfVersion::V1_4));
    let mut page = b.page(PageSize::A6);
    page.user_unit(2.0);
    page.finish().unwrap();
    let doc = b.finish().unwrap();
    assert_eq!(doc.catalog().unwrap().get_name("Version"), Some(&b"1.6"[..]));
}

#[test]
fn a_document_whose_version_has_every_feature_gets_no_catalog_version() {
    let mut b = Builder::new(BuildOptions::default().with_version(PdfVersion::V1_4));
    b.page(PageSize::A6).surface().opacity(0.5, 0.5);
    let doc = b.finish().unwrap();
    assert_eq!(doc.catalog().unwrap().get("Version"), None);
}
