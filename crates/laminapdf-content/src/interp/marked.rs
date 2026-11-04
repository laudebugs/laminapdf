//! The marked-content operators (§14.6).

use laminapdf_cos::{Object, Result, Warning};

use super::{Frame, Interpreter, name, offset_u64};
use crate::lexer::Op;
use crate::visitor::{MarkedContent, Visitor};

/// What a marked-content operator does with its tag and properties.
#[derive(Debug, Clone, Copy)]
pub(super) enum Mark {
    /// `MP`, `DP`: a marked-content point.
    Point,
    /// `BMC`, `BDC`: the start of a sequence, ended by `EMC`.
    Begin,
}

impl<V: Visitor + ?Sized> Interpreter<'_, '_, V> {
    /// `MP`, `DP`, `BMC` and `BDC`: a property list named by a missing resource is left out,
    /// with a warning.
    pub(super) fn marked(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object], mark: Mark) -> Result<()> {
        let Some(tag) = args.first().and_then(name) else {
            return Ok(());
        };
        let found;
        let (properties, properties_name) = match args.get(1) {
            Some(Object::Dict(d)) => (Some(d), None),
            Some(Object::Name(n)) => {
                found = self.lookup(frame, "Properties", n)?;
                let dict = found.as_ref().and_then(|f| f.value.as_dict());
                if dict.is_none() {
                    self.missing(frame, op, "Properties")?;
                }
                (dict, Some(n))
            }
            _ => (None, None),
        };
        let marked = MarkedContent {
            tag,
            properties,
            properties_name,
            mcid: properties.and_then(|p| p.get_int("MCID")),
            depth: self.forms.len(),
        };
        match mark {
            Mark::Begin => {
                frame.marked += 1;
                self.visitor.begin_marked(&marked);
            }
            Mark::Point => self.visitor.marked_point(&marked),
        }
        Ok(())
    }

    /// `EMC`: an `EMC` with no sequence open is skipped, with a warning.
    pub(super) fn end_marked(&mut self, frame: &mut Frame, op: &Op<'_>) -> Result<()> {
        if frame.marked == 0 {
            return self.warn(Warning::ContentUnbalancedMarked {
                object: frame.object,
                offset: offset_u64(op.offset),
            });
        }
        frame.marked -= 1;
        self.visitor.end_marked();
        Ok(())
    }
}
