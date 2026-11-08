//! The rules a profile checks when the build finishes, before anything is written: the first
//! one broken is the error.
//!
//! Each rule is one of the machine-checkable conditions Lamina PDF's builder could break; the
//! others it can't produce (encryption, JavaScript or launch actions, transparency without an
//! output intent, unassociated embedded files) need no check here. The references are the
//! Matterhorn Protocol 1.1 failure conditions for PDF/UA-1 and the ISO 19005-2 clauses (as
//! veraPDF's rule IDs number them) for PDF/A.

use std::collections::BTreeMap;

use crate::annot::link::PendingLink;
use crate::attach::Attachment;
use crate::error::{Error, Result};
use crate::profile::{Conformance, PdfA};
use crate::tag::elem::ElemInfo;
use crate::tag::rolemap::heading_level;
use crate::tag::{ContentKind, TagDiagnostic};

/// What the checks read from the build.
pub(crate) struct Inputs<'a> {
    pub(crate) conformance: Conformance,
    /// The header version asked for.
    pub(crate) version: (u8, u8),
    pub(crate) title: Option<&'a str>,
    pub(crate) lang: Option<&'a str>,
    pub(crate) diagnostics: &'a [TagDiagnostic],
    /// Every structure element, in structure order.
    pub(crate) elements: &'a [ElemInfo],
    pub(crate) links: &'a [PendingLink],
    /// The first page drawing with `DeviceCMYK`.
    pub(crate) device_cmyk: Option<u32>,
    /// The attached files, by name.
    pub(crate) attachments: &'a BTreeMap<String, Attachment>,
}

/// Runs every rule of the build's profiles (fonts are checked by [`fonts`]).
pub(crate) fn check(inputs: &Inputs<'_>) -> Result<()> {
    let c = inputs.conformance;
    if inputs.version > (1, 7) {
        // PDF/UA-1 and PDF/A-2/3 are built on ISO 32000-1 (PDF 1.7); veraPDF 6.1.2-1 checks
        // the header.
        if let Some(profile) = c.primary() {
            return Err(Error::ProfileVersionConflict { profile });
        }
    }
    if c.ua {
        ua1(inputs)?;
    }
    if let Some(pdfa) = c.pdfa {
        pdfa_rules(pdfa, inputs)?;
    }
    Ok(())
}

/// Whether a text is empty or only white space (no usable title, language or description).
fn blank(text: Option<&str>) -> bool {
    text.is_none_or(|t| t.trim().is_empty())
}

fn ua1(inputs: &Inputs<'_>) -> Result<()> {
    // Matterhorn 06-003 (no dc:title in the XMP; the builder writes 07-001's DisplayDocTitle
    // and 06-001/06-002's packet and identifier itself).
    if blank(inputs.title) {
        return Err(Error::MissingTitle);
    }
    // Matterhorn 11-006: the document's natural language can't be determined.
    if blank(inputs.lang) {
        return Err(Error::MissingLang);
    }
    // The first diagnostic is the error (they come in page order).
    if let Some(d) = inputs.diagnostics.first() {
        return Err(match *d {
            // Matterhorn 28-011 (ISO 14289-1 §7.18.5-1): a link not nested in a Link tag.
            TagDiagnostic::Untagged {
                page,
                content: ContentKind::Link,
                count,
            } => Error::LinkWithoutStructure { page, count },
            // Matterhorn 01-005 (ISO 14289-1 §7.1-3): content neither an artifact nor tagged.
            TagDiagnostic::Untagged { page, content, count } => Error::UntaggedContent { page, content, count },
            // Matterhorn 13-004: a Figure without alternate text.
            TagDiagnostic::MissingAlt { page } => Error::MissingAltText { page },
        });
    }
    headings(inputs.elements)?;
    nesting(inputs.elements)?;
    // Matterhorn 28-012 (ISO 14289-1 §7.18.5-2): a link without an alternate description.
    if let Some(link) = inputs.links.iter().find(|l| !has_contents(l)) {
        return Err(Error::LinkWithoutContents { page: link.page });
    }
    Ok(())
}

/// The heading rules of ISO 14289-1 §7.4: the first numbered heading is `H1` and no level is
/// skipped going down (Matterhorn 14-002, 14-003); an unnumbered `H` is never beside another
/// `H` in one parent (14-006), nor in a document with numbered headings (14-007).
fn headings(elements: &[ElemInfo]) -> Result<()> {
    let mut previous = 0;
    // The parents of the `H`s so far (`None`: the `Document` element).
    let mut unnumbered: Vec<Option<usize>> = Vec::new();
    for e in elements {
        if e.kind == "H" {
            if previous > 0 || unnumbered.contains(&e.parent_index) {
                return Err(Error::UnnumberedHeadingMisused { page: e.page });
            }
            unnumbered.push(e.parent_index);
            continue;
        }
        let Some(level) = heading_level(&e.kind) else { continue };
        if !unnumbered.is_empty() {
            return Err(Error::UnnumberedHeadingMisused { page: e.page });
        }
        if level > previous + 1 {
            return Err(Error::HeadingLevelSkipped {
                page: e.page,
                level,
                previous,
            });
        }
        previous = level;
    }
    Ok(())
}

/// Each table and list container and the children it may have (ISO 32000-1 Tables 336 and
/// 337; Matterhorn 09-004 and 09-005; veraPDF 7.2-3 to 7.2-20).
const CHILDREN: [(&str, &[&str]); 7] = [
    ("Table", &["TR", "THead", "TBody", "TFoot", "Caption"]),
    ("THead", &["TR"]),
    ("TBody", &["TR"]),
    ("TFoot", &["TR"]),
    ("TR", &["TH", "TD"]),
    ("L", &["L", "LI", "Caption"]),
    ("LI", &["Lbl", "LBody"]),
];

/// The children in [`CHILDREN`] that may also stand anywhere else: a `Caption` belongs to a
/// figure too, a list in any block, a `Lbl` in a caption.
const ANYWHERE: [&str; 3] = ["Caption", "L", "Lbl"];

/// Whether `kind` may be a child of `parent`: a container takes only its children, and a
/// container's child (other than [`ANYWHERE`]'s) stands only in a container.
fn nests(kind: &str, parent: &str) -> bool {
    if let Some((_, kids)) = CHILDREN.iter().find(|(p, _)| *p == parent) {
        return kids.contains(&kind);
    }
    ANYWHERE.contains(&kind) || !CHILDREN.iter().any(|(_, kids)| kids.contains(&kind))
}

fn nesting(elements: &[ElemInfo]) -> Result<()> {
    for e in elements {
        let parent = e.parent.as_deref().unwrap_or("Document");
        if !nests(&e.kind, parent) {
            return Err(Error::InvalidStructureNesting {
                page: e.page,
                kind: e.kind.clone(),
                parent: parent.to_owned(),
            });
        }
    }
    Ok(())
}

fn has_contents(link: &PendingLink) -> bool {
    match link.dict.get("Contents") {
        Some(laminapdf_cos::Object::String(s)) => !s.bytes.is_empty(),
        _ => false,
    }
}

fn pdfa_rules(pdfa: PdfA, inputs: &Inputs<'_>) -> Result<()> {
    // ISO 19005-2 §6.2.4.3 (veraPDF 6.2.4.3-4): DeviceCMYK only under a CMYK output intent.
    if let Some(page) = inputs.device_cmyk {
        return Err(Error::DeviceColorWithoutOutputIntent { page });
    }
    // ISO 19005-2 §6.8 (veraPDF 6.8-5): embedded files must themselves be PDF/A, which
    // lamina can't check, so PDF/A-2 takes none.
    if !inputs.attachments.is_empty() && pdfa != PdfA::A3b {
        return Err(Error::EmbeddedFileForbidden {
            profile: pdfa.profile(),
        });
    }
    // ISO 19005-3 §6.8: every embedded file has a MIME type (`/Subtype`).
    if let Some(file) = inputs.attachments.values().find(|f| f.mime().is_empty()) {
        return Err(Error::EmbeddedFileWithoutMimeType {
            profile: pdfa.profile(),
            name: file.name().to_owned(),
        });
    }
    Ok(())
}

/// The font rules: every font embedded (PDF/A-2 §6.2.11.4.1, PDF/UA-1 §7.21.4.1, Matterhorn
/// 31-001), and under PDF/A-2u and PDF/UA-1 every glyph drawn with a Unicode value
/// (PDF/A-2 §6.2.11.7.2, PDF/UA-1 §7.21.7, Matterhorn 31-027) that isn't U+0000, U+FEFF or
/// U+FFFE.
///
/// The empty value counts. Shaping can draw one character as several glyphs (Noto Sans Arabic
/// draws the beh as a dotless body and a dot), and the registry maps the glyph that carries no
/// character of its own to `<>` while another glyph of its cluster carries the cluster's text, so
/// each character is extracted once. veraPDF 1.30.3 passes such files under PDF/UA-1 and
/// PDF/A-2u, and pdf.js reads their text back exactly; giving the dot the letter's text too
/// would make every extractor that ignores `/ActualText`, pdf.js among them, read the letter
/// twice.
#[cfg(feature = "text")]
pub(crate) fn fonts(
    conformance: Conformance,
    registry: &laminapdf_text::FontRegistry,
    keys: &[laminapdf_text::FontKey],
) -> Result<()> {
    use laminapdf_text::FontUsage;

    let Some(profile) = conformance.primary() else {
        return Ok(());
    };
    let unicode = conformance.unicode_profile();
    for &key in keys {
        let Ok(usage) = registry.usage(key) else { continue };
        let font = || {
            registry
                .font(key)
                .map(|f| f.info().postscript_name.clone())
                .unwrap_or_default()
        };
        match usage {
            FontUsage::Simple(simple) if !simple.text().is_empty() => {
                return Err(Error::FontNotEmbedded { profile, font: font() });
            }
            FontUsage::Embedded(used) => {
                let Some(profile) = unicode else { continue };
                let text = used.text();
                let missing = used.glyphs().into_iter().find(|&(cid, _)| {
                    cid != 0
                        && text
                            .get(&cid)
                            .is_none_or(|t| t.contains(['\u{0}', '\u{FEFF}', '\u{FFFE}']))
                });
                if let Some((_, gid)) = missing {
                    return Err(Error::GlyphWithoutUnicode {
                        profile,
                        font: font(),
                        gid: gid.get(),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "text")]
mod tests {
    use laminapdf_text::{EmbedPolicy, Font, FontRegistry, GlyphId};

    use super::*;
    use crate::Profile;

    const NOTO_SANS: &[u8] = include_bytes!("../../../../fixtures/fonts/NotoSans-Regular.ttf");

    /// A registry whose one font drew glyph 36 standing for `text`.
    fn registry_with(text: &str) -> (FontRegistry, laminapdf_text::FontKey) {
        let font = Font::from_bytes(NOTO_SANS.to_vec()).unwrap();
        let mut registry = FontRegistry::new();
        let key = registry.add(font, EmbedPolicy::Respect).unwrap();
        registry.use_glyph(key, GlyphId::new(36), text).unwrap();
        (registry, key)
    }

    fn checked(text: &str) -> Result<()> {
        let (registry, key) = registry_with(text);
        fonts(Conformance::default().with(Profile::PdfUa1), &registry, &[key])
    }

    #[test]
    fn a_glyph_with_text_passes() {
        assert_eq!(checked("A"), Ok(()));
    }

    #[test]
    fn a_glyph_whose_cluster_text_another_glyph_carries_passes() {
        assert_eq!(checked(""), Ok(()));
    }

    #[test]
    fn a_glyph_mapped_to_a_noncharacter_fails() {
        for text in ["\u{0}", "\u{FEFF}", "\u{FFFE}"] {
            assert!(
                matches!(checked(text), Err(Error::GlyphWithoutUnicode { gid: 36, .. })),
                "{text:?}"
            );
        }
    }
}
