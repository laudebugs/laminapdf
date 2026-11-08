//! Greedy line breaking over one paragraph's items.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

use super::span::{Group, Style};
use super::{Alignment, LayoutOptions, Line, LineRun};
use crate::bidi::BidiParagraph;
use crate::error::Result;
use crate::font::Font;
use crate::itemize::Item;
use crate::linebreak::{grapheme_boundaries, is_hard_break, is_trailing_space, line_breaks};
use crate::shape::{Direction, ShapeOptions, ShapedRun, Shaper};

pub(super) struct Layouter<'a, S> {
    pub(super) shaper: &'a S,
    pub(super) fonts: &'a [Font],
    /// The paragraph's text: every range here is relative to it.
    pub(super) text: &'a str,
    pub(super) options: &'a LayoutOptions,
    pub(super) para: &'a BidiParagraph<'a>,
    pub(super) items: &'a [Item],
    /// The paragraph's groups of equally styled spans.
    pub(super) groups: &'a [Group],
    /// Each item's group: an index into `groups`.
    pub(super) item_groups: &'a [usize],
    /// The paragraph's own style, for a paragraph without spans (an empty text).
    pub(super) default_style: Style,
    /// Every run shaped so far, by (item index, range, level): shaping is deterministic, so
    /// the estimates' runs serve lines that hold an item whole (most short texts), and the
    /// candidates line breaking tries again (backing off, trying a longer line, splitting an
    /// overlong word, the line finally kept) are shaped once.
    pub(super) shaped: RefCell<BTreeMap<ShapeKey, Option<Rc<ShapedRun>>>>,
}

/// What a run is shaped from in a paragraph: an item, a range of it (start, end), a bidi level.
type ShapeKey = (usize, usize, usize, u8);

/// A candidate line: its runs (shared with the memo) and where they go, before it is kept.
struct Draft {
    range: Range<usize>,
    content: Range<usize>,
    /// Each run with its item index, level and left edge, in visual order.
    runs: Vec<(Rc<ShapedRun>, usize, u8, f64)>,
    width: f64,
}

impl<S: Shaper> Layouter<'_, S> {
    /// Greedy (first-fit) lines. Candidates are estimated from the paragraph shaped once
    /// (the advances of each cluster), then the chosen line is shaped as it is drawn and
    /// moved back or forward one opportunity at a time until it is the longest that fits.
    pub(super) fn lines(&self) -> Result<Vec<Line>> {
        let limit = f64::from(self.options.width);
        let estimate = self.estimates()?;
        let estimated = |range: Range<usize>| {
            let content = self.content(range);
            estimate.get(content.end).copied().unwrap_or(0.0) - estimate.get(content.start).copied().unwrap_or(0.0)
        };
        let breaks = line_breaks(self.text);
        let mut lines = Vec::new();
        let mut start = 0;
        let mut i = 0;
        while let Some(first) = breaks.get(i) {
            // The furthest opportunity the estimate fits, never past a mandatory break.
            let mut j = i;
            if !first.mandatory {
                while let Some(next) = breaks.get(j + 1) {
                    if estimated(start..next.offset) > limit {
                        break;
                    }
                    j += 1;
                    if next.mandatory {
                        break;
                    }
                }
            }
            let end_at = |k: usize| breaks.get(k).map_or(self.text.len(), |b| b.offset);
            let mut line = self.draft(start..end_at(j))?;
            while line.width > limit && j > i {
                j -= 1;
                line = self.draft(start..end_at(j))?;
            }
            if line.width > limit {
                // Nothing fits: break the overlong stretch between grapheme clusters.
                let (full, rest) = self.split_overlong(start..end_at(i), limit)?;
                for range in full {
                    lines.push(self.line(self.draft(range)?));
                }
                start = rest;
                let remainder = self.draft(start..end_at(i))?;
                if remainder.width > limit || first.mandatory {
                    // A single cluster wider than the box, or the paragraph's end.
                    lines.push(self.line(remainder));
                    start = end_at(i);
                    i += 1;
                }
                continue;
            }
            while breaks.get(j).is_some_and(|b| !b.mandatory) && j + 1 < breaks.len() {
                let longer = self.draft(start..end_at(j + 1))?;
                if longer.width > limit {
                    break;
                }
                line = longer;
                j += 1;
            }
            start = end_at(j);
            lines.push(self.line(line));
            i = j + 1;
        }
        Ok(lines)
    }

    /// Prefix sums of the paragraph's advances in points by byte offset: each item shaped
    /// once at its own level, every glyph's advance counted at its cluster.
    fn estimates(&self) -> Result<Vec<f64>> {
        let mut at = vec![0.0; self.text.len() + 1];
        for (i, item) in self.items.iter().enumerate() {
            let Some(run) = self.shaped(i, item, item.range.clone(), item.level)? else {
                continue;
            };
            for glyph in &run.glyphs {
                if let Some(slot) = at.get_mut(glyph.cluster) {
                    *slot += crate::shape::to_points(i64::from(glyph.x_advance), run.units_per_em, self.style(i).size);
                }
            }
        }
        let mut sum = 0.0;
        let mut prefix = Vec::with_capacity(at.len());
        prefix.push(0.0);
        for v in at.iter().take(self.text.len()) {
            sum += v;
            prefix.push(sum);
        }
        Ok(prefix)
    }

    /// `range` without trailing spaces and hard breaks.
    fn content(&self, range: Range<usize>) -> Range<usize> {
        let line = self.text.get(range.clone()).unwrap_or_default();
        let kept = line.trim_end_matches(|ch| is_trailing_space(ch) || is_hard_break(ch));
        range.start..range.start + kept.len()
    }

    /// The items intersecting `range`, clipped to it, in logical order (items are sorted and
    /// contiguous, so a binary search finds the first).
    fn parts(&self, range: Range<usize>) -> impl Iterator<Item = (usize, &Item, Range<usize>)> + '_ {
        let first = self.items.partition_point(|item| item.range.end <= range.start);
        self.items
            .get(first..)
            .unwrap_or_default()
            .iter()
            .zip(first..)
            .take_while(move |(item, _)| item.range.start < range.end)
            .map(move |(item, i)| {
                (
                    i,
                    item,
                    item.range.start.max(range.start)..item.range.end.min(range.end),
                )
            })
    }

    /// `range` of item `i` shaped at `level`, from the memo or shaped now; `None` when the
    /// item's font is not in the list, which [`itemize`] only allows for text with nothing to
    /// draw.
    fn shaped(&self, i: usize, item: &Item, range: Range<usize>, level: u8) -> Result<Option<Rc<ShapedRun>>> {
        let key = (i, range.start, range.end, level);
        if let Some(run) = self.shaped.borrow().get(&key) {
            return Ok(run.clone());
        }
        let run = match self.fonts.get(item.font) {
            Some(font) => {
                let style = self.style(i);
                let options = ShapeOptions {
                    script: Some(item.script),
                    language: style.language.clone(),
                    direction: Some(Direction::from_level(level)),
                    features: style.features.clone(),
                };
                Some(Rc::new(self.shaper.shape_range(font, self.text, range, &options)?))
            }
            None => None,
        };
        self.shaped.borrow_mut().insert(key, run.clone());
        Ok(run)
    }

    /// The candidate line `range`: shaped as drawn (visual runs at their post-L1 levels),
    /// reordered and measured.
    fn draft(&self, range: Range<usize>) -> Result<Draft> {
        let content = self.content(range.clone());
        let mut runs = Vec::new();
        let mut x = 0.0;
        for visual in self.para.visual_runs(content.clone())? {
            let mut parts: Vec<(usize, &Item, Range<usize>)> = self.parts(visual.range.clone()).collect();
            if visual.direction().is_rtl() {
                parts.reverse();
            }
            for (i, item, part) in parts {
                let Some(run) = self.shaped(i, item, part, visual.level)? else {
                    continue;
                };
                let width = run.width(self.style(i).size);
                runs.push((run, i, visual.level, x));
                x += width;
            }
        }
        Ok(Draft {
            range,
            content,
            runs,
            width: x,
        })
    }

    /// The line kept from `draft`: its runs copied out of the memo, aligned.
    fn line(&self, draft: Draft) -> Line {
        let Draft {
            range,
            content,
            runs,
            width,
        } = draft;
        let room = f64::from(self.options.width) - width;
        let rtl = self.para.base_direction().is_rtl();
        let offset = match (self.options.align, rtl) {
            (Alignment::Left, _) | (Alignment::Start, false) | (Alignment::End, true) => 0.0,
            (Alignment::Right, _) | (Alignment::Start, true) | (Alignment::End, false) => room,
            (Alignment::Center, _) => room / 2.0,
        };
        let hard_break = self
            .text
            .get(range.clone())
            .and_then(|t| t.chars().next_back())
            .is_some_and(is_hard_break);
        let runs: Vec<LineRun> = runs
            .into_iter()
            .filter_map(|(run, i, level, x)| {
                let item = self.items.get(i)?;
                let group = self.group(i)?;
                Some(LineRun {
                    run: Rc::unwrap_or_clone(run),
                    font: item.font,
                    level,
                    x,
                    spans: group.spans.clone(),
                    size: group.style.size,
                    rise: group.style.rise,
                })
            })
            .collect();
        let (ascent, descent) = self.metrics(&runs, range.start);
        Line {
            range,
            content,
            runs,
            width,
            x: offset,
            hard_break,
            ascent,
            descent,
        }
    }

    /// Item `i`'s group.
    fn group(&self, i: usize) -> Option<&Group> {
        self.item_groups.get(i).and_then(|&g| self.groups.get(g))
    }

    /// Item `i`'s style (every item has a group, so the fallback is never used).
    fn style(&self, i: usize) -> &Style {
        self.group(i).map_or(&self.default_style, |g| &g.style)
    }

    /// A line's ascent and descent in points from its baseline: the extremes of its runs'
    /// fonts at their sizes, moved by their rises. A line with no runs (an empty paragraph, a
    /// line of only a hard break) takes them from the first font of the span it starts in.
    fn metrics(&self, runs: &[LineRun], start: usize) -> (f64, f64) {
        let extent = |font: usize, size: f32, rise: f32| {
            self.fonts.get(font).map(|f| {
                let info = f.info();
                let scale = f64::from(size) / f64::from(info.units_per_em.max(1));
                (
                    f64::from(info.ascent) * scale + f64::from(rise),
                    f64::from(info.descent) * scale + f64::from(rise),
                )
            })
        };
        let mut extents: Vec<(f64, f64)> = runs.iter().filter_map(|r| extent(r.font, r.size, r.rise)).collect();
        if extents.is_empty() {
            let style = self
                .groups
                .iter()
                .find(|g| g.range.contains(&start))
                .or(self.groups.last())
                .map_or(&self.default_style, |g| &g.style);
            let font = style.fonts.first().copied().unwrap_or(0);
            extents.extend(extent(font, style.size, style.rise));
        }
        extents
            .into_iter()
            .fold((0.0, 0.0), |(a, d), (ra, rd)| (a.max(ra), d.min(rd)))
    }

    /// Splits `range`, too wide for one line, into lines that fit between grapheme clusters
    /// (at least one cluster per line; spaces after a line's last cluster hang on it, so no
    /// line holds only spaces). Returns the full lines and where the remainder starts: the
    /// remainder fits, or is a single cluster wider than the box.
    fn split_overlong(&self, range: Range<usize>, limit: f64) -> Result<(Vec<Range<usize>>, usize)> {
        let segment = self.text.get(range.clone()).unwrap_or_default();
        let bounds: Vec<usize> = grapheme_boundaries(segment)
            .into_iter()
            .map(|b| range.start + b)
            .filter(|&b| b > range.start)
            .collect();
        // A line ending at `end` takes the spaces right after it.
        let hang = |end: usize| {
            let rest = self.text.get(end..range.end).unwrap_or_default();
            end + (rest.len() - rest.trim_start_matches(is_trailing_space).len())
        };
        let mut lines = Vec::new();
        let mut start = range.start;
        let mut last_fit: Option<usize> = None;
        let mut i = 0;
        while let Some(&b) = bounds.get(i) {
            if b <= start {
                i += 1;
                continue;
            }
            if self.draft(start..b)?.width <= limit {
                last_fit = Some(b);
                i += 1;
                continue;
            }
            let end = match last_fit.take() {
                Some(end) => end,
                None if bounds.get(i + 1).is_none() => break,
                // One cluster wider than the box: it gets a line of its own.
                None => b,
            };
            let end = hang(end);
            if end >= range.end {
                break;
            }
            lines.push(start..end);
            start = end;
        }
        Ok((lines, start))
    }
}
