//! What a file's protection is ([`Document::encryption_info`], G.11), for the inspector and the
//! status bar. Nothing here returns a key or a password.

use laminapdf_cos::Cipher;
use laminapdf_syntax::crypt::PasswordKind;

use crate::document::Document;
use crate::properties::DocumentPermissions;

/// Which password opened an encrypted file.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OpenedWith {
    /// The user ("open") password, which includes the empty one a file opens with unprompted.
    User,
    /// The owner ("permissions") password, which lifts the file's limits.
    Owner,
}

/// A file's security handler as the file states it (ISO 32000-2 §7.6): what the encryption
/// inspector shows. The permission bits are what the file asks readers to honour, not access
/// control (research ch. 04 §7.5): anyone who can open the file can read and copy it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EncryptionInfo {
    /// `/Filter`: always `Standard` here, since Lamina PDF reads no other handler yet.
    pub handler: &'static str,
    /// `/V`: the algorithm version (1, 2, 4 or 5).
    pub version: u8,
    /// `/R`: the revision (2 to 6).
    pub revision: u8,
    /// The file key length in bits.
    pub key_bits: u16,
    /// The cipher strings use.
    pub string_cipher: Cipher,
    /// The cipher streams use.
    pub stream_cipher: Cipher,
    /// Whether the XMP metadata is encrypted (`/EncryptMetadata`).
    pub encrypt_metadata: bool,
    /// What the file permits, bit by bit (`/P`).
    pub permissions: DocumentPermissions,
    /// `/P` as the file stores it, a 32-bit two's-complement value read unsigned.
    pub permission_bits: u32,
    /// Which password opened the file.
    pub opened_with: OpenedWith,
}

impl Document {
    /// The file's encryption: handler, revision, ciphers, key length, what `/EncryptMetadata`
    /// says, the permission bits and which password opened it. `None` for an unencrypted file.
    /// Key material is never part of it.
    ///
    /// ```
    /// use laminapdf::{Document, OpenOptions};
    ///
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    /// assert!(doc.encryption_info().is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn encryption_info(&self) -> Option<EncryptionInfo> {
        let model = self.ops();
        let encryption = model.encryption()?;
        let permissions = model.permissions()?;
        Some(EncryptionInfo {
            handler: "Standard",
            version: encryption.version,
            revision: encryption.revision,
            key_bits: encryption.key_bits,
            string_cipher: encryption.string_cipher,
            stream_cipher: encryption.stream_cipher,
            encrypt_metadata: encryption.encrypt_metadata,
            permissions: DocumentPermissions {
                print: permissions.print_low_res(),
                print_high_quality: permissions.print_high_res(),
                modify: permissions.modify(),
                copy: permissions.extract(),
                annotate: permissions.annotate(),
                fill_forms: permissions.fill_forms(),
                accessibility: permissions.accessibility(),
                assemble: permissions.assemble(),
            },
            permission_bits: permissions.raw(),
            opened_with: match model.matched_password()? {
                PasswordKind::Owner => OpenedWith::Owner,
                _ => OpenedWith::User,
            },
        })
    }

    /// Whether a [`SetEncryption`](crate::SetEncryption) is waiting for the next save: the file
    /// will be written protected. The passwords are not readable from the document.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn has_pending_encryption(&self) -> bool {
        self.ops().pending_encryption().is_some()
    }
}
