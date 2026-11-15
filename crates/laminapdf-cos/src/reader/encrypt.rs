//! The standard security handler on the reading side (ISO 32000-2 §7.6; research ch. 04 §7):
//! parses `/Encrypt`, authenticates the password and decrypts strings and streams as objects
//! are loaded. Read only: writing encrypted files is full-editor E11.

use std::collections::BTreeMap;
use std::ops::BitAnd;

use laminapdf_syntax::crypt::{self, Cipher, FileKey, PasswordKind, Permissions, StandardSecurity};
use laminapdf_syntax::{Dict, Object, Stream};

use crate::error::{Error, Result};

/// What a document's encryption is, from its `/Encrypt` dictionary
/// ([`Document::encryption`](crate::Document::encryption)).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Encryption {
    /// `/V`: the algorithm version (1, 2, 4 or 5).
    pub version: u8,
    /// `/R`: the standard security handler revision (2–6). R5 is Adobe Extension Level 3's
    /// handler, never standardised and read only.
    pub revision: u8,
    /// The cipher strings use (`/StrF`; RC4 for V1/V2).
    pub string_cipher: Cipher,
    /// The default cipher streams use (`/StmF`; RC4 for V1/V2).
    pub stream_cipher: Cipher,
    /// The cipher embedded-file streams use (`/EFF`, default `/StmF`).
    pub embedded_file_cipher: Cipher,
    /// The file key length in bits (40–128 for R2–R4, 256 for R5/R6).
    pub key_bits: u16,
    /// `/EncryptMetadata` (V4+; always true before V4): whether the XMP metadata stream is
    /// encrypted.
    pub encrypt_metadata: bool,
}

/// An authenticated standard security handler.
#[derive(Debug, Clone)]
pub(crate) struct Security {
    key: FileKey,
    info: Encryption,
    permissions: Permissions,
    matched: PasswordKind,
    perms_valid: Option<bool>,
    /// Named crypt filters from `/CF`, for per-stream `/Crypt` filters.
    filters: NamedFilters,
    /// The object number of an indirect `/Encrypt` dictionary, which is never decrypted.
    encrypt_object: Option<u32>,
}

pub(crate) fn invalid(message: &'static str) -> Error {
    Error::InvalidStructure {
        kind: "encryption dictionary",
        message,
    }
}

fn string<'d>(dict: &'d Dict, key: &str) -> Option<&'d [u8]> {
    match dict.get(key) {
        Some(Object::String(s)) => Some(&s.bytes),
        _ => None,
    }
}

/// The cipher of a `/CF` entry's `/CFM` (ISO 32000-2 Table 25).
fn cipher_of(filter: &Dict) -> Result<Cipher> {
    match filter.get_name("CFM") {
        None | Some(b"None") => Ok(Cipher::Identity),
        Some(b"V2") => Ok(Cipher::Rc4),
        Some(b"AESV2") => Ok(Cipher::Aes128),
        Some(b"AESV3") => Ok(Cipher::Aes256),
        Some(other) => Err(Error::UnsupportedSecurityHandler {
            filter: format!("Standard /CFM /{}", String::from_utf8_lossy(other)),
            version: None,
            revision: None,
        }),
    }
}

/// `/V` and `/R` of a standard security handler Lamina PDF can read: V1/V2 with R2/R3, V4 with R4,
/// V5 with R5/R6.
fn handler_revision(encrypt: &Dict) -> Result<(i64, u8)> {
    let version = encrypt.get_int("V").unwrap_or(0);
    let revision = encrypt.get_int("R");
    let unsupported = |filter: String| Error::UnsupportedSecurityHandler {
        filter,
        version: Some(version),
        revision,
    };
    match encrypt.get_name("Filter") {
        Some(b"Standard") => {}
        Some(other) => return Err(unsupported(String::from_utf8_lossy(other).into_owned())),
        None => return Err(unsupported(String::new())),
    }
    let ((1 | 2, Some(r @ (2 | 3))) | (4, Some(r @ 4)) | (5, Some(r @ (5 | 6)))) = (version, revision) else {
        return Err(unsupported("Standard".to_owned()));
    };
    let r = u8::try_from(r).map_err(|_| unsupported("Standard".to_owned()))?;
    Ok((version, r))
}

/// Crypt filters by name (`/CF`).
type NamedFilters = BTreeMap<Vec<u8>, Cipher>;

/// Named crypt filters (`/CF`) and the string, stream and embedded-file ciphers (`/StrF`, `/StmF`,
/// `/EFF`; default `/Identity`, `/EFF` defaulting to `/StmF`). Before V4 everything is RC4.
fn crypt_filters(encrypt: &Dict, version: i64) -> Result<(NamedFilters, Cipher, Cipher, Cipher)> {
    let mut filters = BTreeMap::new();
    if version < 4 {
        return Ok((filters, Cipher::Rc4, Cipher::Rc4, Cipher::Rc4));
    }
    if let Some(Object::Dict(cf)) = encrypt.get("CF") {
        for (name, filter) in cf.iter() {
            if let Object::Dict(filter) = filter {
                filters.insert(name.as_bytes().to_vec(), cipher_of(filter)?);
            }
        }
    }
    let pick = |key: &str, default: Cipher| -> Result<Cipher> {
        match encrypt.get_name(key) {
            None => Ok(default),
            Some(b"Identity") => Ok(Cipher::Identity),
            Some(name) => filters
                .get(name)
                .copied()
                .ok_or_else(|| invalid("crypt filter not in /CF")),
        }
    };
    let stream = pick("StmF", Cipher::Identity)?;
    let string = pick("StrF", Cipher::Identity)?;
    let embedded = pick("EFF", stream)?;
    Ok((filters, string, stream, embedded))
}

/// A key length in bytes from a `/Length` value: bits (40–128, a multiple of 8; Table 20), or
/// bytes for the few writers (and the standard handler's crypt filters, Table 25 in practice)
/// that store bytes.
fn length_in_bytes(length: i64) -> Option<usize> {
    match length {
        bits @ 40..=128 if bits % 8 == 0 => usize::try_from(bits / 8).ok(),
        bytes @ 5..=16 => usize::try_from(bytes).ok(),
        _ => None,
    }
}

/// The R2–R4 file key length in bytes. V1: 40 bits. V2/V3: `/Length`, default 40 bits
/// (Table 20). V4: `/Length` of the crypt filter streams use (else strings use) in `/CF`,
/// default 128 bits, as pdf.js and pypdf read it (Table 25; qpdf 12 reads the top-level
/// `/Length` instead, which Table 20 defines for V2/V3 only, and is used here as a fallback).
fn key_length(encrypt: &Dict, version: i64) -> usize {
    let length = if version >= 4 {
        let cf = match encrypt.get("CF") {
            Some(Object::Dict(cf)) => Some(cf),
            _ => None,
        };
        ["StmF", "StrF"]
            .iter()
            .filter_map(|key| encrypt.get_name(key))
            .find(|name| *name != b"Identity")
            .and_then(|name| cf?.iter().find(|(k, _)| k.as_bytes() == name))
            .and_then(|(_, filter)| filter.as_dict()?.get_int("Length"))
            .or_else(|| encrypt.get_int("Length"))
    } else if version >= 2 {
        encrypt.get_int("Length")
    } else {
        None
    };
    length
        .and_then(length_in_bytes)
        .unwrap_or(if version >= 4 { 16 } else { 5 })
}

impl Security {
    /// Parses `encrypt` (references already resolved) and authenticates `password`, or the
    /// empty password when none is given (ISO 32000-2 §7.6.4.4).
    ///
    /// # Errors
    /// [`Error::UnsupportedSecurityHandler`] for a handler other than `/Standard`, an unknown
    /// `/V`/`/R` or crypt filter method; [`Error::InvalidStructure`] for missing or malformed
    /// entries; [`Error::PasswordRequired`] when no password was given and the empty one does not
    /// open the file; [`Error::WrongPassword`] when the given one is neither the user nor the
    /// owner password.
    pub(crate) fn unlock(
        encrypt: &Dict,
        encrypt_object: Option<u32>,
        id0: &[u8],
        password: Option<&[u8]>,
    ) -> Result<Self> {
        let (version, r) = handler_revision(encrypt)?;
        let modern = r >= 5;

        let key_length = if modern { 32 } else { key_length(encrypt, version) };
        let need = if modern { 48 } else { 32 };
        let owner = string(encrypt, "O")
            .filter(|o| o.len() >= need)
            .ok_or_else(|| invalid("bad /O"))?;
        let user = string(encrypt, "U")
            .filter(|u| u.len() >= need)
            .ok_or_else(|| invalid("bad /U"))?;
        // R5/R6: the user password unwraps the file key from `/UE`, the owner password from
        // `/OE` (Algorithm 2.A), so a missing or short one only matters when no password
        // opens the file without it, as in pdf.js and PDFium.
        let wrapped_key = |key: &str| string(encrypt, key).filter(|v| v.len() >= 32);
        let (owner_encryption, user_encryption) = if modern {
            (wrapped_key("OE"), wrapped_key("UE"))
        } else {
            (Some(&[][..]), Some(&[][..]))
        };
        let p = encrypt.get_int("P").ok_or_else(|| invalid("missing /P"))?;
        let permissions = Permissions::new(p, r);
        let encrypt_metadata = version < 4 || !matches!(encrypt.get("EncryptMetadata"), Some(Object::Bool(false)));

        let (filters, string_cipher, stream_cipher, embedded_file_cipher) = crypt_filters(encrypt, version)?;

        let sec = StandardSecurity {
            revision: r,
            key_length,
            owner,
            user,
            owner_encryption: owner_encryption.unwrap_or_default(),
            user_encryption: user_encryption.unwrap_or_default(),
            perms: string(encrypt, "Perms"),
            permissions: permissions.raw(),
            id0,
            encrypt_metadata,
        };
        let auth = crypt::authenticate(&sec, password.unwrap_or_default()).ok_or_else(|| {
            if user_encryption.is_none() {
                invalid("bad /UE")
            } else if owner_encryption.is_none() {
                invalid("bad /OE")
            } else if password.is_some() {
                Error::WrongPassword
            } else {
                Error::PasswordRequired
            }
        })?;
        let key_bits = u16::try_from(auth.key.bits()).unwrap_or(u16::MAX);
        Ok(Self {
            key: auth.key,
            info: Encryption {
                version: u8::try_from(version).unwrap_or_default(),
                revision: r,
                string_cipher,
                stream_cipher,
                embedded_file_cipher,
                key_bits,
                encrypt_metadata,
            },
            permissions,
            matched: auth.matched,
            perms_valid: auth.perms_valid,
            filters,
            encrypt_object,
        })
    }

    pub(crate) const fn info(&self) -> &Encryption {
        &self.info
    }

    pub(crate) const fn permissions(&self) -> Permissions {
        self.permissions
    }

    pub(crate) const fn matched(&self) -> PasswordKind {
        self.matched
    }

    /// False when R5/R6 `/Perms` did not match `/P` (raised as `perms-mismatch`).
    pub(crate) fn perms_valid(&self) -> bool {
        self.perms_valid != Some(false)
    }

    /// Decrypts object `num gen` in place: its strings, and for a stream its data (ISO 32000-2
    /// §7.6.2). `resolve` dereferences an indirect `/DecodeParms` of a `/Crypt` filter.
    ///
    /// Exempt (never decrypted): the `/Encrypt` dictionary itself, cross-reference streams, the
    /// XMP metadata stream when `/EncryptMetadata` is false, and a signature dictionary's
    /// `/Contents`. Members of object streams are not passed here at all: their container was
    /// decrypted as a whole. Returns false when some AES data was malformed (kept, see
    /// [`crypt::Decrypted::well_formed`]).
    pub(crate) fn decrypt_object(
        &self,
        obj: &mut Object,
        num: u32,
        gen_nr: u16,
        resolve: &mut dyn FnMut(&Object) -> Option<Object>,
    ) -> bool {
        if self.encrypt_object == Some(num) {
            return true;
        }
        match obj {
            Object::Stream(stream) => {
                if stream.dict.get_name("Type") == Some(b"XRef") {
                    return true;
                }
                let mut ok = self.decrypt_strings_in_dict(&mut stream.dict, num, gen_nr);
                let cipher = self.stream_cipher(stream, resolve);
                let out = crypt::decrypt(cipher, &self.key, num, gen_nr, &stream.data);
                ok &= out.well_formed;
                stream.data = out.data;
                ok
            }
            other => self.decrypt_strings(other, num, gen_nr),
        }
    }

    /// The cipher for `stream`: a leading `/Crypt` filter's `/Name` (default `/Identity`;
    /// ISO 32000-2 §7.4.10), else `/EFF` for embedded files, else `/StmF`; plaintext for XMP
    /// metadata when `/EncryptMetadata` is false. A `/Name` missing from `/CF` falls back to
    /// `/StmF`.
    fn stream_cipher(&self, stream: &Stream, resolve: &mut dyn FnMut(&Object) -> Option<Object>) -> Cipher {
        let dict = &stream.dict;
        if !self.info.encrypt_metadata && dict.get_name("Type") == Some(b"Metadata") {
            return Cipher::Identity;
        }
        let deref = |o: &Object, resolve: &mut dyn FnMut(&Object) -> Option<Object>| match o {
            Object::Ref(_) => resolve(o),
            other => Some(other.clone()),
        };
        let filter = dict.get("Filter").and_then(|f| deref(f, resolve));
        let first = match &filter {
            Some(Object::Name(n)) => Some(n.as_bytes()),
            Some(Object::Array(items)) => match items.first() {
                Some(Object::Name(n)) => Some(n.as_bytes()),
                _ => None,
            },
            _ => None,
        };
        if first == Some(b"Crypt") {
            let parms = dict.get("DecodeParms").and_then(|p| deref(p, resolve));
            let parms = match parms {
                Some(Object::Array(items)) => items.first().and_then(|p| deref(p, resolve)),
                other => other,
            };
            let name = match parms {
                Some(Object::Dict(d)) => d.get_name("Name").map(<[u8]>::to_vec),
                _ => None,
            };
            return match name.as_deref() {
                None | Some(b"Identity") => Cipher::Identity,
                Some(name) => self.filters.get(name).copied().unwrap_or(self.info.stream_cipher),
            };
        }
        if dict.get_name("Type") == Some(b"EmbeddedFile") {
            return self.info.embedded_file_cipher;
        }
        self.info.stream_cipher
    }

    fn decrypt_strings(&self, obj: &mut Object, num: u32, gen_nr: u16) -> bool {
        match obj {
            Object::String(s) => {
                let out = crypt::decrypt(self.info.string_cipher, &self.key, num, gen_nr, &s.bytes);
                s.bytes = out.data;
                out.well_formed
            }
            // Every item is decrypted (no short-circuit); the result says whether all were well-formed.
            Object::Array(items) => items
                .iter_mut()
                .map(|item| self.decrypt_strings(item, num, gen_nr))
                .fold(true, BitAnd::bitand),
            Object::Dict(dict) => self.decrypt_strings_in_dict(dict, num, gen_nr),
            Object::Stream(stream) => self.decrypt_strings_in_dict(&mut stream.dict, num, gen_nr),
            Object::Null
            | Object::Bool(_)
            | Object::Integer(_)
            | Object::Real(_)
            | Object::Name(_)
            | Object::Ref(_) => true,
        }
    }

    fn decrypt_strings_in_dict(&self, dict: &mut Dict, num: u32, gen_nr: u16) -> bool {
        let signature = is_signature(dict);
        dict.iter_mut()
            .filter(|(key, _)| !(signature && key.as_bytes() == b"Contents"))
            .map(|(_, value)| self.decrypt_strings(value, num, gen_nr))
            .fold(true, BitAnd::bitand)
    }
}

/// A signature dictionary (ISO 32000-2 §12.8.1), whose `/Contents` is never encrypted:
/// `/Type /Sig` or `/DocTimeStamp`, or (since `/Type` is optional) `/ByteRange` with a string
/// `/Contents`.
fn is_signature(dict: &Dict) -> bool {
    matches!(dict.get_name("Type"), Some(b"Sig" | b"DocTimeStamp"))
        || (dict.contains_key("ByteRange") && matches!(dict.get("Contents"), Some(Object::String(_))))
}

/// Decrypts the stream object `num gen` for repair, before the key's document exists: direct
/// `/Crypt` parameters only. Returns false when some AES data was malformed.
pub(crate) fn decrypt_for_repair(security: &Security, stream: &mut Object, num: u32, gen_nr: u16) -> bool {
    security.decrypt_object(stream, num, gen_nr, &mut |_| None)
}

/// Removes a leading `/Crypt` filter (and its `/DecodeParms` entry) from `params`, which holds a
/// stream's resolved `/Filter` and `/DecodeParms`. Only for streams of an encrypted document,
/// which the reader decrypted when it loaded them (ISO 32000-2 §7.4.10: `/Crypt` comes first):
/// the decoder uses it, and `laminapdf-save` when it writes such a stream as plaintext.
///
/// Added in 0.2.0.
pub fn strip_crypt_filter(params: &mut Dict) {
    let is_crypt = |o: &Object| matches!(o, Object::Name(n) if n.as_bytes() == b"Crypt");
    match params.get_mut("Filter") {
        Some(Object::Array(filters)) if filters.first().is_some_and(is_crypt) => {
            filters.remove(0);
            match params.get_mut("DecodeParms") {
                Some(Object::Array(decode_parms)) if !decode_parms.is_empty() => {
                    decode_parms.remove(0);
                }
                Some(Object::Dict(_)) => {
                    params.remove("DecodeParms");
                }
                _ => {}
            }
        }
        Some(filter) if is_crypt(filter) => {
            params.remove("Filter");
            params.remove("DecodeParms");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laminapdf_syntax::PdfString;

    fn dict(entries: Vec<(&str, Object)>) -> Dict {
        entries.into_iter().collect()
    }

    #[test]
    fn unlock_rejects_a_public_key_handler() {
        let encrypt = dict(vec![
            ("Filter", Object::name("Adobe.PubSec")),
            ("V", 4.into()),
            ("R", 4.into()),
        ]);
        assert_eq!(
            Security::unlock(&encrypt, None, b"", None).err().map(|e| e.code()),
            Some("unsupported-security-handler")
        );
    }

    #[test]
    fn unlock_rejects_an_unknown_revision() {
        let encrypt = dict(vec![
            ("Filter", Object::name("Standard")),
            ("V", 5.into()),
            ("R", 7.into()),
        ]);
        assert!(matches!(
            Security::unlock(&encrypt, None, b"", None),
            Err(Error::UnsupportedSecurityHandler {
                version: Some(5),
                revision: Some(7),
                ..
            })
        ));
    }

    #[test]
    fn unlock_reports_missing_o_as_invalid_structure() {
        let encrypt = dict(vec![
            ("Filter", Object::name("Standard")),
            ("V", 2.into()),
            ("R", 3.into()),
        ]);
        assert_eq!(
            Security::unlock(&encrypt, None, b"", None).err().map(|e| e.code()),
            Some("invalid-structure")
        );
    }

    #[test]
    fn v4_key_length_comes_from_the_crypt_filter() {
        let cf: Dict = [(
            "StdCF",
            Object::Dict(
                [("CFM", Object::name("AESV2")), ("Length", 16.into())]
                    .into_iter()
                    .collect(),
            ),
        )]
        .into_iter()
        .collect();
        let encrypt = dict(vec![
            ("Length", 40.into()),
            ("CF", cf.into()),
            ("StmF", Object::name("StdCF")),
        ]);
        assert_eq!(key_length(&encrypt, 4), 16);
    }

    #[test]
    fn v4_key_length_defaults_to_128_bits() {
        assert_eq!(key_length(&Dict::new(), 4), 16);
    }

    #[test]
    fn v2_key_length_defaults_to_40_bits() {
        assert_eq!(key_length(&Dict::new(), 2), 5);
    }

    #[test]
    fn strip_crypt_filter_removes_the_leading_crypt_stage() {
        let mut params = dict(vec![
            (
                "Filter",
                vec![Object::name("Crypt"), Object::name("FlateDecode")].into(),
            ),
            ("DecodeParms", vec![Object::Dict(Dict::new()), Object::Null].into()),
        ]);
        strip_crypt_filter(&mut params);
        assert_eq!(
            params,
            dict(vec![
                ("Filter", vec![Object::name("FlateDecode")].into()),
                ("DecodeParms", vec![Object::Null].into()),
            ])
        );
    }

    #[test]
    fn signature_contents_detected_without_type() {
        let sig = dict(vec![
            ("ByteRange", vec![Object::from(0)].into()),
            ("Contents", Object::String(PdfString::hex(vec![1, 2]))),
        ]);
        assert!(is_signature(&sig));
    }
}
