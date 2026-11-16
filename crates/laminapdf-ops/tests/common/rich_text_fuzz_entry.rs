//! The `rich_text` fuzz target's entry point, shared with `tests/freetext.rs`: arbitrary bytes
//! read as `/RC`'s XHTML never panic, and the text read, written in the subset and read again,
//! is the same text, with nothing simplified.

use laminapdf_ops::RichText;

/// Reads `data` as rich text and checks that the subset writes and reads it back unchanged.
///
/// # Panics
/// When the subset's round trip changes the text or reports it simplified.
pub fn rich_text(data: &[u8]) {
    let Ok(xhtml) = std::str::from_utf8(data) else {
        return;
    };
    let (text, _) = RichText::from_xhtml(xhtml);
    let (again, simplified) = RichText::from_xhtml(&text.to_xhtml());
    assert_eq!(again, text, "the subset didn't read back what it wrote");
    assert!(!simplified, "the subset's own XHTML read back as simplified");
}
