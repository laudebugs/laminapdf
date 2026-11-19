//! Whether what the edits added paints in colour spaces a PDF/A file's output intent allows
//! (ISO 19005-1 §6.2.3.3, ISO 19005-2 §6.2.4.3): `DeviceRGB` only with an RGB intent,
//! `DeviceCMYK` only with a CMYK one, `DeviceGray` with any, and every device space only with an
//! intent or a `Default` space that replaces it. A device-independent space (`ICCBased`, `CalRGB`,
//! `CalGray`, `Lab`) is always allowed. Only resources are read, never content streams: an image
//! XObject's `/ColorSpace`, a form's named colour spaces, its shadings' and its transparency
//! group's, and the forms it draws, as deep as [`MAX_FORM_DEPTH`].

use laminapdf_cos::{Dict, Object};

use crate::added::MAX_FORM_DEPTH;
use crate::error::Result;

/// The device spaces a content stream may paint in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeviceSpaces {
    gray: bool,
    rgb: bool,
    cmyk: bool,
}

impl DeviceSpaces {
    /// What the output intent allows: its profile's component count (`/N`), or `None` for a
    /// file without a `GTS_PDFA1` intent (or one whose profile can't be read), which allows no
    /// device space.
    pub(crate) const fn of_intent(components: Option<i64>) -> Self {
        let any = components.is_some();
        Self {
            gray: any,
            rgb: matches!(components, Some(3)),
            cmyk: matches!(components, Some(4)),
        }
    }

    /// These, plus those `resources`' `/ColorSpace` replaces with a `Default` space that is
    /// itself allowed (ISO 32000-2 §8.6.5.6).
    fn with_defaults(self, doc: &laminapdf_model::Document, resources: Option<&Dict>) -> Result<Self> {
        let Some(spaces) = resolve(doc, resources.and_then(|r| r.get("ColorSpace")))? else {
            return Ok(self);
        };
        let replaced = |key: &str| -> Result<bool> {
            Ok(match spaces.get(key) {
                Some(space) => space_fits(doc, space, self, 0)?,
                None => false,
            })
        };
        Ok(Self {
            gray: self.gray || replaced("DefaultGray")?,
            rgb: self.rgb || replaced("DefaultRGB")?,
            cmyk: self.cmyk || replaced("DefaultCMYK")?,
        })
    }

    fn allows(self, family: &[u8]) -> bool {
        match family {
            b"DeviceGray" | b"G" => self.gray,
            b"DeviceRGB" | b"RGB" => self.rgb,
            b"DeviceCMYK" | b"CMYK" => self.cmyk,
            // Pattern alone, and anything this version doesn't know, aren't device spaces.
            _ => true,
        }
    }
}

/// The first `GTS_PDFA1` output intent's profile's component count (`/N`), or `None` when the
/// catalog has no such intent or its profile can't be read.
///
/// # Errors
/// Reading errors (the catalog, the intents).
pub(crate) fn intent_components(doc: &laminapdf_model::Document) -> Result<Option<i64>> {
    let catalog = doc.catalog()?;
    let Some(intents) = catalog.get("OutputIntents") else {
        return Ok(None);
    };
    let intents = doc
        .deref(intents)?
        .as_array()
        .map(<[Object]>::to_vec)
        .unwrap_or_default();
    for intent in intents {
        let intent = doc.deref(&intent)?;
        let Some(intent) = intent.as_dict() else { continue };
        if intent.get_name("S") != Some(b"GTS_PDFA1".as_slice()) {
            continue;
        }
        let Some(profile) = intent.get("DestOutputProfile").and_then(Object::as_ref) else {
            return Ok(None);
        };
        return Ok(
            laminapdf_cos::tolerant(doc.cos().resolve(profile))?.and_then(|o| o.as_dict().and_then(|d| d.get_int("N")))
        );
    }
    Ok(None)
}

/// Whether the XObject `xobject` paints only in spaces allowed: an image in `around`, the spaces
/// of the content stream that draws it, a form in `intent` and the `Default` spaces of its own
/// resources, its own forms likewise down to [`MAX_FORM_DEPTH`].
///
/// # Errors
/// Reading errors.
pub(crate) fn fits(
    doc: &laminapdf_model::Document,
    xobject: &Object,
    intent: DeviceSpaces,
    around: DeviceSpaces,
    depth: u8,
) -> Result<bool> {
    let Object::Stream(s) = &*doc.deref(xobject)? else {
        return Ok(true);
    };
    if s.dict.get_name("Subtype") == Some(b"Image".as_slice()) {
        // A stencil mask paints in the fill colour, and a JPEG 2000 image without the key in
        // its own file's spaces, which this check doesn't read.
        if matches!(s.dict.get("ImageMask"), Some(Object::Bool(true))) {
            return Ok(true);
        }
        return match s.dict.get("ColorSpace") {
            Some(space) => space_fits(doc, space, around, 0),
            None => Ok(true),
        };
    }
    let resources = resolve(doc, s.dict.get("Resources"))?;
    let here = intent.with_defaults(doc, resources.as_ref())?;
    if let Some(group) = resolve(doc, s.dict.get("Group"))?
        && let Some(space) = group.get("CS")
        && !space_fits(doc, space, here, 0)?
    {
        return Ok(false);
    }
    let Some(resources) = resources else {
        return Ok(true);
    };
    if let Some(spaces) = resolve(doc, resources.get("ColorSpace"))? {
        for (_, space) in spaces.iter() {
            if !space_fits(doc, space, here, 0)? {
                return Ok(false);
            }
        }
    }
    if let Some(shadings) = resolve(doc, resources.get("Shading"))? {
        for (_, shading) in shadings.iter() {
            let space = resolve(doc, Some(shading))?.and_then(|d| d.get("ColorSpace").cloned());
            if let Some(space) = space
                && !space_fits(doc, &space, here, 0)?
            {
                return Ok(false);
            }
        }
    }
    if depth >= MAX_FORM_DEPTH {
        return Ok(true);
    }
    if let Some(xobjects) = resolve(doc, resources.get("XObject"))? {
        for (_, x) in xobjects.iter() {
            if !fits(doc, x, intent, here, depth + 1)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// How deep [`space_fits`] follows a space's base or alternate: `[/Indexed [/Separation …]]` is
/// two.
const MAX_SPACE_DEPTH: u8 = 3;

/// Whether the colour space `space` is allowed: a device space `allowed` lists, a
/// device-independent one, or an `Indexed`, `Pattern`, `Separation` or `DeviceN` space whose base
/// or alternate is (ISO 19005-2 §6.2.4.4).
fn space_fits(doc: &laminapdf_model::Document, space: &Object, allowed: DeviceSpaces, depth: u8) -> Result<bool> {
    let space = doc.deref(space)?;
    let array = match &*space {
        Object::Name(name) => return Ok(allowed.allows(name.as_bytes())),
        Object::Array(a) => a,
        _ => return Ok(true),
    };
    let Some(Object::Name(family)) = array.first() else {
        return Ok(true);
    };
    let base = match family.as_bytes() {
        b"Indexed" | b"I" | b"Pattern" => array.get(1),
        b"Separation" | b"DeviceN" => array.get(2),
        other => return Ok(allowed.allows(other)),
    };
    match base {
        Some(base) if depth < MAX_SPACE_DEPTH => space_fits(doc, base, allowed, depth + 1),
        _ => Ok(true),
    }
}

/// `value` resolved to a dictionary (a stream's too), or `None`.
fn resolve(doc: &laminapdf_model::Document, value: Option<&Object>) -> Result<Option<Dict>> {
    match value {
        Some(v) => Ok(doc.deref(v)?.as_dict().cloned()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::Name;

    use super::*;

    fn name(s: &str) -> Object {
        Object::Name(Name::from(s))
    }

    fn array(items: Vec<Object>) -> Object {
        Object::Array(items)
    }

    fn empty_doc() -> laminapdf_model::Document {
        let bytes = laminapdf_test_fixtures::classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>",
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            ],
            "<< /Size 4 /Root 1 0 R >>",
        );
        laminapdf_model::Document::open(bytes, laminapdf_model::OpenOptions::default()).unwrap()
    }

    fn sweep(space: &Object) -> [bool; 3] {
        let doc = empty_doc();
        [None, Some(3), Some(4)].map(|n| space_fits(&doc, space, DeviceSpaces::of_intent(n), 0).unwrap())
    }

    #[test]
    fn device_spaces_need_an_intent_of_their_own_but_gray_takes_any() {
        let got = ["DeviceGray", "DeviceRGB", "DeviceCMYK"].map(|s| sweep(&name(s)));
        assert_eq!(got, [[false, true, true], [false, true, false], [false, false, true]]);
    }

    #[test]
    fn an_icc_based_space_fits_any_intent() {
        let icc = array(vec![name("ICCBased"), Object::Ref(laminapdf_cos::ObjRef::new(9, 0))]);
        assert_eq!(sweep(&icc), [true, true, true]);
    }

    #[test]
    fn an_indexed_or_separation_space_is_judged_by_its_base() {
        let indexed = array(vec![
            name("Indexed"),
            name("DeviceCMYK"),
            Object::Integer(1),
            Object::Null,
        ]);
        let separation = array(vec![name("Separation"), name("Spot"), name("DeviceRGB"), Object::Null]);
        assert_eq!(
            [sweep(&indexed), sweep(&separation)],
            [[false, false, true], [false, true, false]]
        );
    }
}
