//! Tokenises arbitrary bytes until the end or the first error.
#![no_main]

use lamina_syntax::{Lexer, Limits, Token};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = Limits::default();
    let mut lexer = Lexer::new(data, 0).with_limits(&limits);
    loop {
        let before = lexer.pos();
        match lexer.next_token() {
            Ok(Token::Eof) | Err(_) => break,
            // A token that consumed nothing would loop forever: that is a bug.
            Ok(_) => assert!(lexer.pos() > before, "lexer did not advance at {before}"),
        }
    }
});
