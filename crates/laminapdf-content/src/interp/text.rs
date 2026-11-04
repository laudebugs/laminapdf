//! The text operators: text objects (§9.4.1), `Tf` (§9.3), positioning (§9.4.2) and showing
//! (§9.4.3). The other text state operators (`Tc`, `Tw`, `Tz`, `TL`, `Tr` and `Ts`) are
//! one-line arms in [`Interpreter::execute`].

use laminapdf_cos::{Object, Result, Warning};

use super::{Frame, Interpreter, matrix, name, number, numbers, offset_u64};
use crate::lexer::Op;
use crate::state::{FontRef, Matrix};
use crate::visitor::{TextShow, Visitor};

/// Moves to the start of the next line, offset from the current one by (`tx`, `ty`).
fn translate_line(frame: &mut Frame, tx: f64, ty: f64) {
    frame.tlm = Matrix::translate(tx, ty).multiply(&frame.tlm);
    frame.tm = frame.tlm;
}

/// Moves to the start of the next line, one leading down.
fn advance_line(frame: &mut Frame) {
    let leading = frame.gs.text.leading;
    translate_line(frame, 0.0, -leading);
}

impl<V: Visitor + ?Sized> Interpreter<'_, '_, V> {
    /// A text positioning or showing operator outside `BT … ET` runs anyway, with a warning.
    fn require_text(&mut self, frame: &Frame, op: &Op<'_>) -> Result<()> {
        if frame.in_text {
            return Ok(());
        }
        self.unbalanced_bt(frame, op)
    }

    fn unbalanced_bt(&mut self, frame: &Frame, op: &Op<'_>) -> Result<()> {
        self.warn(Warning::ContentUnbalancedBt {
            object: frame.object,
            offset: offset_u64(op.offset),
        })
    }

    /// `BT`: a `BT` inside a text object starts a new one, with a warning.
    pub(super) fn begin_text(&mut self, frame: &mut Frame, op: &Op<'_>) -> Result<()> {
        if frame.in_text {
            self.unbalanced_bt(frame, op)?;
        }
        frame.in_text = true;
        frame.tm = Matrix::IDENTITY;
        frame.tlm = Matrix::IDENTITY;
        Ok(())
    }

    /// `ET`: an `ET` outside a text object is skipped, with a warning.
    pub(super) fn end_text(&mut self, frame: &mut Frame, op: &Op<'_>) -> Result<()> {
        if frame.in_text {
            frame.in_text = false;
            return Ok(());
        }
        self.unbalanced_bt(frame, op)
    }

    /// `Tf`: a font missing from the resources is kept by name, with a warning.
    pub(super) fn set_font(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        let [font, size] = args else {
            return Ok(());
        };
        let Some(n) = name(font) else {
            return Ok(());
        };
        let size = number(size);
        let found = self.lookup(frame, "Font", n)?;
        let (object, dict) = match found {
            Some(found) if found.value.as_dict().is_some() => (found.object, Some(found.value)),
            _ => {
                self.missing(frame, op, "Font")?;
                (None, None)
            }
        };
        frame.gs.text.font = Some(FontRef {
            name: Some(n.clone()),
            object,
            dict,
        });
        frame.gs.text.font_size = size;
        Ok(())
    }

    /// `Td`.
    pub(super) fn move_text(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        self.require_text(frame, op)?;
        let [tx, ty] = numbers::<2>(args);
        translate_line(frame, tx, ty);
        Ok(())
    }

    /// `TD`: as `Td`, also setting the leading to `-ty`.
    pub(super) fn move_text_set_leading(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        self.require_text(frame, op)?;
        let [tx, ty] = numbers::<2>(args);
        frame.gs.text.leading = -ty;
        translate_line(frame, tx, ty);
        Ok(())
    }

    /// `Tm`.
    pub(super) fn text_matrix(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        self.require_text(frame, op)?;
        frame.tm = matrix(args);
        frame.tlm = frame.tm;
        Ok(())
    }

    /// `T*`.
    pub(super) fn next_line(&mut self, frame: &mut Frame, op: &Op<'_>) -> Result<()> {
        self.require_text(frame, op)?;
        advance_line(frame);
        Ok(())
    }

    /// `Tj` and `TJ`.
    pub(super) fn show_text(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        self.require_text(frame, op)?;
        if let Some(operand) = args.first() {
            self.show(frame, operand);
        }
        Ok(())
    }

    /// `'`: `T*` then `Tj`.
    pub(super) fn next_line_show_text(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        self.require_text(frame, op)?;
        advance_line(frame);
        if let Some(operand) = args.first() {
            self.show(frame, operand);
        }
        Ok(())
    }

    /// `"`: word and character spacing, then `'`.
    pub(super) fn spacing_next_line_show_text(
        &mut self,
        frame: &mut Frame,
        op: &Op<'_>,
        args: &[Object],
    ) -> Result<()> {
        self.require_text(frame, op)?;
        if let [word, char, string] = args {
            frame.gs.text.word_spacing = number(word);
            frame.gs.text.char_spacing = number(char);
            advance_line(frame);
            self.show(frame, string);
        }
        Ok(())
    }

    /// Hands a string or `TJ` array to the visitor and moves the text matrix by the advance
    /// it returns.
    fn show(&mut self, frame: &mut Frame, operand: &Object) {
        let advance = self.visitor.show_text(&TextShow {
            operand,
            state: &frame.gs,
            tm: frame.tm,
            tlm: frame.tlm,
            depth: self.forms.len(),
        });
        frame.tm = Matrix::translate(advance.tx, advance.ty).multiply(&frame.tm);
    }
}
