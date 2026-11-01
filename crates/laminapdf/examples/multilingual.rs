//! Five scripts on one page: Latin, Arabic and Hebrew (right to left), Devanagari and
//! simplified Chinese, each shaped with HarfRust and bidi-resolved, as single lines and as
//! wrapped paragraphs in text boxes. Fonts are Noto subsets from `fixtures/fonts/` (OFL),
//! passed as bytes.
//!
//! ```sh
//! cargo run -p laminapdf --example multilingual -- [out-dir]   # writes <out-dir>/multilingual.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Alignment, BuildOptions, Builder, CancelToken, Direction, PageSize, PointPt, Pt, RectPt, SaveOptions,
    TextBoxOptions,
};

/// Each script's font, its line, its paragraph and the paragraph's direction.
const SCRIPTS: [(&[u8], &str, &str, Direction); 5] = [
    (
        include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf"),
        "Latin: the office staff fill the affine waffle flask",
        "A paragraph breaks at spaces and fits the width of its box, line by line.",
        Direction::LeftToRight,
    ),
    (
        include_bytes!("../../../fixtures/fonts/NotoSansArabic-Regular.ttf"),
        "مرحبا بالعالم السلام عليكم",
        "مرحبا بالعالم السلام عليكم مرحبا بالعالم السلام عليكم",
        Direction::RightToLeft,
    ),
    (
        include_bytes!("../../../fixtures/fonts/NotoSansDevanagari-Regular.ttf"),
        "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण",
        "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण नमस्ते दुनिया।",
        Direction::LeftToRight,
    ),
    (
        include_bytes!("../../../fixtures/fonts/NotoSansHebrew-Regular.ttf"),
        "שלום עולם וברוכים הבאים",
        "שלום עולם וברוכים הבאים שלום עולם וברוכים הבאים",
        Direction::RightToLeft,
    ),
    (
        include_bytes!("../../../fixtures/fonts/NotoSansSC-Regular.otf"),
        "你好世界，这是中文字体子集测试。",
        "你好世界，这是中文字体子集测试。你好世界，这是中文。",
        Direction::LeftToRight,
    ),
];

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("multilingual.pdf"), build()?)?;
    Ok(())
}

/// The document's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-multil")
        .with_producer("lamina examples")
        .with_creation_date("D:20261030120000Z".parse()?);
    let mut b = Builder::new(options);
    let fonts = SCRIPTS
        .iter()
        .map(|(bytes, ..)| b.font_bytes(bytes.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    for (row, ((_, line, paragraph, direction), font)) in (0_u32..).zip(SCRIPTS.iter().zip(&fonts)) {
        let y = 80.0 + 140.0 * f64::from(row);
        let _ = s.text(line, font, Pt::new(16.0), PointPt::from_points(56.0, y))?;
        // A narrow box breaks the paragraph into lines; right-to-left paragraphs start at the right.
        let options = TextBoxOptions::default()
            .with_direction(*direction)
            .with_align(Alignment::Start);
        let area = RectPt::from_xywh(Pt::new(56.0), Pt::new(y + 14.0), Pt::new(220.0), Pt::new(100.0));
        s.text_box(paragraph, font, Pt::new(12.0), area, &options)?;
    }
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}
