//! What kind of mark an [`AddAnnot`](super::AddAnnot) makes ([`AnnotSpec`]), and the changes an
//! [`UpdateAnnot`](super::UpdateAnnot) can make to one ([`AnnotSpecPatch`]).

use serde::{Deserialize, Serialize};

use super::geom::{Point, Quad, Rect};

/// The kind of mark an annotation op makes, with its geometry. Every spec gets an appearance
/// stream that Lamina PDF generates ([`Appearances`](crate::Appearances)).
///
/// On the wire the kind is the `type` field: `{ "type": "textMarkup", "kind": "highlight",
/// "quads": [ … ] }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum AnnotSpec {
    /// A highlight, underline, strike-out or squiggly line over text (ISO 32000-2 §12.5.6.10).
    TextMarkup(TextMarkup),
    /// A sticky note: a Text annotation with its Popup (§12.5.6.4).
    Note(Note),
    /// A caret marking where text goes (§12.5.6.11).
    Caret(Caret),
}

impl AnnotSpec {
    /// The `/Subtype` the spec writes.
    #[must_use]
    pub(crate) const fn subtype(&self) -> &'static str {
        match self {
            Self::TextMarkup(m) => m.kind.subtype(),
            Self::Note(_) => "Text",
            Self::Caret(_) => "Caret",
        }
    }
}

/// Text markup over the quads of the text it marks.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextMarkup {
    /// Which mark.
    pub kind: MarkupKind,
    /// One quad per piece of text, at least one: usually one per line.
    pub quads: Vec<Quad>,
}

impl TextMarkup {
    /// The mark `kind` over `quads`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(kind: MarkupKind, quads: Vec<Quad>) -> Self {
        Self { kind, quads }
    }
}

/// The four text-markup subtypes (Table 171).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum MarkupKind {
    /// `Highlight`: colour behind the text, multiplied so the text stays legible.
    Highlight,
    /// `Underline`.
    Underline,
    /// `StrikeOut`: a line through the middle of the text.
    StrikeOut,
    /// `Squiggly`: a wavy underline.
    Squiggly,
}

impl MarkupKind {
    /// The `/Subtype` name.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn subtype(self) -> &'static str {
        match self {
            Self::Highlight => "Highlight",
            Self::Underline => "Underline",
            Self::StrikeOut => "StrikeOut",
            Self::Squiggly => "Squiggly",
        }
    }

    /// The kind whose `/Subtype` is `name`.
    #[must_use]
    pub(crate) fn from_subtype(name: &[u8]) -> Option<Self> {
        match name {
            b"Highlight" => Some(Self::Highlight),
            b"Underline" => Some(Self::Underline),
            b"StrikeOut" => Some(Self::StrikeOut),
            b"Squiggly" => Some(Self::Squiggly),
            _ => None,
        }
    }
}

/// A sticky note: an icon on the page and a Popup that shows its text.
///
/// The icon is 20 by 20 user-space units with its upper-left corner at [`Note::at`], and stays
/// that size and upright however the page is zoomed or turned (the NoZoom and NoRotate flags,
/// as Acrobat writes them). The Popup sits to the icon's right.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Note {
    /// The icon's upper-left corner.
    pub at: Point,
    /// The icon (`/Name`).
    pub icon: NoteIcon,
    /// Whether the Popup starts open (`/Open`).
    pub open: bool,
}

impl Note {
    /// A closed note with `icon` at `at`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(at: Point, icon: NoteIcon) -> Self {
        Self { at, icon, open: false }
    }

    /// With the Popup open or closed.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The side of a note's icon, in user-space units.
    pub(crate) const ICON: f64 = 20.0;

    /// The Popup's size, in user-space units.
    pub(crate) const POPUP: (f64, f64) = (180.0, 120.0);
}

/// The seven standard note icons (ISO 32000-2 §12.5.6.4).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum NoteIcon {
    /// `Note`: a sheet with lines, the default.
    #[default]
    Note,
    /// `Comment`: a speech bubble.
    Comment,
    /// `Key`.
    Key,
    /// `Help`: a question mark.
    Help,
    /// `Paragraph`: a pilcrow.
    Paragraph,
    /// `NewParagraph`: an upward arrow over "NP".
    NewParagraph,
    /// `Insert`: a caret.
    Insert,
}

impl NoteIcon {
    /// The `/Name`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Comment => "Comment",
            Self::Key => "Key",
            Self::Help => "Help",
            Self::Paragraph => "Paragraph",
            Self::NewParagraph => "NewParagraph",
            Self::Insert => "Insert",
        }
    }

    /// The icon named `name`, `Note` for any other (§12.5.6.4: a reader may show its own
    /// icon for a name it doesn't know).
    #[must_use]
    pub(crate) fn from_name(name: &[u8]) -> Self {
        [
            Self::Comment,
            Self::Key,
            Self::Help,
            Self::Paragraph,
            Self::NewParagraph,
            Self::Insert,
        ]
        .into_iter()
        .find(|i| i.name().as_bytes() == name)
        .unwrap_or_default()
    }
}

/// A caret: where text should go (Table 186).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Caret {
    /// Where it is drawn: the caret fills the rectangle.
    pub rect: Rect,
    /// `/Sy`: nothing, or a paragraph symbol for a new paragraph.
    #[serde(default)]
    pub symbol: CaretSymbol,
}

impl Caret {
    /// A plain caret in `rect`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            symbol: CaretSymbol::None,
        }
    }
}

/// A caret's symbol (`/Sy`).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum CaretSymbol {
    /// `/None`.
    #[default]
    None,
    /// `/P`: a new paragraph.
    Paragraph,
}

/// A change to the spec of an annotation, of the same `type` it has: absent fields are left
/// alone. Applying it regenerates the appearance.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum AnnotSpecPatch {
    /// For text markup.
    TextMarkup(TextMarkupPatch),
    /// For a note.
    Note(NotePatch),
    /// For a caret.
    Caret(CaretPatch),
}

/// A change to text markup.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextMarkupPatch {
    /// The quads it marks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub quads: Option<Vec<Quad>>,
}

/// A change to a note.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct NotePatch {
    /// The icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub icon: Option<NoteIcon>,
    /// Whether the Popup is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub open: Option<bool>,
}

/// A change to a caret.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct CaretPatch {
    /// Where it is drawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub rect: Option<Rect>,
    /// Its symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub symbol: Option<CaretSymbol>,
}

impl AnnotSpecPatch {
    /// `spec` with the patch applied; `None` when the patch is for another type.
    #[must_use]
    pub(crate) fn applied_to(&self, spec: &AnnotSpec) -> Option<AnnotSpec> {
        match (self, spec) {
            (Self::TextMarkup(p), AnnotSpec::TextMarkup(m)) => Some(AnnotSpec::TextMarkup(TextMarkup {
                kind: m.kind,
                quads: p.quads.clone().unwrap_or_else(|| m.quads.clone()),
            })),
            (Self::Note(p), AnnotSpec::Note(n)) => Some(AnnotSpec::Note(Note {
                at: n.at,
                icon: p.icon.unwrap_or(n.icon),
                open: p.open.unwrap_or(n.open),
            })),
            (Self::Caret(p), AnnotSpec::Caret(c)) => Some(AnnotSpec::Caret(Caret {
                rect: p.rect.unwrap_or(c.rect),
                symbol: p.symbol.unwrap_or(c.symbol),
            })),
            _ => None,
        }
    }
}

/// The quads `/QuadPoints` holds, as eight numbers each, in Acrobat's order.
#[must_use]
pub(crate) fn quad_points(quads: &[Quad]) -> laminapdf_cos::Object {
    laminapdf_cos::Object::Array(
        quads
            .iter()
            .flat_map(|q| q.0)
            .flat_map(|p| [laminapdf_cos::Object::number(p.x()), laminapdf_cos::Object::number(p.y())])
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_reads_its_type_from_the_wire_and_refuses_unknown_fields() {
        let spec: AnnotSpec = serde_json::from_str(
            r#"{ "type": "note", "at": { "x": 72, "y": 720 }, "icon": "comment", "open": true }"#,
        )
        .unwrap();
        assert_eq!(spec.subtype(), "Text");
        assert!(
            serde_json::from_str::<AnnotSpec>(
                r#"{ "type": "caret", "rect": { "x0": 0, "y0": 0, "x1": 5, "y1": 8 }, "colour": "red" }"#
            )
            .is_err()
        );
    }

    #[test]
    fn an_unknown_icon_name_reads_as_a_note() {
        assert_eq!(NoteIcon::from_name(b"Star"), NoteIcon::Note);
        assert_eq!(NoteIcon::from_name(b"Help"), NoteIcon::Help);
    }

    #[test]
    fn a_patch_for_another_type_applies_to_nothing() {
        let caret = AnnotSpec::Caret(Caret::new(Rect::from_corners(0.0, 0.0, 5.0, 8.0).unwrap()));
        let patch = AnnotSpecPatch::Note(NotePatch::default());
        assert_eq!(patch.applied_to(&caret), None);
    }
}
