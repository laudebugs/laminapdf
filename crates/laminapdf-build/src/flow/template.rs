//! Header and footer fields (ADR 0032 §3): `{page}`, `{pages}`, `{section}` and `{title}` in a
//! slot's text, `{{` and `}}` for braces. Headers and footers stamped on existing pages use the
//! same syntax.

use std::borrow::Cow;

/// What the fields stand for on one page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
pub struct Fields<'a> {
    /// The 1-based page number.
    pub page: usize,
    /// The number of pages.
    pub pages: usize,
    /// The section's title, if a section has started.
    pub section: Option<&'a str>,
    /// The document's title.
    pub title: Option<&'a str>,
}

/// `text` with its fields filled in. An unknown field, or a brace that opens none, stays as
/// written.
///
/// ```
/// use laminapdf_build::flow::{Fields, fill_fields};
///
/// let fields = Fields { page: 3, pages: 12, section: Some("Results"), title: None };
/// assert_eq!(fill_fields("Page {page} of {pages}", &fields), "Page 3 of 12");
/// assert_eq!(fill_fields("{section} · {title} {{x}}", &fields), "Results ·  {x}");
/// ```
///
/// Added in 0.2.0.
#[must_use]
pub fn fill_fields(text: &str, fields: &Fields<'_>) -> String {
    laminapdf_ops::fill_template(text, |name| match name {
        "page" => Some(Cow::Owned(fields.page.to_string())),
        "pages" => Some(Cow::Owned(fields.pages.to_string())),
        "section" => Some(Cow::Borrowed(fields.section.unwrap_or_default())),
        "title" => Some(Cow::Borrowed(fields.title.unwrap_or_default())),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELDS: Fields<'static> = Fields {
        page: 2,
        pages: 9,
        section: None,
        title: Some("Report"),
    };

    #[test]
    fn unknown_fields_and_lone_braces_stay_as_written() {
        assert_eq!(fill_fields("{nope} { } }", &FIELDS), "{nope} { } }");
    }

    #[test]
    fn a_missing_section_is_empty() {
        assert_eq!(fill_fields("[{section}] {title}", &FIELDS), "[] Report");
    }
}
