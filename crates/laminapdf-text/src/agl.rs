//! Glyph names to Unicode: the Adobe Glyph List (AGL 2.0) and the ITC Zapf Dingbats Glyph
//! List, with the AGL specification's algorithm for the names they don't list (`uni20AC`,
//! `u1F600`, `f_f_i`, `a.sc`). Text extraction uses it for simple fonts without a ToUnicode
//! CMap (ISO 32000-2 §9.10.2); Lamina PDF's simple-font writer uses the same lists in reverse.
//!
//! The lists are © Adobe, BSD-3-Clause (`tools/std14-gen/data/README.md`).

use crate::std14::generated::agl::{AGL, ZAPF_DINGBATS};

/// The text glyph `name` stands for, by the AGL specification ("Mapping glyph names to
/// Unicode"): drop everything from the first `.`, split at `_`, and map each component through
/// the Adobe Glyph List, else as `uniXXXX[XXXX…]` (BMP code points, no surrogates) or
/// `uXXXX[XX]`. Components that map to nothing are dropped; `None` when nothing is left.
///
/// ```
/// use laminapdf_text::{glyph_name_to_unicode, zapf_dingbats_to_unicode};
///
/// assert_eq!(glyph_name_to_unicode("eacute").as_deref(), Some("é"));
/// assert_eq!(glyph_name_to_unicode("f_f_i").as_deref(), Some("ffi"));
/// assert_eq!(glyph_name_to_unicode("uni20AC.alt").as_deref(), Some("€"));
/// assert_eq!(zapf_dingbats_to_unicode("a20").as_deref(), Some("✔"));
/// assert_eq!(glyph_name_to_unicode("g37"), None);
/// ```
#[must_use]
pub fn glyph_name_to_unicode(name: &str) -> Option<String> {
    map_name(name, false)
}

/// As [`glyph_name_to_unicode`], but a component is first looked up in the ITC Zapf Dingbats Glyph List
/// (`a1` … `a191`): for glyphs of the `ZapfDingbats` font.
#[must_use]
pub fn zapf_dingbats_to_unicode(name: &str) -> Option<String> {
    map_name(name, true)
}

fn map_name(name: &str, zapf: bool) -> Option<String> {
    let base = name.split('.').next().unwrap_or_default();
    let mut out = String::new();
    for component in base.split('_') {
        if zapf && let Some(text) = lookup(&ZAPF_DINGBATS, component) {
            out.push_str(text);
        } else if let Some(text) = lookup(&AGL, component) {
            out.push_str(text);
        } else if let Some(text) = uni(component) {
            out.push_str(&text);
        } else if let Some(ch) = u(component) {
            out.push(ch);
        }
    }
    (!out.is_empty()).then_some(out)
}

fn lookup(list: &'static [(&'static str, &'static str)], name: &str) -> Option<&'static str> {
    let i = list.binary_search_by_key(&name, |&(n, _)| n).ok()?;
    list.get(i).map(|&(_, text)| text)
}

/// Uppercase hexadecimal digits only (the AGL specification rejects lowercase).
fn upper_hex(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b)) {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

/// `uniXXXX…`: one or more groups of four digits, each a BMP code point outside the surrogates.
fn uni(component: &str) -> Option<String> {
    let digits = component.strip_prefix("uni")?;
    if digits.is_empty() || digits.len() % 4 != 0 {
        return None;
    }
    (0..digits.len() / 4)
        .map(|i| {
            let group = digits.get(i * 4..i * 4 + 4)?;
            char::from_u32(upper_hex(group)?)
        })
        .collect()
}

/// `uXXXX` to `uXXXXXX`: one code point outside the surrogates.
fn u(component: &str) -> Option<char> {
    let digits = component.strip_prefix('u')?;
    if !(4..=6).contains(&digits.len()) {
        return None;
    }
    char::from_u32(upper_hex(digits)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_names_map_through_the_agl() {
        assert_eq!(glyph_name_to_unicode("Euro").as_deref(), Some("€"));
    }

    #[test]
    fn uni_sequences_map_to_several_characters() {
        assert_eq!(glyph_name_to_unicode("uni00410042").as_deref(), Some("AB"));
    }

    #[test]
    fn uni_rejects_surrogates() {
        assert_eq!(glyph_name_to_unicode("uniD800"), None);
    }

    #[test]
    fn uni_rejects_lowercase_hex() {
        assert_eq!(glyph_name_to_unicode("uni20ac"), None);
    }

    #[test]
    fn u_takes_supplementary_code_points() {
        assert_eq!(glyph_name_to_unicode("u1F600").as_deref(), Some("😀"));
    }

    #[test]
    fn suffixes_are_dropped() {
        assert_eq!(glyph_name_to_unicode("a.sc").as_deref(), Some("a"));
    }

    #[test]
    fn zapf_names_need_the_zapf_list() {
        assert_eq!(glyph_name_to_unicode("a1"), None);
    }

    #[test]
    fn unknown_components_are_dropped() {
        assert_eq!(glyph_name_to_unicode("f_g37_i").as_deref(), Some("fi"));
    }
}
