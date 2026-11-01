//! The rules a profile checks when the build finishes, before anything is written: the first
//! one broken is the error.
//!
//! Each rule is one of the machine-checkable conditions lamina's builder could break; the
//! others it can't produce (encryption, JavaScript or launch actions, transparency without an
//! output intent, unassociated embedded files) need no check here. The references are the
//! Matterhorn Protocol 1.1 failure conditions for PDF/UA-1 and the ISO 19005-2 clauses (as
//! veraPDF's rule IDs number them) for PDF/A.

use crate::annot::link::PendingLink;
use crate::error::{Error, Result};
use crate::profile::{Conformance, PdfA, Profile};
use crate::tag::elem::ElemInfo;
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
    pub(crate) attachments: usize,
}

/// Runs every rule of the build's profiles (fonts are checked by [`fonts`]).
pub(crate) fn check(inputs: &Inputs<'_>) -> Result<()> {
    let c = inputs.conformance;
    if inputs.version > (1, 7) {
        // PDF/UA-1 and PDF/A-2/3 are built on ISO 32000-1 (PDF 1.7); veraPDF 6.1.2-1 checks
        // the header.
        if let Some(profile) = c.pdfa.map(PdfA::profile).or(c.ua.then_some(Profile::PdfUa1)) {
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

/// The level of a numbered heading (`H1` → 1), `None` for anything else.
fn heading_level(kind: &str) -> Option<u32> {
    let digits = kind.strip_prefix('H')?;
    if digits.is_empty() || digits.starts_with('0') {
        return None;
    }
    digits.parse().ok()
}

/// Matterhorn 14-002 and 14-003 (ISO 14289-1 §7.4.2): the first numbered heading is `H1`, and
/// no level is skipped going down.
fn headings(elements: &[ElemInfo]) -> Result<()> {
    let mut previous = 0;
    for e in elements {
        let Some(level) = heading_level(&e.kind) else { continue };
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

/// The parents each table and list element may have, and the children each table and list
/// container may have (ISO 32000-1 Tables 336 and 337; Matterhorn 09-004 and 09-005; veraPDF
/// 7.2-3 to 7.2-20).
fn nesting(elements: &[ElemInfo]) -> Result<()> {
    const ROW_GROUPS: [&str; 4] = ["Table", "THead", "TBody", "TFoot"];
    for e in elements {
        let parent = e.parent.as_deref().unwrap_or("Document");
        let child_ok = match e.kind.as_str() {
            "TR" => ROW_GROUPS.contains(&parent),
            "TH" | "TD" => parent == "TR",
            "THead" | "TBody" | "TFoot" => parent == "Table",
            "LI" => parent == "L",
            "LBody" => parent == "LI",
            _ => true,
        };
        let kind = e.kind.as_str();
        let parent_ok = match parent {
            "Table" => matches!(kind, "TR" | "THead" | "TBody" | "TFoot" | "Caption"),
            "THead" | "TBody" | "TFoot" => kind == "TR",
            "TR" => matches!(kind, "TH" | "TD"),
            "L" => matches!(kind, "L" | "LI" | "Caption"),
            "LI" => matches!(kind, "Lbl" | "LBody"),
            _ => true,
        };
        if !(child_ok && parent_ok) {
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
        Some(lamina_cos::Object::String(s)) => !s.bytes.is_empty(),
        _ => false,
    }
}

fn pdfa_rules(pdfa: PdfA, inputs: &Inputs<'_>) -> Result<()> {
    // ISO 19005-2 §6.2.4.3 (veraPDF 6.2.4.3-4): DeviceCMYK only under a CMYK output intent.
    if let Some(page) = inputs.device_cmyk {
        return Err(Error::DeviceColorWithoutOutputIntent { page });
    }
    // ISO 19005-2 §6.8 (veraPDF 6.8-5): embedded files must themselves be PDF/A.
    if inputs.attachments > 0 && pdfa != PdfA::A3b {
        return Err(Error::EmbeddedFileForbidden {
            profile: pdfa.profile(),
        });
    }
    Ok(())
}

/// The font rules: every font embedded (PDF/A-2 §6.2.11.4.1, PDF/UA-1 §7.21.4.1, Matterhorn
/// 31-001), and under PDF/A-2u and PDF/UA-1 every glyph drawn with a Unicode value
/// (PDF/A-2 §6.2.11.7.2, PDF/UA-1 §7.21.7, Matterhorn 31-027) that isn't U+0000, U+FEFF or
/// U+FFFE.
#[cfg(feature = "text")]
pub(crate) fn fonts(
    conformance: Conformance,
    registry: &lamina_text::FontRegistry,
    keys: &[lamina_text::FontKey],
) -> Result<()> {
    use lamina_text::FontUsage;

    // PDF/A is named first: it is the stricter of the two about fonts.
    let Some(profile) = conformance
        .pdfa
        .map(PdfA::profile)
        .or(conformance.ua.then_some(Profile::PdfUa1))
    else {
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
                            .is_none_or(|t| t.is_empty() || t.contains(['\u{0}', '\u{FEFF}', '\u{FFFE}']))
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
