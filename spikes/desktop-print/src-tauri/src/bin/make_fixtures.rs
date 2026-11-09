//! Writes the spike's three fixtures into `fixtures/` with Lamina PDF's own builder:
//!
//! - `mixed-50.pdf`: 50 A4 pages of text, a colour band, a vector chart, and a photo on every
//!   fifth page. Each page's number is large, so a printed page range is easy to check.
//! - `mixed-sizes.pdf`: A4 portrait, US Letter landscape, A4 with `/Rotate 90`, and A5. Each page
//!   has a border 18 pt inside its edge and corner marks, so clipping and scaling show.
//! - `vectors.pdf`: hairlines (width 0) and 0.1–1 pt lines, a fine grid, concentric circles, and
//!   text at 4–8 pt.
//!
//! ```sh
//! cargo run --release --bin make_fixtures -- <repo-root> <out-dir>
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    BuildOptions, Builder, Color, FillRule, FontHandle, Image, PageSize, PointPt, Pt, RectPt, Rotation, SaveOptions,
    Standard14, StrokeStyle, Surface,
};

type Res<T> = Result<T, Box<dyn Error>>;

fn main() -> Res<()> {
    let mut args = std::env::args().skip(1);
    let repo = PathBuf::from(args.next().ok_or("usage: make_fixtures <repo-root> <out-dir>")?);
    let out = PathBuf::from(args.next().ok_or("usage: make_fixtures <repo-root> <out-dir>")?);
    std::fs::create_dir_all(&out)?;
    let photo = std::fs::read(repo.join("fixtures/images/photo-rgb.jpg"))?;
    std::fs::write(out.join("mixed-50.pdf"), mixed_50(photo)?)?;
    std::fs::write(out.join("mixed-sizes.pdf"), mixed_sizes()?)?;
    std::fs::write(out.join("vectors.pdf"), vectors()?)?;
    Ok(())
}

fn options() -> BuildOptions {
    // A fixed ID and no dates, so the fixtures are byte-identical across runs.
    BuildOptions::new().with_id([7; 16])
}

fn text(s: &mut Surface, font: &FontHandle, size: f64, x: f64, y: f64, line: &str) -> Res<()> {
    s.text(line, font, Pt::new(size), PointPt::from_points(x, y))?;
    Ok(())
}

const LOREM: &str = "Lamina PDF prints this line to check that body text stays sharp and in place on paper.";

fn mixed_50(photo: Vec<u8>) -> Res<Vec<u8>> {
    let mut b = Builder::new(options());
    let (sans, serif) = (b.standard_font(Standard14::HelveticaBold)?, b.standard_font(Standard14::TimesRoman)?);
    let photo = Image::jpeg(photo)?;
    for n in 1..=50 {
        let mut page = b.page(PageSize::A4);
        let s = page.surface();
        text(s, &sans, 72.0, 56.0, 120.0, &n.to_string())?;
        text(s, &sans, 14.0, 56.0, 150.0, &format!("Page {n} of 50"))?;
        let hue = f64::from(n) / 50.0;
        s.fill_color(Color::rgb(0.2 + 0.6 * hue, 0.4, 0.9 - 0.6 * hue))
            .path(|p| _ = p.rect(RectPt::from_points(56.0, 170.0, 483.0, 24.0)))
            .fill(FillRule::NonZero);
        for line in 0..30 {
            text(s, &serif, 10.0, 56.0, 220.0 + 13.0 * f64::from(line), LOREM)?;
        }
        if n % 5 == 0 {
            s.image(&photo, RectPt::from_points(56.0, 620.0, 240.0, 160.0));
        } else {
            chart(s, n)?;
        }
        page.finish()?;
    }
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// A line chart: axes and a polyline whose shape depends on the page number.
fn chart(s: &mut Surface, n: u32) -> Res<()> {
    let thin = StrokeStyle { width: Pt::new(0.75), ..StrokeStyle::default() };
    s.stroke_color(Color::gray(0.0))
        .path(|p| {
            p.move_to(PointPt::from_points(56.0, 620.0))
                .line_to(PointPt::from_points(56.0, 780.0))
                .line_to(PointPt::from_points(539.0, 780.0));
        })
        .stroke(&thin);
    let line = StrokeStyle { width: Pt::new(1.5), ..StrokeStyle::default() };
    s.stroke_color(Color::rgb(0.8, 0.2, 0.2))
        .path(|p| {
            p.move_to(PointPt::from_points(56.0, 700.0));
            for i in 1..=24 {
                let x = 56.0 + 20.0 * f64::from(i);
                let y = 700.0 + 60.0 * (f64::from(i * n) * 0.37).sin();
                p.line_to(PointPt::from_points(x, y));
            }
        })
        .stroke(&line);
    Ok(())
}

fn mixed_sizes() -> Res<Vec<u8>> {
    let mut b = Builder::new(options());
    let sans = b.standard_font(Standard14::HelveticaBold)?;
    let pages: [(&str, PageSize, Rotation); 4] = [
        ("A4 portrait", PageSize::A4, Rotation::None),
        ("US Letter landscape", PageSize::new(Pt::new(792.0), Pt::new(612.0)), Rotation::None),
        ("A4 with /Rotate 90 (displays landscape)", PageSize::A4, Rotation::Clockwise90),
        ("A5 portrait", PageSize::A5, Rotation::None),
    ];
    for (label, size, rotate) in pages {
        let mut page = b.page(size);
        page.rotate(rotate);
        // The surface is in displayed space, so a /Rotate 90 page is drawn landscape.
        let (w, h) = match rotate {
            Rotation::Clockwise90 | Rotation::Clockwise270 => (size.height.get(), size.width.get()),
            _ => (size.width.get(), size.height.get()),
        };
        let s = page.surface();
        let border = StrokeStyle { width: Pt::new(2.0), ..StrokeStyle::default() };
        s.stroke_color(Color::rgb(0.1, 0.3, 0.8))
            .path(|p| _ = p.rect(RectPt::from_points(18.0, 18.0, w - 36.0, h - 36.0)))
            .stroke(&border);
        for (x, y) in [(0.0, 0.0), (w - 18.0, 0.0), (0.0, h - 18.0), (w - 18.0, h - 18.0)] {
            s.fill_color(Color::rgb(0.9, 0.1, 0.1))
                .path(|p| _ = p.rect(RectPt::from_points(x, y, 18.0, 18.0)))
                .fill(FillRule::NonZero);
        }
        text(s, &sans, 24.0, 60.0, 90.0, label)?;
        text(s, &sans, 14.0, 60.0, 120.0, &format!("{w:.0} × {h:.0} pt as displayed; the red squares are the corners"))?;
        page.finish()?;
    }
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

fn vectors() -> Res<Vec<u8>> {
    let mut b = Builder::new(options());
    let sans = b.standard_font(Standard14::Helvetica)?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    // Line widths, one row each; width 0 is the thinnest line the device can draw.
    for (row, width) in [0.0, 0.1, 0.25, 0.5, 1.0].into_iter().enumerate() {
        let y = 60.0 + 24.0 * row as f64;
        text(s, &sans, 6.0, 40.0, y + 2.0, &format!("{width} pt"))?;
        let style = StrokeStyle { width: Pt::new(width), ..StrokeStyle::default() };
        s.stroke_color(Color::gray(0.0))
            .path(|p| {
                p.move_to(PointPt::from_points(80.0, y)).line_to(PointPt::from_points(555.0, y));
            })
            .stroke(&style);
    }
    // A 2 pt grid of hairlines over a 200 pt square.
    let hair = StrokeStyle { width: Pt::new(0.0), ..StrokeStyle::default() };
    s.stroke_color(Color::gray(0.2)).path(|p| {
        for i in 0..=100 {
            let o = 2.0 * f64::from(i);
            p.move_to(PointPt::from_points(40.0 + o, 200.0)).line_to(PointPt::from_points(40.0 + o, 400.0));
            p.move_to(PointPt::from_points(40.0, 200.0 + o)).line_to(PointPt::from_points(240.0, 200.0 + o));
        }
    }).stroke(&hair);
    // Concentric circles, 3 pt apart.
    let quarter = StrokeStyle { width: Pt::new(0.25), ..StrokeStyle::default() };
    s.stroke_color(Color::rgb(0.6, 0.1, 0.1)).path(|p| {
        for r in 1..=30 {
            p.circle(PointPt::from_points(420.0, 300.0), Pt::new(3.0 * f64::from(r)));
        }
    }).stroke(&quarter);
    for (row, size) in [4.0, 5.0, 6.0, 8.0].into_iter().enumerate() {
        let y = 450.0 + 20.0 * row as f64;
        text(s, &sans, size, 40.0, y, &format!("{size} pt: The quick brown fox jumps over the lazy dog 0123456789"))?;
    }
    page.finish()?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}
