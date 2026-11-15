//! Saving a document's edits (E1 plan §7): the engine plans and writes the save, and the main
//! process, which owns the file, puts the bytes where the document lives.
//!
//! A save takes four calls on the document. [`SaveCall::Plan`] says how a save would go.
//! [`SaveCall::Write`] makes the engine write the output and keep it: the revision an incremental
//! save appends, or the whole file of a full rewrite. [`SaveCall::Read`] hands that output over in
//! parts, each no larger than one frame allows. Once the file is written, [`SaveCall::Adopt`]
//! makes the output the document's saved file. Until then the document stays unsaved, so a write
//! that fails leaves it as it was.

use serde::{Deserialize, Serialize};

/// The save's options: the façade's `SaveOptions` that the desktop sets. The webview sends them
/// as JSON, in camelCase, any field left out at its default.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SaveRequest {
    /// Rewrite the whole file, even where an incremental update would do.
    pub rewrite: bool,
    /// The explicit override that removes the signatures a change invalidates.
    pub invalidate_signatures: bool,
    /// When the save happens, as an ISO 8601 date-time: the modification and metadata dates.
    pub metadata_date: Option<String>,
    /// The XMP instance ID the save writes, a UUID's 16 bytes.
    pub instance_id: Option<[u8; 16]>,
}

/// What a save call asks of a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveCall {
    /// How a save with these options would go. Answered by [`SaveAnswer::Planned`].
    Plan(SaveRequest),
    /// Write the save's output and keep it for [`SaveCall::Read`]; the document stays unsaved.
    /// Answered by [`SaveAnswer::Written`].
    Write(SaveRequest),
    /// The kept output's bytes from `offset`, at most `len` of them. Answered by
    /// [`SaveAnswer::Bytes`], whose frame carries them as its payload.
    Read {
        /// Where the bytes start in the output.
        offset: u64,
        /// How many bytes, at most.
        len: u32,
    },
    /// The kept output is written where the document lives: make it the document's saved
    /// file. Answered by [`SaveAnswer::Adopted`].
    Adopt,
    /// Forget the kept output: the write failed or was cancelled. Answered by
    /// [`SaveAnswer::Dropped`].
    Drop,
}

/// What a save call did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveAnswer {
    /// How the save would go.
    Planned(PlanReport),
    /// The output is written and kept.
    Written(SaveWritten),
    /// Bytes of the kept output; they follow the header as the frame's payload.
    Bytes,
    /// The document's saved file is now the output.
    Adopted,
    /// The kept output is gone.
    Dropped,
}

/// The mode a save resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SavedAs {
    /// A revision appended to the file.
    Incremental,
    /// A new file, written whole.
    FullRewrite,
}

/// What restricts a save (the planner's `ConstraintKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanConstraint {
    /// Nothing.
    Free,
    /// Signatures: only an incremental update keeps them.
    AppendOnly,
    /// Only a full rewrite is safe (a repaired file, an encrypted one).
    RequiresRewrite,
    /// The edits break a signature's restriction.
    Conflict,
}

/// What a signature is (the planner's `SignatureRole`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureRole {
    /// An approval signature.
    Approval,
    /// A certification, which restricts later changes.
    Certification,
    /// A document timestamp.
    DocTimeStamp,
    /// Usage rights (UR3).
    UsageRights,
}

/// A signature a save keeps or would invalidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSignature {
    /// What it is.
    pub kind: SignatureRole,
    /// Its field's full name, when it has one.
    pub field: Option<String>,
}

/// How a save would go, as the webview's save flow reads it: the façade's `PlannedSave`, without
/// the parts the desktop doesn't show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanReport {
    /// The mode the save resolves to; `None` when it would be refused.
    pub mode: Option<SavedAs>,
    /// Why the save would be refused (`would-invalidate-signatures`, `requires-rewrite`,
    /// `encrypted-base`).
    pub refusal: Option<String>,
    /// What restricts the save.
    pub constraint: PlanConstraint,
    /// Every signature, in the order found.
    pub signatures: Vec<PlanSignature>,
    /// The signatures the save would invalidate (only with the override).
    pub would_invalidate: Vec<PlanSignature>,
    /// The PDF/A and PDF/UA claims the edited metadata breaks, each as its diagnostic's JSON
    /// (ADR 0026): the pipe's format can't carry a self-describing value.
    pub conformance_warnings: Vec<String>,
    /// The document has no changes.
    pub unchanged: bool,
}

/// The output a save wrote and kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveWritten {
    /// How it was written: an incremental save's output is only the revision to append.
    pub mode: SavedAs,
    /// The output's length in bytes.
    pub len: u64,
    /// The save freed objects, so adopting it clears the undo history.
    pub history_cleared: bool,
    /// The save-time metadata was left out, since stamping it would break a signature.
    pub metadata_not_updated: bool,
}
