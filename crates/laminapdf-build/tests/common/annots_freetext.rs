//! `fixtures/golden/annots/freetext.pdf`: text boxes `AnnotationAppearances` draws. Page 1, from
//! the top: a plain Helvetica box over two paragraphs; a centred Times box with bold and italic,
//! filled; a right-aligned Courier box with a dashed border; a callout with a knee and an open
//! arrow, and one with a closed arrow; typewriter text; a cloudy box; a Noto Sans box (a font
//! given by its bytes) with bold and italic drawn bolder and slanted; an Arabic box; a Helvetica
//! box whose Chinese characters fall back to Noto Sans SC. Page 2 is turned 90° and holds a box
//! upright as shown.

use laminapdf_build::{AnnotationAppearances, BuildOptions, Builder, PageSize, Rotation};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AnnotColor, AnnotSpec, Border, Callout, FontRef, FreeText, FreeTextIntent, LineEnding, Op, Origin,
    RichText, StandardFamily, TextAlign, TextSpan,
};
use laminapdf_test_fixtures::fixture;

use super::save;

/// The page's height: user-space y is this minus the page-space y.
const HEIGHT: f64 = 841.89;

fn at(x: f64, y: f64) -> UserSpacePoint {
    UserSpacePoint::new(x, HEIGHT - y)
}

/// A box in user space from a page-space box's left, top, width and height.
fn area(left: f64, top: f64, width: f64, height: f64) -> UserSpaceRect {
    UserSpaceRect::from_corners(left, HEIGHT - top - height, left + width, HEIGHT - top)
}

fn two_pages() -> laminapdf_ops::Document {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_id(*b"lamina-annots-4e")
            .with_producer("lamina-build golden")
            .with_creation_date("D:20261116120000Z".parse().unwrap())
            .with_compress(false),
    );
    b.page(PageSize::A4).finish().unwrap();
    let mut turned = b.page(PageSize::A4);
    turned.rotate(Rotation::Clockwise90);
    turned.finish().unwrap();
    b.finish().unwrap()
}

fn standard() -> Vec<AnnotSpec> {
    let rich = RichText {
        paragraphs: vec![
            vec![
                TextSpan::plain("Q2 sales are "),
                TextSpan::plain("late").with_bold(true),
                TextSpan::plain(", see "),
                TextSpan::plain("the appendix").with_italic(true),
                TextSpan::plain("."),
            ],
            vec![
                TextSpan::plain("Both styles: "),
                TextSpan::plain("noted").with_bold(true).with_italic(true),
            ],
        ],
    };
    vec![
        AnnotSpec::FreeText(FreeText::new(
            area(60.0, 50.0, 220.0, 70.0),
            RichText::plain(
                "A plain box in Helvetica wraps its words at the box's width.\nA second paragraph starts on its own line.",
            ),
        )),
        AnnotSpec::FreeText(
            FreeText::new(area(300.0, 50.0, 230.0, 70.0), rich)
                .with_font(FontRef::Standard(StandardFamily::Times), 13.0)
                .with_align(TextAlign::Center)
                .with_fill(Some(AnnotColor::Rgb(1.0, 0.98, 0.8))),
        ),
        AnnotSpec::FreeText(
            FreeText::new(
                area(60.0, 140.0, 220.0, 40.0),
                RichText::plain("Right-aligned Courier, €12.50"),
            )
            .with_font(FontRef::Standard(StandardFamily::Courier), 11.0)
            .with_align(TextAlign::Right)
            .with_border(Border::dashed(1.0, vec![4.0, 2.0])),
        ),
        AnnotSpec::FreeText(
            FreeText::new(
                area(380.0, 160.0, 150.0, 34.0),
                RichText::plain("A callout with a knee"),
            )
            .with_intent(FreeTextIntent::Callout(Callout::new(vec![
                at(300.0, 250.0),
                at(340.0, 177.0),
                at(380.0, 177.0),
            ]))),
        ),
        AnnotSpec::FreeText(
            FreeText::new(area(380.0, 220.0, 150.0, 30.0), RichText::plain("Closed arrow"))
                .with_intent(FreeTextIntent::Callout(
                    Callout::new(vec![at(320.0, 290.0), at(380.0, 235.0)]).with_end(LineEnding::ClosedArrow),
                ))
                .with_border(Border::solid(2.0)),
        ),
        AnnotSpec::FreeText(
            FreeText::new(
                area(60.0, 200.0, 220.0, 24.0),
                RichText::plain("Jane Doe, 12 Main Street"),
            )
            .with_intent(FreeTextIntent::Typewriter)
            .with_font(FontRef::Standard(StandardFamily::Courier), 12.0),
        ),
        AnnotSpec::FreeText(
            FreeText::new(area(70.0, 260.0, 200.0, 50.0), RichText::plain("A cloudy box"))
                .with_border(Border::cloudy(1.0, 1.0))
                .with_fill(Some(AnnotColor::Rgb(0.85, 0.93, 1.0)))
                .with_align(TextAlign::Center),
        ),
    ]
}

/// Boxes in fonts given by their bytes: Latin with synthetic styles, Arabic, and Helvetica with
/// a Chinese fallback.
fn embedded(doc: &mut laminapdf_ops::Document) -> Vec<AnnotSpec> {
    let sans = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    let arabic = doc.add_blob(fixture("fonts/NotoSansArabic-Regular.ttf"));
    let chinese = doc.add_blob(fixture("fonts/NotoSansSC-Regular.otf"));
    let styled = RichText {
        paragraphs: vec![vec![
            TextSpan::plain("Noto Sans, "),
            TextSpan::plain("bolder").with_bold(true),
            TextSpan::plain(" and "),
            TextSpan::plain("slanted").with_italic(true),
            TextSpan::plain(": ffi fl"),
        ]],
    };
    vec![
        AnnotSpec::FreeText(FreeText::new(area(60.0, 340.0, 230.0, 46.0), styled).with_font(FontRef::Font(sans), 12.0)),
        AnnotSpec::FreeText(
            FreeText::new(
                area(310.0, 340.0, 220.0, 46.0),
                RichText::plain("مرحبا بالعالم، هذا نص عربي"),
            )
            .with_font(FontRef::Font(arabic), 14.0)
            .with_align(TextAlign::Right),
        ),
        AnnotSpec::FreeText(
            FreeText::new(area(60.0, 410.0, 230.0, 46.0), RichText::plain("Hello, 你好世界"))
                .with_fallbacks(vec![chinese])
                .with_font(FontRef::Standard(StandardFamily::Helvetica), 14.0),
        ),
    ]
}

/// A box on the turned page, upright as the page is shown: 300 pt wide on screen.
fn turned() -> AnnotSpec {
    AnnotSpec::FreeText(
        FreeText::new(
            UserSpaceRect::from_corners(400.0, 200.0, 440.0, 500.0),
            RichText::plain("Upright on a page turned a quarter"),
        )
        .with_rotation(90)
        .with_fill(Some(AnnotColor::Gray(0.95))),
    )
}

pub fn freetext() -> Vec<u8> {
    let mut doc = two_pages().with_appearances(AnnotationAppearances);
    let pages = doc.pages().unwrap();
    let (first, second) = (pages.get(0).unwrap().id().clone(), pages.get(1).unwrap().id().clone());
    let mut ops: Vec<Op> = standard()
        .into_iter()
        .chain(embedded(&mut doc))
        .map(|spec| Op::from(AddAnnot::new(first.clone(), spec)))
        .collect();
    ops.push(AddAnnot::new(second, turned()).into());
    doc.transact(Origin::Local, |tx| {
        for op in ops {
            tx.apply(op)?;
        }
        Ok(())
    })
    .unwrap();
    save(doc)
}
