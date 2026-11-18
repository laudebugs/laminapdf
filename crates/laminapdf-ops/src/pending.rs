//! [`PendingChanges`]: what the changes not yet saved are, for the save planner (ADR 0006 §1,
//! ADR 0007 §2).

use std::collections::BTreeSet;

#[cfg(feature = "content-edit")]
use laminapdf_cos::ObjRef;
use laminapdf_model::{AnnotId, AttachmentId, FieldId, PageId};

use crate::attachments::EmbeddedFileRight;
use crate::class::{AnnotRight, ModificationClass};
use crate::op::{Effect, Op};
use crate::widgets::TouchedWidget;

/// The kinds of change the document holds since it was opened or last rebased onto a save:
/// what the save planner checks against `DocMDP`, `FieldMDP` and UR3, and whether any of them
/// needs a full rewrite ([ADR 0007] §2).
///
/// It errs on the side of reporting: every committed transaction's op classes are added (an
/// undo or redo adds those of the transaction it reverts, with its usage rights turned round:
/// undoing an addition deletes), and only an empty overlay (nothing
/// left to save) or a rebase clears it. A class that is reported but whose change was undone
/// can make the planner refuse a save it could allow; a class that is missing would let a save
/// break a signature without saying so.
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about the pending changes, each read on its own by the planner"
)]
pub struct PendingChanges {
    classes: BTreeSet<ModificationClass>,
    annot_rights: BTreeSet<AnnotRight>,
    requires_rewrite: bool,
    unknown: bool,
    packet_rewritten: bool,
    widgets: BTreeSet<TouchedWidget>,
    filled: BTreeSet<FieldId>,
    tags_incomplete: BTreeSet<PageId>,
    tags_dropped: BTreeSet<PageId>,
    pdfa_unchecked: BTreeSet<PageId>,
    pdfua_unchecked: BTreeSet<PageId>,
    flattened: BTreeSet<AnnotId>,
    flattened_unembedded: BTreeSet<AnnotId>,
    // What content edits did, for their conformance warnings; only content editing compiles them.
    #[cfg(feature = "content-edit")]
    content_pages: BTreeSet<PageId>,
    #[cfg(feature = "content-edit")]
    dropped_xobjects: Vec<(PageId, ObjRef)>,
    #[cfg(feature = "content-edit")]
    alt_cleared: BTreeSet<ObjRef>,
    embedded_file_rights: BTreeSet<EmbeddedFileRight>,
    document_besides_attachments: bool,
    attached: BTreeSet<AttachmentId>,
    changed_attachments: BTreeSet<AttachmentId>,
}

impl PendingChanges {
    /// Changes of unknown kind (an overlay changed without ops, such as a model document
    /// wrapped while dirty): every class, and a full rewrite needed, since nothing tells
    /// otherwise.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            classes: [
                ModificationClass::Metadata,
                ModificationClass::FormFill,
                ModificationClass::Annotation,
                ModificationClass::Signature,
                ModificationClass::PageStructure,
                ModificationClass::Content,
                ModificationClass::Document,
            ]
            .into_iter()
            .collect(),
            annot_rights: AnnotRight::ALL.into_iter().collect(),
            requires_rewrite: true,
            unknown: true,
            packet_rewritten: false,
            widgets: BTreeSet::new(),
            filled: BTreeSet::new(),
            tags_incomplete: BTreeSet::new(),
            tags_dropped: BTreeSet::new(),
            pdfa_unchecked: BTreeSet::new(),
            pdfua_unchecked: BTreeSet::new(),
            flattened: BTreeSet::new(),
            flattened_unembedded: BTreeSet::new(),
            #[cfg(feature = "content-edit")]
            content_pages: BTreeSet::new(),
            #[cfg(feature = "content-edit")]
            dropped_xobjects: Vec::new(),
            #[cfg(feature = "content-edit")]
            alt_cleared: BTreeSet::new(),
            embedded_file_rights: [
                EmbeddedFileRight::Create,
                EmbeddedFileRight::Modify,
                EmbeddedFileRight::Delete,
            ]
            .into_iter()
            .collect(),
            document_besides_attachments: true,
            attached: BTreeSet::new(),
            changed_attachments: BTreeSet::new(),
        }
    }

    /// Whether the changes are of unknown kind ([`PendingChanges::unknown`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        self.unknown
    }

    /// The op classes of the changes not yet saved.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn classes(&self) -> &BTreeSet<ModificationClass> {
        &self.classes
    }

    /// The usage rights the annotation changes need ([`Op::annot_right`]): what the save
    /// planner checks against a UR3 signature's `/Annots`, right by right.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn annot_rights(&self) -> &BTreeSet<AnnotRight> {
        &self.annot_rights
    }

    /// Whether any of them needs a full rewrite ([`Op::requires_rewrite`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn requires_rewrite(&self) -> bool {
        self.requires_rewrite
    }

    /// Whether a metadata op rewrote the XMP packet whole (UTF-16, or read-only by its
    /// `end="r"` trailer), so the save can say the packet lost its layout and comments.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn packet_rewritten(&self) -> bool {
        self.packet_rewritten
    }

    /// The form fields whose widgets the changes deleted, reshowed or moved: what the save
    /// planner checks against signed signature fields and field locks, field by field
    /// ([E2 plan §6]). Changes of unknown kind ([`PendingChanges::unknown`]) name no field;
    /// they hold every class, which the planner already judges.
    ///
    /// [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn widgets(&self) -> &BTreeSet<TouchedWidget> {
        &self.widgets
    }

    /// The form fields whose values the changes set or cleared ([`Op::form_field`]): what the
    /// save planner checks against field locks, field by field ([E8 plan §5]). Changes of
    /// unknown kind ([`PendingChanges::unknown`]) name no field; the planner then holds every
    /// form change to every lock.
    ///
    /// [E8 plan §5]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn filled_fields(&self) -> &BTreeSet<FieldId> {
        &self.filled
    }

    /// The copies page duplications left partly untagged (`DuplicateTagsIncomplete`), which
    /// the save planner reports on a file that claims PDF/UA
    /// ([`ConformanceWarning::DuplicateTagsIncomplete`](crate::ConformanceWarning::DuplicateTagsIncomplete))
    /// while any of them is still a page: like the rest of these changes, the set only grows
    /// until a save, so an undone duplication stays listed.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn tags_incomplete(&self) -> &BTreeSet<PageId> {
        &self.tags_incomplete
    }

    /// The pages copied in from other documents without the tags their source pages had (into
    /// an untagged document), which the save planner reports on a file that claims PDF/UA
    /// ([`ConformanceWarning::ImportedTagsDropped`](crate::ConformanceWarning::ImportedTagsDropped))
    /// while any of them is still a page.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn tags_dropped(&self) -> &BTreeSet<PageId> {
        &self.tags_dropped
    }

    /// The pages copied in from a source that doesn't claim the PDF/A level this file claims,
    /// which the save planner reports
    /// ([`ConformanceWarning::ImportedPdfaUnchecked`](crate::ConformanceWarning::ImportedPdfaUnchecked))
    /// while any of them is still a page.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn pdfa_unchecked(&self) -> &BTreeSet<PageId> {
        &self.pdfa_unchecked
    }

    /// The pages copied in from a source that doesn't claim PDF/UA-1 into this file, which
    /// claims it; the save planner reports them
    /// ([`ConformanceWarning::ImportedPdfuaUnchecked`](crate::ConformanceWarning::ImportedPdfuaUnchecked))
    /// while any of them is still a page.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn pdfua_unchecked(&self) -> &BTreeSet<PageId> {
        &self.pdfua_unchecked
    }

    /// The annotations flattening drew into their pages and removed, which the save planner
    /// reports on a file that claims PDF/UA-1
    /// ([`ConformanceWarning::FlattenUntagged`](crate::ConformanceWarning::FlattenUntagged))
    /// while no page lists them: their text left the structure tree. Like the rest of these
    /// changes, the set only grows until a save, so an undone flattening stays listed, and the
    /// warning leaves it out once its annotations are back.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn flattened(&self) -> &BTreeSet<AnnotId> {
        &self.flattened
    }

    /// The flattened annotations whose appearances brought a font that isn't embedded into a
    /// file that claims PDF/A (`pdfa-font-not-embedded`).
    pub(crate) const fn flattened_unembedded(&self) -> &BTreeSet<AnnotId> {
        &self.flattened_unembedded
    }

    /// The pages whose content the changes edited: where an added font is looked for.
    #[cfg(feature = "content-edit")]
    pub(crate) const fn content_pages(&self) -> &BTreeSet<PageId> {
        &self.content_pages
    }

    /// The tagged XObjects content edits took off their pages (`content-tags-broken`).
    #[cfg(feature = "content-edit")]
    pub(crate) fn dropped_xobjects(&self) -> &[(PageId, ObjRef)] {
        &self.dropped_xobjects
    }

    /// The `Figure` elements whose image an edit replaced (`figure-without-alt`).
    #[cfg(feature = "content-edit")]
    pub(crate) const fn alt_cleared(&self) -> &BTreeSet<ObjRef> {
        &self.alt_cleared
    }

    /// The usage rights the attachment changes need ([`Op::embedded_file_right`]): what the save
    /// planner checks against a UR3 signature's `/EF`, right by right.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn embedded_file_rights(&self) -> &BTreeSet<EmbeddedFileRight> {
        &self.embedded_file_rights
    }

    /// Whether the changes of class `Document` include any besides the attachment ops': only
    /// then does a UR3 signature judge the class rather than its `/EF` rights.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn document_besides_attachments(&self) -> bool {
        self.document_besides_attachments
    }

    /// The attachments [`AttachFile`](crate::AttachFile) made, which the save planner checks
    /// against the file's PDF/A claim (`pdfa-attachment-forbidden`, `pdfa-attachment-incomplete`)
    /// while the document still holds them.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn attached(&self) -> &BTreeSet<AttachmentId> {
        &self.attached
    }

    /// The attachments [`UpdateAttachment`](crate::UpdateAttachment) changed, which the save
    /// planner checks against a PDF/A-3 claim (`pdfa-attachment-incomplete`): a change can drop
    /// what PDF/A-3 asks for, such as the MIME type.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn changed_attachments(&self) -> &BTreeSet<AttachmentId> {
        &self.changed_attachments
    }

    /// Whether there are none.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty() && !self.requires_rewrite
    }

    /// Adds `op`'s class and rewrite need, and what applying it did: whether it rewrote the
    /// packet whole, and the widgets it touched.
    pub(crate) fn record(&mut self, op: &Op, effect: &Effect) {
        self.classes.insert(op.class());
        self.annot_rights.extend(op.annot_right());
        self.embedded_file_rights.extend(op.embedded_file_right());
        // A deletion that rewrote a page's or an annotation's `/AF` changed that dictionary too:
        // held to the same rule as attaching to it (no usage right covers a page; an annotation
        // needs `/Annots /Modify`). Undoing it rewrites the same dictionaries, and keeps these.
        self.document_besides_attachments |=
            effect.rewrote_af.page || (op.class() == ModificationClass::Document && op.embedded_file_right().is_none());
        if effect.rewrote_af.annotation {
            self.annot_rights.insert(AnnotRight::Modify);
        }
        self.attached.extend(effect.created_attachments.iter().cloned());
        self.changed_attachments
            .extend(effect.changed_attachments.iter().cloned());
        self.requires_rewrite |= op.requires_rewrite();
        self.packet_rewritten |= effect.packet_rewritten;
        self.widgets.extend(effect.widgets.iter().cloned());
        self.filled.extend(op.form_field().cloned());
        self.tags_incomplete.extend(effect.tags_incomplete.iter().cloned());
        self.tags_dropped.extend(effect.tags_dropped.iter().cloned());
        self.pdfa_unchecked.extend(effect.pdfa_unchecked.iter().cloned());
        self.pdfua_unchecked.extend(effect.pdfua_unchecked.iter().cloned());
        self.flattened.extend(effect.flattened.iter().cloned());
        self.flattened_unembedded
            .extend(effect.flattened_unembedded.iter().cloned());
        #[cfg(feature = "content-edit")]
        {
            if op.class() == ModificationClass::Content {
                self.content_pages.extend(effect.pages.iter().cloned());
            }
            self.dropped_xobjects.extend(effect.dropped_xobjects.iter().cloned());
            self.dropped_xobjects.dedup();
            self.alt_cleared.extend(effect.alt_cleared.iter().copied());
        }
    }

    /// Adds everything `other` holds.
    pub(crate) fn merge(&mut self, other: &Self) {
        self.classes.extend(other.classes.iter().copied());
        self.annot_rights.extend(other.annot_rights.iter().copied());
        self.embedded_file_rights
            .extend(other.embedded_file_rights.iter().copied());
        self.document_besides_attachments |= other.document_besides_attachments;
        self.attached.extend(other.attached.iter().cloned());
        self.changed_attachments
            .extend(other.changed_attachments.iter().cloned());
        self.requires_rewrite |= other.requires_rewrite;
        self.unknown |= other.unknown;
        self.packet_rewritten |= other.packet_rewritten;
        self.widgets.extend(other.widgets.iter().cloned());
        self.filled.extend(other.filled.iter().cloned());
        self.tags_incomplete.extend(other.tags_incomplete.iter().cloned());
        self.tags_dropped.extend(other.tags_dropped.iter().cloned());
        self.pdfa_unchecked.extend(other.pdfa_unchecked.iter().cloned());
        self.pdfua_unchecked.extend(other.pdfua_unchecked.iter().cloned());
        self.flattened.extend(other.flattened.iter().cloned());
        self.flattened_unembedded
            .extend(other.flattened_unembedded.iter().cloned());
        #[cfg(feature = "content-edit")]
        {
            self.content_pages.extend(other.content_pages.iter().cloned());
            self.dropped_xobjects.extend(other.dropped_xobjects.iter().cloned());
            self.dropped_xobjects.dedup();
            self.alt_cleared.extend(other.alt_cleared.iter().copied());
        }
    }

    /// These changes as their reversion records them: the same, but with each usage right
    /// turned round ([`AnnotRight::reverted`], [`EmbeddedFileRight::reverted`]).
    pub(crate) fn reverted(&self) -> Self {
        Self {
            annot_rights: self.annot_rights.iter().map(|r| r.reverted()).collect(),
            embedded_file_rights: self.embedded_file_rights.iter().map(|r| r.reverted()).collect(),
            ..self.clone()
        }
    }

    /// Forgets everything (after a rebase, or when the overlay is empty again).
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}
