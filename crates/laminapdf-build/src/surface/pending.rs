//! A path being built: segments in page space, painted or clipped once with `fill`, `stroke`,
//! `fill_stroke` or `clip`.

use super::path::{FillRule, PathBuilder};
use super::style::StrokeStyle;
use super::{Command, Paint, Surface};

/// A path built by [`Surface::path`], waiting to be painted. Dropping it draws nothing.
#[derive(Debug)]
#[must_use = "a path is drawn only when it is painted (fill, stroke, fill_stroke or clip)"]
pub struct PendingPath<'s> {
    pub(super) surface: &'s mut Surface,
    pub(super) path: PathBuilder,
}

impl<'s> PendingPath<'s> {
    pub(super) fn paint(self, paint: Paint) -> &'s mut Surface {
        let Self { surface, mut path } = self;
        if !path.is_finite() {
            surface.invalid("a path coordinate is not a finite number");
        }
        if let Paint::Stroke(style) | Paint::FillStroke(_, style) = &paint
            && let Some(problem) = style.problem()
        {
            surface.invalid(problem);
        }
        let segments = path.take();
        if !segments.is_empty() {
            surface.commands.push(Command::Paint(segments, paint));
        }
        surface
    }

    /// Strokes the path (`S`).
    pub fn stroke(self, style: &StrokeStyle) -> &'s mut Surface {
        self.paint(Paint::Stroke(style.clone()))
    }

    /// Fills the path (`f` or `f*`).
    pub fn fill(self, rule: FillRule) -> &'s mut Surface {
        self.paint(Paint::Fill(rule))
    }

    /// Fills, then strokes the path (`B` or `B*`).
    pub fn fill_stroke(self, rule: FillRule, style: &StrokeStyle) -> &'s mut Surface {
        self.paint(Paint::FillStroke(rule, style.clone()))
    }

    /// Intersects the clipping region with the path (`W n` or `W* n`) until the next
    /// [`Surface::restore`].
    pub fn clip(self, rule: FillRule) -> &'s mut Surface {
        self.paint(Paint::Clip(rule))
    }
}
