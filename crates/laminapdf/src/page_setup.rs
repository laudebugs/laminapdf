//! What the page-setup dialogs read before they edit: a page's boxes in user space, which
//! [`SetPageBoxes`](crate::SetPageBoxes) takes, and the document's label ranges in the shape
//! [`SetPageLabels`](crate::SetPageLabels) takes, so a reader can show the current values and
//! write them back changed.

use laminapdf_model::{LETTER, LabelRange, UserSpaceRect};
use laminapdf_ops::PageLabelRange;

use crate::document::Document;
use crate::error::{Error, Result};

/// A page's boxes in user space (ISO 32000-2 §14.11.2), each as the reader uses it: the crop box
/// cut to the media box, the bleed, trim and art boxes cut to the crop box, and a box the page
/// doesn't set taken from the one it defaults to.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct PageGeometry {
    /// The media box: the paper.
    pub media: UserSpaceRect,
    /// The crop box: what readers show.
    pub crop: UserSpaceRect,
    /// The bleed box: where printing is clipped.
    pub bleed: UserSpaceRect,
    /// The trim box: the finished page after trimming.
    pub trim: UserSpaceRect,
    /// The art box: the page's meaningful content.
    pub art: UserSpaceRect,
    /// Which of the bleed, trim and art boxes the page sets itself, rather than taking the crop
    /// box.
    pub own: OwnBoxes,
    /// `/UserUnit`: the size of a user-space unit in points (1 unless the page sets it).
    pub user_unit: f64,
}

/// Which of the boxes that default to the crop box a page sets itself.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct OwnBoxes {
    /// `/BleedBox` is set.
    pub bleed: bool,
    /// `/TrimBox` is set.
    pub trim: bool,
    /// `/ArtBox` is set.
    pub art: bool,
}

impl Document {
    /// Page `index`'s boxes in user space, as [`SetPageBoxes`](crate::SetPageBoxes) takes them.
    /// A page with no usable media box reads as US Letter, as it is shown.
    ///
    /// # Errors
    /// `page-index-out-of-range`; `strict` for a malformed box; reading errors.
    ///
    /// Added in 0.5.0.
    pub fn page_boxes(&self, index: u32) -> Result<PageGeometry> {
        let pages = self.ops().pages()?;
        let count = u32::try_from(pages.len()).unwrap_or(u32::MAX);
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange { index, count })?;
        let media = page.media_box()?.unwrap_or(LETTER);
        // The model cuts the crop box to the media box it read; without one, cut it to Letter here.
        let crop = page.crop_box()?.and_then(|c| c.intersect(&media)).unwrap_or(media);
        let dict = page.dict();
        let own = |key: &str| dict.get(key).is_some();
        Ok(PageGeometry {
            media,
            crop,
            bleed: page.bleed_box()?.unwrap_or(crop),
            trim: page.trim_box()?.unwrap_or(crop),
            art: page.art_box()?.unwrap_or(crop),
            own: OwnBoxes {
                bleed: own("BleedBox"),
                trim: own("TrimBox"),
                art: own("ArtBox"),
            },
            user_unit: page.user_unit()?,
        })
    }

    /// The document's page labels as ranges, in page order, each naming its first page by ID:
    /// what [`SetPageLabels`](crate::SetPageLabels) takes, so a reader can show them, change
    /// them and write them back. Empty when the document has no labels. A range that starts past
    /// the last page is left out; a first number past `u32::MAX` reads as `u32::MAX`.
    ///
    /// # Errors
    /// Reading errors (the catalog, the number tree, the page tree).
    ///
    /// Added in 0.5.0.
    pub fn label_ranges(&self) -> Result<Vec<PageLabelRange>> {
        let doc = self.ops();
        let labels = doc.page_labels()?;
        let pages = doc.pages()?;
        Ok(labels
            .ranges()
            .iter()
            .filter_map(|range| {
                let page = pages.get(usize::try_from(range.start).ok()?)?;
                Some(label_range(page.id().clone(), range))
            })
            .collect())
    }
}

fn label_range(page: crate::PageId, range: &LabelRange) -> PageLabelRange {
    PageLabelRange::new(page, range.style)
        .with_prefix(range.prefix.clone())
        .with_first(u32::try_from(range.first).unwrap_or(u32::MAX))
}
