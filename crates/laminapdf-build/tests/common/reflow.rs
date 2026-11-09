//! The reader-mode corpus (todo U4b): `fixtures/reflow/<name>.pdf` and `<name>.json`. Every
//! reading-order layout (`super::reading_order`) is written tagged, its structure tree in
//! reading order and its blocks drawn last first, so only a reader that follows the tree gets
//! the order right (`<layout>-tagged`). Three layouts are also written with trees reader mode
//! must refuse: in reverse (`-tree-reversed`), shuffled (`-tree-shuffled`), every block a bare
//! `Span` (`-tree-spans`) and every other block left out of the tree (`-tree-partial`). The
//! `mixed` file has the elements reader mode shows as crops or as their own blocks: a figure
//! and a formula with alternate text, a table, a block quote and a list. Each JSON says what
//! reader mode should make of its file: the blocks in reading order, or that it refuses it.
//! `cargo xtask golden --update` rewrites them through `examples/reflow.rs`.

use laminapdf_build::{ArtifactKind, Builder, ElementId, Image, PageSize, Surface, Tag, TagKind};
use laminapdf_model::{Pt, RectPt};
use laminapdf_test_fixtures::Rng;
use serde_json::{Value, json};

use super::reading_order::{Block, Layout, Role, Sample, Script, draw, fonts, layouts, options, words};
use super::save;

/// The layouts written with broken trees too.
const BROKEN: [&str; 3] = ["two-columns", "paper", "three-columns"];

/// How a variant's structure tree relates to its blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tree {
    /// In reading order.
    Reading,
    /// In reverse reading order.
    Reversed,
    /// Shuffled.
    Shuffled,
    /// Every block a `Span` child of the document.
    Spans,
    /// Every other block outside the tree.
    Partial,
}

impl Tree {
    const fn suffix(self) -> &'static str {
        match self {
            Self::Reading => "tagged",
            Self::Reversed => "tree-reversed",
            Self::Shuffled => "tree-shuffled",
            Self::Spans => "tree-spans",
            Self::Partial => "tree-partial",
        }
    }

    /// What reader mode should say about the file: why it refuses it, or nothing.
    const fn refusal(self) -> Option<&'static str> {
        match self {
            Self::Reading => None,
            Self::Reversed | Self::Shuffled => Some("order"),
            Self::Spans => Some("generic"),
            Self::Partial => Some("coverage"),
        }
    }
}

/// The structure type a block's role becomes; `None` for running heads and feet (artifacts).
const fn kind(role: Role, tree: Tree) -> Option<TagKind> {
    if matches!(role, Role::Header | Role::Footer) {
        return None;
    }
    if matches!(tree, Tree::Spans) {
        return Some(TagKind::Span);
    }
    Some(match role {
        Role::Heading => TagKind::H1,
        Role::Paragraph | Role::Header | Role::Footer => TagKind::P,
        Role::Caption => TagKind::Caption,
        Role::Footnote => TagKind::Note,
        Role::TableCell => TagKind::TD,
    })
}

/// The order the tree lists a page's blocks in: indices into `blocks`.
fn tree_order(blocks: &[&Block], tree: Tree, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    match tree {
        Tree::Reversed => order.reverse(),
        Tree::Shuffled => {
            let mut rng = Rng::new(seed);
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i + 1));
            }
        }
        Tree::Reading | Tree::Spans | Tree::Partial => {}
    }
    order
}

/// Opens the elements of one page's blocks in `order`: a table's cells inside `Table` and `TR`
/// elements (a row is the cells at one height), the rest directly. Returns each block's
/// element, `None` for an artifact or a block left out of the tree, and the `Table` and `TR`
/// elements, which are closed with the rest.
fn open(s: &mut Surface, blocks: &[&Block], order: &[usize], tree: Tree) -> (Vec<Option<ElementId>>, Vec<ElementId>) {
    let mut elements = vec![None; blocks.len()];
    let mut groups = Vec::new();
    let mut table: Option<(ElementId, f64, ElementId)> = None;
    for (n, &i) in order.iter().enumerate() {
        let block = blocks[i];
        let Some(kind) = kind(block.role, tree) else { continue };
        if tree == Tree::Partial && n % 2 == 1 {
            continue;
        }
        if kind != TagKind::TD {
            table = None;
            elements[i] = Some(s.open_element(kind));
            continue;
        }
        let top = block.area.1;
        let (outer, row) = match table {
            Some((outer, y, row)) if (y - top).abs() < 1.0 => (outer, row),
            Some((outer, _, _)) => {
                let row = s.resume(outer, |s| Ok(s.open_element(TagKind::TR))).unwrap();
                groups.push(row);
                (outer, row)
            }
            None => {
                let outer = s.open_element(TagKind::Table);
                let row = s.resume(outer, |s| Ok(s.open_element(TagKind::TR))).unwrap();
                groups.extend([outer, row]);
                (outer, row)
            }
        };
        table = Some((outer, top, row));
        elements[i] = Some(s.resume(row, |s| Ok(s.open_element(TagKind::TD))).unwrap());
    }
    (elements, groups)
}

fn build(layout: &Layout, tree: Tree) -> Vec<u8> {
    let mut b = Builder::new(options().with_tagged(true));
    let fonts = fonts(&mut b, layout);
    let mut opened = Vec::new();
    for page_index in 0..layout.pages {
        let mut page = b.page(PageSize::A4);
        page.rotate(layout.rotate);
        let s = page.surface();
        let blocks: Vec<&Block> = layout.blocks.iter().filter(|b| b.page == page_index).collect();
        let order = tree_order(&blocks, tree, 7 + page_index as u64);
        let (elements, groups) = open(s, &blocks, &order, tree);
        opened.extend(groups);
        // Drawn last block first, so the content stream's order is never the answer.
        for (block, element) in blocks.iter().zip(&elements).rev() {
            match (block.role, element) {
                (Role::Header | Role::Footer, _) => {
                    s.artifact(ArtifactKind::Pagination, |s| {
                        draw(s, &fonts, layout, block);
                        Ok(())
                    })
                    .unwrap();
                }
                (_, Some(element)) => s
                    .resume(*element, |s| {
                        draw(s, &fonts, layout, block);
                        Ok(())
                    })
                    .unwrap(),
                (_, None) => draw(s, &fonts, layout, block),
            }
        }
        opened.extend(elements.into_iter().flatten());
        page.finish().unwrap();
    }
    for element in opened {
        b.close_element(element).unwrap();
    }
    save(b.finish().unwrap())
}

fn expected(layout: &Layout, file: &str, tree: Tree) -> String {
    let blocks: Vec<Value> = layout
        .blocks
        .iter()
        .filter(|b| !matches!(b.role, Role::Header | Role::Footer))
        .map(|b| json!({ "page": b.page, "kind": b.role.name(), "text": b.text }))
        .collect();
    let value = match tree.refusal() {
        None => {
            json!({ "file": file, "layout": layout.name, "tree": "in reading order", "available": true, "blocks": blocks })
        }
        Some(reason) => {
            json!({ "file": file, "layout": layout.name, "tree": tree.suffix(), "available": false, "reason": reason })
        }
    };
    serde_json::to_string_pretty(&value).unwrap() + "\n"
}

/// Draws `block` as content of `element`.
fn put(
    s: &mut Surface,
    element: ElementId,
    fonts: &[(Script, laminapdf_build::FontHandle)],
    layout: &Layout,
    block: &Block,
) {
    s.resume(element, |s| {
        draw(s, fonts, layout, block);
        Ok(())
    })
    .unwrap();
}

/// A 2 × 2 RGBA image: the figure's picture.
fn picture() -> Image {
    let pixels = [200, 40, 40, 255, 40, 160, 40, 255, 40, 40, 200, 255, 230, 200, 40, 255];
    Image::rgba(2, 2, &pixels).unwrap()
}

/// The figure's, formula's and table's alternate text.
pub const FIGURE_ALT: &str = "Four coloured squares in a grid";
pub const FORMULA_ALT: &str = "E equals m c squared";
pub const TABLE_ALT: &str = "Two rows of two numbers";

/// The `mixed` file: a heading, a paragraph, a figure and its caption, a formula, a block
/// quote, a two-item list and a two-by-two table, drawn last first.
#[expect(
    clippy::too_many_lines,
    reason = "one file: its blocks, its tree in reading order, then its drawing in reverse"
)]
fn mixed() -> Sample {
    let layout = Layout {
        name: "mixed",
        pages: 1,
        rotate: laminapdf_model::Rotation::None,
        blocks: Vec::new(),
        about: "a figure, a formula, a quote, a list and a table",
    };
    let mut b = Builder::new(options().with_tagged(true));
    let latin = Block {
        page: 0,
        role: Role::Paragraph,
        text: String::new(),
        area: (0.0, 0.0, 0.0, 0.0),
        size: 11.0,
        script: Script::Latin,
        align: laminapdf_build::Alignment::Start,
    };
    let probe = Layout {
        blocks: vec![latin.clone()],
        ..layout
    };
    let fonts = fonts(&mut b, &probe);
    let text = |seed, count, area, size| Block {
        text: words(Script::Latin, seed, count),
        area,
        size,
        ..latin.clone()
    };
    let heading = text(301, 4, (56.0, 56.0, 480.0, 30.0), 20.0);
    let intro = text(302, 30, (56.0, 100.0, 480.0, 60.0), 11.0);
    let caption = text(303, 6, (56.0, 300.0, 480.0, 16.0), 9.0);
    let formula = Block {
        text: "E = mc2".to_owned(),
        ..text(0, 1, (56.0, 330.0, 200.0, 20.0), 14.0)
    };
    let quote = text(304, 20, (90.0, 370.0, 410.0, 40.0), 11.0);
    let items = [
        text(305, 5, (80.0, 430.0, 400.0, 16.0), 11.0),
        text(306, 5, (80.0, 450.0, 400.0, 16.0), 11.0),
    ];
    let cells = [
        text(0, 1, (56.0, 490.0, 100.0, 16.0), 11.0),
        text(0, 1, (160.0, 490.0, 100.0, 16.0), 11.0),
        text(0, 1, (56.0, 510.0, 100.0, 16.0), 11.0),
        text(0, 1, (160.0, 510.0, 100.0, 16.0), 11.0),
    ];
    let cells: Vec<Block> = cells
        .into_iter()
        .zip(["12", "34", "56", "78"])
        .map(|(b, t)| Block {
            text: t.to_owned(),
            ..b
        })
        .collect();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    // The tree, in reading order.
    let h1 = s.open_element(TagKind::H1);
    let p = s.open_element(TagKind::P);
    let figure = s.open_element(Tag::new(TagKind::Figure).with_alt(FIGURE_ALT));
    let cap = s.open_element(TagKind::Caption);
    let math = s.open_element(Tag::custom("Formula", TagKind::Figure).with_alt(FORMULA_ALT));
    let bq = s.open_element(TagKind::BlockQuote);
    let qp = s.resume(bq, |s| Ok(s.open_element(TagKind::P))).unwrap();
    let list = s.open_element(TagKind::L);
    let mut groups = Vec::new();
    let bodies: Vec<ElementId> = (0..2)
        .map(|_| {
            let li = s.resume(list, |s| Ok(s.open_element(TagKind::LI))).unwrap();
            groups.push(li);
            s.resume(li, |s| Ok(s.open_element(TagKind::LBody))).unwrap()
        })
        .collect();
    let table = s.open_element(Tag::new(TagKind::Table).with_alt(TABLE_ALT));
    let tds: Vec<ElementId> = (0..2)
        .flat_map(|_| {
            let tr = s.resume(table, |s| Ok(s.open_element(TagKind::TR))).unwrap();
            groups.push(tr);
            [0, 1].map(|_| s.resume(tr, |s| Ok(s.open_element(TagKind::TD))).unwrap())
        })
        .collect();
    // Drawn last first.
    for (cell, td) in cells.iter().zip(&tds).rev() {
        put(s, *td, &fonts, &layout, cell);
    }
    for (item, body) in items.iter().zip(&bodies).rev() {
        put(s, *body, &fonts, &layout, item);
    }
    put(s, qp, &fonts, &layout, &quote);
    put(s, math, &fonts, &layout, &formula);
    put(s, cap, &fonts, &layout, &caption);
    let area = RectPt::from_xywh(Pt::new(56.0), Pt::new(180.0), Pt::new(160.0), Pt::new(110.0));
    s.resume(figure, |s| {
        s.image(&picture(), area);
        Ok(())
    })
    .unwrap();
    put(s, p, &fonts, &layout, &intro);
    put(s, h1, &fonts, &layout, &heading);
    page.finish().unwrap();
    for element in [h1, p, figure, cap, math, qp, bq, list, table]
        .into_iter()
        .chain(bodies)
        .chain(tds)
        .chain(groups)
    {
        b.close_element(element).unwrap();
    }
    let pdf = save(b.finish().unwrap());
    let blocks = json!([
        { "page": 0, "kind": "heading", "text": heading.text },
        { "page": 0, "kind": "paragraph", "text": intro.text },
        { "page": 0, "kind": "figure", "alt": FIGURE_ALT, "box": [56.0, 180.0, 160.0, 110.0] },
        { "page": 0, "kind": "caption", "text": caption.text },
        { "page": 0, "kind": "formula", "alt": FORMULA_ALT },
        { "page": 0, "kind": "quote", "text": quote.text },
        { "page": 0, "kind": "list-item", "text": items[0].text },
        { "page": 0, "kind": "list-item", "text": items[1].text },
        { "page": 0, "kind": "table", "alt": TABLE_ALT },
    ]);
    let value = json!({ "file": "mixed.pdf", "layout": "mixed", "tree": "in reading order", "available": true, "blocks": blocks });
    Sample {
        name: "mixed".to_owned(),
        pdf,
        json: serde_json::to_string_pretty(&value).unwrap() + "\n",
    }
}

/// Every file of the corpus, in name order.
pub fn corpus() -> Vec<Sample> {
    let mut samples = vec![mixed()];
    for layout in layouts() {
        let trees: &[Tree] = if BROKEN.contains(&layout.name) {
            &[
                Tree::Reading,
                Tree::Reversed,
                Tree::Shuffled,
                Tree::Spans,
                Tree::Partial,
            ]
        } else {
            &[Tree::Reading]
        };
        for &tree in trees {
            let name = format!("{}-{}", layout.name, tree.suffix());
            let file = format!("{name}.pdf");
            samples.push(Sample {
                json: expected(&layout, &file, tree),
                pdf: build(&layout, tree),
                name,
            });
        }
    }
    samples.sort_by(|a, b| a.name.cmp(&b.name));
    samples
}
