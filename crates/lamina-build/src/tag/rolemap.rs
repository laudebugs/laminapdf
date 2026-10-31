//! The `RoleMap` (§14.7.3): each custom structure type the document uses, mapped to the
//! standard type it stands for, in order of first use (deterministic, and without a sort or a
//! map in the WASM build: documents use a handful of custom types).

use lamina_cos::{Dict, Name};

use super::Tag;

/// The role map of `tags`, or `None` when every one is a standard type. A name used with two
/// roles keeps the first.
pub(crate) fn role_map<'a>(tags: impl IntoIterator<Item = &'a Tag>) -> Option<Dict> {
    let mut d = Dict::new();
    for tag in tags {
        if let Some(name) = tag.custom_name()
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
