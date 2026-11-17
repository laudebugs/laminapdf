//! Writing an embedded file (ISO 32000-2 §7.11.4) and its file specification (§7.11.3): what
//! the builder's attachments and the attachment ops both write, so a file attached either way
//! has the same entries.

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString, Result, Stream};
use laminapdf_syntax::text::encode_text_string;
use md5::{Digest as _, Md5};

use super::AfRelationship;
use crate::PdfDate;

/// A file to embed, with the facts its file specification and `/Params` record.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EmbeddedFileSpec<'a> {
    /// The file's name: the specification's `/F` and `/UF`.
    pub name: &'a str,
    /// The file's bytes.
    pub bytes: &'a [u8],
    /// Its MIME type, the stream's `/Subtype`; empty writes none.
    pub mime: &'a str,
    /// `/Desc`.
    pub description: Option<&'a str>,
    /// `/Params /CreationDate`.
    pub created: Option<&'a PdfDate>,
    /// `/Params /ModDate`.
    pub modified: Option<&'a PdfDate>,
    /// `/AFRelationship`.
    pub relationship: AfRelationship,
}

impl<'a> EmbeddedFileSpec<'a> {
    /// The file `name` with `bytes` of type `mime`, with no description or dates and an
    /// unspecified relationship.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(name: &'a str, bytes: &'a [u8], mime: &'a str) -> Self {
        Self {
            name,
            bytes,
            mime,
            description: None,
            created: None,
            modified: None,
            relationship: AfRelationship::Unspecified,
        }
    }
}

/// The MD5 digest of `bytes`, as `/Params /CheckSum` records it (§7.11.4.2).
///
/// Added in 0.6.0.
#[must_use]
pub fn md5(bytes: &[u8]) -> [u8; 16] {
    Md5::digest(bytes).into()
}

/// Writes `file` as a Flate-compressed embedded file stream with its `/Params` (`/Size`, the
/// dates given and `/CheckSum`) and a file specification with `/F`, `/UF`, `/EF`, `/Desc` and
/// `/AFRelationship`, and returns the specification's object. Listing it in the
/// `/EmbeddedFiles` tree or an `/AF` array is the caller's.
///
/// # Errors
/// Errors of the overlay (allocating object numbers).
///
/// Added in 0.6.0.
pub fn write_embedded_file(cos: &mut CosMut<'_>, file: &EmbeddedFileSpec<'_>) -> Result<ObjRef> {
    let text = |s: &str| Object::String(PdfString::literal(encode_text_string(s)));
    let pdf_date = |d: &PdfDate| Object::String(PdfString::literal(d.to_string()));
    let mut params = Dict::new();
    params.insert("Size", i64::try_from(file.bytes.len()).unwrap_or(i64::MAX));
    if let Some(created) = file.created {
        params.insert("CreationDate", pdf_date(created));
    }
    if let Some(modified) = file.modified {
        params.insert("ModDate", pdf_date(modified));
    }
    params.insert("CheckSum", PdfString::hex(md5(file.bytes).to_vec()));
    let mut stream = Dict::new();
    stream.insert("Type", Name::from("EmbeddedFile"));
    if !file.mime.is_empty() {
        stream.insert("Subtype", Name::new(file.mime.to_owned()));
    }
    stream.insert("Params", params);
    let data = cos.create(Object::Stream(Stream::flate(stream, file.bytes)))?;
    let mut ef = Dict::new();
    ef.insert("F", data);
    ef.insert("UF", data);
    let mut spec = Dict::new();
    spec.insert("Type", Name::from("Filespec"));
    spec.insert("F", text(file.name));
    spec.insert("UF", text(file.name));
    spec.insert("EF", ef);
    if let Some(desc) = file.description {
        spec.insert("Desc", text(desc));
    }
    spec.insert("AFRelationship", Name::from(file.relationship.as_str()));
    cos.create(Object::Dict(spec))
}
