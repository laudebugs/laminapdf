//! Drawing laid-out pages: one `Builder::page` per page, each line from its shaping (never
//! shaped again), the running header and footer as `Pagination` artifacts with their
//! `/Subtype`, and the structure elements kept open across pages (ADR 0032 §4).

use std::collections::BTreeMap;

use laminapdf_model::{PageId, PointPt, RectPt};
use laminapdf_text::{Alignment, ShapedRun};

use super::items::{Content, Items};
use super::measure::{Look, MLine, Source};
use super::model::{Flow, PageRegion, ParagraphStyle, TextStyle};
use super::paginate::{PageOut, Placed};
use super::template::{Fields, fill_fields};
use crate::annot::link::LinkSpec;
use crate::builder::Builder;
use crate::error::Result;
use crate::surface::Surface;
use crate::surface::text::ShapedPiece;
use crate::tag::{Artifact, ElementId, Tag, TagKind};

/// Draws `pages` into `builder`; returns the pages added.
pub(crate) fn emit(builder: &mut Builder, flow: &Flow, items: &Items, pages: &[PageOut]) -> Result<Vec<PageId>> {
    let mut open: BTreeMap<usize, ElementId> = BTreeMap::new();
    let mut order = Vec::new();
    let mut ids = Vec::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate() {
        let template = flow.template.for_page(index);
        let mut pb = builder.page(template.size);
        let s = pb.surface();
        let fields = Fields {
            page: index + 1,
            pages: pages.len(),
            section: page.section.as_deref(),
            title: flow.title.as_deref(),
        };
        // The header before the body and the footer after it: extractors that read in content
        // order read the page top to bottom.
        furniture(s, flow, index, &fields, true)?;
        let mut leaf: Option<usize> = None;
        let mut batch: Vec<&Placed> = Vec::new();
        for placed in &page.placed {
            let node = node_of(items, placed);
            if node != leaf && !batch.is_empty() {
                draw_batch(s, flow, items, leaf, &batch, &mut open, &mut order)?;
                batch.clear();
            }
            leaf = node;
            batch.push(placed);
        }
        if !batch.is_empty() {
            draw_batch(s, flow, items, leaf, &batch, &mut open, &mut order)?;
        }
        furniture(s, flow, index, &fields, false)?;
        ids.push(pb.finish()?);
    }
    // Children were opened after their parents: close them first.
    for id in order.into_iter().rev() {
        builder.close_element(id)?;
    }
    Ok(ids)
}

/// The element `placed` draws into.
fn node_of(items: &Items, placed: &Placed) -> Option<usize> {
    match placed {
        Placed::Line { item, .. } | Placed::Figure { item, .. } => items.entries.get(*item).and_then(|i| i.node),
        Placed::Label { item, .. } => items.entries.get(*item).and_then(|i| i.label).map(|(_, n)| n),
    }
}

/// Opens `node` (its parents first) unless it is open, and returns it.
fn ensure(
    s: &mut Surface,
    items: &Items,
    node: usize,
    open: &mut BTreeMap<usize, ElementId>,
    order: &mut Vec<ElementId>,
) -> Result<ElementId> {
    if let Some(id) = open.get(&node) {
        return Ok(*id);
    }
    let Some(n) = items.nodes.get(node) else {
        return Ok(s.open_element(Tag::new(TagKind::P)));
    };
    let tag = n.tag.clone();
    let id = match n.parent {
        Some(parent) => {
            let parent = ensure(s, items, parent, open, order)?;
            s.resume(parent, |s| Ok(s.open_element(tag)))?
        }
        None => s.open_element(tag),
    };
    open.insert(node, id);
    order.push(id);
    Ok(id)
}

fn draw_batch(
    s: &mut Surface,
    flow: &Flow,
    items: &Items,
    leaf: Option<usize>,
    batch: &[&Placed],
    open: &mut BTreeMap<usize, ElementId>,
    order: &mut Vec<ElementId>,
) -> Result<()> {
    let draw = |s: &mut Surface| -> Result<()> {
        for placed in batch {
            draw_placed(s, flow, items, placed);
        }
        Ok(())
    };
    match leaf.filter(|_| flow.tagged) {
        Some(node) => {
            let id = ensure(s, items, node, open, order)?;
            s.resume(id, draw)
        }
        None => draw(s),
    }
}

fn draw_placed(s: &mut Surface, flow: &Flow, items: &Items, placed: &Placed) {
    match placed {
        Placed::Line { item, line, x, top } => {
            let source = items.entries.get(*item).and_then(|it| match it.content {
                Content::Para(i) => items.sources.get(i),
                _ => None,
            });
            if let Some(source) = source {
                draw_line(s, flow.tagged, source, line, *x, *top);
            }
        }
        Placed::Label { item, line, x, top } => {
            let source = items
                .entries
                .get(*item)
                .and_then(|it| it.label)
                .and_then(|(i, _)| items.sources.get(i));
            if let Some(source) = source {
                draw_line(s, flow.tagged, source, line, *x, *top);
            }
        }
        Placed::Figure {
            item,
            x,
            top,
            width,
            height,
        } => {
            let figure = items.entries.get(*item).and_then(|it| match it.content {
                Content::Figure(f) => items.figures.get(f),
                _ => None,
            });
            if let Some(figure) = figure {
                let rect = RectPt::from_points(*x, *top, *width, *height);
                if flow.tagged {
                    s.figure(&figure.image, rect, Some(&figure.alt));
                } else {
                    s.image(&figure.image, rect);
                }
            }
        }
    }
}

/// Draws `line` of `source` with its top at `top` and the frame's left edge at `x`: each run,
/// in logical order, cut where the colour, link or language of its text changes.
fn draw_line(s: &mut Surface, tagged: bool, source: &Source, line: &MLine, x: f64, top: f64) {
    let baseline = top + line.baseline;
    for run in line
        .line
        .drawing_order()
        .into_iter()
        .filter_map(|i| line.line.runs.get(i))
    {
        let Some(font) = source.fonts.get(run.font) else {
            continue;
        };
        let size = f64::from(run.size);
        let mut pen = x + line.line.x + run.x;
        for (glyphs, look) in pieces(source, &run.run) {
            let Some(piece) = piece(source, &run.run, glyphs) else {
                continue;
            };
            let width = piece.width(run.size);
            let text = source.text.get(piece.text_range.clone()).unwrap_or_default().to_owned();
            let link_spec = look.link.and_then(|l| source.links.get(l)).map(|target| LinkSpec {
                target: target.clone(),
                contents: Some(text.trim().to_owned()),
            });
            let tag = look
                .lang
                .as_ref()
                .filter(|_| tagged)
                .map(|lang| Box::new(Tag::new(TagKind::Span).with_lang(lang.clone())));
            let mut rebased = piece;
            let start = rebased.text_range.start;
            rebased.text_range = 0..rebased.text_range.end - start;
            for g in &mut rebased.glyphs {
                g.cluster -= start;
            }
            s.draw_shaped(ShapedPiece {
                text,
                run: rebased,
                font: font.clone(),
                size,
                origin: PointPt::from_points(pen, baseline),
                rise: f64::from(run.rise),
                fill: look.color.clone(),
                link: link_spec,
                tag,
            });
            pen += width;
        }
    }
}

/// `run`'s glyphs (visual order) cut where the look of their text changes.
fn pieces<'s>(source: &'s Source, run: &ShapedRun) -> Vec<(std::ops::Range<usize>, &'s Look)> {
    let mut out: Vec<(std::ops::Range<usize>, &Look)> = Vec::new();
    for (i, glyph) in run.glyphs.iter().enumerate() {
        let Some(look) = source.look_at(glyph.cluster) else {
            continue;
        };
        match out.last_mut() {
            Some((range, last)) if *last == look => range.end = i + 1,
            _ => out.push((i..i + 1, look)),
        }
    }
    out
}

/// The run of `run`'s glyphs `glyphs`, with the text range their clusters cover.
fn piece(source: &Source, run: &ShapedRun, glyphs: std::ops::Range<usize>) -> Option<ShapedRun> {
    let mut clusters: Vec<usize> = run.glyphs.iter().map(|g| g.cluster).collect();
    clusters.sort_unstable();
    clusters.dedup();
    let chosen = run.glyphs.get(glyphs)?.to_vec();
    let start = chosen.iter().map(|g| g.cluster).min()?;
    let last = chosen.iter().map(|g| g.cluster).max()?;
    let end = clusters
        .iter()
        .copied()
        .find(|&c| c > last)
        .unwrap_or(run.text_range.end)
        .min(source.text.len());
    let mut out = run.clone();
    out.glyphs = chosen;
    out.text_range = start..end;
    Some(out)
}

/// The header (`header`) or the footer of page `index`, as a `Pagination` artifact with its
/// subtype, centred in the top or bottom margin.
fn furniture(s: &mut Surface, flow: &Flow, index: usize, fields: &Fields<'_>, header: bool) -> Result<()> {
    let template = flow.template.for_page(index);
    let Some(style) = &template.furniture else {
        return Ok(());
    };
    let height = template.size.height.get();
    let regions = if header {
        [(&template.header, Artifact::header(), template.margins.top / 2.0)]
    } else {
        [(
            &template.footer,
            Artifact::footer(),
            height - template.margins.bottom / 2.0,
        )]
    };
    for (region, artifact, middle) in regions {
        let Some(region) = region else { continue };
        let lines = slot_lines(region, style, fields, template.body_width())?;
        if lines.is_empty() {
            continue;
        }
        let x = template.margins.left;
        let draw = |s: &mut Surface| -> Result<()> {
            for (source, line) in &lines {
                let top = middle - line.height / 2.0;
                draw_line(s, false, source, line, x, top);
            }
            Ok(())
        };
        if flow.tagged {
            s.artifact(artifact, draw)?;
        } else {
            draw(s)?;
        }
    }
    Ok(())
}

/// Each filled slot of `region` as one line across the body's width, aligned to its side.
fn slot_lines(region: &PageRegion, style: &TextStyle, fields: &Fields<'_>, width: f64) -> Result<Vec<(Source, MLine)>> {
    let slots = [
        (&region.start, Alignment::Start),
        (&region.center, Alignment::Center),
        (&region.end, Alignment::End),
    ];
    let mut out = Vec::new();
    for (text, align) in slots {
        let Some(text) = text else { continue };
        let text = fill_fields(text, fields);
        if text.trim().is_empty() {
            continue;
        }
        let source = Source::single(&text, style, ParagraphStyle::new().with_align(align));
        if let Some(line) = source.lines(0, width)?.into_iter().next() {
            out.push((source, line));
        }
    }
    Ok(out)
}
