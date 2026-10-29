//! Standard security handler decryption (ISO 32000-2 §7.6; ISO 32000-1 §7.6.3 for R2–R4):
//! password authentication for revisions 2–6 and string/stream decryption with RC4,
//! AES-128-CBC and AES-256-CBC.
//!
//! **Read only.** RC4 and MD5 are broken primitives that lamina only ever *reads*: nothing here
//! encrypts. Writing encrypted files is full-editor E11 (AES-256 / R6 only).
//!
//! The file structure layer (`lamina-cos`) parses `/Encrypt`, fills a [`StandardSecurity`],
//! calls [`authenticate`] and then [`decrypt`]s each string and stream with the
//! [`FileKey`] it gets back.
//!
//! ```
//! use lamina_syntax::crypt::{Cipher, decrypt, FileKey};
//!
//! // `/Identity` passes data through unchanged.
//! let key = FileKey::from_bytes(vec![0; 16]);
//! let out = decrypt(Cipher::Identity, &key, 1, 0, b"plain");
//! assert_eq!((out.data.as_slice(), out.well_formed), (&b"plain"[..], true));
//! ```
//!
//! Sources: research ch. 04 §7 (`docs/research/pdf/04-interactive-security.md`), the spec
//! sections named on each item.

mod aes;
mod auth;
mod key;
mod perms;
mod rc4;

use std::fmt;

pub use perms::Permissions;

/// A crypt filter method (`/CFM`, ISO 32000-2 Table 25) or the pre-V4 implicit cipher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Cipher {
    /// `/Identity` (or `/CFM /None`): the data is not encrypted.
    Identity,
    /// RC4 with a per-object key (`/V2`, and every V1/V2 handler). Deprecated.
    Rc4,
    /// AES-128-CBC with a per-object key (`/AESV2`). Deprecated in PDF 2.0.
    Aes128,
    /// AES-256-CBC with the file key (`/AESV3`, V5).
    Aes256,
}

impl Cipher {
    /// A short stable name: `"identity"`, `"rc4"`, `"aes-128"` or `"aes-256"`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Rc4 => "rc4",
            Self::Aes128 => "aes-128",
            Self::Aes256 => "aes-256",
        }
    }
}

/// Which password authenticated the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PasswordKind {
    /// The user ("open") password, possibly the empty one.
    User,
    /// The owner ("permissions") password.
    Owner,
}

/// The values of a standard security handler's `/Encrypt` dictionary that authentication needs
/// (ISO 32000-2 Table 21). The caller extracts them; byte strings are taken as stored.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StandardSecurity<'a> {
    /// `/R`, 2–6.
    pub revision: u8,
    /// The file key length in bytes (`/Length` / 8): 5–16 for R2–R4 (R2 is always 5); ignored for
    /// R5/R6, whose key is 32 bytes.
    pub key_length: usize,
    /// `/O` (32 bytes for R2–R4, 48 for R5/R6).
    pub owner: &'a [u8],
    /// `/U` (32 bytes for R2–R4, 48 for R5/R6).
    pub user: &'a [u8],
    /// `/OE` (R5/R6, 32 bytes; empty otherwise).
    pub owner_encryption: &'a [u8],
    /// `/UE` (R5/R6, 32 bytes; empty otherwise).
    pub user_encryption: &'a [u8],
    /// `/Perms` (R5/R6, 16 bytes), if present.
    pub perms: Option<&'a [u8]>,
    /// `/P` as an unsigned 32-bit value (see [`Permissions`]).
    pub permissions: u32,
    /// The first element of the trailer's `/ID` (empty when absent).
    pub id0: &'a [u8],
    /// `/EncryptMetadata` (default true).
    pub encrypt_metadata: bool,
}

impl StandardSecurity<'_> {
    /// The R2–R4 file key length in bytes, clamped to the 5–16 the spec allows.
    fn key_bytes(&self) -> usize {
        if self.revision == 2 {
            5
        } else {
            self.key_length.clamp(5, 16)
        }
    }
}

impl fmt::Debug for StandardSecurity<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StandardSecurity")
            .field("revision", &self.revision)
            .field("key_length", &self.key_length)
            .field("permissions", &self.permissions)
            .field("encrypt_metadata", &self.encrypt_metadata)
            .finish_non_exhaustive()
    }
}

/// The file encryption key. Its bytes are never printed.
#[derive(Clone, PartialEq, Eq)]
pub struct FileKey(Vec<u8>);

impl FileKey {
    /// A key from raw bytes (5–16 for RC4/AES-128 handlers, 32 for AES-256).
    #[must_use]
    pub const fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The key length in bits.
    #[must_use]
    pub fn bits(&self) -> usize {
        self.0.len().saturating_mul(8)
    }
}

impl fmt::Debug for FileKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileKey(<{} bits redacted>)", self.bits())
    }
}

/// A successful [`authenticate`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Authenticated {
    /// The file key.
    pub key: FileKey,
    /// Which password matched.
    pub matched: PasswordKind,
    /// R5/R6 with `/Perms`: whether it decrypted to a value consistent with `/P` and
    /// `/EncryptMetadata` (ISO 32000-2 Algorithm 13). `None` when there is nothing to check.
    pub perms_valid: Option<bool>,
}

/// The password byte strings to try for `password` at `revision`.
///
/// R2–R4 passwords are PDFDocEncoding bytes: the input as given, then (if it is UTF-8 text with
/// non-ASCII characters that all fit in Latin-1) its Latin-1 bytes, which PDFDocEncoding
/// shares for most of the range. R5/R6 passwords are UTF-8 after SASLprep (RFC 4013), truncated
/// to 127 bytes (ISO 32000-2 §7.6.4.3.3); input that isn't UTF-8 or that SASLprep rejects is
/// tried as given.
fn candidates(password: &[u8], revision: u8) -> Vec<Vec<u8>> {
    let text = std::str::from_utf8(password).ok();
    if revision >= 5 {
        let prepared = text
            .and_then(|t| stringprep::saslprep(t).ok())
            .map_or_else(|| password.to_vec(), |p| p.as_bytes().to_vec());
        let mut out = vec![key_prefix(&prepared, 127)];
        let raw = key_prefix(password, 127);
        if !out.contains(&raw) {
            out.push(raw);
        }
        return out;
    }
    let mut out = vec![password.to_vec()];
    if let Some(text) = text
        && !text.is_ascii()
        && let Some(latin1) = text
            .chars()
            .map(|c| u8::try_from(u32::from(c)).ok())
            .collect::<Option<Vec<u8>>>()
    {
        out.push(latin1);
    }
    out
}

fn key_prefix(bytes: &[u8], n: usize) -> Vec<u8> {
    bytes.get(..n).unwrap_or(bytes).to_vec()
}

/// Authenticates `password` as the user password, then as the owner password (ISO 32000-1
/// Algorithms 6–7 for R2–R4; ISO 32000-2 Algorithms 11–12 with 2.A/2.B for R6; the single
/// SHA-256 of Adobe Extension Level 3 for R5, which was never standardised and is read only).
///
/// Returns `None` when neither matches, or for a revision outside 2–6 or byte strings too
/// short for it.
#[must_use]
pub fn authenticate(sec: &StandardSecurity<'_>, password: &[u8]) -> Option<Authenticated> {
    let modern = match sec.revision {
        2..=4 => false,
        5 | 6 => true,
        _ => return None,
    };
    let user = |pw: &[u8]| {
        if modern {
            auth::user_r5_r6(sec, pw)
        } else {
            auth::user_r2_r4(sec, pw)
        }
    };
    let owner = |pw: &[u8]| {
        if modern {
            auth::owner_r5_r6(sec, pw)
        } else {
            auth::owner_r2_r4(sec, pw)
        }
    };
    let candidates = candidates(password, sec.revision);
    let found = candidates
        .iter()
        .find_map(|pw| user(pw).map(|k| (k, PasswordKind::User)))
        .or_else(|| {
            candidates
                .iter()
                .find_map(|pw| owner(pw).map(|k| (k, PasswordKind::Owner)))
        });
    let (key, matched) = found?;
    let perms_valid = (sec.revision >= 5)
        .then_some(sec.perms)
        .flatten()
        .map(|perms| perms::perms_match(&key, perms, sec.permissions, sec.encrypt_metadata));
    Some(Authenticated {
        key: FileKey(key),
        matched,
        perms_valid,
    })
}

/// Decrypted data and whether the input was well-formed for its cipher.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Decrypted {
    /// The plaintext (or, for malformed AES input, the bytes as they were; see `well_formed`).
    pub data: Vec<u8>,
    /// False for AES data that was not `IV ‖ whole blocks` (kept as is) or had invalid PKCS#5
    /// padding (decrypted, kept unstripped). The reader reports it as `aes-padding-invalid`.
    pub well_formed: bool,
}

/// Decrypts the string or stream data of object `num gen` (ISO 32000-1 §7.6.2 Algorithm 1 for
/// the per-object keys of RC4 and AES-128; AES-256 uses the file key itself).
#[must_use]
pub fn decrypt(cipher: Cipher, key: &FileKey, num: u32, gen_nr: u16, data: &[u8]) -> Decrypted {
    let plain = match cipher {
        Cipher::Identity => aes::Plain {
            data: data.to_vec(),
            valid: true,
        },
        Cipher::Rc4 => aes::Plain {
            data: rc4::apply(&key::object_key(&key.0, num, gen_nr, false), data),
            valid: true,
        },
        Cipher::Aes128 => aes::decrypt(&key::object_key(&key.0, num, gen_nr, true), data),
        Cipher::Aes256 => aes::decrypt(&key.0, data),
    };
    Decrypted {
        data: plain.data,
        well_formed: plain.valid,
    }
}

#[cfg(test)]
mod tests;
