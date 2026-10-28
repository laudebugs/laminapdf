//! Experiment A: krilla as-is. Generate the three pages with krilla, then hand the bytes to
//! lamina-cos (open, inspect, incremental update, reopen).

use anyhow::{Result, anyhow};
use krilla::Document;
use krilla::action::LinkAction;
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::color::rgb;
use krilla::destination::XyzDestination;
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::metadata::{DateTime, Metadata};
use krilla::num::NormalizedF32;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::tagging::{ContentTag, SpanTag, Tag, TagGroup, TagTree};
use krilla::text::{Font, TextDirection};

use crate::assets::{Assets, IMAGE_SIDE, LINK_URI, PAGE_H, PAGE_W, PAGES, TITLE};

/// Generates the sample with krilla. `tagged` adds a structure tree (one `P` per line).
pub fn generate(assets: &Assets, tagged: bool) -> Result<Vec<u8>> {
    let mut doc = Document::new();
    let font = Font::new(assets.font.clone().into(), 0).ok_or_else(|| anyhow!("font"))?;
    let png = Image::from_png(assets.png_alpha.clone().into(), true).map_err(|e| anyhow!(e))?;
    let jpeg = Image::from_jpeg(assets.jpeg.clone().into(), true).map_err(|e| anyhow!(e))?;
    let mut tree = TagTree::new().with_lang(Some("en".into()));
    let mut outline = Outline::new();

    for (index, (heading, lines)) in PAGES.iter().enumerate() {
        let settings = PageSettings::from_wh(PAGE_W, PAGE_H).ok_or_else(|| anyhow!("page size"))?;
        let mut page = doc.start_page_with(settings);
        if index == 0 {
            // Line 3 of page 1 is the link (krilla's y axis points down).
            let rect = Rect::from_xywh(56.0, 100.0 + 3.0 * 18.0 - 13.0, 330.0, 17.0).ok_or_else(|| anyhow!("rect"))?;
            let target = Target::Action(LinkAction::new(LINK_URI.into()).into());
            page.add_annotation(Annotation::new_link(
                LinkAnnotation::new(rect, target),
                Some("Visit example.com".into()),
            ));
        }
        let mut surface = page.surface();
        let mut text = |s: &mut Surface<'_>, at: (f32, f32), size: f32, str: &str| {
            let id = tagged.then(|| s.start_tagged(ContentTag::Span(SpanTag::empty())));
            s.draw_text(
                Point::from_xy(at.0, at.1),
                font.clone(),
                size,
                str,
                false,
                TextDirection::Auto,
            );
            if let Some(id) = id {
                s.end_tagged();
                tree.push(TagGroup::with_children(Tag::P, vec![id.into()]));
            }
        };
        text(&mut surface, (56.0, 70.0), 20.0, heading);
        for (i, line) in lines.iter().enumerate() {
            text(&mut surface, (56.0, 100.0 + (i as f32 + 1.0) * 18.0), 12.0, line);
        }
        match index {
            0 => draw_vectors(&mut surface)?,
            1 => {
                let side = IMAGE_SIDE as f32;
                let size = Size::from_wh(side, side).ok_or_else(|| anyhow!("size"))?;
                surface.push_transform(&Transform::from_translate(56.0, 200.0));
                surface.draw_image(png.clone(), size);
                surface.pop();
                let (w, h) = assets.jpeg_size;
                let size = Size::from_wh(w as f32, h as f32).ok_or_else(|| anyhow!("size"))?;
                surface.push_transform(&Transform::from_translate(300.0, 200.0));
                surface.draw_image(jpeg.clone(), size);
                surface.pop();
            }
            _ => {}
        }
        surface.finish();
        page.finish();
        outline.push_child(OutlineNode::new(
            (*heading).into(),
            XyzDestination::new(index, Point::from_xy(0.0, 0.0)),
        ));
    }

    doc.set_outline(outline);
    if tagged {
        doc.set_tag_tree(tree);
    }
    // Fixed date so output is deterministic.
    let date = DateTime::new(2026).month(10).day(28);
    doc.set_metadata(
        Metadata::new()
            .title(TITLE.into())
            .language("en".into())
            .producer("lamina krilla-fit spike".into())
            .creation_date(date),
    );
    doc.finish().map_err(|e| anyhow!("krilla: {e:?}"))
}

fn draw_vectors(surface: &mut Surface<'_>) -> Result<()> {
    let mut pb = PathBuilder::new();
    pb.push_rect(Rect::from_xywh(56.0, 220.0, 200.0, 120.0).ok_or_else(|| anyhow!("rect"))?);
    let rect = pb.finish().ok_or_else(|| anyhow!("path"))?;
    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(30, 120, 200).into(),
        opacity: NormalizedF32::new(0.6).ok_or_else(|| anyhow!("opacity"))?,
        rule: Default::default(),
    }));
    surface.set_stroke(Some(Stroke {
        paint: rgb::Color::new(20, 20, 20).into(),
        width: 2.0,
        ..Default::default()
    }));
    surface.draw_path(&rect);

    let mut pb = PathBuilder::new();
    pb.move_to(300.0, 340.0);
    pb.line_to(400.0, 220.0);
    pb.line_to(500.0, 340.0);
    pb.close();
    // A circle-ish curve (four cubic arcs, κ ≈ 0.5523).
    let (cx, cy, r, k) = (420.0, 450.0, 60.0, 0.5523 * 60.0);
    pb.move_to(cx + r, cy);
    pb.cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
    pb.cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
    pb.cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
    pb.cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
    pb.close();
    let shapes = pb.finish().ok_or_else(|| anyhow!("path"))?;
    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(220, 80, 40).into(),
        ..Default::default()
    }));
    surface.set_stroke(None);
    surface.draw_path(&shapes);
    surface.set_fill(None);
    Ok(())
}
