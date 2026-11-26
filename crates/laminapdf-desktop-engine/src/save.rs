//! Saving a document's edits (E1 plan §7). The engine plans and writes the save; the main process,
//! which owns the file, puts the bytes where the document lives, then asks the engine to adopt
//! them. The engine can't write files, so the output waits here, read in parts, until it is
//! adopted or dropped.
//!
//! An incremental save's output is only the revision to append (`AppendedOnly`), so a large file
//! is never copied. Adopting it reads the file as it was, followed by that revision, through the
//! main process as before: nothing is read into memory.

use std::sync::Arc;

use laminapdf::{
    ConstraintKind, IncrementalOutput, PdfDate, PlannedSave, SaveMode, SaveOptions, SignatureInfo, SignatureKind,
    UndoOutcome,
};
use laminapdf_desktop_protocol::{
    EngineLimits, FailureKind, PlanConstraint, PlanReport, PlanSignature, SaveAnswer, SaveCall, SaveRequest,
    SaveWritten, SavedAs, SignatureRole,
};
use laminapdf_render::{ByteSource, RenderBackend, RenderOpenOptions};

use crate::edit::{Appended, Refused};
use crate::limits;
use crate::worker::Opened;

/// A save's output, written and waiting to be put in place.
pub(crate) struct Pending {
    mode: SavedAs,
    bytes: Arc<[u8]>,
    history_cleared: bool,
}

impl Pending {
    /// A new document, written whole, waiting to be read: never adopted by the document it was
    /// made beside.
    pub(crate) fn whole(bytes: Vec<u8>) -> Self {
        Self {
            mode: SavedAs::FullRewrite,
            bytes: bytes.into(),
            history_cleared: false,
        }
    }

    /// How many bytes wait.
    pub(crate) fn len(&self) -> u64 {
        u64::try_from(<[u8]>::len(&self.bytes)).unwrap_or(u64::MAX)
    }
}

/// What a save call answers: the answer, and the bytes its frame carries.
pub(crate) type Answered = (SaveAnswer, Vec<u8>);

/// Runs `call` on `opened`.
///
/// # Errors
/// [`FailureKind::SaveRefused`] with the error's code: options that can't be read, a refused save,
/// a read or an adoption with no output waiting.
pub(crate) fn run(
    opened: &mut Opened,
    backend: Option<&dyn RenderBackend>,
    limits: &EngineLimits,
    call: SaveCall,
    max_read: u32,
) -> Result<Answered, Refused> {
    match call {
        SaveCall::Plan(request) => {
            let plan = opened.lamina.plan(&options(&request)?).map_err(|e| refused(e.code()))?;
            Ok((SaveAnswer::Planned(report(&plan)), Vec::new()))
        }
        SaveCall::Write(request) => {
            let saved = opened
                .lamina
                .save_copy(&options(&request)?)
                .map_err(|e| refused(e.code()))?;
            let mode = if saved.report.mode() == SaveMode::FullRewrite {
                SavedAs::FullRewrite
            } else {
                SavedAs::Incremental
            };
            let written = SaveWritten {
                mode,
                len: u64::try_from(saved.bytes.len()).unwrap_or(u64::MAX),
                history_cleared: saved.report.history_cleared(),
                metadata_not_updated: saved.report.metadata_not_updated(),
            };
            opened.pending = Some(Pending {
                mode,
                bytes: saved.bytes.into(),
                history_cleared: written.history_cleared,
            });
            Ok((SaveAnswer::Written(written), Vec::new()))
        }
        SaveCall::CopyWith { log, request } => {
            let written = copy_with(opened, &log, &options(&request)?)?;
            Ok((SaveAnswer::Written(written), Vec::new()))
        }
        // A merge needs the other documents: the worker makes it (`Worker::merge`).
        SaveCall::Merge(_) => Err(refused("invalid-argument")),
        SaveCall::Read { offset, len } => {
            let pending = opened.pending.as_ref().ok_or_else(nothing_waiting)?;
            let start = usize::try_from(offset).unwrap_or(usize::MAX);
            let len = usize::try_from(len.min(max_read)).unwrap_or(0);
            let bytes = pending.bytes.get(start..).ok_or_else(|| refused("invalid-argument"))?;
            let part = bytes.get(..len.min(bytes.len())).unwrap_or_default();
            Ok((SaveAnswer::Bytes, part.to_vec()))
        }
        SaveCall::Adopt => {
            adopt(opened, backend, limits)?;
            Ok((SaveAnswer::Adopted, Vec::new()))
        }
        SaveCall::Drop => {
            opened.pending = None;
            Ok((SaveAnswer::Dropped, Vec::new()))
        }
    }
}

/// Writes a whole copy of the document with `log` applied, and keeps it waiting to be read. The
/// log is taken back out at once, so the document stays as it was, apart from its redo stack, which
/// the change cleared; the pages drawn never change.
fn copy_with(opened: &mut Opened, log: &str, options: &SaveOptions) -> Result<SaveWritten, Refused> {
    if opened.diverged {
        return Err((FailureKind::EditDiverged, "diverged".to_owned()));
    }
    if opened.encrypted {
        return Err((FailureKind::EditRefused, "encrypted-base".to_owned()));
    }
    let edited = opened
        .lamina
        .apply_log(log)
        .map_err(|e| (FailureKind::EditRefused, e.code().to_owned()))?;
    if !edited.changed {
        return Err(refused("no-change"));
    }
    let written = opened.lamina.save_copy(options);
    // Taken back before anything else: the copy is made, or the save refused, either way.
    let reverted = opened.lamina.undo();
    if !matches!(&reverted, Ok(Some(r)) if matches!(r.outcome, UndoOutcome::Applied)) {
        opened.diverged = true;
        opened.pending = None;
        opened.facts = None;
        return Err((FailureKind::EditDiverged, "diverged".to_owned()));
    }
    let saved = written.map_err(|e| refused(e.code()))?;
    let pending = Pending::whole(saved.bytes);
    let len = pending.len();
    opened.pending = Some(pending);
    opened.facts = None;
    Ok(SaveWritten {
        mode: SavedAs::FullRewrite,
        len,
        history_cleared: false,
        metadata_not_updated: saved.report.metadata_not_updated(),
    })
}

/// Makes the waiting output the document's saved file. A full rewrite is a new file, so the pages
/// are drawn from it from now on; an incremental save's file starts with the bytes they are drawn
/// from already.
fn adopt(opened: &mut Opened, backend: Option<&dyn RenderBackend>, limits: &EngineLimits) -> Result<(), Refused> {
    let pending = opened.pending.take().ok_or_else(nothing_waiting)?;
    let saved: Arc<dyn ByteSource> = match pending.mode {
        SavedAs::FullRewrite => Arc::new(Arc::clone(&pending.bytes)),
        SavedAs::Incremental => Arc::new(Appended {
            base: Arc::clone(&opened.source),
            tail: pending.bytes,
        }),
    };
    opened
        .lamina
        .adopt_copy_from(Arc::clone(&saved), pending.history_cleared)
        .map_err(|e| refused(e.code()))?;
    opened.source = Arc::clone(&saved);
    if pending.mode == SavedAs::FullRewrite
        && let Some(backend) = backend
    {
        // The old file is gone: drawing from it could read the new file's bytes.
        let options = RenderOpenOptions::new().with_limits(limits::render(limits));
        opened.render = backend.open(saved, &options).ok();
    }
    Ok(())
}

/// The façade's options for `request`. Incremental saves give only the appended revision.
fn options(request: &SaveRequest) -> Result<SaveOptions, Refused> {
    let mut options = SaveOptions::new()
        .with_output(IncrementalOutput::AppendedOnly)
        .with_invalidate_signatures(request.invalidate_signatures);
    if request.rewrite {
        options = options.with_mode(SaveMode::FullRewrite);
    }
    if let Some(date) = &request.metadata_date {
        let date = PdfDate::from_iso8601(date).map_err(|_| refused("invalid-argument"))?;
        options = options.with_metadata_date(date);
    }
    if let Some(id) = request.instance_id {
        options = options.with_instance_id(id);
    }
    Ok(options)
}

/// The plan as the protocol carries it.
fn report(plan: &PlannedSave) -> PlanReport {
    PlanReport {
        mode: plan.mode.map(|mode| {
            if mode == SaveMode::Incremental {
                SavedAs::Incremental
            } else {
                SavedAs::FullRewrite
            }
        }),
        refusal: plan.refusal.map(|code| code.as_str().to_owned()),
        constraint: match plan.constraint {
            ConstraintKind::Free => PlanConstraint::Free,
            ConstraintKind::AppendOnly => PlanConstraint::AppendOnly,
            ConstraintKind::RequiresRewrite => PlanConstraint::RequiresRewrite,
            _ => PlanConstraint::Conflict,
        },
        signatures: plan.signatures.iter().map(signature).collect(),
        would_invalidate: plan.would_invalidate.iter().map(signature).collect(),
        conformance_warnings: plan
            .conformance_warnings
            .iter()
            .map(|warning| warning.diagnostic().to_json().to_string())
            .collect(),
        unchanged: plan.unchanged,
    }
}

fn signature(info: &SignatureInfo) -> PlanSignature {
    PlanSignature {
        kind: match info.kind {
            SignatureKind::Approval => SignatureRole::Approval,
            SignatureKind::Certification => SignatureRole::Certification,
            SignatureKind::DocTimeStamp => SignatureRole::DocTimeStamp,
            _ => SignatureRole::UsageRights,
        },
        field: info.field.as_ref().map(ToString::to_string),
    }
}

fn nothing_waiting() -> Refused {
    refused("no-pending-save")
}

fn refused(code: &str) -> Refused {
    (FailureKind::SaveRefused, code.to_owned())
}
