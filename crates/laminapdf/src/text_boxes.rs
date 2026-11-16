//! Text boxes and callouts on a page (ISO 32000-2 §12.5.6.6), from boxes and points in page space
//! (ADR 0027), as a viewer has them from a pointer: [`Document::text_box_at`] and
//! [`Document::callout_at`] give the edit, in 12-point Helvetica with a 1-point black border,
//! upright as the page is shown. Set the spec's font, size, alignment, fill or border before
//! [`Document::apply`] applies it; a font given by its bytes is added with
//! [`Document::add_blob`].
//!
//! Text boxes in Helvetica, Times and Courier are laid out everywhere. A font given by its bytes,
//! or fallback fonts, needs the `text` feature, on by default (in WebAssembly, the text module):
//! elsewhere the edit fails with `appearance-needs-text`.

use laminapdf_model::{PointPt, RectPt};
use laminapdf_ops::{AddAnnot, AnnotSpec, Callout, FreeText, FreeTextIntent, RichText};

use crate::document::Document;
use crate::error::Result;
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
}

impl EditWarning {
    /// Every code, in the order `fixtures/codes.json`'s `facadeWarnings` lists them after
    /// [`RedactionWarning`](crate::RedactionWarning)'s.
    ///
    /// Added in 0.5.0.
    pub const CODES: &'static [&'static str] = &["rich-text-simplified"];

    /// The stable kebab-case code.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RichTextSimplified { .. } => "rich-text-simplified",
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
