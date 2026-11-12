//! Saving a [`laminapdf_ops::Document`] and rebasing it onto the output ([ADR 0005] §3,
//! [ADR 0007] §6).
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use std::sync::Arc;

use laminapdf_cos::{ByteSource, Source, SourceError};
use laminapdf_ops::{Document, Origin, PendingRebase, SaveState, UpdateSaveMetadata};

use crate::detect::Patches;
use crate::error::{Result, SaveError, SavedOutput};
use crate::full::FullWriter;
use crate::id::{derived_instance_id, uuid};
use crate::incremental::IncrementalWriter;
use crate::options::{SaveMode, SaveOptions, SaveReport};
use crate::planner::{MetadataUpdate, SavePlan, SavePlanner};
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
/// use laminapdf_ops::{Document, MetadataPatch, Origin, SaveState, SetMetadata};
/// use laminapdf_save::{Save, SaveMode, SaveOptions};
/// # use laminapdf_cos::{Dict, Object};
/// # use laminapdf_save::test_support::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
///
/// let mut doc = Document::open(bytes.clone(), Default::default())?;
/// doc.apply_all(Origin::Local, [SetMetadata::new(MetadataPatch::new().title(Some("Report")), None).into()])?;
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

    /// Writes to `sink` what [`Save::save`] would, and leaves the document as it was: not
    /// rebased, its changes still pending (so a later plan still sees them), and the save-time
    /// stamp, when the plan stamps, written into the copy only. For Save a copy, and for the
    /// copy an edit session draws its pages from ([ADR 0007]).
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    ///
    /// # Errors
    /// As [`Save::save`], less [`SaveError::NotRebased`]; the document is unchanged either way.
    fn save_copy<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport>;

    /// Makes `copy`, the whole output of [`Save::save_copy`] ([`IncrementalOutput::Whole`]),
    /// the document's saved file, as if [`Save::save`] had written it: the document rebases onto
    /// it, and keeps or empties its undo history as the copy's
    /// [`SaveReport::history_cleared`] says. A shell writes the copy where the document lives,
    /// then adopts it, so a failed write leaves the document unsaved. The document must not
    /// change between the copy and its adoption.
    ///
    /// [`IncrementalOutput::Whole`]: crate::IncrementalOutput::Whole
    ///
    /// # Errors
    /// [`SaveError::Ops`] while a transaction is open or another save waits for its rebase;
    /// [`SaveError::NotRebased`] when `copy` can't be read (the document then waits for a
    /// rebase, as after a full rewrite to a sink it couldn't read back).
    fn adopt_copy(&mut self, copy: Source, history_cleared: bool) -> Result<()>;
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
///
/// When the plan stamps the save-time metadata, the stamp is applied in a transaction of its
/// own ([`Origin::System`]: never on the undo history) that stays open while the writer runs:
/// committed once the bytes are written, rolled back when writing fails, so a failed save leaves
/// the document as it was.
/// A `copy` rolls the stamp back once written and leaves the document unrebased.
fn write<S: WriteSink + ?Sized>(doc: &mut Document, sink: &mut S, options: SaveOptions, copy: bool) -> Result<Written> {
    ready(doc)?;
    let planner = SavePlanner::new(doc)?;
    let plan = planner.plan(&options)?;
    // Whether the stamp changed anything: a file without a packet and a save without a date
    // has nothing to stamp, and the report must say so.
    let stamped = *plan.metadata_update() == MetadataUpdate::Stamp && stamp(doc, &options)?;
    let result = write_planned(doc, sink, options, &planner, &plan, stamped, copy);
    if result.is_err() && doc.in_transaction() {
        // The writer failed before the stamp was committed; a failed rollback leaves the
        // transaction closed, and the error to report is the writer's.
        let _ = doc.rollback();
    }
    let mut report = result?;
    if copy && doc.in_transaction() {
        // The copy has the stamp; the document keeps its own metadata.
        doc.rollback()?;
    }
    let (Written::Incremental(r) | Written::Full(r)) = &mut report;
    r.invalidated = plan.would_invalidate().to_vec();
    r.metadata.updated = stamped;
    r.metadata.not_updated = matches!(plan.metadata_update(), MetadataUpdate::Skipped { .. });
    r.metadata.packet_rewritten = plan.packet_rewritten();
    Ok(report)
}

/// Opens the stamp's transaction and applies [`UpdateSaveMetadata`] in it; returns whether it
/// changed anything (the transaction is left open only then).
fn stamp(doc: &mut Document, options: &SaveOptions) -> Result<bool> {
    let id = match options.instance_id() {
        Some(id) => id,
        None => derived_instance_id(doc.cos())?,
    };
    let op = UpdateSaveMetadata::new(options.metadata_date(), uuid(id));
    doc.begin(Origin::System)?;
    match doc.apply(op) {
        Ok(applied) if !applied.undo.is_empty() => Ok(true),
        Ok(_) => {
            doc.rollback()?;
            Ok(false)
        }
        Err(e) => {
            doc.rollback()?;
            Err(e.into())
        }
    }
}

/// Writes the planned save; commits the stamp's transaction once the bytes are written.
fn write_planned<S: WriteSink + ?Sized>(
    doc: &mut Document,
    sink: &mut S,
    options: SaveOptions,
    planner: &SavePlanner,
    plan: &SavePlan,
    stamped: bool,
    copy: bool,
) -> Result<Written> {
    let patches = planner.patches(doc.cos(), plan)?;
    if plan.mode() == SaveMode::Incremental {
        write_incremental(doc, sink, options, &patches, stamped, copy).map(Written::Incremental)
    } else {
        let report = FullWriter::new(doc.cos(), options).with_patches(&patches).write(sink)?;
        if stamped && !copy {
            doc.commit()?;
        }
        Ok(Written::Full(report))
    }
}

/// The incremental writer, then the rebase onto old base ‖ revision.
fn write_incremental<S: WriteSink + ?Sized>(
    doc: &mut Document,
    sink: &mut S,
    options: SaveOptions,
    patches: &Patches,
    stamped: bool,
    copy: bool,
) -> Result<SaveReport> {
    let saved = IncrementalWriter::new(doc.cos(), options)
        .with_patches(patches)
        .write(sink)?;
    let pending = if saved.freed_objects() {
        PendingRebase::ClearHistory
    } else {
        PendingRebase::KeepHistory
    };
    let (mut report, appended) = saved.into_parts();
    // For a copy: whether adopting it as the save would.
    report.history_cleared = !appended.is_empty() && pending == PendingRebase::ClearHistory;
    if copy {
        return Ok(report);
    }
    if stamped {
        doc.commit()?;
    }
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
        match write(self, sink, *options, false)? {
            Written::Incremental(report) => Ok(report),
            Written::Full(mut report) => {
                self.saved_not_rebased(PendingRebase::ClearHistory)?;
                report.history_cleared = true;
                Ok(report)
            }
        }
    }

    fn save_and_rebase<S: ReadableSink>(&mut self, mut sink: S, options: &SaveOptions) -> Result<SaveReport> {
        match write(self, &mut sink, *options, false)? {
            Written::Incremental(report) => Ok(report),
            Written::Full(mut report) => {
                let source = Source::from(sink.into_source()?);
                settle(self, source, PendingRebase::ClearHistory)?;
                report.history_cleared = true;
                Ok(report)
            }
        }
    }

    fn save_copy<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport> {
        match write(self, sink, *options, true)? {
            Written::Incremental(report) => Ok(report),
            Written::Full(mut report) => {
                report.history_cleared = true;
                Ok(report)
            }
        }
    }

    fn adopt_copy(&mut self, copy: Source, history_cleared: bool) -> Result<()> {
        ready(self)?;
        let pending = if history_cleared {
            PendingRebase::ClearHistory
        } else {
            PendingRebase::KeepHistory
        };
        settle(self, copy, pending)
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
