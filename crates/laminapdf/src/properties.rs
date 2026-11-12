//! What a properties view shows about a document, in one call: its Info entries and dates,
//! version, pages, size, tagging, the fonts it uses, its encryption and permissions, its
//! signatures, and whether it holds JavaScript, attachments or a form. Everything is read by
//! Lamina PDF's own parser; nothing is validated (a signature is counted, never checked).

use laminapdf_cos::Cipher;
use laminapdf_model::{ActionKind, FontEncoding, PdfDate};

use crate::document::{Document, DocumentInfo};
use crate::error::Result;

/// A document's properties.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DocumentProperties {
    /// The Info dictionary's text entries.
    pub info: DocumentInfo,
    /// `/CreationDate`, as an ISO 8601 date-time (`2026-10-14T09:12:00+02:00`, or without an
    /// offset when the file gives none); `None` when absent or not a valid PDF date.
    pub created: Option<String>,
    /// `/ModDate`, as [`DocumentProperties::created`].
    pub modified: Option<String>,
    /// The XMP metadata packet (§14.3.2) as text, for the caller to read; `None` without one.
    pub xmp: Option<String>,
    /// The PDF version ([`Document::version`]).
    pub version: Option<String>,
    /// The number of pages.
    pub page_count: u32,
    /// The size of the file as opened, in bytes.
    pub file_size: u64,
    /// The file is linearized ("fast web view", Annex F).
    pub linearized: bool,
    /// The catalog says the document is tagged ([`Document::is_tagged`]).
    pub tagged: bool,
    /// How the file is encrypted; `None` when it isn't.
    pub encryption: Option<EncryptionSummary>,
    /// The fonts the pages use, each once.
    pub fonts: Vec<FontSummary>,
    /// The signatures present: signed fields and `/Perms` entries, not verified.
    pub signatures: u32,
    /// The signature fields' full names, signed or not.
    pub signature_fields: Vec<String>,
    /// What the document holds besides its pages.
    pub contains: DocumentContents,
}

/// What a document holds besides its pages.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DocumentContents {
    /// Some action runs JavaScript (a document, page, annotation or field trigger).
    pub javascript: bool,
    /// The number of embedded files.
    pub attachments: u32,
    /// The document has an interactive form with at least one field.
    pub forms: bool,
}

/// How a file is encrypted (§7.6) and what its author permits.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EncryptionSummary {
    /// `/V`.
    pub version: u8,
    /// `/R`: the standard security handler's revision.
    pub revision: u8,
    /// The file key's length in bits.
    pub key_bits: u16,
    /// The cipher streams use.
    pub cipher: Cipher,
    /// What `/P` permits; `None` when the handler gives no permissions.
    pub permissions: Option<DocumentPermissions>,
}

/// What a file's author permits (`/P`, ISO 32000-2 Table 22). A request to readers, not a lock.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per permission bit, as Table 22 lists them"
)]
pub struct DocumentPermissions {
    /// Printing (bit 3).
    pub print: bool,
    /// Printing at full quality (bit 12; bit 3 under revision 2).
    pub print_high_quality: bool,
    /// Changing the contents (bit 4).
    pub modify: bool,
    /// Copying text and graphics (bit 5).
    pub copy: bool,
    /// Adding or changing annotations (bit 6).
    pub annotate: bool,
    /// Filling form fields (bit 9).
    pub fill_forms: bool,
    /// Extracting text for accessibility (bit 10).
    pub accessibility: bool,
    /// Assembling: inserting, rotating or deleting pages (bit 11).
    pub assemble: bool,
}

/// One font the document uses.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FontSummary {
    /// Its name without a subset tag; `None` when the file gives none.
    pub name: Option<String>,
    /// Its type: `Type0`, `Type1`, `MMType1`, `Type3` or `TrueType`, as stored.
    pub kind: Option<String>,
    /// The file holds the font (or, for Type 3, its glyphs).
    pub embedded: bool,
    /// The file holds only the glyphs used.
    pub subset: bool,
    /// Its encoding; `None` when the font gives none.
    pub encoding: Option<FontEncoding>,
}

impl Document {
    /// The document's properties, for a properties view. It reads every object once (to find
    /// JavaScript, attachments and form fields), so call it when the view opens, not per page.
    ///
    /// # Errors
    /// Reading errors (`data-pending` for a range document, a failed source).
    ///
    /// Added in 0.2.0.
    pub fn properties(&self) -> Result<DocumentProperties> {
        let model = self.ops();
        let disclosure = model.disclosure()?;
        let (created, modified) = match model.info()? {
            Some(info) => (
                info.creation_date()?.and_then(|d| d.parsed).map(iso_8601),
                info.mod_date()?.and_then(|d| d.parsed).map(iso_8601),
            ),
            None => (None, None),
        };
        let fonts = model
            .pages()?
            .fonts()?
            .into_iter()
            .map(|f| FontSummary {
                name: f.name,
                kind: f.subtype,
                embedded: f.embedded,
                subset: f.subset,
                encoding: f.encoding,
            })
            .collect();
        let planner = laminapdf_save::SavePlanner::new(self.ops())?;
        let constraint = planner.constraint();
        let signatures = u32::try_from(constraint.signatures.len()).unwrap_or(u32::MAX);
        let signature_fields = disclosure
            .fields
            .iter()
            .filter(|f| f.kind.as_deref() == Some("Sig"))
            .map(|f| f.name.clone())
            .collect();
        Ok(DocumentProperties {
            info: self.info()?,
            created,
            modified,
            xmp: model.xmp_packet()?.map(|b| String::from_utf8_lossy(&b).into_owned()),
            version: self.version()?,
            page_count: self.page_count()?,
            file_size: model.source().len(),
            linearized: disclosure.linearization.is_some(),
            tagged: self.is_tagged()?,
            encryption: self.encryption_summary(),
            fonts,
            signatures,
            signature_fields,
            contains: DocumentContents {
                javascript: disclosure.actions.iter().any(|a| a.kind == ActionKind::JavaScript),
                attachments: u32::try_from(disclosure.attachments.len()).unwrap_or(u32::MAX),
                forms: !disclosure.fields.is_empty(),
            },
        })
    }

    fn encryption_summary(&self) -> Option<EncryptionSummary> {
        let model = self.ops();
        let encryption = model.encryption()?;
        let permissions = model.permissions().map(|p| DocumentPermissions {
            print: p.print_low_res(),
            print_high_quality: p.print_high_res(),
            modify: p.modify(),
            copy: p.extract(),
            annotate: p.annotate(),
            fill_forms: p.fill_forms(),
            accessibility: p.accessibility(),
            assemble: p.assemble(),
        });
        Some(EncryptionSummary {
            version: encryption.version,
            revision: encryption.revision,
            key_bits: encryption.key_bits,
            cipher: encryption.stream_cipher,
            permissions,
        })
    }
}

/// `date` as ISO 8601, with its offset when the file gives one.
fn iso_8601(date: PdfDate) -> String {
    let base = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        date.year(),
        date.month(),
        date.day(),
        date.hour(),
        date.minute(),
        date.second()
    );
    match date.utc_offset_minutes() {
        None => base,
        Some(0) => format!("{base}Z"),
        Some(m) => {
            let sign = if m < 0 { '-' } else { '+' };
            let m = m.unsigned_abs();
            format!("{base}{sign}{:02}:{:02}", m / 60, m % 60)
        }
    }
}
