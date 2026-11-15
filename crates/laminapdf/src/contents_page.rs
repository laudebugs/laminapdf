//! A contents page made from the bookmarks (E3 plan §2.2): a page or pages listing the outline
//! one or two levels deep, each entry linked to its bookmark's place, inserted as one undo step.
//!
//! It isn't an op: an op can't draw text. The page is laid out with the builder's flowing
//! layout into a document of its own, which becomes a source; then one transaction inserts its
//! pages ([`InsertPagesFrom`]), links each entry ([`AddLink`]) and, when asked, adds a bookmark
//! for it ([`AddOutlineItem`]). The op log holds those ops and the source's bytes, so a replay
//! draws the same page everywhere.

use laminapdf_cos::ObjRef;
use laminapdf_model::{PageId, PageLabels, PagePosition, PageSize, Pt, RectPt, SiblingPosition, UserSpaceRect};
use laminapdf_ops::{
    AddLink, AddOutlineItem, DestinationView, ExplicitDestination, InsertPagesFrom, Origin, OutlineTarget,
};

use crate::builder::Builder;
use crate::document::{Document, Edited};
use crate::error::Result;
use crate::flow::{Contents, ContentsEntry, ContentsEntryBox, Flow, FlowBlock, PageTemplate, Paragraph, TextStyle};
use crate::navigation::OutlineNode;
use crate::{BuildOptions, Font, Standard14};

/// How [`Document::create_contents_page`] makes the contents page.
///
/// Added in 0.5.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ContentsOptions {
    /// Where the page goes.
    pub at: PagePosition,
    /// How many levels of bookmarks it lists: 1 or 2.
    pub levels: u8,
    /// The heading at its top.
    pub title: String,
    /// Whether a bookmark for the contents page is added first in the outline.
    pub bookmark: bool,
    /// The font of its text; `None` for the standard 14 Helvetica, which isn't embedded, so a
    /// file that claims PDF/A or PDF/UA-1 refuses it (`font-not-embedded`).
    pub font: Option<Font>,
    /// Fonts for the characters [`ContentsOptions::font`] lacks, tried in order.
    pub fallbacks: Vec<Font>,
    /// The entries' size, points; the heading is half again as large.
    pub size: f64,
}

impl Default for ContentsOptions {
    fn default() -> Self {
        Self {
            at: PagePosition::Start,
            levels: 2,
            title: "Contents".to_owned(),
            bookmark: false,
            font: None,
            fallbacks: Vec::new(),
            size: 11.0,
        }
    }
}

impl ContentsOptions {
    /// The defaults: before the first page, two levels, titled "Contents", no bookmark of its
    /// own, Helvetica at 11 points.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserted at `at`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_at(mut self, at: PagePosition) -> Self {
        self.at = at;
        self
    }

    /// Listing `levels` levels of bookmarks (clamped to 1 or 2).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_levels(mut self, levels: u8) -> Self {
        self.levels = levels.clamp(1, 2);
        self
    }

    /// Headed `title`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// With a bookmark for the contents page, first in the outline.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_bookmark(mut self, bookmark: bool) -> Self {
        self.bookmark = bookmark;
        self
    }

    /// Set in `font`, embedded and subset.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// With `font` for the characters the main font lacks, after any fallbacks given before.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_fallback(mut self, font: Font) -> Self {
        self.fallbacks.push(font);
        self
    }

    /// The entries at `size` points.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }
}

/// A bookmark the page lists: its level, its title and where its link goes.
struct Listed {
    level: u8,
    title: String,
    page: usize,
    target: OutlineTarget,
}

/// The most times the page is laid out while its page count, and so the labels it prints,
/// settle: one page or more pushes the bookmarks' pages along by that many.
const MAX_LAYOUTS: usize = 3;

impl Document {
    /// Adds a contents page made from the bookmarks: their titles one or two levels deep, each
    /// indented by its level, with dot leaders to the page label a reader will see once the
    /// page is in, each entry a link to its bookmark's place. Bookmarks that go to no page of
    /// this document (a web address, another file) are left out. On a tagged document the page
    /// is tagged (`TOC`, `TOCI`) and joins the structure tree.
    ///
    /// It is one transaction, so one undo step, of ops the document already knows: the page is
    /// laid out as a document of its own and added as a source, then
    /// [`InsertPagesFrom`](crate::InsertPagesFrom) inserts it, an
    /// [`AddLink`](crate::AddLink) links each entry and, with
    /// [`ContentsOptions::bookmark`], an [`AddOutlineItem`](crate::AddOutlineItem) adds its
    /// bookmark. The pages it inserted are those [`Edited::page_order`] holds that weren't there
    /// before.
    ///
    /// # Errors
    /// `font-not-embedded` for the standard font on a file that claims PDF/A or PDF/UA-1;
    /// `no-bookmarks` when no bookmark goes to a page of this document; `missing-glyph` for a
    /// character of a title that neither the font nor its fallbacks have, and the other errors
    /// of laying the page out; the ops' errors; `saved-not-rebased`.
    ///
    /// Added in 0.5.0.
    pub fn create_contents_page(&mut self, options: &ContentsOptions) -> Result<Edited> {
        self.refuse_standard_font(options)?;
        let listed = self.listed(options.levels.clamp(1, 2))?;
        if listed.is_empty() {
            return Err(crate::Error::NoBookmarks);
        }
        let infos = self.page_infos()?;
        let size = infos
            .first()
            .map_or(PageSize::A4, |p| PageSize::new(Pt::new(p.width), Pt::new(p.height)));
        let before: Vec<PageId> = infos.iter().map(|p| p.id.clone()).collect();
        let labels = self.ops().page_labels()?;
        let tagged = self.is_tagged()?;
        let mut count = 1;
        let mut laid = None;
        for _ in 0..MAX_LAYOUTS {
            let printed = predicted_labels(&labels, &before, &options.at, count, &listed);
            let (bytes, report) = lay_out(&listed, &printed, options, size, tagged)?;
            let pages = report.pages.len();
            laid = Some((bytes, report));
            if pages == count {
                break;
            }
            count = pages;
        }
        let Some((bytes, report)) = laid else {
            return Err(crate::Error::NoBookmarks);
        };
        let source = self.add_source(bytes, None)?;
        let source_pages = self.source_pages(&source)?;
        let height = size.height.get();
        let insert = InsertPagesFrom::new(source, source_pages, options.at.clone());
        let ((), applied) = self.ops_mut().transact(Origin::Local, |tx| {
            let order = tx.apply(insert)?.page_order.unwrap_or_default();
            let added: Vec<PageId> = order.iter().filter(|id| !before.contains(id)).cloned().collect();
            for entry_box in &report.entries {
                let Some(link) = link_for(entry_box, &report.pages, &added, &listed, height) else {
                    continue;
                };
                tx.apply(link)?;
            }
            if options.bookmark
                && let Some(first) = added.first()
            {
                let target = OutlineTarget::Destination(ExplicitDestination::new(first.clone(), DestinationView::Fit));
                tx.apply(AddOutlineItem::new(
                    None,
                    SiblingPosition::First,
                    options.title.clone(),
                    target,
                ))?;
            }
            Ok(())
        })?;
        Ok(self.edited(applied))
    }

    /// The bookmarks up to `levels` deep that go to a page of this document, in outline order.
    fn listed(&self, levels: u8) -> Result<Vec<Listed>> {
        let mut out = Vec::new();
        for node in self.outline()? {
            self.list(&node, 1, levels, &mut out)?;
        }
        Ok(out)
    }

    fn list(&self, node: &OutlineNode, level: u8, levels: u8, out: &mut Vec<Listed>) -> Result<()> {
        if let Some(target) = node.target {
            let page = usize::try_from(target.page).unwrap_or(usize::MAX);
            // A named destination stays named, so renaming or moving it later still holds.
            let link = match &node.destination_name {
                Some(name) => OutlineTarget::Named(name.clone()),
                None => OutlineTarget::Destination(self.destination_at(target.page, target.view)?),
            };
            out.push(Listed {
                level,
                title: node.title.clone(),
                page,
                target: link,
            });
        }
        if level < levels {
            for child in &node.children {
                self.list(child, level + 1, levels, out)?;
            }
        }
        Ok(())
    }

    /// `font-not-embedded` when the page would set the standard font in a file that claims
    /// PDF/A or PDF/UA-1, which require every font embedded.
    fn refuse_standard_font(&self, options: &ContentsOptions) -> Result<()> {
        if options.font.is_some() {
            return Ok(());
        }
        let claims = self.metadata()?;
        let profile = if claims.pdfua.is_some() {
            laminapdf_build::Profile::PdfUa1
        } else if claims.pdfa.is_some() {
            laminapdf_build::Profile::PdfA2b
        } else {
            return Ok(());
        };
        Err(laminapdf_build::Error::FontNotEmbedded {
            profile,
            font: "Helvetica".to_owned(),
        }
        .into())
    }
}

/// The label each listed bookmark's page will show once `count` pages are inserted at `at`:
/// the file's labels carried over as an insert carries them ([`PageLabels::remap`]), or the
/// page's number without any.
fn predicted_labels(
    labels: &PageLabels,
    before: &[PageId],
    at: &PagePosition,
    count: usize,
    listed: &[Listed],
) -> Vec<String> {
    // Stand-ins for the inserted pages: no page of the file has these object numbers.
    let new: Vec<PageId> = (0..count)
        .map(|n| PageId::object(ObjRef::new(u32::MAX - u32::try_from(n).unwrap_or(0), 0)))
        .collect();
    let index = match at {
        PagePosition::Start => 0,
        PagePosition::Before(id) => before.iter().position(|p| p == id).unwrap_or(before.len()),
        PagePosition::After(id) => before.iter().position(|p| p == id).map_or(before.len(), |i| i + 1),
        // The end, and any position added later: an insert there leaves every page's label.
        _ => before.len(),
    };
    let mut after = before.to_vec();
    after.splice(index..index, new);
    let remapped = labels.remap(before, &after);
    listed
        .iter()
        .map(|entry| {
            let Some(id) = before.get(entry.page) else {
                return String::new();
            };
            let at = after.iter().position(|p| p == id).unwrap_or(entry.page);
            remapped.label(at).unwrap_or_else(|| (at + 1).to_string())
        })
        .collect()
}

/// The contents page laid out as a document of its own: its bytes and where each entry went.
fn lay_out(
    listed: &[Listed],
    labels: &[String],
    options: &ContentsOptions,
    size: PageSize,
    tagged: bool,
) -> Result<(Vec<u8>, crate::flow::FlowReport)> {
    let mut builder = Builder::new(BuildOptions::new().with_tagged(tagged));
    let font = match &options.font {
        Some(font) => builder.font(font.clone())?,
        None => builder.standard_font(Standard14::Helvetica)?,
    };
    let mut body = TextStyle::new(&font, options.size);
    for fallback in &options.fallbacks {
        body = body.with_fallback(&builder.font(fallback.clone())?);
    }
    let heading = body.clone().with_size(options.size * 1.5);
    let entries = listed
        .iter()
        .zip(labels)
        .map(|(entry, label)| ContentsEntry::new(entry.level, entry.title.clone(), label.clone()))
        .collect();
    let flow = Flow::new(PageTemplate::new(size))
        .with_tagged(tagged)
        .with_outline(false)
        .block(FlowBlock::Heading(1, Paragraph::plain(options.title.clone(), &heading)))
        .block(FlowBlock::Contents(Contents::listing(&body, entries)));
    let report = builder.flow(&flow)?;
    let saved = builder.finish()?.save(&crate::SaveOptions::default())?;
    Ok((saved.bytes, report))
}

/// The link over `entry_box`, on the inserted page that holds it, to its bookmark's place.
fn link_for(
    entry_box: &ContentsEntryBox,
    built: &[PageId],
    added: &[PageId],
    listed: &[Listed],
    height: f64,
) -> Option<AddLink> {
    let index = built.iter().position(|p| *p == entry_box.page)?;
    let page = added.get(index)?.clone();
    let entry = listed.get(entry_box.entry)?;
    let r: &RectPt = &entry_box.rect;
    // The page was built upright with its media box at the origin: page space flips into
    // user space.
    let rect = UserSpaceRect::from_corners(r.x0.get(), height - r.y1.get(), r.x1.get(), height - r.y0.get());
    let mut link = AddLink::new(page, rect, entry.target.clone());
    link.contents = Some(entry.title.clone());
    Some(link)
}
