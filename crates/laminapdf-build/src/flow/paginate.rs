//! The paginator (ADR 0032 §3): greedy and deterministic. Paragraphs split between lines with at
//! least `orphans` lines at a page's foot and `widows` at the next one's head; a chain of blocks
//! kept together (a heading and what follows it, a `Keep` group) moves to the next page whole,
//! at most once, and is split with `flow-keep-broken` when it is taller than a page; a figure
//! moves whole. A paragraph continued on a page of another width is broken again at that width.
//! A table places its rows one by one: its header rows again at the top of each later page, no
//! body row split, and the footer rows on the last body row's page.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::items::{Content, Items};
use super::measure::MLine;
use super::model::PageTemplate;
use super::table::{Section, TableLayout, layout};
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
    /// Row `row` of item `item`'s table, its top at `top` and the table's left edge at `x`;
    /// `repeat` for a header row drawn again on a later page.
    Row {
        item: usize,
        table: Rc<TableLayout>,
        row: usize,
        x: f64,
        top: f64,
        repeat: bool,
    },
    /// A contents entry's page number at the end of its last line.
    Entry {
        item: usize,
        line: Rc<MLine>,
        x: f64,
        top: f64,
    },
    /// Dot leaders from `from` to `to` on the baseline `y` (a `Layout` artifact).
    Leader { from: f64, to: f64, y: f64 },
    /// A line of footnote `note`'s body at the foot of the page.
    NoteLine {
        note: usize,
        line: Rc<MLine>,
        x: f64,
        top: f64,
    },
    /// The rule above a page's footnotes (a `Layout` artifact).
    NoteRule { x: f64, y: f64, width: f64 },
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

/// A flow laid out.
#[derive(Debug)]
pub(crate) struct Laid {
    pub(crate) pages: Vec<PageOut>,
    pub(crate) warnings: Vec<Warning>,
    /// Where each item starts: the flow's page and the top of its first line or box.
    pub(crate) starts: Vec<Option<(usize, f64)>>,
}

/// Lays `items` out through `template`: every page, before any is drawn.
pub(crate) fn paginate(items: &Items, template: &PageTemplate, section_level: u8, first_page: usize) -> Result<Laid> {
    let mut pager = Pager {
        first_page,
        items,
        template,
        section_level,
        pages: Vec::new(),
        warnings: Vec::new(),
        measured: RefCell::new(BTreeMap::new()),
        tables: RefCell::new(BTreeMap::new()),
        y: 0.0,
        fresh: true,
        after: 0.0,
        section: None,
        notes: Vec::new(),
        notes_height: 0.0,
        starts: vec![None; items.entries.len()],
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
            Content::Para(_) | Content::Figure(_) | Content::Table(_) => {
                if i >= checked {
                    checked = pager.keep_chain(i)?;
                }
                pager.place(i)?;
            }
        }
        i += 1;
    }
    pager.finish_notes();
    // A final page break or spacer leaves a page with nothing on it: no blank page at the end.
    while pager.pages.len() > 1 && pager.pages.last().is_some_and(|p| p.placed.is_empty()) {
        pager.pages.pop();
    }
    Ok(Laid {
        pages: pager.pages,
        warnings: pager.warnings,
        starts: pager.starts,
    })
}

/// A paragraph's lines, shared by the cache and the pages.
pub(super) type Lines = Rc<Vec<Rc<MLine>>>;

pub(super) struct Pager<'a> {
    /// The document's index of the flow's first page, for warnings.
    first_page: usize,
    pub(super) items: &'a Items,
    template: &'a PageTemplate,
    section_level: u8,
    pub(super) pages: Vec<PageOut>,
    warnings: Vec<Warning>,
    /// Lines by (source, first byte, width bits), each paragraph broken once per width.
    pub(super) measured: RefCell<BTreeMap<(usize, usize, u64), Lines>>,
    /// Tables laid out, by (table, width bits).
    tables: RefCell<BTreeMap<(usize, u64), Rc<TableLayout>>>,
    /// Where the next item starts, page space.
    y: f64,
    /// Nothing is placed on the page yet.
    fresh: bool,
    /// The space after the last item placed.
    after: f64,
    section: Option<String>,
    /// The footnotes of the current page, in order, with their lines.
    pub(super) notes: Vec<(usize, Lines)>,
    /// Their height, the space above them included (0 with none).
    pub(super) notes_height: f64,
    /// Where each item starts (see [`Laid::starts`]).
    starts: Vec<Option<(usize, f64)>>,
}

impl Pager<'_> {
    pub(super) fn page_template(&self) -> &PageTemplate {
        self.template.for_page(self.pages.len().saturating_sub(1))
    }

    /// The bottom of the room left for the body: above the page's footnotes.
    fn bottom(&self) -> f64 {
        self.page_template().body_y().1 - self.notes_height
    }

    fn body_height(&self) -> f64 {
        let (top, bottom) = self.page_template().body_y();
        bottom - top
    }

    fn new_page(&mut self) {
        self.finish_notes();
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
        // The space after the block before it, then the spacer's own.
        if self.y + self.after + h > self.bottom() {
            self.new_page();
        } else {
            self.y += self.after + h;
            self.after = 0.0;
        }
    }

    /// Item `i`'s paragraph from byte `from`, broken at the current page's width.
    fn lines(&self, i: usize, from: usize) -> Result<Lines> {
        let Some(Content::Para(s)) = self.items.entries.get(i).map(|it| it.content) else {
            return Ok(Rc::new(Vec::new()));
        };
        self.source_lines(s, from, self.width(i))
    }

    /// Source `s` from byte `from` broken at `width`, measured once per flow.
    pub(super) fn source_lines(&self, s: usize, from: usize, width: f64) -> Result<Lines> {
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

    /// The width item `i` is laid out at on the current page: the body's, less its indent
    /// and the room it keeps at the end of its lines.
    fn width(&self, i: usize) -> f64 {
        let (indent, reserve) = self
            .items
            .entries
            .get(i)
            .map_or((0.0, 0.0), |it| (it.indent, it.reserve));
        (self.page_template().body_width() - indent - reserve).max(1.0)
    }

    /// Item `i`'s figure box size on the current page.
    fn figure_size(&self, i: usize) -> (f64, f64) {
        let figure = match self.items.entries.get(i).map(|it| it.content) {
            Some(Content::Figure(f)) => self.items.figures.get(f),
            _ => None,
        };
        figure.map_or((0.0, 0.0), |f| {
            let frame = self.width(i);
            let width = f
                .width
                .filter(|w| w.is_finite() && *w > 0.0)
                .map_or(frame, |w| w.min(frame));
            let ratio = f64::from(f.image.height()) / f64::from(f.image.width().max(1));
            (width, width * ratio)
        })
    }

    /// How tall item `i` is whole (`whole`), or its first part on a page: `orphans` lines, or the
    /// whole paragraph when it can't split.
    fn demand(&self, i: usize, whole: bool) -> Result<f64> {
        let Some(item) = self.items.entries.get(i) else {
            return Ok(0.0);
        };
        Ok(match item.content {
            Content::Para(_) => {
                let lines = self.lines(i, 0)?;
                // A paragraph too short to split keeping both rules moves whole.
                let n = if whole || lines.len() < item.orphans + item.widows {
                    lines.len()
                } else {
                    item.orphans
                };
                lines.iter().take(n).map(|l| l.height).sum()
            }
            Content::Figure(_) => self.figure_size(i).1,
            Content::Table(t) => {
                let (Some(table), layout) = (self.items.tables.get(t), self.table(i)?) else {
                    return Ok(0.0);
                };
                if whole {
                    layout.rows.iter().map(|r| r.height).sum()
                } else {
                    // Its header and first body row, and the footer too when that row is the last.
                    let head = layout.height(table, Section::Head);
                    let body = table.count(Section::Body);
                    let first = layout.rows.get(table.count(Section::Head)).map_or(0.0, |r| r.height);
                    let foot = if body <= 1 {
                        layout.height(table, Section::Foot)
                    } else {
                        0.0
                    };
                    head + if body == 0 { 0.0 } else { first } + foot
                }
            }
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
        // The chain's last item needs only its first part, unless a `Keep` group holds it.
        let last_whole = self.items.entries.get(end).is_some_and(|it| it.keep.is_some());
        if end == i && !last_whole {
            return Ok(i + 1);
        }
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
            self.warn_keep_broken();
        }
        Ok(end + 1)
    }

    fn place(&mut self, i: usize) -> Result<()> {
        match self.items.entries.get(i).map(|it| it.content) {
            Some(Content::Para(_)) => self.place_para(i),
            Some(Content::Figure(_)) => self.place_figure(i),
            Some(Content::Table(_)) => self.place_table(i),
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

    /// Item `i`'s table laid out at the width it keeps on every page: the narrower of this page's
    /// and the later pages' (all of which use the template's main page).
    fn table(&self, i: usize) -> Result<Rc<TableLayout>> {
        let (Some(Content::Table(t)), indent) = self
            .items
            .entries
            .get(i)
            .map_or((None, 0.0), |it| (Some(it.content), it.indent))
        else {
            return Err(Error::InvalidGeometry {
                what: "a flow item names no table",
            });
        };
        let later = (self.template.for_page(usize::MAX).body_width() - indent).max(1.0);
        let width = self.width(i).min(later);
        let key = (t, width.to_bits());
        if let Some(layout) = self.tables.borrow().get(&key) {
            return Ok(Rc::clone(layout));
        }
        let table = self.items.tables.get(t).ok_or(Error::InvalidGeometry {
            what: "a flow item names no table",
        })?;
        let laid = Rc::new(layout(table, &self.items.sources, width)?);
        self.tables.borrow_mut().insert(key, Rc::clone(&laid));
        Ok(laid)
    }

    fn place_table(&mut self, i: usize) -> Result<()> {
        let Some(Content::Table(t)) = self.items.entries.get(i).map(|it| it.content) else {
            return Ok(());
        };
        let Some(table) = self.items.tables.get(t) else {
            return Ok(());
        };
        let start = self.demand(i, false)?;
        if self.y + self.gap(i) + start > self.bottom() + 1e-6 && !self.fresh {
            self.new_page();
        }
        let layout = self.table(i)?;
        let heads = table.count(Section::Head);
        let head = layout.height(table, Section::Head);
        let foot = layout.height(table, Section::Foot);
        let tallest = layout.rows.iter().skip(heads).map(|r| r.height).fold(0.0, f64::max);
        // Every row but the first may land on a later page, under the header copies.
        let (top, bottom) = self.template.for_page(usize::MAX).body_y();
        if head + tallest > self.body_height().min(bottom - top) + 1e-6 {
            return Err(Error::FlowContentTooLarge { what: "table row" });
        }
        self.y += self.gap(i);
        self.place_rows(i, t, &layout, 0..heads, false)?;
        let last_body = layout.rows.len() - table.count(Section::Foot);
        for row in heads..layout.rows.len() {
            let height = layout.rows.get(row).map_or(0.0, |r| r.height);
            // The last body row needs room for the footer rows too, and a row for its notes.
            let notes = self.notes_cost(&[], &self.row_notes(t, row))?;
            let needed = notes + if row + 1 == last_body { height + foot } else { height };
            if self.y + needed > self.bottom() + 1e-6 {
                // A row that fits here moves only when a fresh page holds what it needs.
                if self.y + height > self.bottom() + 1e-6 || head + needed <= self.body_height() + 1e-6 {
                    self.new_page();
                    self.place_rows(i, t, &layout, 0..heads, true)?;
                }
                if self.y + needed > self.bottom() + 1e-6 {
                    // The last body row and the footer fit no page together.
                    self.warn_keep_broken();
                }
            }
            self.place_rows(i, t, &layout, row..row + 1, false)?;
        }
        self.after = self.items.entries.get(i).map_or(0.0, |it| it.space_after);
        Ok(())
    }

    /// Places rows `rows` of item `i`'s table from the current position, at the current page's
    /// left margin.
    fn place_rows(
        &mut self,
        i: usize,
        t: usize,
        layout: &Rc<TableLayout>,
        rows: std::ops::Range<usize>,
        repeat: bool,
    ) -> Result<()> {
        let x = self.page_template().margins.left + self.items.entries.get(i).map_or(0.0, |it| it.indent);
        for row in rows {
            // A header row's notes go on the page of its first drawing only.
            if !repeat {
                for note in self.row_notes(t, row) {
                    self.add_note(note)?;
                }
            }
            let height = layout.rows.get(row).map_or(0.0, |r| r.height);
            self.push(Placed::Row {
                item: i,
                table: Rc::clone(layout),
                row,
                x,
                top: self.y,
                repeat,
            });
            self.y += height;
        }
        Ok(())
    }

    fn warn_keep_broken(&mut self) {
        #[expect(clippy::cast_possible_truncation, reason = "page counts are small")]
        let page = (self.first_page + self.pages.len()).saturating_sub(1) as u32;
        self.warnings.push(Warning::FlowKeepBroken { page });
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
            // A line fits with the footnotes it cites, which take room at the page's foot.
            let mut cited = Vec::new();
            for line in rest {
                let notes = self.line_notes(i, line);
                let more = self.notes_cost(&cited, &notes)?;
                if used + line.height + more > room + 1e-6 {
                    break;
                }
                used += line.height + more;
                cited.extend(notes);
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
                for line in &lines_here {
                    for note in self.line_notes(i, line) {
                        self.add_note(note)?;
                    }
                }
                let last = next + take >= lines.len();
                self.draw_lines(i, &lines_here, first, last)?;
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

    fn draw_lines(&mut self, i: usize, lines: &[Rc<MLine>], first: bool, last: bool) -> Result<()> {
        let Some(item) = self.items.entries.get(i) else {
            return Ok(());
        };
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
            if let Some((source, _)) = item.label
                && let (Some(line), Some(first_line)) = (self.label(source)?, lines.first())
            {
                let x = left - line.line.width - 4.0;
                let top = self.y + first_line.baseline - line.baseline;
                self.push(Placed::Label { item: i, line, x, top });
            }
        }
        let entry = item.entry.filter(|_| last);
        let right = self.page_template().margins.left + self.page_template().body_width();
        for (n, line) in lines.iter().enumerate() {
            self.push(Placed::Line {
                item: i,
                line: Rc::clone(line),
                x: left,
                top: self.y,
            });
            if let Some(source) = entry.filter(|_| n + 1 == lines.len())
                && let Some(number) = self.label(source)?
            {
                // The page number at the right edge on the entry's last line, dots before it.
                let x = right - number.line.width;
                let baseline = self.y + line.baseline;
                let from = left + line.line.x + line.line.width;
                self.push(Placed::Leader {
                    from,
                    to: x,
                    y: baseline,
                });
                let top = baseline - number.baseline;
                self.push(Placed::Entry {
                    item: i,
                    line: number,
                    x,
                    top,
                });
            }
            self.y += line.height;
        }
        Ok(())
    }

    /// The marker `source` as one line, measured once per flow (an error is the flow's).
    fn label(&self, source: usize) -> Result<Option<Rc<MLine>>> {
        let key = (source, 0, u64::MAX);
        if let Some(lines) = self.measured.borrow().get(&key) {
            return Ok(lines.first().cloned());
        }
        let Some(s) = self.items.sources.get(source) else {
            return Ok(None);
        };
        let lines: Lines = Rc::new(s.lines(0, f64::from(f32::MAX))?.into_iter().map(Rc::new).collect());
        self.measured.borrow_mut().insert(key, Rc::clone(&lines));
        Ok(lines.first().cloned())
    }

    fn push(&mut self, placed: Placed) {
        let start = match &placed {
            Placed::Line { item, top, .. } | Placed::Figure { item, top, .. } | Placed::Row { item, top, .. } => {
                Some((*item, *top))
            }
            _ => None,
        };
        let page = self.pages.len().saturating_sub(1);
        if let Some((item, top)) = start
            && let Some(slot @ None) = self.starts.get_mut(item)
        {
            *slot = Some((page, top));
        }
        if let Some(page) = self.pages.last_mut() {
            page.placed.push(placed);
        }
        self.fresh = false;
    }
}
