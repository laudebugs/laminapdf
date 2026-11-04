//! CMaps (ISO 32000-2 §9.7.5, Adobe TN #5014) and ToUnicode CMaps (§9.10.3, TN #5411): one
//! tolerant parser for both.
//!
//! A CMap splits a string into codes by its codespace ranges, maps codes to CIDs
//! (`cidchar`, `cidrange`, `notdefrange`) and, in a ToUnicode CMap, to Unicode (`bfchar`,
//! `bfrange` in both forms). Ranges are kept as ranges, never expanded, so a hostile range
//! costs nothing; the stream's size is already bounded by `Limits::max_stream_decoded`.
//! Malformed entries are skipped and counted ([`CMap::skipped`]); the caller raises one
//! `cmap-invalid` warning for them.

use std::collections::BTreeMap;

/// One character code: its value and how many bytes it took (1–4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code {
    /// The code's bytes, big-endian.
    pub value: u32,
    /// Its length in bytes.
    pub len: u8,
}

/// The longest code a CMap may define (TN #5014 §4).
const MAX_CODE_LEN: usize = 4;

/// A codespace range: each byte of a code within the range's bytes (TN #5014 §4.1, a
/// rectangle, not an interval).
#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeSpace {
    lo: Vec<u8>,
    hi: Vec<u8>,
}

impl CodeSpace {
    fn contains(&self, bytes: &[u8]) -> bool {
        bytes.len() == self.lo.len()
            && bytes
                .iter()
                .zip(self.lo.iter().zip(&self.hi))
                .all(|(b, (lo, hi))| lo <= b && b <= hi)
    }
}

/// What a code range maps to in a ToUnicode CMap.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Dst {
    /// `bfrange` with one destination: the range's first code maps to these UTF-16 units and
    /// each next code to the same with the last unit incremented (§9.10.3).
    Base(Vec<u16>),
    /// `bfrange` with an array (or `bfchar`): one destination per code, in order.
    List(Vec<Vec<u16>>),
}

/// A code range and what it maps to, keyed in its map by `(len, lo)`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Range<T> {
    hi: u32,
    value: T,
}

/// Ranges keyed by `(code length, first code)`; a later entry with the same start replaces
/// an earlier one (pdf.js and Poppler keep the last).
type Ranges<T> = BTreeMap<(u8, u32), Range<T>>;

/// Ranges looked at below a code before giving up: overlapping ranges (a wide range with
/// narrower ones inside it) need a few steps back; a hostile CMap with thousands of nested
/// ranges must not make every lookup linear. Single codes are kept apart, so any number of
/// `cidchar`/`bfchar` overrides inside a wide range costs nothing.
const LOOKBACK: usize = 64;

/// One mapping: single codes (`…char`) and ranges (`…range`), kept apart. A single code wins
/// over a range that covers it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Table<T> {
    singles: BTreeMap<(u8, u32), T>,
    ranges: Ranges<T>,
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self {
            singles: BTreeMap::new(),
            ranges: BTreeMap::new(),
        }
    }
}

impl<T: Clone> Table<T> {
    fn insert(&mut self, lo: Code, hi: u32, value: T) {
        if lo.value == hi {
            self.singles.insert((lo.len, lo.value), value);
        } else {
            self.ranges.insert((lo.len, lo.value), Range { hi, value });
        }
    }

    /// The entry covering `code` and `code`'s offset from the entry's first code.
    fn get(&self, code: Code) -> Option<(u32, &T)> {
        if let Some(value) = self.singles.get(&(code.len, code.value)) {
            return Some((0, value));
        }
        self.ranges
            .range((code.len, 0)..=(code.len, code.value))
            .rev()
            .take(LOOKBACK)
            .find(|(_, range)| code.value <= range.hi)
            .map(|(&(_, lo), range)| (code.value - lo, &range.value))
    }

    /// Adds `parent`'s entries where this table has none at the same start.
    fn inherit(&mut self, parent: &Self) {
        for (key, value) in &parent.singles {
            self.singles.entry(*key).or_insert_with(|| value.clone());
        }
        for (key, range) in &parent.ranges {
            self.ranges.entry(*key).or_insert_with(|| range.clone());
        }
    }
}

/// A parsed CMap (either kind).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CMap {
    codespace: Vec<CodeSpace>,
    cids: Table<u32>,
    notdef: Table<u32>,
    unicode: Table<Dst>,
    /// `/WMode` as the source states it (`None`: not stated).
    wmode: Option<u8>,
    use_cmap: Option<Vec<u8>>,
    skipped: usize,
}

impl CMap {
    /// Parses CMap source. Never fails: what can't be read is skipped and counted.
    #[must_use]
    pub fn parse(source: &[u8]) -> Self {
        Parser::new(source).run()
    }

    /// Malformed entries skipped while parsing.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    /// `/WMode` (0 horizontal, 1 vertical; horizontal when not stated).
    #[must_use]
    pub const fn vertical(&self) -> bool {
        matches!(self.wmode, Some(1))
    }

    /// Sets `/WMode` from the CMap stream's dictionary (Table 118) when the source states
    /// none.
    pub fn default_vertical(&mut self, vertical: bool) {
        self.wmode.get_or_insert(u8::from(vertical));
    }

    /// The CMap named by `usecmap`, if any (resolved by the caller).
    #[must_use]
    pub fn use_cmap(&self) -> Option<&[u8]> {
        self.use_cmap.as_deref()
    }

    /// Whether the CMap defines any codespace range.
    #[must_use]
    pub fn has_codespace(&self) -> bool {
        !self.codespace.is_empty()
    }

    /// Adds `parent`'s definitions under this CMap's own (`usecmap`: the parent's entries
    /// apply where this CMap defines none).
    pub fn inherit(&mut self, parent: &Self) {
        for space in &parent.codespace {
            if !self.codespace.contains(space) {
                self.codespace.push(space.clone());
            }
        }
        self.cids.inherit(&parent.cids);
        self.notdef.inherit(&parent.notdef);
        self.unicode.inherit(&parent.unicode);
        if self.wmode.is_none() {
            self.wmode = parent.wmode;
        }
    }

    /// The first code of `bytes` and the bytes it took (§9.7.6.2): the shortest prefix inside
    /// a codespace range. With no match, as many bytes as the shortest codespace range that
    /// the first byte fits (else the shortest range), so one bad code costs few bytes;
    /// `default_len` without codespace ranges. `None` only when `bytes` is empty.
    #[must_use]
    pub fn next_code(&self, bytes: &[u8], default_len: usize) -> Option<(Code, usize)> {
        if bytes.is_empty() {
            return None;
        }
        let len = (1..=MAX_CODE_LEN.min(bytes.len()))
            .find(|&n| {
                bytes
                    .get(..n)
                    .is_some_and(|p| self.codespace.iter().any(|s| s.contains(p)))
            })
            .unwrap_or_else(|| self.fallback_len(bytes, default_len));
        let len = len.clamp(1, bytes.len());
        Some((code_of(bytes.get(..len)?)?, len))
    }

    fn fallback_len(&self, bytes: &[u8], default_len: usize) -> usize {
        let first = bytes.first().copied().unwrap_or_default();
        let fits =
            |s: &&CodeSpace| s.lo.first().is_some_and(|&lo| lo <= first) && s.hi.first().is_some_and(|&hi| first <= hi);
        self.codespace
            .iter()
            .filter(fits)
            .map(|s| s.lo.len())
            .min()
            .or_else(|| self.codespace.iter().map(|s| s.lo.len()).min())
            .unwrap_or(default_len)
    }

    /// The CID of `code` (`cidchar`/`cidrange`, then `notdefrange`), `None` when unmapped
    /// (which is CID 0 for rendering).
    #[must_use]
    pub fn cid(&self, code: Code) -> Option<u32> {
        if let Some((offset, start)) = self.cids.get(code) {
            return start.checked_add(offset);
        }
        self.notdef.get(code).map(|(_, cid)| *cid)
    }

    /// The text `code` maps to in a ToUnicode CMap (`bfchar`/`bfrange`). A code the CMap
    /// writes with another byte length (`<0041>` entries for a one-byte simple font, a common
    /// producer bug) matches by value, as in pdf.js and Poppler.
    #[must_use]
    pub fn unicode(&self, code: Code) -> Option<String> {
        let (offset, dst) = self.unicode.get(code).or_else(|| {
            (1..=4u8)
                .filter(|&len| len != code.len && (len == 4 || code.value >> (8 * u32::from(len)) == 0))
                .find_map(|len| self.unicode.get(Code { len, ..code }))
        })?;
        let units = match dst {
            Dst::List(list) => list.get(usize::try_from(offset).ok()?)?.clone(),
            Dst::Base(base) => {
                let mut units = base.clone();
                let last = units.last_mut()?;
                // §9.10.3: only the last byte is incremented; pdf.js and Poppler add to the
                // whole last unit, which agrees whenever the file keeps the rule.
                *last = last.checked_add(u16::try_from(offset).ok()?)?;
                units
            }
        };
        Some(utf16(&units))
    }
}

/// UTF-16 units to text; an unpaired surrogate becomes U+FFFD.
#[must_use]
pub(super) fn utf16(units: &[u16]) -> String {
    char::decode_utf16(units.iter().copied())
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Big-endian bytes to UTF-16 units; a trailing odd byte is dropped.
fn units(bytes: &[u8]) -> Vec<u16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_be_bytes(*pair))
        .collect()
}

/// The code `bytes` (1–4 of them, big-endian) spell.
pub(crate) fn code_of(bytes: &[u8]) -> Option<Code> {
    if bytes.is_empty() || bytes.len() > MAX_CODE_LEN {
        return None;
    }
    let value = bytes.iter().fold(0u32, |acc, &b| (acc << 8) | u32::from(b));
    Some(Code {
        value,
        len: u8::try_from(bytes.len()).ok()?,
    })
}

/// A CMap token.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Hex(Vec<u8>),
    Str(Vec<u8>),
    Int(i64),
    Name(Vec<u8>),
    ArrayStart,
    ArrayEnd,
    Keyword(Vec<u8>),
    /// Anything else (reals, dictionaries' `<<`/`>>`, braces): skipped.
    Other,
}

/// CMap tokens over `laminapdf-syntax`'s [`Lexer`](laminapdf_syntax::Lexer), tolerantly: bytes
/// that are not a token (a bad hex digit, an unbalanced `)` or `>`) are skipped one at a time
/// as [`Token::Other`], and a token cut off by the end of the data, or one over
/// [`Limits::max_string_length`](laminapdf_syntax::Limits::max_string_length), ends the CMap.
struct Lexer<'a> {
    inner: laminapdf_syntax::Lexer<'a>,
}

impl<'a> Lexer<'a> {
    const fn new(source: &'a [u8]) -> Self {
        Self {
            inner: laminapdf_syntax::Lexer::new(source, 0),
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        use laminapdf_syntax::Token as Syntax;
        self.inner.skip_whitespace();
        let start = self.inner.pos();
        let token = match self.inner.next_token() {
            Ok(token) => token,
            Err(laminapdf_syntax::Error::Syntax { .. }) => {
                self.inner.set_pos(start + 1);
                return Some(Token::Other);
            }
            Err(_) => return None,
        };
        Some(match token {
            Syntax::Eof => return None,
            Syntax::Integer(n) => Token::Int(n),
            Syntax::Name(name) => Token::Name(name),
            Syntax::LiteralString(bytes) => Token::Str(bytes),
            Syntax::HexString(bytes) => Token::Hex(bytes),
            Syntax::ArrayOpen => Token::ArrayStart,
            Syntax::ArrayClose => Token::ArrayEnd,
            Syntax::Keyword(word) if word.iter().all(u8::is_ascii_alphabetic) => Token::Keyword(word.to_vec()),
            Syntax::Real(_) | Syntax::DictOpen | Syntax::DictClose | Syntax::Keyword(_) => Token::Other,
        })
    }
}

/// A `begin…`/`end…` section being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    CodeSpace,
    CidRange,
    CidChar,
    NotDefRange,
    NotDefChar,
    BfChar,
    BfRange,
}

impl Section {
    fn begin(keyword: &[u8]) -> Option<Self> {
        Some(match keyword {
            b"begincodespacerange" => Self::CodeSpace,
            b"begincidrange" => Self::CidRange,
            b"begincidchar" => Self::CidChar,
            b"beginnotdefrange" => Self::NotDefRange,
            b"beginnotdefchar" => Self::NotDefChar,
            b"beginbfchar" => Self::BfChar,
            b"beginbfrange" => Self::BfRange,
            _ => return None,
        })
    }

    /// Operands per entry.
    const fn arity(self) -> usize {
        match self {
            Self::CodeSpace | Self::CidChar | Self::NotDefChar | Self::BfChar => 2,
            Self::CidRange | Self::NotDefRange | Self::BfRange => 3,
        }
    }
}

/// An operand of a section entry (an array is collected whole).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Operand {
    Token(Token),
    Array(Vec<Token>),
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    cmap: CMap,
}

impl<'a> Parser<'a> {
    fn new(source: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(source),
            cmap: CMap::default(),
        }
    }

    fn run(mut self) -> CMap {
        let mut last: Vec<Token> = Vec::new();
        // A keyword that ended a section missing its `end…`: handled here next.
        let mut pending: Option<Token> = None;
        while let Some(token) = pending.take().or_else(|| self.lexer.next_token()) {
            if let Token::Keyword(word) = &token {
                if let Some(section) = Section::begin(word) {
                    pending = self.section(section).map(Token::Keyword);
                } else if word == b"usecmap" {
                    if let Some(Token::Name(name)) = last.last() {
                        self.cmap.use_cmap = Some(name.clone());
                    }
                } else if word == b"def"
                    && let [.., Token::Name(key), Token::Int(value)] = last.as_slice()
                    && key == b"WMode"
                {
                    self.cmap.wmode = Some(u8::from(*value == 1));
                }
                last.clear();
            } else {
                // Only the last two operands matter (`/WMode 1 def`, `/Name usecmap`).
                if last.len() == 2 {
                    last.remove(0);
                }
                last.push(token);
            }
        }
        self.cmap
    }

    /// Reads entries until an `end…` keyword. Any other keyword means the `end…` is missing:
    /// that is counted and the keyword is returned for [`Parser::run`] to handle (so a
    /// `begin…` right after starts its own section).
    fn section(&mut self, section: Section) -> Option<Vec<u8>> {
        let mut operands: Vec<Operand> = Vec::with_capacity(3);
        while let Some(token) = self.lexer.next_token() {
            let operand = match token {
                Token::Keyword(word) => {
                    if word.starts_with(b"end") {
                        self.cmap.skipped += usize::from(!operands.is_empty());
                        return None;
                    }
                    self.cmap.skipped += 1;
                    return Some(word);
                }
                Token::ArrayStart => Operand::Array(self.array()),
                Token::Other | Token::ArrayEnd => {
                    self.cmap.skipped += 1;
                    operands.clear();
                    continue;
                }
                other => Operand::Token(other),
            };
            operands.push(operand);
            if operands.len() == section.arity() {
                if !self.entry(section, &operands) {
                    self.cmap.skipped += 1;
                }
                operands.clear();
            }
        }
        None
    }

    fn array(&mut self) -> Vec<Token> {
        let mut items = Vec::new();
        while let Some(token) = self.lexer.next_token() {
            match token {
                Token::ArrayEnd | Token::Keyword(_) => break,
                other => items.push(other),
            }
        }
        items
    }

    /// Adds one entry; `false` when it is malformed.
    fn entry(&mut self, section: Section, operands: &[Operand]) -> bool {
        let hex = |i: usize| match operands.get(i) {
            Some(Operand::Token(Token::Hex(bytes))) => Some(bytes.as_slice()),
            _ => None,
        };
        let int = |i: usize| match operands.get(i) {
            Some(Operand::Token(Token::Int(n))) => u32::try_from(*n).ok(),
            _ => None,
        };
        let range = || {
            let (lo, hi) = (code_of(hex(0)?)?, code_of(hex(1)?)?);
            (lo.len == hi.len && lo.value <= hi.value).then_some((lo, hi))
        };
        match section {
            Section::CodeSpace => {
                let (Some(lo), Some(hi)) = (hex(0), hex(1)) else {
                    return false;
                };
                if lo.is_empty() || lo.len() != hi.len() || lo.len() > MAX_CODE_LEN {
                    return false;
                }
                self.cmap.codespace.push(CodeSpace {
                    lo: lo.to_vec(),
                    hi: hi.to_vec(),
                });
            }
            Section::CidRange | Section::NotDefRange => {
                let (Some((lo, hi)), Some(cid)) = (range(), int(2)) else {
                    return false;
                };
                let map = if section == Section::CidRange {
                    &mut self.cmap.cids
                } else {
                    &mut self.cmap.notdef
                };
                map.insert(lo, hi.value, cid);
            }
            Section::CidChar | Section::NotDefChar => {
                let (Some(code), Some(cid)) = (hex(0).and_then(code_of), int(1)) else {
                    return false;
                };
                let map = if section == Section::CidChar {
                    &mut self.cmap.cids
                } else {
                    &mut self.cmap.notdef
                };
                map.insert(code, code.value, cid);
            }
            Section::BfChar => {
                let Some(code) = hex(0).and_then(code_of) else {
                    return false;
                };
                let Some(dst) = operands.get(1).and_then(destination) else {
                    return false;
                };
                self.cmap.unicode.insert(code, code.value, Dst::List(vec![dst]));
            }
            Section::BfRange => {
                let Some((lo, hi)) = range() else { return false };
                let value = match operands.get(2) {
                    Some(Operand::Array(items)) => Dst::List(
                        items
                            .iter()
                            .map(|t| destination(&Operand::Token(t.clone())).unwrap_or_default())
                            .collect(),
                    ),
                    Some(other) => match destination(other) {
                        Some(base) if !base.is_empty() => Dst::Base(base),
                        _ => return false,
                    },
                    None => return false,
                };
                self.cmap.unicode.insert(lo, hi.value, value);
            }
        }
        true
    }
}

/// A ToUnicode destination: UTF-16BE hex (the rule), a literal string read as UTF-16BE, or a
/// glyph name (an old form some producers write) through the AGL.
fn destination(operand: &Operand) -> Option<Vec<u16>> {
    match operand {
        Operand::Token(Token::Hex(bytes) | Token::Str(bytes)) => Some(units(bytes)),
        Operand::Token(Token::Name(name)) => {
            let text = crate::agl::to_unicode(std::str::from_utf8(name).ok()?)?;
            Some(text.encode_utf16().collect())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{CMap, Code, utf16};

    const TO_UNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap
        /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
        /CMapName /Adobe-Identity-UCS def /CMapType 2 def
        1 begincodespacerange <0000> <FFFF> endcodespacerange
        3 beginbfchar
        <0003> <0020>
        <0024> <00660069>
        <0051> <D835DC00>
        endbfchar
        2 beginbfrange
        <0010> <001F> <0041>
        <0030> <0032> [<0066> <00660066> <00660066006C>]
        endbfrange
        endcmap CMapName currentdict /CMap defineresource pop end end";

    fn code(value: u32, len: u8) -> Code {
        Code { value, len }
    }

    #[test]
    fn bfchar_maps_single_multi_and_surrogate_destinations() {
        let cmap = CMap::parse(TO_UNICODE);
        assert_eq!(
            [3, 0x24, 0x51].map(|v| cmap.unicode(code(v, 2))),
            [Some(" ".into()), Some("fi".into()), Some("\u{1D400}".into())]
        );
    }

    #[test]
    fn bfrange_increments_the_last_unit() {
        assert_eq!(CMap::parse(TO_UNICODE).unicode(code(0x12, 2)).as_deref(), Some("C"));
    }

    #[test]
    fn bfrange_array_form_maps_each_code() {
        assert_eq!(CMap::parse(TO_UNICODE).unicode(code(0x32, 2)).as_deref(), Some("ffl"));
    }

    #[test]
    fn unmapped_codes_have_no_unicode() {
        let cmap = CMap::parse(TO_UNICODE);
        assert_eq!(
            [cmap.unicode(code(0x20, 2)), cmap.unicode(code(0x2000, 2))],
            [None, None]
        );
    }

    #[test]
    fn a_well_formed_cmap_skips_nothing() {
        assert_eq!(CMap::parse(TO_UNICODE).skipped(), 0);
    }

    #[test]
    fn codes_split_by_codespace_ranges_of_mixed_length() {
        let cmap = CMap::parse(b"2 begincodespacerange <00> <80> <8140> <9FFC> endcodespacerange");
        let bytes = [0x41, 0x81, 0x40, 0x42];
        let mut rest: &[u8] = &bytes;
        let mut codes = Vec::new();
        while let Some((c, n)) = cmap.next_code(rest, 1) {
            codes.push(c);
            rest = rest.get(n..).unwrap_or_default();
        }
        assert_eq!(codes, [code(0x41, 1), code(0x8140, 2), code(0x42, 1)]);
    }

    #[test]
    fn a_code_outside_every_codespace_takes_the_range_length_of_its_first_byte() {
        let cmap = CMap::parse(b"1 begincodespacerange <8140> <9FFC> endcodespacerange");
        assert_eq!(cmap.next_code(&[0x81, 0x20], 1), Some((code(0x8120, 2), 2)));
    }

    #[test]
    fn an_unmatched_code_takes_the_shortest_fitting_range_length() {
        let cmap = CMap::parse(b"2 begincodespacerange <81000000> <81FFFFFF> <8140> <81FF> endcodespacerange");
        assert_eq!(cmap.next_code(&[0x81, 0x20, 0x41], 1), Some((code(0x8120, 2), 2)));
    }

    #[test]
    fn without_codespace_the_default_length_applies() {
        assert_eq!(CMap::default().next_code(&[1, 2, 3], 2), Some((code(0x0102, 2), 2)));
    }

    #[test]
    fn cid_ranges_chars_and_notdef_resolve_in_order() {
        let cmap = CMap::parse(
            b"1 begincodespacerange <0000> <FFFF> endcodespacerange
              1 begincidrange <0100> <01FF> 500 endcidrange
              1 begincidchar <0200> 7 endcidchar
              1 beginnotdefrange <0300> <03FF> 1 endnotdefrange",
        );
        assert_eq!(
            [0x0105, 0x0200, 0x0333, 0x0400].map(|v| cmap.cid(code(v, 2))),
            [Some(505), Some(7), Some(1), None]
        );
    }

    #[test]
    fn wmode_and_usecmap_are_read() {
        let cmap = CMap::parse(b"/WMode 1 def /UniJIS-UCS2-H usecmap");
        assert_eq!((cmap.vertical(), cmap.use_cmap()), (true, Some(&b"UniJIS-UCS2-H"[..])));
    }

    #[test]
    fn names_decode_their_hex_escapes() {
        let cmap = CMap::parse(b"/UniJIS#2DUCS2#2DH usecmap 1 beginbfchar <01> /f#69 endbfchar");
        assert_eq!(
            (cmap.use_cmap(), cmap.unicode(code(1, 1)).as_deref()),
            (Some(&b"UniJIS-UCS2-H"[..]), Some("\u{FB01}"))
        );
    }

    #[test]
    fn inherited_entries_fill_in_below_the_cmaps_own() {
        let mut child = CMap::parse(b"1 begincidchar <0001> 10 endcidchar");
        child.inherit(&CMap::parse(b"1 begincidrange <0000> <00FF> 100 endcidrange"));
        assert_eq!([child.cid(code(1, 2)), child.cid(code(2, 2))], [Some(10), Some(102)]);
    }

    #[test]
    fn malformed_entries_are_skipped_and_counted() {
        let cmap = CMap::parse(
            b"3 beginbfchar <0001> <0041> <0002> 42 <XYZ> <0043> endbfchar
              1 beginbfrange <0010> <0005> <0041> endbfrange",
        );
        assert_eq!((cmap.unicode(code(1, 2)).as_deref(), cmap.skipped()), (Some("A"), 4));
    }

    #[test]
    fn an_unterminated_section_keeps_its_entries() {
        let cmap = CMap::parse(b"1 beginbfchar <01> <0041> endcmap");
        assert_eq!(cmap.unicode(code(1, 1)).as_deref(), Some("A"));
    }

    #[test]
    fn glyph_name_destinations_go_through_the_agl() {
        let cmap = CMap::parse(b"1 beginbfchar <01> /eacute endbfchar");
        assert_eq!(cmap.unicode(code(1, 1)).as_deref(), Some("é"));
    }

    #[test]
    fn a_huge_range_is_not_expanded() {
        let cmap = CMap::parse(b"1 beginbfrange <00000000> <FFFFFFFF> <0041> endbfrange");
        assert_eq!(cmap.unicode(code(1, 4)).as_deref(), Some("B"));
    }

    #[test]
    fn an_increment_past_the_last_unit_maps_to_nothing() {
        let cmap = CMap::parse(b"1 beginbfrange <0000> <FFFF> <FFFF> endbfrange");
        assert_eq!(cmap.unicode(code(2, 2)), None);
    }

    #[test]
    fn a_missing_end_keeps_the_next_section() {
        let cmap = CMap::parse(b"1 beginbfchar <01> <0041> 1 beginbfrange <02> <03> <0042> endbfrange");
        assert_eq!(
            (cmap.unicode(code(3, 1)).as_deref(), cmap.skipped() > 0),
            (Some("C"), true)
        );
    }

    #[test]
    fn many_single_overrides_do_not_hide_a_wide_range() {
        let mut source = b"1 begincidrange <0000> <FFFF> 0 endcidrange 200 begincidchar".to_vec();
        for i in 0..200u32 {
            source.extend(format!(" <{:04X}> 7", 0x0100 + i).bytes());
        }
        source.extend(b" endcidchar");
        let cmap = CMap::parse(&source);
        assert_eq!(
            [cmap.cid(code(0x0150, 2)), cmap.cid(code(0x5000, 2))],
            [Some(7), Some(0x5000)]
        );
    }

    #[test]
    fn to_unicode_codes_of_another_length_match_by_value() {
        let cmap = CMap::parse(b"1 beginbfchar <0041> <0042> endbfchar");
        assert_eq!(cmap.unicode(code(0x41, 1)).as_deref(), Some("B"));
    }

    #[test]
    fn literal_strings_read_octal_escapes_and_line_continuations() {
        let cmap = CMap::parse(b"2 beginbfchar <01> (\\000A) <02> (\\000\\\nB) endbfchar");
        assert_eq!(
            [cmap.unicode(code(1, 1)), cmap.unicode(code(2, 1))],
            [Some("A".into()), Some("B".into())]
        );
    }

    #[test]
    fn wmode_is_inherited_and_defaults_from_the_dictionary() {
        let mut child = CMap::parse(b"");
        child.inherit(&CMap::parse(b"/WMode 1 def"));
        let mut stated = CMap::parse(b"/WMode 0 def");
        stated.default_vertical(true);
        let mut unstated = CMap::parse(b"");
        unstated.default_vertical(true);
        assert_eq!(
            [child.vertical(), stated.vertical(), unstated.vertical()],
            [true, false, true]
        );
    }

    #[test]
    fn unpaired_surrogates_become_replacement_characters() {
        assert_eq!(utf16(&[0xD800, 0x41]), "\u{FFFD}A");
    }
}
