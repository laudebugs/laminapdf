//! A plain release version, `MAJOR.MINOR.PATCH` with no pre-release or build, as the web reader
//! (`web_release.rs`) and the desktop app (`desktop_release/plan.rs`) tag their releases.

use std::fmt;

/// `MAJOR.MINOR.PATCH`, ordered as semver orders plain versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version(pub u64, pub u64, pub u64);

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// One numeric identifier: digits only, and no leading zero (semver's rule).
pub(crate) fn number(part: &str) -> Option<u64> {
    (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()) && (part == "0" || !part.starts_with('0')))
        .then(|| part.parse().ok())
        .flatten()
}

/// `text` as a plain version, or `None` for anything else (a pre-release or build suffix too).
pub(crate) fn parse(text: &str) -> Option<Version> {
    let mut parts = text.split('.');
    let mut next = || parts.next().and_then(number);
    let version = Version(next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_versions_should_parse_and_others_should_not() {
        assert_eq!(parse("0.10.3"), Some(Version(0, 10, 3)));
        for bad in ["0.1", "0.1.0.1", "01.0.0", "0.1.0-rc.1", "0.1.0+b", "", "a.b.c"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }
}
