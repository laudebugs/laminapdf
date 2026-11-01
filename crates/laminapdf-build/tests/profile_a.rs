//! PDF/A-2b, PDF/A-2u and PDF/A-3b: each rule of ISO 19005-2/-3 lamina's builder
//! could break fails `Builder::finish` with its coded error, one test per rule (the veraPDF
//! rule is in each test's comment), and a conforming document carries the output intent with
//! the sRGB profile, the XMP identification and the same values in Info and XMP.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::b5b::font;
use common::{at, fixture, rect, reopen, save};
use laminapdf_build::{
    Attachment, BuildOptions, Builder, Color, Error, FillRule, IccProfile, Image, Metadata, PageSize, PdfVersion,
    Profile, Standard14,
};
use laminapdf_cos::{Dict, Object};
use laminapdf_model::Pt;

fn options(profile: Profile) -> BuildOptions {
    BuildOptions::default()
        .with_id([5; 16])
        .with_creation_date("D:20261031120000Z".parse().unwrap())
        .with_profile(profile)
}

/// A one-page document under `profile`, the page drawn by `draw`.
fn archive(profile: Profile, draw: impl FnOnce(&mut laminapdf_build::Surface)) -> Builder {
    let mut b = Builder::new(options(profile));
    let mut page = b.page(PageSize::A4);
    draw(page.surface());
    page.finish().unwrap();
    b
}

fn square(s: &mut laminapdf_build::Surface, color: Color) {
    s.fill_color(color)
        .path(|p| {
            p.rect(rect(56.0, 56.0, 100.0, 100.0));
        })
        .fill(FillRule::NonZero);
}

fn catalog(b: Builder) -> (laminapdf_model::Document, Dict) {
    let doc = reopen(&save(b.finish().unwrap()));
    let cat = doc.catalog().unwrap();
    (doc, cat)
}

fn stream_text(doc: &laminapdf_model::Document, r: &Object) -> Vec<u8> {
    let Object::Stream(s) = &*doc.resolve(r.as_ref().unwrap()).unwrap() else {
        panic!("not a stream")
    };
    doc.decode_stream(s).unwrap()
}

fn xmp(doc: &laminapdf_model::Document, cat: &Dict) -> String {
    String::from_utf8(stream_text(doc, cat.get("Metadata").unwrap())).unwrap()
}

mod rules {
    use super::*;

    #[test]
    fn rgb_and_gray_colour_builds_under_every_pdfa_profile() {
        for profile in [Profile::PdfA2b, Profile::PdfA2u, Profile::PdfA3b] {
            let b = archive(profile, |s| {
                square(s, Color::rgb(0.1, 0.4, 0.8));
                square(s, Color::gray(0.5));
            });
            assert!(b.finish().is_ok(), "{profile:?}");
        }
    }

    /// veraPDF 6.2.4.3-4 (ISO 19005-2 §6.2.4.3): DeviceCMYK without a CMYK output intent.
    #[test]
    fn a_device_cmyk_colour_fails_with_device_color_without_output_intent() {
        let b = archive(Profile::PdfA2b, |s| square(s, Color::cmyk(0.0, 0.5, 1.0, 0.0)));
        assert_eq!(
            b.finish().unwrap_err(),
            Error::DeviceColorWithoutOutputIntent { page: 0 }
        );
    }

    #[test]
    fn a_cmyk_jpeg_fails_with_device_color_without_output_intent() {
        let photo = Image::jpeg(fixture("images/photo-cmyk.jpg")).unwrap();
        let b = archive(Profile::PdfA3b, |s| {
            s.image(&photo, rect(56.0, 56.0, 100.0, 100.0));
        });
        assert_eq!(
            b.finish().unwrap_err(),
            Error::DeviceColorWithoutOutputIntent { page: 0 }
        );
    }

    #[test]
    fn device_cmyk_on_a_later_page_names_that_page() {
        let mut b = Builder::new(options(Profile::PdfA2u));
        b.page(PageSize::A4).finish().unwrap();
        let mut page = b.page(PageSize::A4);
        square(page.surface(), Color::cmyk(1.0, 0.0, 0.0, 0.0));
        page.finish().unwrap();
        assert_eq!(
            b.finish().unwrap_err(),
            Error::DeviceColorWithoutOutputIntent { page: 1 }
        );
    }

    #[test]
    fn an_icc_based_cmyk_colour_builds() {
        let header = |class: [u8; 4], space: [u8; 4]| {
            let mut h = vec![0u8; 132];
            h[..4].copy_from_slice(&132u32.to_be_bytes());
            h[8] = 2;
            h[12..16].copy_from_slice(&class);
            h[16..20].copy_from_slice(&space);
            h[36..40].copy_from_slice(b"acsp");
            h
        };
        let cmyk = IccProfile::new(header(*b"prtr", *b"CMYK")).unwrap();
        let b = archive(Profile::PdfA2b, |s| {
            square(s, Color::icc(&cmyk, &[0.0, 0.5, 1.0, 0.0]).unwrap());
        });
        assert!(b.finish().is_ok());
    }

    /// veraPDF 6.2.11.4.1-1: every font embedded (the standard 14 never are).
    #[test]
    fn a_standard_14_font_fails_with_font_not_embedded() {
        let mut b = Builder::new(options(Profile::PdfA2b));
        let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
        let mut page = b.page(PageSize::A4);
        let _ = page
            .surface()
            .text("Hello", &helvetica, Pt::new(12.0), at(56.0, 80.0))
            .unwrap();
        page.finish().unwrap();
        assert_eq!(
            b.finish().unwrap_err(),
            Error::FontNotEmbedded {
                profile: Profile::PdfA2b,
                font: "Helvetica".into()
            }
        );
    }

    fn arabic(profile: Profile) -> Builder {
        let mut b = Builder::new(options(profile));
        let f = b.font(font("NotoSansArabic-Regular.ttf")).unwrap();
        let mut page = b.page(PageSize::A4);
        let _ = page
            .surface()
            .text("بالعالم", &f, Pt::new(12.0), at(56.0, 80.0))
            .unwrap();
        page.finish().unwrap();
        b
    }

    /// veraPDF 6.2.11.7.2-1 (level U): every glyph maps to Unicode; the beh's dot stands for no
    /// text of its own.
    #[test]
    fn a_glyph_without_unicode_fails_with_glyph_without_unicode_under_pdfa_2u() {
        assert!(matches!(
            arabic(Profile::PdfA2u).finish().unwrap_err(),
            Error::GlyphWithoutUnicode {
                profile: Profile::PdfA2u,
                ..
            }
        ));
    }

    #[test]
    fn a_glyph_without_unicode_builds_under_pdfa_2b() {
        assert!(arabic(Profile::PdfA2b).finish().is_ok());
    }

    /// veraPDF 6.8-5 (ISO 19005-2 §6.8): attached files must be PDF/A themselves.
    #[test]
    fn an_attachment_fails_with_embedded_file_forbidden_under_pdfa_2() {
        for profile in [Profile::PdfA2b, Profile::PdfA2u] {
            let mut b = Builder::new(options(profile));
            b.attach(Attachment::new("data.csv", b"a,b\n".to_vec(), "text/csv"));
            assert_eq!(b.finish().unwrap_err(), Error::EmbeddedFileForbidden { profile });
        }
    }

    /// ISO 19005-3 §6.8: every embedded file has a MIME type (`/Subtype`).
    #[test]
    fn an_attachment_without_a_mime_type_fails_with_embedded_file_without_mime_type() {
        let mut b = Builder::new(options(Profile::PdfA3b));
        b.attach(Attachment::new("data.bin", vec![1, 2, 3], ""));
        assert_eq!(
            b.finish().unwrap_err(),
            Error::EmbeddedFileWithoutMimeType {
                profile: Profile::PdfA3b,
                name: "data.bin".into()
            }
        );
    }

    #[test]
    fn an_attachment_without_a_mime_type_builds_without_a_profile() {
        let mut b = Builder::new(BuildOptions::default());
        b.attach(Attachment::new("data.bin", vec![1, 2, 3], ""));
        assert!(b.finish().is_ok());
    }

    /// veraPDF 6.1.2-1: PDF/A-2 and -3 files are PDF 1.x.
    #[test]
    fn pdf_2_0_fails_with_profile_version_conflict() {
        let b = Builder::new(options(Profile::PdfA3b).with_version(PdfVersion::V2_0));
        assert_eq!(
            b.finish().unwrap_err(),
            Error::ProfileVersionConflict {
                profile: Profile::PdfA3b
            }
        );
    }

    #[test]
    fn pdfa_alone_does_not_check_pdf_ua_rules() {
        // No title, no language, nothing tagged: PDF/A level B asks for none of them.
        let b = archive(Profile::PdfA2b, |s| square(s, Color::gray(0.0)));
        assert!(b.finish().is_ok());
    }
}

mod output {
    use super::*;

    fn built(profile: Profile) -> (laminapdf_model::Document, Dict) {
        let mut b = archive(profile, |s| square(s, Color::rgb(0.2, 0.3, 0.4)));
        b.metadata(Metadata::default().with_title("Archive").with_producer("lamina tests"));
        catalog(b)
    }

    /// veraPDF 6.2.3-2: one `GTS_PDFA1` output intent with an embedded ICC profile.
    #[test]
    fn the_output_intent_embeds_the_srgb_profile() {
        let (doc, cat) = built(Profile::PdfA2b);
        let intents = cat.get("OutputIntents").and_then(Object::as_array).unwrap();
        let intent = intents[0].as_dict().unwrap();
        let profile = stream_text(&doc, intent.get("DestOutputProfile").unwrap());
        assert_eq!(
            (intents.len(), intent.get_name("S"), profile.len(), &profile[36..40]),
            (1, Some(&b"GTS_PDFA1"[..]), 480, &b"acsp"[..])
        );
    }

    /// veraPDF 6.6.4-1: `pdfaid:part` and `pdfaid:conformance`.
    #[test]
    fn the_xmp_packet_identifies_each_profile() {
        let id = |p| {
            let (doc, cat) = built(p);
            let x = xmp(&doc, &cat);
            let part = x.split("<pdfaid:part>").nth(1).unwrap().chars().next().unwrap();
            let level = x.split("<pdfaid:conformance>").nth(1).unwrap().chars().next().unwrap();
            (part, level)
        };
        assert_eq!(
            [id(Profile::PdfA2b), id(Profile::PdfA2u), id(Profile::PdfA3b)],
            [('2', 'B'), ('2', 'U'), ('3', 'B')]
        );
    }

    /// veraPDF 6.6.2.1-1: a PDF/A file always has an XMP packet, asked for or not.
    #[test]
    fn the_xmp_packet_is_written_without_metadata() {
        let (_, cat) = catalog(archive(Profile::PdfA2b, |_| {}));
        assert!(cat.get("Metadata").is_some());
    }

    /// veraPDF 6.6.3-1: Info and XMP agree (both are written from the same values).
    #[test]
    fn info_and_xmp_say_the_same() {
        let (doc, cat) = built(Profile::PdfA2u);
        let x = xmp(&doc, &cat);
        let info = doc.info().unwrap().unwrap();
        assert_eq!(
            (
                info.title().unwrap(),
                info.producer().unwrap(),
                x.contains("<pdf:Producer>lamina tests</pdf:Producer>"),
                x.contains("<xmp:CreateDate>2026-10-31T12:00:00Z</xmp:CreateDate>")
            ),
            (Some("Archive".into()), Some("lamina tests".into()), true, true)
        );
    }

    /// veraPDF 6.6.2.3.1-1: PDF/A + PDF/UA declares the `pdfuaid` schema it uses.
    #[test]
    fn pdfa_with_pdf_ua_declares_the_pdfuaid_extension_schema() {
        let mut b = Builder::new(options(Profile::PdfA2u).with_profile(Profile::PdfUa1));
        b.metadata(Metadata::default().with_title("Archive")).lang("en");
        let (doc, cat) = catalog(b);
        let x = xmp(&doc, &cat);
        assert!(
            x.contains("<pdfuaid:part>1</pdfuaid:part>")
                && x.contains("<pdfaSchema:prefix>pdfuaid</pdfaSchema:prefix>")
        );
    }

    #[test]
    fn a_document_without_a_profile_has_no_output_intent() {
        let mut b = Builder::new(BuildOptions::default());
        b.page(PageSize::A4).finish().unwrap();
        let (_, cat) = catalog(b);
        assert!(cat.get("OutputIntents").is_none() && cat.get("Metadata").is_none());
    }

    #[test]
    fn the_same_calls_give_the_same_bytes() {
        let bytes = || {
            save(
                archive(Profile::PdfA3b, |s| square(s, Color::gray(0.2)))
                    .finish()
                    .unwrap(),
            )
        };
        assert_eq!(bytes(), bytes());
    }
}
