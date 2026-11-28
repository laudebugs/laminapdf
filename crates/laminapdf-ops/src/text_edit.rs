//! What an edit of text did, or would do: its fidelity (ADR 0022 §7), the characters drawn in
//! another font and the width the line grew by. The text ops report it in
//! [`Applied`](crate::Applied) and `plan_edit_text` says it before the edit is committed.

use laminapdf_model::PageId;
use serde::{Deserialize, Serialize};

/// How faithful an edit's result is to what the text looked like (ADR 0022 §7).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum Fidelity {
    /// The same font, the glyphs placed as they were.
    Exact,
    /// The same fonts, with the glyphs after the edit moved on their line.
    Reflowed,
    /// Some characters are drawn in another font.
    Substituted,
}

/// Where the glyphs of a stretch of new text come from.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum SubstituteKind {
    /// A new subset of the run's own font, from a full copy the caller supplied.
    Subset,
    /// Another font than the run's.
    Substitute,
}

/// Characters of an edit drawn in a font other than the one the run is set in.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextSubstitution {
    /// The characters the run's font has no glyph for.
    pub text: String,
    /// The font that draws them: its PostScript name.
    pub font: String,
    /// Whether it is a new subset of the run's own font or another font.
    pub kind: SubstituteKind,
}

/// What an edit of text did, or would do: what `plan_edit_text` says
/// before the edit is committed and the applied edit reports.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextEditReport {
    /// The page.
    #[serde(with = "crate::wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The text object the run is in, by its ordinal among the page's objects.
    pub ordinal: u32,
    /// The run's first glyph among the object's.
    pub start: u32,
    /// The glyph after the run's last, among the object's glyphs.
    pub end: u32,
    /// How faithful the result is.
    pub fidelity: Fidelity,
    /// The characters drawn in another font, each stretch with the font that draws it; empty
    /// when the run's own font draws everything.
    pub substitutions: Vec<TextSubstitution>,
    /// How much wider the new text is than the old, in points: the line grows by this much,
    /// which may overflow its text block. 0 when it didn't grow.
    pub grew_by: f64,
}

// `grew_by` is a finite width (never NaN), so reports compare by value as the types holding them do.
impl Eq for TextEditReport {}
