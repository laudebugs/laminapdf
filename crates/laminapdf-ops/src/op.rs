//! The typed, serialisable [`Op`] (ADR 0006 §1) and the versioned [`OpLog`] envelope (§6).
#![expect(
    deprecated,
    reason = "SetInfo is defined and dispatched here; it stays, deprecated, so v1 op logs still apply"
)]

use laminapdf_model::{OutlineItemId, PageId};
use serde::{Deserialize, Deserializer, Serialize};

use crate::annot::{AddAnnot, AddReply, Appearances, DeleteAnnot, FlattenAnnots, SetReviewState, UpdateAnnot};
use crate::attachments::{AttachFile, DeleteAttachment, EmbeddedFileRight, UpdateAttachment};
use crate::class::{AnnotRight, ModificationClass};
#[cfg(feature = "content-edit")]
use crate::content::{EditContent, EditText};
use crate::error::{Error, Result};
use crate::forms::{FlattenForm, SetFieldHidden, SetFieldValue};
use crate::images::ImageXObjects;
use crate::info::SetInfo;
use crate::initial_view::SetInitialView;
use crate::labels::SetPageLabels;
use crate::link::{AddLink, DeleteLink, UpdateLink};
use crate::metadata::{
    RemoveConformanceClaims, RemoveMetadata, RemoveXmpProperty, ReplaceXmpPacket, SetMetadata, SetXmpProperty,
    UpdateSaveMetadata,
};
use crate::outline::{
    AddOutlineItem, DeleteNamedDestination, DeleteOutlineItem, DestinationUse, MoveOutlineItem, RenameNamedDestination,
    SetNamedDestination, UpdateOutlineItem,
};
use crate::pages::{
    CropPages, DeletePages, DuplicatePages, FieldChange, InsertBlankPages, InsertPagesFrom, MergeDocuments, MovePages,
    ReferenceChange, RenamedDestination, ReplacePages, ResizePages, ReversePages, RotatePages, SetOrientation,
    SetPageBoxes, StructureRename,
};
use crate::rotate::RotatePage;
#[cfg(feature = "test-support")]
use crate::test_support::Declared;
use crate::widgets::{PageChange, TouchedWidget, WidgetChange};

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
    /// Change how the document asks to be shown when it opens.
    SetInitialView(SetInitialView),
    /// Add a bookmark.
    AddOutlineItem(AddOutlineItem),
    /// Change a bookmark's title, target, style or open state.
    UpdateOutlineItem(UpdateOutlineItem),
    /// Move a bookmark with its children.
    MoveOutlineItem(MoveOutlineItem),
    /// Delete a bookmark.
    DeleteOutlineItem(DeleteOutlineItem),
    /// Add a named destination or point it elsewhere.
    SetNamedDestination(SetNamedDestination),
    /// Rename a named destination (and, by default, its uses).
    RenameNamedDestination(RenameNamedDestination),
    /// Delete a named destination.
    DeleteNamedDestination(DeleteNamedDestination),
    /// Set any XMP property.
    SetXmpProperty(SetXmpProperty),
    /// Remove an XMP property.
    RemoveXmpProperty(RemoveXmpProperty),
    /// Replace the whole XMP packet.
    ReplaceXmpPacket(ReplaceXmpPacket),
    /// Insert blank pages.
    InsertBlankPages(InsertBlankPages),
    /// Delete pages, cleaning up what named them.
    DeletePages(DeletePages),
    /// Move pages to another position.
    MovePages(MovePages),
    /// Rotate pages, set or turned.
    RotatePages(RotatePages),
    /// Reverse pages within the places they hold.
    ReversePages(ReversePages),
    /// Copy pages within the document, with new annotations, fields and structure.
    DuplicatePages(DuplicatePages),
    /// Copy pages of a source document into this one.
    InsertPagesFrom(InsertPagesFrom),
    /// Replace pages with pages of a source document.
    ReplacePages(ReplacePages),
    /// Merge other documents into this one.
    MergeDocuments(MergeDocuments),
    /// Set or remove page boxes (media, crop, bleed, trim, art).
    SetPageBoxes(SetPageBoxes),
    /// Crop pages by margins of the page as displayed.
    CropPages(CropPages),
    /// Give pages new paper, scaling the content or extending the page around it.
    ResizePages(ResizePages),
    /// Show pages portrait or landscape, by `/Rotate` or by swapping the paper's sides.
    SetOrientation(SetOrientation),
    /// The save-time metadata stamp, made only by the save planner. It never crosses the wire:
    /// serialising it fails with serde's error (`OpLog::to_json`: `invalid-op-log`), and no op
    /// log can name it.
    #[serde(skip)]
    UpdateSaveMetadata(UpdateSaveMetadata),
    /// Add a text markup, a note or a caret, with its appearance.
    AddAnnot(AddAnnot),
    /// Change an annotation's common entries, or move it.
    UpdateAnnot(UpdateAnnot),
    /// Delete an annotation with its Popup, replies and group.
    DeleteAnnot(DeleteAnnot),
    /// Reply to a comment.
    AddReply(AddReply),
    /// Set a comment's review status.
    SetReviewState(SetReviewState),
    /// Draw annotations into their pages' content and remove them.
    FlattenAnnots(FlattenAnnots),
    /// Add a link on an area or on text.
    AddLink(AddLink),
    /// Change a link's area, quads, target or description.
    UpdateLink(UpdateLink),
    /// Delete a link, with its place in the structure tree.
    DeleteLink(DeleteLink),
    /// Replace the document's page labels.
    SetPageLabels(SetPageLabels),
    /// Set a form field's value, or clear it.
    SetFieldValue(SetFieldValue),
    /// Hide a form field's widgets, or show them again, as a Hide action does.
    SetFieldHidden(SetFieldHidden),
    /// Draw form fields into their pages and remove them.
    FlattenForm(FlattenForm),
    /// Remove the file's PDF/A or PDF/UA claim from its XMP metadata: the only op that may.
    RemoveConformanceClaims(RemoveConformanceClaims),
    /// Attach a file, to the document's list and what it is associated with.
    AttachFile(AttachFile),
    /// Change an attachment's name, description, type or relationship.
    UpdateAttachment(UpdateAttachment),
    /// Delete an attachment from the document's list and every `/AF` array.
    DeleteAttachment(DeleteAttachment),
    /// Change objects a page already draws: move, resize, rotate, delete, replace an image,
    /// crop or reorder them.
    #[cfg(feature = "content-edit")]
    EditContent(EditContent),
    /// Change the text a run of glyphs shows, in the font it already uses; empty text deletes
    /// the run.
    #[cfg(feature = "content-edit")]
    EditText(EditText),
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
            Self::SetInfo(_)
            | Self::SetMetadata(_)
            | Self::RemoveMetadata(_)
            | Self::SetXmpProperty(_)
            | Self::RemoveXmpProperty(_)
            | Self::ReplaceXmpPacket(_)
            | Self::RemoveConformanceClaims(_)
            | Self::UpdateSaveMetadata(_) => ModificationClass::Metadata,
            Self::RotatePage(_)
            | Self::InsertBlankPages(_)
            | Self::DeletePages(_)
            | Self::MovePages(_)
            | Self::RotatePages(_)
            | Self::ReversePages(_)
            | Self::DuplicatePages(_)
            | Self::InsertPagesFrom(_)
            | Self::ReplacePages(_)
            | Self::MergeDocuments(_)
            | Self::SetPageBoxes(_)
            | Self::CropPages(_)
            | Self::ResizePages(_)
            | Self::SetOrientation(_) => ModificationClass::PageStructure,
            Self::SetInitialView(_)
            | Self::AddOutlineItem(_)
            | Self::UpdateOutlineItem(_)
            | Self::MoveOutlineItem(_)
            | Self::DeleteOutlineItem(_)
            | Self::SetNamedDestination(_)
            | Self::RenameNamedDestination(_)
            | Self::DeleteNamedDestination(_)
            | Self::AttachFile(_)
            | Self::UpdateAttachment(_)
            | Self::DeleteAttachment(_) => ModificationClass::Document,
            Self::UpdateAnnot(_)
            | Self::DeleteAnnot(_)
            | Self::AddReply(_)
            | Self::SetReviewState(_)
            | Self::AddAnnot(_)
            | Self::AddLink(_)
            | Self::UpdateLink(_)
            | Self::DeleteLink(_)
            | Self::SetFieldHidden(_) => ModificationClass::Annotation,
            Self::SetPageLabels(_) => ModificationClass::Document,
            Self::SetFieldValue(_) => ModificationClass::FormFill,
            Self::FlattenAnnots(_) | Self::FlattenForm(_) => ModificationClass::Content,
            #[cfg(feature = "content-edit")]
            Self::EditContent(_) | Self::EditText(_) => ModificationClass::Content,
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
            | Self::SetInitialView(_)
            | Self::SetXmpProperty(_)
            | Self::RemoveXmpProperty(_)
            | Self::ReplaceXmpPacket(_)
            | Self::RemoveConformanceClaims(_)
            | Self::UpdateSaveMetadata(_)
            | Self::AddOutlineItem(_)
            | Self::UpdateOutlineItem(_)
            | Self::MoveOutlineItem(_)
            | Self::DeleteOutlineItem(_)
            | Self::SetNamedDestination(_)
            | Self::RenameNamedDestination(_)
            | Self::DeleteNamedDestination(_)
            | Self::InsertBlankPages(_)
            | Self::DeletePages(_)
            | Self::MovePages(_)
            | Self::RotatePages(_)
            | Self::ReversePages(_)
            | Self::DuplicatePages(_)
            | Self::InsertPagesFrom(_)
            | Self::ReplacePages(_)
            | Self::MergeDocuments(_)
            | Self::SetPageBoxes(_)
            | Self::CropPages(_)
            | Self::ResizePages(_)
            | Self::SetOrientation(_)
            | Self::SetPageLabels(_)
            | Self::SetFieldValue(_)
            | Self::SetFieldHidden(_)
            | Self::UpdateAnnot(_)
            | Self::DeleteAnnot(_)
            | Self::AddReply(_)
            | Self::SetReviewState(_)
            | Self::AddAnnot(_)
            | Self::AddLink(_)
            | Self::UpdateLink(_)
            | Self::FlattenAnnots(_)
            | Self::FlattenForm(_)
            | Self::DeleteLink(_)
            | Self::AttachFile(_)
            | Self::UpdateAttachment(_)
            | Self::DeleteAttachment(_) => false,
            #[cfg(feature = "content-edit")]
            Self::EditContent(_) | Self::EditText(_) => false,
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.op.requires_rewrite(),
        }
    }

    /// The usage right this op needs if it changes annotations: what `laminapdf-save`'s
    /// planner checks against a UR3 signature's `/Annots`, op by op ([E4 plan §6]). Adding an
    /// annotation, a link, a reply or a review state needs `/Create`, updating one `/Modify` and
    /// deleting one `/Delete`; an op that changes no annotation needs none.
    ///
    /// [E4 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn annot_right(&self) -> Option<AnnotRight> {
        match self {
            Self::AddAnnot(_) | Self::AddLink(_) | Self::AddReply(_) | Self::SetReviewState(_) => {
                Some(AnnotRight::Create)
            }
            // Hiding a field changes its widgets' flags.
            Self::UpdateAnnot(_) | Self::UpdateLink(_) | Self::SetFieldHidden(_) => Some(AnnotRight::Modify),
            // Associating a file with an annotation changes the annotation's dictionary (`/AF`).
            Self::AttachFile(op) if matches!(op.attach_to, crate::AttachTarget::Annotation(_)) => {
                Some(AnnotRight::Modify)
            }
            Self::DeleteAnnot(_) | Self::DeleteLink(_) => Some(AnnotRight::Delete),
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.op.annot_right(),
            _ => None,
        }
    }

    /// The usage right this op needs if it changes attachments: what `laminapdf-save`'s planner
    /// checks against a UR3 signature's `/EF`, op by op ([E3 plan §6]). Attaching a file needs
    /// `/Create`, changing one `/Modify` and deleting one `/Delete`; an op that changes no
    /// attachment needs none. Attaching a file to a page changes the page's dictionary (`/AF`),
    /// which no usage right covers, so that op needs none here and is judged as any other
    /// `Document` change; attaching one to an annotation also needs `/Annots /Modify`
    /// ([`Op::annot_right`]).
    ///
    /// [E3 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e3-navigation-attachments-inspection.md#6-saving-and-the-save-constraint
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn embedded_file_right(&self) -> Option<EmbeddedFileRight> {
        match self {
            Self::AttachFile(op) if matches!(op.attach_to, crate::AttachTarget::Page(_)) => None,
            Self::AttachFile(_) => Some(EmbeddedFileRight::Create),
            Self::UpdateAttachment(_) => Some(EmbeddedFileRight::Modify),
            Self::DeleteAttachment(_) => Some(EmbeddedFileRight::Delete),
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.op.embedded_file_right(),
            _ => None,
        }
    }

    /// The pages this op would delete, replace, reshow or move: what `laminapdf-save`'s
    /// `SavePlanner::would_violate` checks against the widgets on them before the op applies
    /// ([E2 plan §6]): deleted pages, rotated ones and moved or reversed ones. Inserting and
    /// duplicating pages (a copy's widgets belong to new fields) and metadata edits change none.
    ///
    /// [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn page_changes(&self) -> Vec<PageChange> {
        let each = |pages: &[PageId], change| pages.iter().map(|p| PageChange::new(p.clone(), change)).collect();
        match self {
            Self::SetInfo(_)
            | Self::SetMetadata(_)
            | Self::RemoveMetadata(_)
            | Self::SetInitialView(_)
            | Self::SetXmpProperty(_)
            | Self::RemoveXmpProperty(_)
            | Self::ReplaceXmpPacket(_)
            | Self::RemoveConformanceClaims(_)
            | Self::UpdateSaveMetadata(_)
            | Self::AddOutlineItem(_)
            | Self::UpdateOutlineItem(_)
            | Self::MoveOutlineItem(_)
            | Self::DeleteOutlineItem(_)
            | Self::SetNamedDestination(_)
            | Self::RenameNamedDestination(_)
            | Self::DeleteNamedDestination(_)
            | Self::InsertBlankPages(_)
            | Self::DuplicatePages(_)
            | Self::InsertPagesFrom(_)
            | Self::MergeDocuments(_)
            | Self::SetPageLabels(_)
            | Self::SetFieldValue(_)
            | Self::SetFieldHidden(_)
            | Self::UpdateAnnot(_)
            | Self::DeleteAnnot(_)
            | Self::AddReply(_)
            | Self::SetReviewState(_)
            | Self::AddAnnot(_)
            | Self::AddLink(_)
            | Self::UpdateLink(_)
            | Self::FlattenAnnots(_)
            | Self::FlattenForm(_)
            | Self::DeleteLink(_)
            | Self::AttachFile(_)
            | Self::UpdateAttachment(_)
            | Self::DeleteAttachment(_) => Vec::new(),
            #[cfg(feature = "content-edit")]
            Self::EditContent(_) | Self::EditText(_) => Vec::new(),
            Self::ReplacePages(op) => each(&op.targets, WidgetChange::Deleted),
            Self::RotatePage(op) => vec![PageChange::new(op.page.clone(), WidgetChange::Reshown)],
            Self::RotatePages(op) => each(&op.pages, WidgetChange::Reshown),
            Self::SetPageBoxes(op) => each(&op.pages, WidgetChange::Reshown),
            Self::CropPages(op) => each(&op.pages, WidgetChange::Reshown),
            Self::ResizePages(op) => each(&op.pages, WidgetChange::Reshown),
            Self::SetOrientation(op) => each(&op.pages, WidgetChange::Reshown),
            Self::DeletePages(op) => each(&op.pages, WidgetChange::Deleted),
            Self::MovePages(op) => each(&op.pages, WidgetChange::Moved),
            Self::ReversePages(op) => each(&op.pages, WidgetChange::Moved),
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.pages.clone(),
        }
    }

    /// Whether this op sets a form field's value: what runs the form's calculations after it
    /// (E8 plan §2.3).
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn sets_field_value(&self) -> bool {
        matches!(self, Self::SetFieldValue(_))
    }

    /// Every form field this op changes or removes: what `laminapdf-save`'s planner checks
    /// against field locks (`FieldMDP`, a signature field's `/Lock`), field by field ([E8 plan
    /// §5]). [`Op::form_field`] for a form-filling op, and each field
    /// [`FlattenForm`](crate::FlattenForm) names.
    ///
    /// [E8 plan §5]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn form_fields(&self) -> &[laminapdf_model::FieldId] {
        match self {
            Self::FlattenForm(op) => &op.fields,
            _ => self.form_field().map(std::slice::from_ref).unwrap_or_default(),
        }
    }

    /// The form field this op changes the value of, when it is a form-filling op: what
    /// `laminapdf-save`'s planner checks against field locks (`FieldMDP`, a signature field's
    /// `/Lock`), field by field ([E8 plan §5]).
    ///
    /// [E8 plan §5]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn form_field(&self) -> Option<&laminapdf_model::FieldId> {
        match self {
            Self::SetFieldValue(op) => Some(&op.field),
            Self::SetFieldHidden(op) => Some(&op.field),
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.op.form_field(),
            _ => None,
        }
    }

    /// Applies the op's writes through the overlay; returns what it did. The caller
    /// ([`Tx::apply`](crate::Tx::apply)) rolls back on error and records the undo.
    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        sources: &crate::document::Sources,
        appearances: Option<&dyn Appearances>,
        content: (&crate::BlobStore, Option<&dyn ImageXObjects>),
    ) -> Result<Effect> {
        match self {
            Self::SetInfo(op) => op.apply_to(doc),
            Self::RotatePage(op) => {
                let pages = op.apply_to(doc)?;
                let widgets =
                    crate::pages::widgets_on(doc, &pages, WidgetChange::Reshown, ModificationClass::PageStructure)?;
                Ok(Effect {
                    pages,
                    widgets,
                    ..Effect::default()
                })
            }
            Self::InsertBlankPages(op) => op.apply_to(doc),
            Self::DeletePages(op) => op.apply_to(doc),
            Self::MovePages(op) => op.apply_to(doc),
            Self::RotatePages(op) => op.apply_to(doc),
            Self::SetPageBoxes(op) => op.apply_to(doc),
            Self::CropPages(op) => op.apply_to(doc),
            Self::ResizePages(op) => op.apply_to(doc),
            Self::SetOrientation(op) => op.apply_to(doc),
            Self::ReversePages(op) => op.apply_to(doc),
            Self::DuplicatePages(op) => op.apply_to(doc),
            Self::InsertPagesFrom(op) => op.apply_to(doc, sources),
            Self::ReplacePages(op) => op.apply_to(doc, sources),
            Self::MergeDocuments(op) => op.apply_to(doc, sources),
            Self::SetMetadata(op) => op.apply_to(doc),
            Self::RemoveMetadata(op) => op.apply_to(doc),
            Self::SetInitialView(op) => op.apply_to(doc).map(|()| Effect::default()),
            Self::SetXmpProperty(op) => op.apply_to(doc),
            Self::RemoveXmpProperty(op) => op.apply_to(doc),
            Self::ReplaceXmpPacket(op) => op.apply_to(doc),
            Self::UpdateSaveMetadata(op) => op.apply_to(doc),
            Self::RemoveConformanceClaims(op) => op.apply_to(doc),
            Self::AddOutlineItem(op) => op.apply_to(doc),
            Self::UpdateOutlineItem(op) => op.apply_to(doc),
            Self::MoveOutlineItem(op) => op.apply_to(doc),
            Self::DeleteOutlineItem(op) => op.apply_to(doc),
            Self::SetNamedDestination(op) => op.apply_to(doc),
            Self::RenameNamedDestination(op) => op.apply_to(doc),
            Self::DeleteNamedDestination(op) => op.apply_to(doc),
            Self::AddAnnot(op) => op.apply_to(doc, appearances, content.0),
            Self::UpdateAnnot(op) => op.apply_to(doc, appearances, content.0),
            Self::DeleteAnnot(op) => op.apply_to(doc),
            Self::AddReply(op) => op.apply_to(doc),
            Self::SetReviewState(op) => op.apply_to(doc),
            Self::FlattenAnnots(op) => op.apply_to(doc, appearances, content.0),
            Self::FlattenForm(op) => op.apply_to(doc, appearances, content.0),
            Self::AddLink(op) => op.apply_to(doc),
            Self::UpdateLink(op) => op.apply_to(doc),
            Self::DeleteLink(op) => op.apply_to(doc),
            Self::SetPageLabels(op) => op.apply_to(doc).map(|()| Effect::default()),
            Self::SetFieldValue(op) => Ok(Effect {
                pages: op.apply_to(doc, appearances, content.0)?,
                ..Effect::default()
            }),
            Self::SetFieldHidden(op) => Ok(Effect {
                pages: op.apply_to(doc)?,
                ..Effect::default()
            }),
            Self::AttachFile(op) => op.apply_to(doc, content.0),
            Self::UpdateAttachment(op) => op.apply_to(doc),
            Self::DeleteAttachment(op) => op.apply_to(doc),
            #[cfg(feature = "content-edit")]
            Self::EditContent(op) => op.apply_to(doc, content.0, content.1),
            #[cfg(feature = "content-edit")]
            Self::EditText(op) => op.apply_to(doc),
            #[cfg(feature = "test-support")]
            Self::Declared(op) => op.apply_to(doc, sources, self.class(), content),
        }
    }
}

/// Which kinds of dictionary an op rewrote the `/AF` of: a page's, which no usage right covers,
/// and an annotation's, which a usage-rights signature allows only with `/Annots /Modify`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AfOwners {
    pub(crate) page: bool,
    pub(crate) annotation: bool,
}

/// What applying one op did, besides the changes the journal records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about what the ops did, each read on its own"
)]
pub(crate) struct Effect {
    /// The pages it touched.
    pub(crate) pages: Vec<PageId>,
    /// It rewrote the XMP packet whole, losing its layout and comments
    /// ([`Written::canonical`](laminapdf_model::xmp::Written::canonical)).
    pub(crate) packet_rewritten: bool,
    /// It removed something that the file's earlier revisions still hold.
    pub(crate) removed: bool,
    /// The properties it declared in a PDF/A packet's extension schemas.
    pub(crate) declared: Vec<laminapdf_model::xmp::ExtensionProperty>,
    /// Annotations it deleted besides the one it names (Popups, replies, a group).
    pub(crate) also_deleted: Vec<laminapdf_model::AnnotId>,
    /// The form fields whose widgets it deleted, reshowed or moved.
    pub(crate) widgets: Vec<TouchedWidget>,
    /// The bookmarks it created.
    pub(crate) created_outline_items: Vec<OutlineItemId>,
    /// The annotations it created.
    pub(crate) created_annotations: Vec<laminapdf_model::AnnotId>,
    /// It rewrote an outline whose links were broken.
    pub(crate) outline_repaired: bool,
    /// What still names a destination it deleted.
    pub(crate) destination_uses: Vec<DestinationUse>,
    /// The page IDs in order, when it changed the page list.
    pub(crate) page_order: Option<Vec<PageId>>,
    /// The references to deleted pages it retargeted or removed.
    pub(crate) references: Vec<ReferenceChange>,
    /// The form fields whose widgets it copied (renamed) or left out (signatures).
    pub(crate) fields: Vec<FieldChange>,
    /// The copies whose tags it couldn't clone whole (`DuplicateTagsIncomplete`).
    pub(crate) tags_incomplete: Vec<PageId>,
    /// The copy holding each link it removed because the link's destination didn't come along.
    pub(crate) links_removed: Vec<PageId>,
    /// The copies of tagged pages it brought in from another document without their tags.
    pub(crate) tags_dropped: Vec<PageId>,
    /// The named destinations it brought in under another name.
    pub(crate) renamed_destinations: Vec<RenamedDestination>,
    /// The role, class and element names it brought in under another name.
    pub(crate) structure_renamed: Vec<StructureRename>,
    /// The pages whose new crop box hides content that stays in the file.
    pub(crate) cropped_content_remains: Vec<PageId>,
    /// The bookmarks it left out because the pages they go to didn't come along.
    pub(crate) bookmarks_dropped: usize,
    /// The copies it brought into a file claiming PDF/A from a source without that claim.
    pub(crate) pdfa_unchecked: Vec<PageId>,
    /// The copies it brought into a file claiming PDF/UA-1 from a source without that claim.
    pub(crate) pdfua_unchecked: Vec<PageId>,
    /// The text boxes whose rich text lost styles outside the subset when the op drew them again.
    pub(crate) rich_text_simplified: Vec<laminapdf_model::AnnotId>,
    /// The annotations it drew into their pages and removed.
    pub(crate) flattened: Vec<laminapdf_model::AnnotId>,
    /// The annotations it left in place for want of an appearance.
    pub(crate) not_flattened: Vec<laminapdf_model::AnnotId>,
    /// The form fields it removed, each with its fully qualified name.
    pub(crate) flattened_fields: Vec<(laminapdf_model::FieldId, String)>,
    /// Whether it removed the form's `/XFA` while other fields were left.
    pub(crate) xfa_removed: bool,
    /// The flattened annotations whose appearances name a font that isn't embedded, in a file
    /// that claims PDF/A.
    pub(crate) flattened_unembedded: Vec<laminapdf_model::AnnotId>,
    /// The tagged XObjects (a form with its own `/StructParents`, or one an `OBJR` names) whose
    /// drawing a content edit took off a page.
    #[cfg(feature = "content-edit")]
    pub(crate) dropped_xobjects: Vec<(PageId, laminapdf_cos::ObjRef)>,
    /// The `Figure` elements whose image a content edit replaced, and whose alternative text it
    /// removed.
    #[cfg(feature = "content-edit")]
    pub(crate) alt_cleared: Vec<laminapdf_cos::ObjRef>,
    /// The attachments it made.
    pub(crate) created_attachments: Vec<laminapdf_model::AttachmentId>,
    /// The attachments it changed, which the save planner checks against a PDF/A-3 claim again.
    pub(crate) changed_attachments: Vec<laminapdf_model::AttachmentId>,
    /// The owners whose `/AF` it rewrote besides the catalog's (deleting a file they hold).
    pub(crate) rewrote_af: AfOwners,
    /// The files it deleted that `FileAttachment` annotations still hold.
    pub(crate) attachments_held: Vec<crate::AttachmentHeld>,
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

impl From<RemoveConformanceClaims> for Op {
    fn from(op: RemoveConformanceClaims) -> Self {
        Self::RemoveConformanceClaims(op)
    }
}

impl From<UpdateSaveMetadata> for Op {
    fn from(op: UpdateSaveMetadata) -> Self {
        Self::UpdateSaveMetadata(op)
    }
}

macro_rules! op_from {
    ($($name:ident),*) => {
        $(impl From<$name> for Op {
            fn from(op: $name) -> Self {
                Self::$name(op)
            }
        })*
    };
}

op_from!(
    SetFieldValue,
    SetFieldHidden,
    FlattenForm,
    SetInitialView,
    AddOutlineItem,
    UpdateOutlineItem,
    MoveOutlineItem,
    DeleteOutlineItem,
    SetNamedDestination,
    RenameNamedDestination,
    DeleteNamedDestination,
    SetXmpProperty,
    RemoveXmpProperty,
    ReplaceXmpPacket,
    InsertBlankPages,
    DeletePages,
    MovePages,
    RotatePages,
    ReversePages,
    DuplicatePages,
    AddAnnot,
    InsertPagesFrom,
    ReplacePages,
    MergeDocuments,
    SetPageBoxes,
    CropPages,
    ResizePages,
    SetOrientation,
    UpdateAnnot,
    DeleteAnnot,
    AddReply,
    SetReviewState,
    FlattenAnnots,
    AddLink,
    UpdateLink,
    DeleteLink,
    AttachFile,
    UpdateAttachment,
    DeleteAttachment
);

#[cfg(feature = "content-edit")]
impl From<EditContent> for Op {
    fn from(op: EditContent) -> Self {
        Self::EditContent(op)
    }
}

#[cfg(feature = "content-edit")]
impl From<EditText> for Op {
    fn from(op: EditText) -> Self {
        Self::EditText(op)
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
