//! `RotatePages`: `/Rotate` on many pages at once (ISO 32000-2 §7.7.3.3).

use laminapdf_model::{PageId, Rotation};

use super::{checked, pages_before, widgets_on};
use crate::class::ModificationClass;
use crate::error::Result;
use crate::op::Effect;
use crate::rotate::RotatePage;
use crate::widgets::WidgetChange;
use crate::wire;

/// How [`RotatePages`] reads its angle.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum RotateMode {
    /// Each page shows at the angle.
    #[default]
    Set,
    /// The angle is added to each page's own rotation, as Rotate left and right do.
    Turn,
}

/// Rotates `pages` by `angle`, set or turned ([`RotateMode`]); each page as
/// [`RotatePage`] does, keeping its object and [`PageId`].
///
/// On the wire: `{ "op": "rotatePages", "args": { "pages": ["p3.0"], "angle": 90, "mode": "turn" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct RotatePages {
    /// The pages.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// The angle in degrees clockwise: 0, 90, 180 or 270.
    #[serde(with = "wire::rotation")]
    #[cfg_attr(feature = "tsify", tsify(type = "0 | 90 | 180 | 270"))]
    pub angle: Rotation,
    /// Whether the angle is set or added; `set` when left out.
    #[serde(default)]
    pub mode: RotateMode,
}

impl RotatePages {
    /// The op that shows `pages` at `angle`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn set(pages: Vec<PageId>, angle: Rotation) -> Self {
        Self {
            pages,
            angle,
            mode: RotateMode::Set,
        }
    }

    /// The op that turns `pages` by `angle`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn turn(pages: Vec<PageId>, angle: Rotation) -> Self {
        Self {
            pages,
            angle,
            mode: RotateMode::Turn,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let all = pages_before(doc)?;
        let pages = checked(&all, &self.pages)?;
        let mut changed: Vec<PageId> = Vec::new();
        for page in pages {
            let to = match self.mode {
                RotateMode::Set => self.angle,
                RotateMode::Turn => turned(shown(doc, &page)?, self.angle),
            };
            changed.extend(RotatePage::new(page, to).apply_to(doc)?);
        }
        let widgets = widgets_on(doc, &changed, WidgetChange::Reshown, ModificationClass::PageStructure)?;
        Ok(Effect {
            pages: changed,
            widgets,
            ..Effect::default()
        })
    }
}

/// The rotation `page` shows (own or inherited); a value that isn't a multiple of 90 reads as
/// upright, as `RotatePage` treats it.
fn shown(doc: &laminapdf_model::Document, page: &PageId) -> Result<Rotation> {
    let pages = doc.pages()?;
    match pages.by_id(page) {
        Some(p) => Ok(p.rotate_quietly()?),
        None => Ok(Rotation::None),
    }
}

/// `from` turned clockwise by `by`.
fn turned(from: Rotation, by: Rotation) -> Rotation {
    Rotation::from_canonical_degrees((from.degrees() + by.degrees()) % 360).unwrap_or(Rotation::None)
}
