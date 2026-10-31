//! Reading a whole [`ByteSource`] for engines that need every byte in memory (hayro).

use lamina_cos::{ByteSource, SourceError, merge_ranges};

use crate::error::RenderError;

/// How much is read per call: lamina-cos's window size.
const CHUNK: usize = 64 * 1024;

/// Reads all of `source` into memory, at most `max_bytes` (an in-memory source is copied:
/// backends that can borrow its slice, like hayro, check [`ByteSource::as_slice`] first).
///
/// One pass over the whole source: every range it answers `Pending` for is collected, so a
/// single `data-pending` error lists everything the shell must fetch before calling again.
///
/// # Errors
/// `render-too-large` when the source is longer than `max_bytes`; `data-pending` with the
/// merged missing ranges; `source` when a read fails.
pub fn read_all(source: &dyn ByteSource, max_bytes: u64) -> Result<Vec<u8>, RenderError> {
    let len = source.len();
    if len > max_bytes {
        return Err(RenderError::InputTooLarge { len, limit: max_bytes });
    }
    if let Some(bytes) = source.as_slice() {
        return Ok(bytes.to_vec());
    }
    let size = usize::try_from(len).map_err(|_| RenderError::InputTooLarge { len, limit: max_bytes })?;
    let mut out = vec![0; size];
    let mut pending = Vec::new();
    let mut at = 0;
    while at < size {
        let end = at.saturating_add(CHUNK).min(size);
        let Some(buf) = out.get_mut(at..end) else { break };
        let offset = at as u64;
        match source.read_at(offset, buf) {
            // A source that returns nothing before the end has lost bytes it claimed to have.
            Ok(0) => {
                return Err(RenderError::Source {
                    offset,
                    len: (end - at) as u64,
                });
            }
            Ok(n) => at += n,
            Err(SourceError::Pending { offset, len }) => {
                pending.push((offset, len));
                // Skip what the source said is missing (at least this chunk) and keep looking.
                let skip = usize::try_from(offset.saturating_add(len)).unwrap_or(size);
                at = skip.max(end);
            }
            Err(e) => return Err(e.into()),
        }
    }
    if pending.is_empty() {
        Ok(out)
    } else {
        Err(RenderError::DataPending {
            ranges: merge_ranges(pending),
        })
    }
}

#[cfg(test)]
mod tests {

    use lamina_cos::{ByteSource, SourceError};

    use super::read_all;
    use crate::RenderError;

    /// 200 KiB of which `missing` is pending.
    struct Gappy {
        missing: std::ops::Range<u64>,
    }

    impl ByteSource for Gappy {
        fn len(&self) -> u64 {
            200 * 1024
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
            let end = offset + buf.len() as u64;
            if offset < self.missing.end && end > self.missing.start {
                return Err(SourceError::Pending {
                    offset: self.missing.start,
                    len: self.missing.end - self.missing.start,
                });
            }
            buf.fill(7);
            Ok(buf.len())
        }
    }

    #[test]
    fn reads_an_in_memory_source_whole() {
        assert_eq!(read_all(&vec![1u8, 2, 3], 10).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn reads_a_windowed_source_whole() {
        let source = Gappy { missing: 0..0 };
        assert_eq!(read_all(&source, u64::MAX).unwrap().len(), 200 * 1024);
    }

    #[test]
    fn reports_every_missing_range_in_one_error() {
        let source = Gappy {
            missing: 70_000..140_000,
        };
        assert_eq!(
            read_all(&source, u64::MAX).unwrap_err(),
            RenderError::DataPending {
                ranges: vec![(70_000, 70_000)]
            }
        );
    }

    #[test]
    fn refuses_an_in_memory_source_over_the_limit_without_copying() {
        assert_eq!(read_all(&vec![0u8; 2048], 1024).unwrap_err().code(), "render-too-large");
    }

    #[test]
    fn refuses_a_source_over_the_limit() {
        let source = Gappy { missing: 0..0 };
        assert_eq!(read_all(&source, 1024).unwrap_err().code(), "render-too-large");
    }
}
