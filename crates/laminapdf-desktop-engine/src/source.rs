//! Source documents: the files whose pages an edit inserts, replaces or merges in (E2 plan §7).
//!
//! The façade's `add_source` takes the file's bytes, so the engine reads the source's file whole
//! from the main process, which gave it the file by id and never by path. The source is also open
//! in the engine as a document of its own, so its pages draw as any document's do; the n-th page
//! of the source is the n-th page of that document.

use laminapdf::Document;
use laminapdf_desktop_protocol::FailureKind;
use laminapdf_render::ByteSource;

use crate::worker::kind_of;

/// A failed add: the protocol's kind and the error's code.
type Refused = (FailureKind, String);

/// Bytes read from the file at a time.
const CHUNK: usize = 1 << 20;

/// Adds `file` to `doc`'s edit session as a source, and gives the source's ID and the IDs of its
/// pages, in page order. The same bytes give the same ID, and adding them again changes nothing.
///
/// # Errors
/// [`FailureKind::Limit`] for a file too large to hold in memory, [`FailureKind::Io`] when the
/// file can't be read, and the façade's code for a file that isn't a PDF or needs another password.
pub(crate) fn add(
    doc: &mut Document,
    file: &dyn ByteSource,
    password: Option<&str>,
) -> Result<(String, Vec<String>), Refused> {
    let bytes = read_whole(file)?;
    let id = doc
        .add_source(bytes, password)
        .map_err(|e| (kind_of(e.code()), e.code().to_owned()))?;
    let pages = doc
        .source_pages(&id)
        .map_err(|e| (kind_of(e.code()), e.code().to_owned()))?;
    Ok((id.to_string(), pages.iter().map(ToString::to_string).collect()))
}

/// Every byte of `file`.
fn read_whole(file: &dyn ByteSource) -> Result<Vec<u8>, Refused> {
    let len = usize::try_from(file.len()).map_err(|_| too_large())?;
    let mut bytes = Vec::new();
    // A refused allocation is a limit, not an abort.
    bytes.try_reserve_exact(len).map_err(|_| too_large())?;
    let mut chunk = vec![0_u8; CHUNK.min(len.max(1))];
    while bytes.len() < len {
        let at = bytes.len() as u64;
        let want = chunk.len().min(len - bytes.len());
        let got = file
            .read_at(at, chunk.get_mut(..want).unwrap_or_default())
            .map_err(|_| (FailureKind::Io, "source".to_owned()))?;
        if got == 0 {
            return Err((FailureKind::Io, "source".to_owned()));
        }
        bytes.extend_from_slice(chunk.get(..got).unwrap_or_default());
    }
    Ok(bytes)
}

fn too_large() -> Refused {
    (FailureKind::Limit, "limit-exceeded".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_whole_should_join_every_chunk() {
        let file: Vec<u8> = (0..=255).cycle().take(CHUNK * 2 + 7).collect();
        assert_eq!(read_whole(&file).ok(), Some(file));
    }

    #[test]
    fn read_whole_should_fail_on_a_file_that_ends_early() {
        struct Short;
        impl ByteSource for Short {
            fn len(&self) -> u64 {
                10
            }
            fn read_at(&self, _: u64, _: &mut [u8]) -> Result<usize, laminapdf_render::SourceError> {
                Ok(0)
            }
        }
        assert_eq!(read_whole(&Short).err().map(|e| e.0), Some(FailureKind::Io));
    }
}
