//! The documents behind `lamina-build`'s goldens. `cargo xtask golden --update` writes them
//! (through `examples/golden.rs`); `tests/goldens.rs` checks them byte for byte.

use lamina_build::{
    BlendMode, BuildOptions, Builder, Color, Dash, FillRule, IccProfile, Image, LineCap, LineJoin, Matrix, PageSize,
    PngOptions, Rotation, StrokeStyle,
};
use lamina_model::{PointPt, Pt, RectPt};

use super::{b5b, fixture, legacy, png, rgba_gradient, save, tagged};

/// The code that builds one golden.
pub type Build = fn() -> Vec<u8>;

/// Every golden: its file name under `fixtures/golden/` and the code that builds it.
pub const GOLDENS: [(&str, Build); 9] = [
    ("hello.pdf", || legacy::hello().unwrap()),
    ("imaging-demo.pdf", || legacy::imaging_demo().unwrap()),
    ("vectors.pdf", vectors),
    ("images.pdf", images),
    ("build-text-latin.pdf", b5b::text_latin),
    ("build-text-multiscript.pdf", b5b::text_multiscript),
    ("links-and-outlines.pdf", b5b::links_and_outlines),
    ("metadata.pdf", b5b::metadata),
    ("tagged-report.pdf", tagged::tagged_report),
];

fn options() -> BuildOptions {
    BuildOptions::default()
        .with_id(*b"lamina-build-B5a")
        .with_producer("lamina-build golden")
        .with_creation_date("D:20261029120000Z".parse().unwrap())
        .with_compress(false)
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

fn p(x: f64, y: f64) -> PointPt {
    PointPt::from_points(x, y)
}

/// `fixtures/golden/vectors.pdf`: paths, strokes, fills, clipping, transforms, opacity, blend
/// modes and colour spaces on an A4 page (positions from its top-left), and a rotated Letter
/// page with a crop box and `/UserUnit 2` whose markers still sit at the displayed top-left.
#[expect(clippy::too_many_lines, reason = "one golden page, drawn top to bottom")]
pub fn vectors() -> Vec<u8> {
    let icc = IccProfile::new(fixture("images/srgb-like.icc")).unwrap();
    let mut b = Builder::new(options());
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    // 1. A filled rectangle and a dashed circle with round joins (imaging-demo's parts 1–2).
    s.save()
        .fill_color(Color::rgb(0.2, 0.4, 0.8))
        .path(|p| {
            p.rect(rect(56.0, 56.0, 200.0, 120.0));
        })
        .fill(FillRule::NonZero)
        .restore();
    let dashed = StrokeStyle {
        width: Pt::new(6.0),
        join: LineJoin::Round,
        dash: Some(Dash {
            array: vec![Pt::new(18.0), Pt::new(6.0)],
            phase: Pt::ZERO,
        }),
        ..StrokeStyle::default()
    };
    s.save()
        .stroke_color(Color::hex("#d91a1a").unwrap())
        .path(|path| {
            path.circle(p(420.0, 116.0), Pt::new(60.0));
        })
        .stroke(&dashed)
        .restore();
    // 2. An even-odd star, filled and stroked, and a quadratic curve with round caps.
    let star: Vec<PointPt> = (0..5)
        .map(|i| {
            let a = (f64::from(i) * 144.0 - 90.0).to_radians();
            p(156.0 + 70.0 * a.cos(), 300.0 + 70.0 * a.sin())
        })
        .collect();
    s.fill_color(Color::cmyk(0.0, 0.3, 0.9, 0.0))
        .stroke_color(Color::gray(0.1))
        .path(|path| {
            path.move_to(star[0]);
            for &pt in &star[1..] {
                path.line_to(pt);
            }
            path.close();
        })
        .fill_stroke(FillRule::EvenOdd, &StrokeStyle::default());
    s.stroke_color(Color::icc(&icc, &[0.1, 0.6, 0.3]).unwrap())
        .path(|path| {
            path.move_to(p(300.0, 360.0)).quad_to(p(400.0, 220.0), p(500.0, 360.0));
        })
        .stroke(&StrokeStyle {
            width: Pt::new(8.0),
            cap: LineCap::Round,
            ..StrokeStyle::default()
        });
    // 3. A clip: stripes inside an ellipse.
    s.save()
        .path(|path| {
            path.ellipse(rect(56.0, 420.0, 480.0, 140.0));
        })
        .clip(FillRule::NonZero);
    for i in 0..12 {
        let level = f64::from(i) / 12.0;
        s.fill_color(Color::rgb(level, 0.5, 1.0 - level))
            .path(|path| {
                path.rect(rect(56.0 + f64::from(i) * 40.0, 420.0, 40.0, 140.0));
            })
            .fill(FillRule::NonZero);
    }
    s.restore();
    // 4. A square turned 30° about its centre at 50 % opacity, and two circles multiplied.
    s.save()
        .opacity(0.5, 0.5)
        .transform(Matrix::translate(Pt::new(156.0), Pt::new(680.0)).after(&Matrix::rotate(30.0)))
        .fill_color(Color::rgb(0.0, 0.6, 0.3))
        .stroke_color(Color::BLACK)
        .path(|path| {
            path.rect(rect(-60.0, -60.0, 120.0, 120.0));
        })
        .fill_stroke(
            FillRule::NonZero,
            &StrokeStyle {
                width: Pt::new(2.0),
                ..StrokeStyle::default()
            },
        )
        .restore();
    s.save().blend_mode(BlendMode::Multiply);
    for (x, color) in [(380.0, "#ffcc00"), (440.0, "#00ccff")] {
        s.fill_color(Color::hex(color).unwrap())
            .path(|path| {
                path.circle(p(x, 680.0), Pt::new(50.0));
            })
            .fill(FillRule::NonZero);
    }
    s.restore();
    page.finish().unwrap();

    // Page 2: Letter turned 90°, cropped by an inch on every side, /UserUnit 2. Page space is
    // the displayed (landscape) crop box: the markers are at its top-left and bottom-right.
    let mut page = b.page(PageSize::LETTER);
    page.rotate(Rotation::Clockwise90)
        .user_unit(2.0)
        .crop_box(rect(72.0, 72.0, 648.0, 468.0));
    page.surface()
        .fill_color(Color::rgb(0.85, 0.1, 0.1))
        .path(|path| {
            path.move_to(p(0.0, 0.0))
                .line_to(p(72.0, 0.0))
                .line_to(p(0.0, 72.0))
                .close();
        })
        .fill(FillRule::NonZero)
        .fill_color(Color::rgb(0.1, 0.3, 0.85))
        .path(|path| {
            path.rect(rect(576.0, 396.0, 72.0, 72.0));
        })
        .fill(FillRule::NonZero);
    page.finish().unwrap();
    save(b.finish().unwrap())
}

/// `fixtures/golden/images.pdf`: a PNG with alpha (`/SMask`), a 16-bit gray PNG kept and
/// down-converted, a palette PNG with `tRNS`, raw RGBA, and the RGB, gray and Adobe CMYK
/// JPEGs passed through, one of them drawn twice (written once).
pub fn images() -> Vec<u8> {
    let rgba = png(
        (64, 64),
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &rgba_gradient(64),
        None,
        None,
    );
    let gray16: Vec<u8> = (0..32u16 * 32)
        .flat_map(|i| ((i % 32) * 2048 + (i / 32) * 16).to_be_bytes())
        .collect();
    let gray16 = png(
        (32, 32),
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen,
        &gray16,
        None,
        None,
    );
    let indexed: Vec<u8> = (0..256u32).map(|i| u8::try_from((i % 16) / 4).unwrap()).collect();
    let indexed = png(
        (16, 16),
        png::ColorType::Indexed,
        png::BitDepth::Eight,
        &indexed,
        Some(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0]),
        Some(&[255, 255, 255, 0]),
    );
    let raw = Image::rgba(
        2,
        2,
        &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 64],
    )
    .unwrap();
    let images = [
        Image::png(&rgba).unwrap(),
        Image::png(&gray16).unwrap(),
        Image::png_with(&gray16, PngOptions::default().with_downconvert_16(true)).unwrap(),
        Image::png(&indexed).unwrap(),
        raw,
        Image::jpeg(fixture("images/photo-rgb.jpg")).unwrap(),
        Image::jpeg(fixture("images/photo-gray.jpg")).unwrap(),
        Image::jpeg(fixture("images/photo-cmyk.jpg")).unwrap(),
    ];
    let mut b = Builder::new(options());
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    // A backdrop so transparency shows.
    s.fill_color(Color::gray(0.85))
        .path(|path| {
            path.rect(rect(36.0, 36.0, 523.0, 280.0));
        })
        .fill(FillRule::NonZero);
    for (i, image) in images.iter().enumerate() {
        let i = f64::from(u32::try_from(i).unwrap());
        let (col, row) = (i % 4.0, (i / 4.0).floor());
        s.image(image, rect(56.0 + col * 128.0, 56.0 + row * 128.0, 112.0, 112.0));
    }
    // The RGB JPEG again, larger: the same XObject.
    s.image(&images[5], rect(56.0, 320.0, 240.0, 240.0));
    page.finish().unwrap();
    save(b.finish().unwrap())
}
