//! What kind of mark an [`AddAnnot`](super::AddAnnot) makes ([`AnnotSpec`]), and the changes an
//! [`UpdateAnnot`](super::UpdateAnnot) can make to one ([`AnnotSpecPatch`]). Geometry is in user
//! space, so a later rotation or crop leaves a spec's meaning unchanged.

use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use serde::{Deserialize, Serialize};

use super::geom;
use crate::wire;

/// The kind of mark an annotation op makes, with its geometry. Every spec gets an appearance
/// stream ([`Appearances`](crate::Appearances)).
///
/// On the wire the kind is the `type` field: `{ "type": "textMarkup", "kind": "highlight",
/// "quads": [ … ] }`.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

// Sound: an op holding a non-finite coordinate is refused before it writes (`invalid-op`), and
// ops are compared only after they parse.
impl Eq for AnnotSpec {}

impl AnnotSpec {
    /// The `/Subtype` the spec writes.
    pub(crate) const fn subtype(&self) -> &'static str {
        match self {
            Self::TextMarkup(m) => m.kind.subtype(),
            Self::Note(_) => "Text",
            Self::Caret(_) => "Caret",
        }
    }

    /// The same spec with every quad in Acrobat's order, which both the appearance and
    /// `/QuadPoints` use, whichever order the op gave.
    pub(crate) fn normalised(&self) -> Self {
        match self {
            Self::TextMarkup(m) => Self::TextMarkup(TextMarkup::new(
                m.kind,
                m.quads.iter().map(|q| geom::acrobat_order(*q)).collect(),
            )),
            other => other.clone(),
        }
    }

    /// Whether every coordinate is finite and the geometry is there: a markup has quads.
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::TextMarkup(m) => !m.quads.is_empty() && geom::finite(m.quads.iter().flatten().copied()),
            Self::Note(n) => geom::finite([n.at]),
            Self::Caret(c) => geom::finite(geom::corners(&c.rect)),
        }
    }
}

/// Text markup over the quads of the text it marks.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextMarkup {
    /// Which mark.
    pub kind: MarkupKind,
    /// One quad per piece of text, at least one: usually one per line, four corners each in
    /// user space, upper left, upper right, lower left, lower right.
    #[serde(with = "wire::quads")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }[][]"))]
    pub quads: Vec<[UserSpacePoint; 4]>,
}

impl TextMarkup {
    /// The mark `kind` over `quads`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn new(kind: MarkupKind, quads: Vec<[UserSpacePoint; 4]>) -> Self {
        Self { kind, quads }
    }
}

/// Which text markup.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum MarkupKind {
    /// A highlight, which multiplies with the text so it stays legible.
    Highlight,
    /// A line under the text.
    Underline,
    /// A line through the text.
    StrikeOut,
    /// A wavy line under the text.
    Squiggly,
}

impl MarkupKind {
    /// Its `/Subtype`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn subtype(self) -> &'static str {
        match self {
            Self::Highlight => "Highlight",
            Self::Underline => "Underline",
            Self::StrikeOut => "StrikeOut",
            Self::Squiggly => "Squiggly",
        }
    }

    /// The markup a `/Subtype` names.
    pub(crate) fn from_subtype(name: &[u8]) -> Option<Self> {
        [Self::Highlight, Self::Underline, Self::StrikeOut, Self::Squiggly]
            .into_iter()
            .find(|k| k.subtype().as_bytes() == name)
    }
}

/// A sticky note at a point: a 20-point icon whose upper-left corner is `at`, and a Popup to its
/// right that shows the comment, open or closed.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Note {
    /// The icon's upper-left corner, in user space.
    #[serde(with = "wire::user_space_point")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }"))]
    pub at: UserSpacePoint,
    /// The icon.
    #[serde(default)]
    pub icon: NoteIcon,
    /// Whether the Popup shows when the file opens (`/Open`).
    #[serde(default)]
    pub open: bool,
}

impl Note {
    /// The side of a note's icon, in points.
    ///
    /// Added in 0.3.0.
    pub const ICON: f64 = 20.0;
    /// The Popup's width and height, in points, as the page is shown.
    pub(crate) const POPUP: (f64, f64) = (180.0, 120.0);

    /// A closed note with `icon` at `at`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn new(at: UserSpacePoint, icon: NoteIcon) -> Self {
        Self { at, icon, open: false }
    }

    /// The same note, its Popup open or closed.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

/// A note's icon: the seven standard names (ISO 32000-2 Table 183).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum NoteIcon {
    /// A page of text, the default.
    #[default]
    Note,
    /// A speech bubble.
    Comment,
    /// A key.
    Key,
    /// A question mark.
    Help,
    /// A paragraph sign.
    Paragraph,
    /// A new-paragraph mark.
    NewParagraph,
    /// An insertion caret.
    Insert,
}

impl NoteIcon {
    /// Its `/Name`.
    ///
    /// Added in 0.3.0.
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

    /// The icon a `/Name` names; any other name is drawn as a note, as readers do.
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

/// A caret filling `rect`, marking where text goes; with a paragraph sign, a new paragraph.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Caret {
    /// The caret's box, in user space.
    #[serde(with = "wire::user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: UserSpaceRect,
    /// The symbol it carries (`/Sy`).
    #[serde(default)]
    pub symbol: CaretSymbol,
}

impl Caret {
    /// A plain caret in `rect`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn new(rect: UserSpaceRect) -> Self {
        Self {
            rect,
            symbol: CaretSymbol::None,
        }
    }
}

/// What a caret carries (`/Sy`).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum CaretSymbol {
    /// Nothing: text goes here.
    #[default]
    None,
    /// A paragraph sign: a new paragraph starts here.
    Paragraph,
}

/// A change to an annotation's spec: its geometry or icon. The appearance is drawn again. A patch
/// of another type than the annotation's is `invalid-op`.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum AnnotSpecPatch {
    /// New quads for a text markup.
    TextMarkup(TextMarkupPatch),
    /// A note's icon or whether its Popup is open.
    Note(NotePatch),
    /// A caret's box or symbol.
    Caret(CaretPatch),
}

// Sound: as for `AnnotSpec`.
impl Eq for AnnotSpecPatch {}

/// New quads for a text markup.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextMarkupPatch {
    /// The quads, at least one.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::quads::opt")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "{ x: number; y: number }[][]"))]
    pub quads: Option<Vec<[UserSpacePoint; 4]>>,
}

/// A note's icon, or whether its Popup is open.
///
/// Added in 0.3.0.
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

/// A caret's box or symbol.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct CaretPatch {
    /// The box.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(optional, type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: Option<UserSpaceRect>,
    /// The symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub symbol: Option<CaretSymbol>,
}

impl AnnotSpecPatch {
    /// `spec` with the patch applied; `None` when the patch is for another type.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_reads_its_type_from_the_wire() {
        let spec: AnnotSpec =
            serde_json::from_str(r#"{ "type": "note", "at": { "x": 72, "y": 720 }, "icon": "comment", "open": true }"#)
                .unwrap();
        assert_eq!(
            spec,
            AnnotSpec::Note(Note::new(UserSpacePoint::new(72.0, 720.0), NoteIcon::Comment).with_open(true))
        );
    }

    #[test]
    fn a_spec_refuses_unknown_fields() {
        assert!(
            serde_json::from_str::<AnnotSpec>(
                r#"{ "type": "caret", "rect": { "x0": 0, "y0": 0, "x1": 5, "y1": 8 }, "colour": "red" }"#
            )
            .is_err()
        );
    }

    #[test]
    fn an_unknown_icon_name_reads_as_a_note() {
        assert_eq!(
            (NoteIcon::from_name(b"Star"), NoteIcon::from_name(b"Help")),
            (NoteIcon::Note, NoteIcon::Help)
        );
    }

    #[test]
    fn a_patch_for_another_type_applies_to_nothing() {
        let caret = AnnotSpec::Caret(Caret::new(UserSpaceRect::from_corners(0.0, 0.0, 5.0, 8.0)));
        assert_eq!(AnnotSpecPatch::Note(NotePatch::default()).applied_to(&caret), None);
    }

    #[test]
    fn markup_without_quads_is_invalid() {
        assert!(!AnnotSpec::TextMarkup(TextMarkup::new(MarkupKind::Highlight, Vec::new())).is_valid());
    }
}
