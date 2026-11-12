//! [`DocumentMetadata`]: what a document says about itself, read from Info and XMP, with the
//! side that wins when they disagree (E1 plan §2, §11).

use laminapdf_model::xmp::{MappedField, Packet, PacketState, Value, ValueKind};
use laminapdf_model::{Info, PdfDate};
use serde::Serialize;

use super::MetadataKey;
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

/// One side's values, one slot per [`MetadataKey`] (in [`MetadataKey::ALL`]'s order), each
/// in the shape its row of [`MAPPINGS`](laminapdf_model::xmp::MAPPINGS) gives it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Side([Option<Value>; MetadataKey::ALL.len()]);

impl Side {
    /// Info's entries: text as decoded, `/Author` split at `;`, dates that parse.
    fn of_info(info: Option<&Info<'_>>) -> Result<Self> {
        let mut side = Self::default();
        let Some(info) = info else {
            return Ok(side);
        };
        for (slot, key) in side.0.iter_mut().zip(MetadataKey::ALL) {
            let mapping = key.field().mapping();
            let Some(info_key) = mapping.info_key else {
                continue;
            };
            *slot = match mapping.kind {
                ValueKind::Date => info.date(info_key)?.and_then(|d| d.parsed).map(Value::Date),
                ValueKind::Seq => info
                    .text(info_key)?
                    .map(|a| split_authors(&a))
                    .filter(|a| !a.is_empty())
                    .map(Value::List),
                _ => info.text(info_key)?.map(Value::Text),
            };
        }
        Ok(side)
    }

    /// The packet's mapped properties.
    fn of_packet(packet: &Packet) -> Self {
        let mut side = Self::default();
        for (slot, key) in side.0.iter_mut().zip(MetadataKey::ALL) {
            *slot = packet.get(key.field());
        }
        side
    }

    fn get(&self, key: MetadataKey) -> Option<&Value> {
        self.0.get(key as usize)?.as_ref()
    }
}

/// Whether two values say the same: dates naming the same instant in any zone, anything else
/// equal.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Date(a), Value::Date(b)) => a.cmp_instant(b).is_eq(),
        _ => a == b,
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    }
}

fn date(value: Option<&Value>) -> Option<PdfDate> {
    match value? {
        Value::Date(d) => Some(*d),
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
        .filter_map(|key| info.get(key).map(|v| (key.field(), v.clone())))
        .collect())
}

/// The fields whose Info entry has no equal XMP property: what PDF/A forbids (ISO 19005-2
/// §6.6.2.3.1).
pub(crate) fn info_without_equal_xmp(doc: &laminapdf_model::Document, packet: &Packet) -> Result<Vec<MetadataKey>> {
    let info = Side::of_info(doc.info()?.as_ref())?;
    let xmp = Side::of_packet(packet);
    Ok(MetadataKey::ALL
        .into_iter()
        .filter(|&key| info.get(key).is_some_and(|i| !xmp.get(key).is_some_and(|x| same(i, x))))
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
        // The newer modification date wins; the same instant, in whatever zone, is a tie,
        // which Info wins.
        Some(_) => match (
            date(info_side.get(MetadataKey::Modified)),
            date(xmp_side.get(MetadataKey::Modified)),
        ) {
            (Some(i), Some(x)) if x.cmp_instant(&i).is_gt() => MetadataSource::Xmp,
            (None, Some(_)) => MetadataSource::Xmp,
            _ => MetadataSource::Info,
        },
    };
    let (first, second) = match source {
        MetadataSource::Xmp => (&xmp_side, &info_side),
        MetadataSource::Info => (&info_side, &xmp_side),
    };
    let pick = |key: MetadataKey| first.get(key).or_else(|| second.get(key));
    let mut disagreements = Vec::new();
    let mut kept_info_keys = Vec::new();
    for key in MetadataKey::ALL {
        let in_info = info_side.get(key);
        let deprecated = pdf2 && !matches!(key, MetadataKey::Created | MetadataKey::Modified);
        if deprecated && in_info.is_some() {
            kept_info_keys.push(key);
        }
        if packet.is_none() {
            continue;
        }
        let differ = match (in_info, xmp_side.get(key)) {
            (Some(i), Some(x)) => !same(i, x),
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
        title: text(pick(MetadataKey::Title)),
        authors: match pick(MetadataKey::Authors) {
            Some(Value::List(items)) => items.clone(),
            _ => Vec::new(),
        },
        subject: text(pick(MetadataKey::Subject)),
        keywords: text(pick(MetadataKey::Keywords)),
        creator: text(pick(MetadataKey::Creator)),
        producer: text(pick(MetadataKey::Producer)),
        created: date(pick(MetadataKey::Created)),
        modified: date(pick(MetadataKey::Modified)),
        source,
        disagreements,
        packet: packet_status,
        pdfa: pdfa_claim,
        pdfua: packet.and_then(Packet::pdfua_part),
        kept_info_keys,
    })
}
