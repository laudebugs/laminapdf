//! The fonts an edit may draw new text with, besides the one the text is set in (ADR 0022 §6;
//! E7 plan §2.2): the embedded font first, then a new subset of the same font when the caller
//! holds the whole font, then a substitute. The caller lists what may be used (`fonts`), so a
//! font is never changed without it having asked: a character none of them draws is
//! `missing-glyph`.

use laminapdf_content::ShownGlyph;
use laminapdf_text::{EmbedPolicy, Font, FontEncoder, FontKey, FontRegistry, Standard14};

use super::text;
use crate::text_edit::{Fidelity, SubstituteKind, TextSubstitution};
use crate::annot::{AnnotFont, StandardFamily};
use crate::blob::BlobStore;
use crate::error::{Error, Result};

/// A font an edit may draw with.
struct Candidate {
    key: FontKey,
    font: Font,
    name: String,
    kind: SubstituteKind,
    used: bool,
}

/// The fonts an edit may draw with, and the registry that encodes text in them.
pub(super) struct Chain {
    registry: FontRegistry,
    fonts: Vec<Candidate>,
}

/// Where a stretch of new text is drawn: in the run's font, or in a font of the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    Own,
    Chain(usize),
}

/// A stretch of new text and where it is drawn.
pub(super) struct Stretch {
    pub(super) text: String,
    pub(super) source: Source,
}

/// A font's name without the subset tag (`ABCDEF+Name`), in lower case without punctuation.
fn plain(name: &str) -> String {
    let base = match name.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()) => rest,
        _ => name,
    };
    base.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The standard 14 face of `family` for a font named `name`: bold and italic when the name says so.
pub(super) fn face_like(family: StandardFamily, name: &str) -> Standard14 {
    let lower = name.to_ascii_lowercase();
    let bold = lower.contains("bold");
    let italic = lower.contains("italic") || lower.contains("oblique");
    standard(family, bold, italic)
}

/// The standard 14 face of `family`.
pub(super) fn standard(family: StandardFamily, bold: bool, italic: bool) -> Standard14 {
    Standard14::from_base_font(family.face(bold, italic)).unwrap_or(Standard14::Helvetica)
}

/// A font given to an op, loaded: `own` is the run's font name, to tell a new subset of it from
/// a substitute, and `style` the bold and italic a standard family follows.
fn load(font: &AnnotFont, blobs: &BlobStore, own: &str) -> Result<(Font, SubstituteKind)> {
    match font {
        AnnotFont::Standard(family) => {
            let face = face_like(*family, own);
            Ok((Font::standard(face), SubstituteKind::Substitute))
        }
        AnnotFont::Font(id) => {
            let bytes = blobs.get(id).ok_or(Error::UnknownBlob(*id))?;
            let font = Font::from_bytes(bytes.clone()).map_err(Error::Text)?;
            let same = plain(&font.info().postscript_name) == plain(own);
            Ok((
                font,
                if same {
                    SubstituteKind::Subset
                } else {
                    SubstituteKind::Substitute
                },
            ))
        }
    }
}

impl Chain {
    /// The fonts `fonts` name, in order, loaded from `blobs`; `own` is the run's `/BaseFont`.
    ///
    /// # Errors
    /// `unknown-blob`; `font-invalid` and `font-not-embeddable` as the text layer raises them.
    pub(super) fn new(fonts: &[AnnotFont], blobs: &BlobStore, own: &str) -> Result<Self> {
        let mut registry = FontRegistry::new();
        let mut loaded = Vec::with_capacity(fonts.len());
        for font in fonts {
            let (font, kind) = load(font, blobs, own)?;
            let name = font.info().postscript_name.clone();
            let key = registry.add(font.clone(), EmbedPolicy::Respect).map_err(Error::Text)?;
            loaded.push(Candidate {
                key,
                font,
                name,
                kind,
                used: false,
            });
        }
        Ok(Self {
            registry,
            fonts: loaded,
        })
    }

    /// A chain of one font that draws all of a run's new text (a font change).
    ///
    /// # Errors
    /// As [`Chain::new`].
    pub(super) fn only(font: Font) -> Result<Self> {
        let mut registry = FontRegistry::new();
        let name = font.info().postscript_name.clone();
        let key = registry.add(font.clone(), EmbedPolicy::Respect).map_err(Error::Text)?;
        Ok(Self {
            registry,
            fonts: vec![Candidate {
                key,
                font,
                name,
                kind: SubstituteKind::Substitute,
                used: false,
            }],
        })
    }

    /// The new text cut into stretches by the font that draws each: a character the run's font
    /// draws (`own`) stays in it; any other goes to the first font of the chain with a glyph.
    /// A space follows the stretch before it.
    ///
    /// # Errors
    /// `missing-glyph` for the first character no font draws.
    pub(super) fn cut(&self, own: Option<&FontEncoder>, text: &str, own_name: &str) -> Result<Vec<Stretch>> {
        let mut out: Vec<Stretch> = Vec::new();
        for ch in text.chars() {
            let source = if ch == ' ' {
                out.last().map_or(Source::Own, |s| s.source)
            } else if own.is_some_and(|e| e.encode(&ch.to_string()).is_ok()) {
                Source::Own
            } else if let Some(at) = self.fonts.iter().position(|c| c.font.glyph_for(ch).is_some()) {
                Source::Chain(at)
            } else {
                return Err(Error::Text(laminapdf_text::Error::MissingGlyph {
                    ch,
                    font: own_name.to_owned(),
                }));
            };
            match out.last_mut() {
                Some(last) if last.source == source => last.text.push(ch),
                _ => out.push(Stretch {
                    text: ch.to_string(),
                    source,
                }),
            }
        }
        Ok(out)
    }

    /// What the stretches say about the edit: each stretch in a font of the chain, named.
    pub(super) fn substitutions(&self, stretches: &[Stretch]) -> Vec<TextSubstitution> {
        stretches
            .iter()
            .filter_map(|s| match s.source {
                Source::Chain(at) => self.fonts.get(at).map(|c| TextSubstitution {
                    text: s.text.clone(),
                    font: c.name.clone(),
                    kind: c.kind,
                }),
                Source::Own => None,
            })
            .collect()
    }

    /// `text` in font `at` of the chain, a glyph per character, with the codes the registry
    /// gives and the font's widths; marks the font used.
    ///
    /// # Errors
    /// `missing-glyph`.
    pub(super) fn encode(&mut self, at: usize, text: &str) -> Result<Vec<ShownGlyph>> {
        let Some(candidate) = self.fonts.get_mut(at) else {
            return Err(Error::InvalidOp("the font isn't in the chain"));
        };
        candidate.used = true;
        let mut out = Vec::new();
        for ch in text.chars() {
            let encoded = self
                .registry
                .encode(candidate.key, &ch.to_string())
                .map_err(Error::Text)?;
            let code = encoded.bytes.iter().fold(0_u32, |all, &b| (all << 8) | u32::from(b));
            let len = u8::try_from(encoded.bytes.len()).unwrap_or(1);
            out.push(ShownGlyph::new(code, len, encoded.advance, len == 1 && code == 32));
        }
        Ok(out)
    }

    /// The font objects of the fonts used, reserved in `doc`, each with its chain index.
    ///
    /// # Errors
    /// As the text layer reserves them.
    pub(super) fn reserve(&mut self, doc: &laminapdf_cos::Document) -> Result<Vec<(usize, laminapdf_syntax::ObjRef)>> {
        let mut out = Vec::new();
        for (at, candidate) in self.fonts.iter().enumerate() {
            if candidate.used {
                out.push((at, self.registry.reference(candidate.key, doc).map_err(Error::Text)?));
            }
        }
        Ok(out)
    }

    /// Adds each font used to the page's resources, as the name the page's content draws it by.
    ///
    /// # Errors
    /// As the text layer reserves the fonts and the writer adds them.
    pub(super) fn names(
        &mut self,
        writer: &mut laminapdf_content::PageContentWriter<'_>,
    ) -> Result<std::collections::BTreeMap<usize, laminapdf_syntax::Name>> {
        let reserved = self.reserve(writer.cos().document())?;
        let mut out = std::collections::BTreeMap::new();
        for (at, object) in reserved {
            out.insert(at, writer.add_font(object)?);
        }
        Ok(out)
    }

    /// Writes the fonts used into the document.
    ///
    /// # Errors
    /// As the text layer writes them.
    pub(super) fn write(self, cos: &mut laminapdf_cos::CosMut<'_>) -> Result<()> {
        self.registry.finish_fonts(cos).map_err(Error::Text)?;
        Ok(())
    }
}

/// How fidelity reads: substituted when any character is in another font, reflowed when the
/// line's width changed, exact otherwise.
pub(super) fn fidelity(substitutions: &[TextSubstitution], shift: f64) -> Fidelity {
    if substitutions.iter().any(|s| s.kind == SubstituteKind::Substitute) {
        Fidelity::Substituted
    } else if shift.abs() > text::WIDTH_EPSILON {
        Fidelity::Reflowed
    } else {
        Fidelity::Exact
    }
}
