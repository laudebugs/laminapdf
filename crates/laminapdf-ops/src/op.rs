//! The typed, serialisable [`Op`] (ADR 0006 §1) and the versioned [`OpLog`] envelope (§6).
#![expect(
    deprecated,
    reason = "SetInfo is defined and dispatched here; it stays, deprecated, so v1 op logs still apply"
)]

use laminapdf_model::PageId;
use serde::{Deserialize, Deserializer, Serialize};

use crate::class::ModificationClass;
use crate::error::{Error, Result};
use crate::info::SetInfo;
use crate::labels::SetPageLabels;
use crate::metadata::{RemoveMetadata, SetMetadata, UpdateSaveMetadata};
use crate::rotate::RotatePage;
#[cfg(feature = "test-support")]
use crate::test_support::Declared;
use crate::widgets::{PageChange, TouchedWidget};

/// An editing operation ([ADR 0006] §1): typed and semantic (it names things by
/// [`LaminaPdfId`](laminapdf_model::LaminaPdfId), never by object reference), serialisable, and free
/// of ambient inputs (a modification date is in the payload), so the same op on the same
/// document gives the same result on every platform.
///
/// Apply it inside a transaction ([`Tx::apply`](crate::Tx::apply)); its JSON form is
/// adjacently tagged, `{ "op": "rotatePage", "args": { … } }`, and unknown ops and fields
/// are rejected, never skipped.
///
/// ```
/// use laminapdf_ops::Op;
///
/// let op: Op = serde_json::from_str(r#"{ "op": "rotatePage", "args": { "page": "p3.0", "rotate": 90 } }"#)?;
/// assert_eq!(serde_json::to_string(&op)?, r#"{"op":"rotatePage","args":{"page":"p3.0","rotate":90}}"#);
/// assert!(serde_json::from_str::<Op>(r#"{ "op": "explode", "args": {} }"#).is_err());
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", content = "args", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum Op {
    /// Edit the document information dictionary (and an existing XMP packet with it).
    ///
    /// Deprecated in favour of [`Op::SetMetadata`], which also writes authors as a list, dates
    /// and XMP-only files; kept, with its v1 wire form, so logs already written still apply
    /// (ADR 0006 §6).
    SetInfo(SetInfo),
    /// Set a page's rotation.
    RotatePage(RotatePage),
    /// Edit the document's metadata in Info and XMP together.
    SetMetadata(SetMetadata),
    /// Remove metadata fields from Info and XMP.
    RemoveMetadata(RemoveMetadata),
    /// The save-time metadata stamp, made only by the save planner. It never crosses the wire:
    /// serialising it fails with serde's error (`OpLog::to_json`: `invalid-op-log`), and no op
    /// log can name it.
    #[serde(skip)]
    UpdateSaveMetadata(UpdateSaveMetadata),
    /// Replace the document's page labels.
    SetPageLabels(SetPageLabels),
    /// An op that declares the widgets it touches, for the save planner's tests
    /// ([`test_support::Declared`](crate::test_support::Declared)); never on the wire.
    #[cfg(feature = "test-support")]
    #[serde(skip)]
    Declared(Declared),
}

impl Op {
    /// The kind of change, for the save planner (ADR 0006 §1).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn class(&self) -> ModificationClass {
        match self {
            Self::SetInfo(_) | Self::SetMetadata(_) | Self::RemoveMetadata(_) | Self::UpdateSaveMetadata(_) => {
                ModificationClass::Metadata
            }
            Self::RotatePage(_) => ModificationClass::PageStructure,
            Self::SetPageLabels(_) => ModificationClass::Document,
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.op.class(),
        }
    }

    /// Whether saving this op needs a full rewrite rather than an incremental update
    /// (redaction, encryption changes; none of these ops does).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn requires_rewrite(&self) -> bool {
        match self {
            Self::SetInfo(_)
            | Self::RotatePage(_)
            | Self::SetMetadata(_)
            | Self::RemoveMetadata(_)
            | Self::UpdateSaveMetadata(_)
            | Self::SetPageLabels(_) => false,
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.op.requires_rewrite(),
        }
    }

    /// The pages this op would delete, replace, reshow or move: what `laminapdf-save`'s
    /// `SavePlanner::would_violate` checks against the widgets on them before the op applies
    /// ([E2 plan §6]). None of these ops reports a page yet: [`Op::RotatePage`] starts
    /// reporting its page along with the page ops that delete, move and reverse pages.
    ///
    /// [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn page_changes(&self) -> Vec<PageChange> {
        match self {
            Self::SetInfo(_)
            | Self::RotatePage(_)
            | Self::SetMetadata(_)
            | Self::RemoveMetadata(_)
            | Self::UpdateSaveMetadata(_)
            | Self::SetPageLabels(_) => Vec::new(),
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.pages.clone(),
        }
    }

    /// Applies the op's writes through the overlay; returns what it did. The caller
    /// ([`Tx::apply`](crate::Tx::apply)) rolls back on error and records the undo.
    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        match self {
            Self::SetInfo(op) => op.apply_to(doc),
            Self::RotatePage(op) => op.apply_to(doc).map(|pages| Effect {
                pages,
                ..Effect::default()
            }),
            Self::SetMetadata(op) => op.apply_to(doc),
            Self::RemoveMetadata(op) => op.apply_to(doc),
            Self::UpdateSaveMetadata(op) => op.apply_to(doc),
            Self::SetPageLabels(op) => op.apply_to(doc).map(|()| Effect::default()),
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.apply_to(doc, self.class()),
        }
    }
}

/// What applying one op did, besides the changes the journal records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Effect {
    /// The pages it touched.
    pub(crate) pages: Vec<PageId>,
    /// It rewrote the XMP packet whole, losing its layout and comments
    /// ([`Written::canonical`](laminapdf_model::xmp::Written::canonical)).
    pub(crate) packet_rewritten: bool,
    /// It removed metadata that the file's earlier revisions still hold.
    pub(crate) removed: bool,
    /// The form fields whose widgets it deleted, reshowed or moved.
    pub(crate) widgets: Vec<TouchedWidget>,
}

impl From<SetInfo> for Op {
    fn from(op: SetInfo) -> Self {
        Self::SetInfo(op)
    }
}

impl From<SetMetadata> for Op {
    fn from(op: SetMetadata) -> Self {
        Self::SetMetadata(op)
    }
}

impl From<RemoveMetadata> for Op {
    fn from(op: RemoveMetadata) -> Self {
        Self::RemoveMetadata(op)
    }
}

impl From<UpdateSaveMetadata> for Op {
    fn from(op: UpdateSaveMetadata) -> Self {
        Self::UpdateSaveMetadata(op)
    }
}

impl From<SetPageLabels> for Op {
    fn from(op: SetPageLabels) -> Self {
        Self::SetPageLabels(op)
    }
}

impl From<RotatePage> for Op {
    fn from(op: RotatePage) -> Self {
        Self::RotatePage(op)
    }
}

/// A list of ops with its schema version ([ADR 0006] §6): `{ "v": 1, "ops": [ … ] }`, used
/// for saved op logs, batch processing and sync messages. Unknown fields are rejected, and so
/// is a `v` this reader does not know; [`OpLog::from_json`] checks `v` before reading any op.
///
/// ```
/// use laminapdf_ops::OpLog;
///
/// let log = OpLog::from_json(r#"{ "v": 1, "ops": [{ "op": "rotatePage", "args": { "page": "p3.0", "rotate": 180 } }] }"#)?;
/// assert_eq!(log.ops().len(), 1);
/// assert_eq!(OpLog::from_json(r#"{ "v": 2, "ops": [] }"#).map_err(|e| e.code()), Err("unsupported-version"));
/// # Ok::<(), laminapdf_ops::Error>(())
/// ```
///
/// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct OpLog {
    #[serde(deserialize_with = "known_version")]
    #[cfg_attr(feature = "tsify", tsify(type = "1"))]
    v: u64,
    ops: Vec<Op>,
}

/// Rejects every `v` but [`OpLog::VERSION`].
fn known_version<'de, D: Deserializer<'de>>(d: D) -> core::result::Result<u64, D::Error> {
    let v = u64::deserialize(d)?;
    if v == OpLog::VERSION {
        Ok(v)
    } else {
        Err(serde::de::Error::custom(format_args!("unsupported op log version {v}")))
    }
}

impl OpLog {
    /// The schema version this crate reads and writes.
    ///
    /// Added in 0.2.0.
    pub const VERSION: u64 = 1;

    /// A log of `ops` at the current version.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(ops: Vec<Op>) -> Self {
        Self { v: Self::VERSION, ops }
    }

    /// The schema version.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.v
    }

    /// The ops, in order.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// The ops, giving up the log.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn into_ops(self) -> Vec<Op> {
        self.ops
    }

    /// Reads a log from JSON, checking `v` before any op: an unknown version is
    /// [`Error::UnsupportedVersion`] even when its ops would not parse.
    ///
    /// # Errors
    /// [`Error::UnsupportedVersion`]; [`Error::InvalidLog`] for anything else that does not
    /// parse (not JSON, no `v`, an unknown op or field, a bad value).
    ///
    /// Added in 0.2.0.
    pub fn from_json(json: &str) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(invalid)?;
        match value.get("v").and_then(serde_json::Value::as_u64) {
            Some(Self::VERSION) => serde_json::from_value(value).map_err(invalid),
            Some(v) => Err(Error::UnsupportedVersion(v)),
            None => Err(Error::InvalidLog("`v` is missing or not a version number".to_owned())),
        }
    }

    /// The log as JSON.
    ///
    /// # Errors
    /// [`Error::InvalidLog`] if serialisation fails (it does not for the ops of this crate).
    ///
    /// Added in 0.2.0.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(invalid)
    }
}

#[expect(clippy::needless_pass_by_value, reason = "used as `map_err(invalid)`")]
fn invalid(e: serde_json::Error) -> Error {
    Error::InvalidLog(e.to_string())
}
