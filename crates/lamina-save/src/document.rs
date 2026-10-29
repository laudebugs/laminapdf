//! Saving a [`lamina_ops::Document`] and rebasing it onto the output ([ADR 0005] §3,
//! [ADR 0007] §6).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use std::sync::Arc;

use lamina_cos::{ByteSource, Source, SourceError};
use lamina_ops::{Document, SaveState};

use crate::error::{Result, SaveError};
use crate::full::FullWriter;
use crate::incremental::IncrementalWriter;
use crate::options::{Renumber, SaveOptions, SaveReport};
use crate::sink::{ReadableSink, WriteSink};

/// Saves for [`lamina_ops::Document`]. These are B3b's names; B3c replaces them with
/// `Document::save(sink, options)` and the save planner.
///
/// Every save is refused while a transaction is open
/// ([`NestedTransaction`](lamina_ops::Error::NestedTransaction)) or the document is
/// [`SaveState::SavedNotRebased`]. A save that fails leaves the document as it was (`Dirty`,
/// overlay untouched). A successful one marks it saved
/// ([`has_unsaved_changes`](lamina_ops::Document::has_unsaved_changes) turns false) and
/// rebases it onto the output, or leaves it `SavedNotRebased` when the output can't be read
/// back.
///
/// ```
/// use lamina_ops::{Document, InfoPatch, Origin, SaveState, SetInfo};
/// use lamina_save::{Save, SaveOptions};
/// # use lamina_cos::{Dict, Object};
/// # use lamina_save::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
///
/// let mut doc = Document::open(bytes.clone(), Default::default())?;
/// doc.apply_all(Origin::Local, [SetInfo::new(InfoPatch::new().title(Some("Report")), None).into()])?;
///
/// let mut out = Vec::new();
/// doc.save_incremental(&mut out, &SaveOptions::new())?;
/// assert!(out.starts_with(&bytes));
/// assert_eq!((doc.save_state(), doc.revisions()), (SaveState::Clean, 2));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait Save {
    /// Appends the overlay as an incremental update ([`IncrementalWriter`]) and rebases onto
    /// old base ‖ appended revision, built in memory (no read-back needed).
    ///
    /// # Errors
    /// As [`IncrementalWriter::write`], plus the refusals above.
    fn save_incremental<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport>;

    /// Writes a full rewrite ([`FullWriter`]) to a sink that can't be read back: the
    /// document is then [`SaveState::SavedNotRebased`] until
    /// [`rebase`](lamina_ops::Document::rebase) gets the saved bytes.
    ///
    /// # Errors
    /// As [`FullWriter::write`], plus the refusals above.
    fn save_full<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport>;

    /// Writes a full rewrite to `sink` and rebases onto what it gives back
    /// ([`ReadableSink::into_source`]); the bytes are then the document's
    /// ([`lamina_cos::Document::source`], [`lamina_cos::Document::data`]).
    ///
    /// # Errors
    /// As [`Save::save_full`]. When the bytes were written but can't be read back or opened,
    /// the document is left `SavedNotRebased` and the error says why.
    fn save_full_and_rebase<S: ReadableSink>(&mut self, sink: S, options: &SaveOptions) -> Result<SaveReport>;
}

/// The refusals every save shares.
fn ready(doc: &Document) -> Result<()> {
    if doc.save_state() == SaveState::SavedNotRebased {
        return Err(SaveError::Ops(lamina_ops::Error::SavedNotRebased));
    }
    if doc.in_transaction() {
        return Err(SaveError::Ops(lamina_ops::Error::NestedTransaction));
    }
    Ok(())
}

impl Save for Document {
    fn save_incremental<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport> {
        ready(self)?;
        let (report, appended) = IncrementalWriter::new(self.cos(), *options).write(sink)?.into_parts();
        if appended.is_empty() {
            self.mark_saved();
            return Ok(report);
        }
        let base = Arc::clone(self.cos().source());
        let rebuilt: Source = match base.as_slice() {
            Some(bytes) => {
                let mut whole = Vec::with_capacity(bytes.len() + appended.len());
                whole.extend_from_slice(bytes);
                whole.extend_from_slice(&appended);
                whole.into()
            }
            None => Appended {
                base,
                tail: appended.into(),
            }
            .into(),
        };
        self.saved_not_rebased(false);
        self.rebase(rebuilt)?;
        Ok(report)
    }

    fn save_full<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport> {
        ready(self)?;
        let report = FullWriter::new(self.cos(), *options).write(sink)?;
        self.saved_not_rebased(options.renumber() == Renumber::Compact);
        Ok(report)
    }

    fn save_full_and_rebase<S: ReadableSink>(&mut self, mut sink: S, options: &SaveOptions) -> Result<SaveReport> {
        ready(self)?;
        let report = FullWriter::new(self.cos(), *options).write(&mut sink)?;
        self.saved_not_rebased(options.renumber() == Renumber::Compact);
        let source = sink.into_source()?;
        self.rebase(source)?;
        Ok(report)
    }
}

/// The base followed by an appended revision, for a base without an in-memory slice: the new
/// base after an incremental save, without copying the old one.
struct Appended {
    base: Arc<dyn ByteSource>,
    tail: Arc<[u8]>,
}

impl ByteSource for Appended {
    fn len(&self) -> u64 {
        self.base
            .len()
            .saturating_add(u64::try_from(<[u8]>::len(&self.tail)).unwrap_or(u64::MAX))
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> core::result::Result<usize, SourceError> {
        let base_len = self.base.len();
        if offset < base_len {
            let room = usize::try_from(base_len - offset).unwrap_or(usize::MAX);
            let n = buf.len().min(room);
            return self.base.read_at(offset, buf.get_mut(..n).unwrap_or_default());
        }
        let Some(tail) = usize::try_from(offset - base_len)
            .ok()
            .and_then(|at| self.tail.get(at..))
        else {
            return Ok(0);
        };
        let n = tail.len().min(buf.len());
        if let (Some(dst), Some(src)) = (buf.get_mut(..n), tail.get(..n)) {
            dst.copy_from_slice(src);
        }
        Ok(n)
    }
}
