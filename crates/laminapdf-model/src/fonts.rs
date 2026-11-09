//! The fonts a document's pages use (ISO 32000-2 §9.5–9.7), each listed once, for a
//! properties view: its name, type, whether the file embeds it and whether that is a subset.
//!
//! Fonts are found in each page's resources and, through them, in the Form XObjects the page
//! draws (§8.10) and in Type 3 fonts' own resources (§9.6.4), each container read once. The
//! walk is bounded by [`Limits::max_tree_depth`](laminapdf_cos::Limits::max_tree_depth); what
//! can't be read is left out (ADR 0003), never fails the list.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, Object, Result};

use crate::document::Document;
use crate::page::Pages;

/// One font the document uses.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FontResource {
    /// `/BaseFont` without a subset tag (`ABCDEF+`), lossily decoded; `None` when absent.
    pub name: Option<String>,
    /// `/Subtype`: `Type0`, `Type1`, `MMType1`, `Type3` or `TrueType`, as stored.
    pub subtype: Option<String>,
    /// The file holds the font program (`/FontFile`, `/FontFile2` or `/FontFile3` in its
    /// descriptor, a Type 0 font's descendant's), or its glyphs (Type 3).
    pub embedded: bool,
    /// `/BaseFont` starts with a subset tag: six capital letters and `+` (§9.9.2).
    pub subset: bool,
    /// `/Encoding` when it is a name (or a Type 0 font's CMap name); `Some("custom")` for an
    /// encoding dictionary or an embedded CMap; `None` when absent.
    pub encoding: Option<String>,
    /// The font dictionary's object number, when it is an indirect object.
    pub object: Option<u32>,
}

impl Pages<'_> {
    /// Every font these pages use, once each, in the order first met.
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    pub fn fonts(&self) -> Result<Vec<FontResource>> {
        let mut walk = Walk {
            doc: self.document(),
            max_depth: self.document().limits().max_tree_depth(),
            seen_containers: BTreeSet::new(),
            seen_fonts: BTreeSet::new(),
            out: Vec::new(),
        };
        for page in self.iter() {
            if let Some(resources) = page.resources() {
                walk.resources(resources, 0)?;
            }
        }
        Ok(walk.out)
    }
}

struct Walk<'d> {
    doc: &'d Document,
    max_depth: usize,
    /// Form XObjects, Type 3 fonts and resource dictionaries already read, by object number.
    seen_containers: BTreeSet<u32>,
    /// Fonts already listed: an object number, or for a direct font its fields.
    seen_fonts: BTreeSet<String>,
    out: Vec<FontResource>,
}

impl Walk<'_> {
    fn resources(&mut self, resources: &Dict, depth: usize) -> Result<()> {
        if depth > self.max_depth {
            return Ok(());
        }
        if let Some(fonts) = self.dict(resources.get("Font"))? {
            for (_, value) in fonts.iter() {
                self.font(value, depth)?;
            }
        }
        if let Some(xobjects) = self.dict(resources.get("XObject"))? {
            for (_, value) in xobjects.iter() {
                if let Some(r) = value.as_ref()
                    && !self.seen_containers.insert(r.num)
                {
                    continue;
                }
                let Some(object) = self.doc.read(value)? else { continue };
                let Object::Stream(form) = &*object else { continue };
                if form.dict.get_name("Subtype") != Some(b"Form") {
                    continue;
                }
                if let Some(inner) = self.dict(form.dict.get("Resources"))? {
                    self.resources(&inner, depth + 1)?;
                }
            }
        }
        Ok(())
    }

    fn font(&mut self, value: &Object, depth: usize) -> Result<()> {
        let Some(object) = self.doc.read(value)? else {
            return Ok(());
        };
        let Some(font) = object.as_dict() else { return Ok(()) };
        let reference = value.as_ref().map(|r| r.num);
        let subtype = font.get_name("Subtype").map(lossy);
        let base = font.get_name("BaseFont").map(lossy);
        let key = match reference {
            Some(num) => num.to_string(),
            None => format!("{subtype:?}/{base:?}"),
        };
        if !self.seen_fonts.insert(key) {
            return Ok(());
        }
        let subset = base.as_deref().is_some_and(has_subset_tag);
        let name = base.map(|b| {
            if subset {
                b.get(7..).unwrap_or_default().to_owned()
            } else {
                b
            }
        });
        let type3 = subtype.as_deref() == Some("Type3");
        let descriptor_font = if subtype.as_deref() == Some("Type0") {
            self.descendant(font)?
        } else {
            None
        };
        let embedded = type3 || self.embeds(descriptor_font.as_ref().unwrap_or(font))?;
        let encoding = self.encoding(font)?;
        self.out.push(FontResource {
            name,
            subtype,
            embedded,
            subset,
            encoding,
            object: reference,
        });
        // A Type 3 font draws its glyphs with content of its own, which can use fonts too.
        if type3 {
            if let Some(r) = value.as_ref()
                && !self.seen_containers.insert(r.num)
            {
                return Ok(());
            }
            if let Some(inner) = self.dict(font.get("Resources"))? {
                self.resources(&inner, depth + 1)?;
            }
        }
        Ok(())
    }

    /// A Type 0 font's first descendant font dictionary.
    fn descendant(&self, font: &Dict) -> Result<Option<Dict>> {
        let Some(list) = font.get("DescendantFonts") else {
            return Ok(None);
        };
        let Some(list) = self.doc.read(list)? else {
            return Ok(None);
        };
        let Some(first) = list.as_array().and_then(<[Object]>::first) else {
            return Ok(None);
        };
        self.dict(Some(first))
    }

    /// Whether `font`'s descriptor holds a font program.
    fn embeds(&self, font: &Dict) -> Result<bool> {
        let Some(descriptor) = self.dict(font.get("FontDescriptor"))? else {
            return Ok(false);
        };
        Ok(["FontFile", "FontFile2", "FontFile3"]
            .iter()
            .any(|key| descriptor.get(key).is_some()))
    }

    fn encoding(&self, font: &Dict) -> Result<Option<String>> {
        let Some(value) = font.get("Encoding") else {
            return Ok(None);
        };
        Ok(match self.doc.read(value)?.as_deref() {
            Some(Object::Name(n)) => Some(lossy(n.as_bytes())),
            Some(Object::Dict(_) | Object::Stream(_)) => Some("custom".to_owned()),
            _ => None,
        })
    }

    /// The dictionary `value` holds (a stream's dictionary too), resolved.
    fn dict(&self, value: Option<&Object>) -> Result<Option<Dict>> {
        let Some(value) = value else { return Ok(None) };
        Ok(match self.doc.read(value)?.as_deref() {
            Some(Object::Dict(d)) => Some(d.clone()),
            Some(Object::Stream(s)) => Some(s.dict.clone()),
            _ => None,
        })
    }
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Six capital letters and `+` (§9.9.2).
fn has_subset_tag(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() > 7 && bytes.get(6) == Some(&b'+') && bytes.iter().take(6).all(u8::is_ascii_uppercase)
}

#[cfg(test)]
mod tests {
    use super::has_subset_tag;

    #[test]
    fn a_subset_tag_is_six_capitals_and_a_plus() {
        assert!(has_subset_tag("ABCDEF+NotoSans-Regular"));
        assert!(!has_subset_tag("ABCDEf+NotoSans"));
        assert!(!has_subset_tag("ABCDEF+"));
        assert!(!has_subset_tag("Helvetica"));
    }
}
