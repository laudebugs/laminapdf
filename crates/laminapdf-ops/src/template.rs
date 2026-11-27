//! Fields in a header or footer's text (ADR 0032 §3): `{name}` for a value the caller knows,
//! `{{` and `}}` for braces. One scanner serves the flowing layout's page templates and the
//! headers and footers stamped on existing pages, which differ only in the names they know.

use std::borrow::Cow;

/// `text` with each `{name}` replaced by what `field` gives for `name`. `{{` and `}}` become
/// braces; a name `field` doesn't know, and a brace that opens no field, stay as written.
///
/// ```
/// use std::borrow::Cow;
///
/// use laminapdf_ops::fill_template;
///
/// let filled = fill_template("Page {page} of {pages} {{x}}", |name| match name {
///     "page" => Some(Cow::Borrowed("3")),
///     "pages" => Some(Cow::Owned(12.to_string())),
///     _ => None,
/// });
/// assert_eq!(filled, "Page 3 of 12 {x}");
/// ```
///
/// Added in 0.7.0.
#[must_use]
pub fn fill_template<'a>(text: &str, field: impl Fn(&str) -> Option<Cow<'a, str>>) -> String {
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
        let named = tail.strip_prefix('{').and_then(|t| t.split_once('}'));
        if let Some((name, after)) = named
            && let Some(value) = field(name)
        {
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
