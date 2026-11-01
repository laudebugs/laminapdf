//! PDF/UA-1: each machine-checkable rule lamina's builder could break fails
//! `Builder::finish` with its coded error, one test per rule (the Matterhorn Protocol 1.1
//! failure condition is in each test's comment), and a conforming document carries what the
//! standard asks for: the XMP identification, `dc:title`, `/DisplayDocTitle`, `/MarkInfo`, the
//! structure tree and `/Tabs /S`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::b5b::font;
use common::{reopen, save};
use lamina_build::{
    ArtifactKind, BuildOptions, Builder, ContentKind, Error, FillRule, LinkTarget, Metadata, PageSize, PdfVersion,
    Profile, Standard14, Tag, TagKind,
};
use lamina_cos::Object;
use lamina_model::{PointPt, Pt, RectPt};

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

fn at(x: f64, y: f64) -> PointPt {
    PointPt::from_points(x, y)
}

fn options() -> BuildOptions {
    BuildOptions::default().with_id([3; 16]).with_profile(Profile::PdfUa1)
}

/// A PDF/UA-1 builder with a title and a language, and one page drawn by `draw` with Noto Sans.
fn ua(draw: impl FnOnce(&mut lamina_build::Surface, &lamina_build::Font) -> lamina_build::Result<()>) -> Builder {
    ua_with(options(), draw)
}

fn ua_with(
    options: BuildOptions,
    draw: impl FnOnce(&mut lamina_build::Surface, &lamina_build::Font) -> lamina_build::Result<()>,
) -> Builder {
    let mut b = Builder::new(options);
    b.metadata(Metadata::default().with_title("A report")).lang("en-GB");
    let f = b.font(font("NotoSans-Regular.ttf")).unwrap();
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &f).unwrap();
    page.finish().unwrap();
    b
}

fn paragraph(s: &mut lamina_build::Surface, f: &lamina_build::Font) -> lamina_build::Result<()> {
    let _ = s.text("Hello", f, Pt::new(12.0), at(56.0, 80.0))?.tag(TagKind::P);
    Ok(())
}

fn finish_error(b: Builder) -> Error {
    b.finish().unwrap_err()
}

mod rules {
    use super::*;

    #[test]
    fn a_document_that_meets_every_rule_builds() {
        assert!(ua(paragraph).finish().is_ok());
    }

    /// Matterhorn 06-003: no `dc:title`.
    #[test]
    fn a_missing_title_fails_with_missing_title() {
        let mut b = Builder::new(options());
        b.lang("en-GB");
        b.page(PageSize::A4).finish().unwrap();
        assert_eq!(finish_error(b), Error::MissingTitle);
    }

    #[test]
    fn a_blank_title_fails_with_missing_title() {
        let mut b = Builder::new(options());
        b.metadata(Metadata::default().with_title("  ")).lang("en-GB");
        assert_eq!(finish_error(b), Error::MissingTitle);
    }

    /// Matterhorn 11-006 (and 11-001: the page content's language comes from the catalog).
    #[test]
    fn a_missing_language_fails_with_missing_lang() {
        let mut b = Builder::new(options());
        b.metadata(Metadata::default().with_title("A report"));
        assert_eq!(finish_error(b), Error::MissingLang);
    }

    /// Matterhorn 13-004: a `Figure` without alternate text.
    #[test]
    fn a_figure_without_alt_text_fails_with_missing_alt_text() {
        let b = ua(|s, _| {
            s.tag(TagKind::Figure, |s| {
                s.path(|p| {
                    p.rect(rect(56.0, 56.0, 10.0, 10.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
        });
        assert_eq!(finish_error(b), Error::MissingAltText { page: 0 });
    }

    #[test]
    fn a_figure_with_alt_text_builds() {
        let b = ua(|s, _| {
            s.tag(Tag::new(TagKind::Figure).with_alt("A blue square"), |s| {
                s.path(|p| {
                    p.rect(rect(56.0, 56.0, 10.0, 10.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
        });
        assert!(b.finish().is_ok());
    }

    /// Matterhorn 01-005: content neither an artifact nor tagged (an untagged path).
    #[test]
    fn an_untagged_path_fails_with_untagged_content() {
        let b = ua(|s, f| {
            paragraph(s, f)?;
            s.path(|p| {
                p.rect(rect(56.0, 100.0, 100.0, 1.0));
            })
            .fill(FillRule::NonZero);
            Ok(())
        });
        assert_eq!(
            finish_error(b),
            Error::UntaggedContent {
                page: 0,
                content: ContentKind::Path,
                count: 1
            }
        );
    }

    #[test]
    fn an_untagged_text_run_fails_with_untagged_content() {
        let b = ua(|s, f| s.text("Loose", f, Pt::new(12.0), at(56.0, 80.0)).map(|_| ()));
        assert_eq!(
            finish_error(b),
            Error::UntaggedContent {
                page: 0,
                content: ContentKind::Text,
                count: 1
            }
        );
    }

    #[test]
    fn a_path_drawn_as_an_artifact_builds() {
        let b = ua(|s, f| {
            paragraph(s, f)?;
            s.artifact(ArtifactKind::Layout, |s| {
                s.path(|p| {
                    p.rect(rect(56.0, 100.0, 100.0, 1.0));
                })
                .fill(FillRule::NonZero);
                Ok(())
            })
        });
        assert!(b.finish().is_ok());
    }

    /// Matterhorn 14-002: the first numbered heading isn't `H1`.
    #[test]
    fn a_first_heading_below_h1_fails_with_heading_level_skipped() {
        let b = ua(|s, f| {
            s.text("Intro", f, Pt::new(14.0), at(56.0, 80.0))
                .map(|r| drop(r.tag(TagKind::H2)))
        });
        assert_eq!(
            finish_error(b),
            Error::HeadingLevelSkipped {
                page: 0,
                level: 2,
                previous: 0
            }
        );
    }

    /// Matterhorn 14-003: an `H3` straight after an `H1`.
    #[test]
    fn a_skipped_heading_level_fails_with_heading_level_skipped() {
        let b = ua(|s, f| {
            let _ = s.text("Title", f, Pt::new(20.0), at(56.0, 80.0))?.tag(TagKind::H1);
            let _ = s.text("Detail", f, Pt::new(12.0), at(56.0, 120.0))?.tag(TagKind::H3);
            Ok(())
        });
        assert_eq!(
            finish_error(b),
            Error::HeadingLevelSkipped {
                page: 0,
                level: 3,
                previous: 1
            }
        );
    }

    #[test]
    fn going_back_up_any_number_of_heading_levels_builds() {
        let b = ua(|s, f| {
            for (i, kind) in [TagKind::H1, TagKind::H2, TagKind::H3, TagKind::H1]
                .into_iter()
                .enumerate()
            {
                let y = 80.0 + 20.0 * f64::from(u8::try_from(i).unwrap());
                let _ = s.text("Heading", f, Pt::new(12.0), at(56.0, y))?.tag(kind);
            }
            Ok(())
        });
        assert!(b.finish().is_ok());
    }

    /// Matterhorn 09-004: a table cell outside a row.
    #[test]
    fn a_cell_directly_in_a_table_fails_with_invalid_structure_nesting() {
        let b = ua(|s, f| {
            s.tag(TagKind::Table, |s| {
                let _ = s.text("Cell", f, Pt::new(12.0), at(56.0, 80.0))?.tag(TagKind::TD);
                Ok(())
            })
        });
        assert_eq!(
            finish_error(b),
            Error::InvalidStructureNesting {
                page: 0,
                kind: "TD".into(),
                parent: "Table".into()
            }
        );
    }

    /// Matterhorn 09-005: a list item's content outside an `LBody`.
    #[test]
    fn a_paragraph_directly_in_a_list_item_fails_with_invalid_structure_nesting() {
        let b = ua(|s, f| {
            s.tag(TagKind::L, |s| {
                s.tag(TagKind::LI, |s| {
                    let _ = s.text("Item", f, Pt::new(12.0), at(56.0, 80.0))?.tag(TagKind::P);
                    Ok(())
                })
            })
        });
        assert_eq!(
            finish_error(b),
            Error::InvalidStructureNesting {
                page: 0,
                kind: "P".into(),
                parent: "LI".into()
            }
        );
    }

    #[test]
    fn a_table_with_header_groups_builds() {
        let b = ua(|s, f| {
            s.tag(TagKind::Table, |s| {
                s.tag(Tag::custom("TBody", TagKind::Part), |s| {
                    s.tag(TagKind::TR, |s| {
                        let _ = s.text("Cell", f, Pt::new(12.0), at(56.0, 80.0))?.tag(TagKind::TD);
                        Ok(())
                    })
                })
            })
        });
        assert!(b.finish().is_ok());
    }

    /// Matterhorn 28-011: a link annotation outside a `Link` element (drawn in an artifact).
    #[test]
    fn a_link_inside_an_artifact_fails_with_link_without_structure() {
        let b = ua(|s, f| {
            paragraph(s, f)?;
            s.artifact(ArtifactKind::Layout, |s| {
                s.link(
                    rect(56.0, 200.0, 50.0, 10.0),
                    LinkTarget::Uri("https://example.com".into()),
                    Some("Example".into()),
                );
                Ok(())
            })
        });
        assert_eq!(finish_error(b), Error::LinkWithoutStructure { page: 0, count: 1 });
    }

    /// Matterhorn 28-012: a link without an alternate description (`/Contents`).
    #[test]
    fn a_link_without_contents_fails_with_link_without_contents() {
        let b = ua(|s, f| {
            paragraph(s, f)?;
            s.link(
                rect(56.0, 200.0, 50.0, 10.0),
                LinkTarget::Uri("https://example.com".into()),
                None,
            );
            Ok(())
        });
        assert_eq!(finish_error(b), Error::LinkWithoutContents { page: 0 });
    }

    /// Matterhorn 31-009: a font program that isn't embedded (a standard 14 font).
    #[test]
    fn a_standard_14_font_fails_with_font_not_embedded() {
        let mut b = Builder::new(options());
        b.metadata(Metadata::default().with_title("A report")).lang("en-GB");
        let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
        let mut page = b.page(PageSize::A4);
        let _ = page
            .surface()
            .text("Hello", &helvetica, Pt::new(12.0), at(56.0, 80.0))
            .unwrap()
            .tag(TagKind::P);
        page.finish().unwrap();
        assert_eq!(
            finish_error(b),
            Error::FontNotEmbedded {
                profile: Profile::PdfUa1,
                font: "Helvetica".into()
            }
        );
    }

    #[test]
    fn a_standard_14_font_registered_but_never_drawn_builds() {
        let mut b = ua(paragraph);
        b.standard_font(Standard14::Helvetica).unwrap();
        assert!(b.finish().is_ok());
    }

    /// Matterhorn 31-027 to 31-029: a glyph without a usable Unicode value (the beh's dot,
    /// which Noto Sans Arabic draws as a glyph of its own that stands for no text).
    #[test]
    fn a_glyph_without_unicode_fails_with_glyph_without_unicode() {
        let mut b = Builder::new(options());
        b.metadata(Metadata::default().with_title("A report")).lang("ar");
        let f = b.font(font("NotoSansArabic-Regular.ttf")).unwrap();
        let mut page = b.page(PageSize::A4);
        let _ = page
            .surface()
            .text("بالعالم", &f, Pt::new(12.0), at(56.0, 80.0))
            .unwrap()
            .tag(TagKind::P);
        page.finish().unwrap();
        assert!(matches!(
            finish_error(b),
            Error::GlyphWithoutUnicode {
                profile: Profile::PdfUa1,
                ..
            }
        ));
    }

    #[test]
    fn pdf_2_0_fails_with_profile_version_conflict() {
        let b = ua_with(options().with_version(PdfVersion::V2_0), paragraph);
        assert_eq!(
            finish_error(b),
            Error::ProfileVersionConflict {
                profile: Profile::PdfUa1
            }
        );
    }

    #[test]
    fn the_first_broken_rule_is_reported_and_nothing_is_written() {
        // Both the title and the language are missing: the title is checked first.
        let b = Builder::new(options());
        assert_eq!(finish_error(b), Error::MissingTitle);
    }
}

mod output {
    use super::*;

    fn built() -> lamina_model::Document {
        reopen(&save(ua(paragraph).finish().unwrap()))
    }

    fn xmp(doc: &lamina_model::Document) -> String {
        let r = doc.catalog().unwrap().get("Metadata").and_then(Object::as_ref).unwrap();
        let Object::Stream(s) = &*doc.resolve(r).unwrap() else {
            panic!("not a stream")
        };
        String::from_utf8(doc.decode_stream(s).unwrap()).unwrap()
    }

    #[test]
    fn pdf_ua_makes_the_document_tagged() {
        assert!(BuildOptions::default().with_profile(Profile::PdfUa1).tagged());
    }

    /// Matterhorn 06-001, 06-002: an XMP packet with the PDF/UA identifier.
    #[test]
    fn the_xmp_packet_identifies_pdf_ua_1() {
        assert!(xmp(&built()).contains("<pdfuaid:part>1</pdfuaid:part>"));
    }

    /// Matterhorn 06-003: `dc:title`.
    #[test]
    fn the_xmp_packet_has_the_title() {
        assert!(xmp(&built()).contains("<rdf:li xml:lang=\"x-default\">A report</rdf:li>"));
    }

    /// Matterhorn 07-001, 07-002: `/DisplayDocTitle true`, without being asked.
    #[test]
    fn the_viewer_shows_the_title() {
        let cat = built().catalog().unwrap();
        let prefs = cat.get("ViewerPreferences").and_then(Object::as_dict).unwrap();
        assert_eq!(prefs.get("DisplayDocTitle"), Some(&Object::Bool(true)));
    }

    /// Matterhorn 01-007 (`/Suspects` absent) and the structure tree's root.
    #[test]
    fn the_catalog_marks_the_document_tagged_with_a_structure_tree() {
        let cat = built().catalog().unwrap();
        let mark = cat.get("MarkInfo").and_then(Object::as_dict).unwrap();
        assert_eq!(
            (
                mark.get("Marked"),
                mark.get("Suspects"),
                cat.get("StructTreeRoot").is_some()
            ),
            (Some(&Object::Bool(true)), None, true)
        );
    }

    /// Matterhorn 28-008, 28-009: a page with annotations has `/Tabs /S`.
    #[test]
    fn a_page_with_a_link_orders_its_annotations_by_structure() {
        let b = ua(|s, f| {
            let _ = s
                .text("Example", f, Pt::new(12.0), at(56.0, 80.0))?
                .link(LinkTarget::Uri("https://example.com".into()), Some("Example".into()))
                .tag(TagKind::P);
            Ok(())
        });
        let doc = reopen(&save(b.finish().unwrap()));
        let pages = doc.pages().unwrap();
        assert_eq!(pages.get(0).unwrap().dict().get_name("Tabs"), Some(&b"S"[..]));
    }

    /// Matterhorn 02-004: standard types are never role-mapped (a custom name that is a
    /// standard type is written as `/S` itself).
    #[test]
    fn a_standard_type_given_as_a_custom_name_is_not_role_mapped() {
        let b = ua(|s, f| s.tag(Tag::custom("Div", TagKind::Part), |s| paragraph(s, f)));
        let doc = reopen(&save(b.finish().unwrap()));
        assert!(doc.structure().unwrap().unwrap().role_map.is_empty());
    }

    #[test]
    fn the_same_calls_give_the_same_bytes() {
        assert_eq!(
            save(ua(paragraph).finish().unwrap()),
            save(ua(paragraph).finish().unwrap())
        );
    }
}
