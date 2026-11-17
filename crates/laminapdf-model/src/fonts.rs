//! The fonts a document's pages use (ISO 32000-2 §9.5–9.7), each listed once, for a
//! properties view: its name, type, whether the file embeds it and whether that is a subset.
//!
//! Fonts are found in each page's resources and, through them, in the Form XObjects the page
//! draws (§8.10), its tiling patterns (§8.7.3.1), its soft masks' groups (§11.6.5.2) and Type 3
//! fonts' own resources (§9.6.4), and its annotations' appearance streams (§12.5.5), each
//! container read once. The walk is bounded by [`Limits::max_tree_depth`](laminapdf_cos::Limits::max_tree_depth); what
//! can't be read is left out (ADR 0003), never fails the list.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object, Result};

use crate::document::Document;
use crate::page::{Page, Pages};

/// One font the document uses.
///
/// Added in 0.2.0.
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
    /// `/Encoding`; `None` when absent.
    pub encoding: Option<FontEncoding>,
    /// The font dictionary, when it is an indirect object; `None` for a direct one.
    ///
    /// Added in 0.6.0.
    pub object: Option<ObjRef>,
    /// The font has a `/ToUnicode` CMap (§9.10.3), which maps its codes to text.
    ///
    /// Added in 0.6.0.
    pub to_unicode: bool,
}

/// A font's `/Encoding` (§9.6.5, §9.7.5).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontEncoding {
    /// A named encoding or predefined CMap (`WinAnsiEncoding`, `Identity-H`), lossily decoded.
    Named(String),
    /// An encoding dictionary or an embedded CMap stream.
    Custom,
}

/// How a font is known once listed: its object number, or for a direct font its type and name.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FontKey {
    Object(u32),
    Direct(Option<String>, Option<String>),
}

impl Pages<'_> {
    /// Every font these pages use, once each, in the order first met.
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.2.0.
    pub fn fonts(&self) -> Result<Vec<FontResource>> {
        let mut walk = Walk::new(self.document());
        for page in self.iter() {
            walk.page(&page)?;
        }
        Ok(walk.out)
    }
}

impl Page<'_> {
    /// Every font this page uses, once each, in the order first met: its own resources', those
    /// of the Form XObjects, tiling patterns, soft masks and Type 3 fonts they reach, and its
    /// annotations' appearance streams'.
    ///
    /// # Errors
    /// As [`Pages::fonts`].
    ///
    /// Added in 0.6.0.
    pub fn fonts(&self) -> Result<Vec<FontResource>> {
        let mut walk = Walk::new(self.doc());
        walk.page(self)?;
        Ok(walk.out)
    }
}

struct Walk<'d> {
    doc: &'d Document,
    max_depth: usize,
    /// Form XObjects, Type 3 fonts and resource dictionaries already read, by object number.
    seen_containers: BTreeSet<u32>,
    /// Fonts already listed.
    seen_fonts: BTreeSet<FontKey>,
    out: Vec<FontResource>,
}

impl<'d> Walk<'d> {
    fn new(doc: &'d Document) -> Self {
        Self {
            doc,
            max_depth: doc.limits().max_tree_depth(),
            seen_containers: BTreeSet::new(),
            seen_fonts: BTreeSet::new(),
            out: Vec::new(),
        }
    }

    /// A page's resources, then its annotations' appearance streams (§12.5.5), which draw
    /// text in fonts of their own: a filled form field's value, a free text comment.
    fn page(&mut self, page: &Page<'_>) -> Result<()> {
        if let Some(resources) = page.resources() {
            self.resources(resources, 0)?;
        }
        for annot in page.annots()? {
            let Some(annot) = self.dict(Some(&Object::Ref(annot)))? else {
                continue;
            };
            let Some(appearances) = self.dict(annot.get("AP"))? else {
                continue;
            };
            for (_, value) in appearances.iter() {
                self.appearance(value)?;
            }
        }
        Ok(())
    }

    /// An appearance entry: a form XObject, or a dictionary of them by state (§12.5.5).
    fn appearance(&mut self, value: &Object) -> Result<()> {
        let Some(object) = self.doc.read(value)? else {
            return Ok(());
        };
        match &*object {
            Object::Stream(_) => self.container(value, 0, |_| true),
            Object::Dict(states) => {
                for (_, state) in states.iter() {
                    if matches!(self.doc.read(state)?.as_deref(), Some(Object::Stream(_))) {
                        self.container(state, 0, |_| true)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

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
                self.container(value, depth, |dict| dict.get_name("Subtype") == Some(b"Form"))?;
            }
        }
        // A tiling pattern's cell (§8.7.3.1) and a soft mask's group (§11.6.5.2) are content
        // with resources of their own.
        if let Some(patterns) = self.dict(resources.get("Pattern"))? {
            for (_, value) in patterns.iter() {
                self.container(value, depth, |dict| dict.get_int("PatternType") == Some(1))?;
            }
        }
        if let Some(states) = self.dict(resources.get("ExtGState"))? {
            for (_, value) in states.iter() {
                let Some(state) = self.dict(Some(value))? else { continue };
                let Some(mask) = self.dict(state.get("SMask"))? else {
                    continue;
                };
                if let Some(group) = mask.get("G") {
                    self.container(group, depth, |_| true)?;
                }
            }
        }
        Ok(())
    }

    /// Walks the resources of the stream `value`, once per object, when `wanted` takes its
    /// dictionary.
    fn container(&mut self, value: &Object, depth: usize, wanted: impl Fn(&Dict) -> bool) -> Result<()> {
        if let Some(r) = value.as_ref()
            && !self.seen_containers.insert(r.num)
        {
            return Ok(());
        }
        let Some(object) = self.doc.read(value)? else {
            return Ok(());
        };
        let Object::Stream(stream) = &*object else {
            return Ok(());
        };
        if !wanted(&stream.dict) {
            return Ok(());
        }
        if let Some(inner) = self.dict(stream.dict.get("Resources"))? {
            self.resources(&inner, depth + 1)?;
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
            Some(num) => FontKey::Object(num),
            None => FontKey::Direct(subtype.clone(), base.clone()),
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
            object: value.as_ref(),
            to_unicode: font.get("ToUnicode").is_some(),
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

    fn encoding(&self, font: &Dict) -> Result<Option<FontEncoding>> {
        let Some(value) = font.get("Encoding") else {
            return Ok(None);
        };
        Ok(match self.doc.read(value)?.as_deref() {
            Some(Object::Name(n)) => Some(FontEncoding::Named(lossy(n.as_bytes()))),
            Some(Object::Dict(_) | Object::Stream(_)) => Some(FontEncoding::Custom),
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
