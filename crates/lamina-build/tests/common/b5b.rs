//! The documents behind B5b's goldens: text with embedded and standard 14 fonts, five
//! scripts through `text()` and `text_box()`, links, destinations, outlines, page labels, and
//! metadata. The input text of each page is here, so tests compare what extractors read back
//! with it.

use lamina_build::{
    Alignment, BuildOptions, Builder, Color, Feature, Fit, LabelStyle, LinkTarget, Metadata, OutlineEntry, PageLabel,
    PageLayout, PageMode, PageSize, Standard14, TextBoxOptions, TextRenderMode, ViewerPreferences,
};
use lamina_model::{PointPt, Pt, RectPt};

use super::{fixture, save};

fn options(id: &[u8; 16]) -> BuildOptions {
    BuildOptions::default()
        .with_id(*id)
        .with_producer("lamina-build golden")
        .with_creation_date("D:20261029120000Z".parse().unwrap())
        .with_compress(false)
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

fn at(x: f64, y: f64) -> PointPt {
    PointPt::from_points(x, y)
}

pub fn font(name: &str) -> lamina_text::Font {
    lamina_text::Font::from_bytes(fixture(&format!("fonts/{name}"))).unwrap()
}

/// `build-text-latin.pdf`'s single lines, top to bottom (`text()`): Noto Sans, then the
/// standard 14.
pub const LATIN_RUNS: [&str; 5] = [
    "Hello, world! The office staff fill the affine waffle flask.",
    "Café crème brûlée, naïve façade: “curly” and ‘single’ quotes",
    "AVATAR Wave: To Yo! (kerned) and fi without ligatures",
    "Helvetica: “Curly” ‘quotes’ — and dashes – too",
    "Times-Roman: The quick brown fox jumps over the lazy dog.",
];

/// The paragraph `build-text-latin.pdf` breaks into a text box, three times (start, centre,
/// end alignment), and once in Times.
pub const LATIN_PARAGRAPH: &str = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs; \
     sphinx of black quartz, judge my vow.";

/// `fixtures/golden/build-text-latin.pdf`: `text()` runs with an embedded font (one with
/// ligatures and kerning on, one with `liga=0`), a coloured run, an invisible run, the
/// standard 14 Helvetica and Times, and `text_box()` paragraphs in three alignments.
pub fn text_latin() -> Vec<u8> {
    let mut b = Builder::new(options(b"lamina-build-B5b"));
    let noto = b.font(font("NotoSans-Regular.ttf")).unwrap();
    let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
    let times = b.standard_font(Standard14::TimesRoman).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let size = Pt::new(12.0);
    let _ = s.text(LATIN_RUNS[0], &noto, size, at(56.0, 72.0)).unwrap();
    let _ = s
        .text(LATIN_RUNS[1], &noto, size, at(56.0, 92.0))
        .unwrap()
        .fill(Color::rgb(0.1, 0.3, 0.7));
    let _ = s
        .text(LATIN_RUNS[2], &noto, size, at(56.0, 112.0))
        .unwrap()
        .features(&["liga=0".parse::<Feature>().unwrap()])
        .unwrap();
    let _ = s.text(LATIN_RUNS[3], &helvetica, size, at(56.0, 132.0)).unwrap();
    let _ = s.text(LATIN_RUNS[4], &times, size, at(56.0, 152.0)).unwrap();
    // An invisible run (OCR-style text layer) right below the others, and a stroked one.
    let _ = s
        .text("Invisible but extractable", &noto, size, at(56.0, 172.0))
        .unwrap()
        .render_mode(TextRenderMode::Invisible);
    let _ = s
        .stroke_color(Color::gray(0.2))
        .text("Outlined", &helvetica, Pt::new(24.0), at(56.0, 204.0))
        .unwrap()
        .render_mode(TextRenderMode::Stroke);
    let boxes = [
        (Alignment::Start, 56.0, &noto),
        (Alignment::Center, 226.0, &noto),
        (Alignment::End, 396.0, &noto),
    ];
    for (align, x, f) in boxes {
        let r = rect(x, 240.0, 150.0, 200.0);
        s.stroke_color(Color::gray(0.7))
            .path(|p| {
                p.rect(r);
            })
            .stroke(&lamina_build::StrokeStyle::default());
        s.text_box(
            LATIN_PARAGRAPH,
            f,
            Pt::new(11.0),
            r,
            &TextBoxOptions::default().with_align(align),
        )
        .unwrap();
    }
    s.text_box(
        LATIN_PARAGRAPH,
        &times,
        Pt::new(11.0),
        rect(56.0, 460.0, 480.0, 100.0),
        &TextBoxOptions::default().with_line_height(1.5),
    )
    .unwrap();
    page.finish().unwrap();
    save(b.finish().unwrap())
}

/// `build-text-multiscript.pdf`: each script's text drawn with `text()` (one line) and in a
/// narrow `text_box()` (several lines): the font file and the text.
pub const SCRIPTS: [(&str, &str); 5] = [
    (
        "NotoSans-Regular.ttf",
        "Latin: the office staff fill the affine waffle flask",
    ),
    ("NotoSansArabic-Regular.ttf", "مرحبا بالعالم السلام عليكم"),
    ("NotoSansDevanagari-Regular.ttf", "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण"),
    ("NotoSansHebrew-Regular.ttf", "שלום עולם וברוכים הבאים"),
    ("NotoSansSC-Regular.otf", "你好世界，这是中文字体子集测试。"),
];

/// The width of the multiscript text boxes, points.
pub const MULTISCRIPT_BOX_WIDTH: f64 = 110.0;

/// `fixtures/golden/build-text-multiscript.pdf`: page 1 has every script with `text()`,
/// page 2 the same texts in narrow text boxes.
pub fn text_multiscript() -> Vec<u8> {
    let mut b = Builder::new(options(b"lamina-build-5sc"));
    let fonts: Vec<_> = SCRIPTS.iter().map(|(f, _)| b.font(font(f)).unwrap()).collect();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    for (i, ((_, text), f)) in SCRIPTS.iter().zip(&fonts).enumerate() {
        let y = 72.0 + 28.0 * f64::from(u32::try_from(i).unwrap());
        let _ = s.text(text, f, Pt::new(14.0), at(56.0, y)).unwrap();
    }
    page.finish().unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    for (i, ((_, text), f)) in SCRIPTS.iter().zip(&fonts).enumerate() {
        let y = 56.0 + 150.0 * f64::from(u32::try_from(i).unwrap());
        s.text_box(
            text,
            f,
            Pt::new(14.0),
            rect(56.0, y, MULTISCRIPT_BOX_WIDTH, 140.0),
            &TextBoxOptions::default(),
        )
        .unwrap();
    }
    page.finish().unwrap();
    save(b.finish().unwrap())
}

/// `links-and-outlines.pdf`'s page labels: i, ii, then 1, 2 from the third page.
pub const LABELS: [&str; 4] = ["i", "ii", "1", "2"];

/// The URI of the golden's external link.
pub const URI: &str = "https://example.com/lamina?q=links&lang=en";

/// `fixtures/golden/links-and-outlines.pdf`: four pages; page 1 has a URI link, links to
/// pages 3 and 4 (forward) and a link through a named destination; a text run that is a
/// link; named destinations; a nested outline with an open chapter, colour and styles; page
/// labels i, ii, 1, 2.
pub fn links_and_outlines() -> Vec<u8> {
    let mut b = Builder::new(options(b"lamina-build-nav"));
    let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
    b.destination(
        "chapter-2",
        3,
        Fit::Xyz {
            left: Some(Pt::new(0.0)),
            top: Some(Pt::new(100.0)),
            zoom: None,
        },
    )
    .destination("contents", 1, Fit::Page)
    .destination(
        "appendix",
        3,
        Fit::Width {
            top: Some(Pt::new(72.0)),
        },
    );
    let titles = ["Title page", "Contents", "Chapter 1", "Chapter 2"];
    for (i, title) in titles.iter().enumerate() {
        let mut page = b.page(PageSize::A5);
        let s = page.surface();
        let _ = s.text(title, &helvetica, Pt::new(18.0), at(36.0, 60.0)).unwrap();
        if i == 0 {
            s.link(
                rect(36.0, 80.0, 200.0, 20.0),
                LinkTarget::Uri(URI.into()),
                Some("Example".into()),
            );
            let _ = s
                .text("Visit example.com", &helvetica, Pt::new(12.0), at(36.0, 95.0))
                .unwrap();
            s.link(
                rect(36.0, 110.0, 200.0, 20.0),
                LinkTarget::Page {
                    index: 2,
                    fit: Fit::Page,
                },
                None,
            );
            let _ = s
                .text("Go to chapter 1", &helvetica, Pt::new(12.0), at(36.0, 125.0))
                .unwrap();
            s.link(
                rect(36.0, 140.0, 200.0, 20.0),
                LinkTarget::Named("chapter-2".into()),
                None,
            );
            let _ = s
                .text("Go to chapter 2 (named)", &helvetica, Pt::new(12.0), at(36.0, 155.0))
                .unwrap();
            let _ = s
                .text("Contents (a linked run)", &helvetica, Pt::new(12.0), at(36.0, 185.0))
                .unwrap()
                .fill(Color::rgb(0.0, 0.2, 0.8))
                .link(LinkTarget::page(1), Some("Contents".into()));
        }
        page.finish().unwrap();
    }
    b.outline([
        OutlineEntry::new("Title page", LinkTarget::page(0)),
        OutlineEntry::new("Contents", LinkTarget::Named("contents".into())).with_color(0.0, 0.0, 0.6),
        OutlineEntry::heading("Chapters")
            .open(true)
            .bold()
            .with_child(OutlineEntry::new("Chapter 1", LinkTarget::page(2)))
            .with_child(
                OutlineEntry::new("Chapter 2", LinkTarget::Named("chapter-2".into()))
                    .with_child(OutlineEntry::new("Appendix", LinkTarget::Named("appendix".into())).italic()),
            ),
        OutlineEntry::new("Website", LinkTarget::Uri(URI.into())),
    ])
    .unwrap();
    b.page_labels([
        PageLabel::new(0, Some(LabelStyle::LowerRoman)),
        PageLabel::new(2, Some(LabelStyle::Decimal)),
    ]);
    b.viewer_preferences(ViewerPreferences::default().with_page_mode(PageMode::Outlines));
    save(b.finish().unwrap())
}

/// `metadata.pdf`'s values.
pub fn golden_metadata() -> Metadata {
    Metadata::default()
        .with_title("Metadata golden: Info & XMP <in sync>")
        .with_author("Ada Lovelace")
        .with_subject("Info and XMP carry the same values")
        .with_keywords("pdf, xmp, info, lamina")
        .with_creator("lamina-build tests")
        .with_producer("lamina-build golden")
        .with_creation_date("D:20261029120000Z".parse().unwrap())
        .with_mod_date("D:20261030083000+01'00'".parse().unwrap())
}

/// `fixtures/golden/metadata.pdf`: one page, the full Info dictionary and an XMP packet from
/// the same values, `/Lang`, `DisplayDocTitle`, page layout and mode.
pub fn metadata() -> Vec<u8> {
    let mut b = Builder::new(options(b"lamina-build-xmp"));
    let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A5);
    let _ = page
        .surface()
        .text("Metadata", &helvetica, Pt::new(18.0), at(36.0, 60.0))
        .unwrap();
    page.finish().unwrap();
    b.metadata(golden_metadata()).lang("en-GB").viewer_preferences(
        ViewerPreferences::default()
            .with_display_doc_title(true)
            .with_page_layout(PageLayout::OneColumn)
            .with_page_mode(PageMode::None),
    );
    save(b.finish().unwrap())
}
