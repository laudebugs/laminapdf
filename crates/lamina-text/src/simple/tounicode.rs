//! The ToUnicode CMap of a simple font (ISO 32000-2 §9.10.3; Adobe TN #5411): one-byte codes
//! (`<00> <FF>` codespace), `bfchar` entries only (at most 256 codes), at most 100 per block,
//! destinations in UTF-16BE; an empty text is `<code> <>` (the code stands for nothing), as
//! in the Type 0 CMap.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The most entries a `beginbfchar` block may hold (TN #5411 §1.4.1).
const MAX_BLOCK: usize = 100;

/// The longest destination string, in UTF-16 code units (512 bytes, §9.10.3).
const MAX_DEST_UNITS: usize = 256;

const HEADER: &str = "/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Adobe-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<00> <FF>\n\
endcodespacerange\n";

const FOOTER: &str = "endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n";

/// The CMap for `map` (code → text).
pub(super) fn one_byte_cmap(map: &BTreeMap<u8, String>) -> Vec<u8> {
    let entries: Vec<(u8, &String)> = map.iter().map(|(&c, t)| (c, t)).collect();
    let mut out = String::from(HEADER);
    for block in entries.chunks(MAX_BLOCK) {
        let _ = writeln!(out, "{} beginbfchar", block.len());
        for (code, text) in block {
            let dest = text
                .encode_utf16()
                .take(MAX_DEST_UNITS)
                .fold(String::new(), |mut s, u| {
                    let _ = write!(s, "{u:04X}");
                    s
                });
            let _ = writeln!(out, "<{code:02X}> <{dest}>");
        }
        out.push_str("endbfchar\n");
    }
    out.push_str(FOOTER);
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_one_byte_codes_to_utf16() {
        let map = BTreeMap::from([(0x41, "A".to_owned()), (0x81, "😀".to_owned())]);
        let cmap = String::from_utf8(one_byte_cmap(&map)).unwrap();
        assert!(
            cmap.contains("2 beginbfchar\n<41> <0041>\n<81> <D83DDE00>\nendbfchar\n"),
            "{cmap}"
        );
    }

    #[test]
    fn empty_text_maps_to_nothing() {
        let cmap = String::from_utf8(one_byte_cmap(&BTreeMap::from([(0x41, String::new())]))).unwrap();
        assert!(cmap.contains("<41> <>\n"), "{cmap}");
    }

    #[test]
    fn blocks_hold_at_most_100_entries() {
        let map: BTreeMap<u8, String> = (0..=u8::MAX).map(|c| (c, "x".to_owned())).collect();
        let cmap = String::from_utf8(one_byte_cmap(&map)).unwrap();
        let blocks: Vec<&str> = cmap.lines().filter(|l| l.ends_with("beginbfchar")).collect();
        assert_eq!(blocks, ["100 beginbfchar", "100 beginbfchar", "56 beginbfchar"]);
    }
}
