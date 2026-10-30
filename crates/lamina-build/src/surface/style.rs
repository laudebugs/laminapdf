//! Stroke parameters (ISO 32000-2 §8.4.3) and blend modes (§11.3.5).

use lamina_model::Pt;

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
/// use lamina_build::{Dash, LineJoin, StrokeStyle};
/// use lamina_model::Pt;
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
