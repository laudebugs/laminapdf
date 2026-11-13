//! An image's colour space (ISO 32000-2 §8.6) and the conversion of its components to grey or
//! RGB for a PNG. CMYK and Lab use simple formulas, not a colour-management module: a
//! colour-managed conversion comes later. Separation is written as its tint (darker is more
//! ink), since evaluating its tint transform needs PDF functions the core doesn't have yet.

use laminapdf_model::Document;
use laminapdf_syntax::Object;

/// How deep `/Indexed` and `/ICCBased` alternates may nest.
const MAX_DEPTH: u8 = 4;

/// An image's colour space, as far as extraction needs it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Space {
    Gray,
    Rgb,
    Cmyk,
    /// `/Lab`: the a* and b* ranges.
    Lab {
        range: [f64; 4],
    },
    /// `/Indexed`: a base space, the highest index, and the table of base components.
    Indexed {
        base: Box<Self>,
        high: u16,
        table: Vec<u8>,
    },
    /// An `/ICCBased` space, converted as the device space its components make.
    Icc(Box<Self>),
    /// `/Separation` (one tint), or `/DeviceN` with one colourant (`device_n`).
    Tint {
        device_n: bool,
    },
    /// A space extraction can't convert, by a fixed English name (never the file's own text):
    /// `/DeviceN` with several colourants, `/Pattern`, one it can't find or read.
    Unsupported(&'static str),
}

/// What a space's components turn into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Output {
    Gray,
    Rgb,
}

impl Space {
    /// The number of components a sample has.
    pub(crate) fn components(&self) -> usize {
        match self {
            Self::Gray | Self::Indexed { .. } | Self::Tint { .. } | Self::Unsupported(_) => 1,
            Self::Icc(device) => device.components(),
            Self::Rgb | Self::Lab { .. } => 3,
            Self::Cmyk => 4,
        }
    }

    /// What the components turn into.
    pub(crate) fn output(&self) -> Output {
        match self {
            Self::Gray | Self::Tint { .. } | Self::Unsupported(_) => Output::Gray,
            Self::Icc(device) => device.output(),
            Self::Rgb | Self::Cmyk | Self::Lab { .. } => Output::Rgb,
            Self::Indexed { base, .. } => base.output(),
        }
    }

    /// The default `/Decode` range of component `i`.
    pub(crate) fn decode_default(&self, i: usize, bits: u8) -> (f64, f64) {
        match self {
            Self::Icc(device) => device.decode_default(i, bits),
            Self::Indexed { .. } => (0.0, f64::from((1u32 << bits) - 1)),
            Self::Lab { range, .. } => match i {
                0 => (0.0, 100.0),
                1 => (range[0], range[1]),
                _ => (range[2], range[3]),
            },
            _ => (0.0, 1.0),
        }
    }

    /// Writes the colour of `values` (decoded components) to `out`, each channel in 0–1.
    pub(crate) fn convert(&self, values: &[f64], out: &mut Vec<f64>) {
        let at = |i: usize| values.get(i).copied().unwrap_or(0.0);
        match self {
            Self::Gray | Self::Unsupported(_) => out.push(at(0).clamp(0.0, 1.0)),
            Self::Tint { .. } => out.push((1.0 - at(0)).clamp(0.0, 1.0)),
            Self::Icc(device) => device.convert(values, out),
            Self::Rgb => out.extend((0..3).map(|i| at(i).clamp(0.0, 1.0))),
            Self::Cmyk => {
                let k = 1.0 - at(3).clamp(0.0, 1.0);
                out.extend((0..3).map(|i| (1.0 - at(i).clamp(0.0, 1.0)) * k));
            }
            Self::Lab { .. } => out.extend(lab_to_srgb(at(0), at(1), at(2))),
            Self::Indexed { base, high, table } => {
                // Clamped to the table, so a bad index reads its last entry.
                let index = usize::from(*high).min(index_of(at(0)));
                let n = base.components();
                let entry: Vec<f64> = (0..n)
                    .map(|c| f64::from(table.get(index * n + c).copied().unwrap_or(0)) / 255.0)
                    .collect();
                let decoded: Vec<f64> = match &**base {
                    // A Lab table's bytes span the Lab ranges (§8.6.6.3).
                    Self::Lab { range, .. } => vec![
                        entry.first().copied().unwrap_or(0.0) * 100.0,
                        range[0] + entry.get(1).copied().unwrap_or(0.0) * (range[1] - range[0]),
                        range[2] + entry.get(2).copied().unwrap_or(0.0) * (range[3] - range[2]),
                    ],
                    _ => entry,
                };
                base.convert(&decoded, out);
            }
        }
    }

    /// The colour space's family, as `pdfimages -list` names it.
    pub(crate) fn family(&self) -> String {
        match self {
            Self::Gray => "gray".into(),
            Self::Rgb => "rgb".into(),
            Self::Cmyk => "cmyk".into(),
            Self::Lab { .. } => "lab".into(),
            Self::Indexed { .. } => "index".into(),
            Self::Icc(_) => "icc".into(),
            Self::Tint { device_n: false } => "sep".into(),
            Self::Tint { device_n: true } => "devn".into(),
            Self::Unsupported(name) => (*name).into(),
        }
    }
}

/// The table index `value` names, rounded; a negative value is 0.
fn index_of(value: f64) -> usize {
    // Clamped to 0–65535 first, so the cast keeps every value.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0–65535"
    )]
    let index = value.round().clamp(0.0, 65535.0) as usize;
    index
}

/// CIE L*a*b* to sRGB, each 0–1. The colour is taken relative to its white point, which is
/// mapped to sRGB's (D65) without a chromatic adaptation: a simple conversion until a
/// colour-managed one.
fn lab_to_srgb(lightness: f64, a_star: f64, b_star: f64) -> [f64; 3] {
    let fy = (lightness + 16.0) / 116.0;
    let fx = fy + a_star / 500.0;
    let fz = fy - b_star / 200.0;
    let inverse = |t: f64| {
        if t > 6.0 / 29.0 {
            t * t * t
        } else {
            3.0 * (6.0f64 / 29.0).powi(2) * (t - 4.0 / 29.0)
        }
    };
    // D65's white, so the Lab white comes out white.
    let (cx, cy, cz) = (0.9505 * inverse(fx), inverse(fy), 1.0890 * inverse(fz));
    let red = 3.2406 * cx - 1.5372 * cy - 0.4986 * cz;
    let green = -0.9689 * cx + 1.8758 * cy + 0.0415 * cz;
    let blue = 0.0557 * cx - 0.2040 * cy + 1.0570 * cz;
    let curve = |linear: f64| {
        let linear = linear.clamp(0.0, 1.0);
        if linear <= 0.003_130_8 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        }
    };
    [curve(red), curve(green), curve(blue)]
}

/// The device space a family name stands for (the inline abbreviations and the CIE-based spaces
/// a simple conversion treats as their device space included).
fn device(name: &[u8]) -> Option<Space> {
    match name {
        b"DeviceGray" | b"G" | b"CalGray" => Some(Space::Gray),
        b"DeviceRGB" | b"RGB" | b"CalRGB" => Some(Space::Rgb),
        b"DeviceCMYK" | b"CMYK" => Some(Space::Cmyk),
        _ => None,
    }
}

/// The colour space `object` names, resolved in `doc`; a name not among the device spaces is
/// looked up in `resources`' `/ColorSpace` (an inline image's `/CS /CS0`).
pub(crate) fn parse(doc: &Document, object: &Object, resources: Option<&Object>) -> Space {
    parse_at(doc, object, resources, 0)
}

fn parse_at(doc: &Document, object: &Object, resources: Option<&Object>, depth: u8) -> Space {
    let Ok(resolved) = doc.deref(object) else {
        return Space::Unsupported("an unreadable colour space");
    };
    match &*resolved {
        Object::Name(name) => match device(name.as_bytes()) {
            Some(space) => space,
            None if depth < MAX_DEPTH => named(doc, resources, name.as_bytes())
                .map_or(Space::Unsupported("a colour space the page doesn't define"), |found| {
                    parse_at(doc, &found, None, depth + 1)
                }),
            None => Space::Unsupported("colour spaces nested too deep"),
        },
        Object::Array(items) if depth < MAX_DEPTH => family(doc, items, resources, depth),
        _ => Space::Unsupported("an unknown colour space"),
    }
}

/// The `/ColorSpace` resource `name`.
fn named(doc: &Document, resources: Option<&Object>, name: &[u8]) -> Option<Object> {
    let resources = doc.deref(resources?).ok()?;
    let spaces = doc.deref(resources.as_dict()?.get("ColorSpace")?).ok()?;
    let key = std::str::from_utf8(name).ok()?;
    spaces.as_dict()?.get(key).cloned()
}

fn family(doc: &Document, items: &[Object], resources: Option<&Object>, depth: u8) -> Space {
    let Some(Object::Name(head)) = items.first() else {
        return Space::Unsupported("an unknown colour space");
    };
    let arg = |i: usize| items.get(i).and_then(|o| doc.deref(o).ok().map(|r| (*r).clone()));
    if let Some(space) = device(head.as_bytes()) {
        return space;
    }
    match head.as_bytes() {
        b"ICCBased" => match arg(1) {
            Some(Object::Stream(icc)) => match icc.dict.get_int("N") {
                Some(1) => Space::Icc(Box::new(Space::Gray)),
                Some(3) => Space::Icc(Box::new(Space::Rgb)),
                Some(4) => Space::Icc(Box::new(Space::Cmyk)),
                // An alternate that can't be converted leaves the image unconvertible.
                _ => match icc
                    .dict
                    .get("Alternate")
                    .map(|alt| parse_at(doc, alt, resources, depth + 1))
                {
                    Some(unsupported @ Space::Unsupported(_)) => unsupported,
                    Some(alternate) => Space::Icc(Box::new(alternate)),
                    None => Space::Unsupported("ICCBased"),
                },
            },
            _ => Space::Unsupported("ICCBased"),
        },
        b"Lab" => {
            let dict = arg(1).and_then(|o| o.as_dict().cloned()).unwrap_or_default();
            let numbers = |key: &str, n: usize| -> Option<Vec<f64>> {
                let values: Vec<f64> = dict.get(key)?.as_array()?.iter().filter_map(Object::as_f64).collect();
                (values.len() == n).then_some(values)
            };
            let range = numbers("Range", 4)
                .and_then(|r| <[f64; 4]>::try_from(r).ok())
                .unwrap_or([-100.0, 100.0, -100.0, 100.0]);
            Space::Lab { range }
        }
        b"Indexed" | b"I" => indexed(doc, items, resources, depth),
        b"Separation" => Space::Tint { device_n: false },
        b"DeviceN" => match arg(1) {
            Some(Object::Array(names)) if names.len() == 1 => Space::Tint { device_n: true },
            _ => Space::Unsupported("DeviceN"),
        },
        b"Pattern" => Space::Unsupported("Pattern"),
        _ => Space::Unsupported("an unknown colour space"),
    }
}

/// `[/Indexed base hival lookup]`, the lookup a string or a stream.
fn indexed(doc: &Document, items: &[Object], resources: Option<&Object>, depth: u8) -> Space {
    let Some(base) = items.get(1).map(|b| parse_at(doc, b, resources, depth + 1)) else {
        return Space::Unsupported("Indexed");
    };
    let high = items
        .get(2)
        .and_then(|h| doc.deref(h).ok()?.as_int())
        .and_then(|h| u16::try_from(h.clamp(0, 255)).ok())
        .unwrap_or(0);
    let table = match items.get(3).and_then(|t| doc.deref(t).ok()).as_deref() {
        Some(Object::String(s)) => s.bytes.clone(),
        Some(Object::Stream(stream)) => doc.decode_stream(stream).unwrap_or_default(),
        _ => Vec::new(),
    };
    if matches!(base, Space::Indexed { .. } | Space::Unsupported(_)) {
        return Space::Unsupported("Indexed");
    }
    Space::Indexed {
        base: Box::new(base),
        high,
        table,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_converts_by_the_simple_formula() {
        let mut out = Vec::new();
        Space::Cmyk.convert(&[0.0, 1.0, 1.0, 0.0], &mut out);
        assert_eq!(out, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_tint_is_darker_with_more_ink() {
        let mut out = Vec::new();
        Space::Tint { device_n: false }.convert(&[0.25], &mut out);
        assert_eq!(out, vec![0.75]);
    }

    #[test]
    fn an_index_past_the_table_reads_its_last_entry() {
        let space = Space::Indexed {
            base: Box::new(Space::Rgb),
            high: 1,
            table: vec![255, 0, 0, 0, 0, 255],
        };
        let mut out = Vec::new();
        space.convert(&[7.0], &mut out);
        assert_eq!(out, vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn lab_white_is_white() {
        let rgb = lab_to_srgb(100.0, 0.0, 0.0);
        assert!(rgb.iter().all(|c| *c > 0.95), "{rgb:?}");
    }
}
