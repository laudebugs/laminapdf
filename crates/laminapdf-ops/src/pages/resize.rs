//! `ResizePages` and `SetOrientation` (E2 plan §2.1): a page's paper changes and its content,
//! annotations, destinations and structure boxes follow. Both work on the page as shown (ADR
//! 0027 page space), so a page turned by `/Rotate` or drawn with a `/UserUnit` changes the same
//! way as an upright one; the move itself is `reshape.rs`'s, and the destinations' and
//! structure boxes' is `dest_map.rs`'s.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_model::{Matrix, PageId, PageSize, Rotation};
use serde::{Deserialize, Serialize};

use super::reshape::{Reshape, reshape};
use super::{checked, dest_map, pages_before, widgets_on};
use crate::class::ModificationClass;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::rotate::RotatePage;
use crate::widgets::WidgetChange;
use crate::wire;

/// Where `extend` puts the old page on the new one, on the page as shown.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum Anchor {
    /// The top-left corners meet.
    TopLeft,
    /// Centred across the top.
    Top,
    /// The top-right corners meet.
    TopRight,
    /// Centred down the left side.
    Left,
    /// Centred.
    #[default]
    Center,
    /// Centred down the right side.
    Right,
    /// The bottom-left corners meet.
    BottomLeft,
    /// Centred across the bottom.
    Bottom,
    /// The bottom-right corners meet.
    BottomRight,
}

impl Anchor {
    /// Where the anchor sits across and down, as a share of the free room: 0, ½ or 1.
    const fn shares(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
        }
    }
}

/// How [`ResizePages`] fits the content on the new paper.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum ResizeMode {
    /// The page as shown is scaled to fit the new one, keeping its proportions, and centred.
    #[default]
    Scale,
    /// The content keeps its size and is placed at `anchor`: the page grows around it, or is
    /// cut down to the part of it the new page covers.
    #[serde(rename_all = "camelCase")]
    Extend {
        /// Where the old page goes on the new one.
        #[serde(default)]
        anchor: Anchor,
    },
}

/// Gives `pages` new paper of `size` (before `/Rotate`, as [`PageSize`] always is: a page
/// turned on its side stays turned), scaling the content to fit or extending the page around it
/// ([`ResizeMode`]). The content is wrapped and moved by one matrix, and the annotations' geometry,
/// every explicit destination naming the page and its structure elements' Layout `/BBox`
/// follow the same matrix; the crop box goes (the new page shows its whole paper), and the
/// bleed, trim and art boxes move with the content.
///
/// On the wire: `{ "op": "resizePages", "args": { "pages": ["p3.0"], "size": { "width": 612,
/// "height": 792 }, "mode": "scale" } }`, or `"mode": { "extend": { "anchor": "topLeft" } }`.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct ResizePages {
    /// The pages.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// The new paper, before `/Rotate`, in points: `{ "width": 612, "height": 792 }`.
    #[serde(with = "wire::page_size")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ width: number; height: number }"))]
    pub size: PageSize,
    /// How the content fits; `scale` when left out.
    #[serde(default)]
    pub mode: ResizeMode,
}

// Ops compare by value (`Op: Eq`); the one value that isn't equal to itself, NaN, never
// reaches an op from JSON, and is refused as `invalid-op`.
impl Eq for ResizePages {}

/// The largest side a page may have, in points (PDF 1.7's implementation limit, ISO 32000-1
/// Annex C), as `InsertBlankPages` allows.
const MAX_SIDE: f64 = 14_400.0;

impl ResizePages {
    /// The op that gives `pages` paper of `size`, scaling their content to fit.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn scale(pages: Vec<PageId>, size: PageSize) -> Self {
        Self {
            pages,
            size,
            mode: ResizeMode::Scale,
        }
    }

    /// The op that gives `pages` paper of `size`, their content kept at its size and placed at
    /// `anchor`.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn extend(pages: Vec<PageId>, size: PageSize, anchor: Anchor) -> Self {
        Self {
            pages,
            size,
            mode: ResizeMode::Extend { anchor },
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let (w, h) = (self.size.width.get(), self.size.height.get());
        let side = |v: f64| v.is_finite() && v > 0.0 && v <= MAX_SIDE;
        if !side(w) || !side(h) {
            return Err(Error::InvalidOp(
                "a page's width and height are above 0 and at most 14,400 points",
            ));
        }
        let all = pages_before(doc)?;
        let pages = checked(&all, &self.pages)?;
        let mut shapes = Vec::with_capacity(pages.len());
        {
            let view = doc.pages()?;
            for id in &pages {
                let page = view.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
                let frame = page.frame()?;
                let (old_w, old_h) = (frame.width().get(), frame.height().get());
                // The new page as shown: the paper turned as the page is.
                let quarter = matches!(frame.rotate(), Rotation::Clockwise90 | Rotation::Clockwise270);
                let (new_w, new_h) = if quarter { (h, w) } else { (w, h) };
                let page_space = match self.mode {
                    ResizeMode::Scale => {
                        let s = (new_w / old_w).min(new_h / old_h);
                        Matrix::scale(s, s).then(&Matrix::translate(
                            s.mul_add(-old_w, new_w) / 2.0,
                            s.mul_add(-old_h, new_h) / 2.0,
                        ))
                    }
                    ResizeMode::Extend { anchor } => {
                        let (across, down) = anchor.shares();
                        Matrix::translate(across * (new_w - old_w), down * (new_h - old_h))
                    }
                };
                shapes.push((
                    id.clone(),
                    Reshape {
                        width: new_w,
                        height: new_h,
                        page_space,
                    },
                ));
            }
        }
        move_pages(doc, pages, shapes)
    }
}

/// Portrait (taller than wide) or landscape (wider than tall), as a page is shown.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum Orientation {
    /// Taller than wide.
    Portrait,
    /// Wider than tall.
    Landscape,
}

/// How [`SetOrientation`] turns a page.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum OrientationMethod {
    /// A quarter turn clockwise in `/Rotate`: nothing in the page changes, and readers show it
    /// turned.
    #[default]
    Rotate,
    /// The paper's sides swap and the content turns a quarter clockwise into it, so the page
    /// is upright with no `/Rotate` change: for tools that ignore `/Rotate`, such as some
    /// printers and imposition software.
    SwapBox,
}

/// Shows `pages` in `orientation`: each page shown the other way is turned a quarter clockwise,
/// by `/Rotate` or by swapping its paper's sides ([`OrientationMethod`]); pages already that
/// way, and square ones, are left alone. With `swapBox` the content, annotations,
/// destinations and structure boxes turn with the page, as [`ResizePages`] moves them.
///
/// On the wire: `{ "op": "setOrientation", "args": { "pages": ["p3.0"], "orientation":
/// "landscape", "method": "swapBox" } }`.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetOrientation {
    /// The pages.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// How the pages should be shown.
    pub orientation: Orientation,
    /// How a page is turned; `rotate` when left out.
    #[serde(default)]
    pub method: OrientationMethod,
}

impl SetOrientation {
    /// The op that shows `pages` in `orientation`, turned by `method`.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>, orientation: Orientation, method: OrientationMethod) -> Self {
        Self {
            pages,
            orientation,
            method,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let all = pages_before(doc)?;
        let pages = checked(&all, &self.pages)?;
        let mut turning = Vec::new();
        {
            let view = doc.pages()?;
            for id in &pages {
                let page = view.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
                let frame = page.frame()?;
                let (w, h) = (frame.width().get(), frame.height().get());
                let wrong = match self.orientation {
                    Orientation::Portrait => w > h,
                    Orientation::Landscape => h > w,
                };
                if wrong {
                    turning.push((id.clone(), w, h, frame.rotate()));
                }
            }
        }
        match self.method {
            OrientationMethod::Rotate => {
                let mut changed = Vec::new();
                for (id, _, _, rotate) in turning {
                    let to = Rotation::from_canonical_degrees((rotate.degrees() + 90) % 360).unwrap_or(Rotation::None);
                    changed.extend(RotatePage::new(id, to).apply_to(doc)?);
                }
                let widgets = widgets_on(doc, &changed, WidgetChange::Reshown, ModificationClass::PageStructure)?;
                Ok(Effect {
                    pages: changed,
                    widgets,
                    ..Effect::default()
                })
            }
            OrientationMethod::SwapBox => {
                let ids = turning.iter().map(|(id, ..)| id.clone()).collect();
                let shapes = turning
                    .into_iter()
                    .map(|(id, w, h, _)| {
                        // A quarter turn clockwise on the page as shown: (x, y) → (h − y, x).
                        let page_space = Matrix::new([0.0, 1.0, -1.0, 0.0, h, 0.0]);
                        (
                            id,
                            Reshape {
                                width: h,
                                height: w,
                                page_space,
                            },
                        )
                    })
                    .collect();
                move_pages(doc, ids, shapes)
            }
        }
    }
}

/// Moves each page of `shapes`, then the destinations and structure boxes that name them.
fn move_pages(
    doc: &mut laminapdf_model::Document,
    pages: Vec<PageId>,
    shapes: Vec<(PageId, Reshape)>,
) -> Result<Effect> {
    let mut appearances = BTreeSet::new();
    let mut moved = BTreeMap::new();
    for (id, shape) in shapes {
        let done = reshape(doc, &id, &shape, &mut appearances)?;
        moved.insert(id, done.user_space);
    }
    dest_map::follow(doc, &moved)?;
    let widgets = widgets_on(doc, &pages, WidgetChange::Reshown, ModificationClass::PageStructure)?;
    Ok(Effect {
        pages,
        widgets,
        ..Effect::default()
    })
}
