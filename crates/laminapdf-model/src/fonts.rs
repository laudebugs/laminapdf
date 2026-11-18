//! The fonts a document's pages use (ISO 32000-2 §9.5–9.7), each listed once, for a
//! properties view: its name, type, whether the file embeds it and whether that is a subset.
//!
//! Fonts are found in each page's resources and, through them, in the Form XObjects the page
//! draws (§8.10), its tiling patterns (§8.7.3.1), its soft masks' groups (§11.6.5.2) and Type 3
//! fonts' own resources (§9.6.4), and on request its annotations' appearance streams
//! (§12.5.5), each container read once. The walk is bounded by [`Limits::max_tree_depth`](laminapdf_cos::Limits::max_tree_depth); what
//! can't be read is left out (ADR 0003), never fails the list.

use std::collections::BTreeSet;
use std::sync::Arc;

use laminapdf_cos::{Dict, ObjRef, Object, Result};

use crate::document::Document;
use crate::page::{Page, Pages};
use crate::values::name_bytes_text;

/// One font the document uses.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FontResource {
    /// Its name without a subset tag (`ABCDEF+`), lossily decoded: `/BaseFont`, else its font
    /// descriptor's `/FontName` (§9.8.1), else for a Type 3 font, which has no `/BaseFont`, its
    /// `/Name` (§9.6.4); `None` when the file gives none.
    pub name: Option<String>,
    /// `/Subtype`: `Type0`, `Type1`, `MMType1`, `Type3` or `TrueType`, as stored.
    pub subtype: Option<String>,
    /// The file holds the font program (`/FontFile`, `/FontFile2` or `/FontFile3` in its
    /// descriptor, a Type 0 font's descendant's), or its glyphs (Type 3).
    pub embedded: bool,
    /// Its name starts with a subset tag: six capital letters and `+` (§9.9.2).
    pub subset: bool,
    /// The encoding `/Encoding` declares; `None` when absent, or when it declares nothing
    /// beyond the font's own: a simple font's encoding dictionary without `/Differences` is its
    /// `/BaseEncoding` (§9.6.5.1), and a stream, which only a Type 0 font can take (its CMap,
    /// §9.7.5), declares nothing for a simple font.
    pub encoding: Option<FontEncoding>,
    /// The font dictionary, when it is an indirect object; `None` for a direct one.
    ///
    /// Added in 0.6.0.
    pub object: Option<ObjRef>,
    /// The font has a `/ToUnicode` CMap stream (§9.10.3), which maps its codes to text; a name
    /// there (`/Identity-H`, written by some producers) maps nothing.
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
    /// A simple font's encoding dictionary with `/Differences`, or a Type 0 font's embedded
    /// CMap stream.
    Custom,
}

/// What a simple font's `/Encoding` declares (ISO 32000-2 §9.6.5.1): the one reading the
/// inspector ([`FontResource::encoding`]) and text extraction share.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct SimpleEncoding {
    /// The base encoding's name: `/Encoding` when it is a name, else its dictionary's
    /// `/BaseEncoding`; either may be an indirect object.
    pub base: Option<Vec<u8>>,
    /// The dictionary's `/Differences`, resolved (an array, in a well-formed file).
    pub differences: Option<Arc<Object>>,
    /// `/Encoding` is there but is neither a name nor a dictionary (a stream, a number): it
    /// declares nothing, and the font's own encoding applies.
    pub malformed: bool,
}

impl SimpleEncoding {
    /// Reads `font`'s `/Encoding`, resolving each value with `resolve` (which decides what a
    /// value that can't be read becomes: `Ok(None)`, or an error that ends the read).
    ///
    /// # Errors
    /// What `resolve` returns.
    ///
    /// Added in 0.6.0.
    pub fn read(font: &Dict, mut resolve: impl FnMut(&Object) -> Result<Option<Arc<Object>>>) -> Result<Self> {
        let Some(value) = font.get("Encoding") else {
            return Ok(Self::default());
        };
        let Some(encoding) = resolve(value)? else {
            return Ok(Self::default());
        };
        Ok(match &*encoding {
            Object::Name(n) => Self {
                base: Some(n.as_bytes().to_vec()),
                ..Self::default()
            },
            Object::Dict(d) => {
                let base = match d.get("BaseEncoding") {
                    Some(base) => match resolve(base)?.as_deref() {
                        Some(Object::Name(n)) => Some(n.as_bytes().to_vec()),
                        _ => None,
                    },
                    None => None,
                };
                let differences = match d.get("Differences") {
                    Some(list) => resolve(list)?,
                    None => None,
                };
                Self {
                    base,
                    differences,
                    malformed: false,
                }
            }
            _ => Self {
                malformed: true,
                ..Self::default()
            },
        })
    }
}

/// A font's `/ToUnicode` CMap (ISO 32000-2 §9.10.3), resolved with `resolve`: only a stream is
/// one, and a name there (`/Identity-H`, written by some producers) maps nothing. The one
/// reading the inspector ([`FontResource::to_unicode`]) and text extraction share.
///
/// # Errors
/// What `resolve` returns.
///
/// Added in 0.6.0.
pub fn to_unicode_cmap(
    font: &Dict,
    mut resolve: impl FnMut(&Object) -> Result<Option<Arc<Object>>>,
) -> Result<Option<Arc<Object>>> {
    let Some(value) = font.get("ToUnicode") else {
        return Ok(None);
    };
    Ok(resolve(value)?.filter(|cmap| matches!(**cmap, Object::Stream(_))))
}

/// How a font is known once listed: its object number, or for a direct font its type and name.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FontKey {
    Object(u32),
    Direct(Option<String>, Option<String>),
}

/// Which fonts a walk lists ([`Pages::fonts_in`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontScope {
    /// The fonts the page content draws with: its resources and what they reach.
    Content,
    /// Those, and the fonts its annotations' appearance streams use (a filled field's value, a
    /// signature's text), as `pdffonts` lists them.
    WithAppearances,
}

impl Pages<'_> {
    /// Every font these pages' content uses, once each, in the order first met
    /// ([`FontScope::Content`]).
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.2.0.
    pub fn fonts(&self) -> Result<Vec<FontResource>> {
        self.fonts_in(FontScope::Content)
    }

    /// Every font these pages use within `scope`, once each, in the order first met.
    ///
    /// # Errors
    /// As [`Pages::fonts`].
    ///
    /// Added in 0.6.0.
    pub fn fonts_in(&self, scope: FontScope) -> Result<Vec<FontResource>> {
        let mut walk = Walk::new(self.document());
        for page in self.iter() {
            walk.page(&page, scope)?;
        }
        Ok(walk.out)
    }
}

impl Page<'_> {
    /// Every font this page's content uses, once each, in the order first met: its own
    /// resources', and those of the Form XObjects, tiling patterns, soft masks and Type 3 fonts
    /// they reach ([`FontScope::Content`]).
    ///
    /// # Errors
    /// As [`Pages::fonts`].
    ///
    /// Added in 0.6.0.
    pub fn fonts(&self) -> Result<Vec<FontResource>> {
        self.fonts_in(FontScope::Content)
    }

    /// Every font this page uses within `scope`, once each, in the order first met.
    ///
    /// # Errors
    /// As [`Pages::fonts`].
    ///
    /// Added in 0.6.0.
    pub fn fonts_in(&self, scope: FontScope) -> Result<Vec<FontResource>> {
        let mut walk = Walk::new(self.doc());
        walk.page(self, scope)?;
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

    /// A page's resources, then, with [`FontScope::WithAppearances`], its annotations'
    /// appearance streams (§12.5.5), which draw text in fonts of their own: a filled form
    /// field's value, a free text comment.
    fn page(&mut self, page: &Page<'_>, scope: FontScope) -> Result<()> {
        if let Some(resources) = page.resources() {
            self.resources(resources, 0)?;
        }
        if scope == FontScope::Content {
            return Ok(());
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
        // A font met before through its object is listed once, before anything else is read.
        if let Some(r) = value.as_ref()
            && self.seen_fonts.contains(&FontKey::Object(r.num))
        {
            return Ok(());
        }
        let Some(object) = self.doc.read(value)? else {
            return Ok(());
        };
        let Some(font) = object.as_dict() else { return Ok(()) };
        let subtype = font.get_name("Subtype").map(name_bytes_text);
        let type3 = subtype.as_deref() == Some("Type3");
        let composite = subtype.as_deref() == Some("Type0");
        // A Type 0 font's descriptor is its descendant's (§9.7.6.1).
        let descriptor_font = if composite { self.descendant(font)? } else { None };
        let descriptor = self.dict(descriptor_font.as_ref().unwrap_or(font).get("FontDescriptor"))?;
        let full_name = self.full_name(font, descriptor.as_ref(), type3)?;
        let key = match value.as_ref() {
            Some(r) => FontKey::Object(r.num),
            None => FontKey::Direct(subtype.clone(), full_name.clone()),
        };
        if !self.seen_fonts.insert(key) {
            return Ok(());
        }
        let subset = full_name.as_deref().is_some_and(has_subset_tag);
        let name = full_name.map(|b| {
            if subset {
                b.get(7..).unwrap_or_default().to_owned()
            } else {
                b
            }
        });
        let embedded = type3 || embeds(descriptor.as_ref());
        let encoding = self.encoding(font, composite)?;
        let to_unicode = to_unicode_cmap(font, |o| self.doc.read(o))?.is_some();
        self.out.push(FontResource {
            name,
            subtype,
            embedded,
            subset,
            encoding,
            object: value.as_ref(),
            to_unicode,
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

    /// The font's name with its subset tag: `/BaseFont`, else its descriptor's `/FontName`
    /// (§9.8.1, which a Type 3 font may give, §9.6.4), else a Type 3 font's `/Name`; each may be
    /// an indirect object (§7.3.10).
    fn full_name(&self, font: &Dict, descriptor: Option<&Dict>, type3: bool) -> Result<Option<String>> {
        if let Some(name) = self.name(font.get("BaseFont"))? {
            return Ok(Some(name));
        }
        if let Some(name) = self.name(descriptor.and_then(|d| d.get("FontName")))? {
            return Ok(Some(name));
        }
        if type3 {
            return self.name(font.get("Name"));
        }
        Ok(None)
    }

    /// The name `value` holds, resolved and lossily decoded.
    fn name(&self, value: Option<&Object>) -> Result<Option<String>> {
        let Some(value) = value else { return Ok(None) };
        Ok(match self.doc.read(value)?.as_deref() {
            Some(Object::Name(n)) => Some(name_bytes_text(n.as_bytes())),
            _ => None,
        })
    }

    /// What `/Encoding` declares ([`FontResource::encoding`]): for a Type 0 font (`composite`)
    /// a name or a CMap stream; for a simple font [`SimpleEncoding`]'s `/Differences`, else its
    /// base encoding.
    fn encoding(&self, font: &Dict, composite: bool) -> Result<Option<FontEncoding>> {
        if composite {
            let Some(value) = font.get("Encoding") else {
                return Ok(None);
            };
            return Ok(match self.doc.read(value)?.as_deref() {
                Some(Object::Name(n)) => Some(FontEncoding::Named(name_bytes_text(n.as_bytes()))),
                Some(Object::Stream(_)) => Some(FontEncoding::Custom),
                _ => None,
            });
        }
        let declared = SimpleEncoding::read(font, |o| self.doc.read(o))?;
        Ok(if matches!(declared.differences.as_deref(), Some(Object::Array(_))) {
            Some(FontEncoding::Custom)
        } else {
            declared.base.map(|b| FontEncoding::Named(name_bytes_text(&b)))
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

/// Whether a font descriptor holds a font program (§9.9).
fn embeds(descriptor: Option<&Dict>) -> bool {
    descriptor.is_some_and(|d| {
        ["FontFile", "FontFile2", "FontFile3"]
            .iter()
            .any(|key| d.get(key).is_some())
    })
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
