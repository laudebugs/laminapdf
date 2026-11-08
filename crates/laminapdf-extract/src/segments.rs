//! A run's kept text, cut into pieces: at column gutters, where its glyphs pass from one layout
//! region into another, and around the stretches a later paint covers (ADR 0030 §4's
//! `covered`).
//!
//! A glyph is covered when one [`Cover`] painted after it holds the centre of its box and at
//! least half its area. Runs whose text is not their glyphs in drawing order (`ActualText`, right
//! to left, vertical) are judged whole: covered when more than half their glyphs are.

use std::collections::BTreeMap;

use laminapdf_model::{Pt, RectPt};
use laminapdf_text::{Glyph, PageText, PageTextRun, Quad, is_right_to_left};

use crate::lines::{GUTTER, SPACE};
use crate::paint::{Cover, intersect};
use crate::plugins::LayoutRegion;
use crate::regions::region_near;

/// A cover must hold at least this share of a glyph's box (and its centre).
const COVERED_SHARE: f64 = 0.5;

/// The grid cell size, in points, of [`Covers`]' index.
const CELL: f64 = 64.0;

/// A page's covers, indexed by a coarse grid so that each glyph tests only the covers near it.
#[derive(Debug, Default)]
pub(crate) struct Covers {
    covers: Vec<Cover>,
    cells: BTreeMap<(i64, i64), Vec<usize>>,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a page coordinate over 64 pt cells, clamped to well inside i64"
)]
fn cell(v: f64) -> i64 {
    (v / CELL).floor().clamp(-1e9, 1e9) as i64
}

impl Covers {
    pub(crate) fn new(covers: Vec<Cover>) -> Self {
        let mut cells: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
        for (i, c) in covers.iter().enumerate() {
            let r = &c.rect;
            if !r.is_finite() {
                continue;
            }
            for x in cell(r.x0.get())..=cell(r.x1.get()) {
                for y in cell(r.y0.get())..=cell(r.y1.get()) {
                    cells.entry((x, y)).or_default().push(i);
                }
            }
        }
        Self { covers, cells }
    }

    /// Whether a paint after the glyph covers it.
    pub(crate) fn covers(&self, glyph: &Glyph) -> bool {
        self.covers_box(&quad_box(&glyph.quad), glyph.paints_before)
    }

    /// Whether a cover painted after `paints_before` paints holds the centre of `r` and at
    /// least half its area.
    fn covers_box(&self, r: &RectPt, paints_before: u32) -> bool {
        let (cx, cy) = (
            f64::midpoint(r.x0.get(), r.x1.get()),
            f64::midpoint(r.y0.get(), r.y1.get()),
        );
        let Some(near) = self.cells.get(&(cell(cx), cell(cy))) else {
            return false;
        };
        let area = r.width().get() * r.height().get();
        near.iter().filter_map(|&i| self.covers.get(i)).any(|c| {
            let holds_centre =
                c.rect.x0.get() <= cx && cx <= c.rect.x1.get() && c.rect.y0.get() <= cy && cy <= c.rect.y1.get();
            let shared = intersect(&c.rect, r).map_or(0.0, |o| o.width().get() * o.height().get());
            c.after >= paints_before && holds_centre && shared >= COVERED_SHARE * area
        })
    }
}

/// The smallest rectangle holding a glyph's quad.
pub(crate) fn quad_box(quad: &Quad) -> RectPt {
    let (x0, x1) = quad.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
        (a.min(p.x.get()), b.max(p.x.get()))
    });
    let (y0, y1) = quad.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
        (a.min(p.y.get()), b.max(p.y.get()))
    });
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// A run's text and box, cut into the pieces that stay in the text and the stretches covered.
#[derive(Debug, Default)]
pub(crate) struct Cut {
    pub(crate) kept: Vec<(String, RectPt)>,
    pub(crate) covered: Vec<(String, RectPt)>,
}

/// A part being built: its text, box, and how far right it reaches with the covered glyphs
/// inside it (a cover never opens a gutter).
struct Part {
    words: String,
    bbox: RectPt,
    reach: f64,
    /// A covered stretch ended inside it: the next visible glyph joins with one space.
    resumed: bool,
}

/// `run` cut at column gutters, at layout regions and around covered stretches. A run whose
/// text isn't its glyphs in drawing order is kept or covered whole; a run with one piece and
/// nothing covered is kept as `laminapdf-text` gave it.
pub(crate) fn segments(text: &PageText, run: &PageTextRun, size: Pt, regions: &[LayoutRegion], covers: &Covers) -> Cut {
    let whole = || Cut {
        kept: vec![(run.text.clone(), run.bounds)],
        covered: Vec::new(),
    };
    let covered_whole = || Cut {
        kept: Vec::new(),
        covered: vec![(run.text.clone(), run.bounds)],
    };
    let Some(glyphs) = text.glyphs.get(run.glyphs.clone()) else {
        return whole();
    };
    let mask: Vec<bool> = glyphs.iter().map(|g| covers.covers(g)).collect();
    let inked = |i: usize| glyphs.get(i).is_some_and(|g| !g.text.trim().is_empty());
    let ink = (0..glyphs.len()).filter(|&i| inked(i)).count();
    let hidden = (0..glyphs.len())
        .filter(|&i| inked(i) && mask.get(i) == Some(&true))
        .count();
    if run.actual_text || run.vertical || run.text.chars().any(is_right_to_left) {
        return if 2 * hidden > ink { covered_whole() } else { whole() };
    }
    if hidden > 0 && hidden == ink {
        return covered_whole();
    }
    let cut = split(glyphs, &mask, size, regions);
    if cut.kept.len() <= 1 && cut.covered.is_empty() {
        whole()
    } else {
        cut
    }
}

/// Appends `glyph`'s text to `words`, with a space first when the pen jumped past one.
fn push_glyph(words: &mut String, gap: f64, space: f64, glyph: &str) {
    if gap > space && !words.ends_with(char::is_whitespace) && !glyph.starts_with(char::is_whitespace) {
        words.push(' ');
    }
    words.push_str(glyph);
}

fn split(glyphs: &[Glyph], mask: &[bool], size: Pt, regions: &[LayoutRegion]) -> Cut {
    let (gutter, space) = (GUTTER * size.get(), SPACE * size.get());
    let mut kept: Vec<Part> = Vec::new();
    let mut covered: Vec<(String, RectPt)> = Vec::new();
    // The covered stretch being built, and whether the last glyph was in it.
    let mut stretch: Option<(String, RectPt)> = None;
    let mut last_region = None;
    for (glyph, &hidden) in glyphs.iter().zip(mask) {
        let r = quad_box(&glyph.quad);
        let blank = glyph.text.trim().is_empty();
        if hidden && !blank {
            // A stretch, like a part, never crosses a column gutter.
            match &mut stretch {
                Some((words, bbox)) if r.x0.get() - bbox.x1.get() <= gutter => {
                    push_glyph(words, r.x0.get() - bbox.x1.get(), space, &glyph.text);
                    *bbox = bbox.union(&r);
                }
                _ => {
                    if let Some((words, bbox)) = stretch.replace((glyph.text.clone(), r)) {
                        covered.push((words.trim_end().to_owned(), bbox));
                    }
                }
            }
            if let Some(part) = kept.last_mut()
                && r.x0.get() - part.reach <= gutter
            {
                part.reach = part.reach.max(r.x1.get());
                part.resumed = true;
            }
            continue;
        }
        if blank && let Some((words, _)) = &mut stretch {
            // A space inside a covered stretch stays in it; trimmed if the stretch ends here.
            words.push_str(&glyph.text);
            continue;
        }
        if let Some((words, bbox)) = stretch.take() {
            covered.push((words.trim_end().to_owned(), bbox));
        }
        // A space, or a glyph outside every region, stays with the part it follows: only a
        // glyph that lands in another region starts a new part.
        let region = if blank {
            last_region
        } else {
            region_near(regions, &r, size.get() / 2.0).or(last_region)
        };
        let same_region = kept.is_empty() || last_region.is_none() || region == last_region;
        last_region = region;
        let after_cover = kept.last().is_some_and(|p| p.resumed);
        match kept.last_mut() {
            Some(part) if same_region && r.x0.get() - part.reach <= gutter => {
                if part.resumed {
                    // The covered words leave one space between the visible ones around them.
                    if blank {
                        continue;
                    }
                    let trimmed = part.words.trim_end().len();
                    part.words.truncate(trimmed);
                    part.words.push(' ');
                    part.resumed = false;
                } else {
                    push_glyph(&mut part.words, r.x0.get() - part.bbox.x1.get(), space, &glyph.text);
                    part.bbox = part.bbox.union(&r);
                    part.reach = part.reach.max(r.x1.get());
                    continue;
                }
                part.words.push_str(glyph.text.trim_start());
                part.bbox = part.bbox.union(&r);
                part.reach = part.reach.max(r.x1.get());
            }
            _ if blank && after_cover => {}
            _ => kept.push(Part {
                words: glyph.text.clone(),
                bbox: r,
                reach: r.x1.get(),
                resumed: false,
            }),
        }
    }
    if let Some((words, bbox)) = stretch.take() {
        covered.push((words.trim_end().to_owned(), bbox));
    }
    let kept = kept.into_iter().map(|p| (p.words, p.bbox));
    if covered.is_empty() {
        return Cut {
            kept: kept.collect(),
            covered,
        };
    }
    // Around a cover, a part's own trailing space would only pad the line.
    Cut {
        kept: kept
            .map(|(w, b)| (w.trim_end().to_owned(), b))
            .filter(|(w, _)| !w.is_empty())
            .collect(),
        covered,
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_model::RectPt;

    use super::Covers;
    use crate::paint::Cover;

    /// A glyph box 10 × 10 at the origin.
    fn glyph() -> RectPt {
        RectPt::from_points(0.0, 0.0, 10.0, 10.0)
    }

    fn covers(rect: RectPt, after: u32) -> Covers {
        Covers::new(vec![Cover { rect, after }])
    }

    #[test]
    fn a_cover_needs_the_centre_and_half_the_box() {
        // Half the box, with the centre on its edge: covered.
        assert!(covers(RectPt::from_points(0.0, 0.0, 5.0, 10.0), 0).covers_box(&glyph(), 0));
        // The centre, but under half the box.
        assert!(!covers(RectPt::from_points(4.0, 4.0, 2.0, 2.0), 0).covers_box(&glyph(), 0));
        // Most of the box, but not the centre.
        assert!(!covers(RectPt::from_points(-10.0, 0.0, 14.9, 10.0), 0).covers_box(&glyph(), 0));
    }

    #[test]
    fn a_cover_painted_before_the_glyph_covers_nothing() {
        let page = covers(RectPt::from_points(-5.0, -5.0, 20.0, 20.0), 3);
        assert!(page.covers_box(&glyph(), 3));
        assert!(!page.covers_box(&glyph(), 4));
    }

    #[test]
    fn a_cover_far_from_the_glyph_is_never_tested() {
        let far = covers(RectPt::from_points(500.0, 500.0, 10.0, 10.0), 0);
        assert!(!far.covers_box(&glyph(), 0));
    }
}
