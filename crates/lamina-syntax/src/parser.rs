//! Object parser: turns tokens into [`Object`]s, including indirect objects and streams.

use crate::error::{Error, Result};
use crate::lexer::{Lexer, Token, is_whitespace};
use crate::object::{Dict, Name, ObjRef, Object, PdfString, Stream, StringFormat};

/// Maximum array/dictionary nesting depth accepted by the parser.
pub const MAX_DEPTH: usize = 256;

/// Parses a single direct object from `input`. Trailing white-space and comments are
/// allowed; anything else after the object is an error.
///
/// # Errors
/// Returns an error on malformed syntax, truncation, or nesting deeper than [`MAX_DEPTH`].
///
/// # Examples
/// ```
/// use lamina_syntax::{parse_object, Object};
/// let obj = parse_object(b"<< /Kids [3 0 R] /Count 1 >>")?;
/// assert_eq!(obj.as_dict().and_then(|d| d.get_int("Count")), Some(1));
/// # Ok::<(), lamina_syntax::Error>(())
/// ```
pub fn parse_object(input: &[u8]) -> Result<Object> {
    let mut parser = Parser::new(input, 0);
    let obj = parser.object()?;
    match parser.lex.next_token()? {
        Token::Eof => Ok(obj),
        _ => Err(Error::Syntax {
            offset: parser.lex.pos(),
            message: "trailing data after object",
        }),
    }
}

/// A parser over PDF bytes, positioned at a byte offset.
///
/// Used for single objects ([`Parser::object`]) and for the file-structure layer
/// ([`Parser::object_header`], [`Parser::indirect_object`]).
#[derive(Debug, Clone)]
pub struct Parser<'a> {
    lex: Lexer<'a>,
}

impl<'a> Parser<'a> {
    /// A parser over `data`, starting at byte offset `pos`.
    pub const fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            lex: Lexer::new(data, pos),
        }
    }

    /// The current byte offset.
    pub const fn pos(&self) -> usize {
        self.lex.pos()
    }

    /// Parses one direct object (references resolved lexically as `n g R`).
    ///
    /// # Errors
    /// Malformed syntax, truncation, or nesting deeper than [`MAX_DEPTH`].
    pub fn object(&mut self) -> Result<Object> {
        let tok = self.lex.next_token()?;
        self.object_from(tok, 0)
    }

    fn object_from(&mut self, tok: Token<'a>, depth: usize) -> Result<Object> {
        let start = self.lex.pos();
        match tok {
            Token::Integer(i) => Ok(self.maybe_reference(i)),
            Token::Real(r) => Ok(Object::Real(r)),
            Token::Name(n) => Ok(Object::Name(Name(n))),
            Token::LiteralString(s) => Ok(Object::String(PdfString {
                bytes: s,
                format: StringFormat::Literal,
            })),
            Token::HexString(s) => Ok(Object::String(PdfString {
                bytes: s,
                format: StringFormat::Hex,
            })),
            Token::ArrayOpen => {
                let depth = enter(depth, start)?;
                let mut items = Vec::new();
                loop {
                    match self.lex.next_token()? {
                        Token::ArrayClose => return Ok(Object::Array(items)),
                        Token::Eof => return Err(Error::UnexpectedEof { offset: self.lex.pos() }),
                        t => items.push(self.object_from(t, depth)?),
                    }
                }
            }
            Token::DictOpen => {
                let depth = enter(depth, start)?;
                let mut dict = Dict::new();
                loop {
                    match self.lex.next_token()? {
                        Token::DictClose => return Ok(Object::Dict(dict)),
                        Token::Name(key) => {
                            let value_tok = self.lex.next_token()?;
                            if value_tok == Token::DictClose {
                                // `<< /Key >>`: tolerate a missing value as null and stop.
                                dict.insert(Name(key), Object::Null);
                                return Ok(Object::Dict(dict));
                            }
                            let value = self.object_from(value_tok, depth)?;
                            dict.insert(Name(key), value);
                        }
                        Token::Eof => return Err(Error::UnexpectedEof { offset: self.lex.pos() }),
                        _ => {
                            return Err(Error::Syntax {
                                offset: start,
                                message: "dictionary key must be a name",
                            });
                        }
                    }
                }
            }
            Token::Keyword(b"true") => Ok(Object::Bool(true)),
            Token::Keyword(b"false") => Ok(Object::Bool(false)),
            Token::Keyword(b"null") => Ok(Object::Null),
            Token::Eof => Err(Error::UnexpectedEof { offset: start }),
            Token::Keyword(_) | Token::ArrayClose | Token::DictClose => Err(Error::Syntax {
                offset: start,
                message: "unexpected token",
            }),
        }
    }

    /// After an integer, looks ahead for `gen R`; rewinds if it is not a reference.
    fn maybe_reference(&mut self, num: i64) -> Object {
        let save = self.lex.pos();
        if let (Ok(num_u32), Ok(Token::Integer(g))) = (u32::try_from(num), self.lex.next_token())
            && let Ok(gen_nr) = u16::try_from(g)
            && let Ok(Token::Keyword(b"R")) = self.lex.next_token()
        {
            return Object::Ref(ObjRef::new(num_u32, gen_nr));
        }
        self.lex.set_pos(save);
        Object::Integer(num)
    }

    /// Reads `num gen obj` at the current position.
    ///
    /// # Errors
    /// [`Error::Syntax`] when the next tokens are not an object header.
    pub fn object_header(&mut self) -> Result<ObjRef> {
        let offset = self.lex.pos();
        let bad = || Error::Syntax {
            offset,
            message: "expected `n g obj`",
        };
        let Token::Integer(num) = self.lex.next_token()? else {
            return Err(bad());
        };
        let Token::Integer(g) = self.lex.next_token()? else {
            return Err(bad());
        };
        let Token::Keyword(b"obj") = self.lex.next_token()? else {
            return Err(bad());
        };
        let num = u32::try_from(num).map_err(|_| bad())?;
        let gen_nr = u16::try_from(g).map_err(|_| bad())?;
        Ok(ObjRef::new(num, gen_nr))
    }

    /// Parses `num gen obj <object> [stream…endstream] endobj` at the current position.
    ///
    /// `resolve_length` resolves an indirect `/Length`; when it fails or the length is
    /// wrong, the stream extent is recovered by scanning for `endstream`.
    ///
    /// # Errors
    /// A malformed header or object, or a stream without any `endstream`.
    pub fn indirect_object(
        &mut self,
        resolve_length: &mut dyn FnMut(ObjRef) -> Option<i64>,
    ) -> Result<(ObjRef, Object)> {
        let id = self.object_header()?;
        let obj = self.object()?;
        let after_obj = self.lex.pos();
        let Object::Dict(dict) = obj else {
            return Ok((id, obj));
        };
        if !matches!(self.lex.next_token(), Ok(Token::Keyword(b"stream"))) {
            self.lex.set_pos(after_obj);
            return Ok((id, Object::Dict(dict)));
        }
        let data_start = skip_stream_eol(self.lex.data(), self.lex.pos());
        let declared = match dict.get("Length") {
            Some(Object::Integer(n)) => Some(*n),
            Some(Object::Ref(r)) => resolve_length(*r),
            _ => None,
        };
        let data = self.lex.data();
        let end = declared
            .and_then(|len| usize::try_from(len).ok())
            .and_then(|len| data_start.checked_add(len))
            .filter(|&end| ends_with_endstream(data, end))
            .map_or_else(|| scan_for_endstream(data, data_start), Ok)?;
        let bytes = data.get(data_start..end).unwrap_or_default().to_vec();
        // Position after `endstream`, then consume an optional `endobj`.
        let after = find(data, b"endstream", end).map_or(data.len(), |p| p + b"endstream".len());
        self.lex.set_pos(after);
        Ok((id, Object::Stream(Stream { dict, data: bytes })))
    }
}

const fn enter(depth: usize, offset: usize) -> Result<usize> {
    if depth >= MAX_DEPTH {
        return Err(Error::TooDeep {
            limit: MAX_DEPTH,
            offset,
        });
    }
    Ok(depth + 1)
}

/// After the `stream` keyword: skip spaces/tabs some writers emit, then CRLF, LF or a lone CR.
fn skip_stream_eol(data: &[u8], mut pos: usize) -> usize {
    while matches!(data.get(pos), Some(b' ' | b'\t')) {
        pos += 1;
    }
    match (data.get(pos), data.get(pos + 1)) {
        (Some(b'\r'), Some(b'\n')) => pos + 2,
        (Some(b'\r' | b'\n'), _) => pos + 1,
        _ => pos,
    }
}

fn ends_with_endstream(data: &[u8], end: usize) -> bool {
    let mut pos = end;
    while data.get(pos).copied().is_some_and(is_whitespace) {
        pos += 1;
    }
    data.get(pos..).is_some_and(|rest| rest.starts_with(b"endstream"))
}

fn scan_for_endstream(data: &[u8], from: usize) -> Result<usize> {
    let idx = find(data, b"endstream", from).ok_or(Error::UnexpectedEof { offset: data.len() })?;
    let mut end = idx;
    if end > from && data.get(end - 1) == Some(&b'\n') {
        end -= 1;
    }
    if end > from && data.get(end - 1) == Some(&b'\r') {
        end -= 1;
    }
    Ok(end)
}

/// Finds the first occurrence of `needle` in `hay` at or after byte offset `from`, returning
/// its offset. Used to recover stream extents (`endstream`) and, in `lamina-cos`, to locate
/// file-structure keywords. `None` when `from` is past the end or `needle` is absent.
///
/// ```
/// assert_eq!(lamina_syntax::find(b"stream\nabc\nendstream", b"endstream", 0), Some(11));
/// assert_eq!(lamina_syntax::find(b"abc", b"b", 2), None);
/// ```
pub fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    let tail = hay.get(from..)?;
    tail.windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_length(_: ObjRef) -> Option<i64> {
        None
    }

    #[test]
    fn parse_object_reads_reference_inside_array() {
        assert_eq!(
            parse_object(b"[1 0 R 2]"),
            Ok(Object::Array(vec![Object::Ref(ObjRef::new(1, 0)), Object::Integer(2)]))
        );
    }

    #[test]
    fn parse_object_rewinds_when_integers_are_not_a_reference() {
        assert_eq!(
            parse_object(b"[1 2 3]"),
            Ok(Object::Array(vec![1.into(), 2.into(), 3.into()]))
        );
    }

    #[test]
    fn parse_object_accepts_nesting_at_the_limit() {
        let input = [vec![b'['; MAX_DEPTH], vec![b']'; MAX_DEPTH]].concat();
        assert!(parse_object(&input).is_ok());
    }

    #[test]
    fn parse_object_rejects_nesting_past_the_limit() {
        let input = [vec![b'['; MAX_DEPTH + 1], vec![b']'; MAX_DEPTH + 1]].concat();
        assert!(matches!(parse_object(&input), Err(Error::TooDeep { .. })));
    }

    #[test]
    fn parse_object_rejects_trailing_garbage() {
        assert!(parse_object(b"1 2").is_err());
    }

    #[test]
    fn indirect_stream_with_wrong_length_recovers_by_scanning() {
        let input = b"4 0 obj << /Length 999 >> stream\r\nabc\r\nendstream endobj";
        let (_, obj) = Parser::new(input, 0).indirect_object(&mut no_length).unwrap();
        assert_eq!(
            obj,
            Object::Stream(Stream::new([("Length", 999)].into_iter().collect(), b"abc".to_vec()))
        );
    }

    #[test]
    fn indirect_stream_uses_resolved_indirect_length() {
        let input = b"4 0 obj << /Length 5 0 R >> stream\nab\nendstream\n x endstream endobj";
        let mut resolve = |_| Some(15);
        let (_, obj) = Parser::new(input, 0).indirect_object(&mut resolve).unwrap();
        assert_eq!(obj.as_ref_stream_data(), Some(&b"ab\nendstream\n x"[..]));
    }

    impl Object {
        fn as_ref_stream_data(&self) -> Option<&[u8]> {
            match self {
                Self::Stream(s) => Some(&s.data),
                _ => None,
            }
        }
    }
}
