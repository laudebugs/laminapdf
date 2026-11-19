//! Whether what the edits added paints in colour spaces a PDF/A file's output intent allows
//! (ISO 19005-2 §6.2.4.3, ISO 19005-1 §6.2.3.3): `DeviceRGB` only with an RGB intent,
//! `DeviceCMYK` only with a CMYK one, `DeviceGray` with any, and no device space in a file
//! without an intent. On PDF/A-2 and later a `Default` space in the resources replaces the
//! device space it names (ISO 19005-2 §6.2.4.3, ISO 32000-2 §8.6.5.6); PDF/A-1's clause names no
//! such exception, so there it doesn't. A device-independent space (`ICCBased`, `CalRGB`,
//! `CalGray`, `Lab`) is always allowed, and `Indexed`, `Pattern`, `Separation` and `DeviceN`
//! spaces are judged by their base or alternate (§6.2.4.4).
//!
//! [`fits`] runs the content of a form the edits added through `laminapdf-content`'s
//! interpreter and judges the colours it paints with (`scan`).

mod scan;

use std::sync::Arc;

use laminapdf_cos::{Object, tolerant};

use crate::error::Result;
pub(crate) use scan::fits;

/// The space of a file's PDF/A output intent, as far as the colour rules need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    /// The catalog has no `GTS_PDFA1` intent: no device space is allowed.
    Missing,
    /// A grey profile, or one of another kind (such as Lab): only `DeviceGray` is allowed.
    Gray,
    /// An RGB profile.
    Rgb,
    /// A CMYK profile.
    Cmyk,
    /// The profile's kind can't be read: nothing is judged.
    Unknown,
}

/// The first `GTS_PDFA1` output intent's kind: the colour space field of its profile's header
/// (bytes 16–19, ICC.1:2022 §7.2.6), else its `/N`.
///
/// # Errors
/// Reading errors (the catalog, the intents) that must propagate.
pub(crate) fn intent(doc: &laminapdf_model::Document) -> Result<Intent> {
    let catalog = doc.catalog()?;
    let Some(intents) = catalog.get("OutputIntents") else {
        return Ok(Intent::Missing);
    };
    let Some(intents) = resolve(doc, intents)? else {
        return Ok(Intent::Missing);
    };
    for intent in intents.as_array().unwrap_or_default() {
        let Some(intent) = resolve(doc, intent)? else { continue };
        let Some(intent) = intent.as_dict() else { continue };
        if intent.get_name("S") != Some(b"GTS_PDFA1".as_slice()) {
            continue;
        }
        let Some(profile) = intent.get("DestOutputProfile") else {
            return Ok(Intent::Unknown);
        };
        let Some(profile) = resolve(doc, profile)? else {
            return Ok(Intent::Unknown);
        };
        let Object::Stream(profile) = &*profile else {
            return Ok(Intent::Unknown);
        };
        let header = tolerant(doc.cos().decode_stream_prefix(profile, 20))?;
        let field = header.as_ref().and_then(|(bytes, _)| bytes.get(16..20));
        return Ok(match (field, profile.dict.get_int("N")) {
            (Some(b"RGB "), _) | (None, Some(3)) => Intent::Rgb,
            (Some(b"CMYK"), _) | (None, Some(4)) => Intent::Cmyk,
            (Some(_), _) | (None, Some(1)) => Intent::Gray,
            (None, _) => Intent::Unknown,
        });
    }
    Ok(Intent::Missing)
}

/// What a file's claim and intent allow: the device spaces, and whether a `Default` space may
/// stand in for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rules {
    intent: DeviceSpaces,
    defaults: bool,
}

impl Rules {
    /// The rules of a file claiming PDF/A-`part` with `intent`; `None` when the intent can't be
    /// read, so nothing is judged.
    pub(crate) const fn of(intent: Intent, part: u8) -> Option<Self> {
        let (gray, rgb, cmyk) = match intent {
            Intent::Unknown => return None,
            Intent::Missing => (false, false, false),
            Intent::Gray => (true, false, false),
            Intent::Rgb => (true, true, false),
            Intent::Cmyk => (true, false, true),
        };
        Some(Self {
            intent: DeviceSpaces { gray, rgb, cmyk },
            defaults: part >= 2,
        })
    }
}

/// The device spaces a content stream may paint in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeviceSpaces {
    gray: bool,
    rgb: bool,
    cmyk: bool,
}

impl DeviceSpaces {
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

/// How deep [`space_fits`] follows a space's base or alternate: `[/Indexed [/Separation …]]` is
/// two.
const MAX_SPACE_DEPTH: u8 = 3;

/// Whether the colour space `space` is allowed: a device space `allowed` lists, a
/// device-independent one, or an `Indexed`, `Pattern`, `Separation` or `DeviceN` space whose base
/// or alternate is. The space and its family name may be indirect, as `laminapdf-content` reads
/// them; one that can't be read counts as allowed.
fn space_fits(doc: &laminapdf_model::Document, space: &Object, allowed: DeviceSpaces, depth: u8) -> Result<bool> {
    let Some(space) = resolve(doc, space)? else {
        return Ok(true);
    };
    let array = match &*space {
        Object::Name(name) => return Ok(allowed.allows(name.as_bytes())),
        Object::Array(a) => a,
        _ => return Ok(true),
    };
    let family = match array.first() {
        Some(family) => resolve(doc, family)?,
        None => None,
    };
    let Some(Object::Name(family)) = family.as_deref() else {
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

/// `value` resolved, shared rather than copied; `None` when it can't be read or is `null`.
fn resolve(doc: &laminapdf_model::Document, value: &Object) -> Result<Option<Arc<Object>>> {
    Ok(tolerant(doc.deref(value))?
        .map(laminapdf_cos::Resolved::into_arc)
        .filter(|v| !matches!(**v, Object::Null)))
}

/// `value`'s entry `key` when `value` is a dictionary (a stream's too), resolved.
fn entry(doc: &laminapdf_model::Document, value: Option<&Object>, key: &str) -> Result<Option<Arc<Object>>> {
    match value.and_then(Object::as_dict).and_then(|d| d.get(key)) {
        Some(v) => resolve(doc, v),
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
                b"/DeviceCMYK",
            ],
            "<< /Size 5 /Root 1 0 R >>",
        );
        laminapdf_model::Document::open(bytes, laminapdf_model::OpenOptions::default()).unwrap()
    }

    /// Whether `space` fits a file with no intent, a grey, an RGB and a CMYK one.
    fn sweep(space: &Object) -> [bool; 4] {
        let doc = empty_doc();
        [Intent::Missing, Intent::Gray, Intent::Rgb, Intent::Cmyk].map(|intent| {
            let rules = Rules::of(intent, 2).unwrap();
            space_fits(&doc, space, rules.intent, 0).unwrap()
        })
    }

    #[test]
    fn device_spaces_need_an_intent_of_their_own_but_gray_takes_any() {
        let got = ["DeviceGray", "DeviceRGB", "DeviceCMYK"].map(|s| sweep(&name(s)));
        assert_eq!(
            got,
            [
                [false, true, true, true],
                [false, false, true, false],
                [false, false, false, true]
            ]
        );
    }

    #[test]
    fn an_icc_based_space_fits_any_intent() {
        let icc = array(vec![name("ICCBased"), Object::Ref(laminapdf_cos::ObjRef::new(9, 0))]);
        assert_eq!(sweep(&icc), [true; 4]);
    }

    #[test]
    fn an_indexed_separation_or_device_n_space_is_judged_by_its_base() {
        let indexed = array(vec![
            name("Indexed"),
            name("DeviceCMYK"),
            Object::Integer(1),
            Object::Null,
        ]);
        let separation = array(vec![name("Separation"), name("Spot"), name("DeviceRGB"), Object::Null]);
        let device_n = array(vec![
            name("DeviceN"),
            array(vec![name("Cyan"), name("Spot")]),
            name("DeviceCMYK"),
            Object::Null,
        ]);
        assert_eq!(
            [sweep(&indexed), sweep(&separation), sweep(&device_n)],
            [
                [false, false, false, true],
                [false, false, true, false],
                [false, false, false, true]
            ]
        );
    }

    #[test]
    fn an_indirect_space_is_read_through_its_reference() {
        // Object 4 is the name /DeviceCMYK.
        let indirect = Object::Ref(laminapdf_cos::ObjRef::new(4, 0));
        assert_eq!(sweep(&indirect), [false, false, false, true]);
    }

    #[test]
    fn an_unreadable_intent_judges_nothing() {
        assert_eq!(Rules::of(Intent::Unknown, 2), None);
    }
}
