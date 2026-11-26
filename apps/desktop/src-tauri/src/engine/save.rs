//! A document's save through its window's engine (E1 plan §7): the engine plans the save and
//! writes its output, which the main process reads in parts and puts in place (`crate::save`),
//! then the engine adopts it. The engine reads untrusted files, so the parts it sends are checked.

use std::path::Path;

use laminapdf_desktop_protocol::{
    DocId, FailureKind, FromEngine, MergeFields, MergeFile, MergeOutline, MergeRequest, PlanReport, SaveAnswer,
    SaveCall, SaveRequest, SaveWritten, ToEngine,
};

use super::{Engines, ask_with_payload, engine_error, needs_password, unexpected};
use crate::DesktopError;

/// The most bytes one read of a save's output asks for: well under a frame.
const PART: u32 = 16 * 1024 * 1024;

impl Engines {
    /// How a save of `window`'s document `doc` with `request` would go.
    ///
    /// # Errors
    /// The engine's failures; `unknown-document` for another window's document.
    pub(crate) fn plan_save(&self, window: &str, doc: DocId, request: SaveRequest) -> Result<PlanReport, DesktopError> {
        match self.save_call(window, doc, SaveCall::Plan(request))? {
            (SaveAnswer::Planned(plan), _) => Ok(plan),
            (other, _) => Err(unexpected_answer(&other)),
        }
    }

    /// Has the engine write the save and keep its output.
    ///
    /// # Errors
    /// `save-refused` with the refusal's code; the engine's failures.
    pub(crate) fn write_save(
        &self,
        window: &str,
        doc: DocId,
        request: SaveRequest,
    ) -> Result<SaveWritten, DesktopError> {
        match self.save_call(window, doc, SaveCall::Write(request))? {
            (SaveAnswer::Written(written), _) => Ok(written),
            (other, _) => Err(unexpected_answer(&other)),
        }
    }

    /// Has the engine write a new document of `window`'s document `doc`, as its edits leave it, and
    /// of `files` (other documents of the window, with the title of each one's bookmark) after it,
    /// and keep it to read. `doc` doesn't change; [`Engines::drop_save`] forgets the output.
    ///
    /// # Errors
    /// `unknown-document` for a document of another window; `save-refused` with the engine's code
    /// (`wrong-password`, `no-pages-left`); the engine's failures.
    pub(crate) fn write_merge(
        &self,
        window: &str,
        doc: DocId,
        title: String,
        files: &[(DocId, String)],
        (outline, fields): (MergeOutline, MergeFields),
    ) -> Result<SaveWritten, DesktopError> {
        let mut taken = Vec::with_capacity(files.len());
        for (source, name) in files {
            self.check(window, *source)?;
            taken.push(MergeFile {
                source: *source,
                password: self.registry.password_of(*source),
                title: name.clone(),
            });
        }
        let request = MergeRequest {
            title,
            files: taken,
            outline,
            fields,
        };
        match self.save_call(window, doc, SaveCall::Merge(request))? {
            (SaveAnswer::Written(written), _) => Ok(written),
            (other, _) => Err(unexpected_answer(&other)),
        }
    }

    /// Copies the kept output, `len` bytes, into `out`, part by part.
    ///
    /// # Errors
    /// An engine that sends no bytes or more than `len` (`Other`); the engine's failures; `out`'s.
    pub(crate) fn copy_save(
        &self,
        window: &str,
        doc: DocId,
        len: u64,
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let mut offset = 0;
        while offset < len {
            let want = u32::try_from(len - offset).unwrap_or(PART).min(PART);
            let part = match self.save_call(window, doc, SaveCall::Read { offset, len: want }) {
                Ok((SaveAnswer::Bytes, part)) => part,
                Ok((other, _)) => return Err(std::io::Error::other(unexpected_answer(&other))),
                Err(error) => return Err(std::io::Error::other(error)),
            };
            let got = u64::try_from(part.len()).unwrap_or(u64::MAX);
            if part.is_empty() || got > u64::from(want) {
                return Err(std::io::Error::other("the engine sent a save's bytes out of order"));
            }
            out.write_all(&part)?;
            offset += got;
        }
        Ok(())
    }

    /// The kept output is in place: the engine makes it the document's saved file.
    ///
    /// # Errors
    /// The engine's failures.
    pub(crate) fn adopt_save(&self, window: &str, doc: DocId) -> Result<(), DesktopError> {
        match self.save_call(window, doc, SaveCall::Adopt)? {
            (SaveAnswer::Adopted, _) => Ok(()),
            (other, _) => Err(unexpected_answer(&other)),
        }
    }

    /// `doc` was saved to `path`: it reads its file from there from now on.
    ///
    /// # Errors
    /// The OS's error when the saved file can't be opened; `doc` then keeps its old file.
    pub(crate) fn saved_to(&self, doc: DocId, path: &Path) -> std::io::Result<()> {
        self.registry.saved_to(doc, path)
    }

    /// Whether `doc` can still read the bytes its file had when it opened (see
    /// `DocumentRegistry::base_intact`).
    pub(crate) fn base_intact(&self, doc: DocId) -> bool {
        self.registry.base_intact(doc)
    }

    /// Forgets the kept output after a write that failed. A failure here is only logged: the next
    /// save writes a new output anyway.
    pub(crate) fn drop_save(&self, window: &str, doc: DocId) {
        if let Err(error) = self.save_call(window, doc, SaveCall::Drop) {
            tracing::debug!(doc = doc.0, "the engine didn't drop a save's output: {error}");
        }
    }

    /// `doc`'s file length as the document read it, while it is open.
    pub(crate) fn file_len(&self, doc: DocId) -> Option<u64> {
        self.registry.len_of(doc)
    }

    /// Whether `doc`'s file changed on disk since it opened or was last saved.
    pub(crate) fn changed_on_disk(&self, doc: DocId) -> bool {
        self.registry.changed_on_disk(doc).unwrap_or(true)
    }

    fn save_call(&self, window: &str, doc: DocId, call: SaveCall) -> Result<(SaveAnswer, Vec<u8>), DesktopError> {
        self.check(window, doc)?;
        match ask_with_payload(&self.engine(window)?, |req| ToEngine::Save { req, doc, call })? {
            (FromEngine::Saved { answer, .. }, payload) => Ok((*answer, payload)),
            (FromEngine::NeedsPassword { .. }, _) => Err(needs_password()),
            (other, _) => Err(unexpected(&other)),
        }
    }
}

fn unexpected_answer(answer: &SaveAnswer) -> DesktopError {
    tracing::warn!("unexpected answer to a save from the engine: {answer:?}");
    engine_error(FailureKind::Protocol)
}
