//! The records of the document API (ADR 0028 F5, F6): what JavaScript passes in (`…Init`,
//! every field optional, unknown keys refused) and what it gets back. Each derives `Tsify`,
//! so the generated `.d.ts` declares it; names are camelCase, string unions camelCase too
//! (as the op JSON's `"op": "setInfo"`, ADR 0016 §7).

use laminapdf::{IncrementalOutput, Limits, OpenOptions, Renumber, SaveMode, SaveOptions, SaveReport, XrefKind};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::error::{JsResult, invalid_argument};

/// Options for `WasmDocument.openBytes`.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OpenOptionsInit {
    /// The user or owner password of an encrypted document.
    #[tsify(optional)]
    pub password: Option<String>,
    /// Refuse files that need repair (`strict`).
    #[tsify(optional)]
    pub strict: Option<bool>,
    /// Resource limits (ADR 0003 §3); each one left out keeps its default.
    #[tsify(optional)]
    pub limits: Option<LimitsInit>,
}

/// Declares [`LimitsInit`] and its conversion from one list of `(field, setter, type)`.
macro_rules! limits_init {
    ($( $field:ident: $ty:ty => $with:ident; )*) => {
        /// `Limits` with every field optional (ADR 0003 §3; names as in Rust, camelCase).
        #[derive(Debug, Default, Deserialize, Tsify)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub(crate) struct LimitsInit {
            $(
                #[doc = concat!("`", stringify!($field), "`.")]
                #[tsify(optional)]
                pub(crate) $field: Option<u64>,
            )*
        }

        impl LimitsInit {
            fn to_limits(&self) -> JsResult<Limits> {
                let mut limits = Limits::new();
                $(
                    if let Some(value) = self.$field {
                        let value = <$ty>::try_from(value).map_err(|_| {
                            invalid_argument("options.limits", format!("{} is too large", stringify!($field)), Vec::new())
                        })?;
                        limits = limits.$with(value);
                    }
                )*
                Ok(limits)
            }
        }
    };
}

limits_init! {
    max_nesting_depth: usize => with_max_nesting_depth;
    max_stream_decoded: usize => with_max_stream_decoded;
    max_decoded_total: usize => with_max_decoded_total;
    max_filter_chain: usize => with_max_filter_chain;
    max_resolve_depth: usize => with_max_resolve_depth;
    max_page_tree_depth: usize => with_max_page_tree_depth;
    max_tree_depth: usize => with_max_tree_depth;
    max_string_length: usize => with_max_string_length;
    max_xref_sections: usize => with_max_xref_sections;
    max_object_number: u32 => with_max_object_number;
    max_warnings: usize => with_max_warnings;
    max_content_operators: u64 => with_max_content_operators;
    max_xobject_depth: usize => with_max_xobject_depth;
    max_image_pixels: u64 => with_max_image_pixels;
    max_read_window: usize => with_max_read_window;
}

impl OpenOptionsInit {
    /// The façade's options.
    pub(crate) fn to_options(&self) -> JsResult<OpenOptions> {
        let mut options = OpenOptions::default().with_strict(self.strict.unwrap_or(false));
        if let Some(limits) = &self.limits {
            options = options.with_limits(limits.to_limits()?);
        }
        if let Some(password) = &self.password {
            options = options.with_password(password.as_bytes());
        }
        Ok(options)
    }
}

/// The save mode (ADR 0007 §3).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SaveModeJs {
    /// Incremental when allowed, else a full rewrite (the default).
    Auto,
    /// Append a revision.
    Incremental,
    /// Rewrite the whole file.
    FullRewrite,
}

/// What an incremental save returns.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum OutputJs {
    /// The whole file (the default).
    Whole,
    /// Only the appended revision (F7).
    AppendedOnly,
}

/// Object numbering in a full rewrite.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RenumberJs {
    /// Keep the numbers (the default).
    Keep,
    /// Number objects densely from 1.
    Compact,
}

/// The cross-reference form.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum XrefJs {
    /// Cross-reference and object streams (the default).
    Stream,
    /// A classic `xref` table.
    Classic,
}

/// Options for saving.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveOptionsInit {
    /// The mode (default `auto`).
    #[tsify(optional)]
    pub mode: Option<SaveModeJs>,
    /// Save even when it invalidates signatures (removing them from the output).
    #[tsify(optional)]
    pub invalidate_signatures: Option<bool>,
    /// Write an encrypted document as a plaintext full rewrite.
    #[tsify(optional)]
    pub remove_encryption: Option<bool>,
    /// What an incremental save returns.
    #[tsify(optional)]
    pub output: Option<OutputJs>,
    /// Numbering in a full rewrite.
    #[tsify(optional)]
    pub renumber: Option<RenumberJs>,
    /// The cross-reference form.
    #[tsify(optional)]
    pub xref: Option<XrefJs>,
    /// When the save happens, as the app's clock reads it: a PDF date (`D:20261110093000Z`)
    /// or an ISO 8601 date-time. A save that writes changes stamps it as the modification and
    /// metadata dates and gives the XMP packet a new instance ID.
    #[tsify(optional)]
    pub metadata_date: Option<String>,
    /// The XMP instance ID the save writes, a UUID (`uuid:` optional); without it, one is
    /// derived from the content.
    #[tsify(optional)]
    pub instance_id: Option<String>,
}

impl SaveOptionsInit {
    /// The façade's options.
    ///
    /// # Errors
    /// `invalid-argument` for a `metadataDate` that is no date or an `instanceId` that is no
    /// UUID.
    pub(crate) fn to_options(&self) -> JsResult<SaveOptions> {
        let mut options = SaveOptions::new();
        if let Some(date) = &self.metadata_date {
            options = options.with_metadata_date(parse_date(date)?);
        }
        if let Some(id) = &self.instance_id {
            options = options.with_instance_id(parse_uuid(id)?);
        }
        if let Some(mode) = self.mode {
            options = options.with_mode(match mode {
                SaveModeJs::Auto => SaveMode::Auto,
                SaveModeJs::Incremental => SaveMode::Incremental,
                SaveModeJs::FullRewrite => SaveMode::FullRewrite,
            });
        }
        if let Some(on) = self.invalidate_signatures {
            options = options.with_invalidate_signatures(on);
        }
        if let Some(on) = self.remove_encryption {
            options = options.with_remove_encryption(on);
        }
        if let Some(output) = self.output {
            options = options.with_output(match output {
                OutputJs::Whole => IncrementalOutput::Whole,
                OutputJs::AppendedOnly => IncrementalOutput::AppendedOnly,
            });
        }
        if let Some(renumber) = self.renumber {
            options = options.with_renumber(match renumber {
                RenumberJs::Keep => Renumber::Keep,
                RenumberJs::Compact => Renumber::Compact,
            });
        }
        if let Some(xref) = self.xref {
            options = options.with_xref(match xref {
                XrefJs::Stream => XrefKind::Stream,
                XrefJs::Classic => XrefKind::Classic,
            });
        }
        Ok(options)
    }
}

/// A PDF date (`D:…`) or an ISO 8601 date-time.
fn parse_date(date: &str) -> JsResult<laminapdf::PdfDate> {
    let parsed = if date.trim_start().starts_with("D:") {
        date.parse().ok()
    } else {
        laminapdf::PdfDate::from_iso8601(date).ok()
    };
    parsed.ok_or_else(|| {
        invalid_argument(
            "options",
            "metadataDate is neither a PDF date nor an ISO 8601 date-time".to_owned(),
            Vec::new(),
        )
    })
}

/// A UUID's 16 bytes, from its hexadecimal form with or without dashes and `uuid:`.
fn parse_uuid(id: &str) -> JsResult<[u8; 16]> {
    let hex: Vec<u8> = id
        .trim()
        .trim_start_matches("uuid:")
        .bytes()
        .filter(|&b| b != b'-')
        .collect();
    if hex.len() != 32 {
        return Err(bad_uuid());
    }
    let mut out = [0u8; 16];
    for (slot, pair) in out.iter_mut().zip(hex.as_chunks::<2>().0) {
        *slot = std::str::from_utf8(pair)
            .ok()
            .and_then(|p| u8::from_str_radix(p, 16).ok())
            .ok_or_else(bad_uuid)?;
    }
    Ok(out)
}

fn bad_uuid() -> wasm_bindgen::JsValue {
    invalid_argument("options", "instanceId is not a UUID".to_owned(), Vec::new())
}

/// One page's facts (`laminapdf::PageInfo`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageInfoJs {
    /// The 0-based index.
    pub index: u32,
    /// The stable page ID (`p12.0`).
    pub id: String,
    /// Displayed width in points.
    pub width: f64,
    /// Displayed height in points.
    pub height: f64,
    /// `/Rotate` in degrees.
    pub rotation: u16,
    /// The page label, when the document defines one (document text).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub label: Option<String>,
}

impl From<laminapdf::PageInfo> for PageInfoJs {
    fn from(p: laminapdf::PageInfo) -> Self {
        Self {
            index: p.index,
            id: p.id.to_string(),
            width: p.width,
            height: p.height,
            rotation: p.rotation,
            label: p.label,
        }
    }
}

/// The Info dictionary's text entries; absent ones are left out (document text).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentInfoJs {
    /// `/Title`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub title: Option<String>,
    /// `/Author`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub author: Option<String>,
    /// `/Subject`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub subject: Option<String>,
    /// `/Keywords`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub keywords: Option<String>,
    /// `/Creator`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub creator: Option<String>,
    /// `/Producer`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub producer: Option<String>,
}

impl From<laminapdf::DocumentInfo> for DocumentInfoJs {
    fn from(i: laminapdf::DocumentInfo) -> Self {
        Self {
            title: i.title,
            author: i.author,
            subject: i.subject,
            keywords: i.keywords,
            creator: i.creator,
            producer: i.producer,
        }
    }
}

/// The pages an edit touched.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a JSON record: one flag per fact the report states"
)]
pub(crate) struct EditedJs {
    /// IDs of the pages whose content or attributes changed.
    pub touched_pages: Vec<String>,
    /// A metadata edit rewrote the XMP packet whole (UTF-16, or marked read-only).
    pub packet_rewritten: bool,
    /// A metadata edit removed values the file's earlier revisions still hold, which only a
    /// full rewrite drops.
    pub still_in_earlier_revisions: bool,
    /// IDs of annotations an annotation op deleted besides the one it names (Popups, replies,
    /// the rest of a group); left out when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[tsify(optional)]
    pub also_deleted: Vec<String>,
    /// Whether the edit changed the document: one that only set values they already had leaves
    /// no step to undo.
    pub changed: bool,
    /// The IDs of the bookmarks the edit added, in order.
    pub created_outline_items: Vec<String>,
    /// The IDs of the links the edit added, in order.
    pub created_annotations: Vec<String>,
    /// An outline edit found the outline's links broken and rewrote them whole; bookmarks
    /// stored inside others have new IDs.
    pub outline_repaired: bool,
    /// What still names a named destination the edit deleted.
    pub destination_uses: Vec<laminapdf::DestinationUse>,
    /// The XMP properties the edit declared in a PDF/A file's extension schemas.
    pub declared: Vec<ExtensionPropertyJs>,
    /// The page IDs in order after the edit, when it changed the page list.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub page_order: Option<Vec<String>>,
    /// What a page deletion did to the references to the deleted pages.
    pub references: Vec<ReferenceEditJs>,
    /// What a page duplication did to the form fields whose widgets the copied pages show;
    /// left out when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[tsify(optional)]
    pub fields: Vec<FieldEditJs>,
    /// The copies a page duplication made whose tags couldn't all be cloned; left out when
    /// there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[tsify(optional)]
    pub tags_incomplete: Vec<String>,
}

/// What a page duplication did to one form field.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FieldEditJs {
    /// The field on the source page.
    pub field: String,
    /// The copy whose widget it concerns.
    pub page: String,
    /// `renamed` (the copy's widget belongs to a new field) or `notCopied` (a signature's
    /// widget).
    #[tsify(type = "\"renamed\" | \"notCopied\"")]
    pub outcome: &'static str,
    /// The new field's ID, when renamed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub to: Option<String>,
    /// The new field's fully qualified name, when renamed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub name: Option<String>,
}

impl From<laminapdf::FieldChange> for FieldEditJs {
    fn from(c: laminapdf::FieldChange) -> Self {
        let (outcome, to, name) = match c.outcome {
            laminapdf::FieldOutcome::Renamed { to, name } => ("renamed", Some(to.to_string()), Some(name)),
            _ => ("notCopied", None, None),
        };
        Self {
            field: c.field.to_string(),
            page: c.page.to_string(),
            outcome,
            to,
            name,
        }
    }
}

/// One reference to a deleted page and what the deletion did to it.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReferenceEditJs {
    /// The deleted page it named.
    pub page: String,
    /// `outlineItem`, `link`, `namedDestination`, `openAction`, `structureElement`,
    /// `structureKid` or `widget`.
    #[tsify(
        type = "\"outlineItem\" | \"link\" | \"namedDestination\" | \"openAction\" | \"structureElement\" | \"structureKid\" | \"widget\""
    )]
    pub kind: &'static str,
    /// The named destination's name, for a `namedDestination`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub name: Option<String>,
    /// `retargeted` or `removed`.
    #[tsify(type = "\"retargeted\" | \"removed\"")]
    pub outcome: &'static str,
    /// The surviving page it now names, when retargeted.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub to: Option<String>,
}

impl From<laminapdf::ReferenceEdit> for ReferenceEditJs {
    fn from(r: laminapdf::ReferenceEdit) -> Self {
        use laminapdf::{ReferenceEditKind as K, ReferenceOutcome as O};
        let (kind, name) = match r.kind {
            K::OutlineItem => ("outlineItem", None),
            K::NamedDestination { name } => ("namedDestination", Some(name)),
            K::OpenAction => ("openAction", None),
            K::StructureElement => ("structureElement", None),
            K::StructureKid => ("structureKid", None),
            K::Widget => ("widget", None),
            _ => ("link", None),
        };
        let (outcome, to) = match r.outcome {
            O::Retargeted { to } => ("retargeted", Some(to.to_string())),
            _ => ("removed", None),
        };
        Self {
            page: r.page.to_string(),
            kind,
            name,
            outcome,
            to,
        }
    }
}

impl From<laminapdf::Edited> for EditedJs {
    fn from(e: laminapdf::Edited) -> Self {
        Self {
            touched_pages: e.touched_pages.iter().map(ToString::to_string).collect(),
            packet_rewritten: e.packet_rewritten,
            still_in_earlier_revisions: e.still_in_earlier_revisions,
            also_deleted: e.also_deleted,
            changed: e.changed,
            created_outline_items: e.created_outline_items.iter().map(ToString::to_string).collect(),
            created_annotations: e.created_annotations.iter().map(ToString::to_string).collect(),
            outline_repaired: e.outline_repaired,
            destination_uses: e.destination_uses,
            page_order: e.page_order.map(|o| o.iter().map(ToString::to_string).collect()),
            references: e.references.into_iter().map(ReferenceEditJs::from).collect(),
            fields: e.fields.into_iter().map(FieldEditJs::from).collect(),
            tags_incomplete: e.tags_incomplete.iter().map(ToString::to_string).collect(),
            declared: e.declared.into_iter().map(ExtensionPropertyJs::from).collect(),
        }
    }
}

/// An XMP property declared in a PDF/A file's extension schemas.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionPropertyJs {
    /// The schema's namespace URI.
    pub namespace: String,
    /// The prefix the packet writes the namespace with.
    pub prefix: String,
    /// The property's local name.
    pub name: String,
    /// Its declared type, e.g. `Text` or `Seq Integer`.
    pub value_type: String,
}

impl From<laminapdf::ExtensionProperty> for ExtensionPropertyJs {
    fn from(p: laminapdf::ExtensionProperty) -> Self {
        Self {
            namespace: p.namespace,
            prefix: p.prefix,
            name: p.name,
            value_type: p.value_type,
        }
    }
}

/// The document's XMP packet as text (`laminapdf::XmpText`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct XmpTextJs {
    /// Whether there is a packet, and whether it was read.
    pub status: laminapdf::PacketStatus,
    /// The packet's XML, with the edits made so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub xml: Option<String>,
    /// The PDF/A part and conformance the packet claims, e.g. `"2B"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub pdfa: Option<String>,
}

impl From<laminapdf::XmpText> for XmpTextJs {
    fn from(p: laminapdf::XmpText) -> Self {
        Self {
            status: p.status,
            xml: p.xml,
            pdfa: p.pdfa,
        }
    }
}

/// What a save wrote (`laminapdf::SaveReport`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a JSON record: one flag per fact the report states"
)]
pub(crate) struct SaveReportJs {
    /// Bytes written.
    pub bytes_written: u64,
    /// The mode used (`incremental` or `fullRewrite`).
    pub mode: SaveModeJs,
    /// Old → new object numbers, when a full rewrite renumbered.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional, type = "Record<string, number>")]
    pub renumbered: Option<std::collections::BTreeMap<u32, u32>>,
    /// Where the appended revision starts, for an incremental save.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub append_offset: Option<u64>,
    /// How many signatures the save invalidated (with `invalidateSignatures`).
    pub invalidated_signatures: u32,
    /// Whether encryption was removed.
    pub encryption_removed: bool,
    /// Whether the save stamped the save-time metadata (`metadataDate`, `instanceId`).
    pub metadata_updated: bool,
    /// Whether the options asked for the stamp and the save left it out, because it would have
    /// broken a signature's restriction the saved changes keep.
    pub metadata_not_updated: bool,
    /// Whether an edit saved here rewrote the XMP packet whole.
    pub packet_rewritten: bool,
    /// Whether the save emptied the undo history (a full rewrite, or an incremental save that
    /// freed objects); for a copy, whether adopting it would.
    pub history_cleared: bool,
}

impl From<&SaveReport> for SaveReportJs {
    fn from(r: &SaveReport) -> Self {
        Self {
            bytes_written: r.bytes_written(),
            mode: if r.mode() == SaveMode::Incremental {
                SaveModeJs::Incremental
            } else {
                SaveModeJs::FullRewrite
            },
            renumbered: r.renumbered().cloned(),
            append_offset: r.append_offset(),
            invalidated_signatures: u32::try_from(r.invalidated().len()).unwrap_or(u32::MAX),
            encryption_removed: r.encryption_removed(),
            metadata_updated: r.metadata_updated(),
            metadata_not_updated: r.metadata_not_updated(),
            packet_rewritten: r.packet_rewritten(),
            history_cleared: r.history_cleared(),
        }
    }
}

/// How an undo or redo went (`laminapdf::UndoOutcome`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UndoOutcomeJs {
    /// Everything was restored.
    Applied,
    /// Some properties were left alone (a later change replaced them, or they are gone).
    Partial,
    /// Nothing could be restored.
    Skipped,
}

impl From<&laminapdf::UndoOutcome> for UndoOutcomeJs {
    fn from(o: &laminapdf::UndoOutcome) -> Self {
        match o {
            laminapdf::UndoOutcome::Applied => Self::Applied,
            laminapdf::UndoOutcome::Partial { .. } => Self::Partial,
            _ => Self::Skipped,
        }
    }
}

/// What an undo or redo did (`laminapdf::Reverted`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RevertedJs {
    /// How it went.
    pub outcome: UndoOutcomeJs,
    /// IDs of the pages the undone or redone transaction touched: draw these again.
    pub touched_pages: Vec<String>,
}

impl From<laminapdf::Reverted> for RevertedJs {
    fn from(r: laminapdf::Reverted) -> Self {
        Self {
            outcome: UndoOutcomeJs::from(&r.outcome),
            touched_pages: r.touched_pages.iter().map(ToString::to_string).collect(),
        }
    }
}

/// A job's progress (`laminapdf::Step::Progress`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProgressJs {
    /// Work done so far (units of the phase).
    pub done: u32,
    /// The total, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub total: Option<u32>,
    /// The phase (`parse`, `layout`, `extract`, `render`, `write`, `sign`, `model`).
    pub phase: &'static str,
}

/// `laminapdf::Phase` as a camelCase string.
pub(crate) fn phase_name(phase: laminapdf::Phase) -> &'static str {
    match phase {
        laminapdf::Phase::Parse => "parse",
        laminapdf::Phase::Layout => "layout",
        laminapdf::Phase::Extract => "extract",
        laminapdf::Phase::Render => "render",
        laminapdf::Phase::Write => "write",
        laminapdf::Phase::Sign => "sign",
        laminapdf::Phase::Model => "model",
        // A phase added to the non-exhaustive enum before this list learns it.
        _ => "other",
    }
}
