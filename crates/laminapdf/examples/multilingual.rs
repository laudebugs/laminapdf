//! Five scripts on one page, each as a single line and as a wrapped paragraph.
//!
//! Latin, Arabic and Hebrew (right to left), Devanagari and simplified Chinese, each shaped
//! and bidi-resolved, with the paragraphs wrapped in text boxes. Each script needs a font that
//! covers it, such as the Noto fonts (OFL).
//!
//! ```sh
//! cargo run -p laminapdf --example multilingual -- latin.ttf arabic.ttf devanagari.ttf hebrew.ttf chinese.otf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Alignment, BuildOptions, Builder, Direction, PageSize, PointPt, Pt, RectPt, SaveOptions, TextBoxOptions,
};

/// Each script's line, its paragraph and the paragraph's direction, in the order of the fonts.
const SCRIPTS: [(&str, &str, Direction); 5] = [
    (
        "Latin: the office staff fill the affine waffle flask",
        "A paragraph breaks at spaces and fits the width of its box, line by line.",
        Direction::LeftToRight,
    ),
    (
        "مرحبا بالعالم السلام عليكم",
        "مرحبا بالعالم السلام عليكم مرحبا بالعالم السلام عليكم",
        Direction::RightToLeft,
    ),
    (
        "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण",
        "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण नमस्ते दुनिया।",
        Direction::LeftToRight,
    ),
    (
        "שלום עולם וברוכים הבאים",
        "שלום עולם וברוכים הבאים שלום עולם וברוכים הבאים",
        Direction::RightToLeft,
    ),
    (
        "你好世界，这是中文字体子集测试。",
        "你好世界，这是中文字体子集测试。你好世界，这是中文。",
        Direction::LeftToRight,
    ),
];
const MARGIN: f64 = 56.0;
const FIRST_BASELINE: f64 = 80.0;
/// The distance from one script's line to the next one's.
const SCRIPT_HEIGHT: f64 = 140.0;
/// The paragraph's box: this far below the line, this wide and this tall.
const PARAGRAPH_DROP: f64 = 14.0;
const PARAGRAPH_WIDTH: f64 = 220.0;
const PARAGRAPH_HEIGHT: f64 = 100.0;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(if args.len() > SCRIPTS.len() {
        args.remove(SCRIPTS.len())
    } else {
        "target/examples-out".into()
    });
    let fonts = args.iter().map(std::fs::read).collect::<Result<Vec<_>, _>>()?;
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("multilingual.pdf"), build(BuildOptions::new(), fonts)?)?;
    Ok(())
}

/// The document's bytes, with one font per script in `fonts` (in the order of [`SCRIPTS`]).
/// `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions, fonts: Vec<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    if fonts.len() != SCRIPTS.len() {
        return Err(format!("{} fonts, one per script, are needed", SCRIPTS.len()).into());
    }
    let mut b = Builder::new(options);
    let fonts = fonts
        .into_iter()
        .map(|bytes| b.font_bytes(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    for (row, ((line, paragraph, direction), font)) in (0_u32..).zip(SCRIPTS.iter().zip(&fonts)) {
        let y = FIRST_BASELINE + SCRIPT_HEIGHT * f64::from(row);
        s.text(line, font, Pt::new(16.0), PointPt::from_points(MARGIN, y))?;
        // A narrow box breaks the paragraph into lines; right-to-left paragraphs start at the right.
        let options = TextBoxOptions::new()
            .with_direction(*direction)
            .with_align(Alignment::Start);
        let area = RectPt::from_points(MARGIN, y + PARAGRAPH_DROP, PARAGRAPH_WIDTH, PARAGRAPH_HEIGHT);
        s.text_box(paragraph, font, Pt::new(12.0), area, &options)?;
    }
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
