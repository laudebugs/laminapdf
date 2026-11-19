//! What an exchange file holds, whatever its format: the file it was made for, and each comment
//! as the entries of its annotation dictionary, the way ISO 19444-1 defines XFDF in terms of PDF
//! keys. The XFDF and FDF readers fill it, the writers write it, and the importer turns it into
//! ops.

use laminapdf_cos::Dict;
use laminapdf_model::UserSpaceRect;

/// An exchange format.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentFormat {
    /// XFDF (ISO 19444-1): XML.
    Xfdf,
    /// FDF (ISO 32000-2 §12.7.8): PDF syntax.
    Fdf,
    /// Markdown, written for notes apps; never read.
    Markdown,
}

impl CommentFormat {
    /// The format's name as people know it: `XFDF`, `FDF` or `Markdown`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Xfdf => "XFDF",
            Self::Fdf => "FDF",
            Self::Markdown => "Markdown",
        }
    }
}

impl std::fmt::Display for CommentFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The comments of an exchange file, and the PDF they were made for.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Exchange {
    /// The PDF's name as the file gives it (XFDF's `<f href>`, FDF's `/F`).
    pub file: Option<String>,
    /// The PDF's `/ID`: its permanent and its changing identifier.
    pub ids: Option<[Vec<u8>; 2]>,
    /// The comments, in the file's order.
    pub comments: Vec<Comment>,
    /// The form fields' values, in the file's order.
    ///
    /// Added in 0.7.0.
    pub fields: Vec<crate::FieldData>,
    /// What the file holds that isn't imported, in the file's order.
    pub skipped: Vec<SkippedComment>,
}

/// One comment: an annotation's entries, its page and what refers to other objects.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Comment {
    /// The page, counted from 0.
    pub page: u32,
    /// The annotation dictionary's entries (`/Subtype`, `/Rect`, `/NM`, `/Contents`, …), without
    /// those that name other objects: `/P`, `/Popup`, `/IRT`, `/FS`, `/AP` and `/Parent`.
    pub entries: Dict,
    /// The `/NM` of the comment it answers or is grouped with (`/IRT`).
    pub in_reply_to: Option<String>,
    /// Its Popup.
    pub popup: Option<Popup>,
    /// A file attachment's file.
    pub file: Option<ExchangedFile>,
    /// Where it starts in the exchange file, counted from 1; for messages.
    pub line: Option<u32>,
}

impl Comment {
    /// A comment of `subtype` on `page`, with no other entries.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(page: u32, subtype: &str) -> Self {
        let mut entries = Dict::new();
        entries.insert("Subtype", laminapdf_cos::Object::name(subtype));
        Self {
            page,
            entries,
            in_reply_to: None,
            popup: None,
            file: None,
            line: None,
        }
    }

    /// Its `/Subtype`, empty when it has none.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn subtype(&self) -> &str {
        self.entries
            .get_name("Subtype")
            .and_then(|n| std::str::from_utf8(n).ok())
            .unwrap_or_default()
    }

    /// Its `/NM`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn name(&self) -> Option<String> {
        text(&self.entries, "NM")
    }
}

/// A text string entry of `dict`, decoded.
pub(crate) fn text(dict: &Dict, key: &str) -> Option<String> {
    match dict.get(key) {
        Some(laminapdf_cos::Object::String(s)) => Some(laminapdf_syntax::text::decode_text_string(&s.bytes)),
        _ => None,
    }
}

/// A comment's Popup: where it opens and whether it starts open.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Popup {
    /// Its `/Rect`, in user space.
    pub rect: UserSpaceRect,
    /// `/Open`.
    pub open: bool,
}

impl Popup {
    /// A Popup at `rect`, open or closed.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(rect: UserSpaceRect, open: bool) -> Self {
        Self { rect, open }
    }
}

/// A file attachment's file, carried in the exchange file.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExchangedFile {
    /// Its name.
    pub name: String,
    /// Its MIME type; empty for none.
    pub mime: String,
    /// Its bytes.
    pub bytes: Vec<u8>,
    /// Its description.
    pub description: Option<String>,
    /// When it was made, as a PDF date.
    pub created: Option<String>,
    /// When it was last changed, as a PDF date.
    pub modified: Option<String>,
}

/// Something an exchange file holds that an import leaves out.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkippedComment {
    /// Its page, counted from 0, when it has one.
    pub page: Option<u32>,
    /// What it is: an annotation's subtype (`Sound`), or `field` for a form field.
    pub kind: String,
    /// Why it is left out.
    pub cause: SkipCause,
    /// Where it starts in the exchange file, counted from 1.
    pub line: Option<u32>,
}

/// Why an import leaves something out.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SkipCause {
    /// A kind of annotation Lamina PDF doesn't make: a sound, a link, a movie, a redaction, a
    /// widget, one another app invented.
    Unsupported,
    /// A form field's value, which comment import leaves to form data.
    FormField,
    /// Its page isn't in the document.
    MissingPage,
    /// What it answers isn't in the file or the document.
    MissingParent,
    /// An entry it can't do without is missing or malformed.
    Incomplete,
}

impl SkipCause {
    /// A short stable name: `unsupported`, `form-field`, `missing-page`, `missing-parent` or
    /// `incomplete`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::FormField => "form-field",
            Self::MissingPage => "missing-page",
            Self::MissingParent => "missing-parent",
            Self::Incomplete => "incomplete",
        }
    }
}
