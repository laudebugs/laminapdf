//! `fixtures/golden/annots/markup.pdf`: every appearance `AnnotationAppearances` draws, on one
//! A4 page of grey bars standing for lines of text. Row by row from the top: a highlight, an
//! underline, a strike-out and a squiggly line, each over a bar; the same four over a bar
//! turned a quarter; a translucent blue highlight over two lines; the seven note icons; a plain
//! caret and one with a paragraph sign.

use laminapdf_build::{AnnotationAppearances, BuildOptions, Builder, Color, PageSize};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AnnotColor, AnnotSpec, Caret, CaretSymbol, MarkupKind, Note, NoteIcon, Op, Opacity, Origin, TextMarkup,
};

use super::{rect, save};

/// The page's height: user-space y is this minus the page-space y.
const HEIGHT: f64 = 841.89;

const KINDS: [MarkupKind; 4] = [
    MarkupKind::Highlight,
    MarkupKind::Underline,
    MarkupKind::StrikeOut,
    MarkupKind::Squiggly,
];

/// A bar's quad in user space, from its page-space box, in Acrobat's order.
fn quad(x: f64, y: f64, w: f64, h: f64) -> [UserSpacePoint; 4] {
    let p = |x: f64, y: f64| UserSpacePoint::new(x, HEIGHT - y);
    [p(x, y), p(x + w, y), p(x, y + h), p(x + w, y + h)]
}

/// The same bar read top to bottom, as text turned a quarter clockwise runs.
fn turned(x: f64, y: f64, w: f64, h: f64) -> [UserSpacePoint; 4] {
    let p = |x: f64, y: f64| UserSpacePoint::new(x, HEIGHT - y);
    [p(x + w, y), p(x + w, y + h), p(x, y), p(x, y + h)]
}

/// The bars, drawn as the page's content.
fn page_of_bars() -> laminapdf_ops::Document {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_id(*b"lamina-annots-E4")
            .with_producer("lamina-build golden")
            .with_creation_date("D:20261113120000Z".parse().unwrap())
            .with_compress(false),
    );
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    s.fill_color(Color::gray(0.75));
    for (i, _) in KINDS.iter().enumerate() {
        let y = 60.0 + 40.0 * i as f64;
        s.path(|p| {
            p.rect(rect(60.0, y, 200.0, 14.0));
        })
        .fill(laminapdf_model::FillRule::NonZero);
        let x = 320.0 + 40.0 * i as f64;
        s.path(|p| {
            p.rect(rect(x, 60.0, 14.0, 140.0));
        })
        .fill(laminapdf_model::FillRule::NonZero);
    }
    for y in [240.0, 260.0] {
        s.path(|p| {
            p.rect(rect(60.0, y, 200.0, 14.0));
        })
        .fill(laminapdf_model::FillRule::NonZero);
    }
    page.finish().unwrap();
    b.finish().unwrap()
}

/// The marks on the bars.
fn marks(doc: &laminapdf_ops::Document) -> Vec<Op> {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let add = |spec| Op::from(AddAnnot::new(page.clone(), spec));
    let mut ops = Vec::new();
    for (i, kind) in KINDS.into_iter().enumerate() {
        let i = i as f64;
        ops.push(add(AnnotSpec::TextMarkup(TextMarkup::new(
            kind,
            vec![quad(60.0, 60.0 + 40.0 * i, 200.0, 14.0)],
        ))));
        ops.push(add(AnnotSpec::TextMarkup(TextMarkup::new(
            kind,
            vec![turned(320.0 + 40.0 * i, 60.0, 14.0, 140.0)],
        ))));
    }
    let mut translucent = AddAnnot::new(
        page.clone(),
        AnnotSpec::TextMarkup(TextMarkup::new(
            MarkupKind::Highlight,
            vec![quad(60.0, 240.0, 200.0, 14.0), quad(60.0, 260.0, 120.0, 14.0)],
        )),
    );
    translucent.common.color = Some(AnnotColor::Rgb(0.2, 0.5, 1.0));
    translucent.common.opacity = Opacity::new(0.5).unwrap();
    ops.push(translucent.into());
    let icons = [
        NoteIcon::Note,
        NoteIcon::Comment,
        NoteIcon::Key,
        NoteIcon::Help,
        NoteIcon::Paragraph,
        NoteIcon::NewParagraph,
        NoteIcon::Insert,
    ];
    for (i, icon) in icons.into_iter().enumerate() {
        let at = UserSpacePoint::new(60.0 + 40.0 * i as f64, HEIGHT - 320.0);
        ops.push(add(AnnotSpec::Note(Note::new(at, icon))));
    }
    let caret = |x: f64, symbol| {
        let mut caret = Caret::new(UserSpaceRect::from_corners(x, HEIGHT - 400.0, x + 12.0, HEIGHT - 380.0));
        caret.symbol = symbol;
        add(AnnotSpec::Caret(caret))
    };
    ops.push(caret(60.0, CaretSymbol::None));
    ops.push(caret(100.0, CaretSymbol::Paragraph));
    ops
}

pub fn markup() -> Vec<u8> {
    let mut doc = page_of_bars().with_appearances(AnnotationAppearances);
    let ops = marks(&doc);
    doc.transact(Origin::Local, |tx| {
        for op in ops {
            tx.apply(op)?;
        }
        Ok(())
    })
    .unwrap();
    save(doc)
}
