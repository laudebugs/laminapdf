//! The `flow-layout` spike's report (`spikes/flow-layout/src/inputs.rs`) as a flow, its words
//! drawn from the spike's list by the shared test generator: sections of a heading, four
//! subheadings and five paragraphs each, the Arabic and Devanagari paragraphs in the first, a running header and a "Page X of Y" footer. The spike's tables are
//! left out until flows have tables (T4c). The façade's flow tests compare it with what Typst
//! wrote for the spike, and `benches/flowing.rs` times it.
#![allow(dead_code, reason = "the tests and the bench use different parts")]

use laminapdf::flow::{Flow, FlowBlock, PageRegion, PageTemplate, Paragraph, TextStyle};
use laminapdf::{BuildOptions, Builder, Font, Metadata, PageSize, Profile};
use laminapdf_test_fixtures::Rng;

pub(crate) const TITLE: &str = "Quarterly report";
pub(crate) const ARABIC: &str = "تُظهر هذه الفقرة أن النص العربي يُشكَّل ويُرتَّب من اليمين إلى اليسار، \
                          وأن الحروف تتصل كما ينبغي في كل سطر من الأسطر.";
pub(crate) const DEVANAGARI: &str = "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण। नमस्ते दुनिया। हिन्दी पाठ का परीक्षण।";

const WORDS: &[&str] = &[
    "the",
    "report",
    "reader",
    "pages",
    "quarter",
    "revenue",
    "support",
    "files",
    "offline",
    "mode",
    "customers",
    "growth",
    "costs",
    "team",
    "release",
    "measured",
    "faster",
    "smaller",
    "every",
    "week",
    "requests",
    "fell",
    "rose",
    "while",
    "across",
    "regions",
    "and",
    "of",
    "in",
    "with",
    "for",
    "a",
    "documents",
    "opened",
    "signed",
    "printed",
    "searched",
    "shared",
    "archived",
    "tables",
    "figures",
];

fn sentence(rng: &mut Rng) -> String {
    let n = 8 + rng.below(12);
    let words: Vec<&str> = (0..n).map(|_| WORDS[rng.below(WORDS.len())]).collect();
    let text = words.join(" ");
    text[..1].to_uppercase() + &text[1..] + "."
}

fn paragraph(rng: &mut Rng) -> String {
    (0..3 + rng.below(4))
        .map(|_| sentence(rng))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The report's paragraphs in order, for checking what extraction and the structure tree read.
pub(crate) fn paragraphs(sections: usize) -> Vec<String> {
    let mut rng = Rng::new(0x7461_3431);
    let mut out = Vec::new();
    for section in 1..=sections {
        if section == 1 {
            out.push(ARABIC.to_owned());
            out.push(DEVANAGARI.to_owned());
        }
        for _ in 1..=4 {
            out.extend((0..5).map(|_| paragraph(&mut rng)));
        }
    }
    out
}

/// The report of `sections` sections (the spike's has 32) built through a flow, under PDF/UA-1
/// when `ua`; the fonts are Noto Sans, Noto Sans Bold's collection, Noto Sans Arabic and Noto
/// Sans Devanagari.
pub(crate) fn build(fonts: [&[u8]; 4], ua: bool, sections: usize) -> laminapdf::Document {
    let profile = if ua { Profile::PdfUa1 } else { Profile::None };
    let mut b = Builder::new(BuildOptions::new().with_id(*b"lamina-flow-spk!").with_profile(profile));
    b.lang("en");
    b.metadata(Metadata::default().with_title(TITLE));
    let regular = b.font(Font::from_bytes(fonts[0].to_vec()).unwrap()).unwrap();
    let bold = b.font(Font::from_collection(fonts[1].to_vec(), 1).unwrap()).unwrap();
    let body = TextStyle::new(&regular, 11.0)
        .with_fallback(&b.font(Font::from_bytes(fonts[2].to_vec()).unwrap()).unwrap())
        .with_fallback(&b.font(Font::from_bytes(fonts[3].to_vec()).unwrap()).unwrap());
    let header = PageRegion::new().with_end("{title}");
    let footer = PageRegion::new().with_center("Page {page} of {pages}");
    let template =
        PageTemplate::new(PageSize::A4).with_furniture(Some(header), Some(footer), &body.clone().with_size(9.0));
    let mut flow = Flow::new(template).with_title(TITLE);
    let mut paragraphs = paragraphs(sections).into_iter();
    for section in 1..=sections {
        let h1 = TextStyle::new(&bold, 16.0);
        flow = flow.block(FlowBlock::Heading(
            1,
            Paragraph::plain(format!("{section}. Section {section}"), &h1),
        ));
        let (scripts, per_section) = if section == 1 { (2, 22) } else { (0, 20) };
        for (i, text) in paragraphs.by_ref().take(per_section).enumerate() {
            if i >= scripts && (i - scripts) % 5 == 0 {
                let h2 = TextStyle::new(&bold, 13.0);
                let sub = (i - scripts) / 5 + 1;
                flow = flow.block(FlowBlock::Heading(
                    2,
                    Paragraph::plain(format!("{section}.{sub} Findings"), &h2),
                ));
            }
            let style = match i {
                0 if section == 1 => body.clone().with_language("ar"),
                1 if section == 1 => body.clone().with_language("hi"),
                _ => body.clone(),
            };
            flow = flow.block(FlowBlock::Paragraph(Paragraph::plain(text, &style)));
        }
    }
    b.flow(&flow).unwrap();
    b.finish().unwrap()
}
