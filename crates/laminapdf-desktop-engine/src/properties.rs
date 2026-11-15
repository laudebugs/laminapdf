//! What the Properties dialog shows of a document: its properties, its metadata and its XMP
//! packet, as the edits so far leave them, in one JSON object the webview reads as it is
//! (`FromEngine::Properties`). The shapes are `@laminapdf/ui`'s: `DocumentPropertiesData`,
//! `DocumentMetadataData` and `XmpPacketData`, in camelCase, an absent value left out.

use laminapdf::{Cipher, Document, DocumentMetadata, DocumentProperties, PacketStatus, XmpText};
use serde::Serialize;

/// The dialog's facts of one document: the File tab's, which read every object.
pub(crate) type Facts = PropertiesData;

/// The dialog's three reads of one document.
#[derive(Debug, Serialize)]
struct DialogData<'a> {
    /// The facts the File tab lists.
    properties: &'a PropertiesData,
    /// What the Description form edits: Info and XMP read together.
    metadata: DocumentMetadata,
    /// The XMP packet, for the XMP tab.
    xmp: XmpData,
    /// Each page's label, when the document labels its pages (a page without one by its number).
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<String>>,
}

/// `DocumentPropertiesData`: the façade's properties, flattened as the dialog reads them.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PropertiesData {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keywords: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    producer: Option<String>,
    /// ISO 8601.
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    /// The XMP packet's text.
    #[serde(skip_serializing_if = "Option::is_none")]
    xmp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    page_count: u32,
    file_size: u64,
    linearized: bool,
    tagged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    encryption: Option<EncryptionData>,
    fonts: Vec<FontData>,
    signatures: u32,
    contains: ContainsData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionData {
    revision: u8,
    key_bits: u16,
    /// `PropertiesCipher`.
    cipher: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    permissions: Option<PermissionsData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(clippy::struct_excessive_bools, reason = "one flag per permission bit")]
struct PermissionsData {
    print: bool,
    print_high_quality: bool,
    modify: bool,
    copy: bool,
    annotate: bool,
    fill_forms: bool,
    accessibility: bool,
    assemble: bool,
}

#[derive(Debug, Serialize)]
struct FontData {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    embedded: bool,
    subset: bool,
}

#[derive(Debug, Serialize)]
struct ContainsData {
    javascript: bool,
    attachments: u32,
    forms: bool,
}

/// `XmpPacketData`.
#[derive(Debug, Serialize)]
struct XmpData {
    status: PacketStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    xml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pdfa: Option<String>,
}

/// The name `@laminapdf/ui`'s `PropertiesCipher` gives `cipher`.
const fn cipher_name(cipher: Cipher) -> &'static str {
    match cipher {
        Cipher::Rc4 => "rc4",
        Cipher::Aes128 => "aes-128",
        Cipher::Aes256 => "aes-256",
        Cipher::Identity => "identity",
        // A cipher a later version adds still encrypts: it is never reported as Identity.
        _ => "other",
    }
}

impl From<DocumentProperties> for PropertiesData {
    fn from(p: DocumentProperties) -> Self {
        let encryption = p.encryption.map(|e| EncryptionData {
            revision: e.revision,
            key_bits: e.key_bits,
            cipher: cipher_name(e.cipher),
            permissions: e.permissions.map(|p| PermissionsData {
                print: p.print,
                print_high_quality: p.print_high_quality,
                modify: p.modify,
                copy: p.copy,
                annotate: p.annotate,
                fill_forms: p.fill_forms,
                accessibility: p.accessibility,
                assemble: p.assemble,
            }),
        });
        Self {
            title: p.info.title,
            author: p.info.author,
            subject: p.info.subject,
            keywords: p.info.keywords,
            creator: p.info.creator,
            producer: p.info.producer,
            created: p.created,
            modified: p.modified,
            xmp: p.xmp,
            version: p.version,
            page_count: p.page_count,
            file_size: p.file_size,
            linearized: p.linearized,
            tagged: p.tagged,
            encryption,
            fonts: p
                .fonts
                .into_iter()
                .map(|f| FontData {
                    name: f.name,
                    kind: f.kind,
                    embedded: f.embedded,
                    subset: f.subset,
                })
                .collect(),
            signatures: p.signatures,
            contains: ContainsData {
                javascript: p.contains.javascript,
                attachments: p.contains.attachments,
                forms: p.contains.forms,
            },
        }
    }
}

impl From<XmpText> for XmpData {
    fn from(x: XmpText) -> Self {
        Self {
            status: x.status,
            xml: x.xml,
            pdfa: x.pdfa,
        }
    }
}

/// The dialog's data for `doc`, as JSON: its facts from `facts` when kept there, else read
/// (every object) and kept; its metadata and XMP packet read each time, cheaply, with the edits so far.
///
/// # Errors
/// The code of the façade's reading error, or `internal` when the data can't be written as JSON.
pub(crate) fn read(doc: &Document, facts: &mut Option<Facts>) -> Result<String, String> {
    let code = |error: laminapdf::Error| error.code().to_owned();
    let properties = match facts.take() {
        Some(kept) => kept,
        None => doc.properties().map_err(code)?.into(),
    };
    let json = (|| {
        let data = DialogData {
            properties: &properties,
            metadata: doc.metadata().map_err(code)?,
            xmp: doc.xmp_packet().map_err(code)?.into(),
            labels: labels(doc).map_err(code)?,
        };
        serde_json::to_string(&data).map_err(|_| "internal".to_owned())
    })();
    *facts = Some(properties);
    json
}

/// Each page's label, or `None` when no page has one.
fn labels(doc: &Document) -> Result<Option<Vec<String>>, laminapdf::Error> {
    let pages = doc.page_infos()?;
    if pages.iter().all(|page| page.label.is_none()) {
        return Ok(None);
    }
    Ok(Some(
        pages
            .into_iter()
            .map(|page| page.label.unwrap_or_else(|| (page.index + 1).to_string()))
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(json: &str) -> serde_json::Value {
        serde_json::from_str(json).unwrap_or_default()
    }

    fn open(name: &str) -> Document {
        let bytes = laminapdf_test_fixtures::fixture(name);
        Document::open_bytes(bytes, laminapdf::OpenOptions::default()).expect("the fixture opens")
    }

    #[test]
    fn read_should_give_the_metadata_in_the_shape_the_description_form_reads() {
        let json = data(&read(&open("golden/metadata.pdf"), &mut None).expect("reads"));
        assert_eq!(
            (
                json["metadata"]["authors"].as_array().map(Vec::len),
                &json["metadata"]["packet"]
            ),
            (Some(1), &serde_json::json!("parsed"))
        );
    }

    #[test]
    fn read_should_give_the_properties_in_camel_case_with_absent_values_left_out() {
        let json = data(&read(&open("golden/metadata.pdf"), &mut None).expect("reads"));
        let properties = &json["properties"];
        assert_eq!(
            (properties["pageCount"].as_u64(), properties.get("encryption")),
            (Some(1), None)
        );
    }

    /// The webview's copy of this data (`apps/desktop/src/engine/properties.fixture.json`), whose
    /// keys its test holds to `@laminapdf/ui`'s shapes. `UPDATE_GOLDEN=1` rewrites it (then run
    /// `cargo xtask fmt`, whose Biome lays it out).
    #[test]
    fn read_should_give_the_json_the_webviews_fixture_holds() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/src/engine/properties.fixture.json");
        let read = data(&read(&open("golden/metadata.pdf"), &mut None).expect("reads"));
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            let pretty = serde_json::to_string_pretty(&read).expect("writes") + "\n";
            std::fs::write(&path, pretty).expect("writes the fixture");
        }
        // Compared as values: the repository's formatter lays the file out its own way.
        assert_eq!(data(&std::fs::read_to_string(&path).unwrap_or_default()), read);
    }

    #[test]
    fn read_should_keep_the_facts_and_read_the_metadata_again() {
        let doc = open("golden/metadata.pdf");
        let mut facts = None;
        read(&doc, &mut facts).expect("reads");
        let kept = facts.as_ref().map(|f| f.page_count);
        let again = data(&read(&doc, &mut facts).expect("reads again"));
        assert_eq!(
            (kept, &again["metadata"]["packet"]),
            (Some(1), &serde_json::json!("parsed"))
        );
    }

    #[test]
    fn read_should_give_the_page_labels_only_for_a_document_that_labels_its_pages() {
        let labelled = data(&read(&open("golden/examples/links_and_outlines.pdf"), &mut None).expect("reads"));
        let plain = data(&read(&open("golden/metadata.pdf"), &mut None).expect("reads"));
        assert_eq!(
            (
                labelled["labels"].as_array().map(|l| !l.is_empty()),
                plain.get("labels")
            ),
            (Some(true), None)
        );
    }

    #[test]
    fn read_should_give_the_xmp_packet_with_its_status() {
        let json = data(&read(&open("golden/metadata.pdf"), &mut None).expect("reads"));
        assert_eq!(
            (&json["xmp"]["status"], json["xmp"]["xml"].is_string()),
            (&serde_json::json!("parsed"), true)
        );
    }
}
