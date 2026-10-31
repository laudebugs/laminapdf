//! The `RoleMap` (§14.7.3): each custom structure type the document uses, mapped to the
//! standard type it stands for, in order of first use (deterministic, and without a sort or a
//! map in the WASM build: documents use a handful of custom types).

use lamina_cos::{Dict, Name};

use super::Tag;

/// Every standard structure type of ISO 32000-1 §14.8.4 (the PDF 1.7 namespace) and of
/// ISO 32000-2 §14.8.4 (the PDF 2.0 namespace) that [`TagKind`](super::TagKind) doesn't
/// have; `Hn` headings are matched by [`is_standard`]. Standard types are never role-mapped
/// (§14.7.3).
const OTHER_STANDARD: [&str; 32] = [
    "Art",
    "Div",
    "BlockQuote",
    "TOC",
    "TOCI",
    "Index",
    "NonStruct",
    "Private",
    "H",
    "THead",
    "TBody",
    "TFoot",
    "Reference",
    "BibEntry",
    "Annot",
    "Ruby",
    "RB",
    "RT",
    "RP",
    "Warichu",
    "WT",
    "WP",
    "Formula",
    "Form",
    "DocumentFragment",
    "Aside",
    "Title",
    "FENote",
    "Sub",
    "Em",
    "Strong",
    "Artifact",
];

/// Whether `name` is a standard structure type (PDF 1.7 or 2.0 namespace), `H1`, `H7` and
/// any other `Hn` included.
pub(crate) fn is_standard(name: &str) -> bool {
    let heading = name
        .strip_prefix('H')
        .is_some_and(|n| !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit()));
    heading || OTHER_STANDARD.contains(&name) || super::TagKind::from_name(name).is_some()
}

/// The role map of `tags`, or `None` when every one is a standard type. A name used with two
/// roles keeps the first; a custom name that is a standard type is written unmapped.
pub(crate) fn role_map<'a>(tags: impl IntoIterator<Item = &'a Tag>) -> Option<Dict> {
    let mut d = Dict::new();
    for tag in tags {
        if let Some(name) = tag.custom_name()
            && !is_standard(name)
            && !d.contains_key(name)
        {
            d.insert(Name::from(name), Name::from(tag.kind().as_str()));
        }
    }
    (!d.is_empty()).then_some(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TagKind;

    #[test]
    fn standard_types_need_no_role_map() {
        assert_eq!(role_map(&[Tag::new(TagKind::P)]), None);
    }

    #[test]
    fn standard_types_outside_tag_kind_are_never_mapped() {
        let tags = [
            Tag::custom("Div", TagKind::Sect),
            Tag::custom("BlockQuote", TagKind::Quote),
            Tag::custom("H7", TagKind::H6),
            Tag::custom("Strong", TagKind::Span),
        ];
        assert_eq!(role_map(&tags), None);
    }

    #[test]
    fn only_digits_after_h_make_a_heading() {
        assert_eq!(
            [
                is_standard("H12"),
                is_standard("H"),
                is_standard("Hx"),
                is_standard("H0")
            ],
            [true, true, false, false]
        );
    }

    #[test]
    fn custom_names_are_mapped_once_in_order_of_first_use() {
        let tags = [
            Tag::custom("Total", TagKind::P),
            Tag::custom("Amount", TagKind::Span),
            Tag::custom("Total", TagKind::P),
        ];
        let map = role_map(&tags).unwrap();
        let keys: Vec<&[u8]> = map.iter().map(|(k, _)| k.as_bytes()).collect();
        assert_eq!(keys, [b"Total".as_slice(), b"Amount"]);
    }
}
