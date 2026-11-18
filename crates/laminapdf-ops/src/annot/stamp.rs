//! The `Stamp` spec (ISO 32000-2 §12.5.6.12): a standard stamp drawn from its name, an image
//! from the blob store, or a dynamic stamp whose lines the payload carries already resolved
//! ("Approved by A. Mensah, 12 Nov 2026"), never by a script.

use laminapdf_cos::{Dict, Object, PdfString};
use laminapdf_model::UserSpaceRect;
use laminapdf_syntax::text::{decode_text_string, encode_text_string};
use serde::{Deserialize, Serialize};

use super::geom;
use crate::blob::BlobId;
use crate::wire;

/// The private key that holds a dynamic stamp's lines, so it can be drawn again.
const LINES: &str = "LMNA_Lines";
/// The private key that holds a dynamic stamp's colour.
const STYLE: &str = "LMNA_Style";
/// `/Name` of an image stamp: no standard name describes it.
const IMAGE_NAME: &str = "LMNA_Image";
/// `/Name` of a dynamic stamp.
const DYNAMIC_NAME: &str = "LMNA_Dynamic";

/// A stamp over `rect`: a standard name, an image or resolved lines of text, drawn upright on a
/// page whose `/Rotate` is `rotation`.
///
/// On the wire: `{ "type": "stamp", "rect": { … }, "stamp": { "standard": "approved" } }`,
/// `{ "image": "<64 hex digits>" }` or `{ "dynamic": { "lines": ["APPROVED", "by R. Osei, 29
/// Oct 2026"], "style": "green" } }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Stamp {
    /// The stamp's box, in user space: its `/Rect`.
    #[serde(with = "wire::user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: UserSpaceRect,
    /// The page's `/Rotate`, 0, 90, 180 or 270, so the stamp reads upright as the page is shown.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub rotation: u16,
    /// What it shows.
    pub stamp: StampKind,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_zero(rotation: &u16) -> bool {
    *rotation == 0
}

impl Stamp {
    /// A stamp over `rect` showing `stamp`, upright on an unrotated page.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(rect: UserSpaceRect, stamp: StampKind) -> Self {
        Self {
            rect,
            rotation: 0,
            stamp,
        }
    }

    /// The same stamp, upright on a page whose `/Rotate` is `rotation`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn with_rotation(mut self, rotation: u16) -> Self {
        self.rotation = rotation;
        self
    }

    /// The text a screen reader reads when the op gives no `/Contents`: the stamp's name, or its
    /// lines.
    pub(crate) fn default_contents(&self) -> Option<String> {
        match &self.stamp {
            StampKind::Standard(name) => Some(name.words().to_owned()),
            StampKind::Dynamic(d) => Some(d.lines.join(" ")),
            StampKind::Image(_) => None,
        }
    }

    pub(crate) fn is_valid(&self) -> bool {
        let r = &self.rect;
        geom::finite(geom::corners(r))
            && r.x1 - r.x0 > 0.0
            && r.y1 - r.y0 > 0.0
            && matches!(self.rotation, 0 | 90 | 180 | 270)
            && match &self.stamp {
                StampKind::Dynamic(d) => !d.lines.is_empty() && d.lines.len() <= DynamicStamp::MAX_LINES,
                StampKind::Standard(_) | StampKind::Image(_) => true,
            }
    }

    /// Writes `/Name`, `/Rotate` and a dynamic stamp's lines and colour.
    pub(crate) fn write(&self, dict: &mut Dict) {
        let name = match &self.stamp {
            StampKind::Standard(name) => name.name(),
            StampKind::Image(_) => IMAGE_NAME,
            StampKind::Dynamic(d) => {
                let lines = d
                    .lines
                    .iter()
                    .map(|l| Object::String(PdfString::literal(encode_text_string(l))))
                    .collect();
                dict.insert(LINES, Object::Array(lines));
                dict.insert(STYLE, Object::name(d.style.name()));
                DYNAMIC_NAME
            }
        };
        dict.insert("Name", Object::name(name));
        if self.rotation == 0 {
            dict.remove("Rotate");
        } else {
            dict.insert("Rotate", Object::Integer(i64::from(self.rotation)));
        }
    }

    /// The stamp `dict` describes, when Lamina PDF can draw it again: a standard name, or a
    /// dynamic stamp with its lines. An image stamp's image lives only in its appearance, and an
    /// unknown name is another app's own stamp, so neither has a spec.
    pub(crate) fn read(dict: &Dict, rect: Option<UserSpaceRect>) -> Option<Self> {
        let rect = rect?;
        let rotation = dict.get("Rotate").and_then(Object::as_int).unwrap_or(0).rem_euclid(360);
        let rotation = u16::try_from(rotation - rotation % 90).unwrap_or(0);
        let name = dict.get_name("Name").unwrap_or(b"Draft");
        let stamp = if name == DYNAMIC_NAME.as_bytes() {
            let lines: Vec<String> = dict
                .get(LINES)
                .and_then(Object::as_array)?
                .iter()
                .filter_map(|o| match o {
                    Object::String(s) => Some(decode_text_string(&s.bytes)),
                    _ => None,
                })
                .collect();
            let style = dict.get_name(STYLE).map_or(StampStyle::Blue, StampStyle::from_name);
            StampKind::Dynamic(DynamicStamp { lines, style })
        } else {
            StampKind::Standard(StampName::from_name(name)?)
        };
        let stamp = Self { rect, rotation, stamp };
        stamp.is_valid().then_some(stamp)
    }
}

/// What a stamp shows.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum StampKind {
    /// A standard stamp, its name drawn in capitals in a frame of its colour. It needs no font.
    Standard(StampName),
    /// A JPEG or PNG image, by its ID in the blob store, fitted to the box with its proportions
    /// kept.
    Image(#[cfg_attr(feature = "tsify", tsify(type = "string"))] BlobId),
    /// Lines of text, the first in bold, in a frame. It needs text layout, which the WebAssembly
    /// base module lacks (`appearance-needs-text`), and characters Helvetica draws
    /// (`annotation-glyph-missing`).
    Dynamic(DynamicStamp),
}

/// A dynamic stamp's lines, resolved before the op is made, and its colour.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DynamicStamp {
    /// One to eight lines; the first is the stamp's title.
    pub lines: Vec<String>,
    /// Its colour.
    #[serde(default)]
    pub style: StampStyle,
}

impl DynamicStamp {
    /// The most lines a dynamic stamp holds.
    pub(crate) const MAX_LINES: usize = 8;

    /// The stamp of `lines` in `style`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(lines: Vec<String>, style: StampStyle) -> Self {
        Self { lines, style }
    }
}

/// A stamp's colour, as Acrobat's stamps use them: green for approval, red for refusal, blue for
/// everything else.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum StampStyle {
    /// Green.
    Green,
    /// Red.
    Red,
    /// Blue, the default.
    #[default]
    Blue,
}

impl StampStyle {
    const fn name(self) -> &'static str {
        match self {
            Self::Green => "Green",
            Self::Red => "Red",
            Self::Blue => "Blue",
        }
    }

    fn from_name(name: &[u8]) -> Self {
        match name {
            b"Green" => Self::Green,
            b"Red" => Self::Red,
            _ => Self::Blue,
        }
    }
}

/// The standard stamps: the fourteen names of ISO 32000-2 §12.5.6.12, and `Reviewed` and
/// `Void`, which Acrobat's business stamps add.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum StampName {
    /// Approved.
    Approved,
    /// Experimental.
    Experimental,
    /// Not approved.
    NotApproved,
    /// As is.
    AsIs,
    /// Expired.
    Expired,
    /// Not for public release.
    NotForPublicRelease,
    /// Confidential.
    Confidential,
    /// Final.
    Final,
    /// Sold.
    Sold,
    /// Departmental.
    Departmental,
    /// For comment.
    ForComment,
    /// Top secret.
    TopSecret,
    /// Draft, the name readers assume when a stamp has none.
    Draft,
    /// For public release.
    ForPublicRelease,
    /// Reviewed (not one of ISO 32000-2's names).
    Reviewed,
    /// Void (not one of ISO 32000-2's names).
    Void,
}

/// One standard stamp's facts: its `/Name`, its lettering in capitals, its name in words (its
/// `/Contents` by default) and its colour.
struct StampRow {
    stamp: StampName,
    name: &'static str,
    lettering: &'static str,
    words: &'static str,
    style: StampStyle,
}

/// Every standard stamp.
const STAMPS: [StampRow; 16] = {
    use StampName as N;
    use StampStyle::{Blue, Green, Red};
    const fn row(
        stamp: N,
        name: &'static str,
        lettering: &'static str,
        words: &'static str,
        style: StampStyle,
    ) -> StampRow {
        StampRow {
            stamp,
            name,
            lettering,
            words,
            style,
        }
    }
    [
        row(N::Approved, "Approved", "APPROVED", "Approved", Green),
        row(N::Experimental, "Experimental", "EXPERIMENTAL", "Experimental", Blue),
        row(N::NotApproved, "NotApproved", "NOT APPROVED", "Not approved", Red),
        row(N::AsIs, "AsIs", "AS IS", "As is", Blue),
        row(N::Expired, "Expired", "EXPIRED", "Expired", Red),
        row(
            N::NotForPublicRelease,
            "NotForPublicRelease",
            "NOT FOR PUBLIC RELEASE",
            "Not for public release",
            Red,
        ),
        row(N::Confidential, "Confidential", "CONFIDENTIAL", "Confidential", Red),
        row(N::Final, "Final", "FINAL", "Final", Green),
        row(N::Sold, "Sold", "SOLD", "Sold", Green),
        row(N::Departmental, "Departmental", "DEPARTMENTAL", "Departmental", Blue),
        row(N::ForComment, "ForComment", "FOR COMMENT", "For comment", Blue),
        row(N::TopSecret, "TopSecret", "TOP SECRET", "Top secret", Red),
        row(N::Draft, "Draft", "DRAFT", "Draft", Blue),
        row(
            N::ForPublicRelease,
            "ForPublicRelease",
            "FOR PUBLIC RELEASE",
            "For public release",
            Green,
        ),
        row(N::Reviewed, "Reviewed", "REVIEWED", "Reviewed", Green),
        row(N::Void, "Void", "VOID", "Void", Red),
    ]
};

impl StampName {
    /// Every name, in the order of the variants.
    ///
    /// Added in 0.6.0.
    pub const ALL: [Self; 16] = [
        Self::Approved,
        Self::Experimental,
        Self::NotApproved,
        Self::AsIs,
        Self::Expired,
        Self::NotForPublicRelease,
        Self::Confidential,
        Self::Final,
        Self::Sold,
        Self::Departmental,
        Self::ForComment,
        Self::TopSecret,
        Self::Draft,
        Self::ForPublicRelease,
        Self::Reviewed,
        Self::Void,
    ];

    /// Its row of [`STAMPS`], which lists every name (a test holds it to [`Self::ALL`]).
    fn row(self) -> &'static StampRow {
        const DRAFT: StampRow = StampRow {
            stamp: StampName::Draft,
            name: "Draft",
            lettering: "DRAFT",
            words: "Draft",
            style: StampStyle::Blue,
        };
        STAMPS.iter().find(|r| r.stamp == self).unwrap_or(&DRAFT)
    }

    /// Its `/Name`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.row().name
    }

    /// Its lettering, in capitals.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn lettering(self) -> &'static str {
        self.row().lettering
    }

    /// Its name in words, which its `/Contents` defaults to.
    fn words(self) -> &'static str {
        self.row().words
    }

    /// Its colour.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn style(self) -> StampStyle {
        self.row().style
    }

    /// The stamp a `/Name` names; `None` for any other name.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn from_name(name: &[u8]) -> Option<Self> {
        STAMPS.iter().find(|r| r.name.as_bytes() == name).map(|r| r.stamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> UserSpaceRect {
        UserSpaceRect::from_corners(100.0, 600.0, 250.0, 650.0)
    }

    #[test]
    fn a_standard_stamp_reads_back_from_its_keys() {
        let stamp = Stamp::new(rect(), StampKind::Standard(StampName::NotApproved)).with_rotation(90);
        let mut dict = Dict::new();
        stamp.write(&mut dict);
        assert_eq!(Stamp::read(&dict, Some(rect())), Some(stamp));
    }

    #[test]
    fn a_dynamic_stamp_keeps_its_lines_and_colour() {
        let lines = vec!["APPROVED".to_owned(), "by R. Osei · 29 Oct 2026".to_owned()];
        let stamp = Stamp::new(rect(), StampKind::Dynamic(DynamicStamp::new(lines, StampStyle::Green)));
        let mut dict = Dict::new();
        stamp.write(&mut dict);
        assert_eq!(dict.get_name("Name"), Some(b"LMNA_Dynamic".as_slice()));
        assert_eq!(Stamp::read(&dict, Some(rect())), Some(stamp));
    }

    #[test]
    fn an_image_stamp_and_another_apps_name_have_no_spec() {
        let image = Stamp::new(rect(), StampKind::Image(BlobId::of(b"png")));
        let mut dict = Dict::new();
        image.write(&mut dict);
        assert_eq!(Stamp::read(&dict, Some(rect())), None);
        let other: Dict = [("Name", Object::name("SBSignHere"))].into_iter().collect();
        assert_eq!(Stamp::read(&other, Some(rect())), None);
    }

    #[test]
    fn each_name_is_its_own_row() {
        assert_eq!(STAMPS.map(|r| r.stamp), StampName::ALL);
        // So `row` finds every name in the table, never its fallback.
        assert!(
            StampName::ALL
                .iter()
                .all(|n| n.row().stamp == *n && n.row().name == STAMPS.iter().find(|r| r.stamp == *n).unwrap().name)
        );
    }

    #[test]
    fn a_stamp_without_a_name_is_a_draft() {
        assert_eq!(
            Stamp::read(&Dict::new(), Some(rect())).map(|s| s.stamp),
            Some(StampKind::Standard(StampName::Draft))
        );
    }

    #[test]
    fn the_wire_names_the_kind() {
        let stamp: Stamp = serde_json::from_str(
            r#"{ "rect": { "x0": 0, "y0": 0, "x1": 100, "y1": 40 }, "stamp": { "standard": "forComment" } }"#,
        )
        .unwrap();
        assert_eq!(stamp.stamp, StampKind::Standard(StampName::ForComment));
    }

    #[test]
    fn a_turn_that_isnt_a_quarter_or_an_empty_stamp_is_invalid() {
        assert!(
            !Stamp::new(rect(), StampKind::Standard(StampName::Final))
                .with_rotation(45)
                .is_valid()
        );
        let empty = StampKind::Dynamic(DynamicStamp::new(Vec::new(), StampStyle::Blue));
        assert!(!Stamp::new(rect(), empty).is_valid());
    }
}
