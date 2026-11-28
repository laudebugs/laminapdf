//! [`SetEncryption`]: protect the document with passwords and permissions (E11 plan §2.1).
//!
//! The op holds the passwords as [`Secret`]s, which print as `<secret>` and serialise as
//! `null`, so no log, error or crash report carries one. Applying it records the request on the
//! document ([`Document::pending_encryption`](crate::Document::pending_encryption)); the save
//! planner writes the handler, with the entropy the host passes to the save.

use std::fmt;

use laminapdf_syntax::crypt::{Grant, PrintRight};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

use crate::error::{Error, Result};
use crate::op::Effect;

/// A password or key: bytes that overwrite themselves when dropped, print as `<secret>` in
/// `Debug`, and serialise as JSON `null`. An op log that holds one can't replay an encrypted
/// save: reading the `null` back gives a missing secret, and applying an op with one fails with
/// `secret-missing`.
///
/// ```
/// use laminapdf_ops::Secret;
///
/// let secret = Secret::new("hunter2");
/// assert_eq!(format!("{secret:?}"), "<secret>");
/// assert_eq!(serde_json::to_string(&secret)?, "null");
/// assert!(serde_json::from_str::<Secret>("null")?.is_missing());
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// Added in 0.7.0.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
    bytes: Vec<u8>,
    missing: bool,
}

impl Secret {
    /// A secret of `text`'s UTF-8 bytes (an empty one is a password that is not set).
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(text: impl AsRef<str>) -> Self {
        Self {
            bytes: text.as_ref().as_bytes().to_vec(),
            missing: false,
        }
    }

    /// A secret that is not there: what an op log gives back for a password it never stored.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn missing() -> Self {
        Self {
            bytes: Vec::new(),
            missing: true,
        }
    }

    /// Whether the secret was left out ([`Secret::missing`]).
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn is_missing(&self) -> bool {
        self.missing
    }

    /// Whether it holds no bytes.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The bytes, for the code that hashes them. Never log what this returns.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.bytes
    }
}

impl Default for Secret {
    /// A password that is not set.
    fn default() -> Self {
        Self::new("")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<secret>")
    }
}

impl Serialize for Secret {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_none()
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Ok(Option::<String>::deserialize(deserializer)?.map_or_else(Self::missing, Self::new))
    }
}

/// How much printing a protected file allows.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum PrintPermission {
    /// No printing.
    None,
    /// Low-resolution printing only.
    LowResolution,
    /// Printing at full quality.
    #[default]
    HighResolution,
}

/// What a person without the permissions (owner) password may do: the controls of the Protect
/// dialog, mapped to the `/P` bits of ISO 32000-2 Table 22 (E11 plan §2.1). Screen readers are
/// always allowed (bit 10), so a protected file stays readable by assistive technology, and
/// editing the contents (bit 4) is allowed only when no permissions password is set.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[expect(clippy::struct_excessive_bools, reason = "one switch per control of the dialog")]
#[non_exhaustive]
pub struct EncryptionPermissions {
    /// Printing (bits 3 and 12).
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub print: PrintPermission,
    /// Copying text and images (bit 5).
    #[serde(default = "yes")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub copy: bool,
    /// Filling in forms and signing (bit 9).
    #[serde(default = "yes")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub fill_forms: bool,
    /// Adding comments (bit 6).
    #[serde(default = "yes")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub comment: bool,
    /// Inserting, rotating and deleting pages (bit 11).
    #[serde(default = "yes")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub assemble: bool,
}

const fn yes() -> bool {
    true
}

impl Default for EncryptionPermissions {
    /// Everything allowed.
    fn default() -> Self {
        Self {
            print: PrintPermission::HighResolution,
            copy: true,
            fill_forms: true,
            comment: true,
            assemble: true,
        }
    }
}

impl EncryptionPermissions {
    /// The same permissions with printing set to `print`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_print(mut self, print: PrintPermission) -> Self {
        self.print = print;
        self
    }

    /// The same permissions with copying text and images allowed or not.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_copy(mut self, allowed: bool) -> Self {
        self.copy = allowed;
        self
    }

    /// The same permissions with filling in forms and signing allowed or not.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_fill_forms(mut self, allowed: bool) -> Self {
        self.fill_forms = allowed;
        self
    }

    /// The same permissions with adding comments allowed or not.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_comment(mut self, allowed: bool) -> Self {
        self.comment = allowed;
        self
    }

    /// The same permissions with inserting, rotating and deleting pages allowed or not.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_assemble(mut self, allowed: bool) -> Self {
        self.assemble = allowed;
        self
    }

    /// The `/P` grant: editing the contents (bit 4) only when `owner_set` is false.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn grant(&self, owner_set: bool) -> Grant {
        let mut grant = Grant::all();
        grant.print = match self.print {
            PrintPermission::None => PrintRight::None,
            PrintPermission::LowResolution => PrintRight::LowResolution,
            PrintPermission::HighResolution => PrintRight::HighResolution,
        };
        grant.extract = self.copy;
        grant.fill_forms = self.fill_forms;
        grant.annotate = self.comment;
        grant.assemble = self.assemble;
        grant.modify = !owner_set;
        grant
    }
}

/// Protects the document: encrypts it with AES-256 (the standard security handler, revision 6)
/// when it is next saved, with `user` as the password that opens it and `owner` as the
/// permissions password that lifts `permissions`.
///
/// The op needs a full rewrite and is a barrier ([ADR 0006] §5): a save that commits it clears
/// the undo history. The passwords never reach the file or a log. An empty `owner` is replaced
/// by a random password the save reports
/// ([`SaveReport::generated_owner_password`](https://docs.rs/laminapdf-save)), since the handler needs
/// one; both passwords empty is refused (`encryption-no-password`). A file that claims PDF/A is
/// refused too (`encryption-pdfa-conflict`), since PDF/A forbids encryption.
///
/// Applying it to an encrypted document changes its passwords or limits. The save needs
/// [`SaveOptions::with_entropy_seed`](https://docs.rs/laminapdf-save) (`encryption-no-entropy`).
///
/// ```
/// use laminapdf_ops::{EncryptionPermissions, Op, SetEncryption};
///
/// let op = Op::from(SetEncryption::new("open sesame", "owner secret"));
/// let json = serde_json::to_string(&op)?;
/// assert!(!json.contains("sesame") && json.contains(r#""user":null"#));
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetEncryption {
    /// The password that opens the file; empty for none.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(type = "string | null", optional))]
    pub user: Secret,
    /// The permissions password; empty to have one made.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(type = "string | null", optional))]
    pub owner: Secret,
    /// What the file allows a person without the permissions password.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub permissions: EncryptionPermissions,
    /// Whether the XMP metadata is encrypted too (default true); with false, readers that
    /// index a file's metadata without a password still can.
    #[serde(default = "yes")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub encrypt_metadata: bool,
}

impl SetEncryption {
    /// An op with `user` and `owner` passwords (either may be empty, not both), everything
    /// allowed and the metadata encrypted.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(user: impl AsRef<str>, owner: impl AsRef<str>) -> Self {
        Self {
            user: Secret::new(user),
            owner: Secret::new(owner),
            permissions: EncryptionPermissions::default(),
            encrypt_metadata: true,
        }
    }

    /// The same op with `permissions`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_permissions(mut self, permissions: EncryptionPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// The same op, leaving the XMP metadata unencrypted when `encrypt` is false.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_encrypt_metadata(mut self, encrypt: bool) -> Self {
        self.encrypt_metadata = encrypt;
        self
    }

    /// Checks the request and records it. Nothing in the overlay changes, so the document's own
    /// objects stay as they are until the save.
    pub(crate) fn apply_to(&self, doc: &laminapdf_model::Document) -> Result<Effect> {
        if self.user.is_missing() || self.owner.is_missing() {
            return Err(Error::SecretMissing);
        }
        if self.user.is_empty() && self.owner.is_empty() {
            return Err(Error::EncryptionNoPassword);
        }
        if let laminapdf_model::xmp::PacketState::Parsed(packet) = doc.xmp()?
            && packet.pdfa_part().is_some()
        {
            return Err(Error::EncryptionPdfaConflict);
        }
        Ok(Effect {
            encryption: Some(Box::new(self.clone())),
            ..Effect::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_never_prints_or_serialises_its_bytes() {
        let secret = Secret::new("hunter2");
        assert_eq!(format!("{secret:?} {secret:#?}"), "<secret> <secret>");
        assert_eq!(serde_json::to_string(&secret).unwrap(), "null");
    }

    #[test]
    fn a_secret_read_back_from_a_log_is_missing() {
        let secret: Secret = serde_json::from_str("null").unwrap();
        assert!(secret.is_missing() && secret.is_empty());
    }

    #[test]
    fn a_password_sent_by_the_caller_is_kept() {
        let secret: Secret = serde_json::from_str("\"hunter2\"").unwrap();
        assert_eq!((secret.expose(), secret.is_missing()), (&b"hunter2"[..], false));
    }

    #[test]
    fn the_default_secret_is_an_empty_password_that_is_set() {
        assert_eq!((Secret::default().is_empty(), Secret::default().is_missing()), (true, false));
    }

    #[test]
    fn the_permissions_password_turns_off_editing_the_contents() {
        let p = EncryptionPermissions::default();
        let (with, without) = (p.grant(true), p.grant(false));
        assert_eq!((with.modify, without.modify), (false, true));
    }

    #[test]
    fn each_dialog_control_reaches_its_grant_field() {
        let p = EncryptionPermissions::default()
            .with_print(PrintPermission::LowResolution)
            .with_copy(false)
            .with_fill_forms(false)
            .with_comment(false)
            .with_assemble(false);
        let grant = p.grant(false);
        assert_eq!(
            (grant.print, grant.extract, grant.fill_forms, grant.annotate, grant.assemble),
            (PrintRight::LowResolution, false, false, false, false)
        );
    }
}
