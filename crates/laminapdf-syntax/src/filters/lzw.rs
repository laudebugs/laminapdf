//! `LZWDecode` (ISO 32000-2 §7.4.4): codes of 9 to 12 bits, most significant bit first; 256
//! clears the table, 257 ends the data. With `/EarlyChange 1` (the default) the code width
//! grows one code early, as the encoders PDF's LZW comes from do.

use super::Output;
use crate::error::{Error, Result};

const CLEAR: u16 = 256;
const END: u16 = 257;
const FIRST: u16 = 258;
const MAX_CODES: usize = 4096;

/// The decoder's string table: each code is a previous code plus one byte, so a string is
/// read back by following `prefix` to a single byte.
struct Table {
    prefix: Vec<u16>,
    last: Vec<u8>,
    first: Vec<u8>,
    len: Vec<u16>,
}

impl Table {
    fn new() -> Self {
        let mut table = Self {
            prefix: Vec::with_capacity(MAX_CODES),
            last: Vec::with_capacity(MAX_CODES),
            first: Vec::with_capacity(MAX_CODES),
            len: Vec::with_capacity(MAX_CODES),
        };
        table.reset();
        table
    }

    fn reset(&mut self) {
        self.prefix.clear();
        self.last.clear();
        self.first.clear();
        self.len.clear();
        for byte in 0..=255u8 {
            self.prefix.push(u16::MAX);
            self.last.push(byte);
            self.first.push(byte);
            self.len.push(1);
        }
        // CLEAR and END take two places that hold no string.
        for _ in 0..2 {
            self.prefix.push(u16::MAX);
            self.last.push(0);
            self.first.push(0);
            self.len.push(0);
        }
    }

    fn next_code(&self) -> usize {
        self.prefix.len()
    }

    fn add(&mut self, prefix: u16, byte: u8) {
        if self.next_code() >= MAX_CODES {
            return;
        }
        let at = usize::from(prefix);
        let first = self.first.get(at).copied().unwrap_or(byte);
        let len = self.len.get(at).copied().unwrap_or(0).saturating_add(1);
        self.prefix.push(prefix);
        self.last.push(byte);
        self.first.push(first);
        self.len.push(len);
    }

    /// Writes code `code`'s string to `out`.
    fn write(&self, code: u16, out: &mut Output) -> Result<()> {
        let len = usize::from(self.len.get(usize::from(code)).copied().unwrap_or(0));
        let start = out.len();
        out.fill(0, len)?;
        let mut at = code;
        for i in (0..len).rev() {
            let index = usize::from(at);
            out.set(start + i, self.last.get(index).copied().unwrap_or(0));
            at = self.prefix.get(index).copied().unwrap_or(u16::MAX);
        }
        Ok(())
    }
}

/// Reads codes of a given width from the data, most significant bit first.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
    buffer: u32,
    count: u32,
}

impl<'a> Bits<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            at: 0,
            buffer: 0,
            count: 0,
        }
    }

    fn read(&mut self, width: u32) -> Option<u16> {
        while self.count < width {
            let byte = *self.data.get(self.at)?;
            self.at += 1;
            self.buffer = self.buffer << 8 | u32::from(byte);
            self.count += 8;
        }
        self.count -= width;
        let code = (self.buffer >> self.count) & ((1 << width) - 1);
        u16::try_from(code).ok()
    }
}

/// Decodes LZW `data` into `out`; `early` is `/EarlyChange` (1 by default).
pub(super) fn decode(data: &[u8], early: bool, out: &mut Output) -> Result<()> {
    let mut table = Table::new();
    let mut bits = Bits::new(data);
    let mut previous: Option<u16> = None;
    let mut width = 9;
    while let Some(code) = bits.read(width) {
        match code {
            CLEAR => {
                table.reset();
                previous = None;
                width = 9;
                continue;
            }
            END => break,
            _ => {}
        }
        let next = u16::try_from(table.next_code()).unwrap_or(u16::MAX);
        match previous {
            None => {
                if code >= FIRST {
                    return Err(Error::InvalidStructure {
                        kind: "stream filter",
                        message: "LZWDecode: a first code that isn't a byte",
                    });
                }
                table.write(code, out)?;
            }
            Some(prev) if code < next => {
                table.write(code, out)?;
                let first = table.first.get(usize::from(code)).copied().unwrap_or(0);
                table.add(prev, first);
            }
            Some(prev) if code == next => {
                // The code being defined: the previous string plus its own first byte.
                let first = table.first.get(usize::from(prev)).copied().unwrap_or(0);
                table.add(prev, first);
                table.write(code, out)?;
            }
            Some(_) => {
                return Err(Error::InvalidStructure {
                    kind: "stream filter",
                    message: "LZWDecode: a code past the table",
                });
            }
        }
        previous = Some(code);
        let size = table.next_code() + usize::from(early);
        width = match size {
            ..=511 => 9,
            512..=1023 => 10,
            1024..=2047 => 11,
            _ => 12,
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limit;

    /// Packs `codes` of the given widths, most significant bit first.
    fn pack(codes: &[(u16, u32)]) -> Vec<u8> {
        let mut out = Vec::new();
        let (mut buffer, mut count) = (0u64, 0u32);
        for &(code, width) in codes {
            buffer = buffer << width | u64::from(code);
            count += width;
            while count >= 8 {
                count -= 8;
                out.push(u8::try_from((buffer >> count) & 0xff).unwrap());
            }
        }
        if count > 0 {
            out.push(u8::try_from((buffer << (8 - count)) & 0xff).unwrap());
        }
        out
    }

    fn run(data: &[u8], early: bool) -> Result<Vec<u8>> {
        let mut out = Output::new(1 << 20, Limit::StreamDecoded);
        decode(data, early, &mut out)?;
        Ok(out.into_inner())
    }

    #[test]
    fn decodes_the_spec_example() {
        // ISO 32000-2 §7.4.4.2's example: 45 45 45 45 45 65 45 45 45 66.
        let data = [0x80, 0x0b, 0x60, 0x50, 0x22, 0x0c, 0x0c, 0x85, 0x01];
        assert_eq!(run(&data, true), Ok(vec![45, 45, 45, 45, 45, 65, 45, 45, 45, 66]));
    }

    #[test]
    fn handles_a_code_defined_by_its_own_use() {
        // a, then code 258 ("aa") before 258 is in the table: "aaa".
        let data = pack(&[(CLEAR, 9), (u16::from(b'a'), 9), (258, 9), (END, 9)]);
        assert_eq!(run(&data, true), Ok(b"aaa".to_vec()));
    }

    #[test]
    fn refuses_a_code_past_the_table() {
        let data = pack(&[(u16::from(b'a'), 9), (300, 9)]);
        assert!(run(&data, true).is_err());
    }
}
