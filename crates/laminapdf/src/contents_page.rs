//! A contents page made from the bookmarks (E3 plan §2.2): a page or pages listing the outline
//! one or two levels deep, each entry linked to its bookmark's place, inserted as one undo step.
//!
//! It isn't an op: an op can't draw text. It runs in three parts, so a host whose document is
//! open in a build without fonts (the WebAssembly base and extract modules) can lay the page out
//! in another (the text module):
//!
//! 1. [`Document::contents_listing`] reads what the page lists: the bookmarks, the label each
//!    bookmark's page will show, the page size and the file's claims. Every build.
//! 2. [`ContentsListing::lay_out`] lays the page out with the builder's flowing layout into a
//!    document of its own. Only with the `text` feature.
//! 3. [`Document::insert_contents_page`] adds that document as a source and, in one transaction,
//!    inserts its pages ([`InsertPagesFrom`]), links each entry ([`AddLink`]) and, when asked,
//!    adds a bookmark for it ([`AddOutlineItem`]). Every build.
//!
//! [`Document::create_contents_page`] runs all three.

use laminapdf_cos::ObjRef;
use laminapdf_model::{PageId, PageLabels, PagePosition, RectPt, SiblingPosition, UserSpaceRect};
use laminapdf_ops::{
    AddLink, AddOutlineItem, DestinationView, ExplicitDestination, InsertPagesFrom, Origin, OutlineTarget,
};
use serde::{Deserialize, Serialize};

use crate::document::{Document, Edited};
use crate::error::Result;
use crate::navigation::OutlineNode;

/// How a contents page is made: where it goes, what it lists and how it reads.
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
            size: 11.0,
        }
    }
}

impl ContentsOptions {
    /// The defaults: before the first page, two levels, titled "Contents", no bookmark of its
    /// own, entries at 11 points.
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

    /// The entries at `size` points, from 4 to 72 ([`Document::contents_listing`] refuses others).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }
}

/// What a contents page lists, read from a document by [`Document::contents_listing`]: the
/// options it was read with, the page size, the claims the file makes, and each entry with the
/// labels its page will show. It is plain data that serialises to JSON, so a host can read it in
/// one build and lay the page out in another.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentsListing {
    at: ListingAt,
    levels: u8,
    title: String,
    bookmark: bool,
    size: f64,
    width: f64,
    height: f64,
    tagged: bool,
    claim: Option<Claim>,
    entries: Vec<ListedEntry>,
}

/// Where the listing's page goes, by page ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ListingAt {
    Start,
    End,
    Before(String),
    After(String),
}

/// The profile a file claims that requires every font embedded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Claim {
    PdfUa,
    PdfA,
}

/// One bookmark a contents page lists.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct ListedEntry {
    /// Its level: 1 for a top-level bookmark, 2 for one inside it.
    pub level: u8,
    /// The bookmark's title.
    pub title: String,
    /// The label its page shows once the contents page is in: `labels[n - 1]` when the contents
    /// page takes `n` pages (the file's labels, or page numbers, move along by that many).
    pub labels: Vec<String>,
}

impl ContentsListing {
    /// The entries, in outline order.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn entries(&self) -> &[ListedEntry] {
        &self.entries
    }

    /// The listing as JSON, which serde reads back as a `ContentsListing`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn to_json(&self) -> String {
        // A struct of strings, numbers and lists always serialises.
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// A contents page laid out as a document of its own, by [`ContentsListing::lay_out`], for
/// [`Document::insert_contents_page`].
///
/// Added in 0.5.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ContentsPage {
    /// The document's bytes.
    pub bytes: Vec<u8>,
    /// Where each entry was drawn.
    pub entries: Vec<PlacedEntry>,
    /// The listing it was laid out from, as JSON: the insert compares it with the document's
    /// listing then, so it never inserts a page whose entries or labels have gone stale.
    pub listing: String,
}

impl ContentsPage {
    /// A laid-out contents page: its document's bytes, where each entry was drawn, and the
    /// listing it was laid out from as JSON ([`ContentsListing::to_json`]).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(bytes: Vec<u8>, entries: Vec<PlacedEntry>, listing: String) -> Self {
        Self {
            bytes,
            entries,
            listing,
        }
    }
}

/// Where a contents page drew one entry: the box its link covers.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct PlacedEntry {
    /// The entry's index in [`ContentsListing::entries`].
    pub entry: usize,
    /// The 0-based index of the contents page's page it is on.
    pub page: usize,
    /// Its box in that page's page space (top-left origin, y down).
    pub rect: RectPt,
}

impl PlacedEntry {
    /// Entry `entry` drawn in `rect` on page `page` of the contents page.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(entry: usize, page: usize, rect: RectPt) -> Self {
        Self { entry, page, rect }
    }
}

/// A bookmark the page lists: its level, its title, its page and where its link goes.
struct Listed {
    level: u8,
    title: String,
    page: usize,
    target: OutlineTarget,
}

/// The most pages a contents page may take: the listing holds each entry's label for every
/// count up to it, since each page pushes the bookmarks' pages along by one. Ten pages hold some
/// 400 entries at the default size.
const MAX_PAGES: usize = 10;

/// The smallest and largest entry size, points.
const SIZES: std::ops::RangeInclusive<f64> = 4.0..=72.0;

impl Document {
    /// What a contents page made with `options` lists: each bookmark up to `options.levels`
    /// deep that goes to a page of this document, in outline order, with the label its page will
    /// show once the page is in. Bookmarks that go to no page of this document (a web address,
    /// another file) are left out; the ones inside them are listed.
    ///
    /// # Errors
    /// `no-bookmarks` when no bookmark goes to a page of this document; `unknown-page` for an
    /// `at` that names no page; `invalid-op` for an entry size outside 4 to 72 points (or not a
    /// number); reading errors.
    ///
    /// Added in 0.5.0.
    pub fn contents_listing(&self, options: &ContentsOptions) -> Result<ContentsListing> {
        if !SIZES.contains(&options.size) {
            let reason = "a contents page's entry size is from 4 to 72 points";
            return Err(laminapdf_ops::Error::InvalidOp(reason).into());
        }
        let listed = self.listed(options.levels.clamp(1, 2))?;
        if listed.is_empty() {
            return Err(crate::Error::NoBookmarks);
        }
        let infos = self.page_infos()?;
        let before: Vec<PageId> = infos.iter().map(|p| p.id.clone()).collect();
        let at = match &options.at {
            PagePosition::Start => ListingAt::Start,
            PagePosition::Before(id) => ListingAt::Before(known(&before, id)?),
            PagePosition::After(id) => ListingAt::After(known(&before, id)?),
            // The end, and any position added later.
            _ => ListingAt::End,
        };
        let labels = self.ops().page_labels()?;
        let by_count: Vec<Vec<String>> = (1..=MAX_PAGES)
            .map(|count| predicted_labels(&labels, &before, &options.at, count, &listed))
            .collect();
        let entries = listed
            .iter()
            .enumerate()
            .map(|(n, entry)| ListedEntry {
                level: entry.level,
                title: entry.title.clone(),
                labels: by_count.iter().map(|l| l.get(n).cloned().unwrap_or_default()).collect(),
            })
            .collect();
        let claims = self.metadata()?;
        let claim = if claims.pdfua.is_some() {
            Some(Claim::PdfUa)
        } else {
            claims.pdfa.is_some().then_some(Claim::PdfA)
        };
        let (width, height) = infos.first().map_or((595.0, 842.0), |p| (p.width, p.height));
        Ok(ContentsListing {
            at,
            levels: options.levels.clamp(1, 2),
            title: options.title.clone(),
            bookmark: options.bookmark,
            size: options.size,
            width,
            height,
            tagged: self.is_tagged()?,
            claim,
            entries,
        })
    }

    /// Inserts `page`, laid out from the listing `options` gives, in one transaction (one undo
    /// step): its pages where `options` says ([`InsertPagesFrom`](crate::InsertPagesFrom)), a link over each
    /// entry to its bookmark's place with the bookmark's title as its description
    /// ([`AddLink`](crate::AddLink)), and, with [`ContentsOptions::bookmark`], a bookmark for the
    /// contents page first in the outline ([`AddOutlineItem`](crate::AddOutlineItem)). The pages it
    /// inserted are those [`Edited::page_order`] holds that weren't there before.
    ///
    /// # Errors
    /// `document-changed` when this document no longer gives the listing `page` was laid out
    /// from (its bookmarks, labels or pages changed since, or `options` differ); `no-bookmarks`; the errors of a source document that
    /// doesn't open; the ops' errors; `saved-not-rebased`.
    ///
    /// Added in 0.5.0.
    pub fn insert_contents_page(&mut self, options: &ContentsOptions, page: ContentsPage) -> Result<Edited> {
        let listing = self.contents_listing(options)?;
        if listing.to_json() != page.listing {
            return Err(crate::Error::DocumentChanged);
        }
        let listed = self.listed(listing.levels)?;
        let before: Vec<PageId> = self.page_infos()?.into_iter().map(|p| p.id).collect();
        let source = self.add_source(page.bytes, None)?;
        let source_pages = self.source_pages(&source)?;
        let count = source_pages.len();
        let insert = InsertPagesFrom::new(source, source_pages, options.at.clone());
        let height = listing.height;
        let ((), applied) = self.ops_mut().transact(Origin::Local, |tx| {
            let order = tx.apply(insert)?.page_order.unwrap_or_default();
            let added: Vec<PageId> = order.iter().filter(|id| !before.contains(id)).cloned().collect();
            for placed in page.entries.iter().filter(|p| p.page < count) {
                if let Some(link) = link_for(placed, &added, &listed, height) {
                    tx.apply(link)?;
                }
            }
            if listing.bookmark
                && let Some(first) = added.first()
            {
                let target = OutlineTarget::Destination(ExplicitDestination::new(first.clone(), DestinationView::Fit));
                tx.apply(AddOutlineItem::new(
                    None,
                    SiblingPosition::First,
                    listing.title.clone(),
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
}

/// `id` as text when it is one of `pages`, else `unknown-page`.
fn known(pages: &[PageId], id: &PageId) -> Result<String> {
    if pages.contains(id) {
        Ok(id.to_string())
    } else {
        Err(laminapdf_ops::Error::UnknownPage(id.clone()).into())
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

/// The link over `placed`, on the inserted page that holds it, to its bookmark's place.
fn link_for(placed: &PlacedEntry, added: &[PageId], listed: &[Listed], height: f64) -> Option<AddLink> {
    let page = added.get(placed.page)?.clone();
    let entry = listed.get(placed.entry)?;
    let r = &placed.rect;
    // The page was built upright with its media box at the origin: page space flips into
    // user space.
    let rect = UserSpaceRect::from_corners(r.x0.get(), height - r.y1.get(), r.x1.get(), height - r.y0.get());
    let mut link = AddLink::new(page, rect, entry.target.clone());
    link.contents = Some(entry.title.clone());
    Some(link)
}

#[cfg(feature = "text")]
pub use layout::ContentsFonts;

/// Laying the page out, with the builder's flowing layout and fonts.
#[cfg(feature = "text")]
mod layout {
    use laminapdf_model::{PageSize, Pt};

    use super::{Claim, ContentsListing, ContentsOptions, ContentsPage, MAX_PAGES, PlacedEntry};
    use crate::builder::Builder;
    use crate::document::{Document, Edited};
    use crate::error::Result;
    use crate::flow::{Contents, ContentsEntry, Flow, FlowBlock, PageTemplate, Paragraph, TextStyle};
    use crate::{BuildOptions, Font, Standard14};

    /// The fonts a contents page is set in.
    ///
    /// Added in 0.5.0.
    #[derive(Debug, Clone, Default)]
    #[non_exhaustive]
    pub struct ContentsFonts {
        /// The font of its text; `None` for the standard 14 Helvetica, which isn't embedded, so
        /// a file that claims PDF/A or PDF/UA-1 refuses it (`font-not-embedded`).
        pub font: Option<Font>,
        /// Fonts for the characters [`ContentsFonts::font`] lacks, tried in order.
        pub fallbacks: Vec<Font>,
    }

    impl ContentsFonts {
        /// Helvetica, with no fallbacks.
        ///
        /// Added in 0.5.0.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
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
    }

    impl ContentsListing {
        /// The contents page laid out in `fonts` as a document of its own, for
        /// [`Document::insert_contents_page`]: the heading, then each entry indented by its
        /// level, with dot leaders to its label. When it takes more than one page it is laid out
        /// again with the labels for that many, until the count it prints for is the count it
        /// takes. On a tagged document it is tagged (`TOC`, `TOCI`).
        ///
        /// # Errors
        /// `font-not-embedded` for a font that isn't embedded (the default Helvetica) when the
        /// file claims PDF/A or PDF/UA-1; `missing-glyph` for a character of a title that
        /// neither the font nor its fallbacks have; `invalid-op` for a page that would take more
        /// than ten pages (list one level, or a smaller size); the other errors of laying text out.
        ///
        /// Added in 0.5.0.
        pub fn lay_out(&self, fonts: &ContentsFonts) -> Result<ContentsPage> {
            self.refuse_unembedded(fonts)?;
            let too_long = || laminapdf_ops::Error::InvalidOp("a contents page takes at most ten pages").into();
            let mut count = 1;
            // Longer labels can add a page, and the labels for more pages can be longer again:
            // each pass prints the count the last one took, which settles in a pass or two.
            for _ in 0..=MAX_PAGES {
                let (page, pages) = self.lay_out_for(fonts, count)?;
                if pages == count {
                    return Ok(page);
                }
                if pages > MAX_PAGES {
                    return Err(too_long());
                }
                count = pages.max(1);
            }
            Err(too_long())
        }

        /// The page laid out with the labels for `count` pages, and how many it took.
        fn lay_out_for(&self, fonts: &ContentsFonts, count: usize) -> Result<(ContentsPage, usize)> {
            let mut builder = Builder::new(BuildOptions::new().with_tagged(self.tagged));
            let font = match &fonts.font {
                Some(font) => builder.font(font.clone())?,
                None => builder.standard_font(Standard14::Helvetica)?,
            };
            let mut body = TextStyle::new(&font, self.size);
            for fallback in &fonts.fallbacks {
                body = body.with_fallback(&builder.font(fallback.clone())?);
            }
            let heading = body.clone().with_size(self.size * 1.5);
            let entries = self
                .entries
                .iter()
                .map(|e| {
                    let label = e.labels.get(count - 1).cloned().unwrap_or_default();
                    ContentsEntry::new(e.level, e.title.clone(), label)
                })
                .collect();
            let size = PageSize::new(Pt::new(self.width), Pt::new(self.height));
            let flow = Flow::new(PageTemplate::new(size))
                .with_tagged(self.tagged)
                .with_outline(false)
                .block(FlowBlock::Heading(1, Paragraph::plain(self.title.clone(), &heading)))
                .block(FlowBlock::Contents(Contents::listing(&body, entries)));
            let report = builder.flow(&flow)?;
            let bytes = builder.finish()?.save(&crate::SaveOptions::default())?.bytes;
            let placed = report
                .entries
                .iter()
                .filter_map(|b| {
                    let page = report.pages.iter().position(|p| *p == b.page)?;
                    Some(PlacedEntry::new(b.entry, page, b.rect))
                })
                .collect();
            Ok((ContentsPage::new(bytes, placed, self.to_json()), report.pages.len()))
        }

        /// `font-not-embedded` when the page would set a font that isn't embedded in a file that
        /// claims PDF/A or PDF/UA-1, which require every font embedded.
        fn refuse_unembedded(&self, fonts: &ContentsFonts) -> Result<()> {
            let name = match &fonts.font {
                None => "Helvetica".to_owned(),
                Some(font) if !font.requires_embedding() => font.info().postscript_name.clone(),
                Some(_) => return Ok(()),
            };
            let profile = match self.claim {
                Some(Claim::PdfUa) => laminapdf_build::Profile::PdfUa1,
                Some(Claim::PdfA) => laminapdf_build::Profile::PdfA2b,
                None => return Ok(()),
            };
            Err(laminapdf_build::Error::FontNotEmbedded { profile, font: name }.into())
        }
    }

    impl Document {
        /// Adds a contents page made from the bookmarks, set in `fonts`: their titles one or two
        /// levels deep, each indented by its level, with dot leaders to the page label a reader
        /// will see once the page is in, each entry a link to its bookmark's place. It runs
        /// [`Document::contents_listing`], [`ContentsListing::lay_out`] and
        /// [`Document::insert_contents_page`], so it is one undo step.
        ///
        /// # Errors
        /// As those three.
        ///
        /// Added in 0.5.0.
        pub fn create_contents_page(&mut self, options: &ContentsOptions, fonts: &ContentsFonts) -> Result<Edited> {
            let listing = self.contents_listing(options)?;
            let page = listing.lay_out(fonts)?;
            self.insert_contents_page(options, page)
        }
    }
}
