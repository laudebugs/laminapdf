//! Encrypted text (todo B9d): `fixtures/text/encrypted/` holds text goldens encrypted by qpdf
//! (`tools/fixtures/text_encrypted.py`: R6 AES-256 and R3 RC4-128). Opened with the user
//! password, each extracts the same glyphs (text, codes, positions, quads, font names), runs
//! and lines as its plaintext golden (font object numbers aside: qpdf renumbers objects).
#![allow(clippy::expect_used, reason = "tests: a panic is a test failure")]

use laminapdf_cos::OpenOptions;
use laminapdf_model::Document;
use laminapdf_text::{PageText, TextExtractor};

fn pages(rel: &str, options: OpenOptions) -> Vec<PageText> {
    let path = format!("{}/../../fixtures/{rel}", env!("CARGO_MANIFEST_DIR"));
    let doc = Document::open(std::fs::read(path).expect("fixture"), options).expect("opens");
    let mut extractor = TextExtractor::new();
    let pages = doc.pages().expect("pages");
    pages
        .iter()
        .map(|page| extractor.page(&doc, &page).expect("extracts"))
        .collect()
}

fn same_as_plaintext(encrypted: &str, golden: &str) {
    let plain = pages(&format!("golden/{golden}"), OpenOptions::default());
    let decrypted = pages(
        &format!("text/encrypted/{encrypted}"),
        OpenOptions::default().with_password("user"),
    );
    assert!(plain.iter().any(|p| !p.glyphs.is_empty()), "{golden} has text");
    // Everything but the font's object number, which qpdf renumbers.
    let view = |pages: &[PageText]| {
        pages
            .iter()
            .map(|p| {
                let glyphs: Vec<_> = p
                    .glyphs
                    .iter()
                    .map(|g| {
                        (
                            g.text.clone(),
                            g.code,
                            g.origin,
                            g.advance,
                            g.quad,
                            g.size,
                            g.font_name.clone(),
                        )
                    })
                    .collect();
                let runs: Vec<_> = p
                    .runs
                    .iter()
                    .map(|r| (r.text.clone(), r.glyphs.clone(), r.bounds))
                    .collect();
                (glyphs, runs, p.lines())
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(view(&decrypted), view(&plain), "{encrypted}");
}

#[test]
fn aes_256_text_extracts_as_its_plaintext() {
    same_as_plaintext("text-shaped-aes-256.pdf", "text-shaped.pdf");
}

#[test]
fn rc4_128_text_extracts_as_its_plaintext() {
    same_as_plaintext("build-text-multiscript-rc4-128.pdf", "build-text-multiscript.pdf");
}

#[test]
fn the_wrong_password_extracts_nothing() {
    let path = format!(
        "{}/../../fixtures/text/encrypted/text-shaped-aes-256.pdf",
        env!("CARGO_MANIFEST_DIR")
    );
    let error = Document::open(
        std::fs::read(path).expect("fixture"),
        OpenOptions::default().with_password("nope"),
    )
    .expect_err("refused");
    assert_eq!(error.code(), "wrong-password");
}
