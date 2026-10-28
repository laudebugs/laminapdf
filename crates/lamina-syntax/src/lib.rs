//! The PDF lexical layer: turns raw bytes into tokens and objects.
//!
//! This crate is part of lamina's pure core: it takes bytes and returns values, and never touches
//! the file system, network, clocks or threads, so it builds for native, WebAssembly and mobile
//! targets alike.
//!
//! It is a placeholder for now; the lexer and object parser land here next.
//!
//! ```
//! use lamina_syntax::has_pdf_header;
//!
//! assert!(has_pdf_header(b"%PDF-2.0\n"));
//! assert!(!has_pdf_header(b"PK\x03\x04"));
//! ```
#![forbid(unsafe_code)]

/// The bytes every PDF file header starts with (ISO 32000-2 §7.5.2).
const HEADER_PREFIX: &[u8] = b"%PDF-";

/// Returns `true` when `bytes` starts with the PDF header prefix `%PDF-` (ISO 32000-2 §7.5.2).
///
/// This is a strict check at offset 0. Real-world files sometimes have junk before the header;
/// tolerant header search belongs to the reader, not to this function.
pub fn has_pdf_header(bytes: &[u8]) -> bool {
    bytes.starts_with(HEADER_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::has_pdf_header;

    #[test]
    fn accepts_input_starting_with_pdf_header() {
        assert!(has_pdf_header(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n"));
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(!has_pdf_header(b"%PDF"));
    }

    #[test]
    fn rejects_empty_input() {
        assert!(!has_pdf_header(b""));
    }
}
