//! Text added to pages as content (E6 plan §2.2): [`AddText`] lays out its runs in a box, in the
//! font and its fallbacks, and puts them on the page as one Form XObject like every added object.
//! [`UpdateContentObject`](crate::UpdateContentObject) edits it afterwards and
//! [`DeleteContentObject`](crate::DeleteContentObject) takes it off.

use laminapdf_model::{Matrix, PageId, PdfDate, RectPt};
use serde::{Deserialize, Serialize};

use super::spec::{ContentId, ContentLayer, ContentObjectSpec, TextBlock, is_twelve, twelve};
use super::{Added, Tagging, add_object};
use crate::annot::{AnnotColor, AnnotFont, Appearances, Opacity, TextAlign, TextSpan};
use crate::blob::BlobStore;
use crate::error::Result;
use crate::op::Effect;
use crate::wire;

/// How added text is tagged in a tagged file (ISO 32000-2 §14.8).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum TagChoice {
    /// A `P` element that owns the text's marked content, so screen readers read it.
    #[default]
    Paragraph,
    /// An artifact (§14.8.2.2): decoration, which screen readers skip.
    Artifact,
}

/// Adds text to a page as page content (ISO 32000-2 §8.10): a Form XObject of its own, drawn over
/// the page's content, with the runs laid out in `rect` by `laminapdf-text`: bidirectional text
/// and shaping, line breaking at the box's width, and each font embedded as a subset with a
/// `ToUnicode` map, so the text copies and searches as typed. The text starts at the top of
/// `rect`, wraps at its width, and is clipped to it.
///
/// `fonts` is the font, then the fallbacks tried in order for a character it lacks (a symbol
/// font, a font for another script): `helvetica`, `times` or `courier` need no bytes and every
/// module draws them in `WinAnsiEncoding`; a font's bytes, added to the session first, are
/// embedded and need the text module (`appearance-needs-text` otherwise). A character in none of
/// the fonts is `content-glyph-missing`, never a silent blank; a font that forbids embedding
/// (`OS/2` `fsType`) is `content-font-not-embeddable`.
///
/// The form keeps `id` in its page-piece dictionary (§14.5), dated `modified`, as every added
/// object does, so [`UpdateContentObject`](crate::UpdateContentObject) and
/// [`DeleteContentObject`](crate::DeleteContentObject) find it after a save. In a tagged file the
/// text is a `P` element, or an artifact for [`TagChoice::Artifact`].
///
/// On the wire: `{ "op": "addText", "args": { "id": "note", "page": "p3.0", "rect": [72, 72, 288,
/// 120], "runs": [{ "text": "Reviewed " }, { "text": "twice", "bold": true }], "fonts":
/// ["helvetica"], "modified": "D:20261118120000Z" } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AddText {
    /// The text's ID, unique on the page.
    pub id: ContentId,
    /// The page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The box, in page space (ADR 0027): its top-left corner is `(x0, y0)`.
    #[serde(with = "wire::rect_pt")]
    #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number]"))]
    pub rect: RectPt,
    /// The text, in spans of one style each; a line feed in a span starts a paragraph.
    pub runs: Vec<TextSpan>,
    /// How lines sit in the box; left when absent.
    #[serde(default, skip_serializing_if = "TextAlign::is_left")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub align: TextAlign,
    /// The font, then its fallbacks; Helvetica when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fonts: Vec<AnnotFont>,
    /// The size in points, from 1 to 500; 12 when absent.
    #[serde(default = "twelve", skip_serializing_if = "is_twelve")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub size: f64,
    /// The text's colour; black when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub color: Option<AnnotColor>,
    /// How a tagged file marks the text; a paragraph when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub tag: Option<TagChoice>,
    /// The form's `/LastModified`.
    #[serde(with = "wire::date")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub modified: PdfDate,
}

// Sound: as for `AddContentObject`.
impl Eq for AddText {}

impl AddText {
    /// The op that adds `runs` to `page` as `id`, in `rect`, in 12-point black Helvetica,
    /// left-aligned, as a paragraph.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, page: PageId, rect: RectPt, runs: Vec<TextSpan>, modified: PdfDate) -> Self {
        Self {
            id,
            page,
            rect,
            runs,
            align: TextAlign::Left,
            fonts: Vec::new(),
            size: 12.0,
            color: None,
            tag: None,
            modified,
        }
    }

    /// The same op in `fonts` (the font, then fallbacks) at `size` points.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: Vec<AnnotFont>, size: f64) -> Self {
        self.fonts = fonts;
        self.size = size;
        self
    }

    /// The same op with its lines aligned by `align`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// The same op in `color`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_color(mut self, color: AnnotColor) -> Self {
        self.color = Some(color);
        self
    }

    /// The same op, tagged as `tag` in a tagged file.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_tag(mut self, tag: TagChoice) -> Self {
        self.tag = Some(tag);
        self
    }

    /// The text's box and style as the object holds them.
    fn block(&self) -> TextBlock {
        let mut block = TextBlock::new(
            self.rect.x1.get() - self.rect.x0.get(),
            self.rect.y1.get() - self.rect.y0.get(),
            self.runs.clone(),
        )
        .with_fonts(self.fonts.clone(), self.size)
        .with_align(self.align);
        block.color = self.color;
        block
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        let object = ContentObjectSpec::Text(self.block());
        // The box's own space has y up and its origin at the bottom-left corner; page space has
        // y down, so the box's top edge sits on the rect's top.
        let matrix = Matrix::new([1.0, 0.0, 0.0, -1.0, self.rect.x0.get(), self.rect.y1.get()]);
        add_object(
            doc,
            appearances,
            blobs,
            &Added {
                id: &self.id,
                page: &self.page,
                object: &object,
                matrix,
                opacity: Opacity::OPAQUE,
                layer: ContentLayer::Front,
                tagging: Tagging::Text {
                    artifact: self.tag == Some(TagChoice::Artifact),
                },
                modified: &self.modified,
            },
        )
    }
}
