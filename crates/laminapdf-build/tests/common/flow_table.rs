//! The document behind `fixtures/golden/flow-table.pdf` (todo T4c): an invoice under PDF/UA-1
//! whose table of 60 rows runs over three A4 pages. Its header row is drawn again at the top of
//! pages 2 and 3, no row is split, the totals footer is on the last row's page, and the
//! automatic column widths come from the widest description.

use laminapdf_build::flow::{
    ColumnWidth, Flow, FlowBlock, Margins, PageRegion, PageTemplate, Paragraph, Table, TableCell, TableRow, TableRule,
    TextStyle,
};
use laminapdf_build::{BuildOptions, Builder, Color, Metadata, PageSize, Profile};
use laminapdf_text::Alignment;

use laminapdf_test_fixtures::fixture;

use super::{save, text_and_navigation::font};

/// How many body rows the table has.
pub const ROWS: usize = 60;
/// The header row's cells.
pub const HEADER: [&str; 4] = ["Item", "Description", "Qty", "Amount"];
/// The footer row's first cell.
pub const TOTAL: &str = "Total";

const DESCRIPTIONS: [&str; 6] = [
    "Paper, A4, 80 g, box of 2,500 sheets",
    "Toner cartridge",
    "Desk lamp with a weighted base and an adjustable arm, in matt black",
    "Stapler",
    "Archive boxes, set of ten, flat-packed, with lids",
    "Pens",
];

/// Row `i`'s quantity and amount in cents.
fn line(i: usize) -> (usize, usize) {
    let qty = i % 7 + 1;
    (qty, qty * (450 + (i * 137) % 9_000))
}

fn money(cents: usize) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// The invoice table over `body`, its numbers aligned to the end.
pub fn invoice_table(body: &TextStyle, bold: &TextStyle) -> Table {
    let number = |text: String, style: &TextStyle| TableCell::plain(text, style).with_align(Alignment::End);
    let rule = |width| TableRule::new(width, Color::gray(0.4));
    let mut table = Table::new([
        ColumnWidth::Auto,
        ColumnWidth::Auto,
        ColumnWidth::Auto,
        ColumnWidth::Auto,
    ])
    .with_padding(6.0)
    .with_frame(rule(0.75))
    .with_header_rule(rule(0.75))
    .with_row_rules(TableRule::new(0.25, Color::gray(0.7)))
    .with_header_background(Color::gray(0.92))
    .header(
        TableRow::new()
            .cell(TableCell::plain(HEADER[0], bold))
            .cell(TableCell::plain(HEADER[1], bold))
            .cell(TableCell::plain(HEADER[2], bold).with_align(Alignment::End))
            .cell(TableCell::plain(HEADER[3], bold).with_align(Alignment::End)),
    );
    let mut total = 0;
    for i in 0..ROWS {
        let (qty, cents) = line(i);
        total += cents;
        table = table.row(
            TableRow::new()
                .cell(TableCell::plain(format!("{:03}", i + 1), body))
                .cell(TableCell::plain(DESCRIPTIONS[i % DESCRIPTIONS.len()], body))
                .cell(number(qty.to_string(), body))
                .cell(number(money(cents), body)),
        );
    }
    table.footer(
        TableRow::new()
            .cell(TableCell::plain(TOTAL, bold).with_span(3))
            .cell(number(money(total), bold)),
    )
}

/// `fixtures/golden/flow-table.pdf`.
pub fn flow_table() -> Vec<u8> {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_id(*b"lamina-build-T4c")
            .with_producer("lamina-build golden")
            .with_creation_date("D:20261109120000Z".parse().unwrap())
            .with_compress(false)
            .with_profile(Profile::PdfUa1),
    );
    let regular = b.font(font("NotoSans-Regular.ttf")).unwrap();
    let bold = laminapdf_text::Font::from_collection(fixture("fonts/NotoSans-RegularBold.ttc"), 1).unwrap();
    let bold = b.font(bold).unwrap();
    let body = TextStyle::new(&regular, 10.0);
    let strong = TextStyle::new(&bold, 10.0);
    let footer = PageRegion::new().with_center("Page {page} of {pages}");
    let template = PageTemplate::new(PageSize::A4)
        .with_margins(Margins::all(56.0))
        .with_furniture(None, Some(footer), &body.clone().with_size(8.0));
    let flow = Flow::new(template)
        .block(FlowBlock::Heading(
            1,
            Paragraph::plain("Invoice 2026-114", &strong.clone().with_size(18.0)),
        ))
        .block(FlowBlock::Table(invoice_table(&body, &strong)));
    b.flow(&flow).unwrap();
    b.metadata(Metadata::default().with_title("Invoice 2026-114"))
        .lang("en-GB");
    save(b.finish().unwrap())
}
