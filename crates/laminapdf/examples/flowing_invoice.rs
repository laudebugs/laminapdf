//! An invoice whose table runs over three pages: its header row repeats at the top of each
//! page, no row is split between pages, and the total stays on the page of the last row.
//!
//! The columns fit their widest cell, and numbers line up on the right. The output is tagged
//! (one `Table` element across the pages) and checked against PDF/UA-1. Run it with
//! `cargo run -p laminapdf --example flowing_invoice -- regular.ttf bold.ttc`.

use std::error::Error;

use laminapdf::flow::{
    ColumnWidth, Flow, FlowBlock, Margins, PageRegion, PageTemplate, Paragraph, Table, TableCell, TableRow, TableRule,
    TextStyle,
};
use laminapdf::{Alignment, BuildOptions, Builder, Color, Font, Metadata, PageSize, Profile, SaveOptions};

const TITLE: &str = "Invoice 2026-114";
const PRODUCTS: [(&str, usize); 6] = [
    ("Paper, A4, 80 g, box of 2,500 sheets", 2_450),
    ("Toner cartridge, black", 6_900),
    ("Desk lamp with a weighted base and an adjustable arm", 4_275),
    ("Stapler", 1_290),
    ("Archive boxes, set of ten, with lids", 3_150),
    ("Ballpoint pens, box of fifty", 1_875),
];

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [regular, bold] = paths.as_slice() else {
        return Err("arguments: <regular.ttf> <bold.ttc>".into());
    };
    let pdf = build(BuildOptions::new(), std::fs::read(regular)?, std::fs::read(bold)?)?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/flowing_invoice.pdf", pdf)?)
}

/// The invoice's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions, regular: Vec<u8>, bold: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options.with_profile(Profile::PdfUa1));
    b.metadata(Metadata::default().with_title(TITLE));
    b.lang("en");
    let body = TextStyle::new(&b.font(Font::from_bytes(regular)?)?, 10.0);
    let strong = TextStyle::new(&b.font(Font::from_collection(bold, 1)?)?, 10.0);
    let footer = PageRegion::new().with_center("{title}, page {page} of {pages}");
    let pages = PageTemplate::new(PageSize::A4)
        .with_margins(Margins::all(56.0))
        .with_furniture(None, Some(footer), &body.clone().with_size(8.0));
    let flow = Flow::new(pages)
        .with_title(TITLE)
        .block(FlowBlock::Heading(
            1,
            Paragraph::plain(TITLE, &strong.clone().with_size(18.0)),
        ))
        .block(FlowBlock::Paragraph(Paragraph::plain(
            "Office supplies delivered in October, to be paid within 30 days.",
            &body,
        )))
        .block(FlowBlock::Table(table(&body, &strong)));
    b.flow(&flow)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// Sixty lines of products, a header row and a footer row with the total.
fn table(body: &TextStyle, strong: &TextStyle) -> Table {
    let number = |text: String, style: &TextStyle| TableCell::plain(text, style).with_align(Alignment::End);
    let header = ["Item", "Description", "Qty", "Amount"]
        .iter()
        .enumerate()
        .fold(TableRow::new(), |row, (i, text)| {
            let cell = TableCell::plain(*text, strong);
            row.cell(if i >= 2 { cell.with_align(Alignment::End) } else { cell })
        });
    let mut table = Table::new([ColumnWidth::Auto; 4])
        .with_padding(6.0)
        .with_frame(TableRule::new(0.75, Color::rgb8(0x66, 0x66, 0x66)))
        .with_header_rule(TableRule::new(0.75, Color::rgb8(0x66, 0x66, 0x66)))
        .with_row_rules(TableRule::new(0.25, Color::rgb8(0xb3, 0xb3, 0xb3)))
        .with_header_background(Color::rgb8(0xeb, 0xeb, 0xeb))
        .header(header);
    let mut total = 0;
    for (i, (name, cents)) in PRODUCTS.iter().cycle().take(60).enumerate() {
        let qty = i % 5 + 1;
        total += qty * cents;
        table = table.row(
            TableRow::new()
                .cell(TableCell::plain(format!("{:03}", i + 1), body))
                .cell(TableCell::plain(*name, body))
                .cell(number(qty.to_string(), body))
                .cell(number(money(qty * cents), body)),
        );
    }
    let total_row = TableRow::new()
        .cell(TableCell::plain("Total", strong).with_span(3))
        .cell(number(money(total), strong));
    table.footer(total_row)
}

fn money(cents: usize) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}
