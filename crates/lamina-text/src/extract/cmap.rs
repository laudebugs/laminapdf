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

/// Entries looked at below a code before giving up: a wide range with single-code overrides
/// inside it (`<0000> <FFFF>` plus `cidchar`s) needs a few steps back; hostile CMaps with
/// thousands of overrides must not make every lookup linear.
const LOOKBACK: usize = 64;

fn lookup<T>(ranges: &Ranges<T>, code: Code) -> Option<(u32, &T)> {
    ranges
        .range((code.len, 0)..=(code.len, code.value))
        .rev()
        .take(LOOKBACK)
        .find(|(_, range)| code.value <= range.hi)
        .map(|(&(_, lo), range)| (code.value - lo, &range.value))
}

/// A parsed CMap (either kind).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CMap {
    codespace: Vec<CodeSpace>,
    cids: Ranges<u32>,
    notdef: Ranges<u32>,
    unicode: Ranges<Dst>,
    wmode: u8,
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

    /// `/WMode` (0 horizontal, 1 vertical).
    #[must_use]
    pub const fn vertical(&self) -> bool {
        self.wmode == 1
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
        for (key, range) in &parent.cids {
            self.cids.entry(*key).or_insert_with(|| range.clone());
        }
        for (key, range) in &parent.notdef {
            self.notdef.entry(*key).or_insert_with(|| range.clone());
        }
        for (key, range) in &parent.unicode {
            self.unicode.entry(*key).or_insert_with(|| range.clone());
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
        let value = bytes
            .get(..len)
            .unwrap_or_default()
            .iter()
            .fold(0u32, |acc, &b| (acc << 8) | u32::from(b));
        let code = Code {
            value,
            len: u8::try_from(len).unwrap_or(1),
        };
        Some((code, len))
    }

    fn fallback_len(&self, bytes: &[u8], default_len: usize) -> usize {
        let first = bytes.first().copied().unwrap_or_default();
        self.codespace
            .iter()
            .filter(|s| s.lo.first().is_some_and(|&lo| lo <= first) && s.hi.first().is_some_and(|&hi| first <= hi))
            .chain(self.codespace.iter())
            .map(|s| s.lo.len())
            .next()
            .unwrap_or(default_len)
    }

    /// The CID of `code` (`cidchar`/`cidrange`, then `notdefrange`), `None` when unmapped
    /// (which is CID 0 for rendering).
    #[must_use]
    pub fn cid(&self, code: Code) -> Option<u32> {
        if let Some((offset, start)) = lookup(&self.cids, code) {
            return start.checked_add(offset);
        }
        lookup(&self.notdef, code).map(|(_, cid)| *cid)
    }

    /// The text `code` maps to in a ToUnicode CMap (`bfchar`/`bfrange`).
    #[must_use]
    pub fn unicode(&self, code: Code) -> Option<String> {
        let (offset, dst) = lookup(&self.unicode, code)?;
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
pub fn utf16(units: &[u16]) -> String {
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

fn code_of(bytes: &[u8]) -> Option<Code> {
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

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

const fn is_white(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0')
}

const fn is_delim(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

impl Lexer<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn word(&mut self) -> &[u8] {
        let start = self.pos;
        while self.peek().is_some_and(|b| !is_white(b) && !is_delim(b)) {
            self.pos += 1;
        }
        self.bytes.get(start..self.pos).unwrap_or_default()
    }

    fn next_token(&mut self) -> Option<Token> {
        loop {
            match self.peek()? {
                b if is_white(b) => self.pos += 1,
                b'%' => {
                    while self.peek().is_some_and(|b| b != b'\n' && b != b'\r') {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        let first = self.peek()?;
        self.pos += 1;
        Some(match first {
            b'[' => Token::ArrayStart,
            b']' => Token::ArrayEnd,
            b'<' if self.peek() == Some(b'<') => {
                self.pos += 1;
                Token::Other
            }
            b'<' => self.hex(),
            b'>' => {
                if self.peek() == Some(b'>') {
                    self.pos += 1;
                }
                Token::Other
            }
            b'(' => self.string(),
            b'/' => Token::Name(self.word().to_vec()),
            b'{' | b'}' | b')' => Token::Other,
            _ => {
                self.pos -= 1;
                let word = self.word();
                if word.is_empty() {
                    self.pos += 1;
                    return Some(Token::Other);
                }
                match std::str::from_utf8(word).ok().and_then(|w| w.parse::<i64>().ok()) {
                    Some(n) => Token::Int(n),
                    None if word.iter().all(u8::is_ascii_alphabetic) => Token::Keyword(word.to_vec()),
                    None => Token::Other,
                }
            }
        })
    }

    fn hex(&mut self) -> Token {
        let mut digits = Vec::new();
        while let Some(b) = self.peek() {
            self.pos += 1;
            match b {
                b'>' => break,
                b if b.is_ascii_hexdigit() => digits.push(b),
                b if is_white(b) => {}
                _ => return Token::Other,
            }
        }
        if digits.len() % 2 == 1 {
            digits.push(b'0');
        }
        let value = |d: u8| (d as char).to_digit(16).and_then(|v| u8::try_from(v).ok()).unwrap_or(0);
        Token::Hex(
            digits
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[hi, lo]| (value(*hi) << 4) | value(*lo))
                .collect(),
        )
    }

    fn string(&mut self) -> Token {
        let mut out = Vec::new();
        let mut depth = 1usize;
        while let Some(b) = self.peek() {
            self.pos += 1;
            match b {
                b'\\' => {
                    if let Some(next) = self.peek() {
                        self.pos += 1;
                        out.push(match next {
                            b'n' => b'\n',
                            b'r' => b'\r',
                            b't' => b'\t',
                            other => other,
                        });
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(b);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(b);
                }
                _ => out.push(b),
            }
        }
        Token::Str(out)
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
            lexer: Lexer { bytes: source, pos: 0 },
            cmap: CMap::default(),
        }
    }

    fn run(mut self) -> CMap {
        let mut last: Vec<Token> = Vec::new();
        while let Some(token) = self.lexer.next_token() {
            if let Token::Keyword(word) = &token {
                if let Some(section) = Section::begin(word) {
                    self.section(section);
                } else if word == b"usecmap" {
                    if let Some(Token::Name(name)) = last.last() {
                        self.cmap.use_cmap = Some(name.clone());
                    }
                } else if word == b"def"
                    && let [.., Token::Name(key), Token::Int(value)] = last.as_slice()
                    && key == b"WMode"
                {
                    self.cmap.wmode = u8::from(*value == 1);
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

    /// Reads entries until the section's `end…` keyword (or any keyword: a missing `end` is
    /// counted and the keyword handled as usual by the caller's next round).
    fn section(&mut self, section: Section) {
        let mut operands: Vec<Operand> = Vec::with_capacity(3);
        while let Some(token) = self.lexer.next_token() {
            let operand = match token {
                Token::Keyword(_) => {
                    if !operands.is_empty() {
                        self.cmap.skipped += 1;
                    }
                    return;
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
                map.insert(
                    (lo.len, lo.value),
                    Range {
                        hi: hi.value,
                        value: cid,
                    },
                );
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
                map.insert(
                    (code.len, code.value),
                    Range {
                        hi: code.value,
                        value: cid,
                    },
                );
            }
            Section::BfChar => {
                let Some(code) = hex(0).and_then(code_of) else {
                    return false;
                };
                let Some(dst) = operands.get(1).and_then(destination) else {
                    return false;
                };
                self.cmap.unicode.insert(
                    (code.len, code.value),
                    Range {
                        hi: code.value,
                        value: Dst::List(vec![dst]),
                    },
                );
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
                self.cmap
                    .unicode
                    .insert((lo.len, lo.value), Range { hi: hi.value, value });
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
    fn unmapped_codes_and_other_lengths_have_no_unicode() {
        let cmap = CMap::parse(TO_UNICODE);
        assert_eq!([cmap.unicode(code(0x20, 2)), cmap.unicode(code(0x12, 1))], [None, None]);
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
        assert_eq!((cmap.unicode(code(1, 2)).as_deref(), cmap.skipped()), (Some("A"), 3));
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
    fn unpaired_surrogates_become_replacement_characters() {
        assert_eq!(utf16(&[0xD800, 0x41]), "\u{FFFD}A");
    }
}
