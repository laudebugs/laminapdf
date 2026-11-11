//! [`DocumentMetadata`]: what a document says about itself, read from Info and XMP, with the
//! side that wins when they disagree (E1 plan §2, §11).

use laminapdf_model::xmp::{MappedField, Packet, PacketState, Value};
use laminapdf_model::{Info, PdfDate};
use serde::Serialize;

use super::MetadataKey;
use super::write::AUTHOR_SEPARATOR;
use crate::error::Result;
use crate::wire;

/// Which side [`DocumentMetadata`]'s values come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum MetadataSource {
    /// The Info dictionary.
    #[default]
    Info,
    /// The XMP packet.
    Xmp,
}

/// What the document's `/Metadata` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum PacketStatus {
    /// No packet.
    #[default]
    Absent,
    /// A packet that was read.
    Parsed,
    /// A packet that can't be read (`xmp-invalid`): it counts as absent, and an edit replaces
    /// it only when asked to.
    Malformed,
}

/// A document's metadata as one record, read from the Info dictionary and the XMP packet.
///
/// When the two disagree, the values come from the XMP packet in a PDF 2.0 file (ISO 32000-2
/// §14.3.3 deprecates the Info entries) or a file that claims PDF/A (whose packet is
/// authoritative, ISO 19005-2 §6.6.2.3.1), and otherwise from the side whose modification date
/// is newer, Info winning a tie. A field the winning side lacks is taken from the other.
/// [`disagreements`](DocumentMetadata::disagreements) names the fields where the two differ, so
/// a properties view can say so.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DocumentMetadata {
    /// The title.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub title: Option<String>,
    /// The authors, in order (Info's `/Author` split at `;`).
    pub authors: Vec<String>,
    /// The subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub subject: Option<String>,
    /// The keywords.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub keywords: Option<String>,
    /// The application the content came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub creator: Option<String>,
    /// The application that wrote the PDF.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub producer: Option<String>,
    /// The creation date (a PDF date string on the wire).
    #[serde(with = "wire::opt_date", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub created: Option<PdfDate>,
    /// The modification date.
    #[serde(with = "wire::opt_date", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub modified: Option<PdfDate>,
    /// Which side the values come from.
    pub source: MetadataSource,
    /// The fields where Info and XMP hold different values.
    pub disagreements: Vec<MetadataKey>,
    /// What `/Metadata` holds.
    pub packet: PacketStatus,
    /// The PDF/A part and conformance the packet claims (`pdfaid:`), e.g. `"2B"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub pdfa: Option<String>,
    /// The PDF/UA part the packet claims (`pdfuaid:part`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub pdfua: Option<u8>,
    /// In a PDF 2.0 file, the Info entries besides the two dates: deprecated, kept as they
    /// were for older readers, and never written by an edit.
    pub kept_info_keys: Vec<MetadataKey>,
}

/// One side's values, keyed like [`MetadataKey::ALL`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Side {
    text: [Option<String>; 6],
    authors: Vec<String>,
    created: Option<PdfDate>,
    modified: Option<PdfDate>,
}

impl Side {
    fn of_info(info: Option<&Info<'_>>) -> Result<Self> {
        let Some(info) = info else {
            return Ok(Self::default());
        };
        let date = |d: Option<laminapdf_model::InfoDate>| d.and_then(|d| d.parsed);
        Ok(Self {
            text: [
                info.title()?,
                None,
                info.subject()?,
                info.keywords()?,
                info.creator()?,
                info.producer()?,
            ],
            authors: info.author()?.map(|a| split_authors(&a)).unwrap_or_default(),
            created: date(info.creation_date()?),
            modified: date(info.mod_date()?),
        })
    }

    fn of_packet(packet: &Packet) -> Self {
        Self {
            text: [
                packet.title(),
                None,
                packet.subject(),
                packet.keywords(),
                packet.creator_tool(),
                packet.producer(),
            ],
            authors: packet.authors(),
            created: packet.create_date(),
            modified: packet.modify_date(),
        }
    }

    /// The field's value in a comparable form; `None` when absent.
    fn value(&self, key: MetadataKey) -> Option<String> {
        match key {
            MetadataKey::Authors => (!self.authors.is_empty()).then(|| self.authors.join(AUTHOR_SEPARATOR)),
            MetadataKey::Created => self.created.map(|d| d.to_iso8601()),
            MetadataKey::Modified => self.modified.map(|d| d.to_iso8601()),
            other => text_slot(other).and_then(|i| self.text.get(i)).cloned().flatten(),
        }
    }
}

const fn text_slot(key: MetadataKey) -> Option<usize> {
    match key {
        MetadataKey::Title => Some(0),
        MetadataKey::Subject => Some(2),
        MetadataKey::Keywords => Some(3),
        MetadataKey::Creator => Some(4),
        MetadataKey::Producer => Some(5),
        _ => None,
    }
}

/// Info's `/Author` as a list: split at `;` (the separator the ops write), each name trimmed.
fn split_authors(author: &str) -> Vec<String> {
    author
        .split(';')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The values the Info dictionary holds for the mapped fields, in the table's order: what a new
/// packet starts from, so it agrees with Info from the first edit.
pub(crate) fn info_values(doc: &laminapdf_model::Document) -> Result<Vec<(MappedField, Value)>> {
    let info = Side::of_info(doc.info()?.as_ref())?;
    Ok(MetadataKey::ALL
        .into_iter()
        .filter_map(|key| {
            let value = match key {
                MetadataKey::Authors => (!info.authors.is_empty()).then(|| Value::List(info.authors.clone())),
                MetadataKey::Created => info.created.map(Value::Date),
                MetadataKey::Modified => info.modified.map(Value::Date),
                other => text_slot(other)
                    .and_then(|i| info.text.get(i).cloned().flatten())
                    .map(Value::Text),
            };
            value.map(|v| (key.field(), v))
        })
        .collect())
}

/// The fields whose Info entry has no equal XMP property: what PDF/A forbids (ISO 19005-2
/// §6.6.2.3.1). Empty without a readable packet.
pub(crate) fn info_without_equal_xmp(doc: &laminapdf_model::Document, packet: &Packet) -> Result<Vec<MetadataKey>> {
    let info = Side::of_info(doc.info()?.as_ref())?;
    let xmp = Side::of_packet(packet);
    Ok(MetadataKey::ALL
        .into_iter()
        .filter(|&key| info.value(key).is_some_and(|i| xmp.value(key).as_ref() != Some(&i)))
        .collect())
}

/// Reads the document's metadata ([`DocumentMetadata`]).
///
/// # Errors
/// Reading errors (Info, the catalog, the packet's stream); in strict mode a packet that
/// can't be read or an invalid date.
pub fn read_metadata(doc: &laminapdf_model::Document) -> Result<DocumentMetadata> {
    let info = doc.info()?;
    let info_side = Side::of_info(info.as_ref())?;
    let state = doc.xmp()?;
    let pdf2 = doc.is_pdf2()?;
    let (packet_status, packet) = match &state {
        PacketState::Parsed(p) => (PacketStatus::Parsed, Some(p)),
        PacketState::Malformed { .. } => (PacketStatus::Malformed, None),
        _ => (PacketStatus::Absent, None),
    };
    let xmp_side = packet.map(Side::of_packet).unwrap_or_default();
    let claims_pdfa = packet.and_then(Packet::pdfa_part).is_some();
    let source = match packet {
        None => MetadataSource::Info,
        Some(_) if pdf2 || claims_pdfa => MetadataSource::Xmp,
        Some(_) => match (info_side.modified, xmp_side.modified) {
            (Some(i), Some(x)) if x > i => MetadataSource::Xmp,
            (None, Some(_)) => MetadataSource::Xmp,
            _ => MetadataSource::Info,
        },
    };
    let (first, second) = match source {
        MetadataSource::Xmp => (&xmp_side, &info_side),
        MetadataSource::Info => (&info_side, &xmp_side),
    };
    let pick_text = |key: MetadataKey| {
        text_slot(key).and_then(|i| {
            let a = first.text.get(i).cloned().flatten();
            a.or_else(|| second.text.get(i).cloned().flatten())
        })
    };
    let mut disagreements = Vec::new();
    let mut kept_info_keys = Vec::new();
    for key in MetadataKey::ALL {
        let in_info = info_side.value(key);
        let deprecated = pdf2 && !matches!(key, MetadataKey::Created | MetadataKey::Modified);
        if deprecated && in_info.is_some() {
            kept_info_keys.push(key);
        }
        if packet.is_none() {
            continue;
        }
        let in_xmp = xmp_side.value(key);
        let differ = match (&in_info, &in_xmp) {
            (Some(i), Some(x)) => i != x,
            // In PDF 2.0 Info needn't repeat what XMP holds.
            (None, Some(_)) => !deprecated,
            (Some(_), None) => true,
            (None, None) => false,
        };
        if differ {
            disagreements.push(key);
        }
    }
    let pdfa_claim = packet.and_then(|p| {
        p.pdfa_part().map(|part| {
            format!(
                "{part}{}",
                p.pdfa_conformance().unwrap_or_default().to_ascii_uppercase()
            )
        })
    });
    Ok(DocumentMetadata {
        title: pick_text(MetadataKey::Title),
        authors: if first.authors.is_empty() {
            second.authors.clone()
        } else {
            first.authors.clone()
        },
        subject: pick_text(MetadataKey::Subject),
        keywords: pick_text(MetadataKey::Keywords),
        creator: pick_text(MetadataKey::Creator),
        producer: pick_text(MetadataKey::Producer),
        created: first.created.or(second.created),
        modified: first.modified.or(second.modified),
        source,
        disagreements,
        packet: packet_status,
        pdfa: pdfa_claim,
        pdfua: packet.and_then(Packet::pdfua_part),
        kept_info_keys,
    })
}
