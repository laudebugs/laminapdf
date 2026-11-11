//! System fonts from the caller instead of PDFium's own folder scan: [`SystemFontSource`]. A
//! sandboxed process can't open font files, so it gives PDFium the fonts it gets from elsewhere
//! (the desktop engine asks its main process). PDFium calls the source when a PDF names a font it
//! doesn't embed; with no answer it falls back to its built-in fonts.

use pdfium_render::prelude::{
    PdfFontCharacterSet, PdfFontWeight, PdfiumCustomFontProvider, PdfiumCustomFontProviderRequest,
    PdfiumCustomFontProviderResponse,
};

/// The character set PDFium wants a font for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontCharset {
    /// Latin 1 (and PDFium's default).
    Ansi,
    /// Symbol.
    Symbol,
    /// Japanese (Shift JIS).
    ShiftJis,
    /// Korean (Hangul).
    Hangul,
    /// Simplified Chinese (GB 2312).
    Gb2312,
    /// Traditional Chinese (Big5).
    Big5,
    /// Another script (Greek, Cyrillic, Arabic, …).
    Other,
}

/// What PDFium asks for: a face name as the PDF gives it (`Helvetica-Bold`, `SimSun`), a weight
/// (400 regular, 700 bold) and whether it is italic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontQuery<'a> {
    /// The face name.
    pub face: &'a str,
    /// The weight.
    pub weight: u32,
    /// Italic.
    pub italic: bool,
    /// The script.
    pub charset: FontCharset,
}

/// A font the source found: a non-zero id it keeps for that face, its name, and the font file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemFont {
    /// The face's id, never 0 (PDFium uses it as a handle).
    pub id: u64,
    /// Its name.
    pub face: String,
    /// The whole font file, one face.
    pub data: Vec<u8>,
}

/// Where PDFium's system fonts come from.
pub trait SystemFontSource: Send + Sync {
    /// The face that best answers `query`, with its bytes; `None` lets PDFium fall back to its
    /// built-in fonts.
    fn find(&self, query: &FontQuery<'_>) -> Option<SystemFont>;
}

/// `pdfium-render`'s provider over a [`SystemFontSource`].
pub(crate) struct Provider(pub(crate) Box<dyn SystemFontSource>);

impl PdfiumCustomFontProvider for Provider {
    fn provide(&mut self, request: PdfiumCustomFontProviderRequest) -> Option<PdfiumCustomFontProviderResponse> {
        let query = FontQuery {
            face: &request.font_face,
            weight: weight(request.weight),
            italic: request.is_italic,
            charset: charset(&request.character_set),
        };
        let found = self.0.find(&query)?;
        if found.id == 0 {
            return None;
        }
        Some(PdfiumCustomFontProviderResponse {
            id: found.id,
            font_face: found.face,
            character_set: request.character_set,
            data: found.data,
        })
    }
}

const fn weight(weight: PdfFontWeight) -> u32 {
    match weight {
        PdfFontWeight::Weight100 => 100,
        PdfFontWeight::Weight200 => 200,
        PdfFontWeight::Weight300 => 300,
        PdfFontWeight::Weight400Normal => 400,
        PdfFontWeight::Weight500 => 500,
        PdfFontWeight::Weight600 => 600,
        PdfFontWeight::Weight700Bold => 700,
        PdfFontWeight::Weight800 => 800,
        PdfFontWeight::Weight900 => 900,
        PdfFontWeight::Custom(weight) => weight,
    }
}

const fn charset(charset: &PdfFontCharacterSet) -> FontCharset {
    match charset {
        PdfFontCharacterSet::Ansi | PdfFontCharacterSet::Default => FontCharset::Ansi,
        PdfFontCharacterSet::Symbol => FontCharset::Symbol,
        PdfFontCharacterSet::JapaneseShiftJis => FontCharset::ShiftJis,
        PdfFontCharacterSet::KoreanHangul => FontCharset::Hangul,
        PdfFontCharacterSet::ChineseGb2312 => FontCharset::Gb2312,
        PdfFontCharacterSet::ChineseBig5 => FontCharset::Big5,
        _ => FontCharset::Other,
    }
}
