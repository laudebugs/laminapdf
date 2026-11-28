//! The error type of the ops layer.

use core::fmt;

use laminapdf_cos::{ErrorKind, Location};
use laminapdf_model::{AnnotId, FieldId, OutlineItemId, PageId};

/// Everything that can go wrong while applying ops, running transactions or reading op logs.
///
/// `Display` formats only fixed text, numbers and object IDs (`p12.0`), never text from the
/// input or the op log (ADR 0026 §4). A path ID (`p5.0/Kids/3`) carries dictionary keys, so
/// it stays in the variant's field and is not printed.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error from the COS or model layer (reading the document, allocating a number).
    #[error(transparent)]
    Cos(laminapdf_cos::Error),
    /// A transaction was started (or undo/redo called) while one is open (ADR 0006 §2). Use
    /// [`Tx::savepoint`](crate::Tx::savepoint) to try something inside a transaction.
    #[error("a transaction is already open")]
    NestedTransaction,
    /// [`Document::apply`](crate::Document::apply), `commit` or `rollback` without an open
    /// transaction.
    #[error("no transaction is open")]
    NoTransaction,
    /// The op names a page the document does not have.
    #[error("no page {}", Shown(.0))]
    UnknownPage(PageId),
    /// The page is a direct dictionary inside its parent; promoting it to an indirect object
    /// (ADR 0005 §5) arrives with the page-structure ops.
    #[error("page {} is a direct dictionary, which ops cannot edit yet", Shown(.0))]
    DirectPage(PageId),
    /// An op log with a schema version this reader does not know (ADR 0006 §6).
    #[error("unsupported op log version {0}")]
    UnsupportedVersion(u64),
    /// An op log that does not parse: an unknown op or field, a bad value, or not JSON. The
    /// parser's message can quote the log, so it is kept here but never formatted
    /// (ADR 0026 §4).
    #[error("invalid op log")]
    InvalidLog(String),
    /// The document was saved to a sink the core can't read back and waits for
    /// [`Document::rebase`](crate::Document::rebase): ops, undo, redo and saves are refused
    /// until then (ADR 0005 §3).
    #[error("the document was saved but not rebased yet")]
    SavedNotRebased,
    /// [`Document::rebase`](crate::Document::rebase) without a save waiting for it: other
    /// bytes would not match the undo history, and unsaved changes would be lost.
    #[error("no save is waiting to be rebased onto")]
    NoPendingSave,
    /// A metadata edit would have to replace the document's XMP packet, which can't be read
    /// (`xmp-invalid`), losing whatever it holds. The edit is refused unless it says to
    /// replace the packet ([`SetMetadata::replace_malformed`](crate::SetMetadata::replace_malformed)).
    #[error("the XMP packet can't be read, and the edit would replace it")]
    MalformedXmp,
    /// An op, or a call, names a source document the session doesn't hold: add it with
    /// [`Document::add_source`](crate::Document::add_source) first (a restarted document worker re-adds
    /// its sources before it replays the log).
    #[error("no source document with this ID was added to the session")]
    UnknownSource(crate::SourceId),
    /// A [`SetInitialView`](crate::SetInitialView) would replace or remove the catalog's
    /// `/OpenAction`, which is an action other than `GoTo` (JavaScript, a named action, a link
    /// to another file). The op never replaces one: leave `open` out to keep it.
    #[error("the document's open action is not a destination, and the op would replace it")]
    OpenActionNotDestination,
    /// The op names a bookmark the outline does not have.
    #[error("the outline has no item {}", ShownItem(.0))]
    UnknownOutlineItem(OutlineItemId),
    /// The bookmark is a direct dictionary inside another item; the outline ops edit it once
    /// another outline op has rewritten the outline, which gives it an object of its own.
    #[error("outline item {} is a direct dictionary, which the outline ops can't name", ShownItem(.0))]
    DirectOutlineItem(OutlineItemId),
    /// A position names a bookmark that is not a child of the parent the op names.
    #[error("outline item {} is not a child of the given parent", ShownItem(.0))]
    NotASibling(OutlineItemId),
    /// A move would put a bookmark inside itself or one of its descendants.
    #[error("outline item {} can't move inside itself", ShownItem(.0))]
    OutlineIntoItself(OutlineItemId),
    /// The op names a destination the file does not define. The name is kept here but never
    /// formatted.
    #[error("no named destination has this name")]
    UnknownNamedDestination(String),
    /// A rename to a name another destination already has. The name is kept here but never
    /// formatted.
    #[error("another named destination already has this name")]
    NamedDestinationExists(String),
    /// A destination name is empty.
    #[error("a destination name can't be empty")]
    EmptyDestinationName,
    /// A web address that isn't an absolute `http`, `https` or `mailto` address in ASCII.
    #[error("only absolute http, https and mailto addresses are allowed")]
    LinkUriRefused,
    /// A destination's view has a number that isn't finite.
    #[error("a destination's coordinates and zoom must be finite numbers")]
    ViewNotFinite,
    /// An XMP edit would change the file's PDF/A or PDF/UA identification (`pdfaid:`,
    /// `pdfuaid:`): a claim only conversion or a check can make, never an edit.
    #[error("the PDF/A and PDF/UA identification can't be edited")]
    XmpClaimReadOnly,
    /// The packet given to [`ReplaceXmpPacket`](crate::ReplaceXmpPacket) doesn't parse.
    #[error("the XMP packet doesn't parse")]
    InvalidXmpPacket(laminapdf_model::xmp::ParseError),
    /// An XMP property edit names a property, or gives a value, that can't be written:
    /// [`reason`](Error::InvalidXmpProperty::reason) says which part.
    #[error("the XMP property or its value can't be written")]
    InvalidXmpProperty {
        /// `namespace`, `prefix`, `name`, `value` (an empty array, an array of mixed or nested
        /// values, a repeated language) or `shape` (a value of the wrong shape for a property
        /// the Info dictionary mirrors).
        reason: &'static str,
    },
    /// A [`DeletePages`](crate::DeletePages) would delete every page: a document needs a page
    /// to show, so delete all but one, or insert a page first.
    #[error("the edit would delete every page")]
    NoPagesLeft,
    /// An op's values are out of range: the reason names which (fixed text, never the op's).
    #[error("the op's values are out of range: {0}")]
    InvalidOp(&'static str),
    /// [`SetPageLabels`](crate::SetPageLabels)' ranges break a rule of ISO 32000-2 §12.4.2:
    /// `what` is `first-page` (the first range isn't on the first page), `order` (ranges out
    /// of page order, or two on one page) or `first-number` (a range numbered from 0).
    #[error("page label ranges are invalid ({what})")]
    InvalidLabelRanges {
        /// Which rule.
        what: &'static str,
    },
    /// An edit names a content object by a [`ContentObjectRef`](laminapdf_content::ContentObjectRef)
    /// whose hash doesn't match the object at that ordinal (ADR 0022 §2): the content changed
    /// since the name was taken, so the edit would change another object.
    #[error("content object {ordinal} is not the one the edit names")]
    ContentObjectStale {
        /// The object's ordinal among its owner's objects.
        ordinal: u32,
    },
    /// The content can't be edited (ADR 0022 §5 and §7), for the reason given.
    #[error("the content can't be edited ({})", .reason.as_str())]
    ContentNotEditable {
        /// Why.
        reason: laminapdf_content::NotEditable,
    },
    /// A text error: [`EditText`](crate::EditText)'s text has a character no code of the run's
    /// font draws (`missing-glyph`), or the font failed to read.
    #[cfg(feature = "content-edit")]
    #[error(transparent)]
    Text(laminapdf_text::Error),
    /// The op names an annotation that no page's `/Annots` lists.
    #[error("no annotation {}", ShownAnnot(.0))]
    UnknownAnnotation(AnnotId),
    /// The annotation's flags forbid the change (ISO 32000-2 Table 167): `Locked` forbids
    /// deleting it or changing its properties, `LockedContents` changing its contents. A
    /// change of the flags alone is always allowed, so a lock can be lifted.
    #[error("annotation {} is locked", ShownAnnot(.0))]
    AnnotationLocked(AnnotId),
    /// The annotation can't be edited by the annotation ops: a widget (a form field's, edited
    /// by the form ops), or a direct dictionary in `/Annots`, which has no object to edit.
    #[error("annotation {} can't be edited by the annotation ops", ShownAnnot(.0))]
    AnnotationNotEditable(AnnotId),
    /// A link op names an annotation that isn't a link (`/Subtype /Link`).
    #[error("annotation {} is not a link", ShownAnnot(.0))]
    NotALink(AnnotId),
    /// The file claims PDF/UA-1 but has no structure tree to put a new link in: a link
    /// without its `Link` element would break the claim (ISO 14289-1 §7.18.5).
    #[error("the link can't join the structure tree that a PDF/UA file needs")]
    LinkNotTagged,
    /// An annotation op needs an appearance stream drawn, and the document has no generator
    /// ([`Document::with_appearances`](crate::Document::with_appearances)): the `laminapdf`
    /// façade always sets one.
    #[error("the document has no appearance generator for annotations")]
    AppearanceUnavailable,
    /// A `FreeText` annotation needs text layout the appearance generator was built without: a
    /// font given by its bytes or a fallback font. In WebAssembly, open the document in the text
    /// module.
    ///
    /// Added in 0.5.0.
    #[error("drawing this text box needs the text layout that this build leaves out")]
    AppearanceNeedsText,
    /// A character of a text box is in none of its fonts: add a fallback font that has it.
    ///
    /// Added in 0.5.0.
    #[error("a character of the text box is in none of its fonts")]
    AnnotationGlyphMissing,
    /// A text box's font bytes aren't a font that can be read and embedded.
    ///
    /// Added in 0.5.0.
    #[error("a text box's font bytes aren't a font that can be embedded")]
    AnnotationFontInvalid,
    /// An image stamp's bytes aren't a JPEG or PNG image Lamina PDF can read.
    ///
    /// Added in 0.6.0.
    #[error("an image stamp's bytes aren't a JPEG or PNG image")]
    StampImageInvalid,
    /// An op names bytes by a [`BlobId`](crate::BlobId) the session doesn't hold: add them
    /// with [`Document::add_blob`](crate::Document::add_blob) first.
    #[error("no bytes with this ID were added to the session")]
    UnknownBlob(crate::BlobId),
    /// An op needs an image XObject written from a JPEG or PNG, and the document has no image
    /// writer ([`Document::with_images`](crate::Document::with_images)): the `laminapdf` façade
    /// always sets one.
    #[error("the document has no image writer for replacement images")]
    ImagesUnavailable,
    /// An attachment op names an attachment the document doesn't hold: it was deleted, or the
    /// ID comes from another document.
    ///
    /// Added in 0.6.0.
    #[error("no attachment {}", ShownAttachment(.0))]
    UnknownAttachment(laminapdf_model::AttachmentId),
    /// A file to attach is larger than the document's
    /// [`Limits::max_stream_decoded`](laminapdf_cos::Limits::max_stream_decoded): attached, it
    /// could never be read back out, so it isn't attached.
    ///
    /// Added in 0.6.0.
    #[error("the file is {size} bytes, over the {limit} bytes a document can read back out")]
    AttachmentTooLarge {
        /// The file's size in bytes.
        size: u64,
        /// The limit in bytes.
        limit: u64,
    },
    /// Flattening names a widget ([`FlattenAnnots`](crate::FlattenAnnots)): flattening form
    /// fields needs an edit that knows about their values, which Lamina PDF doesn't have yet, and
    /// a signature field is never flattened.
    ///
    /// Added in 0.6.0.
    #[error("annotation {} is a form field's widget, which flattening refuses", ShownAnnot(.0))]
    FlattenWidget(AnnotId),
    /// The op names a form field the form doesn't have: no terminal field of `/AcroForm
    /// /Fields` has this ID.
    ///
    /// Added in 0.6.0.
    #[error("no form field {}", ShownField(.0))]
    UnknownField(FieldId),
    /// A [`SetFieldValue`](crate::SetFieldValue) names a signature field: a signature's value
    /// is written by signing, never by filling.
    ///
    /// Added in 0.6.0.
    #[error("form field {} is a signature field, which filling can't change", ShownField(.0))]
    FieldIsSignature(FieldId),
    /// A [`SetFieldValue`](crate::SetFieldValue)'s value doesn't fit the field:
    /// [`reason`](Error::FieldValueInvalid::reason) says how.
    ///
    /// Added in 0.6.0.
    #[error("the value doesn't fit the form field ({reason})")]
    FieldValueInvalid {
        /// `kind` (a value of another kind of field, or any value for a push button),
        /// `max-len` (text longer than `/MaxLen`), `option` (a choice the options don't hold),
        /// `state` (a button state no widget has), `count` (several choices for a field that
        /// takes one, or none for a combo box), `format` (text the field's number, date, time or
        /// special format refuses) or `range` (a number outside its `AFRange_Validate` bounds).
        reason: &'static str,
    },
    /// An object added to a page has an ID another object Lamina PDF added to that page already
    /// has.
    ///
    /// Added in 0.7.0.
    #[error("another object added to the page already has this ID")]
    ContentIdInUse(crate::ContentId),
    /// The page draws no object Lamina PDF added with this ID.
    ///
    /// Added in 0.7.0.
    #[error("the page draws no added object with this ID")]
    UnknownContentObject(crate::ContentId),
    /// The page draws no page mark Lamina PDF added with this ID: a watermark, background, header
    /// or footer, or Bates number.
    ///
    /// Added in 0.7.0.
    #[error("the page draws no page mark with this ID")]
    UnknownPageMark(crate::ContentId),
    /// An added object's bytes aren't a JPEG or PNG image, or a PDF with the page it names, that
    /// Lamina PDF can read.
    ///
    /// Added in 0.7.0.
    #[error("an added object's bytes aren't a readable JPEG or PNG image, or a PDF with that page")]
    ContentObjectInvalid,
    /// An image or page added to a file that claims PDF/UA-1 has no alternate text and isn't
    /// marked as decoration (ISO 14289-1 §7.3).
    ///
    /// Added in 0.7.0.
    #[error("a picture added to a PDF/UA-1 file needs alternate text, or to be marked as decoration")]
    ContentNeedsAltText,
    /// A character of added text is in none of its fonts, so it can't be drawn.
    ///
    /// Added in 0.7.0.
    #[error("a character of the added text is in none of its fonts")]
    ContentGlyphMissing,
    /// A font given for added text isn't a font Lamina PDF can read.
    ///
    /// Added in 0.7.0.
    #[error("a font given for the added text isn't a readable font")]
    ContentFontInvalid,
    /// A font given for added text forbids embedding in its `OS/2` `fsType` (ISO 32000-2
    /// §9.9), so the text can't be drawn in it.
    ///
    /// Added in 0.7.0.
    #[error("a font given for the added text forbids being embedded")]
    ContentFontNotEmbeddable,
    /// [`SetEncryption`](crate::SetEncryption) with neither a user nor an owner password: the
    /// file would open for anyone and nobody could change its permissions.
    ///
    /// Added in 0.7.0.
    #[error("encryption needs a user password, an owner password or both")]
    EncryptionNoPassword,
    /// [`SetEncryption`](crate::SetEncryption) on a file that claims PDF/A, which forbids
    /// encryption (ISO 19005-2 §6.1.3). Remove the claim first, or keep the file unprotected.
    ///
    /// Added in 0.7.0.
    #[error("the file claims PDF/A, which forbids encryption")]
    EncryptionPdfaConflict,
    /// An op that needs a password or key got a secret that was never stored: an op log holds
    /// secrets as `null`, so it can't replay an encrypted save. Pass the password again.
    ///
    /// Added in 0.7.0.
    #[error("a password the op needs was left out of the op log")]
    SecretMissing,
}

/// A field ID as a message may show it, as [`Shown`] does pages.
struct ShownField<'a>(&'a FieldId);

impl fmt::Display for ShownField<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_lamina_id().obj_ref() {
            Some(_) => self.0.fmt(f),
            None => f.write_str("(a path ID)"),
        }
    }
}

/// An outline item ID as a message may show it, as [`Shown`] does pages.
struct ShownItem<'a>(&'a OutlineItemId);

impl fmt::Display for ShownItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_lamina_id().obj_ref() {
            Some(_) => self.0.fmt(f),
            None => f.write_str("(a path ID)"),
        }
    }
}

impl From<laminapdf_model::xmp::PropertyError> for Error {
    fn from(e: laminapdf_model::xmp::PropertyError) -> Self {
        use laminapdf_model::xmp::PropertyError as P;
        let reason = match e {
            P::Namespace => "namespace",
            P::Prefix => "prefix",
            P::Name => "name",
            P::Mapped(_) => "shape",
            _ => "value",
        };
        Self::InvalidXmpProperty { reason }
    }
}

/// An annotation ID as a message may show it, as [`Shown`] does for pages.
struct ShownAnnot<'a>(&'a AnnotId);

impl fmt::Display for ShownAnnot<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_lamina_id().obj_ref() {
            Some(_) => self.0.fmt(f),
            None => f.write_str("(a path ID)"),
        }
    }
}

/// An attachment ID as a message may show it: an object ID in full, a path ID without its keys.
struct ShownAttachment<'a>(&'a laminapdf_model::AttachmentId);

impl fmt::Display for ShownAttachment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_lamina_id().obj_ref() {
            Some(_) => self.0.fmt(f),
            None => f.write_str("(a path ID)"),
        }
    }
}

/// A page ID as a message may show it: an object ID in full (lamina-made: a kind and two
/// numbers), a path ID without its keys.
struct Shown<'a>(&'a PageId);

impl fmt::Display for Shown<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_lamina_id().obj_ref() {
            Some(_) => self.0.fmt(f),
            None => f.write_str("(a path ID)"),
        }
    }
}

impl From<laminapdf_model::PageTreeError> for Error {
    fn from(e: laminapdf_model::PageTreeError) -> Self {
        use laminapdf_model::PageTreeError as E;
        match e {
            E::Cos(e) => Self::Cos(e),
            E::UnknownPage(page) => Self::UnknownPage(page),
            // The ops insert only pages they make, so these name a page tree that changed
            // under them: report it as a broken structure.
            _ => Self::Cos(laminapdf_cos::Error::InvalidStructure {
                kind: "page tree",
                message: "a page is listed twice or is not a page",
            }),
        }
    }
}

impl From<laminapdf_cos::Error> for Error {
    fn from(e: laminapdf_cos::Error) -> Self {
        Self::Cos(e)
    }
}

impl Error {
    /// Every code [`Error::code`] can return besides those of [`laminapdf_cos::Error`] (listed in
    /// `fixtures/codes.json` under `opErrors`).
    ///
    /// Added in 0.2.0.
    pub const CODES: &'static [&'static str] = &[
        "nested-transaction",
        "no-transaction",
        "unknown-page",
        "direct-page",
        "unsupported-version",
        "invalid-op-log",
        "saved-not-rebased",
        "no-pending-save",
        "malformed-xmp",
        "unknown-source",
        "open-action-not-destination",
        "unknown-outline-item",
        "direct-outline-item",
        "outline-not-a-sibling",
        "outline-move-into-itself",
        "unknown-named-destination",
        "named-destination-exists",
        "empty-destination-name",
        "link-uri-refused",
        "view-not-finite",
        "xmp-claim-read-only",
        "invalid-xmp-packet",
        "invalid-xmp-property",
        "no-pages-left",
        "invalid-op",
        "unknown-annotation",
        "annotation-locked",
        "annotation-not-editable",
        "not-a-link",
        "link-not-tagged",
        "appearance-unavailable",
        "appearance-needs-text",
        "annotation-glyph-missing",
        "annotation-font-invalid",
        "stamp-image-invalid",
        "invalid-label-ranges",
        "content-object-stale",
        "content-not-editable",
        "unknown-blob",
        "images-unavailable",
        "flatten-widget",
        "unknown-attachment",
        "attachment-too-large",
        "unknown-field",
        "field-is-signature",
        "field-value-invalid",
        "content-id-in-use",
        "unknown-content-object",
        "unknown-page-mark",
        "content-object-invalid",
        "content-needs-alt-text",
        "content-glyph-missing",
        "content-font-invalid",
        "content-font-not-embeddable",
        "encryption-no-password",
        "encryption-pdfa-conflict",
        "secret-missing",
    ];

    /// A stable kebab-case code for this error; COS errors report [`laminapdf_cos::Error::code`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cos(e) => e.code(),
            #[cfg(feature = "content-edit")]
            Self::Text(e) => e.code(),
            Self::NestedTransaction => "nested-transaction",
            Self::NoTransaction => "no-transaction",
            Self::UnknownPage(_) => "unknown-page",
            Self::DirectPage(_) => "direct-page",
            Self::UnsupportedVersion(_) => "unsupported-version",
            Self::InvalidLog(_) => "invalid-op-log",
            Self::SavedNotRebased => "saved-not-rebased",
            Self::NoPendingSave => "no-pending-save",
            Self::MalformedXmp => "malformed-xmp",
            Self::UnknownSource(_) => "unknown-source",
            Self::OpenActionNotDestination => "open-action-not-destination",
            Self::UnknownOutlineItem(_) => "unknown-outline-item",
            Self::DirectOutlineItem(_) => "direct-outline-item",
            Self::NotASibling(_) => "outline-not-a-sibling",
            Self::OutlineIntoItself(_) => "outline-move-into-itself",
            Self::UnknownNamedDestination(_) => "unknown-named-destination",
            Self::NamedDestinationExists(_) => "named-destination-exists",
            Self::EmptyDestinationName => "empty-destination-name",
            Self::LinkUriRefused => "link-uri-refused",
            Self::ViewNotFinite => "view-not-finite",
            Self::XmpClaimReadOnly => "xmp-claim-read-only",
            Self::InvalidXmpPacket(_) => "invalid-xmp-packet",
            Self::InvalidXmpProperty { .. } => "invalid-xmp-property",
            Self::NoPagesLeft => "no-pages-left",
            Self::InvalidOp(_) => "invalid-op",
            Self::UnknownAnnotation(_) => "unknown-annotation",
            Self::AnnotationLocked(_) => "annotation-locked",
            Self::AnnotationNotEditable(_) => "annotation-not-editable",
            Self::NotALink(_) => "not-a-link",
            Self::LinkNotTagged => "link-not-tagged",
            Self::AppearanceUnavailable => "appearance-unavailable",
            Self::AppearanceNeedsText => "appearance-needs-text",
            Self::AnnotationGlyphMissing => "annotation-glyph-missing",
            Self::AnnotationFontInvalid => "annotation-font-invalid",
            Self::StampImageInvalid => "stamp-image-invalid",
            Self::InvalidLabelRanges { .. } => "invalid-label-ranges",
            Self::ContentObjectStale { .. } => "content-object-stale",
            Self::ContentNotEditable { .. } => "content-not-editable",
            Self::UnknownBlob(_) => "unknown-blob",
            Self::ImagesUnavailable => "images-unavailable",
            Self::FlattenWidget(_) => "flatten-widget",
            Self::UnknownAttachment(_) => "unknown-attachment",
            Self::AttachmentTooLarge { .. } => "attachment-too-large",
            Self::UnknownField(_) => "unknown-field",
            Self::FieldIsSignature(_) => "field-is-signature",
            Self::FieldValueInvalid { .. } => "field-value-invalid",
            Self::ContentIdInUse(_) => "content-id-in-use",
            Self::UnknownContentObject(_) => "unknown-content-object",
            Self::UnknownPageMark(_) => "unknown-page-mark",
            Self::ContentObjectInvalid => "content-object-invalid",
            Self::ContentNeedsAltText => "content-needs-alt-text",
            Self::ContentGlyphMissing => "content-glyph-missing",
            Self::ContentFontInvalid => "content-font-invalid",
            Self::ContentFontNotEmbeddable => "content-font-not-embeddable",
            Self::EncryptionNoPassword => "encryption-no-password",
            Self::EncryptionPdfaConflict => "encryption-pdfa-conflict",
            Self::SecretMissing => "secret-missing",
        }
    }

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Error::code`] (a test checks every variant). COS errors report
    /// [`laminapdf_cos::Error::kind`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Cos(e) => e.kind(),
            #[cfg(feature = "content-edit")]
            Self::Text(e) => e.kind(),
            Self::NestedTransaction | Self::NoTransaction | Self::SavedNotRebased | Self::NoPendingSave => {
                ErrorKind::State
            }
            Self::UnknownPage(_)
            | Self::UnknownSource(_)
            | Self::UnknownBlob(_)
            | Self::UnknownAttachment(_)
            | Self::UnknownOutlineItem(_)
            | Self::NotASibling(_)
            | Self::OutlineIntoItself(_)
            | Self::UnknownNamedDestination(_)
            | Self::NamedDestinationExists(_)
            | Self::EmptyDestinationName
            | Self::LinkUriRefused
            | Self::ViewNotFinite
            | Self::NoPagesLeft
            | Self::InvalidOp(_)
            | Self::AnnotationGlyphMissing
            | Self::AnnotationFontInvalid
            | Self::StampImageInvalid
            | Self::UnknownAnnotation(_)
            | Self::NotALink(_)
            | Self::FlattenWidget(_)
            | Self::UnknownField(_)
            | Self::FieldIsSignature(_)
            | Self::FieldValueInvalid { .. }
            | Self::ContentIdInUse(_)
            | Self::UnknownContentObject(_)
            | Self::UnknownPageMark(_)
            | Self::ContentObjectInvalid
            | Self::ContentNeedsAltText
            | Self::ContentGlyphMissing
            | Self::ContentFontInvalid
            | Self::ContentFontNotEmbeddable
            | Self::EncryptionNoPassword
            | Self::SecretMissing
            | Self::OpenActionNotDestination
            | Self::XmpClaimReadOnly
            | Self::InvalidXmpProperty { .. }
            | Self::InvalidLabelRanges { .. } => ErrorKind::Usage,
            Self::AnnotationLocked(_) | Self::ContentObjectStale { .. } | Self::EncryptionPdfaConflict => {
                ErrorKind::State
            }
            Self::DirectPage(_)
            | Self::UnsupportedVersion(_)
            | Self::DirectOutlineItem(_)
            | Self::AnnotationNotEditable(_)
            | Self::LinkNotTagged
            | Self::AppearanceUnavailable
            | Self::AppearanceNeedsText
            | Self::ImagesUnavailable
            | Self::ContentNotEditable { .. } => ErrorKind::Unsupported,
            Self::InvalidLog(_) | Self::MalformedXmp | Self::InvalidXmpPacket(_) => ErrorKind::Input,
            Self::AttachmentTooLarge { .. } => ErrorKind::Limit,
        }
    }

    /// Where the error was found (ADR 0026 §1.1): a COS error's position, or the page's
    /// object and ID when it is an object ID (a path ID's keys are not lamina-made, so a path
    /// ID gives no location).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Cos(e) => e.location(),
            #[cfg(feature = "content-edit")]
            Self::Text(e) => e.location(),
            Self::UnknownPage(page) | Self::DirectPage(page) => match page.as_lamina_id().obj_ref() {
                Some(r) => Location {
                    object: Some(r.num),
                    id: Some(page.to_string()),
                    ..Location::NONE
                },
                None => Location::NONE,
            },
            Self::UnknownAnnotation(annot)
            | Self::AnnotationLocked(annot)
            | Self::AnnotationNotEditable(annot)
            | Self::FlattenWidget(annot)
            | Self::NotALink(annot) => match annot.as_lamina_id().obj_ref() {
                Some(r) => Location {
                    object: Some(r.num),
                    id: Some(annot.to_string()),
                    ..Location::NONE
                },
                None => Location::NONE,
            },
            Self::UnknownField(field) | Self::FieldIsSignature(field) => match field.as_lamina_id().obj_ref() {
                Some(r) => Location {
                    object: Some(r.num),
                    id: Some(field.to_string()),
                    ..Location::NONE
                },
                None => Location::NONE,
            },
            Self::UnknownAttachment(id) => match id.as_lamina_id().obj_ref() {
                Some(r) => Location {
                    object: Some(r.num),
                    id: Some(id.to_string()),
                    ..Location::NONE
                },
                None => Location::NONE,
            },
            Self::UnknownOutlineItem(item)
            | Self::DirectOutlineItem(item)
            | Self::NotASibling(item)
            | Self::OutlineIntoItself(item) => match item.as_lamina_id().obj_ref() {
                Some(r) => Location {
                    object: Some(r.num),
                    id: Some(item.to_string()),
                    ..Location::NONE
                },
                None => Location::NONE,
            },
            Self::UnknownNamedDestination(_)
            | Self::NamedDestinationExists(_)
            | Self::EmptyDestinationName
            | Self::LinkUriRefused
            | Self::LinkNotTagged
            | Self::AppearanceUnavailable
            | Self::AppearanceNeedsText
            | Self::AnnotationGlyphMissing
            | Self::AnnotationFontInvalid
            | Self::StampImageInvalid
            | Self::ImagesUnavailable
            | Self::UnknownBlob(_)
            | Self::AttachmentTooLarge { .. }
            | Self::ViewNotFinite
            | Self::NestedTransaction
            | Self::NoTransaction
            | Self::UnsupportedVersion(_)
            | Self::InvalidLog(_)
            | Self::SavedNotRebased
            | Self::NoPendingSave
            | Self::MalformedXmp
            | Self::UnknownSource(_)
            | Self::OpenActionNotDestination
            | Self::XmpClaimReadOnly
            | Self::InvalidXmpPacket(_)
            | Self::InvalidXmpProperty { .. }
            | Self::NoPagesLeft
            | Self::InvalidOp(_)
            | Self::InvalidLabelRanges { .. }
            | Self::FieldValueInvalid { .. }
            | Self::ContentIdInUse(_)
            | Self::UnknownContentObject(_)
            | Self::UnknownPageMark(_)
            | Self::ContentObjectInvalid
            | Self::ContentNeedsAltText
            | Self::ContentGlyphMissing
            | Self::ContentFontInvalid
            | Self::ContentFontNotEmbeddable
            | Self::EncryptionNoPassword
            | Self::EncryptionPdfaConflict
            | Self::SecretMissing
            | Self::ContentObjectStale { .. }
            | Self::ContentNotEditable { .. } => Location::NONE,
        }
    }
}

/// Shorthand for `Result<T, laminapdf_ops::Error>`.
///
/// Added in 0.2.0.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    /// One value of every variant: the match in [`Error::kind`] has no wildcard, so a new
    /// variant is a compile error there, and this list is what the tests compare.
    fn samples() -> Vec<Error> {
        let page: PageId = "p3.0".parse().unwrap();
        let item: OutlineItemId = "o4.0".parse().unwrap();
        vec![
            Error::NestedTransaction,
            Error::NoTransaction,
            Error::UnknownPage(page.clone()),
            Error::DirectPage(page),
            Error::UnsupportedVersion(9),
            Error::InvalidLog(String::new()),
            Error::SavedNotRebased,
            Error::NoPendingSave,
            Error::MalformedXmp,
            Error::UnknownSource(crate::SourceId::from_bytes([0; 32])),
            Error::OpenActionNotDestination,
            Error::UnknownOutlineItem(item.clone()),
            Error::DirectOutlineItem(item.clone()),
            Error::NotASibling(item.clone()),
            Error::OutlineIntoItself(item),
            Error::UnknownNamedDestination(String::new()),
            Error::NamedDestinationExists(String::new()),
            Error::EmptyDestinationName,
            Error::LinkUriRefused,
            Error::ViewNotFinite,
            Error::XmpClaimReadOnly,
            Error::InvalidXmpPacket(laminapdf_model::xmp::ParseError::NoRdf),
            Error::InvalidXmpProperty { reason: "name" },
            Error::NoPagesLeft,
            Error::InvalidOp("a reason"),
            Error::UnknownAnnotation("a7.0".parse().unwrap()),
            Error::AnnotationLocked("a7.0".parse().unwrap()),
            Error::AnnotationNotEditable("a7.0".parse().unwrap()),
            Error::NotALink("a7.0".parse().unwrap()),
            Error::LinkNotTagged,
            Error::AppearanceUnavailable,
            Error::AppearanceNeedsText,
            Error::AnnotationGlyphMissing,
            Error::AnnotationFontInvalid,
            Error::StampImageInvalid,
            Error::InvalidLabelRanges { what: "order" },
            Error::ContentObjectStale { ordinal: 2 },
            Error::ContentNotEditable {
                reason: laminapdf_content::NotEditable::Type3Font,
            },
            Error::UnknownBlob(crate::BlobId::of(b"")),
            Error::ImagesUnavailable,
            Error::FlattenWidget("a7.0".parse().unwrap()),
            Error::UnknownAttachment("e9.0".parse().unwrap()),
            Error::AttachmentTooLarge { size: 2, limit: 1 },
            Error::UnknownField("f5.0".parse().unwrap()),
            Error::FieldIsSignature("f5.0".parse().unwrap()),
            Error::FieldValueInvalid { reason: "kind" },
            Error::ContentIdInUse(crate::ContentId::new("logo").unwrap()),
            Error::UnknownContentObject(crate::ContentId::new("logo").unwrap()),
            Error::UnknownPageMark(crate::ContentId::new("draft").unwrap()),
            Error::ContentObjectInvalid,
            Error::ContentNeedsAltText,
            Error::ContentGlyphMissing,
            Error::ContentFontInvalid,
            Error::ContentFontNotEmbeddable,
            Error::EncryptionNoPassword,
            Error::EncryptionPdfaConflict,
            Error::SecretMissing,
        ]
    }

    #[test]
    fn codes_lists_the_code_of_every_variant_in_order() {
        assert_eq!(codes::distinct(samples().iter().map(Error::code)), Error::CODES);
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        codes::assert_kinds_match(samples().iter().map(|e| (e.code(), e.kind().as_str())));
    }

    #[test]
    fn a_cos_error_keeps_its_kind() {
        assert_eq!(
            Error::Cos(laminapdf_cos::Error::WrongPassword).kind(),
            ErrorKind::Security
        );
    }
}
