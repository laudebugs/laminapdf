//! The filters that turn text back into bytes, and run-length: `ASCIIHexDecode`,
//! `ASCII85Decode` and `RunLengthDecode` (ISO 32000-2 §7.4.2, §7.4.3, §7.4.5).

use super::{Output, bad};
use crate::error::Result;
use crate::lexer::hex_value;

/// `ASCIIHexDecode`: pairs of hex digits, white space ignored, `>` ends the data; a last
/// unpaired digit counts as followed by 0. Any other character is an error to ISO 32000-2
/// §7.4.2; it is skipped, as pdf.js and poppler skip it, and noted on `out` (ADR 0003 §1).
pub(super) fn ascii_hex(data: &[u8], out: &mut Output) -> Result<()> {
    let mut high: Option<u8> = None;
    for &byte in data {
        if byte == b'>' {
            break;
        }
        if byte.is_ascii_whitespace() || byte == 0 {
            continue;
        }
        let Some(digit) = hex_value(byte) else {
            out.skip();
            continue;
        };
        match high.take() {
            None => high = Some(digit),
            Some(h) => out.push(h << 4 | digit)?,
        }
    }
    if let Some(h) = high {
        out.push(h << 4)?;
    }
    Ok(())
}

/// `ASCII85Decode`: groups of five characters `!`–`u` for four bytes, `z` for four zeros,
/// white space ignored, `~>` ends the data; a last group of two to four characters gives one
/// to three bytes. A character outside the alphabet, and a last group of one character, are
/// errors to ISO 32000-2 §7.4.3; they are skipped, as pdf.js skips them, and noted on `out`.
pub(super) fn ascii85(data: &[u8], out: &mut Output) -> Result<()> {
    let mut group = [0u8; 5];
    let mut len = 0;
    for &byte in data {
        match byte {
            b'~' => break,
            b'z' if len == 0 => out.extend(&[0; 4])?,
            b'!'..=b'u' => {
                if let Some(slot) = group.get_mut(len) {
                    *slot = byte - b'!';
                }
                len += 1;
                if len == 5 {
                    out.extend(&group_value(group)?.to_be_bytes())?;
                    len = 0;
                }
            }
            _ if byte.is_ascii_whitespace() || byte == 0 => {}
            _ => out.skip(),
        }
    }
    match len {
        0 => Ok(()),
        1 => {
            out.skip();
            Ok(())
        }
        _ => {
            // Pad with `u` (84), the largest digit, and keep the first len - 1 bytes.
            group.iter_mut().skip(len).for_each(|slot| *slot = 84);
            let bytes = group_value(group)?.to_be_bytes();
            out.extend(bytes.get(..len - 1).unwrap_or_default())
        }
    }
}

fn group_value(group: [u8; 5]) -> Result<u32> {
    let value = group.iter().fold(0u64, |acc, &digit| acc * 85 + u64::from(digit));
    u32::try_from(value).map_err(|_| bad("ASCII85Decode: a group over 2^32"))
}

/// `RunLengthDecode`: a length byte `n` then `n + 1` literal bytes (`n` < 128), or one byte
/// repeated `257 - n` times (`n` > 128); 128 ends the data.
pub(super) fn run_length(data: &[u8], out: &mut Output) -> Result<()> {
    let mut rest = data;
    while let Some((&n, tail)) = rest.split_first() {
        match n {
            128 => break,
            0..=127 => {
                let count = usize::from(n) + 1;
                let literal = tail.get(..count).unwrap_or(tail);
                out.extend(literal)?;
                rest = tail.get(count..).unwrap_or_default();
            }
            _ => {
                let Some((&byte, tail)) = tail.split_first() else {
                    break;
                };
                out.fill(byte, 257 - usize::from(n))?;
                rest = tail;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(filter: fn(&[u8], &mut Output) -> Result<()>, data: &[u8]) -> Result<Vec<u8>> {
        let mut out = Output::new(1 << 20, crate::limits::Limit::StreamDecoded);
        filter(data, &mut out)?;
        Ok(out.into_inner())
    }

    #[test]
    fn ascii_hex_skips_white_space_and_pads_an_odd_digit() {
        assert_eq!(run(ascii_hex, b"48 65\n6c6C6f 7>ignored"), Ok(b"Hello\x70".to_vec()));
    }

    /// The decoded bytes and whether the filter skipped characters.
    fn run_skipping(filter: fn(&[u8], &mut Output) -> Result<()>, data: &[u8]) -> Result<(Vec<u8>, bool)> {
        let mut out = Output::new(1 << 20, crate::limits::Limit::StreamDecoded);
        filter(data, &mut out)?;
        let skipped = out.skipped;
        Ok((out.into_inner(), skipped))
    }

    #[test]
    fn ascii_hex_skips_a_character_that_is_not_a_hex_digit_and_says_so() {
        assert_eq!(run_skipping(ascii_hex, b"4G8 6@5>"), Ok((b"He".to_vec(), true)));
    }

    #[test]
    fn ascii_hex_says_nothing_was_skipped_for_clean_data() {
        assert_eq!(run_skipping(ascii_hex, b"48 65>"), Ok((b"He".to_vec(), false)));
    }

    #[test]
    fn ascii85_skips_a_character_outside_its_alphabet_and_says_so() {
        // "Man " is 9jqo^; `{` (past `u`) is skipped.
        assert_eq!(run_skipping(ascii85, b"9j{qo^~>"), Ok((b"Man ".to_vec(), true)));
    }

    #[test]
    fn ascii85_drops_a_last_group_of_one_character_and_says_so() {
        assert_eq!(run_skipping(ascii85, b"9jqo^9~>"), Ok((b"Man ".to_vec(), true)));
    }

    #[test]
    fn ascii85_decodes_groups_zeros_and_a_short_last_group() {
        // "Man " is 9jqo^; four zeros are z; "su" is F*. (a group of three).
        assert_eq!(run(ascii85, b"9jqo^ z F*.~>"), Ok(b"Man \0\0\0\0su".to_vec()));
    }

    #[test]
    fn ascii85_refuses_a_group_over_two_to_the_32() {
        assert!(run(ascii85, b"uuuuu~>").is_err());
    }

    #[test]
    fn run_length_repeats_and_copies_until_128() {
        assert_eq!(
            run(run_length, &[2, b'a', b'b', b'c', 254, b'x', 128, 0, b'q']),
            Ok(b"abcxxx".to_vec())
        );
    }

    #[test]
    fn run_length_keeps_a_cut_literal() {
        assert_eq!(run(run_length, &[5, b'a', b'b']), Ok(b"ab".to_vec()));
    }
}
