//! The ToUnicode CMap (ISO 32000-2 §9.10.3; Adobe TN #5411, TN #5014; research 03 §5.2).
//!
//! Codes are two bytes (Identity-H). Runs of consecutive codes whose texts are single
//! consecutive UTF-16 code units become `bfrange` entries, everything else `bfchar` entries.
//! Each `begin…end` block holds at most 100 entries, a range never crosses a first-byte
//! boundary of the source codes, and the last destination byte never passes 0xFF.
//! Destinations are UTF-16BE, with surrogate pairs outside the BMP.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The most entries a `beginbfchar` or `beginbfrange` block may hold (TN #5411 §1.4.1).
pub(crate) const MAX_BLOCK: usize = 100;

/// The longest destination string, in UTF-16 code units (512 bytes, §9.10.3).
const MAX_DEST_UNITS: usize = 256;

const HEADER: &str = "/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Adobe-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n";

const FOOTER: &str = "endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n";

/// One `bfrange` entry: `lo..=hi` map to `first`, `first + 1`, … (one UTF-16 unit each).
#[derive(Debug, PartialEq, Eq)]
struct Range {
    lo: u16,
    hi: u16,
    first: u16,
}

/// Writes the CMap for `map` (code → text). Empty texts are skipped.
pub(crate) fn to_unicode_cmap(map: &BTreeMap<u16, String>) -> Vec<u8> {
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
    for (&code, text) in map {
        let units = utf16_capped(text);
        let [unit] = *units.as_slice() else {
            close(run.take(), &mut chars, &mut ranges);
            if !units.is_empty() {
                chars.push((code, units));
            }
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
    for block in chars.chunks(MAX_BLOCK) {
        let _ = writeln!(out, "{} beginbfchar", block.len());
        for (code, units) in block {
            let _ = writeln!(out, "<{code:04X}> <{}>", hex(units));
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
/// with the same first byte, and the next unit with the same high byte (only the last byte of
/// the destination is incremented, and it must stay ≤ 0xFF).
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

    fn cmap(entries: &[(u16, &str)]) -> String {
        let map = entries.iter().map(|&(c, t)| (c, t.to_owned())).collect();
        String::from_utf8(to_unicode_cmap(&map)).unwrap()
    }

    fn body(entries: &[(u16, &str)]) -> String {
        let full = cmap(entries);
        full.strip_prefix(HEADER)
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
    fn empty_text_is_skipped() {
        assert_eq!(body(&[(1, "")]), "");
    }

    #[test]
    fn blocks_hold_at_most_100_entries() {
        // Every other code, so none form a range: 250 bfchar entries in 3 blocks.
        let entries: Vec<(u16, String)> = (0..250u16).map(|i| (i * 2, "x".to_owned())).collect();
        let map = entries.into_iter().collect();
        let text = String::from_utf8(to_unicode_cmap(&map)).unwrap();
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
        let map = (0..101u16)
            .flat_map(|i| [(i * 3, "a".to_owned()), (i * 3 + 1, "b".to_owned())])
            .filter(|(c, _)| c >> 8 == (c | 1) >> 8)
            .collect::<BTreeMap<_, _>>();
        let text = String::from_utf8(to_unicode_cmap(&map)).unwrap();
        assert!(
            text.lines()
                .filter(|l| l.ends_with("beginbfrange") || l.ends_with("beginbfchar"))
                .all(|l| l.split(' ').next().unwrap().parse::<usize>().unwrap() <= MAX_BLOCK)
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
