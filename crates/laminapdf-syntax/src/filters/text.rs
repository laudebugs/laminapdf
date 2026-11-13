//! The filters that turn text back into bytes, and run-length: `ASCIIHexDecode`,
//! `ASCII85Decode` and `RunLengthDecode` (ISO 32000-2 §7.4.2, §7.4.3, §7.4.5).

use super::Output;
use crate::error::{Error, Result};

const fn bad(message: &'static str) -> Error {
    Error::InvalidStructure {
        kind: "stream filter",
        message,
    }
}

/// `ASCIIHexDecode`: pairs of hex digits, white space ignored, `>` ends the data; a last
/// unpaired digit counts as followed by 0.
pub(super) fn ascii_hex(data: &[u8], out: &mut Output) -> Result<()> {
    let mut high: Option<u8> = None;
    for &byte in data {
        if byte == b'>' {
            break;
        }
        if byte.is_ascii_whitespace() || byte == 0 {
            continue;
        }
        let digit = hex_digit(byte).ok_or(bad("ASCIIHexDecode: not a hex digit"))?;
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

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// `ASCII85Decode`: groups of five characters `!`–`u` for four bytes, `z` for four zeros,
/// white space ignored, `~>` ends the data; a last group of two to four characters gives one
/// to three bytes.
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
            _ => return Err(bad("ASCII85Decode: a character outside the alphabet")),
        }
    }
    match len {
        0 => Ok(()),
        1 => Err(bad("ASCII85Decode: a last group of one character")),
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

    #[test]
    fn ascii_hex_refuses_a_letter_past_f() {
        assert!(run(ascii_hex, b"4G").is_err());
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
