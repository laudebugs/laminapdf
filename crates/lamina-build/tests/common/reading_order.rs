//! The reading-order corpus (todo U2b): `fixtures/reading-order/<name>.pdf` and
//! `<name>.json`, generated here so the ground truth is exact. Each layout lists its blocks
//! (page, role, text, box) in reading order; each is written twice, once drawn in reading
//! order (`-ordered`) and once drawn in reverse (`-reversed`), since an extractor that follows
//! the content stream gets the first right for free. The JSON holds the blocks in reading
//! order with the method; `crates/lamina/tests/reading_order.rs` scores extraction against
//! it. `cargo xtask golden --update` rewrites the files through `examples/reading_order.rs`.

use lamina_build::{BuildOptions, Builder, Direction, Font, PageSize, TextBoxOptions};
use lamina_model::{Pt, RectPt, Rotation};
use serde_json::json;

use super::{fixture, save};

/// A block's role, as the expected files name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Heading,
    Paragraph,
    Caption,
    Footnote,
    Header,
    Footer,
    TableCell,
}

impl Role {
    const fn name(self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::Paragraph => "paragraph",
            Self::Caption => "caption",
            Self::Footnote => "footnote",
            Self::Header => "header",
            Self::Footer => "footer",
            Self::TableCell => "table-cell",
        }
    }
}

/// The fonts a block can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    Latin,
    Arabic,
    Hebrew,
    Chinese,
}

/// One block: where it is and what it says.
#[derive(Debug, Clone)]
pub struct Block {
    pub page: usize,
    pub role: Role,
    pub text: String,
    /// Its box in page space: x, y (top), width, height.
    pub area: (f64, f64, f64, f64),
    pub size: f64,
    pub script: Script,
}

/// A layout: its pages and blocks in reading order.
#[derive(Debug, Clone)]
pub struct Layout {
    pub name: &'static str,
    pub pages: usize,
    pub rotate: Rotation,
    pub blocks: Vec<Block>,
    /// What the layout tests, for the JSON.
    pub about: &'static str,
}

/// One file of the corpus.
#[derive(Debug, Clone)]
pub struct Sample {
    pub name: String,
    pub pdf: Vec<u8>,
    pub json: String,
}

const LATIN_WORDS: [&str; 64] = [
    "river", "stone", "lantern", "harbour", "meadow", "copper", "violet", "engine", "orchard", "signal", "timber",
    "glacier", "pepper", "canyon", "ribbon", "falcon", "marble", "thunder", "velvet", "compass", "island", "garden",
    "silver", "window", "basket", "candle", "ladder", "pocket", "mirror", "saddle", "tunnel", "anchor", "bridge",
    "cotton", "desert", "feather", "goblet", "hammer", "jacket", "kettle", "lemon", "mantle", "needle", "oyster",
    "parcel", "quarry", "rocket", "spindle", "tablet", "umbrella", "valley", "walnut", "yonder", "zephyr", "amber",
    "beacon", "cobalt", "dynamo", "ember", "fresco", "granite", "hollow", "indigo", "juniper",
];

const ARABIC_WORDS: [&str; 16] = [
    "مرحبا",
    "بالعالم",
    "السلام",
    "عليكم",
    "كتاب",
    "مدرسة",
    "قلم",
    "بيت",
    "شمس",
    "قمر",
    "بحر",
    "نهر",
    "جبل",
    "مدينة",
    "طريق",
    "حديقة",
];

const HEBREW_WORDS: [&str; 16] = [
    "שלום",
    "עולם",
    "וברוכים",
    "הבאים",
    "ספר",
    "בית",
    "ים",
    "שמש",
    "ירח",
    "הר",
    "עיר",
    "דרך",
    "גן",
    "נהר",
    "אור",
    "מים",
];

/// The characters the Chinese fixture font has (`tools/fixtures/fonts.py`'s `CJK_TEXT`,
/// punctuation aside).
const CHINESE_CHARS: [&str; 20] = [
    "你", "好", "世", "界", "这", "是", "中", "文", "字", "体", "子", "集", "测", "试", "汉", "排", "版", "与", "本",
    "提",
];

/// `count` words of `script`, a different sequence for each `seed` (each block's own seed),
/// so a line of one block is never a line of another.
fn words(script: Script, seed: usize, count: usize) -> String {
    let list: &[&str] = match script {
        Script::Latin => &LATIN_WORDS,
        Script::Arabic => &ARABIC_WORDS,
        Script::Hebrew => &HEBREW_WORDS,
        Script::Chinese => &CHINESE_CHARS,
    };
    // A stride coprime with the list length visits every word before repeating.
    let strides = [1, 3, 5, 7, 9, 11, 13, 15];
    let stride = strides[seed % strides.len()];
    let start = seed * 17;
    let picked: Vec<&str> = (0..count).map(|i| list[(start + i * stride) % list.len()]).collect();
    let joiner = if script == Script::Chinese { "" } else { " " };
    let mut text = picked.join(joiner);
    if script == Script::Latin
        && let Some(first) = text.get(..1)
    {
        text = first.to_uppercase() + text.get(1..).unwrap_or_default();
        text.push('.');
    }
    text
}

fn block(page: usize, role: Role, seed: usize, count: usize, area: (f64, f64, f64, f64), size: f64) -> Block {
    Block {
        page,
        role,
        text: words(Script::Latin, seed, count),
        area,
        size,
        script: Script::Latin,
    }
}

fn scripted(script: Script, role: Role, seed: usize, count: usize, area: (f64, f64, f64, f64), size: f64) -> Block {
    Block {
        script,
        text: words(script, seed, count),
        ..block(0, role, seed, count, area, size)
    }
}

/// Every layout, each block in reading order. A4 is 595 × 842 pt.
#[expect(
    clippy::too_many_lines,
    reason = "one table of layouts: splitting it would scatter the corpus"
)]
pub fn layouts() -> Vec<Layout> {
    use Role::{Caption, Footer, Footnote, Header, Heading, Paragraph, TableCell};
    let single = |rotate, name, about| Layout {
        name,
        pages: 1,
        rotate,
        about,
        blocks: vec![
            block(0, Heading, 1, 4, (56.0, 56.0, 480.0, 30.0), 18.0),
            block(0, Paragraph, 2, 60, (56.0, 100.0, 480.0, 150.0), 11.0),
            block(0, Paragraph, 3, 50, (56.0, 260.0, 480.0, 150.0), 11.0),
            block(0, Paragraph, 4, 40, (56.0, 420.0, 480.0, 150.0), 11.0),
        ],
    };
    vec![
        single(
            Rotation::None,
            "one-column",
            "a heading and three paragraphs in one column",
        ),
        Layout {
            name: "two-columns",
            pages: 1,
            rotate: Rotation::None,
            about: "a heading across two columns of two paragraphs each: left column, then right",
            blocks: vec![
                block(0, Heading, 10, 5, (56.0, 56.0, 480.0, 30.0), 18.0),
                block(0, Paragraph, 11, 45, (56.0, 100.0, 230.0, 220.0), 10.0),
                block(0, Paragraph, 12, 40, (56.0, 330.0, 230.0, 220.0), 10.0),
                block(0, Paragraph, 13, 45, (306.0, 100.0, 230.0, 220.0), 10.0),
                block(0, Paragraph, 14, 40, (306.0, 330.0, 230.0, 220.0), 10.0),
            ],
        },
        Layout {
            name: "three-columns",
            pages: 1,
            rotate: Rotation::None,
            about: "a newspaper page: headline, byline, then three columns left to right",
            blocks: vec![
                block(0, Heading, 20, 6, (40.0, 40.0, 515.0, 36.0), 24.0),
                block(0, Paragraph, 21, 4, (40.0, 84.0, 515.0, 20.0), 9.0),
                block(0, Paragraph, 22, 70, (40.0, 112.0, 160.0, 400.0), 9.0),
                block(0, Paragraph, 23, 70, (217.0, 112.0, 160.0, 400.0), 9.0),
                block(0, Paragraph, 24, 70, (395.0, 112.0, 160.0, 400.0), 9.0),
            ],
        },
        Layout {
            name: "paper",
            pages: 1,
            rotate: Rotation::None,
            about: "an academic page: title, authors, abstract, two columns with a figure caption, a footnote",
            blocks: vec![
                block(0, Heading, 30, 7, (56.0, 50.0, 480.0, 30.0), 16.0),
                block(0, Paragraph, 31, 4, (56.0, 84.0, 480.0, 16.0), 10.0),
                block(0, Paragraph, 32, 45, (96.0, 110.0, 400.0, 90.0), 9.0),
                block(0, Paragraph, 33, 40, (56.0, 215.0, 230.0, 160.0), 9.0),
                block(0, Caption, 34, 10, (56.0, 520.0, 230.0, 40.0), 8.0),
                block(0, Paragraph, 35, 50, (306.0, 215.0, 230.0, 200.0), 9.0),
                block(0, Footnote, 36, 12, (56.0, 780.0, 480.0, 30.0), 7.0),
            ],
        },
        Layout {
            name: "sidebar",
            pages: 1,
            rotate: Rotation::None,
            about: "a main column, then a boxed sidebar to its right with its own heading",
            blocks: vec![
                block(0, Heading, 40, 4, (56.0, 56.0, 330.0, 30.0), 18.0),
                block(0, Paragraph, 41, 55, (56.0, 100.0, 330.0, 200.0), 10.0),
                block(0, Paragraph, 42, 45, (56.0, 310.0, 330.0, 200.0), 10.0),
                block(0, Heading, 43, 2, (406.0, 100.0, 130.0, 20.0), 12.0),
                block(0, Paragraph, 44, 30, (406.0, 126.0, 130.0, 250.0), 9.0),
            ],
        },
        Layout {
            name: "table",
            pages: 1,
            rotate: Rotation::None,
            about: "a caption and a three by three table, read row by row",
            blocks: {
                let mut blocks = vec![block(0, Caption, 50, 6, (56.0, 56.0, 480.0, 20.0), 11.0)];
                for row in 0..3 {
                    for col in 0..3 {
                        let x = 56.0 + 160.0 * f64::from(col);
                        let y = 90.0 + 40.0 * f64::from(row);
                        let seed = 51 + usize::try_from(row * 3 + col).unwrap_or(0);
                        blocks.push(block(0, TableCell, seed, 3, (x, y, 150.0, 30.0), 10.0));
                    }
                }
                blocks
            },
        },
        Layout {
            name: "header-footer",
            pages: 2,
            rotate: Rotation::None,
            about: "two pages, each with a running header, a paragraph and a footer",
            blocks: (0..2)
                .flat_map(|page| {
                    let seed = 60 + page * 3;
                    [
                        block(page, Header, seed, 4, (56.0, 30.0, 480.0, 16.0), 8.0),
                        block(page, Paragraph, seed + 1, 60, (56.0, 80.0, 480.0, 300.0), 11.0),
                        block(page, Footer, seed + 2, 3, (56.0, 800.0, 480.0, 16.0), 8.0),
                    ]
                })
                .collect(),
        },
        single(
            Rotation::Clockwise90,
            "rotated",
            "the one-column page turned by /Rotate 90",
        ),
        Layout {
            name: "arabic",
            pages: 1,
            rotate: Rotation::None,
            about: "a right-to-left page: an Arabic heading and two paragraphs",
            blocks: vec![
                scripted(Script::Arabic, Heading, 70, 3, (56.0, 50.0, 480.0, 60.0), 18.0),
                scripted(Script::Arabic, Paragraph, 71, 30, (56.0, 120.0, 480.0, 260.0), 12.0),
                scripted(Script::Arabic, Paragraph, 72, 25, (56.0, 400.0, 480.0, 260.0), 12.0),
            ],
        },
        Layout {
            name: "hebrew-two-columns",
            pages: 1,
            rotate: Rotation::None,
            about: "Hebrew in two columns, read right column first",
            blocks: vec![
                scripted(Script::Hebrew, Heading, 80, 3, (56.0, 50.0, 480.0, 60.0), 18.0),
                scripted(Script::Hebrew, Paragraph, 81, 30, (306.0, 120.0, 230.0, 500.0), 12.0),
                scripted(Script::Hebrew, Paragraph, 82, 30, (56.0, 120.0, 230.0, 500.0), 12.0),
            ],
        },
        Layout {
            name: "chinese",
            pages: 1,
            rotate: Rotation::None,
            about: "horizontal Chinese: a heading and two paragraphs (vertical writing is not built yet)",
            blocks: vec![
                scripted(Script::Chinese, Heading, 90, 6, (56.0, 50.0, 480.0, 50.0), 18.0),
                scripted(Script::Chinese, Paragraph, 91, 80, (56.0, 110.0, 480.0, 200.0), 12.0),
                scripted(Script::Chinese, Paragraph, 92, 70, (56.0, 330.0, 480.0, 200.0), 12.0),
            ],
        },
    ]
}

/// How a variant draws its blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawOrder {
    Reading,
    Reversed,
}

fn font_file(script: Script) -> &'static str {
    match script {
        Script::Latin => "NotoSans-Regular.ttf",
        Script::Arabic => "NotoSansArabic-Regular.ttf",
        Script::Hebrew => "NotoSansHebrew-Regular.ttf",
        Script::Chinese => "NotoSansSC-Regular.otf",
    }
}

fn build(layout: &Layout, order: DrawOrder) -> Vec<u8> {
    let options = BuildOptions::default()
        .with_id(*b"lamina-reading-o")
        .with_producer("lamina-build reading-order corpus")
        .with_creation_date("D:20261030120000Z".parse().unwrap())
        .with_compress(false);
    let mut b = Builder::new(options);
    let scripts = [Script::Latin, Script::Arabic, Script::Hebrew, Script::Chinese];
    let fonts: Vec<(Script, Font)> = scripts
        .iter()
        .filter(|s| layout.blocks.iter().any(|b| b.script == **s))
        .map(|s| {
            (
                *s,
                b.font(lamina_text::Font::from_bytes(fixture(&format!("fonts/{}", font_file(*s)))).unwrap())
                    .unwrap(),
            )
        })
        .collect();
    for page_index in 0..layout.pages {
        let mut page = b.page(PageSize::A4);
        page.rotate(layout.rotate);
        let s = page.surface();
        let mut blocks: Vec<&Block> = layout.blocks.iter().filter(|b| b.page == page_index).collect();
        if order == DrawOrder::Reversed {
            blocks.reverse();
        }
        for block in blocks {
            let font = &fonts.iter().find(|(s, _)| *s == block.script).unwrap().1;
            let direction = match block.script {
                Script::Arabic | Script::Hebrew => Direction::RightToLeft,
                Script::Latin | Script::Chinese => Direction::LeftToRight,
            };
            let (left, top, width, height) = block.area;
            let area = RectPt::from_xywh(Pt::new(left), Pt::new(top), Pt::new(width), Pt::new(height));
            let options = TextBoxOptions::default()
                .with_direction(direction)
                .with_overflow(lamina_build::Overflow::Error);
            s.text_box(&block.text, font, Pt::new(block.size), area, &options)
                .unwrap_or_else(|e| panic!("{} block {:?}: {e}", layout.name, block.role));
        }
        page.finish().unwrap();
    }
    save(b.finish().unwrap())
}

fn expected(layout: &Layout, file: &str, order: DrawOrder) -> String {
    let drawn = match order {
        DrawOrder::Reading => "in reading order",
        DrawOrder::Reversed => "in reverse reading order (last block first, on each page)",
    };
    let blocks: Vec<_> = layout
        .blocks
        .iter()
        .map(|b| json!({ "page": b.page, "role": b.role.name(), "text": b.text }))
        .collect();
    let value = json!({
        "file": file,
        "layout": layout.name,
        "about": layout.about,
        "drawn": drawn,
        "method": "generated by crates/lamina-build/tests/common/reading_order.rs: each block is one text_box, so the order and text are the generator's (no hand annotation)",
        "blocks": blocks,
    });
    serde_json::to_string_pretty(&value).unwrap() + "\n"
}

/// Every file of the corpus, in name order.
pub fn corpus() -> Vec<Sample> {
    let mut samples = Vec::new();
    for layout in layouts() {
        for (order, suffix) in [(DrawOrder::Reading, "ordered"), (DrawOrder::Reversed, "reversed")] {
            let name = format!("{}-{suffix}", layout.name);
            let file = format!("{name}.pdf");
            samples.push(Sample {
                json: expected(&layout, &file, order),
                pdf: build(&layout, order),
                name,
            });
        }
    }
    samples.sort_by(|a, b| a.name.cmp(&b.name));
    samples
}
