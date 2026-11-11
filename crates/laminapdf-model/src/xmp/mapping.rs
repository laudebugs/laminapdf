//! The one table that maps each metadata field to its Info key and its XMP property
//! (research 04 §5.2; ISO 32000-2 §14.3.2), shared by the builder, which writes both from
//! the same values, and by the editor, which keeps them in step.

/// The RDF namespace (ISO 16684-1 §7.4).
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
/// The namespace of the `xml:` prefix (`xml:lang`), bound in every XML document.
pub const XML: &str = "http://www.w3.org/XML/1998/namespace";
/// The `x:xmpmeta` wrapper's namespace.
pub const ADOBE_META: &str = "adobe:ns:meta/";
/// Dublin Core (`dc:`).
pub const DC: &str = "http://purl.org/dc/elements/1.1/";
/// The XMP basic schema (`xmp:`).
pub const XMP: &str = "http://ns.adobe.com/xap/1.0/";
/// The Adobe PDF schema (`pdf:`).
pub const PDF: &str = "http://ns.adobe.com/pdf/1.3/";
/// The XMP media management schema (`xmpMM:`).
pub const XMP_MM: &str = "http://ns.adobe.com/xap/1.0/mm/";
/// PDF/A identification (`pdfaid:`, ISO 19005-2 §6.6.4).
pub const PDFA_ID: &str = "http://www.aiim.org/pdfa/ns/id/";
/// PDF/UA identification (`pdfuaid:`, ISO 14289-1 §5).
pub const PDFUA_ID: &str = "http://www.aiim.org/pdfua/ns/id/";

/// The prefixes ISO 16684-1 and the schemas' own documents give their namespaces, which a new
/// packet declares and an edit uses when the packet hasn't bound the namespace already.
pub const STANDARD_PREFIXES: [(&str, &str); 6] = [
    ("dc", DC),
    ("xmp", XMP),
    ("pdf", PDF),
    ("xmpMM", XMP_MM),
    ("pdfaid", PDFA_ID),
    ("pdfuaid", PDFUA_ID),
];

/// A metadata field Lamina PDF reads and writes in both the Info dictionary and XMP, or in
/// XMP alone (the dates and IDs XMP keeps for itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum MappedField {
    /// `/Title`, `dc:title`.
    Title,
    /// `/Author`, `dc:creator` (an ordered list; Info holds one string).
    Authors,
    /// `/Subject`, `dc:description`.
    Subject,
    /// `/Keywords`, `pdf:Keywords`.
    Keywords,
    /// `/Producer` (the application that wrote the PDF), `pdf:Producer`.
    Producer,
    /// `/Creator` (the application the content came from), `xmp:CreatorTool`.
    Creator,
    /// `/CreationDate`, `xmp:CreateDate`.
    Created,
    /// `/ModDate`, `xmp:ModifyDate`.
    Modified,
    /// `xmp:MetadataDate`: when the metadata last changed (XMP only).
    MetadataDate,
    /// `xmpMM:DocumentID`: the document's identity across versions (XMP only).
    DocumentId,
    /// `xmpMM:InstanceID`: this version's identity (XMP only).
    InstanceId,
}

/// The shape of a mapped property's value in XMP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    /// A language alternative (`rdf:Alt` of `rdf:li xml:lang`); Lamina PDF writes the
    /// `x-default` entry.
    LangAlt,
    /// An ordered array (`rdf:Seq`).
    Seq,
    /// Plain text.
    Text,
    /// A date (ISO 8601, ISO 16684-1 §8.2.1.1).
    Date,
}

/// One row of [`MAPPINGS`]: where a [`MappedField`] lives in Info and in XMP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Mapping {
    /// The field.
    pub field: MappedField,
    /// Its key in the Info dictionary, or `None` for a field XMP alone holds.
    pub info_key: Option<&'static str>,
    /// The XMP property's namespace URI.
    pub namespace: &'static str,
    /// The prefix Lamina PDF writes for that namespace when the packet hasn't bound one.
    pub prefix: &'static str,
    /// The property's local name, e.g. `title`.
    pub name: &'static str,
    /// The shape of its value.
    pub kind: ValueKind,
}

const fn row(
    field: MappedField,
    info_key: Option<&'static str>,
    (prefix, namespace): (&'static str, &'static str),
    name: &'static str,
    kind: ValueKind,
) -> Mapping {
    Mapping {
        field,
        info_key,
        namespace,
        prefix,
        name,
        kind,
    }
}

const DC_NS: (&str, &str) = ("dc", DC);
const XMP_NS: (&str, &str) = ("xmp", XMP);
const PDF_NS: (&str, &str) = ("pdf", PDF);
const MM_NS: (&str, &str) = ("xmpMM", XMP_MM);

/// Every mapped field, in the order a packet Lamina PDF builds lists them.
///
/// ```
/// use laminapdf_model::xmp::{MAPPINGS, MappedField, ValueKind};
///
/// let title = MappedField::Title.mapping();
/// assert_eq!((title.info_key, title.prefix, title.name, title.kind), (Some("Title"), "dc", "title", ValueKind::LangAlt));
/// assert_eq!(MAPPINGS.iter().filter(|m| m.info_key.is_some()).count(), 8);
/// ```
pub const MAPPINGS: [Mapping; 11] = [
    row(MappedField::Title, Some("Title"), DC_NS, "title", ValueKind::LangAlt),
    row(MappedField::Authors, Some("Author"), DC_NS, "creator", ValueKind::Seq),
    row(
        MappedField::Subject,
        Some("Subject"),
        DC_NS,
        "description",
        ValueKind::LangAlt,
    ),
    row(
        MappedField::Keywords,
        Some("Keywords"),
        PDF_NS,
        "Keywords",
        ValueKind::Text,
    ),
    row(
        MappedField::Producer,
        Some("Producer"),
        PDF_NS,
        "Producer",
        ValueKind::Text,
    ),
    row(
        MappedField::Creator,
        Some("Creator"),
        XMP_NS,
        "CreatorTool",
        ValueKind::Text,
    ),
    row(
        MappedField::Created,
        Some("CreationDate"),
        XMP_NS,
        "CreateDate",
        ValueKind::Date,
    ),
    row(
        MappedField::Modified,
        Some("ModDate"),
        XMP_NS,
        "ModifyDate",
        ValueKind::Date,
    ),
    row(MappedField::MetadataDate, None, XMP_NS, "MetadataDate", ValueKind::Date),
    row(MappedField::DocumentId, None, MM_NS, "DocumentID", ValueKind::Text),
    row(MappedField::InstanceId, None, MM_NS, "InstanceID", ValueKind::Text),
];

impl MappedField {
    /// The field's row of [`MAPPINGS`].
    #[must_use]
    pub const fn mapping(self) -> &'static Mapping {
        // MAPPINGS lists the fields in declaration order, so the discriminant is the row.
        match self {
            Self::Title => &MAPPINGS[0],
            Self::Authors => &MAPPINGS[1],
            Self::Subject => &MAPPINGS[2],
            Self::Keywords => &MAPPINGS[3],
            Self::Producer => &MAPPINGS[4],
            Self::Creator => &MAPPINGS[5],
            Self::Created => &MAPPINGS[6],
            Self::Modified => &MAPPINGS[7],
            Self::MetadataDate => &MAPPINGS[8],
            Self::DocumentId => &MAPPINGS[9],
            Self::InstanceId => &MAPPINGS[10],
        }
    }

    /// The field whose Info key is `key`, e.g. `Title`.
    #[must_use]
    pub fn from_info_key(key: &str) -> Option<Self> {
        MAPPINGS.iter().find(|m| m.info_key == Some(key)).map(|m| m.field)
    }
}

/// The prefix ISO 16684-1 gives `namespace`, when it is one of [`STANDARD_PREFIXES`].
pub(super) fn standard_prefix(namespace: &str) -> Option<&'static str> {
    STANDARD_PREFIXES
        .iter()
        .find(|(_, ns)| *ns == namespace)
        .map(|(p, _)| *p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_maps_to_its_own_row() {
        assert!(MAPPINGS.iter().all(|m| m.field.mapping() == m));
    }

    #[test]
    fn info_keys_find_their_field() {
        assert_eq!(
            ["Title", "Author", "ModDate", "Trapped"].map(MappedField::from_info_key),
            [
                Some(MappedField::Title),
                Some(MappedField::Authors),
                Some(MappedField::Modified),
                None
            ]
        );
    }

    #[test]
    fn each_row_uses_its_namespaces_standard_prefix() {
        assert!(MAPPINGS.iter().all(|m| standard_prefix(m.namespace) == Some(m.prefix)));
    }
}
