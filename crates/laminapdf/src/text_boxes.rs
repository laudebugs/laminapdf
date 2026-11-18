//! Text boxes and callouts on a page (ISO 32000-2 §12.5.6.6), from boxes and points in page space
//! (ADR 0027), as a viewer has them from a pointer: [`Document::text_box_at`] and
//! [`Document::callout_at`] give the edit, in 12-point Helvetica with a 1-point black border,
//! upright as the page is shown. Set the spec's font, size, alignment, fill or border before
//! [`Document::apply`] applies it; a font given by its bytes is added with
//! [`Document::add_blob`]. [`Document::text_box`] reads a box back, whichever app made it, as
//! the spec an editor changes.
//!
//! Text boxes in Helvetica, Times and Courier are laid out everywhere. A font given by its bytes,
//! or fallback fonts, needs the `text` feature, on by default (in WebAssembly, the text module):
//! elsewhere the edit fails with `appearance-needs-text`.

use laminapdf_model::{AnnotId, PointPt, RectPt};
use laminapdf_ops::{AddAnnot, AnnotColor, AnnotSpec, Callout, FreeText, FreeTextIntent, RichText};

use crate::document::Document;
use crate::error::{Error, Result};
use crate::links::user_rect;
use laminapdf_cos::Location;

impl Document {
    /// The edit that adds a text box over `area`, a box in page space on page `page` (0-based),
    /// holding `text`, upright as the page is shown.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.5.0.
    pub fn text_box_at(&self, page: u32, area: RectPt, text: RichText) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let spec = FreeText::new(user_rect(&frame, &[area]), text).with_rotation(frame.rotate().degrees());
        Ok(AddAnnot::new(id, AnnotSpec::FreeText(spec)))
    }

    /// The edit that adds a callout: a text box over `area` with a leader line through `line`, in
    /// page space on page `page` (0-based). `line` runs from the point it points at, through an
    /// optional knee, to the box: two or three points. Its ending is an open arrow.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). A line of another length fails
    /// when the edit is applied (`invalid-op`).
    ///
    /// Added in 0.5.0.
    pub fn callout_at(&self, page: u32, area: RectPt, line: &[PointPt], text: RichText) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let line = line.iter().map(|p| frame.to_user_space(*p)).collect();
        let spec = FreeText::new(user_rect(&frame, &[area]), text)
            .with_rotation(frame.rotate().degrees())
            .with_intent(FreeTextIntent::Callout(Callout::new(line)));
        Ok(AddAnnot::new(id, AnnotSpec::FreeText(spec)))
    }

    /// The text box `id` read back: where it sits in page space, beside the spec in user space
    /// that a [`FreeTextPatch`](crate::FreeTextPatch) changes. `None` for an annotation that
    /// isn't a text box.
    ///
    /// # Errors
    /// `unknown-annotation` for an ID no page lists; `annotation-not-editable` for a widget or a
    /// direct dictionary; reading errors.
    ///
    /// Added in 0.7.0.
    pub fn text_box(&self, id: &AnnotId) -> Result<Option<TextBox>> {
        let Some(read) = laminapdf_ops::read_text_box(self.ops(), id)? else {
            return Ok(None);
        };
        let pages = self.ops().pages()?;
        let (index, page) = pages
            .iter()
            .enumerate()
            .find(|(_, p)| *p.id() == read.page)
            .ok_or_else(|| Error::from(laminapdf_ops::Error::UnknownAnnotation(id.clone())))?;
        let frame = page.frame()?;
        let spec = read.text_box;
        let line = match &spec.intent {
            FreeTextIntent::Callout(callout) => callout.line.iter().map(|p| frame.to_page(*p)).collect(),
            _ => Vec::new(),
        };
        Ok(Some(TextBox {
            page: u32::try_from(index).unwrap_or(u32::MAX),
            rect: frame.rect_to_page(&spec.rect),
            line,
            spec,
            color: read.color,
            simplified: read.simplified,
        }))
    }
}

/// A text box as [`Document::text_box`] reads it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextBox {
    /// Its page, 0-based.
    pub page: u32,
    /// The box the text is laid out in, in page space: inside the annotation's rectangle, which
    /// also holds the border and a callout's line.
    pub rect: RectPt,
    /// A callout's line in page space, from the point it points at to the box; empty for
    /// another kind.
    pub line: Vec<PointPt>,
    /// The box as a spec, in user space.
    pub spec: FreeText,
    /// The text's colour; `None` means black.
    pub color: Option<AnnotColor>,
    /// Whether reading dropped styles or a font the spec can't hold; changing the box writes it
    /// again without them (`rich-text-simplified`).
    pub simplified: bool,
}

/// What an edit did that its caller may want to check ([`Edited::warnings`](crate::Edited::warnings)).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditWarning {
    /// Changing a text box dropped styles of its rich text that Lamina PDF doesn't keep
    /// (`rich-text-simplified`).
    RichTextSimplified {
        /// The text boxes' IDs.
        annotations: Vec<String>,
    },
    /// Flattening left annotations in place: they have no appearance, and Lamina PDF can't
    /// draw one for their kind (`flatten-no-appearance`).
    ///
    /// Added in 0.6.0.
    FlattenNoAppearance {
        /// The annotations' IDs.
        annotations: Vec<String>,
    },
    /// A deleted attachment stays in the document: a `FileAttachment` annotation still holds
    /// it (`attachment-still-held`).
    ///
    /// Added in 0.6.0.
    AttachmentStillHeld {
        /// Each attachment's ID with the annotation that holds it.
        attachments: Vec<(String, String)>,
    },
}

impl EditWarning {
    /// Every code, in the order `fixtures/codes.json`'s `facadeWarnings` lists them after
    /// [`RedactionWarning`](crate::RedactionWarning)'s.
    ///
    /// Added in 0.5.0.
    pub const CODES: &'static [&'static str] =
        &["rich-text-simplified", "flatten-no-appearance", "attachment-still-held"];

    /// The stable kebab-case code.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RichTextSimplified { .. } => "rich-text-simplified",
            Self::FlattenNoAppearance { .. } => "flatten-no-appearance",
            Self::AttachmentStillHeld { .. } => "attachment-still-held",
        }
    }

    /// Where it is: nowhere in particular, since it names its annotations in its data.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn location(&self) -> Location {
        Location::NONE
    }
}
