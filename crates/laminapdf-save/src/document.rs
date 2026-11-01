//! Saving a [`laminapdf_ops::Document`] and rebasing it onto the output ([ADR 0005] §3,
//! [ADR 0007] §6).
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use std::sync::Arc;

use laminapdf_cos::{ByteSource, Source, SourceError};
use laminapdf_ops::{Document, PendingRebase, SaveState};

use crate::detect::Patches;
use crate::error::{Result, SaveError, SavedOutput};
use crate::full::FullWriter;
use crate::incremental::IncrementalWriter;
use crate::options::{SaveMode, SaveOptions, SaveReport};
use crate::planner::{SavePlan, SavePlanner};
use crate::sink::{ReadableSink, WriteSink};

/// Saving a [`laminapdf_ops::Document`] ([ADR 0007] §1): [`Save::plan`] shows how a save would
/// go, [`Save::save`] and [`Save::save_and_rebase`] plan and write.
///
/// Every save plans first ([`SavePlanner`]) and refuses before writing anything when the
/// plan does ([`SaveError::WouldInvalidateSignatures`], [`SaveError::RequiresRewrite`],
/// [`SaveError::EncryptedBase`]). With the explicit override
/// ([`SaveOptions::with_invalidate_signatures`]) the output has the invalidated signatures'
/// values (`/V`) and their `/Perms` entries removed: the writers write the planner's changes
/// over the document's values (the document itself only changes through the rebase onto the
/// output, so a failed save leaves nothing behind), and [`SaveReport::invalidated`] lists
/// them.
///
/// Every save is refused while a transaction is open
/// ([`NestedTransaction`](laminapdf_ops::Error::NestedTransaction)) or the document is
/// [`SaveState::SavedNotRebased`]. What a failure leaves behind depends on when it happened
/// ([`SaveError`]): before or while writing, the document is unchanged (`Dirty`, overlay
/// untouched); after writing ([`SaveError::NotRebased`]), it is `SavedNotRebased` and the
/// error carries the output to retry the rebase with. A successful save marks the document
/// saved ([`has_unsaved_changes`](laminapdf_ops::Document::has_unsaved_changes) turns false) and
/// rebases it onto the output, or leaves it `SavedNotRebased` when a full rewrite went to a
/// sink that can't be read back ([`Save::save`]).
///
/// The undo history survives a save only when every object it names still exists under its
/// number and generation: an incremental save that freed nothing
/// ([`PendingRebase::KeepHistory`]). A full rewrite, or an incremental save that deleted
/// objects, clears it ([`PendingRebase::ClearHistory`]).
///
/// ```
/// use laminapdf_ops::{Document, InfoPatch, Origin, SaveState, SetInfo};
/// use laminapdf_save::{Save, SaveMode, SaveOptions};
/// # use laminapdf_cos::{Dict, Object};
/// # use laminapdf_save::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
///
/// let mut doc = Document::open(bytes.clone(), Default::default())?;
/// doc.apply_all(Origin::Local, [SetInfo::new(InfoPatch::new().title(Some("Report")), None).into()])?;
///
/// let mut out = Vec::new();
/// let report = doc.save(&mut out, &SaveOptions::new())?; // Auto: incremental here
/// assert_eq!(report.mode(), SaveMode::Incremental);
/// assert!(out.starts_with(&bytes));
/// assert_eq!((doc.save_state(), doc.revisions()), (SaveState::Clean, 2));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
pub trait Save {
    /// How a save with `options` would go ([`SavePlanner::plan`]).
    ///
    /// # Errors
    /// As [`SavePlanner::new`] and [`SavePlanner::plan`].
    fn plan(&self, options: &SaveOptions) -> Result<SavePlan>;

    /// Plans and writes to `sink`. An incremental save rebases onto old base ‖ appended
    /// revision, built in memory; a full rewrite into a sink that can't be read back leaves the
    /// document [`SaveState::SavedNotRebased`] until
    /// [`rebase`](laminapdf_ops::Document::rebase) gets the saved bytes (use
    /// [`Save::save_and_rebase`] with a [`ReadableSink`] to avoid that).
    ///
    /// # Errors
    /// The plan's refusals ([`Save::plan`]); as [`IncrementalWriter::write`] and
    /// [`FullWriter::write`]; the refusals above; [`SaveError::NotRebased`].
    fn save<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport>;

    /// [`Save::save`] into a sink that gives the bytes back ([`ReadableSink::into_source`]),
    /// so a full rewrite rebases too; the bytes are then the document's
    /// ([`laminapdf_cos::Document::source`], [`laminapdf_cos::Document::data`]).
    ///
    /// # Errors
    /// As [`Save::save`]. When the sink can't give the bytes back ([`SaveError::Sink`]) the
    /// document is unchanged; when the rebase onto them fails, it is `SavedNotRebased` and
    /// [`SaveError::NotRebased`] carries the output.
    fn save_and_rebase<S: ReadableSink>(&mut self, sink: S, options: &SaveOptions) -> Result<SaveReport>;
}

/// The refusals every save shares.
fn ready(doc: &Document) -> Result<()> {
    if doc.save_state() == SaveState::SavedNotRebased {
        return Err(SaveError::Ops(laminapdf_ops::Error::SavedNotRebased));
    }
    if doc.in_transaction() {
        return Err(SaveError::Ops(laminapdf_ops::Error::NestedTransaction));
    }
    Ok(())
}

/// Records the written save and rebases onto `source`; a failed rebase hands the output back.
fn settle(doc: &mut Document, source: Source, pending: PendingRebase) -> Result<()> {
    doc.saved_not_rebased(pending)?;
    doc.rebase(source.clone()).map_err(|error| SaveError::NotRebased {
        saved: SavedOutput::new(source),
        error,
    })
}

/// What [`write`] did: an incremental save (already rebased), or a full rewrite whose output
/// the caller rebases onto or leaves pending.
enum Written {
    Incremental(SaveReport),
    Full(SaveReport),
}

/// Plans and writes: the shared part of [`Save::save`] and [`Save::save_and_rebase`].
fn write<S: WriteSink + ?Sized>(doc: &mut Document, sink: &mut S, options: SaveOptions) -> Result<Written> {
    ready(doc)?;
    let planner = SavePlanner::new(doc)?;
    let plan = planner.plan(&options)?;
    let patches = planner.patches(doc.cos(), &plan)?;
    let mut report = if plan.mode() == SaveMode::Incremental {
        write_incremental(doc, sink, options, &patches).map(Written::Incremental)?
    } else {
        FullWriter::new(doc.cos(), options)
            .with_patches(&patches)
            .write(sink)
            .map(Written::Full)?
    };
    let (Written::Incremental(r) | Written::Full(r)) = &mut report;
    r.invalidated = plan.would_invalidate().to_vec();
    Ok(report)
}

/// The incremental writer, then the rebase onto old base ‖ revision.
fn write_incremental<S: WriteSink + ?Sized>(
    doc: &mut Document,
    sink: &mut S,
    options: SaveOptions,
    patches: &Patches,
) -> Result<SaveReport> {
    let saved = IncrementalWriter::new(doc.cos(), options)
        .with_patches(patches)
        .write(sink)?;
    let pending = if saved.freed_objects() {
        PendingRebase::ClearHistory
    } else {
        PendingRebase::KeepHistory
    };
    let (report, appended) = saved.into_parts();
    if appended.is_empty() {
        doc.mark_saved();
        return Ok(report);
    }
    let base = Arc::clone(doc.cos().source());
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
    settle(doc, rebuilt, pending)?;
    Ok(report)
}

impl Save for Document {
    fn plan(&self, options: &SaveOptions) -> Result<SavePlan> {
        SavePlanner::new(self)?.plan(options)
    }

    fn save<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport> {
        match write(self, sink, *options)? {
            Written::Incremental(report) => Ok(report),
            Written::Full(report) => {
                self.saved_not_rebased(PendingRebase::ClearHistory)?;
                Ok(report)
            }
        }
    }

    fn save_and_rebase<S: ReadableSink>(&mut self, mut sink: S, options: &SaveOptions) -> Result<SaveReport> {
        match write(self, &mut sink, *options)? {
            Written::Incremental(report) => Ok(report),
            Written::Full(report) => {
                let source = Source::from(sink.into_source()?);
                settle(self, source, PendingRebase::ClearHistory)?;
                Ok(report)
            }
        }
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
