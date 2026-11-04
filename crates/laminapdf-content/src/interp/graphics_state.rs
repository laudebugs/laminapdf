//! The graphics state operators (§8.4.4): `d`, `gs`, `q` and `Q`. The others (`w`, `J`, `j`,
//! `M`, `ri`, `i` and `cm`) are one-line arms in [`Interpreter::execute`].

use laminapdf_cos::{Error, Limit, Object, Result, Warning};
use laminapdf_model::Dash;

use super::{Frame, Interpreter, in_content, integer, name, number, offset_u64};
use crate::lexer::Op;
use crate::resources;
use crate::state::{FontRef, GraphicsState};
use crate::visitor::{GraphicsStateParams, Visitor};

fn dash(array: &[Object], phase: f64) -> Dash {
    Dash {
        array: array.iter().filter_map(Object::as_f64).collect(),
        phase,
    }
}

/// `d`: the dash array and phase.
pub(super) fn set_dash(gs: &mut GraphicsState, args: &[Object]) {
    if let [Object::Array(array), phase] = args {
        gs.dash = dash(array, number(phase));
    }
}

impl<V: Visitor + ?Sized> Interpreter<'_, '_, V> {
    /// `q`, bounded by [`laminapdf_cos::Limits::max_nesting_depth`].
    pub(super) fn save(&self, frame: &mut Frame, op: &Op<'_>) -> Result<()> {
        if frame.saved.len() >= self.limits.max_nesting_depth() {
            return Err(Error::LimitExceeded {
                limit: Limit::NestingDepth,
                at: in_content(frame.object, offset_u64(op.offset)),
            });
        }
        frame.saved.push(frame.gs.clone());
        Ok(())
    }

    /// `Q`: a `Q` with nothing saved is skipped, with a warning.
    pub(super) fn restore(&mut self, frame: &mut Frame, op: &Op<'_>) -> Result<()> {
        match frame.saved.pop() {
            Some(saved) => frame.gs = saved,
            None => self.warn(Warning::ContentUnbalancedQ {
                object: frame.object,
                offset: offset_u64(op.offset),
            })?,
        }
        Ok(())
    }

    /// `gs`: the `ExtGState` subset of ADR 0023 (line style, rendering intent, flatness,
    /// opacity, font); entries may be indirect.
    pub(super) fn ext_gstate(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        let Some(n) = args.first().and_then(name) else {
            return Ok(());
        };
        let found = self.lookup(frame, "ExtGState", n)?;
        let Some((object, dict)) = found.as_ref().and_then(|f| Some((f.object, f.value.as_dict()?))) else {
            return self.missing(frame, op, "ExtGState");
        };
        for (key, value) in dict.iter() {
            let Some(value) = resources::resolve(self.doc, value)? else {
                continue;
            };
            let gs = &mut frame.gs;
            match (key.as_bytes(), &*value) {
                (b"LW", v) if v.as_f64().is_some() => gs.line_width = number(v),
                (b"LC", v) if v.as_f64().is_some() => gs.line_cap = integer(v),
                (b"LJ", v) if v.as_f64().is_some() => gs.line_join = integer(v),
                (b"ML", v) if v.as_f64().is_some() => gs.miter_limit = number(v),
                (b"FL", v) if v.as_f64().is_some() => gs.flatness = number(v),
                (b"CA", v) if v.as_f64().is_some() => gs.stroke_alpha = number(v),
                (b"ca", v) if v.as_f64().is_some() => gs.fill_alpha = number(v),
                (b"RI", Object::Name(intent)) => gs.rendering_intent = Some(intent.clone()),
                (b"D", Object::Array(d)) => set_dash(gs, d.as_slice()),
                (b"Font", Object::Array(font)) => {
                    if let [font, size] = font.as_slice() {
                        let size = resources::number(self.doc, size)?.unwrap_or_default();
                        let dict = resources::resolve(self.doc, font)?.filter(|f| f.as_dict().is_some());
                        if dict.is_none() {
                            self.missing(frame, op, "Font")?;
                        }
                        frame.gs.text.font = Some(FontRef {
                            name: None,
                            object: font.as_ref(),
                            dict,
                        });
                        frame.gs.text.font_size = size;
                    }
                }
                _ => {}
            }
        }
        self.visitor.set_graphics_state(&GraphicsStateParams {
            name: n,
            object,
            dict,
            state: &frame.gs,
        });
        Ok(())
    }
}
