//! Attributes of raw HTML start tags: explicit anchors (`id`, `name`) and link targets (`href`,
//! `src`).
//!
//! A small tokenizer, not a full HTML parser: comments are skipped, attribute names are
//! case-insensitive, values may be double-quoted, single-quoted or unquoted, and whitespace is
//! allowed around `=`.

/// The `id`/`name` values in a fragment of HTML, in order.
pub(crate) fn anchors(html: &str) -> Vec<String> {
    values(html, &["id", "name"])
}

/// The `href`/`src` values in a fragment of HTML, in order.
pub(crate) fn targets(html: &str) -> Vec<String> {
    values(html, &["href", "src"])
}

/// The values of the attributes called one of `names` (case-insensitive), in order.
fn values(html: &str, names: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        let (_, tag) = split(rest, lt);
        if let Some(comment) = tag.strip_prefix("<!--") {
            rest = comment.find("-->").map_or("", |end| split(comment, end + 3).1);
            continue;
        }
        let after_lt = tag.get(1..).unwrap_or_default();
        if !after_lt.starts_with(|c: char| c.is_ascii_alphabetic()) {
            rest = after_lt;
            continue;
        }
        let name_end = after_lt
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(after_lt.len());
        rest = attributes(split(after_lt, name_end).1, names, &mut found);
    }
    found
}

/// Reads the attributes of one start tag up to its `>`, pushing the values of those called one of
/// `names` to `found`; returns the text after the tag.
fn attributes<'a>(mut s: &'a str, names: &[&str], found: &mut Vec<String>) -> &'a str {
    loop {
        s = s.trim_start();
        let Some(first) = s.chars().next() else {
            return s;
        };
        match first {
            '>' => return split(s, 1).1,
            '/' | '"' | '\'' | '=' => {
                s = split(s, first.len_utf8()).1;
                continue;
            }
            _ => {}
        }
        let name_end = s
            .find(|c: char| c.is_whitespace() || matches!(c, '=' | '>' | '/' | '"' | '\''))
            .unwrap_or(s.len());
        let (name, after_name) = split(s, name_end);
        s = after_name;
        let Some(value_start) = after_name.trim_start().strip_prefix('=') else {
            continue;
        };
        let value_start = value_start.trim_start();
        let value = if let Some(quote @ ('"' | '\'')) = value_start.chars().next() {
            let body = split(value_start, 1).1;
            let end = body.find(quote).unwrap_or(body.len());
            let (value, after) = split(body, end);
            s = after.get(1..).unwrap_or_default();
            value
        } else {
            let end = value_start
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(value_start.len());
            let (value, after) = split(value_start, end);
            s = after;
            value
        };
        if names.iter().any(|n| name.eq_ignore_ascii_case(n)) && !value.is_empty() {
            found.push(value.to_owned());
        }
    }
}

/// `s` split at byte `at` (a char boundary found by `find`); everything left of it if not.
fn split(s: &str, at: usize) -> (&str, &str) {
    s.split_at_checked(at).unwrap_or((s, ""))
}

#[cfg(test)]
mod tests {
    use super::{anchors, targets};

    #[test]
    fn href_and_src_values_are_found() {
        assert_eq!(
            targets(r#"<a HREF="https://a.example">x</a><img src=i.png id="z">"#),
            ["https://a.example", "i.png"]
        );
    }

    #[test]
    fn quoted_ids_and_names_are_found() {
        assert_eq!(anchors(r#"<a id="one"></a> <a name='two'></a>"#), ["one", "two"]);
    }

    #[test]
    fn unquoted_values_are_found() {
        assert_eq!(anchors("<a id=bare>x</a>"), ["bare"]);
    }

    #[test]
    fn attribute_names_are_case_insensitive() {
        assert_eq!(anchors(r#"<A ID="upper" Name="mixed">"#), ["upper", "mixed"]);
    }

    #[test]
    fn whitespace_around_equals_is_allowed() {
        assert_eq!(anchors(r#"<a id = "spaced" >"#), ["spaced"]);
    }

    #[test]
    fn ids_inside_comments_are_ignored() {
        assert_eq!(anchors(r#"<!-- <a id="hidden"> --><a id="shown">"#), ["shown"]);
    }

    #[test]
    fn other_attributes_ending_in_id_are_ignored() {
        assert_eq!(anchors(r#"<div data-id="x" class="id"></div>"#), Vec::<String>::new());
    }

    #[test]
    fn closing_tags_and_text_are_ignored() {
        assert_eq!(anchors("</a> 1 < 2 id=\"no\""), Vec::<String>::new());
    }
}
