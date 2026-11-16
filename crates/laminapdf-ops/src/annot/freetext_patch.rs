//! The changes an [`UpdateAnnot`](super::UpdateAnnot) makes to a `FreeText` annotation: each field
//! given replaces the box's, and the box is laid out and drawn again. A text editor sends one per
//! editing session with the whole new text (E4 plan §2.1).

use laminapdf_model::UserSpaceRect;
use serde::{Deserialize, Serialize};

use super::drawing::Border;
use super::freetext::{FontRef, FreeText, FreeTextIntent, RichText, TextAlign};
use super::geom::AnnotColor;
use crate::blob::BlobId;
use crate::wire;

/// A change to a text box or callout: every field given replaces the box's.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct FreeTextPatch {
    /// The box.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(optional, type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: Option<UserSpaceRect>,
    /// The text, whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub text: Option<RichText>,
    /// The font.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub font: Option<FontRef>,
    /// The fallback fonts, all of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fallbacks: Option<Vec<BlobId>>,
    /// The size in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub size: Option<f64>,
    /// The alignment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub align: Option<TextAlign>,
    /// A text box, callout or typewriter text, with the callout's line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub intent: Option<FreeTextIntent>,
    /// The background; `null` removes it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "[number] | [number, number, number] | [number, number, number, number] | null"
        )
    )]
    pub fill: Option<Option<AnnotColor>>,
    /// The outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub border: Option<Border>,
    /// The page rotation the text is upright for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub rotation: Option<u16>,
}

impl FreeTextPatch {
    /// The patch that replaces the text alone.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn text(text: RichText) -> Self {
        Self {
            text: Some(text),
            ..Self::default()
        }
    }

    /// `text` changed. The fallbacks of a box read from a file are none (they live only in the
    /// session that added them), so a patch that keeps the font keeps the box's.
    pub(crate) fn applied_to(&self, text: &FreeText) -> FreeText {
        FreeText {
            rect: self.rect.unwrap_or(text.rect),
            text: self.text.clone().unwrap_or_else(|| text.text.clone()),
            font: self.font.unwrap_or(text.font),
            fallbacks: self.fallbacks.clone().unwrap_or_else(|| text.fallbacks.clone()),
            size: self.size.unwrap_or(text.size),
            align: self.align.unwrap_or(text.align),
            intent: self.intent.clone().unwrap_or_else(|| text.intent.clone()),
            fill: self.fill.unwrap_or(text.fill),
            border: self.border.clone().unwrap_or_else(|| text.border.clone()),
            rotation: self.rotation.unwrap_or(text.rotation),
        }
    }
}
