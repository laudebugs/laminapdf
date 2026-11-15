//! A document's edits, as the web reader's document worker makes them (E1 plan §6).
//!
//! PDFium draws the pages from the bytes it opened, so an edit to Lamina PDF's model reaches the
//! screen as bytes: after a call that touched pages or moved them, the model writes every edit
//! since the file was opened as one revision appended to it (`save_copy`, which leaves the model
//! unsaved), and PDFium opens the file with it. The new render document replaces the old one only
//! once it is open, so a failure leaves the pages drawn as they were, and the call is taken back
//! out of the model. The worker runs one job at a time, so no tile renders while they swap.
//!
//! The history (labels, times, the saved point) is the webview's: it names steps, the engine only
//! does them.

use std::sync::Arc;

use laminapdf::{IncrementalOutput, PageId, SaveMode, SaveOptions, UndoOutcome};
use laminapdf_desktop_protocol::{
    EditCall, EditReport, EditedPage, EngineLimits, FailureKind, Reversal,
};
use laminapdf_render::{ByteSource, RenderBackend, RenderOpenOptions, SourceError};

use crate::limits;
use crate::worker::Opened;

/// Why an encrypted document takes no edit: the copy its pages would be drawn from can't be
/// written yet (the save's `encrypted-base`).
const ENCRYPTED: &str = "encrypted-base";

/// A failed call: the protocol's kind and the error's code.
pub(crate) type Refused = (FailureKind, String);

/// The failure of a call made on a document that diverged, and of the take-back that made it so.
fn diverged() -> Refused {
    (FailureKind::EditDiverged, "diverged".to_owned())
}

/// Applies, undoes or redoes `call` on `opened`, and draws its pages from the edited file from
/// now on.
///
/// # Errors
/// The op's, undo's or redo's code (the document is unchanged); a save or reopen error, after the
/// call is taken back ([`FailureKind::RenderFailed`]); [`FailureKind::EditDiverged`] when taking
/// it back failed too, and for every call after.
pub(crate) fn run(
    opened: &mut Opened,
    backend: Option<&dyn RenderBackend>,
    limits: &EngineLimits,
    call: EditCall,
) -> Result<EditReport, Refused> {
    if opened.diverged {
        return Err(diverged());
    }
    if opened.encrypted {
        return Err(refused(ENCRYPTED));
    }
    let before = page_ids(opened)?;
    let (way, done) = match call {
        EditCall::Apply { log } => {
            let edited = opened.lamina.apply_log(&log).map_err(|e| refused(e.code()))?;
            if !edited.changed {
                return report(opened, Done::nothing());
            }
            (
                Way::Undo,
                Done {
                    changed: true,
                    reversal: None,
                    touched: edited.touched_pages,
                    page_order: edited.page_order,
                    created_outline_items: edited.created_outline_items.iter().map(ToString::to_string).collect(),
                    created_annotations: edited.created_annotations.iter().map(ToString::to_string).collect(),
                    outline_repaired: edited.outline_repaired,
                },
            )
        }
        EditCall::Undo | EditCall::Redo => {
            let undo = call == EditCall::Undo;
            let reverted = if undo {
                opened.lamina.undo()
            } else {
                opened.lamina.redo()
            };
            let Some(reverted) = reverted.map_err(|e| refused(e.code()))? else {
                return report(opened, Done::nothing());
            };
            let reversal = match reverted.outcome {
                UndoOutcome::Applied => Reversal::Applied,
                UndoOutcome::Partial { .. } => Reversal::Partial,
                _ => Reversal::Skipped,
            };
            let done = Done {
                reversal: Some(reversal),
                touched: reverted.touched_pages.into_iter().collect(),
                ..Done::nothing()
            };
            // A skipped undo or redo changed nothing, so there is nothing to draw again.
            if reversal == Reversal::Skipped {
                return report(opened, done);
            }
            (if undo { Way::Redo } else { Way::Undo }, Done { changed: true, ..done })
        }
    };
    let moved = page_ids(opened)? != before;
    if (moved || !done.touched.is_empty())
        && let Err(failure) = redraw(opened, backend, limits)
    {
        take_back(opened, way)?;
        return Err(failure);
    }
    report(opened, done)
}

/// What a call did, before the pages after it are read.
struct Done {
    changed: bool,
    reversal: Option<Reversal>,
    touched: Vec<PageId>,
    page_order: Option<Vec<PageId>>,
    created_outline_items: Vec<String>,
    created_annotations: Vec<String>,
    outline_repaired: bool,
}

impl Done {
    /// A call that changed nothing.
    const fn nothing() -> Self {
        Self {
            changed: false,
            reversal: None,
            touched: Vec::new(),
            page_order: None,
            created_outline_items: Vec::new(),
            created_annotations: Vec::new(),
            outline_repaired: false,
        }
    }
}

/// Which call takes a call back out of the model.
#[derive(Clone, Copy)]
enum Way {
    Undo,
    Redo,
}

/// The report of `done`, with every page as it is now.
fn report(opened: &Opened, done: Done) -> Result<EditReport, Refused> {
    let pages = opened.lamina.page_infos().map_err(|e| refused(e.code()))?;
    Ok(EditReport {
        changed: done.changed,
        reversal: done.reversal,
        touched: done.touched.iter().map(ToString::to_string).collect(),
        page_order: done
            .page_order
            .map(|order| order.iter().map(ToString::to_string).collect()),
        pages: pages
            .iter()
            .map(|page| EditedPage {
                id: page.id.to_string(),
                size: crate::worker::dimensions(page),
            })
            .collect(),
        created_outline_items: done.created_outline_items,
        created_annotations: done.created_annotations,
        outline_repaired: done.outline_repaired,
    })
}

/// Every page's ID, in page order.
fn page_ids(opened: &Opened) -> Result<Vec<PageId>, Refused> {
    let pages = opened.lamina.page_infos().map_err(|e| refused(e.code()))?;
    Ok(pages.into_iter().map(|page| page.id).collect())
}

/// Writes the model's changes since the file was opened and swaps in a render document opened
/// from the file with them. The copy overrides signatures a change breaks: the pages must show
/// the edit, and only a save asks first.
fn redraw(opened: &mut Opened, backend: Option<&dyn RenderBackend>, limits: &EngineLimits) -> Result<(), Refused> {
    let options = SaveOptions::new()
        .with_mode(SaveMode::Auto)
        .with_output(IncrementalOutput::AppendedOnly)
        .with_invalidate_signatures(true);
    // A failure here is the redraw's, after the call changed the model: it is taken back.
    let saved = opened
        .lamina
        .save_copy(&options)
        .map_err(|e| (FailureKind::RenderFailed, e.code().to_owned()))?;
    // Without PDFium there is nothing to draw; the model still takes the edit.
    let Some(backend) = backend else { return Ok(()) };
    let source: Arc<dyn ByteSource> = if saved.report.mode() == SaveMode::FullRewrite {
        Arc::new(saved.bytes)
    } else {
        Arc::new(Appended {
            base: Arc::clone(&opened.source),
            tail: saved.bytes.into(),
        })
    };
    let options = RenderOpenOptions::new().with_limits(limits::render(limits));
    let render = backend
        .open(source, &options)
        .map_err(|e| (FailureKind::RenderFailed, e.code().to_owned()))?;
    opened.render = Some(render);
    Ok(())
}

/// Takes back the call just made, with `way`; when that fails too, marks the document diverged.
fn take_back(opened: &mut Opened, way: Way) -> Result<(), Refused> {
    let reverted = match way {
        Way::Undo => opened.lamina.undo(),
        Way::Redo => opened.lamina.redo(),
    };
    // A partial outcome still restored what it could; only nothing at all leaves them apart.
    match reverted {
        Ok(Some(reverted)) if !matches!(reverted.outcome, UndoOutcome::Skipped { .. }) => Ok(()),
        _ => {
            opened.diverged = true;
            Err(diverged())
        }
    }
}

fn refused(code: &str) -> Refused {
    (FailureKind::EditRefused, code.to_owned())
}

/// The file followed by the revision an edit appended, read in place, never copied into one
/// buffer.
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

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        let base = self.base.len();
        if offset < base {
            let room = usize::try_from(base - offset).unwrap_or(usize::MAX);
            let n = buf.len().min(room);
            return self.base.read_at(offset, buf.get_mut(..n).unwrap_or_default());
        }
        let Some(tail) = usize::try_from(offset - base).ok().and_then(|at| self.tail.get(at..)) else {
            return Ok(0);
        };
        let n = tail.len().min(buf.len());
        if let (Some(dst), Some(src)) = (buf.get_mut(..n), tail.get(..n)) {
            dst.copy_from_slice(src);
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appended_should_read_across_the_join() {
        let joined = Appended {
            base: Arc::new(b"abc".to_vec()),
            tail: Arc::from(&b"de"[..]),
        };
        let mut buf = [0_u8; 4];
        let n = joined.read_at(1, &mut buf).unwrap_or_default();
        let m = joined.read_at(1 + n as u64, &mut buf[n..]).unwrap_or_default();
        assert_eq!((joined.len(), &buf[..n + m]), (5, &b"bcde"[..]));
    }
}
