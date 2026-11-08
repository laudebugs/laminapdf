//! Experiment C, the hybrid: Typst lays the document out, and its frames are drawn through
//! `laminapdf-build`'s `Surface`, without `typst-pdf`, so the result lives in a Lamina PDF
//! document's overlay. It stops at the first page that renders (the todo's time box) and counts
//! what doesn't map.

use std::collections::HashMap;

use laminapdf::{
    BuildOptions, Builder, Color, FillRule, Font, FontHandle, Matrix, PageSize, PointPt, Pt, StrokeStyle, Surface,
};
use typst::layout::{Frame, FrameItem};
use typst::visualize::{CurveItem, Geometry, Paint};
use typst_layout::PagedDocument;

/// What didn't map from Typst's frames onto `Surface`, counted over the pages drawn.
#[derive(Debug, Default, Clone)]
pub struct Unmapped {
    /// Text items: drawn from their string, so our shaper shaped them again; Typst's glyph IDs
    /// and advances were thrown away (`Surface` draws strings, not glyph runs).
    pub reshaped_text: usize,
    /// Text items whose glyph advances differ from ours by more than 0.5 pt over the run.
    pub drifted_text: usize,
    /// Typst's introspection tags: markers for its own tagger, which lives in `typst-pdf`.
    pub tags: usize,
    /// Clips on groups (table cells): `Surface` clips, but the curve would need converting.
    pub clips: usize,
    /// Gradient and tiling paints.
    pub paints: usize,
    pub links: usize,
    pub images: usize,
}

fn pt(x: f64, y: f64) -> PointPt {
    PointPt::new(Pt::new(x), Pt::new(y))
}

type Drawn<T> = Result<T, laminapdf::BuildError>;

/// Draws the first `pages` pages of `doc` through `Surface` and saves the result.
pub fn draw(doc: &PagedDocument, pages: usize) -> Result<(Vec<u8>, Unmapped), laminapdf::Error> {
    let mut b = Builder::new(BuildOptions::new().with_id(*b"lamina-flow-hyb!"));
    let mut fonts: HashMap<(usize, u32), FontHandle> = HashMap::new();
    let mut unmapped = Unmapped::default();
    for page in doc.pages().iter().take(pages) {
        // Register every font the page uses first: `Builder::font` needs the builder, which the
        // page borrows while it's open.
        collect_fonts(&page.frame, &mut |font| {
            let key = (font.data().as_ptr() as usize, font.index());
            if !fonts.contains_key(&key) {
                let ours = Font::from_collection(font.data().to_vec(), font.index())?;
                fonts.insert(key, b.font(ours)?);
            }
            Ok::<_, laminapdf::Error>(())
        })?;
        let size = page.frame.size();
        let mut p = b.page(PageSize::new(Pt::new(size.x.to_pt()), Pt::new(size.y.to_pt())));
        frame(p.surface(), &page.frame, &fonts, &mut unmapped)?;
        p.finish()?;
    }
    let mut out = b.finish()?;
    Ok((out.save(&laminapdf::SaveOptions::default())?.bytes, unmapped))
}

fn collect_fonts<E>(f: &Frame, visit: &mut impl FnMut(&typst::text::Font) -> Result<(), E>) -> Result<(), E> {
    for (_, item) in f.items() {
        match item {
            FrameItem::Group(group) => collect_fonts(&group.frame, visit)?,
            FrameItem::Text(text) => visit(text.font.font())?,
            _ => {}
        }
    }
    Ok(())
}

fn colour(paint: &Paint, unmapped: &mut Unmapped) -> Color {
    match paint {
        Paint::Solid(c) => {
            let rgb = c.to_rgb();
            Color::rgb(f64::from(rgb.red), f64::from(rgb.green), f64::from(rgb.blue))
        }
        _ => {
            unmapped.paints += 1;
            Color::gray(0.0)
        }
    }
}

fn frame(s: &mut Surface, f: &Frame, fonts: &HashMap<(usize, u32), FontHandle>, un: &mut Unmapped) -> Drawn<()> {
    for (pos, item) in f.items() {
        let (x, y) = (pos.x.to_pt(), pos.y.to_pt());
        match item {
            FrameItem::Group(group) => {
                let t = group.transform;
                s.save();
                s.transform(Matrix::new([t.sx.get(), t.ky.get(), t.kx.get(), t.sy.get(), x + t.tx.to_pt(), y + t.ty.to_pt()]));
                if group.clip.is_some() {
                    un.clips += 1;
                }
                frame(s, &group.frame, fonts, un)?;
                s.restore();
            }
            FrameItem::Text(text) => {
                let font = text.font.font();
                let Some(handle) = fonts.get(&(font.data().as_ptr() as usize, font.index())) else { continue };
                un.reshaped_text += 1;
                let fill = colour(&text.fill, un);
                let size = text.size.to_pt();
                let theirs = text.width().to_pt();
                let run = s.fill_color(fill).text(&text.text, handle, Pt::new(size), pt(x, y))?;
                let ours = run.bounds().width().get();
                if (ours - theirs).abs() > 0.5 {
                    un.drifted_text += 1;
                }
            }
            FrameItem::Shape(shape, _) => {
                let fill = shape.fill.as_ref().map(|p| colour(p, un));
                let stroke = shape.stroke.as_ref().map(|st| (colour(&st.paint, un), st.thickness.to_pt()));
                if let Some(c) = fill.clone() {
                    s.fill_color(c);
                }
                if let Some((c, _)) = stroke.clone() {
                    s.stroke_color(c);
                }
                let path = s.path(|p| match &shape.geometry {
                    Geometry::Line(to) => {
                        p.move_to(pt(x, y)).line_to(pt(x + to.x.to_pt(), y + to.y.to_pt()));
                    }
                    Geometry::Rect(size) => {
                        p.rect(laminapdf::RectPt::from_points(x, y, size.x.to_pt(), size.y.to_pt()));
                    }
                    Geometry::Curve(curve) => {
                        let at = |q: &typst::layout::Point| pt(x + q.x.to_pt(), y + q.y.to_pt());
                        for c in curve.0.iter() {
                            match c {
                                CurveItem::Move(a) => _ = p.move_to(at(a)),
                                CurveItem::Line(a) => _ = p.line_to(at(a)),
                                CurveItem::Cubic(a, b2, c2) => _ = p.cubic_to(at(a), at(b2), at(c2)),
                                CurveItem::Close => _ = p.close(),
                            }
                        }
                    }
                });
                let style = |w: f64| StrokeStyle { width: Pt::new(w), ..StrokeStyle::default() };
                match (fill, stroke) {
                    (Some(_), Some((_, w))) => _ = path.fill_stroke(FillRule::NonZero, &style(w)),
                    (Some(_), None) => _ = path.fill(FillRule::NonZero),
                    (None, Some((_, w))) => _ = path.stroke(&style(w)),
                    (None, None) => {}
                }
            }
            FrameItem::Tag(_) => un.tags += 1,
            FrameItem::Link(..) => un.links += 1,
            FrameItem::Image(..) => un.images += 1,
        }
    }
    Ok(())
}
