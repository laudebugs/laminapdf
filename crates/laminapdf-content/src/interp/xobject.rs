//! The operators that paint images and forms: `Do` (§8.8) and inline images (§8.9.7).

use std::sync::Arc;

use laminapdf_cos::{Error, Limit, Name, ObjRef, Object, Position, Result, Stream, Warning};
use laminapdf_model::Matrix;

use super::{Frame, Interpreter, name, offset_u64};
use crate::lexer::Op;
use crate::resources;
use crate::visitor::{FormXObject, ImageDraw, ImageSource, Visitor};

impl<V: Visitor + ?Sized> Interpreter<'_, '_, V> {
    /// `BI … ID … EI`, read whole by the tokenizer.
    pub(super) fn inline_image(&mut self, frame: &Frame, op: &Op<'_>) {
        if let Some(image) = &op.inline_image {
            self.visitor.image(&ImageDraw {
                source: ImageSource::Inline(image),
                state: &frame.gs,
            });
        }
    }

    /// `ID` or `EI` outside an inline image: skipped, with a warning.
    pub(super) fn stray_inline_image_operator(&mut self, frame: &Frame, op: &Op<'_>) -> Result<()> {
        self.warn(Warning::ContentSyntax {
            object: frame.object,
            offset: offset_u64(op.offset),
        })
    }

    /// `Do` (§8.8): an image is reported, a form is run, a PostScript XObject is ignored. An
    /// XObject must be an indirect stream (§7.3.8); anything else is a missing resource.
    pub(super) fn paint_xobject(&mut self, frame: &Frame, op: &Op<'_>, args: &[Object]) -> Result<()> {
        let Some(n) = args.first().and_then(name) else {
            return Ok(());
        };
        let found = self.lookup(frame, "XObject", n)?;
        let Some((object, found)) = found.and_then(|f| Some((f.object?, f))) else {
            return self.missing(frame, op, "XObject");
        };
        let Object::Stream(stream) = &*found.value else {
            return self.missing(frame, op, "XObject");
        };
        match stream.dict.get_name("Subtype") {
            Some(b"Image") => {
                self.visitor.image(&ImageDraw {
                    source: ImageSource::XObject {
                        name: n,
                        object: Some(object),
                        stream: &found.value,
                    },
                    state: &frame.gs,
                });
                Ok(())
            }
            Some(b"Form") => self.form(frame, op, n, object, stream),
            // PostScript XObjects (§8.8.2) are ignored when rendering, and so here.
            Some(b"PS") => Ok(()),
            _ => self.missing(frame, op, "XObject"),
        }
    }

    /// Runs a Form XObject (§8.10.1): `q`, its `/Matrix` concatenated to the CTM, its content
    /// against its own resources (the parent's when it has none), `Q`.
    fn form(&mut self, frame: &Frame, op: &Op<'_>, n: &Name, object: ObjRef, stream: &Stream) -> Result<()> {
        if self.forms.contains(&object) {
            return self.warn(Warning::XObjectCycle { object: object.num });
        }
        if self.forms.len() >= self.limits.max_xobject_depth() {
            return Err(Error::LimitExceeded {
                limit: Limit::XObjectDepth,
                at: Some(Position::Object(object.num)),
            });
        }
        let content = if let Some(content) = self.decoded.get(&object) {
            Arc::clone(content)
        } else {
            let Some(content) = laminapdf_cos::tolerant(self.doc.decode_stream(stream))? else {
                return self.missing(frame, op, "XObject");
            };
            let content: Arc<[u8]> = content.into();
            self.decoded.insert(object, Arc::clone(&content));
            content
        };
        let dict = &stream.dict;
        let invalid = |key| Warning::InvalidEntry {
            object: Some(object.num),
            key,
        };
        let matrix = match dict.get("Matrix") {
            None => Matrix::IDENTITY,
            Some(m) => {
                if let Some(values) = resources::numbers(self.doc, m, 6)? {
                    Matrix::from_slice(&values).unwrap_or_default()
                } else {
                    self.warn(invalid("Matrix"))?;
                    Matrix::IDENTITY
                }
            }
        };
        let bbox = match dict.get("BBox") {
            Some(b) => resources::numbers(self.doc, b, 4)?,
            None => None,
        }
        .and_then(|b| match *b.as_slice() {
            [x0, y0, x1, y1] => Some(laminapdf_model::UserSpaceRect::from_corners(x0, y0, x1, y1)),
            _ => None,
        });
        if bbox.is_none() {
            // `/BBox` is required (§8.10.2, Table 93).
            self.warn(invalid("BBox"))?;
        }
        let own = match dict.get("Resources") {
            Some(r) => {
                let own = resources::resolve(self.doc, r)?.filter(|r| r.as_dict().is_some());
                if own.is_none() {
                    self.warn(invalid("Resources"))?;
                }
                own
            }
            None => None,
        };
        let resources = own.or_else(|| frame.resources.clone());
        let mut gs = frame.gs.clone();
        gs.ctm = matrix.then(&gs.ctm);
        let entry = gs.clone();
        let enter = FormXObject {
            name: n,
            object: Some(object),
            dict,
            matrix,
            bbox,
            depth: self.forms.len() + 1,
            state: &entry,
        };
        self.visitor.enter_xobject(&enter);
        self.forms.push(object);
        self.summary.forms += 1;
        let result = self.run(Frame::new(Some(object.num), resources, gs), &content);
        self.forms.pop();
        self.visitor.leave_xobject(&enter);
        result
    }
}
