//! The spike's documents, built once and handed to every experiment: a two-page invoice from
//! `inputs/invoice.json` and a seeded 100-page report. `to_typst` writes the same content as Typst
//! markup, so the engines lay out the same words.

use std::fmt::Write as _;

/// One block of a flowing document.
#[derive(Debug, Clone)]
pub enum Block {
    /// A heading, level 1 or 2.
    Heading(u8, String),
    /// A paragraph in the body font.
    Para(String),
    /// A paragraph in another script, with its BCP 47 language (`ar`, `hi`).
    Script { lang: &'static str, text: String },
    /// A table whose header row repeats on every page it crosses.
    Table { header: Vec<String>, rows: Vec<Vec<String>> },
}

/// A document: a title for the running header, and its blocks.
#[derive(Debug, Clone)]
pub struct Doc {
    pub title: String,
    pub blocks: Vec<Block>,
}

pub const ARABIC: &str = "تُظهر هذه الفقرة أن النص العربي يُشكَّل ويُرتَّب من اليمين إلى اليسار، \
                          وأن الحروف تتصل كما ينبغي في كل سطر من الأسطر.";
/// The Devanagari subset in `fixtures/fonts/` covers only this sentence (`tools/fixtures/fonts.py`).
pub const DEVANAGARI: &str = "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण। नमस्ते दुनिया। हिन्दी पाठ का परीक्षण।";

/// A tiny deterministic generator (SplitMix64), so every run builds the same report.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const WORDS: &[&str] = &[
    "the", "report", "reader", "pages", "quarter", "revenue", "support", "files", "offline", "mode",
    "customers", "growth", "costs", "team", "release", "measured", "faster", "smaller", "every", "week",
    "requests", "fell", "rose", "while", "across", "regions", "and", "of", "in", "with", "for", "a",
    "documents", "opened", "signed", "printed", "searched", "shared", "archived", "tables", "figures",
];

fn sentence(rng: &mut Rng) -> String {
    let n = 8 + rng.below(12) as usize;
    let words: Vec<&str> = (0..n).map(|_| WORDS[rng.below(WORDS.len() as u64) as usize]).collect();
    let text = words.join(" ");
    text[..1].to_uppercase() + &text[1..] + "."
}

fn paragraph(rng: &mut Rng) -> String {
    (0..3 + rng.below(4)).map(|_| sentence(rng)).collect::<Vec<_>>().join(" ")
}

/// The 100-page report: 32 sections of a heading and paragraphs, then a long table every third
/// section, and the Arabic and Devanagari paragraphs in the first section.
pub fn report() -> Doc {
    let mut rng = Rng(0x7461_3431); // "ta41"
    let mut blocks = Vec::new();
    for section in 1..=32 {
        blocks.push(Block::Heading(1, format!("{section}. Section {section}")));
        if section == 1 {
            blocks.push(Block::Script { lang: "ar", text: ARABIC.to_owned() });
            blocks.push(Block::Script { lang: "hi", text: DEVANAGARI.to_owned() });
        }
        for sub in 1..=4 {
            blocks.push(Block::Heading(2, format!("{section}.{sub} Findings")));
            for _ in 0..5 {
                blocks.push(Block::Para(paragraph(&mut rng)));
            }
        }
        if section % 3 == 0 {
            let header = ["Region", "Quarter", "Files", "Pages", "Change"].map(str::to_owned).to_vec();
            let rows = (1..=60)
                .map(|i| {
                    vec![
                        format!("Region {}", 1 + rng.below(9)),
                        format!("Q{}", 1 + i % 4),
                        format!("{}", 100 + rng.below(9_000)),
                        format!("{}", 1_000 + rng.below(90_000)),
                        format!("{}%", rng.below(40)),
                    ]
                })
                .collect();
            blocks.push(Block::Table { header, rows });
        }
    }
    Doc { title: "Annual report".to_owned(), blocks }
}

/// The invoice from `inputs/invoice.json`: a heading, the parties, the line items as a table, and
/// the total.
pub fn invoice(json: &str) -> Result<Doc, serde_json::Error> {
    let v: serde_json::Value = serde_json::from_str(json)?;
    let s = |key: &str| v[key].as_str().unwrap_or_default().to_owned();
    let mut rows = Vec::new();
    let mut total = 0_u64;
    for item in v["items"].as_array().into_iter().flatten() {
        let qty = item["qty"].as_u64().unwrap_or(0);
        let cents = item["unitCents"].as_u64().unwrap_or(0);
        total += qty * cents;
        rows.push(vec![
            item["description"].as_str().unwrap_or_default().to_owned(),
            qty.to_string(),
            money(cents),
            money(qty * cents),
        ]);
    }
    let header = ["Description", "Qty", "Unit", "Amount"].map(str::to_owned).to_vec();
    Ok(Doc {
        title: format!("Invoice {}", s("number")),
        blocks: vec![
            Block::Heading(1, format!("Invoice {}", s("number"))),
            Block::Para(format!("From {} to {}, issued {}.", s("seller"), s("buyer"), s("date"))),
            Block::Table { header, rows },
            Block::Para(format!("Total due: {}", money(total))),
        ],
    })
}

fn money(cents: u64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// The same document as Typst markup, with the running header and the "Page X of Y" footer that
/// experiment A draws.
pub fn to_typst(doc: &Doc) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "#set document(title: \"{title}\")\n\
         #set page(paper: \"a4\", margin: (x: 56pt, top: 72pt, bottom: 64pt),\n\
           header: align(right, text(9pt)[{title}]),\n\
           footer: context align(center, text(9pt)[Page #counter(page).display() of #counter(page).final().first()]))\n\
         #set text(font: \"Noto Sans\", size: 11pt, lang: \"en\")\n\
         #set par(justify: false, leading: 0.65em)\n\
         #show heading: set block(sticky: true)",
        title = escape(&doc.title)
    );
    for block in &doc.blocks {
        match block {
            Block::Heading(level, text) => {
                let _ = writeln!(out, "\n{} {}\n", "=".repeat(*level as usize), escape(text));
            }
            Block::Para(text) => {
                let _ = writeln!(out, "{}\n", escape(text));
            }
            Block::Script { lang, text } => {
                let font = if *lang == "ar" { "Noto Sans Arabic" } else { "Noto Sans Devanagari" };
                let _ = writeln!(out, "#text(lang: \"{lang}\", font: \"{font}\")[{}]\n", escape(text));
            }
            Block::Table { header, rows } => {
                let cols = header.len();
                let _ = write!(out, "#table(columns: {cols}, table.header(");
                for cell in header {
                    let _ = write!(out, "[*{}*],", escape(cell));
                }
                out.push_str("),\n");
                for row in rows {
                    for cell in row {
                        let _ = write!(out, "[{}],", escape(cell));
                    }
                    out.push('\n');
                }
                out.push_str(")\n\n");
            }
        }
    }
    out
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '#' | '*' | '_' | '[' | ']' | '$' | '<' | '>' | '@' | '`' | '"') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
