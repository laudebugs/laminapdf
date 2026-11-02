//! The PDF object model (ISO 32000-1 §7.3) and its exact serialization rules.

use std::fmt;

/// A reference to an indirect object: `num gen R`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjRef {
    /// Object number.
    pub num: u32,
    /// Generation number.
    pub gen_nr: u16,
}

impl ObjRef {
    /// Creates a reference to object `num` with generation `gen_nr`.
    #[must_use]
    pub const fn new(num: u32, gen_nr: u16) -> Self {
        Self { num, gen_nr }
    }
}

impl fmt::Display for ObjRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.num, self.gen_nr)
    }
}

/// How a string was (or should be) written: `(literal)` or `<hex>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StringFormat {
    /// `( … )` with backslash escapes.
    #[default]
    Literal,
    /// `< … >` uppercase hexadecimal.
    Hex,
}

/// A PDF string: raw bytes plus the syntax it was written in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfString {
    /// The decoded string bytes.
    pub bytes: Vec<u8>,
    /// Literal or hex form, remembered so that re-serialization round-trips.
    pub format: StringFormat,
}

impl PdfString {
    /// A literal string `( … )`.
    #[must_use]
    pub fn literal(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
            format: StringFormat::Literal,
        }
    }

    /// A hex string `< … >`.
    #[must_use]
    pub fn hex(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
            format: StringFormat::Hex,
        }
    }
}

/// A PDF name, stored as its decoded bytes without the leading `/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Name(pub Vec<u8>);

impl Name {
    /// Creates a name from its decoded bytes (no leading `/`).
    #[must_use]
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// The decoded bytes of the name.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

/// An insertion-ordered dictionary. Lookups are linear, which is faster than hashing
/// for the handful of keys real PDF dictionaries carry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dict {
    entries: Vec<(Name, Object)>,
}

impl Dict {
    /// An empty dictionary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up `key` (without the leading `/`).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Object> {
        self.entries.iter().find(|(k, _)| k.0 == key.as_bytes()).map(|(_, v)| v)
    }

    /// Mutable lookup of `key`.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Object> {
        self.entries
            .iter_mut()
            .find(|(k, _)| k.0 == key.as_bytes())
            .map(|(_, v)| v)
    }

    /// True when `key` is present.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Sets `key` to `value`. An existing key keeps its position; a new key is appended.
    pub fn insert(&mut self, key: impl Into<Name>, value: impl Into<Object>) {
        let key = key.into();
        let value = value.into();
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.entries.push((key, value)),
        }
    }

    /// Removes `key`, returning its value.
    pub fn remove(&mut self, key: &str) -> Option<Object> {
        let pos = self.entries.iter().position(|(k, _)| k.0 == key.as_bytes())?;
        Some(self.entries.remove(pos).1)
    }

    /// Entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&Name, &Object)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// Entries in insertion order, with mutable values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Name, &mut Object)> {
        self.entries.iter_mut().map(|(k, v)| (&*k, v))
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the dictionary has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `key` as a name, if present and a name.
    #[must_use]
    pub fn get_name(&self, key: &str) -> Option<&[u8]> {
        match self.get(key) {
            Some(Object::Name(n)) => Some(n.as_bytes()),
            _ => None,
        }
    }

    /// `key` as an integer, if present and an integer.
    #[must_use]
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Object::as_int)
    }
}

impl<K: Into<Name>, V: Into<Object>> FromIterator<(K, V)> for Dict {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut dict = Self::new();
        for (k, v) in iter {
            dict.insert(k, v);
        }
        dict
    }
}

/// A stream: its dictionary plus the raw (still encoded) bytes between `stream` and `endstream`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stream {
    /// The stream dictionary. `/Length` is managed by the writer.
    pub dict: Dict,
    /// Raw stream bytes as stored in the file (encoded by `/Filter`, if any).
    pub data: Vec<u8>,
}

impl Stream {
    /// A stream whose bytes are stored as-is.
    #[must_use]
    pub fn new(dict: Dict, data: impl Into<Vec<u8>>) -> Self {
        Self {
            dict,
            data: data.into(),
        }
    }

    /// A Flate-compressed stream: compresses `decoded` and sets `/Filter /FlateDecode`
    /// (keeping the key's position if the caller already placed it).
    #[must_use]
    pub fn flate(mut dict: Dict, decoded: &[u8]) -> Self {
        dict.insert("Filter", Name::from("FlateDecode"));
        Self {
            dict,
            data: crate::filters::flate_encode(decoded),
        }
    }
}

/// A PDF object.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Object {
    /// `null`.
    #[default]
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// An integer.
    Integer(i64),
    /// A real number.
    Real(f64),
    /// A literal or hex string.
    String(PdfString),
    /// A name.
    Name(Name),
    /// An array.
    Array(Vec<Object>),
    /// A dictionary.
    Dict(Dict),
    /// A stream (only valid as the direct value of an indirect object).
    Stream(Stream),
    /// An indirect reference.
    Ref(ObjRef),
}

impl Object {
    /// `v` as a number: an integer when it is integral (`612`, not `612.0`), else a real.
    #[must_use]
    pub fn number(v: f64) -> Self {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "only integral values well inside i64 get here"
        )]
        if v.fract() == 0.0 && v.abs() < 1e15 {
            Self::Integer(v as i64)
        } else {
            Self::Real(v)
        }
    }

    /// `v` rounded to `decimals` decimal places, then written as [`Object::number`] writes it.
    #[must_use]
    pub fn rounded(v: f64, decimals: i32) -> Self {
        let scale = 10f64.powi(decimals);
        Self::number((v * scale).round() / scale)
    }

    /// A name object.
    #[must_use]
    pub fn name(s: &str) -> Self {
        Self::Name(Name::from(s))
    }

    /// The integer value, if this is an integer.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// The numeric value of an integer or real.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "PDF numbers are approximate by definition")]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Integer(i) => Some(*i as f64),
            Self::Real(r) => Some(*r),
            _ => None,
        }
    }

    /// The dictionary, if this is a dictionary (or a stream's dictionary).
    #[must_use]
    pub fn as_dict(&self) -> Option<&Dict> {
        match self {
            Self::Dict(d) => Some(d),
            Self::Stream(s) => Some(&s.dict),
            _ => None,
        }
    }

    /// The array, if this is an array.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Object]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// The reference, if this is a reference.
    #[must_use]
    pub fn as_ref(&self) -> Option<ObjRef> {
        match self {
            Self::Ref(r) => Some(*r),
            _ => None,
        }
    }

    /// Serializes this object to a fresh buffer using the crate's exact formatting rules.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_to(&mut out);
        out
    }

    /// Appends the serialization of this object to `out`.
    ///
    /// Streams are written as `<<dict>>\nstream\n…\nendstream` with `/Length` set to the
    /// data length (kept in place if present, appended last otherwise).
    pub fn write_to(&self, out: &mut Vec<u8>) {
        match self {
            Self::Null => out.extend_from_slice(b"null"),
            Self::Bool(true) => out.extend_from_slice(b"true"),
            Self::Bool(false) => out.extend_from_slice(b"false"),
            Self::Integer(i) => out.extend_from_slice(i.to_string().as_bytes()),
            Self::Real(r) => out.extend_from_slice(format_real(*r).as_bytes()),
            Self::String(s) => write_string(s, out),
            Self::Name(n) => write_name(n.as_bytes(), out),
            Self::Array(items) => {
                out.push(b'[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(b' ');
                    }
                    item.write_to(out);
                }
                out.push(b']');
            }
            Self::Dict(d) => write_dict(d, out),
            Self::Stream(s) => {
                let mut dict = s.dict.clone();
                let len = i64::try_from(s.data.len()).unwrap_or(i64::MAX);
                dict.insert("Length", len);
                write_dict(&dict, out);
                out.extend_from_slice(b"\nstream\n");
                out.extend_from_slice(&s.data);
                out.extend_from_slice(b"\nendstream");
            }
            Self::Ref(r) => out.extend_from_slice(r.to_string().as_bytes()),
        }
    }
}

macro_rules! impl_from {
    ($($t:ty => $variant:ident),* $(,)?) => {
        $(impl From<$t> for Object {
            fn from(v: $t) -> Self { Self::$variant(v.into()) }
        })*
    };
}

impl_from! {
    bool => Bool, i64 => Integer, i32 => Integer, u32 => Integer, f64 => Real,
    PdfString => String, Name => Name, Vec<Object> => Array, Dict => Dict,
    Stream => Stream, ObjRef => Ref,
}

/// Formats a real: never an exponent, at most 6 fractional digits, trailing zeros and a
/// trailing `.` trimmed, `-0` becomes `0`. Non-finite values (which PDF cannot express)
/// are written as `0`.
#[must_use]
pub fn format_real(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_owned();
    }
    let mut s = format!("{value:.6}");
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.').len();
        s.truncate(trimmed);
    }
    if s == "-0" {
        s.clear();
        s.push('0');
    }
    s
}

/// Bytes that must be `#xx`-escaped inside a name besides those outside `!`..`~`.
fn name_needs_escape(b: u8) -> bool {
    !(b'!'..=b'~').contains(&b) || b"#()<>[]{}/%".contains(&b)
}

const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

fn push_hex(b: u8, out: &mut Vec<u8>) {
    let digit = |nibble: u8| HEX_UPPER.get(usize::from(nibble)).copied().unwrap_or(b'0');
    out.push(digit(b >> 4));
    out.push(digit(b & 0x0F));
}

pub(crate) fn write_name(bytes: &[u8], out: &mut Vec<u8>) {
    out.push(b'/');
    for &b in bytes {
        if name_needs_escape(b) {
            out.push(b'#');
            push_hex(b, out);
        } else {
            out.push(b);
        }
    }
}

pub(crate) fn write_literal_string(bytes: &[u8], out: &mut Vec<u8>) {
    out.push(b'(');
    for &b in bytes {
        match b {
            b'\\' | b'(' | b')' => {
                out.push(b'\\');
                out.push(b);
            }
            b'\r' => out.extend_from_slice(b"\\r"),
            _ => out.push(b),
        }
    }
    out.push(b')');
}

fn write_string(s: &PdfString, out: &mut Vec<u8>) {
    match s.format {
        StringFormat::Literal => write_literal_string(&s.bytes, out),
        StringFormat::Hex => {
            out.push(b'<');
            for &b in &s.bytes {
                push_hex(b, out);
            }
            out.push(b'>');
        }
    }
}

fn write_dict(d: &Dict, out: &mut Vec<u8>) {
    out.extend_from_slice(b"<<");
    for (k, v) in d.iter() {
        out.push(b' ');
        write_name(k.as_bytes(), out);
        out.push(b' ');
        v.write_to(out);
    }
    out.extend_from_slice(b" >>");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ser(o: &Object) -> String {
        String::from_utf8_lossy(&o.to_bytes()).into_owned()
    }

    #[test]
    fn number_writes_integral_values_as_integers() {
        assert_eq!(
            (Object::number(612.0), Object::number(0.5), Object::number(-3.0)),
            (Object::Integer(612), Object::Real(0.5), Object::Integer(-3))
        );
    }

    #[test]
    fn rounded_rounds_then_writes_a_whole_result_as_an_integer() {
        assert_eq!(
            (
                Object::rounded(-12.004, 2),
                Object::rounded(-12.345, 2),
                Object::rounded(600.098, 3)
            ),
            (Object::Integer(-12), Object::Real(-12.35), Object::Real(600.098))
        );
    }

    #[test]
    fn format_real_trims_trailing_zeros() {
        assert_eq!(format_real(0.5), "0.5");
    }

    #[test]
    fn format_real_drops_trailing_dot_for_whole_numbers() {
        assert_eq!(format_real(1.0), "1");
    }

    #[test]
    fn format_real_keeps_sign_of_negative_fraction() {
        assert_eq!(format_real(-0.25), "-0.25");
    }

    #[test]
    fn format_real_turns_negative_zero_into_zero() {
        assert_eq!(format_real(-0.0), "0");
    }

    #[test]
    fn format_real_turns_tiny_negative_rounding_to_zero_into_zero() {
        assert_eq!(format_real(-0.000_000_1), "0");
    }

    #[test]
    fn format_real_never_uses_an_exponent() {
        assert_eq!(format_real(1e20), "100000000000000000000");
    }

    #[test]
    fn format_real_rounds_exact_tie_half_to_even() {
        // 2^-7 = 0.0078125 is exactly representable: a true tie at 6 decimals.
        assert_eq!(format_real(0.007_812_5), "0.007812");
    }

    #[test]
    fn format_real_rounds_to_six_digits() {
        assert_eq!(format_real(1.0 / 3.0), "0.333333");
    }

    #[test]
    fn name_escapes_delimiters_space_and_hash() {
        assert_eq!(
            ser(&Object::Name(Name::new(&b"a b#(c)/%"[..]))),
            "/a#20b#23#28c#29#2F#25"
        );
    }

    #[test]
    fn name_escapes_non_ascii_with_uppercase_hex() {
        assert_eq!(ser(&Object::Name(Name::new(vec![0xE9, b'x']))), "/#E9x");
    }

    #[test]
    fn literal_string_escapes_parens_backslash_and_cr() {
        assert_eq!(
            ser(&PdfString::literal(&b"a(b)c\\d\re\n"[..]).into()),
            "(a\\(b\\)c\\\\d\\re\n)"
        );
    }

    #[test]
    fn hex_string_is_uppercase() {
        assert_eq!(ser(&PdfString::hex(vec![0xAB, 0x01]).into()), "<AB01>");
    }

    #[test]
    fn dict_uses_single_spaces() {
        let d: Dict = [
            ("K", Object::from(1)),
            ("K2", Object::Array(vec![Object::Null, true.into()])),
        ]
        .into_iter()
        .collect();
        assert_eq!(ser(&d.into()), "<< /K 1 /K2 [null true] >>");
    }

    #[test]
    fn empty_dict_serializes_as_double_brackets_with_space() {
        assert_eq!(ser(&Dict::new().into()), "<< >>");
    }

    #[test]
    fn stream_appends_length_last_when_absent() {
        let s = Stream::new([("Type", Object::name("X"))].into_iter().collect(), b"abc".to_vec());
        assert_eq!(ser(&s.into()), "<< /Type /X /Length 3 >>\nstream\nabc\nendstream");
    }

    #[test]
    fn stream_keeps_caller_length_position() {
        let d: Dict = [("Length", Object::from(99)), ("Filter", Object::name("F"))]
            .into_iter()
            .collect();
        assert_eq!(
            ser(&Stream::new(d, b"ab".to_vec()).into()),
            "<< /Length 2 /Filter /F >>\nstream\nab\nendstream"
        );
    }

    #[test]
    fn dict_insert_keeps_existing_key_position() {
        let mut d: Dict = [("A", 1), ("B", 2)].into_iter().collect();
        d.insert("A", 3);
        assert_eq!(ser(&d.into()), "<< /A 3 /B 2 >>");
    }
}
