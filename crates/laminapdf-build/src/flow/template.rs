//! Header and footer fields (ADR 0032 §3): `{page}`, `{pages}`, `{section}` and `{title}` in a
//! slot's text, `{{` and `}}` for braces. Headers and footers stamped on existing pages use the
//! same syntax.

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
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(['{', '}']) {
        out.push_str(rest.get(..at).unwrap_or_default());
        let tail = rest.get(at..).unwrap_or_default();
        if let Some(after) = tail.strip_prefix("{{") {
            out.push('{');
            rest = after;
            continue;
        }
        if let Some(after) = tail.strip_prefix("}}") {
            out.push('}');
            rest = after;
            continue;
        }
        let field = tail.strip_prefix('{').and_then(|t| t.split_once('}'));
        let value = field.and_then(|(name, _)| match name {
            "page" => Some(fields.page.to_string()),
            "pages" => Some(fields.pages.to_string()),
            "section" => Some(fields.section.unwrap_or_default().to_owned()),
            "title" => Some(fields.title.unwrap_or_default().to_owned()),
            _ => None,
        });
        if let (Some(value), Some((_, after))) = (value, field) {
            out.push_str(&value);
            rest = after;
        } else {
            // Not a field: the brace as written.
            out.push_str(tail.get(..1).unwrap_or_default());
            rest = tail.get(1..).unwrap_or_default();
        }
    }
    out.push_str(rest);
    out
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
