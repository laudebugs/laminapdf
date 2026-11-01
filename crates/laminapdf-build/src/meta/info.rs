//! The document information dictionary (§14.3.3).

use laminapdf_cos::{Dict, PdfString};
use laminapdf_model::PdfDate;
use laminapdf_syntax::text::encode_text_string;

/// Whether XML 1.0 can't hold `ch` (§2.2 `Char`).
pub(crate) const fn not_xml(ch: char) -> bool {
    matches!(ch, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')
}

/// What the Info dictionary and the XMP packet say about the document. Dates are given,
/// never read from a clock (ADR 0001): the same values give the same bytes. Characters XML
/// can't hold (C0 controls but tab, line feed and carriage return) are left out of both.
///
/// ```
/// use laminapdf_build::Metadata;
///
/// let meta = Metadata::default()
///     .with_title("Annual report")
///     .with_author("A. Writer")
///     .with_creation_date("D:20261029120000Z".parse()?);
/// assert_eq!(meta.title.as_deref(), Some("Annual report"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Metadata {
    /// `/Title`, `dc:title`.
    pub title: Option<String>,
    /// `/Author`, `dc:creator` (one entry).
    pub author: Option<String>,
    /// `/Subject`, `dc:description`.
    pub subject: Option<String>,
    /// `/Keywords`, `pdf:Keywords`.
    pub keywords: Option<String>,
    /// `/Creator` (the application the content came from), `xmp:CreatorTool`.
    pub creator: Option<String>,
    /// `/Producer` (the application that wrote the PDF), `pdf:Producer`.
    pub producer: Option<String>,
    /// `/CreationDate`, `xmp:CreateDate`.
    pub creation_date: Option<PdfDate>,
    /// `/ModDate`, `xmp:ModifyDate`.
    pub mod_date: Option<PdfDate>,
}

impl Metadata {
    /// With the title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// With the author.
    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// With the subject.
    #[must_use]
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// With the keywords.
    #[must_use]
    pub fn with_keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// With the creator.
    #[must_use]
    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.creator = Some(creator.into());
        self
    }

    /// With the producer.
    #[must_use]
    pub fn with_producer(mut self, producer: impl Into<String>) -> Self {
        self.producer = Some(producer.into());
        self
    }

    /// With the creation date.
    #[must_use]
    pub const fn with_creation_date(mut self, date: PdfDate) -> Self {
        self.creation_date = Some(date);
        self
    }

    /// With the modification date.
    #[must_use]
    pub const fn with_mod_date(mut self, date: PdfDate) -> Self {
        self.mod_date = Some(date);
        self
    }

    /// Removes the characters XML 1.0 can't hold (C0 controls other than tab, line feed and
    /// carriage return; U+FFFE, U+FFFF) from every text value, so the Info dictionary and the
    /// XMP packet written from it say the same.
    pub(crate) fn sanitize(&mut self) {
        let fields = [
            &mut self.title,
            &mut self.author,
            &mut self.subject,
            &mut self.keywords,
            &mut self.creator,
            &mut self.producer,
        ];
        for text in fields.into_iter().flatten() {
            if text.chars().any(not_xml) {
                text.retain(|c| !not_xml(c));
            }
        }
    }

    /// The Info dictionary: text strings in PDFDocEncoding or UTF-16BE, dates as written by
    /// [`PdfDate`]'s `Display`.
    pub(crate) fn info_dict(&self) -> Dict {
        let mut d = Dict::new();
        for (key, value) in [
            ("Title", &self.title),
            ("Author", &self.author),
            ("Subject", &self.subject),
            ("Keywords", &self.keywords),
            ("Creator", &self.creator),
            ("Producer", &self.producer),
        ] {
            if let Some(text) = value {
                d.insert(key, PdfString::literal(encode_text_string(text)));
            }
        }
        for (key, value) in [("CreationDate", &self.creation_date), ("ModDate", &self.mod_date)] {
            if let Some(date) = value {
                d.insert(key, PdfString::literal(date.to_string()));
            }
        }
        d
    }
}
