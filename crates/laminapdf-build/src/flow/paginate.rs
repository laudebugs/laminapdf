//! The paginator (ADR 0032 §3): greedy and deterministic. Paragraphs split between lines with at
//! least `orphans` lines at a page's foot and `widows` at the next one's head; a chain of blocks
//! kept together (a heading and what follows it, a `Keep` group) moves to the next page whole,
//! at most once, and is split with `flow-keep-broken` when it is taller than a page; a figure
//! moves whole. A paragraph continued on a page of another width is broken again at that width.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::items::{Content, Items};
use super::measure::MLine;
use super::model::PageTemplate;
use crate::error::{Error, Result, Warning};

/// Something placed on a page.
#[derive(Debug, Clone)]
pub(crate) enum Placed {
    /// A line of item `item`'s paragraph, its top at `top` and its left edge at `x`.
    Line {
        item: usize,
        line: Rc<MLine>,
        x: f64,
        top: f64,
    },
    /// A list item's marker beside its first line.
    Label {
        item: usize,
        line: Rc<MLine>,
        x: f64,
        top: f64,
    },
    /// Item `item`'s figure in a box, page space.
    Figure {
        item: usize,
        x: f64,
        top: f64,
        width: f64,
        height: f64,
    },
}

/// A laid-out page.
#[derive(Debug, Clone, Default)]
pub(crate) struct PageOut {
    pub(crate) placed: Vec<Placed>,
    /// The text of the last section heading started on or before the page.
    pub(crate) section: Option<String>,
}

/// Lays `items` out through `template`: every page, before any is drawn.
pub(crate) fn paginate(
    items: &Items,
    template: &PageTemplate,
    section_level: u8,
) -> Result<(Vec<PageOut>, Vec<Warning>)> {
    let mut pager = Pager {
        items,
        template,
        section_level,
        pages: Vec::new(),
        warnings: Vec::new(),
        measured: RefCell::new(BTreeMap::new()),
        y: 0.0,
        fresh: true,
        after: 0.0,
        section: None,
    };
    pager.new_page();
    let mut checked = 0;
    let mut i = 0;
    while let Some(item) = items.entries.get(i) {
        if item.break_before && !pager.fresh {
            pager.new_page();
        }
        match item.content {
            Content::Break => {
                if !pager.fresh {
                    pager.new_page();
                }
            }
            Content::Spacer(h) => pager.spacer(h),
            Content::Para(_) | Content::Figure(_) => {
                if i >= checked {
                    checked = pager.keep_chain(i)?;
                }
                pager.place(i)?;
            }
        }
        i += 1;
    }
    Ok((pager.pages, pager.warnings))
}

/// A paragraph's lines, shared by the cache and the pages.
type Lines = Rc<Vec<Rc<MLine>>>;

struct Pager<'a> {
    items: &'a Items,
    template: &'a PageTemplate,
    section_level: u8,
    pages: Vec<PageOut>,
    warnings: Vec<Warning>,
    /// Lines by (source, first byte, width bits), each paragraph broken once per width.
    measured: RefCell<BTreeMap<(usize, usize, u64), Lines>>,
    /// Where the next item starts, page space.
    y: f64,
    /// Nothing is placed on the page yet.
    fresh: bool,
    /// The space after the last item placed.
    after: f64,
    section: Option<String>,
}

impl Pager<'_> {
    fn page_template(&self) -> &PageTemplate {
        self.template.for_page(self.pages.len().saturating_sub(1))
    }

    fn bottom(&self) -> f64 {
        self.page_template().body_y().1
    }

    fn body_height(&self) -> f64 {
        let (top, bottom) = self.page_template().body_y();
        bottom - top
    }

    fn new_page(&mut self) {
        self.pages.push(PageOut {
            placed: Vec::new(),
            section: self.section.clone(),
        });
        self.y = self.page_template().body_y().0;
        self.fresh = true;
        self.after = 0.0;
    }

    fn gap(&self, i: usize) -> f64 {
        if self.fresh {
            return 0.0;
        }
        let before = self.items.entries.get(i).map_or(0.0, |it| it.space_before);
        self.after.max(before)
    }

    fn spacer(&mut self, h: f64) {
        if self.fresh {
            return;
        }
        if self.y + h > self.bottom() {
            self.new_page();
        } else {
            self.y += h;
            self.after = 0.0;
        }
    }

    /// Item `i`'s paragraph from byte `from`, broken at the current page's width.
    fn lines(&self, i: usize, from: usize) -> Result<Lines> {
        let Some(Content::Para(s)) = self.items.entries.get(i).map(|it| it.content) else {
            return Ok(Rc::new(Vec::new()));
        };
        let width = self.width(i);
        let key = (s, from, width.to_bits());
        if let Some(lines) = self.measured.borrow().get(&key) {
            return Ok(Rc::clone(lines));
        }
        let source = self.items.sources.get(s).ok_or(Error::InvalidGeometry {
            what: "a flow item names no paragraph",
        })?;
        let lines = Rc::new(source.lines(from, width)?.into_iter().map(Rc::new).collect::<Vec<_>>());
        self.measured.borrow_mut().insert(key, Rc::clone(&lines));
        Ok(lines)
    }

    /// The width item `i` is laid out at on the current page.
    fn width(&self, i: usize) -> f64 {
        let indent = self.items.entries.get(i).map_or(0.0, |it| it.indent);
        (self.page_template().body_width() - indent).max(1.0)
    }

    /// Item `i`'s figure box size on the current page.
    fn figure_size(&self, i: usize) -> (f64, f64) {
        let figure = match self.items.entries.get(i).map(|it| it.content) {
            Some(Content::Figure(f)) => self.items.figures.get(f),
            _ => None,
        };
        figure.map_or((0.0, 0.0), |f| {
            let width = f.width.unwrap_or(f64::MAX).min(self.width(i));
            let ratio = f64::from(f.image.height()) / f64::from(f.image.width().max(1));
            (width, width * ratio)
        })
    }

    /// How tall item `i` is whole (`whole`), or its first part on a page: `orphans` lines.
    fn demand(&self, i: usize, whole: bool) -> Result<f64> {
        let Some(item) = self.items.entries.get(i) else {
            return Ok(0.0);
        };
        Ok(match item.content {
            Content::Para(_) => {
                let lines = self.lines(i, 0)?;
                let n = if whole {
                    lines.len()
                } else {
                    item.orphans.min(lines.len())
                };
                lines.iter().take(n).map(|l| l.height).sum()
            }
            Content::Figure(_) => self.figure_size(i).1,
            Content::Spacer(h) => h,
            Content::Break => 0.0,
        })
    }

    /// If items from `i` are bound together, moves to a new page when they don't fit what is
    /// left of this one but would fit a fresh one, and warns when they wouldn't fit any page.
    /// Returns the first item after the chain.
    fn keep_chain(&mut self, i: usize) -> Result<usize> {
        let mut end = i;
        while self.items.binds(end) {
            end += 1;
        }
        if end == i {
            return Ok(i + 1);
        }
        // The chain's last item needs only its first part, unless the group holds it too.
        let last_whole = {
            let (a, b) = (self.items.entries.get(end - 1), self.items.entries.get(end));
            a.zip(b).is_some_and(|(a, b)| a.keep.is_some() && a.keep == b.keep)
        };
        let needed = |p: &Self| -> Result<f64> {
            let mut total = 0.0;
            let mut after = 0.0;
            for k in i..=end {
                let item = p.items.entries.get(k);
                let before = item.map_or(0.0, |it| it.space_before);
                if k > i {
                    total += f64::max(after, before);
                }
                total += p.demand(k, k < end || last_whole)?;
                after = item.map_or(0.0, |it| it.space_after);
            }
            Ok(total)
        };
        if self.y + self.gap(i) + needed(self)? > self.bottom() && !self.fresh {
            self.new_page();
        }
        if needed(self)? > self.body_height() {
            #[expect(clippy::cast_possible_truncation, reason = "page counts are small")]
            let page = self.pages.len().saturating_sub(1) as u32;
            self.warnings.push(Warning::FlowKeepBroken { page });
        }
        Ok(end + 1)
    }

    fn place(&mut self, i: usize) -> Result<()> {
        match self.items.entries.get(i).map(|it| it.content) {
            Some(Content::Para(_)) => self.place_para(i),
            Some(Content::Figure(_)) => self.place_figure(i),
            _ => Ok(()),
        }
    }

    fn place_figure(&mut self, i: usize) -> Result<()> {
        let (_, h) = self.figure_size(i);
        if self.y + self.gap(i) + h > self.bottom() && !self.fresh {
            self.new_page();
        }
        let (width, height) = self.figure_size(i);
        if height > self.body_height() {
            return Err(Error::FlowContentTooLarge { what: "figure" });
        }
        let item = self.items.entries.get(i).map_or(0.0, |it| it.indent);
        self.y += self.gap(i);
        let x = self.page_template().margins.left + item;
        self.push(Placed::Figure {
            item: i,
            x,
            top: self.y,
            width,
            height,
        });
        self.y += height;
        self.after = self.items.entries.get(i).map_or(0.0, |it| it.space_after);
        Ok(())
    }

    fn place_para(&mut self, i: usize) -> Result<()> {
        let Some(item) = self.items.entries.get(i) else {
            return Ok(());
        };
        let (widows, orphans) = (item.widows, item.orphans);
        let mut lines = self.lines(i, 0)?;
        let mut next = 0;
        let mut first = true;
        loop {
            let rest = lines.get(next..).unwrap_or_default();
            let room = self.bottom() - self.y - self.gap(i);
            let mut fit = 0;
            let mut used = 0.0;
            for line in rest {
                if used + line.height > room + 1e-6 {
                    break;
                }
                used += line.height;
                fit += 1;
            }
            let take = if fit >= rest.len() {
                rest.len()
            } else {
                let split = fit.min(rest.len().saturating_sub(widows));
                if split >= orphans.min(rest.len()) && split > 0 {
                    split
                } else if !self.fresh {
                    0
                } else if fit == 0 {
                    return Err(Error::FlowContentTooLarge { what: "line" });
                } else {
                    // Even a fresh page can't keep the rules: as many lines as fit.
                    fit
                }
            };
            if take > 0 {
                self.y += self.gap(i);
                let lines_here = rest.get(..take).unwrap_or_default().to_vec();
                self.draw_lines(i, &lines_here, first);
                first = false;
                next += take;
            }
            if next >= lines.len() {
                break;
            }
            let width = self.width(i);
            self.new_page();
            if (self.width(i) - width).abs() > 1e-9 {
                // Broken again at the new page's width, from the first line left.
                let from = lines.get(next).map_or(0, |l| l.line.range.start);
                lines = self.lines(i, from)?;
                next = 0;
            }
        }
        self.after = self.items.entries.get(i).map_or(0.0, |it| it.space_after);
        Ok(())
    }

    fn draw_lines(&mut self, i: usize, lines: &[Rc<MLine>], first: bool) {
        let Some(item) = self.items.entries.get(i) else { return };
        let left = self.page_template().margins.left + item.indent;
        if first {
            if item.heading == Some(self.section_level)
                && let Content::Para(s) = item.content
            {
                self.section = self.items.sources.get(s).map(|s| s.text.replace('\u{2028}', " "));
                if let Some(page) = self.pages.last_mut() {
                    page.section.clone_from(&self.section);
                }
            }
            if let Some((source, _)) = item.label {
                let label = self
                    .items
                    .sources
                    .get(source)
                    .and_then(|s| s.lines(0, f64::from(f32::MAX)).ok());
                if let (Some(line), Some(first_line)) = (label.and_then(|l| l.into_iter().next()), lines.first()) {
                    let x = left - line.line.width - 4.0;
                    let top = self.y + first_line.baseline - line.baseline;
                    self.push(Placed::Label {
                        item: i,
                        line: Rc::new(line),
                        x,
                        top,
                    });
                }
            }
        }
        for line in lines {
            self.push(Placed::Line {
                item: i,
                line: Rc::clone(line),
                x: left,
                top: self.y,
            });
            self.y += line.height;
        }
    }

    fn push(&mut self, placed: Placed) {
        if let Some(page) = self.pages.last_mut() {
            page.placed.push(placed);
        }
        self.fresh = false;
    }
}
