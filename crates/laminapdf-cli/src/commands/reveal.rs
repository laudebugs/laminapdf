//! `laminapdf reveal`: what a PDF holds beyond its visible pages, by category: earlier
//! revisions and what each changed, metadata, attachments, scripts and actions, annotations,
//! form values, and hidden content. Hidden text is counted; its text shows only with
//! `--include-hidden-text`, which `laminapdf mcp` never offers.

use std::collections::BTreeMap;

use laminapdf::Diagnostic;
use laminapdf::reveal::{Action, ActionKind, Category, EmbeddedFileHolder, Owner, Reveal, Trigger};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, open, open_options, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::Io;

/// `laminapdf reveal <input>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevealArgs {
    /// The PDF to read, or `-` for standard input.
    // The command line takes several inputs, so the front end fills this in per input.
    #[arg(skip)]
    pub input: String,
    /// Keep only these top-level keys of the JSON report (`schema` and `input` always stay).
    #[arg(long, value_delimiter = ',', value_name = "KEYS")]
    #[serde(default)]
    pub fields: Vec<String>,
    /// List each hidden run's page, box, reason and text.
    #[arg(long)]
    #[serde(default)]
    pub include_hidden_text: bool,
    /// Leave the document's own strings out of the report: what `laminapdf mcp` asks for.
    #[arg(skip)]
    #[serde(skip)]
    pub withhold_document_text: bool,
}

/// `laminapdf_reveal`'s arguments: those of `laminapdf reveal` without `--include-hidden-text`
/// and `--fail-on`. The tool withholds the document's own strings (Info values, comments, field
/// names and values, file and layer names) as well as hidden text, and keeps a name the file
/// chose (an Info key, a subtype, an `/AA` key, an XMP namespace) only when the standard defines
/// it: text the document's author wrote can carry instructions to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevealToolArgs {
    /// The PDF to read.
    pub input: String,
    /// Keep only these top-level keys of the JSON report (`schema` and `input` always stay).
    #[serde(default)]
    pub fields: Vec<String>,
}

impl From<RevealToolArgs> for RevealArgs {
    fn from(args: RevealToolArgs) -> Self {
        Self {
            input: args.input,
            fields: args.fields,
            include_hidden_text: false,
            withhold_document_text: true,
        }
    }
}

/// A page: its 0-based index and 1-based number (ADR 0027 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
pub struct PageRef {
    /// The 0-based page index.
    pub index: u32,
    /// The 1-based page number.
    pub number: u32,
}

impl PageRef {
    fn of(index: impl TryInto<u32>) -> Self {
        let index = index.try_into().unwrap_or(u32::MAX);
        Self {
            index,
            number: index.saturating_add(1),
        }
    }
}

/// `laminapdf.reveal/1`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealReport {
    /// `laminapdf.reveal/1`.
    #[schemars(schema_with = "reveal_id")]
    pub schema: &'static str,
    /// The input as given: a path, or `-`.
    pub input: String,
    /// Whether the report carries the document's own strings: `false` from `laminapdf mcp`,
    /// which reports keys, counts and pages and leaves out Info values, comments, field names
    /// and values, and file and layer names.
    pub document_text: bool,
    /// The categories with findings, in this order: `history`, `metadata`, `attachments`,
    /// `active-content`, `annotations`, `forms`, `hidden-content`.
    pub found: Vec<String>,
    /// Revisions and objects nothing references.
    pub history: RevealHistory,
    /// The Info dictionary, XMP and other metadata.
    pub metadata: RevealMetadata,
    /// Embedded files.
    pub attachments: Vec<AttachmentEntry>,
    /// Scripts, actions and XFA.
    pub active_content: ActiveContent,
    /// Annotations by subtype, and the comments of markup annotations.
    pub annotations: Annotations,
    /// Terminal form fields.
    pub forms: Vec<FieldEntry>,
    /// Hidden text, layers off by default, and unapplied redactions.
    pub hidden_content: HiddenContent,
    /// The warnings reading the document raised (ADR 0026 objects).
    #[schemars(schema_with = "warnings_schema")]
    pub warnings: Vec<Diagnostic>,
}

impl RevealReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.reveal/1";
}

fn reveal_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(RevealReport::SCHEMA)
}

/// `history`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealHistory {
    /// The revisions, oldest first; empty for a repaired file.
    pub revisions: Vec<RevisionEntry>,
    /// Whether the file is linearized.
    pub linearized: bool,
    /// Whether its cross-reference table had to be rebuilt.
    pub repaired: bool,
    /// In-use objects nothing references.
    pub unreferenced: Vec<u32>,
}

/// One revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevisionEntry {
    /// 0 for the original file.
    pub index: u32,
    /// Its length in bytes.
    pub bytes: u64,
    /// Objects it adds.
    pub added: Vec<u32>,
    /// Objects it rewrites.
    pub changed: Vec<u32>,
    /// Objects it frees.
    pub freed: Vec<u32>,
    /// The latest revision's pages whose dictionary or content it rewrites.
    pub pages: Vec<PageRef>,
    /// Words it holds that the latest revision doesn't.
    pub removed_words: u64,
    /// Its pages where they are.
    pub removed_on: Vec<PageRef>,
}

/// `metadata`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealMetadata {
    /// Every Info entry: document text.
    pub info: Vec<InfoEntry>,
    /// The XMP packet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xmp: Option<Xmp>,
    /// Object-level `/Metadata` streams.
    pub metadata_streams: u32,
    /// Objects with `/PieceInfo`.
    pub piece_info: u32,
    /// Pages with a `/Thumb`.
    pub thumbnails: u32,
    /// The trailer's `/ID`, in hexadecimal.
    pub id: Vec<String>,
}

/// An Info entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct InfoEntry {
    /// The key, as written.
    pub key: String,
    /// The value: document text, left out when `documentText` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// The XMP packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Xmp {
    /// Its decoded size in bytes.
    pub bytes: u64,
    /// The namespace URIs it declares.
    pub namespaces: Vec<String>,
}

/// An embedded file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct AttachmentEntry {
    /// Its file name: document text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Its MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    /// Its file specification's object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<u32>,
    /// What points at it: `embedded-files`, `associated-files`, `annotation`.
    pub holders: Vec<String>,
}

/// `activeContent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveContent {
    /// JavaScript actions.
    pub scripts: u32,
    /// Their scripts' total size in bytes.
    pub script_bytes: u64,
    /// Every action, once.
    pub actions: Vec<ActionEntry>,
    /// Whether the form holds an XFA part.
    pub xfa: bool,
}

/// An action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionEntry {
    /// `/S`, as written (`JavaScript`, `Launch`, `URI`, …).
    pub kind: String,
    /// What fires it: `open-action`, `activate`, `additional:<key>` or `named-script`.
    pub trigger: String,
    /// What it belongs to: `document`, `page`, `annotation` or `field`.
    pub owner: String,
    /// The owner's page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<PageRef>,
    /// Its object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<u32>,
}

/// `annotations`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// How many of each subtype.
    pub by_subtype: BTreeMap<String, u32>,
    /// Markup annotations with an author or a comment.
    pub comments: Vec<CommentEntry>,
}

/// A markup annotation's author and comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CommentEntry {
    /// Its page.
    pub page: PageRef,
    /// `/Subtype`.
    pub subtype: String,
    /// `/T`: document text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// `/Contents`: document text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
}

/// A form field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct FieldEntry {
    /// Its full name: document text, left out when `documentText` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `/FT`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Its value: document text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Whether it has a value, also when `documentText` is false.
    pub filled: bool,
    /// Whether all its widgets are hidden.
    pub hidden: bool,
}

/// `hiddenContent`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HiddenContent {
    /// Hidden text by reason (`covered`, `invisible`, `off-page`, `tiny`, `clipped`,
    /// `low-contrast`).
    pub text: Vec<HiddenCount>,
    /// Each hidden run, only with `--include-hidden-text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runs: Option<Vec<HiddenRun>>,
    /// The layers off by default.
    pub layers_off: Vec<LayerEntry>,
    /// Pages with unapplied `Redact` annotations.
    pub redactions: Vec<RedactionEntry>,
}

/// A layer off by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct LayerEntry {
    /// Its name: document text, left out when `documentText` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Hidden characters for one reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct HiddenCount {
    /// The reason.
    pub reason: String,
    /// Characters hidden, spaces left out.
    pub chars: u64,
    /// The pages they are on.
    pub pages: Vec<PageRef>,
}

/// One hidden run.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HiddenRun {
    /// Its page.
    pub page: PageRef,
    /// Why it is hidden.
    pub reason: String,
    /// Its box in page space: left, top, right, bottom in points (ADR 0027).
    pub bbox: [f64; 4],
    /// Its text: document text.
    pub text: String,
}

/// A page's unapplied redactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RedactionEntry {
    /// The page.
    pub page: PageRef,
    /// How many `Redact` annotations.
    pub count: u32,
}

/// Lists what `args.input` holds beyond its visible pages. Earlier revisions open with the
/// same password and limits, never strictly.
///
/// # Errors
///
/// The first problem a read can't get past; `cancelled`.
pub fn reveal(args: &RevealArgs, settings: &Settings, io: &mut dyn Io) -> Result<RevealReport, CliError> {
    let opened = open(&args.input, settings, io)?;
    let doc = &opened.doc;
    settings.stop_if_cancelled()?;
    let reveal = doc.reveal(&open_options(settings), &settings.cancel)?;
    let mut report = RevealReport {
        schema: RevealReport::SCHEMA,
        input: args.input.clone(),
        document_text: true,
        found: reveal.found().iter().map(|c| c.as_str().to_owned()).collect(),
        history: history(&reveal),
        metadata: metadata(&reveal),
        attachments: attachments(&reveal),
        active_content: active_content(&reveal.disclosure.actions, reveal.disclosure.xfa),
        annotations: annotations(&reveal),
        forms: reveal
            .disclosure
            .fields
            .iter()
            .map(|f| FieldEntry {
                name: Some(f.name.clone()),
                kind: f.kind.clone(),
                value: f.value.clone(),
                filled: f.value.is_some(),
                hidden: f.hidden,
            })
            .collect(),
        hidden_content: hidden_content(&reveal, args.include_hidden_text && !args.withhold_document_text),
        warnings: warnings(doc),
    };
    if args.withhold_document_text {
        report.withhold_document_text();
    }
    Ok(report)
}

impl RevealReport {
    /// Leaves out every string the document's author wrote: Info values, attachment names,
    /// comment authors and text, field names and values, layer names and hidden runs. Keys,
    /// subtypes, counts, sizes and pages stay.
    fn withhold_document_text(&mut self) {
        self.document_text = false;
        for entry in &mut self.metadata.info {
            entry.value = None;
        }
        for attachment in &mut self.attachments {
            attachment.name = None;
        }
        for comment in &mut self.annotations.comments {
            comment.author = None;
            comment.contents = None;
        }
        for field in &mut self.forms {
            field.name = None;
            field.value = None;
        }
        for layer in &mut self.hidden_content.layers_off {
            layer.name = None;
        }
        self.hidden_content.runs = None;
        self.withhold_document_names();
    }

    /// Names a file chooses (Info keys, `/AA` keys, subtypes, field types, MIME types, XMP
    /// namespaces) can carry text too: each stays only when ISO 32000-2 (or, for XMP, a
    /// common schema) defines it, and becomes `other` otherwise.
    fn withhold_document_names(&mut self) {
        for entry in &mut self.metadata.info {
            entry.key = known(&entry.key, INFO_KEYS);
        }
        if let Some(xmp) = &mut self.metadata.xmp {
            for ns in &mut xmp.namespaces {
                *ns = known(ns, XMP_NAMESPACES);
            }
        }
        for attachment in &mut self.attachments {
            attachment.mime = None;
        }
        for action in &mut self.active_content.actions {
            action.kind = known(&action.kind, ACTION_KINDS);
            if let Some(key) = action.trigger.strip_prefix("additional:") {
                action.trigger = format!("additional:{}", known(key, AA_KEYS));
            }
        }
        let by_subtype = std::mem::take(&mut self.annotations.by_subtype);
        for (subtype, n) in by_subtype {
            let entry = self
                .annotations
                .by_subtype
                .entry(known(&subtype, SUBTYPES))
                .or_default();
            *entry = entry.saturating_add(n);
        }
        for comment in &mut self.annotations.comments {
            comment.subtype = known(&comment.subtype, SUBTYPES);
        }
        for field in &mut self.forms {
            field.kind = field.kind.as_deref().map(|k| known(k, FIELD_TYPES));
        }
    }
}

/// `name` when `list` holds it, else `other`.
fn known(name: &str, list: &[&str]) -> String {
    if list.contains(&name) {
        name.to_owned()
    } else {
        "other".to_owned()
    }
}

/// The Info dictionary's keys (§14.3.3).
const INFO_KEYS: &[&str] = &[
    "Title",
    "Author",
    "Subject",
    "Keywords",
    "Creator",
    "Producer",
    "CreationDate",
    "ModDate",
    "Trapped",
];

/// Action types (§12.6.4.1).
const ACTION_KINDS: &[&str] = &[
    "GoTo",
    "GoToR",
    "GoToE",
    "GoToDp",
    "Launch",
    "Thread",
    "URI",
    "Sound",
    "Movie",
    "Hide",
    "Named",
    "SubmitForm",
    "ResetForm",
    "ImportData",
    "SetOCGState",
    "Rendition",
    "Trans",
    "GoTo3DView",
    "JavaScript",
    "RichMediaExecute",
];

/// Additional-actions keys of the catalog, pages, annotations and fields (§12.6.3).
const AA_KEYS: &[&str] = &[
    "E", "X", "D", "U", "Fo", "Bl", "PO", "PC", "PV", "PI", "O", "C", "K", "F", "V", "WC", "WS", "DS", "WP", "DP",
];

/// Annotation subtypes (§12.5.6.1).
const SUBTYPES: &[&str] = &[
    "Text",
    "Link",
    "FreeText",
    "Line",
    "Square",
    "Circle",
    "Polygon",
    "PolyLine",
    "Highlight",
    "Underline",
    "Squiggly",
    "StrikeOut",
    "Caret",
    "Stamp",
    "Ink",
    "Popup",
    "FileAttachment",
    "Sound",
    "Movie",
    "Screen",
    "Widget",
    "PrinterMark",
    "TrapNet",
    "Watermark",
    "3D",
    "Redact",
    "Projection",
    "RichMedia",
];

/// Field types (§12.7.4.1).
const FIELD_TYPES: &[&str] = &["Btn", "Tx", "Ch", "Sig"];

/// XMP namespaces of the schemas PDF writers commonly use (ISO 16684-1, PDF/A, PDF/UA).
const XMP_NAMESPACES: &[&str] = &[
    "adobe:ns:meta/",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://purl.org/dc/elements/1.1/",
    "http://ns.adobe.com/xap/1.0/",
    "http://ns.adobe.com/xap/1.0/mm/",
    "http://ns.adobe.com/xap/1.0/rights/",
    "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
    "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
    "http://ns.adobe.com/pdf/1.3/",
    "http://ns.adobe.com/photoshop/1.0/",
    "http://www.aiim.org/pdfa/ns/id/",
    "http://www.aiim.org/pdfa/ns/extension/",
    "http://www.aiim.org/pdfa/ns/schema#",
    "http://www.aiim.org/pdfa/ns/property#",
    "http://www.aiim.org/pdfua/ns/id/",
];

/// Whether `report` found anything in `categories`.
#[must_use]
pub fn found_any(report: &RevealReport, categories: &[Category]) -> bool {
    categories.iter().any(|c| report.found.iter().any(|f| f == c.as_str()))
}

fn pages(indices: &[u32]) -> Vec<PageRef> {
    indices.iter().map(|&i| PageRef::of(i)).collect()
}

fn history(r: &Reveal) -> RevealHistory {
    RevealHistory {
        revisions: r
            .revisions
            .iter()
            .enumerate()
            .map(|(i, rev)| RevisionEntry {
                index: u32::try_from(i).unwrap_or(u32::MAX),
                bytes: rev.bytes,
                added: rev.added.clone(),
                changed: rev.changed.clone(),
                freed: rev.freed.clone(),
                pages: pages(&rev.pages),
                removed_words: rev.removed_words,
                removed_on: pages(&rev.removed_on),
            })
            .collect(),
        linearized: r.linearized,
        repaired: r.repaired,
        unreferenced: r.disclosure.unreferenced.clone(),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn metadata(r: &Reveal) -> RevealMetadata {
    let d = &r.disclosure;
    RevealMetadata {
        info: r
            .info
            .iter()
            .map(|(key, value)| InfoEntry {
                key: key.clone(),
                value: Some(value.clone()),
            })
            .collect(),
        xmp: d.xmp.as_ref().map(|x| Xmp {
            bytes: x.bytes,
            namespaces: x.namespaces.clone(),
        }),
        metadata_streams: count(d.metadata_streams.len()),
        piece_info: count(d.piece_info.len()),
        thumbnails: count(d.thumbnails.len()),
        id: r.id.clone(),
    }
}

fn attachments(r: &Reveal) -> Vec<AttachmentEntry> {
    r.disclosure
        .attachments
        .iter()
        .map(|a| AttachmentEntry {
            name: a.name.clone(),
            size: a.size,
            mime: a.mime.clone(),
            object: a.object,
            holders: a
                .holders
                .iter()
                .map(|h| match h {
                    EmbeddedFileHolder::AssociatedFiles => "associated-files",
                    EmbeddedFileHolder::Annotation => "annotation",
                    _ => "embedded-files",
                })
                .map(str::to_owned)
                .collect(),
        })
        .collect()
}

fn kind_name(kind: &ActionKind) -> String {
    match kind {
        ActionKind::GoTo => "GoTo".into(),
        ActionKind::GoToR => "GoToR".into(),
        ActionKind::GoToE => "GoToE".into(),
        ActionKind::Launch => "Launch".into(),
        ActionKind::Thread => "Thread".into(),
        ActionKind::Uri => "URI".into(),
        ActionKind::Named => "Named".into(),
        ActionKind::JavaScript => "JavaScript".into(),
        ActionKind::SubmitForm => "SubmitForm".into(),
        ActionKind::ResetForm => "ResetForm".into(),
        ActionKind::ImportData => "ImportData".into(),
        ActionKind::Other(name) => String::from_utf8_lossy(name).into_owned(),
        _ => "Other".into(),
    }
}

fn active_content(actions: &[Action], xfa: bool) -> ActiveContent {
    let scripts: Vec<&Action> = actions.iter().filter(|a| a.kind == ActionKind::JavaScript).collect();
    ActiveContent {
        scripts: count(scripts.len()),
        script_bytes: scripts.iter().filter_map(|a| a.script_bytes).sum(),
        actions: actions
            .iter()
            .map(|a| {
                let (owner, page) = match &a.owner {
                    Owner::Page(i) => ("page", Some(PageRef::of(*i))),
                    Owner::Annotation { page, .. } => ("annotation", Some(PageRef::of(*page))),
                    Owner::FormField(_) => ("field", None),
                    _ => ("document", None),
                };
                ActionEntry {
                    kind: kind_name(&a.kind),
                    trigger: match &a.trigger {
                        Trigger::OpenAction => "open-action".into(),
                        Trigger::Activate => "activate".into(),
                        Trigger::Additional(key) => format!("additional:{key}"),
                        _ => "named-script".into(),
                    },
                    owner: owner.into(),
                    page,
                    object: a.object,
                }
            })
            .collect(),
        xfa,
    }
}

fn annotations(r: &Reveal) -> Annotations {
    let mut by_subtype: BTreeMap<String, u32> = BTreeMap::new();
    let mut comments = Vec::new();
    for a in &r.disclosure.annotations {
        let n = by_subtype.entry(a.subtype.clone()).or_default();
        *n = n.saturating_add(1);
        if !a.is_plain() && (a.author.is_some() || a.contents.is_some()) {
            comments.push(CommentEntry {
                page: PageRef::of(a.page),
                subtype: a.subtype.clone(),
                author: a.author.clone(),
                contents: a.contents.clone(),
            });
        }
    }
    Annotations { by_subtype, comments }
}

fn reason_name(reason: laminapdf::extract::DropReason) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn hidden_content(r: &Reveal, include_text: bool) -> HiddenContent {
    let mut by_reason: BTreeMap<String, (u64, Vec<u32>)> = BTreeMap::new();
    for d in &r.hidden {
        let e = by_reason.entry(reason_name(d.reason)).or_default();
        e.0 = e.0.saturating_add(u64::try_from(d.chars).unwrap_or(u64::MAX));
        let page = u32::try_from(d.page).unwrap_or(u32::MAX);
        if !e.1.contains(&page) {
            e.1.push(page);
        }
    }
    let runs = include_text.then(|| {
        r.hidden
            .iter()
            .map(|d| HiddenRun {
                page: PageRef::of(d.page),
                reason: reason_name(d.reason),
                bbox: [d.bbox.x0.get(), d.bbox.y0.get(), d.bbox.x1.get(), d.bbox.y1.get()],
                text: d.spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" "),
            })
            .collect()
    });
    HiddenContent {
        text: by_reason
            .into_iter()
            .map(|(reason, (chars, on))| HiddenCount {
                reason,
                chars,
                pages: pages(&on),
            })
            .collect(),
        runs,
        layers_off: r
            .disclosure
            .layers_off
            .iter()
            .map(|name| LayerEntry {
                name: Some(name.clone()),
            })
            .collect(),
        redactions: r
            .disclosure
            .redactions()
            .into_iter()
            .map(|(page, count)| RedactionEntry {
                page: PageRef::of(page),
                count,
            })
            .collect(),
    }
}
