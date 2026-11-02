//! Stroke parameters (ISO 32000-2 §8.4.3) and blend modes (§11.3.5).

use laminapdf_model::Pt;

/// How open subpath ends are drawn (§8.4.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum LineCap {
    /// Square end at the endpoint (0).
    #[default]
    Butt,
    /// Semicircular end (1).
    Round,
    /// Square end extending half the line width past the endpoint (2).
    Square,
}

impl LineCap {
    /// Every line cap, in declaration order.
    pub const ALL: [Self; 3] = [Self::Butt, Self::Round, Self::Square];
}

/// How corners are drawn (§8.4.3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum LineJoin {
    /// Sharp corners, bevelled past the miter limit (0).
    #[default]
    Miter,
    /// Rounded corners (1).
    Round,
    /// Bevelled corners (2).
    Bevel,
}

impl LineJoin {
    /// Every line join, in declaration order.
    pub const ALL: [Self; 3] = [Self::Miter, Self::Round, Self::Bevel];
}

/// A dash pattern (§8.4.3.6): alternating dash and gap lengths, and how far into the pattern
/// the line starts.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dash {
    /// Dash and gap lengths; an empty list draws a solid line.
    pub array: Vec<Pt>,
    /// Where in the pattern the line starts.
    pub phase: Pt,
}

/// How a path is stroked. Lengths are in points (page space, physical).
///
/// ```
/// use laminapdf_build::{Dash, LineJoin, StrokeStyle};
/// use laminapdf_model::Pt;
///
/// let style = StrokeStyle {
///     width: Pt::new(6.0),
///     join: LineJoin::Round,
///     dash: Some(Dash { array: vec![Pt::new(18.0), Pt::new(6.0)], phase: Pt::ZERO }),
///     ..StrokeStyle::default()
/// };
/// assert_eq!(style.miter_limit, 10.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeStyle {
    /// Line width (default 1 pt).
    pub width: Pt,
    /// Line cap (default butt).
    pub cap: LineCap,
    /// Line join (default miter).
    pub join: LineJoin,
    /// Miter limit (default 10, as in PDF).
    pub miter_limit: f64,
    /// Dash pattern (default solid).
    pub dash: Option<Dash>,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: Pt::new(1.0),
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash: None,
        }
    }
}

impl StrokeStyle {
    /// What PDF can't express in this style (§8.4.3): a width that is negative or not finite,
    /// a miter limit below 1, or a dash pattern with a negative or non-finite length, a phase
    /// that isn't finite, or only zeros.
    pub(crate) fn problem(&self) -> Option<&'static str> {
        let width = self.width.get();
        if !width.is_finite() || width < 0.0 {
            return Some("a line width must be a finite number of at least 0");
        }
        if !self.miter_limit.is_finite() || self.miter_limit < 1.0 {
            return Some("a miter limit must be a finite number of at least 1");
        }
        if let Some(dash) = &self.dash {
            let lengths = dash.array.iter().map(|d| d.get());
            if !dash.phase.get().is_finite() || lengths.clone().any(|d| !d.is_finite() || d < 0.0) {
                return Some("dash lengths must be finite numbers of at least 0");
            }
            if !dash.array.is_empty() && lengths.clone().all(|d| d == 0.0) {
                return Some("a dash pattern can't be all zeros");
            }
        }
        None
    }
}

/// A blend mode (§11.3.5, Tables 134 and 135).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[non_exhaustive]
pub enum BlendMode {
    /// `Normal`.
    #[default]
    Normal,
    /// `Multiply`.
    Multiply,
    /// `Screen`.
    Screen,
    /// `Overlay`.
    Overlay,
    /// `Darken`.
    Darken,
    /// `Lighten`.
    Lighten,
    /// `ColorDodge`.
    ColorDodge,
    /// `ColorBurn`.
    ColorBurn,
    /// `HardLight`.
    HardLight,
    /// `SoftLight`.
    SoftLight,
    /// `Difference`.
    Difference,
    /// `Exclusion`.
    Exclusion,
    /// `Hue`.
    Hue,
    /// `Saturation`.
    Saturation,
    /// `Color`.
    Color,
    /// `Luminosity`.
    Luminosity,
}

impl BlendMode {
    /// Every blend mode, in declaration order.
    pub const ALL: [Self; 16] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Darken,
        Self::Lighten,
        Self::ColorDodge,
        Self::ColorBurn,
        Self::HardLight,
        Self::SoftLight,
        Self::Difference,
        Self::Exclusion,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
    ];

    /// The PDF name (`/BM`).
    #[must_use]
    pub const fn pdf_name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "ColorDodge",
            Self::ColorBurn => "ColorBurn",
            Self::HardLight => "HardLight",
            Self::SoftLight => "SoftLight",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The match is exhaustive, so a new variant fails to build until it has an arm here and a
    /// place in `ALL`.
    #[test]
    fn all_should_list_every_line_cap_once_in_declaration_order() {
        let index = |v: LineCap| match v {
            LineCap::Butt => 0,
            LineCap::Round => 1,
            LineCap::Square => 2,
        };
        assert_eq!(LineCap::ALL.map(index), [0, 1, 2]);
    }

    /// The match is exhaustive, so a new variant fails to build until it has an arm here and a
    /// place in `ALL`.
    #[test]
    fn all_should_list_every_line_join_once_in_declaration_order() {
        let index = |v: LineJoin| match v {
            LineJoin::Miter => 0,
            LineJoin::Round => 1,
            LineJoin::Bevel => 2,
        };
        assert_eq!(LineJoin::ALL.map(index), [0, 1, 2]);
    }

    /// The match is exhaustive, so a new variant fails to build until it has an arm here and a
    /// place in `ALL`.
    #[test]
    fn all_should_list_every_blend_mode_once_in_declaration_order() {
        let index = |v: BlendMode| match v {
            BlendMode::Normal => 0,
            BlendMode::Multiply => 1,
            BlendMode::Screen => 2,
            BlendMode::Overlay => 3,
            BlendMode::Darken => 4,
            BlendMode::Lighten => 5,
            BlendMode::ColorDodge => 6,
            BlendMode::ColorBurn => 7,
            BlendMode::HardLight => 8,
            BlendMode::SoftLight => 9,
            BlendMode::Difference => 10,
            BlendMode::Exclusion => 11,
            BlendMode::Hue => 12,
            BlendMode::Saturation => 13,
            BlendMode::Color => 14,
            BlendMode::Luminosity => 15,
        };
        assert_eq!(
            BlendMode::ALL.map(index),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    fn dashed(array: &[f64]) -> StrokeStyle {
        StrokeStyle {
            dash: Some(Dash {
                array: array.iter().map(|d| Pt::new(*d)).collect(),
                phase: Pt::ZERO,
            }),
            ..StrokeStyle::default()
        }
    }

    #[test]
    fn the_default_style_is_valid() {
        assert_eq!((StrokeStyle::default().problem(), dashed(&[]).problem()), (None, None));
    }

    #[test]
    fn a_negative_width_is_refused() {
        let style = StrokeStyle {
            width: Pt::new(-1.0),
            ..StrokeStyle::default()
        };
        assert!(style.problem().is_some());
    }

    #[test]
    fn an_all_zero_or_negative_dash_is_refused() {
        assert!(dashed(&[0.0, 0.0]).problem().is_some() && dashed(&[3.0, -1.0]).problem().is_some());
    }

    #[test]
    fn a_miter_limit_below_one_is_refused() {
        let style = StrokeStyle {
            miter_limit: 0.5,
            ..StrokeStyle::default()
        };
        assert!(style.problem().is_some());
    }
}
