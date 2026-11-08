//! What a document holds beyond its visible pages: metadata, attachments, actions and
//! scripts, annotations, form fields, layers that are off, and objects nothing references
//! (ISO 32000-2 §14.3, §7.11.4, §14.13, §12.6, §12.5, §12.7, §8.11). One scan reads them,
//! each item with its object number, so a report can list them and a sanitiser can find them.
//!
//! The typed models of these structures arrive with the editor rounds; when each lands, the
//! scan reads through it and its results stay the same.

mod actions;
mod forms;
mod objects;

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Object, Result};
use laminapdf_syntax::text::decode_text_string;

use crate::document::Document;
use crate::outline::ActionKind;

/// The XMP metadata packet the catalog's `/Metadata` names (§14.3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmpPacket {
    /// Its stream object.
    pub object: Option<u32>,
    /// Its decoded length in bytes (its stored length when it can't be decoded).
    pub bytes: u64,
    /// The namespace URIs its `xmlns:` attributes declare, in the packet's order, once each.
    pub namespaces: Vec<String>,
}

/// What points at an embedded file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EmbeddedFileHolder {
    /// The document's `/EmbeddedFiles` name tree (§7.7.4).
    EmbeddedFiles,
    /// An `/AF` array of associated files (§14.13).
    AssociatedFiles,
    /// A `FileAttachment` annotation's `/FS` (§12.5.6.15).
    Annotation,
}

/// An embedded file: a file specification with an `/EF` entry (§7.11.4).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EmbeddedFile {
    /// `/UF`, else `/F`, decoded.
    pub name: Option<String>,
    /// `/Params /Size`, else the embedded stream's stored length.
    pub size: Option<u64>,
    /// The embedded stream's `/Subtype` (a MIME type).
    pub mime: Option<String>,
    /// The file specification's object (`None` when it is a direct dictionary).
    pub object: Option<u32>,
    /// What points at it; empty when nothing the scan reads does.
    pub holders: Vec<EmbeddedFileHolder>,
}

/// What fires an action.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Trigger {
    /// The catalog's `/OpenAction`.
    OpenAction,
    /// A key of an `/AA` dictionary (`O`, `C`, `K`, `WC`, …), as written.
    Additional(String),
    /// An annotation's `/A`: activating it.
    Activate,
    /// An entry of the document's `/JavaScript` name tree, by its name.
    NamedScript(String),
}

/// The object an action belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Owner {
    /// The catalog.
    Document,
    /// A page, by 0-based index.
    Page(usize),
    /// An annotation on a page.
    Annotation {
        /// The page's 0-based index.
        page: usize,
        /// The annotation's object.
        object: Option<u32>,
    },
    /// A form field, by its full name.
    FormField(String),
}

/// An action reached from a trigger, directly or through `/Next` (§12.6.4).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Action {
    /// `/S`.
    pub kind: ActionKind,
    /// What fires it (for an action reached through `/Next`, what fires the first one).
    pub trigger: Trigger,
    /// What it belongs to.
    pub owner: Owner,
    /// Its object (`None` for a direct dictionary).
    pub object: Option<u32>,
    /// For a JavaScript action, the script's length in bytes (a stream that can't be decoded
    /// counts its stored length).
    pub script_bytes: Option<u64>,
}

/// An annotation on a page (§12.5).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Annotation {
    /// `/Subtype`, as written.
    pub subtype: String,
    /// The page's 0-based index.
    pub page: usize,
    /// Its object (`None` for a direct dictionary).
    pub object: Option<u32>,
    /// `/T`, the author of a markup annotation, decoded.
    pub author: Option<String>,
    /// `/Contents`, decoded.
    pub contents: Option<String>,
}

impl Annotation {
    /// Whether this is a link, a widget or a popup: annotations that carry no comment of
    /// their own.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        matches!(self.subtype.as_str(), "Link" | "Widget" | "Popup")
    }
}

/// A terminal form field (§12.7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FormField {
    /// The full name, the `/T` of each ancestor joined with `.`.
    pub name: String,
    /// `/FT` (`Tx`, `Btn`, `Ch`, `Sig`), own or inherited.
    pub kind: Option<String>,
    /// `/V`: a text string decoded, a name as written, an array's items joined with `, `.
    pub value: Option<String>,
    /// Whether every widget of the field is hidden (`/F` with Hidden or `NoView`).
    pub hidden: bool,
    /// The field's object (`None` for a direct dictionary).
    pub object: Option<u32>,
}

/// What [`Document::disclosure`] finds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Disclosure {
    /// The catalog's XMP packet.
    pub xmp: Option<XmpPacket>,
    /// Other `/Type /Metadata` streams (object-level metadata, §14.3.2).
    pub metadata_streams: Vec<u32>,
    /// Objects with a `/PieceInfo` dictionary (§14.5).
    pub piece_info: Vec<u32>,
    /// Pages with a `/Thumb` image (§12.3.4), by object.
    pub thumbnails: Vec<u32>,
    /// Embedded files, indirect ones by object number, then direct ones.
    pub attachments: Vec<EmbeddedFile>,
    /// Every action reached from a trigger, once each.
    pub actions: Vec<Action>,
    /// Whether the interactive form holds an XFA form (`/XFA`).
    pub xfa: bool,
    /// Annotations, page by page.
    pub annotations: Vec<Annotation>,
    /// Terminal form fields, in the form's order.
    pub fields: Vec<FormField>,
    /// The names of the optional content groups the default configuration turns off (§8.11.4.3).
    pub layers_off: Vec<String>,
    /// In-use objects nothing reachable from the trailer references, object and
    /// cross-reference streams aside.
    pub unreferenced: Vec<u32>,
}

impl Disclosure {
    /// The `Redact` annotations, as `(page, count)` with pages ascending.
    #[must_use]
    pub fn redactions(&self) -> Vec<(usize, u32)> {
        let mut out: BTreeMap<usize, u32> = BTreeMap::new();
        for annot in self.annotations.iter().filter(|a| a.subtype == "Redact") {
            let n = out.entry(annot.page).or_default();
            *n = n.saturating_add(1);
        }
        out.into_iter().collect()
    }
}

impl Document {
    /// What the document holds beyond its visible pages ([`Disclosure`]). It reads every
    /// object once, within the [`Limits`](crate::Limits); anything it can't read counts as
    /// absent, as ADR 0003's tolerant paths do.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::DataPending`], [`laminapdf_cos::Error::Source`] and
    /// [`laminapdf_cos::Error::Strict`]: the errors that must propagate.
    pub fn disclosure(&self) -> Result<Disclosure> {
        let mut scan = Scan {
            doc: self,
            out: Disclosure::default(),
            attachments: BTreeMap::new(),
            direct_attachments: Vec::new(),
            seen_actions: BTreeSet::new(),
        };
        let catalog = self.catalog()?;
        scan.objects(&catalog)?;
        scan.document_actions(&catalog)?;
        scan.pages()?;
        scan.fields(&catalog)?;
        scan.layers(&catalog)?;
        let mut attachments: Vec<EmbeddedFile> = std::mem::take(&mut scan.attachments).into_values().collect();
        attachments.append(&mut scan.direct_attachments);
        for a in &mut attachments {
            a.holders.sort_unstable();
            a.holders.dedup();
        }
        scan.out.attachments = attachments;
        Ok(scan.out)
    }
}

/// The scan's state.
struct Scan<'d> {
    doc: &'d Document,
    out: Disclosure,
    /// Indirect file specifications with `/EF`, by object.
    attachments: BTreeMap<u32, EmbeddedFile>,
    direct_attachments: Vec<EmbeddedFile>,
    /// Action objects already listed.
    seen_actions: BTreeSet<u32>,
}

impl Scan<'_> {
    /// `obj` resolved tolerantly.
    fn read(&self, obj: &Object) -> Result<Option<std::sync::Arc<Object>>> {
        self.doc.read(obj)
    }

    /// Marks the indirect file specification `obj` as held by `holder`, or adds a direct one.
    fn hold(&mut self, obj: &Object, holder: EmbeddedFileHolder) -> Result<()> {
        if let Some(r) = obj.as_ref() {
            if let Some(a) = self.attachments.get_mut(&r.num) {
                a.holders.push(holder);
            }
            return Ok(());
        }
        if let Some(dict) = obj.as_dict()
            && let Some(mut a) = objects::attachment(self.doc, dict, None)?
        {
            a.holders.push(holder);
            self.direct_attachments.push(a);
        }
        Ok(())
    }
}

/// A name's bytes, as written.
fn name_of(obj: &Object) -> Option<String> {
    match obj {
        Object::Name(n) => Some(String::from_utf8_lossy(n.as_bytes()).into_owned()),
        _ => None,
    }
}

/// A text string, decoded (PDFDocEncoding or UTF-16, §7.9.2.2).
fn text_of(obj: &Object) -> Option<String> {
    match obj {
        Object::String(s) => Some(decode_text_string(&s.bytes)),
        _ => None,
    }
}

/// `n` as a byte count.
fn len(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}
