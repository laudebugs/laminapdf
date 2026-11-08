//! The paginator's rules on small flows of one font: page breaks, a first page of another width,
//! keep chains, spacers, a line taller than a page; and a property test: random flows place
//! every line exactly once, inside the body, in order, and give the same pages twice.

use proptest::prelude::*;

use super::items::Items;
use super::model::{Flow, FlowBlock, Margins, PageTemplate, Paragraph, ParagraphStyle, TextStyle};
use super::paginate::{PageOut, Placed, paginate};
use crate::builder::{BuildOptions, Builder};
use crate::error::Error;
use crate::page::PageSize;
use crate::surface::text::FontHandle;

const NOTO: &[u8] = include_bytes!("../../../../fixtures/fonts/NotoSans-Regular.ttf");

fn font() -> FontHandle {
    let mut b = Builder::new(BuildOptions::new());
    b.font(laminapdf_text::Font::from_bytes(NOTO.to_vec()).unwrap())
        .unwrap()
}

/// A page `width` points wide whose body holds `lines` lines of 10-point Noto Sans.
fn template(font: &FontHandle, width: f64, lines: f64) -> PageTemplate {
    let info = font.info();
    let line = f64::from(info.ascent - info.descent) * 10.0 / f64::from(info.units_per_em);
    let size = PageSize::new(
        laminapdf_model::Pt::new(width),
        laminapdf_model::Pt::new(100.0 + lines * line + 0.01),
    );
    PageTemplate::new(size).with_margins(Margins::all(50.0))
}

/// `n` lines, each ended by a line break, so the count is exact at any width.
fn lines(n: usize, style: &TextStyle, para: ParagraphStyle) -> Paragraph {
    (0..n).fold(Paragraph::new(para), |p, i| {
        let p = p.text(format!("Line {i}."), style);
        if i + 1 < n { p.line_break() } else { p }
    })
}

fn pages(flow: &Flow) -> Result<Vec<PageOut>, Error> {
    let items = Items::new(&flow.blocks);
    paginate(&items, &flow.template, 1).map(|(pages, _)| pages)
}

/// Each page's lines as (item, first byte).
fn placed(pages: &[PageOut]) -> Vec<Vec<(usize, usize)>> {
    pages
        .iter()
        .map(|p| {
            p.placed
                .iter()
                .filter_map(|x| match x {
                    Placed::Line { item, line, .. } => Some((*item, line.line.range.start)),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

fn tight() -> ParagraphStyle {
    ParagraphStyle::new().with_space(0.0, 0.0)
}

#[test]
fn a_page_break_starts_a_page_and_a_second_one_adds_none() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    let flow = Flow::new(template(&font, 300.0, 10.0))
        .block(FlowBlock::Paragraph(lines(1, &style, tight())))
        .block(FlowBlock::PageBreak)
        .block(FlowBlock::PageBreak)
        .block(FlowBlock::Paragraph(lines(1, &style, tight())));
    assert_eq!(
        placed(&pages(&flow).unwrap()).iter().map(Vec::len).collect::<Vec<_>>(),
        [1, 1]
    );
}

#[test]
fn a_paragraph_continued_on_a_wider_page_is_broken_again_at_its_width() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    let text = "word ".repeat(60);
    let narrow = template(&font, 150.0, 4.0);
    let flow = Flow::new(template(&font, 500.0, 4.0).with_first_page(narrow))
        .block(FlowBlock::Paragraph(Paragraph::plain(text.trim_end(), &style)));
    let pages = pages(&flow).unwrap();
    let widths: Vec<f64> = pages
        .iter()
        .map(|p| {
            p.placed
                .iter()
                .filter_map(|x| match x {
                    Placed::Line { line, .. } => Some(line.line.width),
                    _ => None,
                })
                .fold(0.0, f64::max)
        })
        .collect();
    assert!(widths[0] <= 50.0 + 1e-9 && widths[1] > 300.0, "{widths:?}");
}

#[test]
fn a_spacer_at_the_top_of_a_page_is_dropped() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    let flow = Flow::new(template(&font, 300.0, 4.0))
        .block(FlowBlock::Paragraph(lines(4, &style, tight())))
        .block(FlowBlock::Spacer(30.0))
        .block(FlowBlock::Paragraph(lines(1, &style, tight())));
    let pages = pages(&flow).unwrap();
    let top = pages[1].placed.iter().find_map(|x| match x {
        Placed::Line { top, .. } => Some(*top),
        _ => None,
    });
    assert_eq!(top, Some(50.0));
}

#[test]
fn a_heading_chain_moves_to_the_next_page_once() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    let flow = Flow::new(template(&font, 300.0, 6.0))
        .block(FlowBlock::Paragraph(lines(3, &style, tight())))
        .block(FlowBlock::Heading(1, lines(1, &style, tight())))
        .block(FlowBlock::Heading(2, lines(1, &style, tight())))
        .block(FlowBlock::Paragraph(lines(3, &style, tight())));
    let items: Vec<Vec<usize>> = placed(&pages(&flow).unwrap())
        .iter()
        .map(|p| p.iter().map(|(i, _)| *i).collect())
        .collect();
    assert_eq!(items, [vec![0, 0, 0], vec![1, 2, 3, 3, 3]]);
}

#[test]
fn a_line_taller_than_the_body_fails() {
    let font = font();
    let style = TextStyle::new(&font, 40.0);
    let flow = Flow::new(template(&font, 300.0, 1.0)).block(FlowBlock::Paragraph(Paragraph::plain("Tall", &style)));
    assert_eq!(pages(&flow).unwrap_err(), Error::FlowContentTooLarge { what: "line" });
}

/// One block of a random flow: a paragraph's line count and rules, or a page break.
#[derive(Debug, Clone)]
enum Spec {
    Para {
        lines: usize,
        widows: usize,
        orphans: usize,
        keep: bool,
        heading: bool,
    },
    Break,
}

fn spec() -> impl Strategy<Value = Spec> {
    prop_oneof![
        9 => (1..14_usize, 1..4_usize, 1..4_usize, any::<bool>(), any::<bool>()).prop_map(
            |(lines, widows, orphans, keep, heading)| Spec::Para { lines, widows, orphans, keep, heading }
        ),
        1 => Just(Spec::Break),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn every_line_is_placed_once_in_order_inside_the_body(specs in prop::collection::vec(spec(), 1..25)) {
        let font = font();
        let style = TextStyle::new(&font, 10.0);
        let template = template(&font, 300.0, 10.0);
        let mut flow = Flow::new(template.clone());
        let mut expected = Vec::new();
        for (i, s) in specs.iter().enumerate() {
            flow = flow.block(match *s {
                Spec::Break => FlowBlock::PageBreak,
                Spec::Para { lines: n, widows, orphans, keep, heading } => {
                    let para = tight().with_widows_and_orphans(widows, orphans).with_keep_with_next(keep);
                    let p = lines(n, &style, para);
                    expected.extend((0..n).map(|_| i));
                    if heading { FlowBlock::Heading(1, p) } else { FlowBlock::Paragraph(p) }
                }
            });
        }
        let first = pages(&flow).unwrap();
        let (top, bottom) = template.body_y();
        for page in &first {
            for x in &page.placed {
                if let Placed::Line { line, top: y, .. } = x {
                    prop_assert!(*y >= top - 1e-9 && *y + line.height <= bottom + 1e-6);
                }
            }
        }
        let order: Vec<(usize, usize)> = placed(&first).concat();
        // In order: item by item, and within one, byte by byte.
        prop_assert!(order.windows(2).all(|w| w[0] < w[1]));
        prop_assert_eq!(order.iter().map(|(i, _)| *i).collect::<Vec<_>>(), expected);
        prop_assert_eq!(placed(&pages(&flow).unwrap()), placed(&first));
    }
}
