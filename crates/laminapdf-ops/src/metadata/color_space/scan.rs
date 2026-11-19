//! The colours a form's content paints with, read by `laminapdf-content`'s tokenizer through a
//! small state machine rather than its interpreter, which this check would link into every module
//! for what a few operators tell: the fill and stroke colours (`g`, `rg`, `k`, `cs` and their
//! stroking twins, `scn`'s pattern) and the text render mode (`Tr`), saved and restored by `q` and
//! `Q`. Painted paths and shown text are judged by them; so are images (an image XObject's
//! `/ColorSpace`, an inline image's `/CS`, a stencil mask's fill colour), shadings (`sh`), the
//! patterns a colour names (a shading pattern's space, a coloured tiling pattern's own content),
//! the forms drawn (`Do`) and each form's transparency group space. Only what is painted counts, so
//! a colour space a resource dictionary names but nothing uses never warns.

use std::sync::Arc;

use laminapdf_content::{ContentIter, InlineImage, Lexed, Op, Operator as O};
use laminapdf_cos::{Dict, ObjRef, Object, Stream, tolerant};

use super::{DeviceSpaces, Rules, entry, resolve, space_fits};
use crate::added::MAX_FORM_DEPTH;
use crate::error::Result;

/// The most `q` levels kept; deeper ones share the last (a real page nests a few).
const MAX_SAVED: usize = 64;

/// Whether the form `form` paints only in colour spaces `rules` allow, drawn inside the resource
/// dictionary `outer` (the page's, for an object placed on it), whose `Default` spaces apply
/// where the form has none of its own. Content that can't be read to its end is judged as far as
/// it was read.
///
/// # Errors
/// Reading errors that must propagate.
pub(crate) fn fits(
    doc: &laminapdf_model::Document,
    form: &Object,
    rules: Rules,
    outer: Option<Arc<Object>>,
) -> Result<bool> {
    let Some(form) = resolve(doc, form)? else {
        return Ok(true);
    };
    let Object::Stream(stream) = &*form else {
        return Ok(true);
    };
    let mut scan = Scan {
        doc,
        rules,
        fits: true,
        seen: Vec::new(),
    };
    let allowed = scan.allowed_in(outer.as_deref(), rules.intent)?;
    scan.run(stream, outer, allowed, State::initial(), 0)?;
    Ok(scan.fits)
}

/// A colour's space as the content set it.
#[derive(Debug, Clone, PartialEq)]
enum Space {
    /// `DeviceGray`, `DeviceRGB` or `DeviceCMYK`, by name.
    Device(&'static [u8]),
    /// `Pattern` with no base.
    Pattern,
    /// A `/ColorSpace` resource, resolved when `cs` or `CS` selected it: a form it draws
    /// inherits the space, not the name, which the form's own resources may not hold.
    Named(Arc<Object>),
    /// A `/ColorSpace` resource that is missing or can't be read: not judged.
    Missing,
}

/// A pattern `scn` named, resolved then for the same reason as [`Space::Named`], with its
/// reference.
type Pattern = (Option<ObjRef>, Arc<Object>);

/// A current colour: its space and the pattern `scn` named.
#[derive(Debug, Clone, PartialEq)]
struct Color {
    space: Space,
    pattern: Option<Pattern>,
}

impl Color {
    const fn device(family: &'static [u8]) -> Self {
        Self {
            space: Space::Device(family),
            pattern: None,
        }
    }
}

/// What `q` saves and `Q` restores, as far as colours go, and what a form drawn by `Do`
/// inherits (ISO 32000-2 §8.10.1).
#[derive(Debug, Clone, PartialEq)]
struct State {
    fill: Color,
    stroke: Color,
    render_mode: i64,
}

impl State {
    /// The state a content stream starts with (§8.4.1, Table 51): black in `DeviceGray`, text
    /// filled.
    const fn initial() -> Self {
        Self {
            fill: Color::device(b"DeviceGray"),
            stroke: Color::device(b"DeviceGray"),
            render_mode: 0,
        }
    }
}

/// A content stream being read: its resources and the device spaces allowed in it.
struct Frame {
    resources: Option<Arc<Object>>,
    allowed: DeviceSpaces,
}

struct Scan<'d> {
    doc: &'d laminapdf_model::Document,
    rules: Rules,
    fits: bool,
    /// The forms and tiling patterns read already, with the spaces allowed and the state they
    /// started with: a list, as a page draws a few (a set would cost the module more than the
    /// lookups do). A form without resources of its own isn't listed, as what it reads depends
    /// on the stream that draws it.
    seen: Vec<(ObjRef, DeviceSpaces, State)>,
}

impl Scan<'_> {
    /// `inherited`, plus the device spaces `resources`' allowed `Default` spaces replace
    /// (ISO 32000-2 §8.6.5.6), on PDF/A-2 and later.
    fn allowed_in(&self, resources: Option<&Object>, inherited: DeviceSpaces) -> Result<DeviceSpaces> {
        if !self.rules.defaults {
            return Ok(inherited);
        }
        let spaces = entry(self.doc, resources, "ColorSpace")?;
        let replaced = |key: &str| -> Result<bool> {
            Ok(
                match spaces.as_deref().and_then(Object::as_dict).and_then(|d| d.get(key)) {
                    Some(space) => space_fits(self.doc, space, self.rules.intent, 0)?,
                    None => false,
                },
            )
        };
        Ok(DeviceSpaces {
            gray: inherited.gray || replaced("DefaultGray")?,
            rgb: inherited.rgb || replaced("DefaultRGB")?,
            cmyk: inherited.cmyk || replaced("DefaultCMYK")?,
        })
    }

    /// Reads the form or tiling pattern `stream`, drawn inside `outer` where `allowed` holds:
    /// with its own resources, or `outer`'s when it has none.
    fn run(
        &mut self,
        stream: &Stream,
        outer: Option<Arc<Object>>,
        allowed: DeviceSpaces,
        mut state: State,
        depth: u8,
    ) -> Result<()> {
        let resources = match stream.dict.get("Resources") {
            Some(r) => resolve(self.doc, r)?,
            None => None,
        }
        .or(outer);
        let frame = Frame {
            allowed: self.allowed_in(resources.as_deref(), allowed)?,
            resources,
        };
        let group = match stream.dict.get("Group") {
            Some(g) => resolve(self.doc, g)?,
            None => None,
        };
        if let Some(space) = entry(self.doc, group.as_deref(), "CS")? {
            self.space(&frame, &space)?;
        }
        let Some(content) = tolerant(self.doc.cos().decode_stream(stream))? else {
            return Ok(());
        };

        let mut saved: Vec<State> = Vec::new();
        for item in ContentIter::new(&content) {
            if !self.fits {
                break;
            }
            let op = match item {
                Ok(Lexed::Op(op)) => op,
                Ok(_) => continue,
                Err(e) if e.must_propagate() => return Err(e.into()),
                // The rest can't be read: judge what was.
                Err(_) => break,
            };
            // Matched by variant, not keyword: a jump table is far smaller than byte compares.
            match op.operator {
                O::Save => {
                    if saved.len() < MAX_SAVED {
                        saved.push(state.clone());
                    }
                }
                O::Restore => {
                    if let Some(s) = saved.pop() {
                        state = s;
                    }
                }
                _ => self.operator(&frame, &mut state, &op, depth)?,
            }
        }
        Ok(())
    }

    /// One operator other than `q` and `Q`.
    fn operator(&mut self, frame: &Frame, state: &mut State, op: &Op<'_>, depth: u8) -> Result<()> {
        let name = || match op.operands.first() {
            Some(Object::Name(n)) => Some(n.as_bytes().to_vec()),
            _ => None,
        };
        match op.operator {
            O::FillGray => state.fill = Color::device(b"DeviceGray"),
            O::FillRgb => state.fill = Color::device(b"DeviceRGB"),
            O::FillCmyk => state.fill = Color::device(b"DeviceCMYK"),
            O::StrokeGray => state.stroke = Color::device(b"DeviceGray"),
            O::StrokeRgb => state.stroke = Color::device(b"DeviceRGB"),
            O::StrokeCmyk => state.stroke = Color::device(b"DeviceCMYK"),
            O::FillColorSpace | O::StrokeColorSpace => {
                if let Some(n) = name() {
                    let space = match n.as_slice() {
                        b"DeviceGray" => Space::Device(b"DeviceGray"),
                        b"DeviceRGB" => Space::Device(b"DeviceRGB"),
                        b"DeviceCMYK" => Space::Device(b"DeviceCMYK"),
                        b"Pattern" => Space::Pattern,
                        _ => self
                            .resource(frame, "ColorSpace", &n)?
                            .map_or(Space::Missing, Space::Named),
                    };
                    let color = Color { space, pattern: None };
                    *if op.operator == O::FillColorSpace {
                        &mut state.fill
                    } else {
                        &mut state.stroke
                    } = color;
                }
            }
            O::FillColorN | O::StrokeColorN => {
                if let Some(Object::Name(n)) = op.operands.last() {
                    let pattern = self.resource_ref(frame, "Pattern", n.as_bytes())?;
                    let color = if op.operator == O::FillColorN {
                        &mut state.fill
                    } else {
                        &mut state.stroke
                    };
                    color.pattern = pattern;
                }
            }
            O::RenderMode => state.render_mode = op.operands.first().and_then(Object::as_int).unwrap_or(0),
            // Paths: fill, stroke, or both (§8.5.3).
            O::Fill | O::FillObsolete | O::FillEvenOdd => self.color(frame, &state.fill, depth)?,
            O::Stroke | O::CloseStroke => self.color(frame, &state.stroke, depth)?,
            O::FillStroke | O::FillStrokeEvenOdd | O::CloseFillStroke | O::CloseFillStrokeEvenOdd => {
                self.color(frame, &state.fill, depth)?;
                self.color(frame, &state.stroke, depth)?;
            }
            // Text: render modes 0, 2, 4 and 6 fill; 1, 2, 5 and 6 stroke (§9.3.6, Table 104).
            O::ShowText | O::ShowTextAdjusted | O::NextLineShowText | O::SpacingNextLineShowText => {
                if matches!(state.render_mode, 0 | 2 | 4 | 6) {
                    self.color(frame, &state.fill, depth)?;
                }
                if matches!(state.render_mode, 1 | 2 | 5 | 6) {
                    self.color(frame, &state.stroke, depth)?;
                }
            }
            O::Shading => {
                if let Some(n) = name() {
                    let shading = self.resource(frame, "Shading", &n)?;
                    if let Some(space) = entry(self.doc, shading.as_deref(), "ColorSpace")? {
                        self.space(frame, &space)?;
                    }
                }
            }
            O::PaintXObject => {
                if let Some(n) = name() {
                    self.xobject(frame, state, &n, depth)?;
                }
            }
            O::BeginInlineImage => {
                if let Some(image) = &op.inline_image {
                    self.inline_image(frame, state, image, depth)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// `Do`: an image's space (a stencil mask's fill colour), or a form's content.
    fn xobject(&mut self, frame: &Frame, state: &State, name: &[u8], depth: u8) -> Result<()> {
        let Some((reference, xobject)) = self.resource_ref(frame, "XObject", name)? else {
            return Ok(());
        };
        let Object::Stream(stream) = &*xobject else {
            return Ok(());
        };
        if stream.dict.get_name("Subtype") == Some(b"Image".as_slice()) {
            if matches!(stream.dict.get("ImageMask"), Some(Object::Bool(true))) {
                return self.color(frame, &state.fill, depth);
            }
            // A JPEG 2000 image without the key paints in its own file's spaces, unread here.
            return match stream.dict.get("ColorSpace") {
                Some(space) => self.space(frame, space),
                None => Ok(()),
            };
        }
        // A form inherits the graphics state of the stream that draws it (ISO 32000-2 §8.10.1);
        // one without resources of its own reads that stream's, so it is read every time.
        let own = stream.dict.get("Resources").is_some();
        if depth < MAX_FORM_DEPTH && (!own || self.first_time(reference, frame.allowed, state)) {
            self.run(stream, frame.resources.clone(), frame.allowed, state.clone(), depth + 1)?;
        }
        Ok(())
    }

    /// An inline image's space, which may name a `/ColorSpace` resource, or a stencil mask's fill
    /// colour.
    fn inline_image(&mut self, frame: &Frame, state: &State, image: &InlineImage<'_>, depth: u8) -> Result<()> {
        let get = |short: &str, long: &str| image.dict.get(short).or_else(|| image.dict.get(long));
        if matches!(get("IM", "ImageMask"), Some(Object::Bool(true))) {
            return self.color(frame, &state.fill, depth);
        }
        match get("CS", "ColorSpace") {
            Some(Object::Name(n)) if !is_device(n.as_bytes()) => {
                match self.resource(frame, "ColorSpace", n.as_bytes())? {
                    Some(space) => self.space(frame, &space),
                    None => Ok(()),
                }
            }
            Some(space) => self.space(frame, &space.clone()),
            None => Ok(()),
        }
    }

    /// Notes a colour painted with: its space, and the pattern it names.
    fn color(&mut self, frame: &Frame, color: &Color, depth: u8) -> Result<()> {
        match &color.space {
            Space::Device(family) => {
                if !frame.allowed.allows(family) {
                    self.fits = false;
                }
            }
            Space::Pattern => self.pattern(frame, color.pattern.as_ref(), depth)?,
            Space::Named(space) => {
                // A `Pattern` space's base, if any, is checked with it.
                self.space(frame, space)?;
                let family = match space.as_array().and_then(<[Object]>::first) {
                    Some(family) => resolve(self.doc, family)?,
                    None => None,
                };
                if matches!(family.as_deref(), Some(Object::Name(n)) if n.as_bytes() == b"Pattern") {
                    self.pattern(frame, color.pattern.as_ref(), depth)?;
                }
            }
            Space::Missing => {}
        }
        Ok(())
    }

    /// Notes the pattern `name`: a shading pattern's space, or a coloured tiling pattern's content.
    fn pattern(&mut self, frame: &Frame, pattern: Option<&Pattern>, depth: u8) -> Result<()> {
        let Some((reference, pattern)) = pattern.cloned() else {
            return Ok(());
        };
        if pattern.as_dict().and_then(|d| d.get_int("PatternType")) == Some(2) {
            let shading = entry(self.doc, Some(&pattern), "Shading")?;
            if let Some(space) = entry(self.doc, shading.as_deref(), "ColorSpace")? {
                self.space(frame, &space)?;
            }
            return Ok(());
        }
        let Object::Stream(tiling) = &*pattern else {
            return Ok(());
        };
        // An uncoloured tiling pattern (PaintType 2) paints in its `Pattern` space's base. A
        // pattern's content uses its own resources; the page's `Default` spaces still count.
        let coloured = tiling.dict.get_int("PaintType") != Some(2);
        // A pattern's content starts from the initial graphics state, not the painter's
        // (§8.7.3.1).
        let initial = State::initial();
        if coloured && depth < MAX_FORM_DEPTH && self.first_time(reference, frame.allowed, &initial) {
            self.run(tiling, None, frame.allowed, initial, depth + 1)?;
        }
        Ok(())
    }

    /// Notes a colour space painted with.
    fn space(&mut self, frame: &Frame, space: &Object) -> Result<()> {
        if !space_fits(self.doc, space, frame.allowed, 0)? {
            self.fits = false;
        }
        Ok(())
    }

    /// Whether the form or pattern `reference` is read for the first time where `allowed` holds,
    /// starting from `state`; a direct one always is.
    fn first_time(&mut self, reference: Option<ObjRef>, allowed: DeviceSpaces, state: &State) -> bool {
        let Some(r) = reference else { return true };
        if self
            .seen
            .iter()
            .any(|(seen, a, s)| *seen == r && *a == allowed && s == state)
        {
            return false;
        }
        self.seen.push((r, allowed, state.clone()));
        true
    }

    /// The resource `name` in `category` of `frame`'s resources, resolved.
    fn resource(&self, frame: &Frame, category: &str, name: &[u8]) -> Result<Option<Arc<Object>>> {
        Ok(self.resource_ref(frame, category, name)?.map(|(_, v)| v))
    }

    /// As [`Scan::resource`], with the entry's reference when it is one.
    fn resource_ref(
        &self,
        frame: &Frame,
        category: &str,
        name: &[u8],
    ) -> Result<Option<(Option<ObjRef>, Arc<Object>)>> {
        let Some(entries) = entry(self.doc, frame.resources.as_deref(), category)? else {
            return Ok(None);
        };
        let Some(value) = entries.as_dict().and_then(|d| by_name(d, name)) else {
            return Ok(None);
        };
        Ok(resolve(self.doc, value)?.map(|v| (value.as_ref(), v)))
    }
}

/// Whether `name` is a device family, in full or as an inline image abbreviates it (§8.9.7).
fn is_device(name: &[u8]) -> bool {
    matches!(
        name,
        b"G" | b"RGB" | b"CMYK" | b"DeviceGray" | b"DeviceRGB" | b"DeviceCMYK"
    )
}

/// The entry of `dict` named by the bytes `name` (a resource name need not be UTF-8).
fn by_name<'d>(dict: &'d Dict, name: &[u8]) -> Option<&'d Object> {
    dict.iter().find(|(k, _)| k.as_bytes() == name).map(|(_, v)| v)
}
