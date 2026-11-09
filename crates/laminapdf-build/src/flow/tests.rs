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
    paginate(&items, &flow.template, 1, 0).map(|(pages, _)| pages)
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

#[test]
fn a_heading_before_a_paragraph_too_short_to_split_moves_with_it() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    // Room for the heading and two lines of a three-line paragraph, which can't split 2 and 2.
    let flow = Flow::new(template(&font, 300.0, 10.0))
        .block(FlowBlock::Paragraph(lines(7, &style, tight())))
        .block(FlowBlock::Heading(1, lines(1, &style, tight())))
        .block(FlowBlock::Paragraph(lines(3, &style, tight())));
    let counts: Vec<usize> = placed(&pages(&flow).unwrap()).iter().map(Vec::len).collect();
    assert_eq!(counts, [7, 4]);
}

#[test]
fn a_keep_group_of_one_paragraph_moves_whole() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    let flow = Flow::new(template(&font, 300.0, 10.0))
        .block(FlowBlock::Paragraph(lines(6, &style, tight())))
        .block(FlowBlock::Keep(vec![FlowBlock::Paragraph(lines(6, &style, tight()))]));
    let counts: Vec<usize> = placed(&pages(&flow).unwrap()).iter().map(Vec::len).collect();
    assert_eq!(counts, [6, 6]);
}

#[test]
fn a_spacer_adds_to_the_space_after_the_block_before_it() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    let spaced = ParagraphStyle::new().with_space(0.0, 6.0);
    let flow = Flow::new(template(&font, 300.0, 10.0))
        .block(FlowBlock::Paragraph(lines(1, &style, spaced)))
        .block(FlowBlock::Spacer(2.0))
        .block(FlowBlock::Paragraph(lines(1, &style, tight())));
    let pages = pages(&flow).unwrap();
    let boxes: Vec<(f64, f64)> = pages[0]
        .placed
        .iter()
        .filter_map(|x| match x {
            Placed::Line { top, line, .. } => Some((*top, top + line.height)),
            _ => None,
        })
        .collect();
    // 6 points after the first paragraph, then the spacer's 2.
    assert!((boxes[1].0 - boxes[0].1 - 8.0).abs() < 1e-9, "{boxes:?}");
}

#[test]
fn a_page_break_at_the_end_adds_no_blank_page() {
    let font = font();
    let style = TextStyle::new(&font, 10.0);
    let flow = Flow::new(template(&font, 300.0, 10.0))
        .block(FlowBlock::Paragraph(lines(2, &style, tight())))
        .block(FlowBlock::PageBreak);
    assert_eq!(pages(&flow).unwrap().len(), 1);
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

mod tables {
    use super::*;
    use crate::flow::table::{ColumnWidth, Table, TableRow};

    /// A table of `rows` one-line body rows under one header row and over one footer row.
    fn invoice(style: &TextStyle, rows: usize) -> Table {
        let mut table = Table::new([ColumnWidth::Auto, ColumnWidth::Fraction(1.0)])
            .with_padding(0.0)
            .header(TableRow::plain(["Item", "Price"], style))
            .footer(TableRow::plain(["Total", "100"], style));
        for i in 0..rows {
            table = table.row(TableRow::plain([format!("Row {i}"), "1".to_owned()], style));
        }
        table
    }

    /// Each page's rows as (row, repeated).
    fn rows(pages: &[PageOut]) -> Vec<Vec<(usize, bool)>> {
        pages
            .iter()
            .map(|p| {
                p.placed
                    .iter()
                    .filter_map(|x| match x {
                        Placed::Row { row, repeat, .. } => Some((*row, *repeat)),
                        _ => None,
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_long_table_repeats_its_header_and_keeps_its_footer_with_the_last_row() {
        let font = font();
        let style = TextStyle::new(&font, 10.0);
        // 10 lines a page: the header and 9 body rows; 20 body rows fill two pages and two
        // rows of a third, which takes the footer too.
        let flow = Flow::new(template(&font, 300.0, 10.0)).block(FlowBlock::Table(invoice(&style, 20)));
        let pages = rows(&pages(&flow).unwrap());
        assert_eq!(pages.len(), 3);
        for (n, page) in pages.iter().enumerate() {
            assert_eq!(page.first(), Some(&(0, n > 0)), "page {n} starts with the header");
        }
        let body: Vec<usize> = pages.iter().flatten().filter(|r| r.0 > 0).map(|r| r.0).collect();
        assert_eq!(body, (1..=21).collect::<Vec<_>>(), "every row once, in order");
        assert_eq!(pages.last().and_then(|p| p.last()), Some(&(21, false)));
    }

    #[test]
    fn the_last_body_row_moves_with_the_footer_when_both_do_not_fit() {
        let font = font();
        let style = TextStyle::new(&font, 10.0);
        // The header, 8 rows and the footer fill a page exactly; 9 rows leave the footer no
        // room, so the ninth row moves with it.
        let flow = Flow::new(template(&font, 300.0, 10.0)).block(FlowBlock::Table(invoice(&style, 9)));
        let pages = rows(&pages(&flow).unwrap());
        assert_eq!(pages.len(), 2);
        assert_eq!(pages.get(1), Some(&vec![(0, true), (9, false), (10, false)]));
    }

    #[test]
    fn a_table_that_fits_after_a_paragraph_stays_on_its_page() {
        let font = font();
        let style = TextStyle::new(&font, 10.0);
        let flow = Flow::new(template(&font, 300.0, 10.0))
            .block(FlowBlock::Paragraph(lines(2, &style, tight())))
            .block(FlowBlock::Table(invoice(&style, 3).with_space(0.0, 0.0)));
        let pages = rows(&pages(&flow).unwrap());
        assert_eq!(
            pages,
            vec![vec![(0, false), (1, false), (2, false), (3, false), (4, false)]]
        );
    }

    #[test]
    fn a_row_taller_than_the_body_fails() {
        let font = font();
        let style = TextStyle::new(&font, 10.0);
        let tall = (0..12)
            .map(|i| format!("Line {i}"))
            .collect::<Vec<_>>()
            .join("\u{2028}");
        let table = Table::new([ColumnWidth::Fraction(1.0)])
            .with_padding(0.0)
            .row(TableRow::plain([tall], &style));
        let flow = Flow::new(template(&font, 300.0, 10.0)).block(FlowBlock::Table(table));
        assert!(matches!(
            pages(&flow),
            Err(Error::FlowContentTooLarge { what: "table row" })
        ));
    }

    #[test]
    fn column_widths_are_the_same_on_every_page() {
        let font = font();
        let style = TextStyle::new(&font, 10.0);
        let first = template(&font, 500.0, 10.0);
        let flow =
            Flow::new(template(&font, 300.0, 10.0).with_first_page(first)).block(FlowBlock::Table(invoice(&style, 30)));
        let laid = pages(&flow).unwrap();
        let widths: Vec<f64> = laid
            .iter()
            .flat_map(|p| &p.placed)
            .filter_map(|x| match x {
                Placed::Row { table, .. } => Some(table.width),
                _ => None,
            })
            .collect();
        assert!(laid.len() > 2);
        assert!(widths.iter().all(|w| (w - 200.0).abs() < 1e-9), "{widths:?}");
    }
}
