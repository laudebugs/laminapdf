//! Flowing layout (ADR 0032): blocks of styled text, lists and figures poured through a page
//! template into as many pages as they need.
//!
//! [`Builder::flow`] measures every block, breaks every page, and only then draws, so a footer's
//! `{pages}` is known when it is drawn. Paragraphs split between lines with at least
//! [`ParagraphStyle::with_widows_and_orphans`] lines at a page's foot and head (2 and 2 by
//! default); a heading stays with the block after it; a [`FlowBlock::Keep`] group and a figure
//! move whole. Output is tagged unless [`Flow::with_tagged`] turns it off: `P`, `H1`–`H6`, `L`,
//! `LI`, `Lbl`, `LBody`, `Figure` with `/Alt`, `Link` and `Span` with `/Lang`, each one element
//! however many pages it crosses, and the running header and footer as `Pagination` artifacts
//! with `/Subtype /Header` and `/Footer`.
//!
//! Fonts must be shapable: the standard 14 fail with `font-not-shapable`.
//!
//! ```
//! # fn font() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
//! use laminapdf_build::flow::{Flow, FlowBlock, PageRegion, PageTemplate, Paragraph, TextStyle};
//! use laminapdf_build::{BuildOptions, Builder, PageSize};
//!
//! let mut b = Builder::new(BuildOptions::new());
//! let font = b.font(laminapdf_text::Font::from_bytes(font())?)?;
//! let body = TextStyle::new(&font, 11.0);
//! let footer = PageRegion::new().with_center("Page {page} of {pages}");
//! let template = PageTemplate::new(PageSize::A4).with_furniture(None, Some(footer), &body.clone().with_size(9.0));
//! let mut flow = Flow::new(template);
//! for _ in 0..80 {
//!     flow = flow.block(FlowBlock::Paragraph(Paragraph::plain("A sentence that fills the page. ".repeat(6), &body)));
//! }
//! let report = b.flow(&flow)?;
//! assert!(report.pages.len() > 2);
//! b.finish()?;
//! # Ok::<(), laminapdf_build::Error>(())
//! ```

mod contents;
mod draw_table;
mod emit;
mod items;
mod measure;
mod model;
mod notes;
mod paginate;
mod table;
mod template;
#[cfg(test)]
mod tests;

use laminapdf_model::{PageId, Pt};

pub use contents::Contents;
pub use model::{
    Figure, Flow, FlowBlock, Inline, List, ListItem, ListKind, Margins, PageRegion, PageTemplate, Paragraph,
    ParagraphStyle, TextStyle,
};
pub use table::{ColumnWidth, Table, TableCell, TableRow, TableRule, VerticalAlign};
pub use template::{Fields, fill_fields};

use crate::builder::Builder;
use crate::error::{Result, Warning};
use crate::nav::{Fit, LinkTarget};

/// What [`Builder::flow`] did.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FlowReport {
    /// The pages it added, in order.
    pub pages: Vec<PageId>,
    /// What it worked around (`flow-keep-broken`).
    pub warnings: Vec<Warning>,
}

impl Builder {
    /// Lays `flow` out and adds its pages after any already added. `{page}` and `{pages}` count
    /// the document's pages, those added before the flow included; the first-page template is
    /// the flow's first page.
    ///
    /// Every heading gets a named destination, and the outline gains an item per heading
    /// unless [`Flow::with_outline`] turns it off (each flow adds its own items after the
    /// outline's). A flow with a [`Contents`] block is laid out again until the page numbers
    /// it prints stop changing, at most four times.
    ///
    /// # Errors
    /// [`Error::FlowContentTooLarge`](crate::Error::FlowContentTooLarge) for a line or a
    /// figure taller than a page's body; [`Error::FlowLayoutUnstable`](crate::Error::FlowLayoutUnstable)
    /// for a contents page whose numbers don't settle; shaping errors (`missing-glyph`,
    /// `font-not-shapable`); the errors of finishing a page. Pages drawn before a failure stay
    /// in the document, and the flow's structure elements are closed.
    pub fn flow(&mut self, flow: &Flow) -> Result<FlowReport> {
        let first = self.page_count();
        let mut items = items::Items::new(&flow.blocks, first);
        let measures = paginate::Measures::default();
        let lay =
            |items: &items::Items| paginate::paginate(items, &flow.template, flow.section_level, first, &measures);
        let contents = contents::has_contents(&flow.blocks);
        let laid = if contents {
            // Every layout but the numbers' is measured once: only the numbers change.
            contents::settle(vec![first + 1; items.headings.len()], |numbers| {
                items.set_numbers(numbers);
                measures.forget(&items.number_sources());
                let laid = lay(&items)?;
                let found = items
                    .heading_at
                    .iter()
                    .map(|&i| {
                        laid.starts
                            .get(i)
                            .copied()
                            .flatten()
                            .map_or(0, |(page, _)| first + page + 1)
                    })
                    .collect();
                Ok((laid, found))
            })?
        } else {
            lay(&items)?
        };
        let pages = emit::emit(self, flow, &items, &laid.pages, first)?;
        if contents || flow.outline {
            for (n, &item) in items.heading_at.iter().enumerate() {
                if let Some(Some((page, top))) = laid.starts.get(item) {
                    let index = u32::try_from(first + page).unwrap_or(u32::MAX);
                    let fit = Fit::Xyz {
                        left: None,
                        top: Some(Pt::new(*top)),
                        zoom: None,
                    };
                    self.destination(contents::destination(first, n), index, fit);
                }
            }
        }
        if flow.outline && !items.headings.is_empty() {
            self.outline(contents::outline(&items.headings, |n| {
                LinkTarget::Named(contents::destination(first, n))
            }))?;
        }
        Ok(FlowReport {
            pages,
            warnings: laid.warnings,
        })
    }
}
