//! A file's protection across the boundary: `WasmDocument.encryptionInfo()` (G.11) and
//! `hasPendingEncryption()`. Base module: reading the handler needs no extra module.

use laminapdf::{Cipher, EncryptionInfo, OpenedWith};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;

/// The cipher a file's strings or streams use (ISO 32000-2 §7.6).
#[derive(Debug, Serialize, Tsify)]
pub(crate) enum EncryptionCipherJs {
    /// RC4 (deprecated).
    #[serde(rename = "rc4")]
    Rc4,
    /// AES with a 128-bit key (deprecated in PDF 2.0).
    #[serde(rename = "aes-128")]
    Aes128,
    /// AES with a 256-bit key.
    #[serde(rename = "aes-256")]
    Aes256,
    /// Not encrypted (`/Identity`).
    #[serde(rename = "identity")]
    Identity,
    /// A cipher this version doesn't name.
    #[serde(rename = "other")]
    Other,
}

impl From<Cipher> for EncryptionCipherJs {
    fn from(cipher: Cipher) -> Self {
        match cipher {
            Cipher::Rc4 => Self::Rc4,
            Cipher::Aes128 => Self::Aes128,
            Cipher::Aes256 => Self::Aes256,
            Cipher::Identity => Self::Identity,
            // A cipher a later version adds still encrypts: it is never reported as Identity.
            _ => Self::Other,
        }
    }
}

/// Which password opened the file.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum OpenedWithJs {
    /// The user ("open") password, including the empty one a file opens with unprompted.
    User,
    /// The owner ("permissions") password.
    Owner,
}

/// What a file allows, bit by bit: a request to readers, not a lock.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[expect(clippy::struct_excessive_bools, reason = "one flag per permission bit")]
pub(crate) struct EncryptionPermissionsJs {
    /// Printing (bit 3).
    pub print: bool,
    /// Printing at full quality (bits 3 and 12).
    pub print_high_quality: bool,
    /// Changing the contents (bit 4).
    pub modify: bool,
    /// Copying text and graphics (bit 5).
    pub copy: bool,
    /// Adding or changing annotations (bit 6).
    pub annotate: bool,
    /// Filling form fields (bits 6 and 9).
    pub fill_forms: bool,
    /// Extracting text for accessibility (bit 10).
    pub accessibility: bool,
    /// Inserting, rotating or deleting pages (bit 11).
    pub assemble: bool,
}

/// A file's security handler: what the encryption inspector shows. No key material.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EncryptionInfoJs {
    /// `/Filter`: `Standard`.
    pub handler: &'static str,
    /// `/V`.
    pub version: u8,
    /// `/R`.
    pub revision: u8,
    /// The file key length in bits.
    pub key_bits: u16,
    /// The cipher strings use.
    pub string_cipher: EncryptionCipherJs,
    /// The cipher streams use.
    pub stream_cipher: EncryptionCipherJs,
    /// Whether the XMP metadata is encrypted (`/EncryptMetadata`).
    pub encrypt_metadata: bool,
    /// What `/P` permits.
    pub permissions: EncryptionPermissionsJs,
    /// `/P` as the file stores it, read unsigned.
    pub permission_bits: u32,
    /// Which password opened the file.
    pub opened_with: OpenedWithJs,
}

impl From<EncryptionInfo> for EncryptionInfoJs {
    fn from(info: EncryptionInfo) -> Self {
        let p = info.permissions;
        Self {
            handler: info.handler,
            version: info.version,
            revision: info.revision,
            key_bits: info.key_bits,
            string_cipher: info.string_cipher.into(),
            stream_cipher: info.stream_cipher.into(),
            encrypt_metadata: info.encrypt_metadata,
            permissions: EncryptionPermissionsJs {
                print: p.print,
                print_high_quality: p.print_high_quality,
                modify: p.modify,
                copy: p.copy,
                annotate: p.annotate,
                fill_forms: p.fill_forms,
                accessibility: p.accessibility,
                assemble: p.assemble,
            },
            permission_bits: info.permission_bits,
            opened_with: match info.opened_with {
                OpenedWith::Owner => OpenedWithJs::Owner,
                _ => OpenedWithJs::User,
            },
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The file's encryption (handler, revision, ciphers, key length, permission bits and which
    /// password opened it), or `undefined` for a file that isn't encrypted. Never a key.
    #[wasm_bindgen(js_name = encryptionInfo, unchecked_return_type = "EncryptionInfoJs | undefined")]
    #[must_use]
    pub fn encryption_info(&self) -> JsValue {
        self.shared()
            .borrow()
            .encryption_info()
            .map_or(JsValue::UNDEFINED, |info| to_js(&EncryptionInfoJs::from(info)))
    }

    /// Whether a `setEncryption` edit waits for the next save.
    #[wasm_bindgen(js_name = hasPendingEncryption)]
    #[must_use]
    pub fn has_pending_encryption(&self) -> bool {
        self.shared().borrow().has_pending_encryption()
    }
}
