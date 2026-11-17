//! An attached file's bytes ([`Document::attachment_bytes`]), held to the document's
//! [`Limits`](laminapdf_cos::Limits) and checked against the checksum the file states.

use laminapdf_cos::{Error, Limit, Object, Result};
use md5::{Digest as _, Md5};

use super::embedded_stream;
use crate::document::Document;
use crate::id::AttachmentId;

/// What comparing an attached file's bytes with its `/Params /CheckSum` found.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Checksum {
    /// The file states no checksum.
    Absent,
    /// The bytes' MD5 digest is the one the file states.
    Matches,
    /// It isn't: the bytes changed after the checksum was written, or the checksum is wrong.
    Mismatch,
}

/// An attached file's bytes, decoded, and what its checksum says of them.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttachmentBytes {
    /// The file's bytes, exactly as attached.
    pub bytes: Vec<u8>,
    /// The checksum comparison.
    pub checksum: Checksum,
}

impl Document {
    /// The bytes of the attached file `id`, decoded from its embedded file stream: `None` when
    /// the document holds no attachment with this ID, or its file isn't embedded.
    ///
    /// The size is checked against [`Limits::max_stream_decoded`](laminapdf_cos::Limits::max_stream_decoded)
    /// before anything is decoded: the size `/Params` states and the stored length, so a file
    /// too large for the device is refused without being read. The bytes are then compared
    /// with the stated `/CheckSum`.
    ///
    /// # Errors
    /// [`Error::LimitExceeded`] for a file larger than the limit; decoding errors of the
    /// stream (a filter that fails, over a limit); errors that must propagate.
    ///
    /// Added in 0.6.0.
    pub fn attachment_bytes(&self, id: &AttachmentId) -> Result<Option<AttachmentBytes>> {
        let Some(found) = self.find_attachment(id)? else {
            return Ok(None);
        };
        let Some(stream) = embedded_stream(self, &found.spec)? else {
            return Ok(None);
        };
        let Object::Stream(stream) = &*stream else {
            return Ok(None);
        };
        let cap = u64::try_from(self.limits().max_stream_decoded()).unwrap_or(u64::MAX);
        let stated = found.attachment.size;
        let stored = u64::try_from(stream.data.len()).unwrap_or(u64::MAX);
        if stated.is_some_and(|s| s > cap) || stored > cap {
            return Err(Error::LimitExceeded {
                limit: Limit::StreamDecoded,
                at: None,
            });
        }
        let bytes = self.decode_stream(stream)?;
        let checksum = match found.attachment.checksum {
            None => Checksum::Absent,
            Some(sum) if Md5::digest(&bytes).as_slice() == sum.as_slice() => Checksum::Matches,
            Some(_) => Checksum::Mismatch,
        };
        Ok(Some(AttachmentBytes { bytes, checksum }))
    }
}
