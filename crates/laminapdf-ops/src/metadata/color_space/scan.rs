//! The colours a form's content paints with, read by `laminapdf-content`'s interpreter: the fill
//! and stroke colours of painted paths and shown text (`g`, `rg`, `k`, `cs` and the rest), images
//! (an image XObject's `/ColorSpace`, an inline image's `/CS`, a stencil mask's fill colour),
//! shadings, the patterns a colour names (a shading pattern's space, a coloured tiling pattern's
//! own content) and each form's transparency group space. Only what is painted counts, so a
//! colour space a resource dictionary names but nothing uses never warns.

use std::collections::BTreeSet;
use std::sync::Arc;

use laminapdf_content::{
    ColorSpace, CurrentColor, FormXObject, ImageDraw, ImageSource, Paint, PathPaint, ShadingPaint, TextAdvance,
    TextShow, Visitor, interpret_content,
};
use laminapdf_cos::{ObjRef, Object, Stream};

use super::{DeviceSpaces, Rules, entry, resolve, space_fits};
use crate::error::{Error, Result};

/// How deep coloured tiling patterns are followed into patterns their own content paints with.
const MAX_PATTERN_DEPTH: u8 = 2;

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
        levels: Vec::new(),
        fits: true,
        error: None,
        patterns: BTreeSet::new(),
        depth: 0,
    };
    let outer_allowed = scan.allowed_in(outer.as_deref(), rules.intent)?;
    scan.levels.push((outer, outer_allowed));
    scan.run_form(stream)?;
    match scan.error {
        Some(e) => Err(e),
        None => Ok(scan.fits),
    }
}

/// The interpreter's visitor.
struct Scan<'d> {
    doc: &'d laminapdf_model::Document,
    rules: Rules,
    /// The resource dictionaries in effect, the current one last, each with the device spaces
    /// allowed in it.
    levels: Vec<(Option<Arc<Object>>, DeviceSpaces)>,
    fits: bool,
    /// The first error that must propagate; the scan stops there.
    error: Option<Error>,
    /// The tiling patterns already followed.
    patterns: BTreeSet<ObjRef>,
    depth: u8,
}

impl Scan<'_> {
    /// The current resource dictionary and the device spaces allowed in it.
    fn here(&self) -> (Option<Arc<Object>>, DeviceSpaces) {
        self.levels
            .last()
            .map_or((None, self.rules.intent), |(r, a)| (r.clone(), *a))
    }

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

    /// Enters a content stream drawn with `resources` (`None`: the current ones, as a form without
    /// its own inherits them).
    fn push(&mut self, resources: Option<Arc<Object>>) -> Result<()> {
        let (current, allowed) = self.here();
        let resources = resources.or(current);
        let allowed = self.allowed_in(resources.as_deref(), allowed)?;
        self.levels.push((resources, allowed));
        Ok(())
    }

    /// Runs the form or tiling pattern `stream` with its own resources.
    fn run_form(&mut self, stream: &Stream) -> Result<()> {
        let resources = match stream.dict.get("Resources") {
            Some(r) => resolve(self.doc, r)?,
            None => None,
        };
        self.push(resources.clone())?;
        self.group(&stream.dict)?;
        if let Some(content) = laminapdf_cos::tolerant(self.doc.cos().decode_stream(stream))? {
            let doc = self.doc;
            let resources = resources.or_else(|| self.here().0);
            if let Err(e) = interpret_content(doc, &content, resources, self)
                && e.must_propagate()
            {
                return Err(e.into());
            }
        }
        self.levels.pop();
        Ok(())
    }

    /// Checks a form's transparency group space.
    fn group(&mut self, dict: &laminapdf_cos::Dict) -> Result<()> {
        let group = match dict.get("Group") {
            Some(g) => resolve(self.doc, g)?,
            None => None,
        };
        if let Some(space) = entry(self.doc, group.as_deref(), "CS")? {
            self.space(&space)?;
        }
        Ok(())
    }

    /// Notes a colour space painted with.
    fn space(&mut self, space: &Object) -> Result<()> {
        let allowed = self.here().1;
        if !space_fits(self.doc, space, allowed, 0)? {
            self.fits = false;
        }
        Ok(())
    }

    /// Notes a current colour painted with: its space, and the pattern it names.
    fn color(&mut self, color: &CurrentColor) -> Result<()> {
        let family: &[u8] = match &color.space {
            ColorSpace::DeviceGray => b"DeviceGray",
            ColorSpace::DeviceRgb => b"DeviceRGB",
            ColorSpace::DeviceCmyk => b"DeviceCMYK",
            ColorSpace::Pattern => return self.pattern(color.pattern.as_ref()),
            ColorSpace::Named(name) => {
                if let Some(space) = self.resource("ColorSpace", name.as_bytes())? {
                    // A `Pattern` space's base, if any, is checked with it.
                    self.space(&space)?;
                    let family = match space.as_array().and_then(<[Object]>::first) {
                        Some(family) => resolve(self.doc, family)?,
                        None => None,
                    };
                    if matches!(family.as_deref(), Some(Object::Name(n)) if n.as_bytes() == b"Pattern") {
                        self.pattern(color.pattern.as_ref())?;
                    }
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        if !self.here().1.allows(family) {
            self.fits = false;
        }
        Ok(())
    }

    /// The resource `name` in `category` of the current resources, resolved.
    fn resource(&self, category: &str, name: &[u8]) -> Result<Option<Arc<Object>>> {
        let (current, _) = self.here();
        let Some(entries) = entry(self.doc, current.as_deref(), category)? else {
            return Ok(None);
        };
        match entries.as_dict().and_then(|d| by_name(d, name)) {
            Some(value) => resolve(self.doc, value),
            None => Ok(None),
        }
    }

    /// Notes the pattern `name`: a shading pattern's space, or a coloured tiling pattern's content,
    /// each tiling pattern once.
    fn pattern(&mut self, name: Option<&laminapdf_cos::Name>) -> Result<()> {
        let Some(name) = name else { return Ok(()) };
        let (current, _) = self.here();
        let Some(patterns) = entry(self.doc, current.as_deref(), "Pattern")? else {
            return Ok(());
        };
        let Some(reference) = patterns.as_dict().and_then(|d| by_name(d, name.as_bytes())).cloned() else {
            return Ok(());
        };
        let Some(pattern) = resolve(self.doc, &reference)? else {
            return Ok(());
        };
        if pattern.as_dict().and_then(|d| d.get_int("PatternType")) == Some(2) {
            let shading = entry(self.doc, Some(&pattern), "Shading")?;
            if let Some(space) = entry(self.doc, shading.as_deref(), "ColorSpace")? {
                self.space(&space)?;
            }
            return Ok(());
        }
        let Object::Stream(tiling) = &*pattern else {
            return Ok(());
        };
        // An uncoloured tiling pattern (PaintType 2) paints in its `Pattern` space's base.
        let coloured = tiling.dict.get_int("PaintType") != Some(2);
        let first = reference.as_ref().is_none_or(|r| self.patterns.insert(r));
        if coloured && first && self.depth < MAX_PATTERN_DEPTH {
            self.depth += 1;
            self.run_form(tiling)?;
            self.depth -= 1;
        }
        Ok(())
    }

    /// Runs `check` unless the scan has already found a fault or an error, keeping its error.
    fn guarded(&mut self, check: impl FnOnce(&mut Self) -> Result<()>) {
        if self.fits
            && self.error.is_none()
            && let Err(e) = check(self)
        {
            self.error = Some(e);
        }
    }
}

/// The entry of `dict` named by the bytes `name` (a resource name need not be UTF-8).
fn by_name<'d>(dict: &'d laminapdf_cos::Dict, name: &[u8]) -> Option<&'d Object> {
    dict.iter().find(|(k, _)| k.as_bytes() == name).map(|(_, v)| v)
}

impl Visitor for Scan<'_> {
    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        let (fill, stroke) = match paint.paint {
            Paint::Fill(_) => (true, false),
            Paint::Stroke { .. } => (false, true),
            Paint::FillStroke { .. } => (true, true),
            _ => return,
        };
        self.guarded(|s| {
            if fill {
                s.color(&paint.state.fill)?;
            }
            if stroke {
                s.color(&paint.state.stroke)?;
            }
            Ok(())
        });
    }

    fn show_text(&mut self, show: &TextShow<'_>) -> TextAdvance {
        // Render modes 0, 2, 4 and 6 fill; 1, 2, 5 and 6 stroke (§9.3.6, Table 104).
        let mode = show.state.text.render_mode;
        self.guarded(|s| {
            if matches!(mode, 0 | 2 | 4 | 6) {
                s.color(&show.state.fill)?;
            }
            if matches!(mode, 1 | 2 | 5 | 6) {
                s.color(&show.state.stroke)?;
            }
            Ok(())
        });
        TextAdvance::default()
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        self.guarded(|s| {
            let (dict, mask_key, space_key) = match image.source {
                ImageSource::XObject { stream, .. } => match stream.as_dict() {
                    Some(d) => (d, "ImageMask", "ColorSpace"),
                    None => return Ok(()),
                },
                ImageSource::Inline(inline) => (&inline.dict, "IM", "CS"),
                _ => return Ok(()),
            };
            let mask = dict.get(mask_key).or_else(|| dict.get("ImageMask"));
            if matches!(mask, Some(Object::Bool(true))) {
                return s.color(&image.state.fill);
            }
            // A JPEG 2000 image without the key paints in its own file's spaces, unread here.
            let Some(space) = dict.get(space_key).or_else(|| dict.get("ColorSpace")) else {
                return Ok(());
            };
            // An inline image may name a `/ColorSpace` resource of the stream that draws it.
            if let (ImageSource::Inline(_), Object::Name(n)) = (image.source, space)
                && !matches!(
                    n.as_bytes(),
                    b"G" | b"RGB" | b"CMYK" | b"DeviceGray" | b"DeviceRGB" | b"DeviceCMYK"
                )
            {
                return match entry(s.doc, image.resources, "ColorSpace")? {
                    Some(spaces) => match spaces.as_dict().and_then(|d| by_name(d, n.as_bytes())) {
                        Some(named) => {
                            let named = named.clone();
                            s.space(&named)
                        }
                        None => Ok(()),
                    },
                    None => Ok(()),
                };
            }
            s.space(space)
        });
    }

    fn paint_shading(&mut self, shading: &ShadingPaint<'_>) {
        self.guarded(|s| match entry(s.doc, Some(shading.shading), "ColorSpace")? {
            Some(space) => s.space(&space),
            None => Ok(()),
        });
    }

    fn enter_xobject(&mut self, form: &FormXObject<'_>) {
        let resources = match form.dict.get("Resources") {
            Some(r) => resolve(self.doc, r),
            None => Ok(None),
        };
        // The level is pushed even after a fault, so `leave_xobject` always has one to pop.
        match resources.and_then(|r| self.push(r)) {
            Ok(()) => self.guarded(|s| s.group(form.dict)),
            Err(e) => {
                let level = self.here();
                self.levels.push(level);
                self.error.get_or_insert(e);
            }
        }
    }

    fn leave_xobject(&mut self, _form: &FormXObject<'_>) {
        self.levels.pop();
    }
}
