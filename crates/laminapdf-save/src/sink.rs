//! Where saved bytes go ([ADR 0007] §6): [`WriteSink`], [`ReadableSink`] and the core
//! implementations.
//!
//! Output streams through the sink in pieces, never as one buffer, and nothing seeks or
//! patches: a sink only ever appends. Core implementations are `Vec<u8>` (readable) and
//! [`IoWrite`] over any [`std::io::Write`]; spooling to OPFS or a temporary file, files and
//! uploads are edge implementations.
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use std::sync::Arc;

use laminapdf_cos::ByteSource;

/// Why a sink could not take (or give back) the bytes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SinkError {
    /// An [`IoWrite`] adapter's writer failed. `message` comes from the host (it may name a
    /// path), so it is not formatted (ADR 0026 §4).
    #[error("write failed ({kind:?})")]
    Io {
        /// The I/O error kind.
        kind: std::io::ErrorKind,
        /// The I/O error's message.
        message: String,
    },
    /// Any other sink failure (a full spool, a closed upload, a refused read-back). The text
    /// comes from the sink, so it is not formatted (ADR 0026 §4).
    #[error("sink failed")]
    Failed(String),
    /// The sink already held `held` bytes. Offsets in the output count from its first byte,
    /// so a writer refuses before writing (an `AppendedOnly` incremental save also accepts a
    /// sink holding exactly the base).
    #[error("the sink already holds {held} bytes")]
    NotEmpty {
        /// Bytes the sink held.
        held: u64,
    },
}

impl From<std::io::Error> for SinkError {
    fn from(e: std::io::Error) -> Self {
        Self::Io {
            kind: e.kind(),
            message: e.to_string(),
        }
    }
}

/// An append-only destination for saved bytes.
pub trait WriteSink {
    /// Appends all of `bytes`.
    ///
    /// # Errors
    /// Whatever the destination reports; the save then fails with
    /// [`SaveError::Sink`](crate::SaveError::Sink).
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError>;

    /// How many bytes the sink already holds, when it knows (`Vec<u8>`: its length); `None`
    /// (the default) when it can't tell. Writers refuse a sink that holds bytes
    /// ([`SinkError::NotEmpty`]), because their offsets count from the output's first byte.
    fn held(&self) -> Option<u64> {
        None
    }
}

/// A sink that can give back what was written as a [`ByteSource`], so a full rewrite can
/// rebase the document onto it (and, in E10, sign on top of it).
pub trait ReadableSink: WriteSink {
    /// What was written, as a source.
    ///
    /// # Errors
    /// When the bytes can't be read back.
    fn into_source(self) -> Result<Arc<dyn ByteSource>, SinkError>;
}

impl WriteSink for Vec<u8> {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        self.extend_from_slice(bytes);
        Ok(())
    }

    fn held(&self) -> Option<u64> {
        u64::try_from(self.len()).ok()
    }
}

impl ReadableSink for Vec<u8> {
    fn into_source(self) -> Result<Arc<dyn ByteSource>, SinkError> {
        Ok(Arc::new(self))
    }
}

impl<S: WriteSink + ?Sized> WriteSink for &mut S {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        (**self).write_all(bytes)
    }

    fn held(&self) -> Option<u64> {
        (**self).held()
    }
}

/// A [`WriteSink`] over any [`std::io::Write`] (a file opened by the shell, a socket, a
/// compressor). Not readable: a full rewrite into it leaves the document `SavedNotRebased`.
///
/// ```
/// use laminapdf_save::{IoWrite, WriteSink};
///
/// let mut sink = IoWrite::new(std::io::Cursor::new(Vec::new()));
/// sink.write_all(b"%PDF-")?;
/// assert_eq!(sink.into_inner().into_inner(), b"%PDF-");
/// # Ok::<(), laminapdf_save::SinkError>(())
/// ```
#[derive(Debug)]
pub struct IoWrite<W>(W);

impl<W: std::io::Write> IoWrite<W> {
    /// Wraps `writer`.
    pub const fn new(writer: W) -> Self {
        Self(writer)
    }

    /// The writer back (flush it yourself).
    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W: std::io::Write> WriteSink for IoWrite<W> {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        self.0.write_all(bytes).map_err(SinkError::from)
    }
}

/// Counts the bytes written through it: the position every offset in the output is taken
/// from, since sinks never seek.
///
/// ```
/// use laminapdf_save::{CountingSink, WriteSink};
///
/// let mut out = Vec::new();
/// let mut sink = CountingSink::new(&mut out);
/// sink.write_all(b"abc")?;
/// assert_eq!(sink.position(), 3);
/// # Ok::<(), laminapdf_save::SinkError>(())
/// ```
#[derive(Debug)]
pub struct CountingSink<S> {
    inner: S,
    position: u64,
}

impl<S: WriteSink> CountingSink<S> {
    /// Wraps `inner`, starting at position 0.
    pub const fn new(inner: S) -> Self {
        Self { inner, position: 0 }
    }

    /// Bytes written so far.
    pub const fn position(&self) -> u64 {
        self.position
    }

    /// The wrapped sink.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: WriteSink> WriteSink for CountingSink<S> {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        self.inner.write_all(bytes)?;
        self.position = self
            .position
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        Ok(())
    }

    fn held(&self) -> Option<u64> {
        self.inner.held()
    }
}

/// Refuses a sink that holds bytes, unless it holds exactly `allowed` (an appended-only save
/// into a sink that already holds the base).
pub(crate) fn require_empty<S: WriteSink + ?Sized>(sink: &S, allowed: Option<u64>) -> Result<(), SinkError> {
    match sink.held() {
        Some(held) if held != 0 && Some(held) != allowed => Err(SinkError::NotEmpty { held }),
        _ => Ok(()),
    }
}
