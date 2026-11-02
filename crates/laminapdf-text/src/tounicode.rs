//! The ToUnicode CMap of every font Lamina PDF writes (ISO 32000-2 §9.10.3; Adobe TN #5411,
//! TN #5014; research 03 §5.2): two-byte codes for Type 0 fonts (Identity-H), one-byte codes
//! for simple fonts.
//!
//! Two-byte maps turn runs of consecutive codes whose texts are single consecutive UTF-16
//! code units into `bfrange` entries, everything else into `bfchar` entries. A range never
//! crosses a first-byte boundary of the source codes, and the last destination byte never
//! passes 0xFF. One-byte maps hold at most 256 codes and use `bfchar` entries only, so a
//! standard 14 font's map lists each character. Each `begin…end` block holds at most 100
//! entries.
//!
//! Destinations are UTF-16BE; a character outside the BMP becomes a surrogate pair. A
//! destination stops at 512 bytes without splitting a pair. An empty text is `<code> <>`,
//! for a code that stands for no text, such as the second glyph of a cluster whose first
//! carries the text (B4b).

use std::fmt::Write as _;

/// The most entries a `beginbfchar` or `beginbfrange` block may hold (TN #5411 §1.4.1).
const MAX_BLOCK: usize = 100;

/// The longest destination string, in UTF-16 code units (512 bytes, §9.10.3).
const MAX_DEST_UNITS: usize = 256;

const HEADER: &str = "/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Adobe-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n";

const CODESPACE_END: &str = "endcodespacerange\n";

const FOOTER: &str = "endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n";

/// How long the font's codes are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Codespace {
    /// One byte, `<00>` to `<FF>`: simple fonts.
    OneByte,
    /// Two bytes, `<0000>` to `<FFFF>`: Type 0 fonts with Identity-H.
    TwoByte,
}

impl Codespace {
    /// `code` in hex, as wide as the codespace.
    fn hex(self, code: u16) -> String {
        debug_assert!(self == Self::TwoByte || code <= 0xFF, "a one-byte code above 0xFF");
        match self {
            Self::OneByte => format!("{code:02X}"),
            Self::TwoByte => format!("{code:04X}"),
        }
    }

    fn range(self) -> &'static str {
        match self {
            Self::OneByte => "<00> <FF>\n",
            Self::TwoByte => "<0000> <FFFF>\n",
        }
    }
}

/// One `bfrange` entry: `lo..=hi` map to `first`, `first + 1`, … (one UTF-16 unit each).
#[derive(Debug, PartialEq, Eq)]
struct Range {
    lo: u16,
    hi: u16,
    first: u16,
}

/// Writes the CMap for `entries` (code → text, in increasing code order; a one-byte map's
/// codes are at most 0xFF).
pub(crate) fn to_unicode_cmap<'a>(codespace: Codespace, entries: impl IntoIterator<Item = (u16, &'a str)>) -> Vec<u8> {
    let mut chars: Vec<(u16, Vec<u16>)> = Vec::new();
    let mut ranges: Vec<Range> = Vec::new();
    // The open run of single-unit destinations, as (first code, first unit, length).
    let mut run: Option<(u16, u16, u16)> = None;
    let close = |run: Option<(u16, u16, u16)>, chars: &mut Vec<(u16, Vec<u16>)>, ranges: &mut Vec<Range>| {
        if let Some((lo, first, len)) = run {
            if len == 1 {
                chars.push((lo, vec![first]));
            } else {
                ranges.push(Range {
                    lo,
                    hi: lo + (len - 1),
                    first,
                });
            }
        }
    };
    for (code, text) in entries {
        let units = utf16_capped(text);
        let single = match codespace {
            Codespace::OneByte => None,
            Codespace::TwoByte => match *units.as_slice() {
                [unit] => Some(unit),
                _ => None,
            },
        };
        let Some(unit) = single else {
            close(run.take(), &mut chars, &mut ranges);
            chars.push((code, units));
            continue;
        };
        run = match run {
            Some((lo, first, len)) if extends(lo, first, len, code, unit) => Some((lo, first, len + 1)),
            other => {
                close(other, &mut chars, &mut ranges);
                Some((code, unit, 1))
            }
        };
    }
    close(run, &mut chars, &mut ranges);

    let mut out = String::from(HEADER);
    out.push_str(codespace.range());
    out.push_str(CODESPACE_END);
    for block in chars.chunks(MAX_BLOCK) {
        let _ = writeln!(out, "{} beginbfchar", block.len());
        for &(code, ref units) in block {
            let _ = writeln!(out, "<{}> <{}>", codespace.hex(code), hex(units));
        }
        out.push_str("endbfchar\n");
    }
    for block in ranges.chunks(MAX_BLOCK) {
        let _ = writeln!(out, "{} beginbfrange", block.len());
        for r in block {
            let _ = writeln!(out, "<{:04X}> <{:04X}> <{:04X}>", r.lo, r.hi, r.first);
        }
        out.push_str("endbfrange\n");
    }
    out.push_str(FOOTER);
    out.into_bytes()
}

/// Whether `code` → `unit` continues the run of `len` codes from `lo` → `first`: the next code
/// with the same first byte, and the next unit with the same high byte that is not a
/// surrogate (a range increments only the destination's last byte, which must stay ≤ 0xFF).
fn extends(lo: u16, first: u16, len: u16, code: u16, unit: u16) -> bool {
    let Some(next_code) = lo.checked_add(len) else {
        return false;
    };
    let Some(next_unit) = first.checked_add(len) else {
        return false;
    };
    code == next_code && unit == next_unit && code >> 8 == lo >> 8 && unit >> 8 == first >> 8 && !is_surrogate(unit)
}

const fn is_surrogate(unit: u16) -> bool {
    matches!(unit, 0xD800..=0xDFFF)
}

/// `text` as UTF-16 code units, capped at [`MAX_DEST_UNITS`] without splitting a surrogate
/// pair.
fn utf16_capped(text: &str) -> Vec<u16> {
    let mut units = Vec::new();
    for ch in text.chars() {
        let mut buf = [0u16; 2];
        let encoded = ch.encode_utf16(&mut buf);
        if units.len() + encoded.len() > MAX_DEST_UNITS {
            break;
        }
        units.extend_from_slice(encoded);
    }
    units
}

fn hex(units: &[u16]) -> String {
    units.iter().fold(String::with_capacity(units.len() * 4), |mut s, u| {
        let _ = write!(s, "{u:04X}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmap_in(codespace: Codespace, entries: &[(u16, &str)]) -> String {
        String::from_utf8(to_unicode_cmap(codespace, entries.iter().copied())).unwrap()
    }

    /// The two-byte CMap's entries, without the header and footer.
    fn body(entries: &[(u16, &str)]) -> String {
        body_in(Codespace::TwoByte, entries)
    }

    fn body_in(codespace: Codespace, entries: &[(u16, &str)]) -> String {
        let full = cmap_in(codespace, entries);
        let prefix = format!("{HEADER}{}{CODESPACE_END}", codespace.range());
        full.strip_prefix(&prefix)
            .unwrap()
            .strip_suffix(FOOTER)
            .unwrap()
            .to_owned()
    }

    #[test]
    fn single_code_is_a_bfchar() {
        assert_eq!(body(&[(3, " ")]), "1 beginbfchar\n<0003> <0020>\nendbfchar\n");
    }

    #[test]
    fn consecutive_codes_and_units_become_a_bfrange() {
        assert_eq!(
            body(&[(1, "a"), (2, "b"), (3, "c")]),
            "1 beginbfrange\n<0001> <0003> <0061>\nendbfrange\n"
        );
    }

    #[test]
    fn ligature_maps_to_several_units() {
        assert_eq!(body(&[(7, "ffi")]), "1 beginbfchar\n<0007> <006600660069>\nendbfchar\n");
    }

    #[test]
    fn non_bmp_character_is_a_surrogate_pair() {
        assert_eq!(
            body(&[(9, "\u{1D400}")]),
            "1 beginbfchar\n<0009> <D835DC00>\nendbfchar\n"
        );
    }

    #[test]
    fn range_does_not_cross_a_source_first_byte_boundary() {
        assert_eq!(
            body(&[(0x00FF, "a"), (0x0100, "b")]),
            "2 beginbfchar\n<00FF> <0061>\n<0100> <0062>\nendbfchar\n"
        );
    }

    #[test]
    fn range_does_not_carry_out_of_the_last_destination_byte() {
        assert_eq!(
            body(&[(1, "\u{00FF}"), (2, "\u{0100}")]),
            "2 beginbfchar\n<0001> <00FF>\n<0002> <0100>\nendbfchar\n"
        );
    }

    #[test]
    fn empty_text_maps_to_an_empty_string() {
        assert_eq!(body(&[(1, "")]), "1 beginbfchar\n<0001> <>\nendbfchar\n");
    }

    #[test]
    fn blocks_hold_at_most_100_entries() {
        // Every other code, so none form a range: 250 bfchar entries in 3 blocks.
        let entries: Vec<(u16, &str)> = (0..250u16).map(|i| (i * 2, "x")).collect();
        let text = cmap_in(Codespace::TwoByte, &entries);
        let counts: Vec<&str> = text
            .lines()
            .filter(|l| l.ends_with("beginbfchar"))
            .map(|l| l.split(' ').next().unwrap())
            .collect();
        assert_eq!(counts, ["100", "100", "50"]);
    }

    #[test]
    fn ranges_are_split_into_blocks_of_100() {
        // 101 two-code ranges separated by gaps.
        let entries: Vec<(u16, &str)> = (0..101u16)
            .flat_map(|i| [(i * 3, "a"), (i * 3 + 1, "b")])
            .filter(|(c, _)| c >> 8 == (c | 1) >> 8)
            .collect();
        let text = cmap_in(Codespace::TwoByte, &entries);
        assert!(
            text.lines()
                .filter(|l| l.ends_with("beginbfrange") || l.ends_with("beginbfchar"))
                .all(|l| l.split(' ').next().unwrap().parse::<usize>().unwrap() <= MAX_BLOCK)
        );
    }

    #[test]
    fn one_byte_codes_are_bfchar_entries_only() {
        assert_eq!(
            body_in(Codespace::OneByte, &[(0x41, "A"), (0x42, "B"), (0x81, "\u{1F600}")]),
            "3 beginbfchar\n<41> <0041>\n<42> <0042>\n<81> <D83DDE00>\nendbfchar\n"
        );
    }

    #[test]
    fn a_full_one_byte_map_is_split_into_blocks_of_100() {
        let entries: Vec<(u16, &str)> = (0..=0xFF).map(|c| (c, "x")).collect();
        let cmap = cmap_in(Codespace::OneByte, &entries);
        let blocks: Vec<&str> = cmap.lines().filter(|l| l.ends_with("beginbfchar")).collect();
        assert_eq!(blocks, ["100 beginbfchar", "100 beginbfchar", "56 beginbfchar"]);
    }

    #[test]
    fn a_one_byte_codespace_is_00_to_ff() {
        assert!(cmap_in(Codespace::OneByte, &[]).contains("begincodespacerange\n<00> <FF>\n"));
    }

    #[test]
    fn a_two_byte_codespace_is_0000_to_ffff() {
        assert!(cmap_in(Codespace::TwoByte, &[]).contains("begincodespacerange\n<0000> <FFFF>\n"));
    }

    #[test]
    fn a_one_byte_empty_text_maps_to_nothing() {
        assert_eq!(
            body_in(Codespace::OneByte, &[(0x41, "")]),
            "1 beginbfchar\n<41> <>\nendbfchar\n"
        );
    }

    #[test]
    fn a_one_byte_map_keeps_a_surrogate_pair_whole_at_the_cap() {
        let text = format!("{}\u{1F600}", "a".repeat(255));
        assert_eq!(
            body_in(Codespace::OneByte, &[(0x41, &text)]),
            format!("1 beginbfchar\n<41> <{}>\nendbfchar\n", "0061".repeat(255))
        );
    }

    #[test]
    fn destination_is_capped_at_512_bytes() {
        let long = "a".repeat(300);
        assert_eq!(utf16_capped(&long).len(), MAX_DEST_UNITS);
    }

    #[test]
    fn cap_does_not_split_a_surrogate_pair() {
        let text = format!("{}\u{1D400}", "a".repeat(255));
        assert_eq!(utf16_capped(&text).len(), 255);
    }
}
