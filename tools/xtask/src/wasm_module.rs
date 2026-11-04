//! What `cargo xtask wasm` checks in each module it builds, read from the binary itself: the
//! memory's maximum, ADR 0016 §8's ceiling, set at link time.

use anyhow::{Context, Result, bail};

/// The WebAssembly page size, in bytes.
const PAGE: u64 = 65_536;

/// Fails unless the module declares `max_memory` as its memory's maximum.
pub(crate) fn check(name: &str, bytes: &[u8], max_memory: u64) -> Result<()> {
    let declared = max_memory_of(bytes).with_context(|| format!("reading the {name} module"))?;
    if declared != Some(max_memory) {
        bail!(
            "the {name} module's maximum memory is {declared:?} bytes, not {max_memory}: check the --max-memory link argument"
        );
    }
    Ok(())
}

/// The maximum of the first memory the module defines, in bytes; `None` without one.
fn max_memory_of(bytes: &[u8]) -> Result<Option<u64>> {
    let mut r = Reader { bytes, at: 0 };
    if r.take(8)? != b"\0asm\x01\0\0\0" {
        bail!("not a WebAssembly module (version 1)");
    }
    let mut max = None;
    while r.at < bytes.len() {
        let id = r.byte()?;
        let len = usize::try_from(r.leb()?)?;
        let mut section = Reader {
            bytes: r.take(len)?,
            at: 0,
        };
        if id == 5 {
            max = section.first_memory_max()?;
        }
    }
    Ok(max)
}

/// A cursor over a module's bytes.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|end| *end <= self.bytes.len());
        let Some(end) = end else { bail!("the module ends early") };
        let out = self.bytes.get(self.at..end).context("the module ends early")?;
        self.at = end;
        Ok(out)
    }

    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?.first().copied().unwrap_or_default())
    }

    /// An unsigned LEB128 number.
    fn leb(&mut self) -> Result<u64> {
        let mut value = 0_u64;
        for shift in (0..64).step_by(7) {
            let byte = self.byte()?;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        bail!("a number in the module is too long")
    }

    /// The memory section: the first memory's maximum, in bytes.
    fn first_memory_max(&mut self) -> Result<Option<u64>> {
        if self.leb()? == 0 {
            return Ok(None);
        }
        let flags = self.byte()?;
        let _initial = self.leb()?;
        if flags & 1 == 0 {
            return Ok(None);
        }
        Ok(self.leb()?.checked_mul(PAGE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leb(mut n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = u8::try_from(n & 0x7f).unwrap_or_default();
            n >>= 7;
            if n == 0 {
                out.push(byte);
                return out;
            }
            out.push(byte | 0x80);
        }
    }

    /// A module with a name section and one memory of 64 KiB pages: one page initially, and
    /// `max` pages at most when given.
    fn module(max: Option<u64>) -> Vec<u8> {
        let mut memory = vec![1, u8::from(max.is_some())];
        memory.extend(leb(1));
        if let Some(max) = max {
            memory.extend(leb(max));
        }
        let mut out = b"\0asm\x01\0\0\0".to_vec();
        for (id, payload) in [(0_u8, b"\x04name".to_vec()), (5, memory)] {
            out.push(id);
            out.extend(leb(payload.len() as u64));
            out.extend(payload);
        }
        out
    }

    #[test]
    fn the_maximum_should_be_read_in_bytes() {
        assert_eq!(max_memory_of(&module(Some(32_768))).ok(), Some(Some(2 << 30)));
    }

    #[test]
    fn a_module_with_the_maximum_should_pass() {
        assert!(check("base", &module(Some(32_768)), 2 << 30).is_ok());
    }

    #[test]
    fn a_module_without_the_maximum_should_fail() {
        let error = check("base", &module(None), 2 << 30).map_err(|e| e.to_string());
        assert_eq!(
            error,
            Err(
                "the base module's maximum memory is None bytes, not 2147483648: check the --max-memory link argument"
                    .to_owned()
            )
        );
        assert!(check("base", &module(Some(16_384)), 2 << 30).is_err());
    }

    #[test]
    fn a_truncated_module_should_be_an_error() {
        let bytes = module(Some(32_768));
        assert!(max_memory_of(bytes.get(..bytes.len() - 3).unwrap_or_default()).is_err());
        assert!(max_memory_of(b"\0asm\x02\0\0\0").is_err());
    }
}
