//! The colour operators (§8.6.8) and `sh` (§8.7.4.2).

use laminapdf_cos::{Name, Object, Result};

use super::{Frame, Interpreter, name, number};
use crate::lexer::Op;
use crate::resources;
use crate::state::{ColorSpace, CurrentColor, GraphicsState};
use crate::visitor::{ShadingPaint, Visitor};

/// Which colour of the graphics state an operator sets.
#[derive(Debug, Clone, Copy)]
pub(super) enum Target {
    /// `CS`, `SC`, `SCN`.
    Stroke,
    /// `cs`, `sc`, `scn`.
    Fill,
}

impl Target {
    fn color(self, gs: &mut GraphicsState) -> &mut CurrentColor {
        match self {
            Self::Stroke => &mut gs.stroke,
            Self::Fill => &mut gs.fill,
        }
    }
}

/// The colour of `G`/`g`, `RG`/`rg` and `K`/`k`: a device space and its components.
pub(super) fn device_color(space: ColorSpace, args: &[Object]) -> CurrentColor {
    let mut color = CurrentColor::initial(space);
    color.components = args.iter().map(number).collect();
    color
}

impl<V: Visitor + ?Sized> Interpreter<'_, '_, V> {
    /// `CS` and `cs`.
    pub(super) fn set_color_space(
        &mut self,
        frame: &mut Frame,
        op: &Op<'_>,
        args: &[Object],
        target: Target,
    ) -> Result<()> {
        if let Some(n) = args.first().and_then(name) {
            *target.color(&mut frame.gs) = self.color_space(frame, op, n)?;
        }
        Ok(())
    }

    /// The colour `CS`/`cs` sets: the space and its initial colour (§8.6.8). A named space
    /// that is missing or not a colour space keeps its name, with no components.
    fn color_space(&mut self, frame: &Frame, op: &Op<'_>, n: &Name) -> Result<CurrentColor> {
        let space = match n.as_bytes() {
            b"DeviceGray" => ColorSpace::DeviceGray,
            b"DeviceRGB" => ColorSpace::DeviceRgb,
            b"DeviceCMYK" => ColorSpace::DeviceCmyk,
            b"Pattern" => ColorSpace::Pattern,
            _ => {
                let found = self.lookup(frame, "ColorSpace", n)?;
                let components = match found {
                    Some(found) => resources::initial_components(self.doc, &found.value)?,
                    None => None,
                };
                let mut color = CurrentColor::initial(ColorSpace::Named(n.clone()));
                match components {
                    Some(components) => color.components = components,
                    None => self.missing(frame, op, "ColorSpace")?,
                }
                return Ok(color);
            }
        };
        Ok(CurrentColor::initial(space))
    }

    /// `SC`, `SCN`, `sc` and `scn`: components, then a pattern name for `SCN`/`scn`. A pattern
    /// missing from the resources is kept by name, with a warning.
    pub(super) fn set_color(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object], target: Target) -> Result<()> {
        let (pattern, components) = match args.split_last() {
            Some((Object::Name(n), rest)) => (Some(n), rest),
            _ => (None, args),
        };
        if let Some(n) = pattern {
            let found = self.lookup(frame, "Pattern", n)?;
            if found.is_none_or(|f| f.value.as_dict().is_none()) {
                self.missing(frame, op, "Pattern")?;
            }
        }
        let color = target.color(&mut frame.gs);
        color.components = components.iter().map(number).collect();
        color.pattern = pattern.cloned();
        Ok(())
    }

    /// `sh`: a shading missing from the resources is skipped, with a warning.
    pub(super) fn shading(&mut self, frame: &Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        let Some(n) = args.first().and_then(name) else {
            return Ok(());
        };
        match self.lookup(frame, "Shading", n)? {
            Some(found) if found.value.as_dict().is_some() => {
                self.visitor.paint_shading(&ShadingPaint {
                    name: n,
                    object: found.object,
                    shading: &found.value,
                    state: &frame.gs,
                });
                Ok(())
            }
            _ => self.missing(frame, op, "Shading"),
        }
    }
}
