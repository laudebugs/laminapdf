//! XFDF's names for an annotation's entries (ISO 19444-1 §6.4–6.5), one table both ways: the
//! element for each subtype, and each attribute with its PDF key and how its value is written.

use laminapdf_cos::{Dict, Object, PdfString};
use laminapdf_syntax::format_real;
use laminapdf_syntax::text::{decode_text_string, encode_text_string};

/// The subtypes an import makes: the rest are skipped.
pub(crate) const IMPORTED: [&str; 15] = [
    "Text",
    "Caret",
    "FreeText",
    "FileAttachment",
    "Highlight",
    "Underline",
    "StrikeOut",
    "Squiggly",
    "Ink",
    "Line",
    "Circle",
    "Square",
    "Polygon",
    "PolyLine",
    "Stamp",
];

/// Each XFDF element and the subtype it holds (ISO 19444-1 §6.5).
pub(crate) const ELEMENTS: [(&str, &str); 18] = [
    ("text", "Text"),
    ("caret", "Caret"),
    ("freetext", "FreeText"),
    ("fileattachment", "FileAttachment"),
    ("highlight", "Highlight"),
    ("underline", "Underline"),
    ("strikeout", "StrikeOut"),
    ("squiggly", "Squiggly"),
    ("ink", "Ink"),
    ("line", "Line"),
    ("circle", "Circle"),
    ("square", "Square"),
    ("polygon", "Polygon"),
    ("polyline", "PolyLine"),
    ("stamp", "Stamp"),
    ("sound", "Sound"),
    ("link", "Link"),
    ("redact", "Redact"),
];

/// The subtype XFDF element `element` holds.
pub(crate) fn subtype_of(element: &str) -> Option<&'static str> {
    ELEMENTS.iter().find(|(e, _)| *e == element).map(|(_, s)| *s)
}

/// The XFDF element that holds `subtype`.
pub(crate) fn element_of(subtype: &str) -> Option<&'static str> {
    ELEMENTS.iter().find(|(_, s)| *s == subtype).map(|(e, _)| *e)
}

/// How an attribute's value is written.
#[derive(Clone, Copy)]
enum Kind {
    /// A text string.
    Text,
    /// A number.
    Number,
    /// An integer.
    Integer,
    /// `true` or `false`.
    Bool,
    /// A name, written as it is.
    Name,
    /// Numbers separated by commas: a rectangle, quadrilaterals, a callout.
    Numbers,
    /// `#RRGGBB`.
    Color,
    /// The flag names, separated by commas (Table 167).
    Flags,
    /// One of a few names, each with its own value in the dictionary.
    Choice(&'static [(&'static str, Value)]),
}

/// A choice's value in the dictionary.
#[derive(Clone, Copy, PartialEq)]
enum Value {
    Name(&'static str),
    Integer(i64),
}

/// The attributes that map to one entry each, in the order the writer writes them.
const ATTRIBUTES: [(&str, &str, Kind); 24] = [
    ("rect", "Rect", Kind::Numbers),
    ("name", "NM", Kind::Text),
    ("title", "T", Kind::Text),
    ("subject", "Subj", Kind::Text),
    ("creationdate", "CreationDate", Kind::Text),
    ("date", "M", Kind::Text),
    ("color", "C", Kind::Color),
    ("interior-color", "IC", Kind::Color),
    ("opacity", "CA", Kind::Number),
    ("flags", "F", Kind::Flags),
    ("state", "State", Kind::Text),
    ("statemodel", "StateModel", Kind::Text),
    ("intent", "IT", Kind::Name),
    ("icon", "Name", Kind::Name),
    ("fringe", "RD", Kind::Numbers),
    ("coords", "QuadPoints", Kind::Numbers),
    ("rotation", "Rotate", Kind::Integer),
    ("callout", "CL", Kind::Numbers),
    ("leaderLength", "LL", Kind::Number),
    ("leaderExtended", "LLE", Kind::Number),
    ("leaderOffset", "LLO", Kind::Number),
    ("caption", "Cap", Kind::Bool),
    (
        "replyType",
        "RT",
        Kind::Choice(&[("reply", Value::Name("R")), ("group", Value::Name("Group"))]),
    ),
    (
        "justification",
        "Q",
        Kind::Choice(&[
            ("left", Value::Integer(0)),
            ("centered", Value::Integer(1)),
            ("right", Value::Integer(2)),
        ]),
    ),
];

/// A caret's `symbol` (§6.5.2).
const SYMBOLS: &[(&str, Value)] = &[("paragraph", Value::Name("P")), ("none", Value::Name("None"))];

/// The flag names in bit order (ISO 32000-2 Table 167; ISO 19444-1 §6.4.6).
const FLAGS: [&str; 10] = [
    "invisible",
    "hidden",
    "print",
    "nozoom",
    "norotate",
    "noview",
    "readonly",
    "locked",
    "togglenoview",
    "lockedcontents",
];

/// The border styles (`/BS /S`).
const STYLES: [(&str, &str); 5] = [
    ("solid", "S"),
    ("dash", "D"),
    ("bevelled", "B"),
    ("inset", "I"),
    ("underline", "U"),
];

/// Writes the entry `attr` names into `dict` from `value`; `Err` when the value doesn't parse.
/// An attribute the table doesn't know is left for the caller.
pub(crate) fn read_attribute(dict: &mut Dict, subtype: &str, attr: &str, value: &str) -> Result<bool, &'static str> {
    let found = ATTRIBUTES.iter().find(|(a, ..)| *a == attr).copied();
    let (key, kind) = match (found, attr) {
        (Some((_, key, kind)), _) => (key, kind),
        (None, "symbol") if subtype == "Caret" => ("Sy", Kind::Choice(SYMBOLS)),
        _ => return Ok(false),
    };
    dict.insert(key, parse(kind, value)?);
    Ok(true)
}

fn parse(kind: Kind, value: &str) -> Result<Object, &'static str> {
    Ok(match kind {
        Kind::Text => Object::String(PdfString::literal(encode_text_string(value))),
        Kind::Number => Object::number(number(value)?),
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a rotation or count: saturates far past any real one"
        )]
        Kind::Integer => Object::Integer(number(value)?.round() as i64),
        Kind::Bool => Object::Bool(value.trim() == "true"),
        Kind::Name => Object::name(value.trim()),
        Kind::Numbers => Object::Array(numbers(value)?.into_iter().map(Object::number).collect()),
        Kind::Color => Object::Array(color(value)?.into_iter().map(|c| Object::rounded(c, 4)).collect()),
        Kind::Flags => Object::Integer(flags(value)),
        Kind::Choice(choices) => {
            let (_, v) = choices
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(value.trim()))
                .ok_or("an unknown value")?;
            match *v {
                Value::Name(n) => Object::name(n),
                Value::Integer(i) => Object::Integer(i),
            }
        }
    })
}

/// A number as XFDF writes it.
pub(crate) fn number(value: &str) -> Result<f64, &'static str> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or("a number that doesn't parse")
}

/// Numbers separated by commas, semicolons or white space.
pub(crate) fn numbers(value: &str) -> Result<Vec<f64>, &'static str> {
    value
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(number)
        .collect()
}

/// `#RRGGBB` as three components from 0 to 1.
fn color(value: &str) -> Result<Vec<f64>, &'static str> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return Err("a colour that isn't #RRGGBB");
    }
    (0..3)
        .map(|i| {
            hex.get(i * 2..i * 2 + 2)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .map(|b| f64::from(b) / 255.0)
                .ok_or("a colour that isn't #RRGGBB")
        })
        .collect()
}

/// The flag names as `/F`'s bits; a name the table doesn't know is ignored.
fn flags(value: &str) -> i64 {
    value
        .split(',')
        .filter_map(|f| FLAGS.iter().position(|n| n.eq_ignore_ascii_case(f.trim())))
        .fold(0, |bits, i| bits | (1 << i))
}

/// The border attributes (`width`, `style`, `dashes`, `intensity`) as `/BS` and `/BE`.
pub(crate) fn read_border(
    dict: &mut Dict,
    width: Option<&str>,
    style: Option<&str>,
    dashes: Option<&str>,
    intensity: Option<&str>,
) -> Result<(), &'static str> {
    let mut bs = Dict::new();
    if let Some(w) = width {
        bs.insert("W", Object::number(number(w)?));
    }
    match style.map(str::trim) {
        Some("cloudy") => {
            let mut be = Dict::new();
            be.insert("S", Object::name("C"));
            be.insert("I", Object::number(intensity.map_or(Ok(1.0), number)?));
            dict.insert("BE", Object::Dict(be));
        }
        Some(s) => {
            if let Some((_, n)) = STYLES.iter().find(|(x, _)| *x == s) {
                bs.insert("S", Object::name(n));
            }
        }
        None => {}
    }
    if let Some(d) = dashes {
        bs.insert(
            "D",
            Object::Array(numbers(d)?.into_iter().map(Object::number).collect()),
        );
    }
    if !bs.is_empty() {
        dict.insert("BS", Object::Dict(bs));
    }
    Ok(())
}

/// The attributes `dict`'s entries map to, in the table's order, then the border's and a line's.
pub(crate) fn write_attributes(dict: &Dict) -> Vec<(&'static str, String)> {
    let subtype = dict.get_name("Subtype").unwrap_or_default();
    let mut out = Vec::new();
    for (attr, key, kind) in ATTRIBUTES {
        if let Some(value) = dict.get(key).and_then(|v| show(kind, v)) {
            out.push((attr, value));
        }
    }
    if subtype == b"Caret"
        && let Some(v) = dict.get("Sy").and_then(|v| show(Kind::Choice(SYMBOLS), v))
    {
        out.push(("symbol", v));
    }
    write_border(dict, &mut out);
    if let Some([x0, y0, x1, y1]) = dict.get("L").and_then(Object::as_array).and_then(fixed::<4>) {
        out.push(("start", format!("{},{}", format_real(x0), format_real(y0))));
        out.push(("end", format!("{},{}", format_real(x1), format_real(y1))));
    }
    if let Some(ends) = dict.get("LE").and_then(Object::as_array) {
        for (attr, end) in ["head", "tail"].into_iter().zip(ends) {
            if let Object::Name(n) = end {
                out.push((attr, String::from_utf8_lossy(n.as_bytes()).into_owned()));
            }
        }
    }
    out
}

fn write_border(dict: &Dict, out: &mut Vec<(&'static str, String)>) {
    let bs = dict.get("BS").and_then(Object::as_dict);
    if let Some(w) = bs.and_then(|b| b.get("W")).and_then(Object::as_f64) {
        out.push(("width", format_real(w)));
    }
    let cloudy = dict
        .get("BE")
        .and_then(Object::as_dict)
        .filter(|be| be.get_name("S") == Some(b"C".as_slice()));
    if let Some(be) = cloudy {
        out.push(("style", "cloudy".to_owned()));
        if let Some(i) = be.get("I").and_then(Object::as_f64) {
            out.push(("intensity", format_real(i)));
        }
    } else if let Some(s) = bs.and_then(|b| b.get_name("S"))
        && let Some((name, _)) = STYLES.iter().find(|(_, n)| n.as_bytes() == s)
    {
        out.push(("style", (*name).to_owned()));
    }
    if let Some(d) = bs.and_then(|b| b.get("D")).and_then(Object::as_array) {
        out.push(("dashes", joined(d)));
    }
}

/// `value` as attribute `kind` writes it; `None` when it has the wrong type.
fn show(kind: Kind, value: &Object) -> Option<String> {
    Some(match (kind, value) {
        (Kind::Text, Object::String(s)) => decode_text_string(&s.bytes),
        (Kind::Number, v) => format_real(v.as_f64()?),
        (Kind::Integer, v) => v.as_int()?.to_string(),
        (Kind::Bool, Object::Bool(b)) => b.to_string(),
        (Kind::Name, Object::Name(n)) => String::from_utf8_lossy(n.as_bytes()).into_owned(),
        (Kind::Numbers, Object::Array(items)) => joined(items),
        (Kind::Color, Object::Array(items)) => hex_color(items)?,
        (Kind::Flags, v) => {
            let bits = v.as_int()?;
            let names: Vec<&str> = FLAGS
                .iter()
                .enumerate()
                .filter(|(i, _)| bits & (1 << i) != 0)
                .map(|(_, n)| *n)
                .collect();
            names.join(",")
        }
        (Kind::Choice(choices), v) => {
            let wanted = match v {
                Object::Name(n) => choices
                    .iter()
                    .find(|(_, c)| matches!(c, Value::Name(x) if x.as_bytes() == n.as_bytes())),
                v => {
                    let i = v.as_int()?;
                    choices.iter().find(|(_, c)| *c == Value::Integer(i))
                }
            };
            (*wanted?.0).to_owned()
        }
        _ => return None,
    })
}

/// Numbers separated by commas.
pub(crate) fn joined(items: &[Object]) -> String {
    items
        .iter()
        .filter_map(Object::as_f64)
        .map(format_real)
        .collect::<Vec<_>>()
        .join(",")
}

/// A DeviceRGB colour as `#RRGGBB`; a grey or CMYK one as its RGB equivalent.
fn hex_color(items: &[Object]) -> Option<String> {
    let c: Vec<f64> = items.iter().filter_map(Object::as_f64).collect();
    let rgb = match c.as_slice() {
        [g] => [*g, *g, *g],
        [r, g, b] => [*r, *g, *b],
        [c, m, y, k] => [(1.0 - c) * (1.0 - k), (1.0 - m) * (1.0 - k), (1.0 - y) * (1.0 - k)],
        _ => return None,
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0–255 first"
    )]
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(format!("#{:02X}{:02X}{:02X}", byte(rgb[0]), byte(rgb[1]), byte(rgb[2])))
}

/// The first `N` numbers of `items`, when it has exactly `N`.
pub(crate) fn fixed<const N: usize>(items: &[Object]) -> Option<[f64; N]> {
    let numbers: Vec<f64> = items.iter().map(Object::as_f64).collect::<Option<_>>()?;
    numbers.try_into().ok()
}
