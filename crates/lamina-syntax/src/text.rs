//! Text strings (ISO 32000-2 §7.9.2.2): `PDFDocEncoding`, UTF-16BE with a byte-order mark,
//! and (PDF 2.0) UTF-8 with a byte-order mark.
//!
//! This module holds the workspace's one `PDFDocEncoding` table (Annex D, Table D.2), in both
//! directions: [`pdf_doc_decode`] maps a code to its Unicode scalar and [`pdf_doc_encode`]
//! maps a scalar back to its code. Layers above (the string encoder for ops, simple fonts)
//! reuse it instead of writing their own.
//!
//! ```
//! use lamina_syntax::text::decode_text_string;
//!
//! assert_eq!(decode_text_string(b"Caf\xe9 \x84 menu"), "Café — menu");
//! assert_eq!(decode_text_string(b"\xfe\xff\x00H\x00i\xd8\x3d\xde\x00"), "Hi😀");
//! assert_eq!(decode_text_string(b"\xef\xbb\xbfna\xc3\xafve"), "naïve");
//! ```

/// The Unicode replacement character, produced for undefined codes and invalid sequences.
pub const REPLACEMENT: char = '\u{FFFD}';

/// Codes 0x80–0xA0 of `PDFDocEncoding` (Table D.2); `None` marks an undefined code (0x9F).
const HIGH: [Option<char>; 33] = [
    Some('\u{2022}'), // 0x80 bullet
    Some('\u{2020}'), // 0x81 dagger
    Some('\u{2021}'), // 0x82 daggerdbl
    Some('\u{2026}'), // 0x83 ellipsis
    Some('\u{2014}'), // 0x84 emdash
    Some('\u{2013}'), // 0x85 endash
    Some('\u{0192}'), // 0x86 florin
    Some('\u{2044}'), // 0x87 fraction
    Some('\u{2039}'), // 0x88 guilsinglleft
    Some('\u{203A}'), // 0x89 guilsinglright
    Some('\u{2212}'), // 0x8A minus
    Some('\u{2030}'), // 0x8B perthousand
    Some('\u{201E}'), // 0x8C quotedblbase
    Some('\u{201C}'), // 0x8D quotedblleft
    Some('\u{201D}'), // 0x8E quotedblright
    Some('\u{2018}'), // 0x8F quoteleft
    Some('\u{2019}'), // 0x90 quoteright
    Some('\u{201A}'), // 0x91 quotesinglbase
    Some('\u{2122}'), // 0x92 trademark
    Some('\u{FB01}'), // 0x93 fi
    Some('\u{FB02}'), // 0x94 fl
    Some('\u{0141}'), // 0x95 Lslash
    Some('\u{0152}'), // 0x96 OE
    Some('\u{0160}'), // 0x97 Scaron
    Some('\u{0178}'), // 0x98 Ydieresis
    Some('\u{017D}'), // 0x99 Zcaron
    Some('\u{0131}'), // 0x9A dotlessi
    Some('\u{0142}'), // 0x9B lslash
    Some('\u{0153}'), // 0x9C oe
    Some('\u{0161}'), // 0x9D scaron
    Some('\u{017E}'), // 0x9E zcaron
    None,             // 0x9F undefined
    Some('\u{20AC}'), // 0xA0 Euro
];

/// Codes 0x18–0x1F of `PDFDocEncoding`: spacing accents.
const ACCENTS: [char; 8] = [
    '\u{02D8}', // 0x18 breve
    '\u{02C7}', // 0x19 caron
    '\u{02C6}', // 0x1A circumflex
    '\u{02D9}', // 0x1B dotaccent
    '\u{02DD}', // 0x1C hungarumlaut
    '\u{02DB}', // 0x1D ogonek
    '\u{02DA}', // 0x1E ring
    '\u{02DC}', // 0x1F tilde
];

/// The Unicode scalar of `PDFDocEncoding` code `code`, or `None` for an undefined code.
///
/// Defined (Annex D, Table D.2): tab, line feed and carriage return; the spacing accents at
/// 0x18–0x1F; printable ASCII 0x20–0x7E; the punctuation and letters at 0x80–0x9E; the Euro
/// sign at 0xA0; and ISO Latin-1 at 0xA1–0xFF except 0xAD. Undefined: the other C0 controls,
/// 0x7F, 0x9F and 0xAD.
///
/// ```
/// use lamina_syntax::text::pdf_doc_decode;
///
/// assert_eq!(pdf_doc_decode(0x92), Some('™'));
/// assert_eq!(pdf_doc_decode(0x9F), None);
/// ```
#[must_use]
pub fn pdf_doc_decode(code: u8) -> Option<char> {
    match code {
        0x09 | 0x0A | 0x0D | 0x20..=0x7E | 0xA1..=0xAC | 0xAE..=0xFF => Some(char::from(code)),
        0x18..=0x1F => ACCENTS.get(usize::from(code - 0x18)).copied(),
        0x80..=0xA0 => HIGH.get(usize::from(code - 0x80)).copied().flatten(),
        _ => None,
    }
}

/// The `PDFDocEncoding` code of `c`, or `None` when `c` has no code (the reverse of
/// [`pdf_doc_decode`]).
///
/// ```
/// use lamina_syntax::text::pdf_doc_encode;
///
/// assert_eq!(pdf_doc_encode('€'), Some(0xA0));
/// assert_eq!(pdf_doc_encode('é'), Some(0xE9));
/// assert_eq!(pdf_doc_encode('中'), None);
/// ```
#[must_use]
pub fn pdf_doc_encode(c: char) -> Option<u8> {
    let scalar = u32::from(c);
    if let Ok(code) = u8::try_from(scalar)
        && pdf_doc_decode(code) == Some(c)
    {
        return Some(code);
    }
    // Every other defined code maps outside Latin-1: search the two small tables.
    let in_table = |table: &[Option<char>], base: u8| {
        table
            .iter()
            .position(|&t| t == Some(c))
            .and_then(|i| u8::try_from(i).ok())
            .map(|i| base + i)
    };
    let accents: [Option<char>; 8] = ACCENTS.map(Some);
    in_table(&accents, 0x18).or_else(|| in_table(&HIGH, 0x80))
}

/// Decodes a PDF text string (§7.9.2.2) to Unicode.
///
/// * Bytes starting with `FE FF` are UTF-16BE. Unpaired surrogates and a trailing odd byte
///   become U+FFFD; language escape sequences (`U+001B`, a language code, `U+001B`,
///   §7.9.2.2.1) are removed.
/// * Bytes starting with `EF BB BF` are UTF-8 (PDF 2.0); invalid sequences become U+FFFD.
/// * Anything else is `PDFDocEncoding`; undefined codes become U+FFFD.
///
/// Never fails: text strings are display data, and a damaged one is still worth showing.
#[must_use]
pub fn decode_text_string(bytes: &[u8]) -> String {
    if let Some(utf16) = bytes.strip_prefix(b"\xFE\xFF") {
        decode_utf16be(utf16)
    } else if let Some(utf8) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
        String::from_utf8_lossy(utf8).into_owned()
    } else {
        bytes
            .iter()
            .map(|&b| pdf_doc_decode(b).unwrap_or(REPLACEMENT))
            .collect()
    }
}

/// Encodes `text` as a PDF text string (§7.9.2.2), the inverse of [`decode_text_string`]:
/// `PDFDocEncoding` when every character has a code, else UTF-16BE with a `FE FF` byte
/// order mark. PDFDocEncoding output that would start with a byte order mark (`þÿ`, `ï»¿`)
/// is written as UTF-16BE too, so it never decodes as something else.
///
/// ```
/// use lamina_syntax::text::{decode_text_string, encode_text_string};
///
/// assert_eq!(encode_text_string("Café €"), b"Caf\xE9 \xA0");
/// assert_eq!(encode_text_string("中"), b"\xFE\xFF\x4E\x2D");
/// assert_eq!(decode_text_string(&encode_text_string("Grüße, 世界")), "Grüße, 世界");
/// ```
#[must_use]
pub fn encode_text_string(text: &str) -> Vec<u8> {
    let pdf_doc: Option<Vec<u8>> = text.chars().map(pdf_doc_encode).collect();
    match pdf_doc {
        Some(bytes) if !bytes.starts_with(b"\xFE\xFF") && !bytes.starts_with(b"\xEF\xBB\xBF") => bytes,
        _ => {
            let mut out = Vec::with_capacity(2 + text.len() * 2);
            out.extend_from_slice(b"\xFE\xFF");
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_be_bytes());
            }
            out
        }
    }
}

fn decode_utf16be(bytes: &[u8]) -> String {
    let (pairs, rest) = bytes.as_chunks::<2>();
    let odd = !rest.is_empty();
    let units = pairs.iter().map(|&pair| u16::from_be_bytes(pair));
    let chars: Vec<char> = char::decode_utf16(units).map(|c| c.unwrap_or(REPLACEMENT)).collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c == ESCAPE {
            if let Some(len) = language_escape(chars.get(i..).unwrap_or_default()) {
                i += len;
                continue;
            }
            // A stray or unterminated escape is not a language marker: keep the text.
            out.push(REPLACEMENT);
        } else {
            out.push(c);
        }
        i += 1;
    }
    if odd {
        out.push(REPLACEMENT);
    }
    out
}

/// The escape character that brackets a language code in a UTF-16 text string.
const ESCAPE: char = '\u{1B}';

/// The length of the language escape sequence `chars` starts with (§7.9.2.2.1): ESC, a
/// two-letter ISO 639 language code, an optional two-letter ISO 3166 country code, ESC.
/// `None` when `chars` does not start with one.
fn language_escape(chars: &[char]) -> Option<usize> {
    let rest = chars.strip_prefix(&[ESCAPE])?;
    [2, 4].into_iter().find_map(|code_len| {
        let code = rest.get(..code_len)?;
        let closed = rest.get(code_len) == Some(&ESCAPE);
        (closed && code.iter().all(char::is_ascii_alphabetic)).then_some(code_len + 2)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Table D.2 as (code, scalar) pairs for every code outside ASCII and Latin-1 identity.
    const NON_IDENTITY: [(u8, u32); 41] = [
        (0x18, 0x02D8),
        (0x19, 0x02C7),
        (0x1A, 0x02C6),
        (0x1B, 0x02D9),
        (0x1C, 0x02DD),
        (0x1D, 0x02DB),
        (0x1E, 0x02DA),
        (0x1F, 0x02DC),
        (0x80, 0x2022),
        (0x81, 0x2020),
        (0x82, 0x2021),
        (0x83, 0x2026),
        (0x84, 0x2014),
        (0x85, 0x2013),
        (0x86, 0x0192),
        (0x87, 0x2044),
        (0x88, 0x2039),
        (0x89, 0x203A),
        (0x8A, 0x2212),
        (0x8B, 0x2030),
        (0x8C, 0x201E),
        (0x8D, 0x201C),
        (0x8E, 0x201D),
        (0x8F, 0x2018),
        (0x90, 0x2019),
        (0x91, 0x201A),
        (0x92, 0x2122),
        (0x93, 0xFB01),
        (0x94, 0xFB02),
        (0x95, 0x0141),
        (0x96, 0x0152),
        (0x97, 0x0160),
        (0x98, 0x0178),
        (0x99, 0x017D),
        (0x9A, 0x0131),
        (0x9B, 0x0142),
        (0x9C, 0x0153),
        (0x9D, 0x0161),
        (0x9E, 0x017E),
        (0xA0, 0x20AC),
        (0x09, 0x0009),
    ];

    const UNDEFINED: [u8; 4] = [0x7F, 0x9F, 0xAD, 0x00];

    #[test]
    fn decode_maps_every_non_identity_code_as_table_d2() {
        for (code, scalar) in NON_IDENTITY {
            assert_eq!(pdf_doc_decode(code).map(u32::from), Some(scalar), "code {code:#04x}");
        }
    }

    #[test]
    fn decode_is_identity_for_ascii_and_latin1() {
        for code in (0x20..=0x7E).chain(0xA1..=0xAC).chain(0xAE..=0xFF) {
            assert_eq!(pdf_doc_decode(code), Some(char::from(code)), "code {code:#04x}");
        }
    }

    #[test]
    fn decode_leaves_undefined_codes_undefined() {
        let undefined: Vec<u8> = (0..=0xFF).filter(|&c| pdf_doc_decode(c).is_none()).collect();
        let mut expected: Vec<u8> = (0x00..0x09).chain([0x0B, 0x0C]).chain(0x0E..0x18).collect();
        expected.extend([0x7F, 0x9F, 0xAD]);
        assert_eq!(undefined, expected);
    }

    #[test]
    fn encode_reverses_decode_for_every_defined_code() {
        for code in 0..=0xFF_u8 {
            if let Some(c) = pdf_doc_decode(code) {
                assert_eq!(pdf_doc_encode(c), Some(code), "code {code:#04x}");
            }
        }
    }

    #[test]
    fn encode_rejects_scalars_without_a_code() {
        for c in ['\u{AD}', '\u{7F}', '\u{0}', '中', '\u{2023}'] {
            assert_eq!(pdf_doc_encode(c), None, "{c:?}");
        }
    }

    #[test]
    fn decode_text_string_replaces_undefined_pdfdoc_codes() {
        for code in UNDEFINED {
            assert_eq!(decode_text_string(&[b'a', code]), "a\u{FFFD}");
        }
    }

    #[test]
    fn decode_text_string_reads_utf16be_surrogate_pairs() {
        assert_eq!(decode_text_string(b"\xFE\xFF\xD8\x34\xDD\x1E"), "\u{1D11E}");
    }

    #[test]
    fn decode_text_string_replaces_an_unpaired_high_surrogate() {
        assert_eq!(decode_text_string(b"\xFE\xFF\xD8\x34\x00A"), "\u{FFFD}A");
    }

    #[test]
    fn decode_text_string_replaces_an_unpaired_low_surrogate() {
        assert_eq!(decode_text_string(b"\xFE\xFF\xDD\x1E"), "\u{FFFD}");
    }

    #[test]
    fn decode_text_string_replaces_a_trailing_odd_byte() {
        assert_eq!(decode_text_string(b"\xFE\xFF\x00A\x00"), "A\u{FFFD}");
    }

    #[test]
    fn decode_text_string_drops_utf16_language_escapes() {
        assert_eq!(
            decode_text_string(b"\xFE\xFF\x00\x1B\x00e\x00n\x00\x1B\x00H\x00i"),
            "Hi"
        );
    }

    #[test]
    fn decode_text_string_drops_language_and_country_escapes() {
        assert_eq!(
            decode_text_string(b"\xFE\xFF\x00\x1B\x00e\x00n\x00U\x00S\x00\x1B\x00H\x00i"),
            "Hi"
        );
    }

    #[test]
    fn decode_text_string_keeps_text_after_a_stray_escape() {
        assert_eq!(decode_text_string(b"\xFE\xFF\x00\x1B\x00H\x00i"), "\u{FFFD}Hi");
    }

    #[test]
    fn decode_text_string_keeps_text_after_an_overlong_escape() {
        assert_eq!(
            decode_text_string(b"\xFE\xFF\x00\x1B\x00a\x00b\x00c\x00d\x00e\x00\x1B\x00!"),
            "\u{FFFD}abcde\u{FFFD}!"
        );
    }

    #[test]
    fn decode_text_string_reads_utf8_with_bom() {
        assert_eq!(decode_text_string(b"\xEF\xBB\xBF\xE2\x82\xAC"), "€");
    }

    #[test]
    fn decode_text_string_replaces_invalid_utf8() {
        assert_eq!(decode_text_string(b"\xEF\xBB\xBFa\xFFb"), "a\u{FFFD}b");
    }

    #[test]
    fn decode_text_string_of_an_empty_string_is_empty() {
        assert_eq!(decode_text_string(b""), "");
    }

    #[test]
    fn encode_text_string_round_trips_through_decode() {
        for text in [
            "",
            "Title",
            "Café €",
            "• – —",
            "中文",
            "emoji 😀",
            "þÿ",
            "ï»¿x",
            "\u{7F}",
        ] {
            assert_eq!(decode_text_string(&encode_text_string(text)), text, "{text:?}");
        }
    }

    #[test]
    fn encode_text_string_prefers_pdfdoc() {
        assert_eq!(encode_text_string("A€"), b"A\xA0");
    }

    #[test]
    fn decode_text_string_without_a_bom_is_pdfdoc() {
        assert_eq!(decode_text_string(b"\xFE"), "\u{FE}");
    }
}
