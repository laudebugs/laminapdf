//! [`Fonts`]: the system fonts PDFium gets inside the sandbox (ADR 0008 §5, fonts). The main
//! process sends the list of faces it can serve (`ToEngine::Fonts`) right after `Ready`; when a PDF
//! names a font it doesn't embed, PDFium asks [`Fonts::find`], which picks a face the way PDFium's
//! own folder scan would and fetches its bytes with a `FontRequest`. The engine never names a
//! path, and never opens a file.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use laminapdf_desktop_protocol::{Charsets, FontFace, FromEngine};
use laminapdf_render_pdfium::{FontCharset, FontQuery, SystemFont, SystemFontSource};

use crate::remote::Ranges;

/// Faces kept in memory: a CJK face is 10–20 MB, so a few cover a document's fonts.
const CACHED: usize = 4;

/// The base-14 names and the system faces that stand in for them, as PDFium's own folder font
/// info substitutes them (`kBase14Substs`), plus the common metric clones on Linux.
const ALIASES: &[(&str, &[&str])] = &[
    (
        "helvetica",
        &["helvetica", "arial", "liberationsans", "nimbussans", "nimbussansl"],
    ),
    ("arial", &["arial", "helvetica", "liberationsans", "nimbussans"]),
    (
        "times",
        &[
            "times",
            "timesnewroman",
            "liberationserif",
            "nimbusroman",
            "nimbusromanno9l",
        ],
    ),
    (
        "timesroman",
        &["times", "timesnewroman", "liberationserif", "nimbusroman"],
    ),
    ("timesnewroman", &["timesnewroman", "times", "liberationserif"]),
    (
        "courier",
        &["courier", "couriernew", "liberationmono", "nimbusmonops", "nimbusmono"],
    ),
    ("couriernew", &["couriernew", "courier", "liberationmono"]),
];

/// The engine's system fonts: the main process's list and a small cache of faces.
#[derive(Default)]
pub struct Fonts {
    faces: OnceLock<Vec<FontFace>>,
    ranges: OnceLock<Arc<Ranges>>,
    cache: Mutex<VecDeque<(u32, Arc<[u8]>)>>,
}

impl Fonts {
    /// The main process's list; only the first counts.
    pub(crate) fn set_faces(&self, faces: Vec<FontFace>) {
        let _ = self.faces.set(faces);
    }

    /// Where requests for bytes go, once the engine serves.
    pub(crate) fn connect(&self, ranges: Arc<Ranges>) {
        let _ = self.ranges.set(ranges);
    }

    fn lock(&self) -> MutexGuard<'_, VecDeque<(u32, Arc<[u8]>)>> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Face `id`'s bytes, from the cache or the main process.
    fn bytes(&self, id: u32) -> Option<Arc<[u8]>> {
        if let Some(bytes) = self
            .lock()
            .iter()
            .find(|(cached, _)| *cached == id)
            .map(|(_, b)| Arc::clone(b))
        {
            return Some(bytes);
        }
        let ranges = self.ranges.get()?;
        let bytes: Arc<[u8]> = match ranges.ask(|req| FromEngine::FontRequest { req, font: id }) {
            Ok(bytes) => bytes.into(),
            Err(why) => {
                crate::log(&format!("font face {id} unavailable: {why}"));
                return None;
            }
        };
        let mut cache = self.lock();
        cache.push_front((id, Arc::clone(&bytes)));
        cache.truncate(CACHED);
        Some(bytes)
    }
}

/// `Fonts` shared with PDFium's font provider.
pub struct FontSource(pub Arc<Fonts>);

impl SystemFontSource for FontSource {
    fn find(&self, query: &FontQuery<'_>) -> Option<SystemFont> {
        let face = best(self.0.faces.get()?, query)?;
        let bytes = self.0.bytes(face.id)?;
        Some(SystemFont {
            id: u64::from(face.id) + 1,
            face: face.family.clone(),
            data: bytes.to_vec(),
        })
    }
}

/// The face that best answers `query`: one whose family the PDF names (or a stand-in for a
/// base-14 name), closest in weight and style; failing that, for a CJK or symbol script, any face
/// covering it. Latin text with no match falls back to PDFium's built-in fonts.
fn best<'a>(faces: &'a [FontFace], query: &FontQuery<'_>) -> Option<&'a FontFace> {
    let wanted = normalise(query.face);
    let names: Vec<&str> = ALIASES
        .iter()
        .find(|(name, _)| *name == wanted)
        .map_or_else(|| vec![wanted.as_str()], |(_, names)| names.to_vec());
    let script = charsets(query.charset);
    let score = |face: &FontFace| {
        let weight = i64::from(face.weight).abs_diff(i64::from(query.weight));
        let style = if face.italic == query.italic { 0 } else { 1000 };
        weight + style
    };
    for name in &names {
        let found = faces
            .iter()
            .filter(|face| normalise(&face.family) == *name)
            .filter(|face| script.is_none_or(|set| face.charsets.contains(set)))
            .min_by_key(|face| score(face));
        if found.is_some() {
            return found;
        }
    }
    let script = script.filter(|set| *set != Charsets::ANSI)?;
    faces
        .iter()
        .filter(|face| face.charsets.contains(script))
        .min_by_key(|face| score(face))
}

/// The face sets a script needs, or `None` for one PDFium's list doesn't track.
const fn charsets(charset: FontCharset) -> Option<Charsets> {
    match charset {
        FontCharset::Ansi => Some(Charsets::ANSI),
        FontCharset::Symbol => Some(Charsets::SYMBOL),
        FontCharset::ShiftJis => Some(Charsets::SHIFT_JIS),
        FontCharset::Hangul => Some(Charsets::HANGUL),
        FontCharset::Gb2312 => Some(Charsets::GB2312),
        FontCharset::Big5 => Some(Charsets::BIG5),
        _ => None,
    }
}

/// A face name without its style suffix (`,Bold`, `-BoldItalic`), spaces or case: `Times-Roman`
/// and `times roman` match.
fn normalise(face: &str) -> String {
    let base = face.split(',').next().unwrap_or(face);
    let base = match base.rsplit_once('-') {
        Some((family, style)) if is_style(style) => family,
        _ => base,
    };
    base.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn is_style(word: &str) -> bool {
    [
        "Bold",
        "Italic",
        "BoldItalic",
        "Oblique",
        "BoldOblique",
        "Regular",
        "Roman",
    ]
    .iter()
    .any(|style| word.eq_ignore_ascii_case(style))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(id: u32, family: &str, weight: u16, charsets: Charsets) -> FontFace {
        FontFace {
            id,
            family: family.into(),
            style: String::new(),
            weight,
            italic: false,
            charsets,
        }
    }

    fn query(face: &str, weight: u32, charset: FontCharset) -> FontQuery<'_> {
        FontQuery {
            face,
            weight,
            italic: false,
            charset,
        }
    }

    #[test]
    fn a_base_14_name_should_find_its_stand_in_at_the_right_weight() {
        let faces = [
            face(0, "Arial", 400, Charsets::ANSI),
            face(1, "Arial", 700, Charsets::ANSI),
        ];
        let found = best(&faces, &query("Helvetica-Bold", 700, FontCharset::Ansi));
        assert_eq!(found.map(|f| f.id), Some(1));
    }

    #[test]
    fn an_unlisted_cjk_name_should_find_a_face_covering_its_script() {
        let faces = [
            face(0, "Arial", 400, Charsets::ANSI),
            face(1, "Noto Sans CJK SC", 400, Charsets::ANSI.with(Charsets::GB2312)),
        ];
        let found = best(&faces, &query("SimSun", 400, FontCharset::Gb2312));
        assert_eq!(found.map(|f| f.id), Some(1));
    }

    #[test]
    fn an_unlisted_latin_name_should_fall_back_to_pdfium() {
        let faces = [face(0, "Arial", 400, Charsets::ANSI)];
        assert!(best(&faces, &query("Futura", 400, FontCharset::Ansi)).is_none());
    }
}
