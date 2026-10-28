//! [`OpenOptions`]: limits, strict mode and the password (ADR 0003 §3).

use std::fmt;

use lamina_syntax::Limits;

/// How [`Document::open`](crate::Document::open) reads its input.
///
/// Private fields; start from [`OpenOptions::default`] and use the `with_*` builders:
///
/// ```
/// use lamina_cos::{Limits, OpenOptions};
///
/// let options = OpenOptions::default()
///     .with_limits(Limits::default().with_max_stream_decoded(1 << 20))
///     .with_strict(true);
/// assert!(options.strict());
/// assert_eq!(options.limits().max_stream_decoded(), 1 << 20);
/// ```
#[derive(Clone, Default, PartialEq, Eq)]
pub struct OpenOptions {
    limits: Limits,
    strict: bool,
    password: Option<Vec<u8>>,
}

impl OpenOptions {
    /// Sets the resource [`Limits`] (default: [`Limits::default`]).
    #[must_use]
    pub const fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Strict mode (default off): every [`Warning`](crate::Warning) becomes an
    /// [`Error::Strict`](crate::Error::Strict), so a file that needs repair is reported
    /// instead of silently mended (validators, signature verification).
    #[must_use]
    pub const fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// The password for an encrypted file (user or owner).
    ///
    /// Stored for the standard security handler, which arrives in a later round (B2b).
    /// Until then encrypted files open without decryption and
    /// [`Document::is_encrypted`](crate::Document::is_encrypted) reports them.
    #[must_use]
    pub fn with_password(mut self, password: impl Into<Vec<u8>>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// The resource limits.
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Whether strict mode is on.
    #[must_use]
    pub const fn strict(&self) -> bool {
        self.strict
    }

    /// The password, if one was given.
    #[must_use]
    pub fn password(&self) -> Option<&[u8]> {
        self.password.as_deref()
    }
}

impl fmt::Debug for OpenOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never print the password.
        f.debug_struct("OpenOptions")
            .field("limits", &self.limits)
            .field("strict", &self.strict)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_the_password() {
        let options = OpenOptions::default().with_password("hunter2");
        assert!(!format!("{options:?}").contains("hunter2"));
    }

    #[test]
    fn default_is_tolerant_with_default_limits() {
        let options = OpenOptions::default();
        assert_eq!((options.strict(), *options.limits()), (false, Limits::default()));
    }
}
