//! Byte-level tokenizer for PDF syntax (ISO 32000-1 §7.2–7.3).
//!
//! The lexer never panics: every slice access is bounds-checked through `get`, and every
//! loop consumes at least one byte, so lexing is linear in the input length.

use crate::diagnostic::Position;
use crate::error::{Error, Result};
use crate::limits::{Limit, Limits};
use crate::object::{Name, Object, PdfString, StringFormat};

/// One lexical token. Keywords borrow from the input; decoded values are owned.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    /// An integer that fits in `i64` (§7.3.3).
    Integer(i64),
    /// A real number, or an integer too large for `i64` (§7.3.3).
    Real(f64),
    /// A name with `#xx` escapes decoded, without the leading `/` (§7.3.5).
    Name(Vec<u8>),
    /// A literal string with escapes decoded (§7.3.4.2).
    LiteralString(Vec<u8>),
    /// A hex string, decoded to bytes (§7.3.4.3).
    HexString(Vec<u8>),
    /// `[`.
    ArrayOpen,
    /// `]`.
    ArrayClose,
    /// `<<`.
    DictOpen,
    /// `>>`.
    DictClose,
    /// Any other run of regular characters: `true`, `obj`, `R`, `stream`, operators, …
    Keyword(&'a [u8]),
    /// The end of the input.
    Eof,
}

impl Token<'_> {
    /// The object this token is by itself: a number, name, string, `true`, `false` or `null`
    /// (`None` for delimiters, other keywords and the end). An integer stays an integer: only
    /// the parser looks ahead for `gen R`.
    #[must_use]
    pub fn into_scalar(self) -> Option<Object> {
        Some(match self {
            Self::Integer(i) => Object::Integer(i),
            Self::Real(r) => Object::Real(r),
            Self::Name(n) => Object::Name(Name(n)),
            Self::LiteralString(bytes) => Object::String(PdfString {
                bytes,
                format: StringFormat::Literal,
            }),
            Self::HexString(bytes) => Object::String(PdfString {
                bytes,
                format: StringFormat::Hex,
            }),
            Self::Keyword(b"true") => Object::Bool(true),
            Self::Keyword(b"false") => Object::Bool(false),
            Self::Keyword(b"null") => Object::Null,
            Self::Keyword(_) | Self::ArrayOpen | Self::ArrayClose | Self::DictOpen | Self::DictClose | Self::Eof => {
                return None;
            }
        })
    }
}

/// PDF white-space characters (Table 1).
pub const fn is_whitespace(b: u8) -> bool {
    matches!(b, b'\0' | b'\t' | b'\n' | b'\x0C' | b'\r' | b' ')
}

/// PDF delimiter characters (Table 2).
pub const fn is_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

/// Neither white-space nor delimiter.
pub const fn is_regular(b: u8) -> bool {
    !is_whitespace(b) && !is_delimiter(b)
}

const fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// A cursor over PDF bytes that yields [`Token`]s.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
    max_string_length: usize,
    /// Whether a byte past the end of `data` was looked for (see [`Lexer::reached_end`]).
    reached_end: bool,
}

impl<'a> Lexer<'a> {
    /// A lexer over `data`, starting at byte offset `pos`, with the default [`Limits`].
    pub const fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos,
            max_string_length: Limits::new().max_string_length(),
            reached_end: false,
        }
    }

    /// Applies `limits` ([`Limits::max_string_length`] bounds every string and name token).
    #[must_use]
    pub const fn with_limits(mut self, limits: &Limits) -> Self {
        self.max_string_length = limits.max_string_length();
        self
    }

    /// Fails once a string or name token that started at `start` holds more than
    /// [`Limits::max_string_length`] bytes.
    fn check_length(&self, out: &[u8], start: usize) -> Result<()> {
        if out.len() > self.max_string_length {
            return Err(Error::limit(Limit::StringLength, Some(start)));
        }
        Ok(())
    }

    /// The current byte offset.
    pub const fn pos(&self) -> usize {
        self.pos
    }

    /// Moves the cursor to byte offset `pos` (past the end is fine: the next token is [`Token::Eof`]).
    pub const fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// The whole input.
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Whether lexing so far has looked for a byte past the end of the input (to finish a
    /// token, skip white-space or find out that there is no next token).
    ///
    /// When it has not, every token read so far depends only on the bytes before the end, so
    /// lexing a window of a larger input gave the same tokens as lexing the whole input would.
    /// A caller reading a window (`lamina-cos` over a source without an in-memory slice)
    /// retries with a larger one when this is `true` and the window does not end the input.
    pub const fn reached_end(&self) -> bool {
        self.reached_end
    }

    fn peek(&mut self) -> Option<u8> {
        self.peek_at(0)
    }

    fn peek_at(&mut self, ahead: usize) -> Option<u8> {
        let byte = self.data.get(self.pos.saturating_add(ahead)).copied();
        if byte.is_none() {
            self.reached_end = true;
        }
        byte
    }

    /// Skips white-space and `%` comments.
    pub fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if is_whitespace(b) {
                self.pos += 1;
            } else if b == b'%' {
                while let Some(c) = self.peek() {
                    if c == b'\n' || c == b'\r' {
                        break;
                    }
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn syntax(&self, message: &'static str) -> Error {
        Error::Syntax {
            at: Position::offset(self.pos),
            message,
        }
    }

    /// Reads the next token, skipping leading white-space and comments.
    ///
    /// # Errors
    /// [`Error::UnexpectedEof`] for an unterminated string, [`Error::Syntax`] for an unbalanced
    /// `)` or `>` or an invalid hex digit, [`Error::LimitExceeded`] for a string or name longer
    /// than [`Limits::max_string_length`].
    pub fn next_token(&mut self) -> Result<Token<'a>> {
        self.skip_whitespace();
        let Some(b) = self.peek() else {
            return Ok(Token::Eof);
        };
        match b {
            b'[' => {
                self.pos += 1;
                Ok(Token::ArrayOpen)
            }
            b']' => {
                self.pos += 1;
                Ok(Token::ArrayClose)
            }
            b'<' if self.peek_at(1) == Some(b'<') => {
                self.pos += 2;
                Ok(Token::DictOpen)
            }
            b'>' if self.peek_at(1) == Some(b'>') => {
                self.pos += 2;
                Ok(Token::DictClose)
            }
            b'<' => self.hex_string(),
            b'(' => self.literal_string(),
            b'/' => self.name(),
            b'{' | b'}' => {
                let kw = self.data.get(self.pos..=self.pos).unwrap_or_default();
                self.pos += 1;
                Ok(Token::Keyword(kw))
            }
            b')' | b'>' => Err(self.syntax("unbalanced delimiter")),
            _ => Ok(self.regular_run()),
        }
    }

    fn regular_run(&mut self) -> Token<'a> {
        let start = self.pos;
        while self.peek().is_some_and(is_regular) {
            self.pos += 1;
        }
        let run = self.data.get(start..self.pos).unwrap_or_default();
        parse_number(run).unwrap_or(Token::Keyword(run))
    }

    fn name(&mut self) -> Result<Token<'a>> {
        let start = self.pos;
        self.pos += 1; // the '/'
        let mut out = Vec::new();
        while let Some(b) = self.peek().filter(|&b| is_regular(b)) {
            if b == b'#' {
                let hi = self.peek_at(1).and_then(hex_value);
                let lo = self.peek_at(2).and_then(hex_value);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi << 4) | lo);
                    self.pos += 3;
                    self.check_length(&out, start)?;
                    continue;
                }
            }
            out.push(b);
            self.pos += 1;
            self.check_length(&out, start)?;
        }
        Ok(Token::Name(out))
    }

    fn hex_string(&mut self) -> Result<Token<'a>> {
        let start = self.pos;
        self.pos += 1; // the '<'
        let mut out = Vec::new();
        let mut high: Option<u8> = None;
        loop {
            let Some(b) = self.peek() else {
                return Err(Error::UnexpectedEof {
                    at: Position::offset(self.pos),
                });
            };
            self.pos += 1;
            if b == b'>' {
                break;
            }
            if is_whitespace(b) {
                continue;
            }
            let Some(v) = hex_value(b) else {
                return Err(Error::Syntax {
                    at: Position::offset(self.pos - 1),
                    message: "invalid hex digit",
                });
            };
            match high.take() {
                Some(h) => {
                    out.push((h << 4) | v);
                    self.check_length(&out, start)?;
                }
                None => high = Some(v),
            }
        }
        if let Some(h) = high {
            out.push(h << 4); // odd length: the missing final digit is 0 (§7.3.4.3)
            self.check_length(&out, start)?;
        }
        Ok(Token::HexString(out))
    }

    fn literal_string(&mut self) -> Result<Token<'a>> {
        let start = self.pos;
        self.pos += 1; // the '('
        let mut out = Vec::new();
        let mut depth = 0usize;
        loop {
            self.check_length(&out, start)?;
            let Some(b) = self.peek() else {
                return Err(Error::UnexpectedEof {
                    at: Position::offset(self.pos),
                });
            };
            self.pos += 1;
            match b {
                b'(' => {
                    depth += 1;
                    out.push(b);
                }
                b')' if depth == 0 => break,
                b')' => {
                    depth -= 1;
                    out.push(b);
                }
                b'\\' => self.string_escape(&mut out)?,
                b'\r' => {
                    // An unescaped EOL of any kind reads as a single LF (§7.3.4.2).
                    if self.peek() == Some(b'\n') {
                        self.pos += 1;
                    }
                    out.push(b'\n');
                }
                _ => out.push(b),
            }
        }
        Ok(Token::LiteralString(out))
    }

    fn string_escape(&mut self, out: &mut Vec<u8>) -> Result<()> {
        let Some(e) = self.peek() else {
            return Err(Error::UnexpectedEof {
                at: Position::offset(self.pos),
            });
        };
        self.pos += 1;
        match e {
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'b' => out.push(b'\x08'),
            b'f' => out.push(b'\x0C'),
            b'0'..=b'7' => {
                let mut value = u32::from(e - b'0');
                for _ in 0..2 {
                    match self.peek() {
                        Some(d @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(d - b'0');
                            self.pos += 1;
                        }
                        _ => break,
                    }
                }
                // High-order overflow is ignored (§7.3.4.2).
                out.push(value.to_le_bytes()[0]);
            }
            b'\r' => {
                // Line continuation: backslash + EOL produces nothing.
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
            }
            b'\n' => {}
            // `\(`, `\)`, `\\`, and any unknown escape: the backslash is ignored.
            other => out.push(other),
        }
        Ok(())
    }
}

/// Parses a run of regular characters as an integer or real, if it is one.
#[expect(
    clippy::naive_bytecount,
    reason = "number tokens are a few bytes long; not worth a dependency"
)]
fn parse_number(run: &[u8]) -> Option<Token<'_>> {
    let (sign_len, digits) = match run.first()? {
        b'+' | b'-' => (1, run.get(1..)?),
        _ => (0, run),
    };
    let dots = digits.iter().filter(|&&b| b == b'.').count();
    let all_valid = digits.iter().all(|&b| b.is_ascii_digit() || b == b'.');
    let has_digit = digits.iter().any(u8::is_ascii_digit);
    if !all_valid || !has_digit || dots > 1 {
        return None;
    }
    let text = std::str::from_utf8(run).ok()?;
    if dots == 0
        && let Ok(i) = text.parse::<i64>()
    {
        return Some(Token::Integer(i));
    }
    // Reals, and integers too large for i64, degrade gracefully to f64.
    let normalized = if digits.first() == Some(&b'.') {
        let (sign, rest) = text.split_at_checked(sign_len)?;
        format!("{sign}0{rest}")
    } else {
        text.to_owned()
    };
    normalized.parse::<f64>().ok().map(Token::Real)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: &[u8]) -> Vec<Token<'_>> {
        let mut lex = Lexer::new(input, 0);
        let mut out = Vec::new();
        loop {
            match lex.next_token() {
                Ok(Token::Eof) | Err(_) => return out,
                Ok(t) => out.push(t),
            }
        }
    }

    #[test]
    fn numbers_accept_leading_dot_sign_and_trailing_dot() {
        assert_eq!(
            tokens(b"+5 -.5 4. 0.25 -3"),
            vec![
                Token::Integer(5),
                Token::Real(-0.5),
                Token::Real(4.0),
                Token::Real(0.25),
                Token::Integer(-3)
            ]
        );
    }

    #[test]
    fn integer_overflow_degrades_to_real() {
        assert_eq!(tokens(b"99999999999999999999"), vec![Token::Real(1e20)]);
    }

    #[test]
    fn comments_and_all_whitespace_kinds_are_skipped() {
        assert_eq!(
            tokens(b"%c\r\n\x00\t\x0C 1%x\n2"),
            vec![Token::Integer(1), Token::Integer(2)]
        );
    }

    #[test]
    fn literal_string_decodes_all_escapes() {
        assert_eq!(
            tokens(b"(\\n\\r\\t\\b\\f\\(\\)\\\\\\101\\7x\\q)"),
            vec![Token::LiteralString(b"\n\r\t\x08\x0C()\\A\x07xq".to_vec())]
        );
    }

    #[test]
    fn literal_string_handles_line_continuation() {
        assert_eq!(
            tokens(b"(ab\\\r\ncd\\\nef)"),
            vec![Token::LiteralString(b"abcdef".to_vec())]
        );
    }

    #[test]
    fn literal_string_keeps_balanced_parens() {
        assert_eq!(tokens(b"(a(b)c)"), vec![Token::LiteralString(b"a(b)c".to_vec())]);
    }

    #[test]
    fn literal_string_normalizes_raw_crlf_to_lf() {
        assert_eq!(tokens(b"(a\r\nb\rc)"), vec![Token::LiteralString(b"a\nb\nc".to_vec())]);
    }

    #[test]
    fn hex_string_ignores_whitespace_and_pads_odd_length() {
        assert_eq!(tokens(b"<4 1\n4>"), vec![Token::HexString(vec![0x41, 0x40])]);
    }

    #[test]
    fn name_decodes_hash_escapes() {
        assert_eq!(tokens(b"/A#20B#2"), vec![Token::Name(b"A B#2".to_vec())]);
    }

    fn lex_with_max_string(input: &[u8], max: usize) -> Result<Token<'_>> {
        Lexer::new(input, 0)
            .with_limits(&Limits::default().with_max_string_length(max))
            .next_token()
    }

    #[test]
    fn literal_string_at_the_length_limit_is_accepted() {
        assert_eq!(
            lex_with_max_string(b"(abcd)", 4),
            Ok(Token::LiteralString(b"abcd".to_vec()))
        );
    }

    #[test]
    fn literal_string_past_the_length_limit_is_rejected() {
        assert_eq!(
            lex_with_max_string(b"(abcde)", 4),
            Err(Error::LimitExceeded {
                limit: Limit::StringLength,
                at: Some(Position::FileOffset(0))
            })
        );
    }

    #[test]
    fn hex_string_at_the_length_limit_is_accepted() {
        assert_eq!(
            lex_with_max_string(b"<41424344>", 4),
            Ok(Token::HexString(b"ABCD".to_vec()))
        );
    }

    #[test]
    fn hex_string_past_the_length_limit_is_rejected() {
        assert!(matches!(
            lex_with_max_string(b"<4142434445>", 4),
            Err(Error::LimitExceeded {
                limit: Limit::StringLength,
                ..
            })
        ));
    }

    #[test]
    fn name_past_the_length_limit_is_rejected() {
        assert!(matches!(
            lex_with_max_string(b"/ABCDE", 4),
            Err(Error::LimitExceeded {
                limit: Limit::StringLength,
                ..
            })
        ));
    }

    #[test]
    fn unterminated_string_is_an_error() {
        assert!(Lexer::new(b"(abc", 0).next_token().is_err());
    }

    fn reached_end_after_one_token(input: &[u8]) -> bool {
        let mut lex = Lexer::new(input, 0);
        let _ = lex.next_token();
        lex.reached_end()
    }

    #[test]
    fn a_token_followed_by_a_delimiter_does_not_reach_the_end() {
        assert!(!reached_end_after_one_token(b"123 "));
    }

    #[test]
    fn a_token_that_runs_to_the_end_reaches_it() {
        // `123` might continue past the end (`1234`), so the result depends on what follows.
        assert!(reached_end_after_one_token(b"123"));
    }

    #[test]
    fn a_lone_less_than_at_the_end_reaches_it() {
        // `<` could be the start of `<<`.
        assert!(reached_end_after_one_token(b"<"));
    }
}
