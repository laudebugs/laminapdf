//! What `cargo xtask wasm` checks in each module it builds, read from the binary itself: the
//! memory's maximum (ADR 0016's cap, set at link time) and the exports (a shipped module has no
//! test hooks; the test build has them all).

use anyhow::{Context, Result, bail};

/// The exports `laminapdf-wasm`'s `test-hooks` feature adds.
pub(crate) const TEST_HOOKS: [&str; 2] = ["__trap", "__grow"];

/// The WebAssembly page size, in bytes.
const PAGE: u64 = 65_536;

/// What a module declares about itself.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Declared {
    /// The export names, in order.
    pub(crate) exports: Vec<String>,
    /// The maximum of the first memory the module defines, in bytes; `None` without one.
    pub(crate) max_memory: Option<u64>,
}

/// Fails unless the module declares `max_memory` as its memory's maximum, and every test hook
/// (`hooks`) or none of them.
pub(crate) fn check(name: &str, bytes: &[u8], max_memory: u64, hooks: bool) -> Result<()> {
    let declared = declared(bytes).with_context(|| format!("reading the {name} module"))?;
    if declared.max_memory != Some(max_memory) {
        bail!(
            "the {name} module's maximum memory is {:?} bytes, not {max_memory}: check the --max-memory link argument",
            declared.max_memory
        );
    }
    for hook in TEST_HOOKS {
        let exported = declared.exports.iter().any(|e| e == hook);
        if exported != hooks {
            bail!(
                "the {name} module {} `{hook}`: test hooks belong only in the test build",
                if exported { "exports" } else { "lacks" }
            );
        }
    }
    Ok(())
}

/// Reads the exports and the memory's maximum from a module's bytes.
pub(crate) fn declared(bytes: &[u8]) -> Result<Declared> {
    let mut r = Reader { bytes, at: 0 };
    if r.take(8)? != b"\0asm\x01\0\0\0" {
        bail!("not a WebAssembly module (version 1)");
    }
    let mut out = Declared {
        exports: Vec::new(),
        max_memory: None,
    };
    while r.at < bytes.len() {
        let id = r.byte()?;
        let len = usize::try_from(r.leb()?)?;
        let mut section = Reader {
            bytes: r.take(len)?,
            at: 0,
        };
        match id {
            5 => out.max_memory = section.first_memory_max()?,
            7 => out.exports = section.export_names()?,
            _ => {}
        }
    }
    Ok(out)
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

    /// The export section: each export's name.
    fn export_names(&mut self) -> Result<Vec<String>> {
        let count = self.leb()?;
        let mut names = Vec::new();
        for _ in 0..count {
            let len = usize::try_from(self.leb()?)?;
            names.push(String::from_utf8_lossy(self.take(len)?).into_owned());
            let _kind = self.byte()?;
            let _index = self.leb()?;
        }
        Ok(names)
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

    /// A module with one memory (64 KiB pages: `initial`, and `max` when given) and functions
    /// exported under `exports`.
    fn module(max: Option<u64>, exports: &[&str]) -> Vec<u8> {
        let mut memory = vec![1, u8::from(max.is_some())];
        memory.extend(leb(1));
        if let Some(max) = max {
            memory.extend(leb(max));
        }
        let mut export = leb(exports.len() as u64);
        for name in exports {
            export.extend(leb(name.len() as u64));
            export.extend(name.as_bytes());
            export.extend([0, 0]);
        }
        let mut out = b"\0asm\x01\0\0\0".to_vec();
        for (id, payload) in [(0_u8, b"\x04name".to_vec()), (5, memory), (7, export)] {
            out.push(id);
            out.extend(leb(payload.len() as u64));
            out.extend(payload);
        }
        out
    }

    #[test]
    fn the_exports_and_the_maximum_should_be_read() {
        let declared = declared(&module(Some(32_768), &["memory", "allocationFailed"])).ok();
        assert_eq!(
            declared,
            Some(Declared {
                exports: vec!["memory".to_owned(), "allocationFailed".to_owned()],
                max_memory: Some(2 << 30),
            })
        );
    }

    #[test]
    fn a_shipped_module_with_the_cap_and_no_hooks_should_pass() {
        assert!(check("base", &module(Some(32_768), &["memory"]), 2 << 30, false).is_ok());
    }

    #[test]
    fn a_shipped_module_exporting_a_test_hook_should_fail() {
        let error = check("base", &module(Some(32_768), &["__trap"]), 2 << 30, false).map_err(|e| e.to_string());
        assert_eq!(
            error,
            Err("the base module exports `__trap`: test hooks belong only in the test build".to_owned())
        );
    }

    #[test]
    fn the_test_build_without_every_hook_should_fail() {
        assert!(check("hooks", &module(Some(1024), &["__trap"]), 1024 * PAGE, true).is_err());
        assert!(check("hooks", &module(Some(1024), &TEST_HOOKS), 1024 * PAGE, true).is_ok());
    }

    #[test]
    fn a_module_without_the_cap_should_fail() {
        assert!(check("base", &module(None, &[]), 2 << 30, false).is_err());
        assert!(check("base", &module(Some(16_384), &[]), 2 << 30, false).is_err());
    }

    #[test]
    fn a_truncated_module_should_be_an_error() {
        let bytes = module(Some(32_768), &["memory"]);
        assert!(declared(bytes.get(..bytes.len() - 3).unwrap_or_default()).is_err());
        assert!(declared(b"\0asm\x02\0\0\0").is_err());
    }
}
