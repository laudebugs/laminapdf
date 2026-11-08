//! Stroke parameters (ISO 32000-2 §8.4.3).

use laminapdf_model::{Dash, Pt};

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

/// How a path is stroked. Lengths are in points (page space, physical).
///
/// ```
/// use laminapdf_build::{Dash, LineJoin, StrokeStyle};
/// use laminapdf_model::Pt;
///
/// let style = StrokeStyle {
///     width: Pt::new(6.0),
///     join: LineJoin::Round,
///     dash: Some(Dash::new(vec![18.0, 6.0], 0.0)),
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
            let lengths = dash.array.iter().copied();
            if !dash.phase.is_finite() || lengths.clone().any(|d| !d.is_finite() || d < 0.0) {
                return Some("dash lengths must be finite numbers of at least 0");
            }
            if !dash.array.is_empty() && lengths.clone().all(|d| d == 0.0) {
                return Some("a dash pattern can't be all zeros");
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The match is exhaustive, so a new variant fails to build here first: give it the next
    /// index and add it to `ALL` there, which this test then checks.
    #[test]
    fn all_should_list_every_line_cap_once_in_declaration_order() {
        let index = |v: LineCap| match v {
            LineCap::Butt => 0,
            LineCap::Round => 1,
            LineCap::Square => 2,
        };
        assert!(LineCap::ALL.iter().enumerate().all(|(i, v)| index(*v) == i));
    }

    /// The match is exhaustive, so a new variant fails to build here first: give it the next
    /// index and add it to `ALL` there, which this test then checks.
    #[test]
    fn all_should_list_every_line_join_once_in_declaration_order() {
        let index = |v: LineJoin| match v {
            LineJoin::Miter => 0,
            LineJoin::Round => 1,
            LineJoin::Bevel => 2,
        };
        assert!(LineJoin::ALL.iter().enumerate().all(|(i, v)| index(*v) == i));
    }

    fn dashed(array: &[f64]) -> StrokeStyle {
        StrokeStyle {
            dash: Some(Dash::new(array.to_vec(), 0.0)),
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
