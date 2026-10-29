//! Saving a [`lamina_ops::Document`] and rebasing it onto the output ([ADR 0005] §3,
//! [ADR 0007] §6).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use std::sync::Arc;

use lamina_cos::{ByteSource, CosMut, JournalMark, ObjRef, Object, Source, SourceError};
use lamina_ops::{Document, PendingRebase, SaveState};

use crate::constraint::SignatureInfo;
use crate::error::{Result, SaveError, SavedOutput};
use crate::full::FullWriter;
use crate::incremental::IncrementalWriter;
use crate::options::{SaveMode, SaveOptions, SaveReport};
use crate::planner::{SavePlan, SavePlanner};
use crate::sink::{ReadableSink, WriteSink};

/// Saving a [`lamina_ops::Document`] ([ADR 0007] §1): [`Save::plan`] shows how a save would
/// go, [`Save::save`] and [`Save::save_and_rebase`] plan and write.
///
/// Every save plans first ([`SavePlanner`]) and refuses before writing anything when the
/// plan does ([`SaveError::WouldInvalidateSignatures`], [`SaveError::RequiresRewrite`],
/// [`SaveError::EncryptedBase`]). With the explicit override
/// ([`SaveOptions::with_invalidate_signatures`]) the planner removes the invalidated
/// signatures' values (`/V`) and their `/Perms` entries in the overlay, outside the undo
/// history, right before writing (undone again if the save fails before its output is
/// complete), and [`SaveReport::invalidated`] lists them.
///
/// Every save is refused while a transaction is open
/// ([`NestedTransaction`](lamina_ops::Error::NestedTransaction)) or the document is
/// [`SaveState::SavedNotRebased`]. What a failure leaves behind depends on when it happened
/// ([`SaveError`]): before or while writing, the document is unchanged (`Dirty`, overlay
/// untouched); after writing ([`SaveError::NotRebased`]), it is `SavedNotRebased` and the
/// error carries the output to retry the rebase with. A successful save marks the document
/// saved ([`has_unsaved_changes`](lamina_ops::Document::has_unsaved_changes) turns false) and
/// rebases it onto the output, or leaves it `SavedNotRebased` when a full rewrite went to a
/// sink that can't be read back ([`Save::save`]).
///
/// The undo history survives a save only when every object it names still exists under its
/// number and generation: an incremental save that freed nothing
/// ([`PendingRebase::KeepHistory`]). A full rewrite, or an incremental save that deleted
/// objects, clears it ([`PendingRebase::ClearHistory`]).
///
/// ```
/// use lamina_ops::{Document, InfoPatch, Origin, SaveState, SetInfo};
/// use lamina_save::{Save, SaveMode, SaveOptions};
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
/// let report = doc.save(&mut out, &SaveOptions::new())?; // Auto: incremental here
/// assert_eq!(report.mode(), SaveMode::Incremental);
/// assert!(out.starts_with(&bytes));
/// assert_eq!((doc.save_state(), doc.revisions()), (SaveState::Clean, 2));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
pub trait Save {
    /// How a save with `options` would go ([`SavePlanner::plan`]).
    ///
    /// # Errors
    /// As [`SavePlanner::new`] and [`SavePlanner::plan`].
    fn plan(&self, options: &SaveOptions) -> Result<SavePlan>;

    /// Plans and writes to `sink`. An incremental save rebases onto old base ‖ appended
    /// revision, built in memory; a full rewrite into a sink that can't be read back leaves the
    /// document [`SaveState::SavedNotRebased`] until
    /// [`rebase`](lamina_ops::Document::rebase) gets the saved bytes (use
    /// [`Save::save_and_rebase`] with a [`ReadableSink`] to avoid that).
    ///
    /// # Errors
    /// The plan's refusals ([`Save::plan`]); as [`IncrementalWriter::write`] and
    /// [`FullWriter::write`]; the refusals above; [`SaveError::NotRebased`].
    fn save<S: WriteSink + ?Sized>(&mut self, sink: &mut S, options: &SaveOptions) -> Result<SaveReport>;

    /// [`Save::save`] into a sink that gives the bytes back ([`ReadableSink::into_source`]),
    /// so a full rewrite rebases too; the bytes are then the document's
    /// ([`lamina_cos::Document::source`], [`lamina_cos::Document::data`]).
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
        return Err(SaveError::Ops(lamina_ops::Error::SavedNotRebased));
    }
    if doc.in_transaction() {
        return Err(SaveError::Ops(lamina_ops::Error::NestedTransaction));
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

/// Plans, and applies the override's removals. Returns the plan and the journal mark to roll
/// the removals back to if the save fails before its output is complete.
fn prepare(doc: &mut Document, options: SaveOptions) -> Result<(SavePlan, Option<JournalMark>)> {
    ready(doc)?;
    let plan = doc.plan(&options)?;
    let mark = if plan.would_invalidate().is_empty() {
        None
    } else {
        Some(doc.edit_outside_history(|cos| strip_signatures(cos, plan.would_invalidate()))?)
    };
    Ok((plan, mark))
}

/// The report, or the error after the override's removals were undone (unless the output was
/// written and the document waits for its rebase: the removals are in it).
fn finish(
    doc: &mut Document,
    plan: &SavePlan,
    mark: Option<JournalMark>,
    result: Result<SaveReport>,
) -> Result<SaveReport> {
    match result {
        Ok(mut report) => {
            report.invalidated = plan.would_invalidate().to_vec();
            Ok(report)
        }
        Err(e) => {
            if let Some(mark) = mark
                && doc.save_state() != SaveState::SavedNotRebased
            {
                doc.rollback_outside_history(mark)?;
            }
            Err(e)
        }
    }
}

/// Removes the value (`/V`) of each invalidated signature's field and the `/Perms` entries
/// that point at an invalidated signature (`/Perms` itself when nothing is left) ([ADR 0007]
/// §3). A signature without an indirect field keeps nothing to remove there: a full rewrite
/// drops its dictionary when nothing else reaches it.
///
/// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
fn strip_signatures(cos: &mut CosMut<'_>, signatures: &[SignatureInfo]) -> lamina_cos::Result<()> {
    for field in signatures
        .iter()
        .filter_map(|s| s.field.as_ref()?.as_lamina_id().obj_ref())
    {
        if let Some(object) = cos.get(field)?
            && let Some(dict) = object.as_dict()
            && dict.contains_key("V")
        {
            let mut dict = dict.clone();
            dict.remove("V");
            cos.set(field, Object::Dict(dict))?;
        }
    }
    let values: Vec<ObjRef> = signatures.iter().filter_map(|s| s.value).collect();
    let Some(root) = cos.trailer().get("Root").and_then(Object::as_ref) else {
        return Ok(());
    };
    let Some(catalog) = cos.get(root)?.and_then(|o| o.as_dict().cloned()) else {
        return Ok(());
    };
    let Some(perms_value) = catalog.get("Perms").cloned() else {
        return Ok(());
    };
    let perms_ref = perms_value.as_ref();
    let perms = match perms_ref {
        Some(r) => cos.get(r)?.and_then(|o| o.as_dict().cloned()),
        None => perms_value.as_dict().cloned(),
    };
    let Some(mut perms) = perms else {
        return Ok(());
    };
    let before = perms.len();
    for key in ["DocMDP", "UR3", "UR"] {
        if perms
            .get(key)
            .and_then(Object::as_ref)
            .is_some_and(|r| values.contains(&r))
        {
            perms.remove(key);
        }
    }
    if perms.len() == before {
        return Ok(());
    }
    let mut catalog = catalog;
    if perms.is_empty() {
        catalog.remove("Perms");
        cos.set(root, Object::Dict(catalog))?;
    } else if let Some(r) = perms_ref {
        cos.set(r, Object::Dict(perms))?;
    } else {
        catalog.insert("Perms", Object::Dict(perms));
        cos.set(root, Object::Dict(catalog))?;
    }
    Ok(())
}

/// The incremental writer, then the rebase onto old base ‖ revision.
fn write_incremental<S: WriteSink + ?Sized>(
    doc: &mut Document,
    sink: &mut S,
    options: SaveOptions,
) -> Result<SaveReport> {
    let saved = IncrementalWriter::new(doc.cos(), options).write(sink)?;
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
        let (plan, mark) = prepare(self, *options)?;
        let result = if plan.mode() == SaveMode::Incremental {
            write_incremental(self, sink, *options)
        } else {
            FullWriter::new(self.cos(), *options).write(sink).and_then(|report| {
                self.saved_not_rebased(PendingRebase::ClearHistory)?;
                Ok(report)
            })
        };
        finish(self, &plan, mark, result)
    }

    fn save_and_rebase<S: ReadableSink>(&mut self, mut sink: S, options: &SaveOptions) -> Result<SaveReport> {
        let (plan, mark) = prepare(self, *options)?;
        let result = if plan.mode() == SaveMode::Incremental {
            write_incremental(self, &mut sink, *options)
        } else {
            FullWriter::new(self.cos(), *options)
                .write(&mut sink)
                .and_then(|report| {
                    let source = Source::from(sink.into_source()?);
                    settle(self, source, PendingRebase::ClearHistory)?;
                    Ok(report)
                })
        };
        finish(self, &plan, mark, result)
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
