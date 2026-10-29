//! Object parser: turns tokens into [`Object`]s, including indirect objects and streams.

use crate::error::{Error, Result};
use crate::lexer::{Lexer, Token, is_whitespace};
use crate::limits::{Limit, Limits};
use crate::object::{Dict, Name, ObjRef, Object, PdfString, Stream, StringFormat};

/// Parses a single direct object from `input` with the default [`Limits`]. Trailing
/// white-space and comments are allowed; anything else after the object is an error.
///
/// # Errors
/// Returns an error on malformed syntax, truncation, or a hard limit exceeded
/// ([`Limits::max_nesting_depth`], [`Limits::max_string_length`]).
///
/// # Examples
/// ```
/// use lamina_syntax::{parse_object, Object};
/// let obj = parse_object(b"<< /Kids [3 0 R] /Count 1 >>")?;
/// assert_eq!(obj.as_dict().and_then(|d| d.get_int("Count")), Some(1));
/// # Ok::<(), lamina_syntax::Error>(())
/// ```
pub fn parse_object(input: &[u8]) -> Result<Object> {
    parse_object_with(input, &Limits::default())
}

/// [`parse_object`] with explicit `limits`.
///
/// # Errors
/// As [`parse_object`].
///
/// # Examples
/// ```
/// use lamina_syntax::{Error, Limit, Limits, parse_object_with};
/// let shallow = Limits::default().with_max_nesting_depth(1);
/// assert!(parse_object_with(b"[1]", &shallow).is_ok());
/// assert!(matches!(
///     parse_object_with(b"[[1]]", &shallow),
///     Err(Error::LimitExceeded { limit: Limit::NestingDepth, .. })
/// ));
/// ```
pub fn parse_object_with(input: &[u8], limits: &Limits) -> Result<Object> {
    let mut parser = Parser::new(input, 0).with_limits(limits);
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
    max_nesting_depth: usize,
    /// Whether a stream-extent check looked past the end of the input (the lexer tracks its
    /// own reads; see [`Parser::reached_end`]).
    reached_end: bool,
}

/// The result of [`Parser::indirect_object`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IndirectObject {
    /// The object's number and generation, from its `num gen obj` header.
    pub id: ObjRef,
    /// The object's value (a [`Object::Stream`] for streams).
    pub object: Object,
    /// True when the object is a stream whose `/Length` was missing, wrong or unresolvable,
    /// so its extent was recovered by scanning for `endstream`.
    pub length_recovered: bool,
}

impl<'a> Parser<'a> {
    /// A parser over `data`, starting at byte offset `pos`, with the default [`Limits`].
    pub const fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            lex: Lexer::new(data, pos),
            max_nesting_depth: Limits::new().max_nesting_depth(),
            reached_end: false,
        }
    }

    /// Applies `limits` ([`Limits::max_nesting_depth`] and [`Limits::max_string_length`]).
    #[must_use]
    pub const fn with_limits(mut self, limits: &Limits) -> Self {
        self.lex = self.lex.with_limits(limits);
        self.max_nesting_depth = limits.max_nesting_depth();
        self
    }

    /// The current byte offset.
    pub const fn pos(&self) -> usize {
        self.lex.pos()
    }

    /// Whether parsing so far has looked for a byte past the end of the input: to finish a
    /// token ([`Lexer::reached_end`]), or to find a stream's `endstream`.
    ///
    /// When it has not, the result (value or error) depends only on bytes before the end, so
    /// parsing a window of a larger input gave the same result as parsing the whole input.
    /// `lamina-cos` relies on this to read sources without an in-memory slice in windows.
    pub const fn reached_end(&self) -> bool {
        self.reached_end || self.lex.reached_end()
    }

    /// Parses one direct object (references resolved lexically as `n g R`).
    ///
    /// # Errors
    /// Malformed syntax, truncation, or a hard limit exceeded ([`Limits::max_nesting_depth`],
    /// [`Limits::max_string_length`]).
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
                let depth = self.enter(depth, start)?;
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
                let depth = self.enter(depth, start)?;
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
    pub fn indirect_object(&mut self, resolve_length: &mut dyn FnMut(ObjRef) -> Option<i64>) -> Result<IndirectObject> {
        let id = self.object_header()?;
        let obj = self.object()?;
        let after_obj = self.lex.pos();
        let plain = |object| IndirectObject {
            id,
            object,
            length_recovered: false,
        };
        let Object::Dict(dict) = obj else {
            return Ok(plain(obj));
        };
        if !matches!(self.lex.next_token(), Ok(Token::Keyword(b"stream"))) {
            self.lex.set_pos(after_obj);
            return Ok(plain(Object::Dict(dict)));
        }
        let end_flag = &mut self.reached_end;
        let data_start = skip_stream_eol(self.lex.data(), self.lex.pos(), end_flag);
        let declared = match dict.get("Length") {
            Some(Object::Integer(n)) => Some(*n),
            Some(Object::Ref(r)) => resolve_length(*r),
            _ => None,
        };
        let data = self.lex.data();
        let declared_end = declared
            .and_then(|len| usize::try_from(len).ok())
            .and_then(|len| data_start.checked_add(len))
            .filter(|&end| ends_with_endstream(data, end, end_flag));
        let length_recovered = declared_end.is_none();
        let end = match declared_end {
            Some(end) => end,
            None => scan_for_endstream(data, data_start, end_flag)?,
        };
        let bytes = data.get(data_start..end).unwrap_or_default().to_vec();
        // Position after `endstream`, then consume an optional `endobj`.
        let after = find(data, b"endstream", end).map_or_else(
            || {
                *end_flag = true;
                data.len()
            },
            |p| p + b"endstream".len(),
        );
        self.lex.set_pos(after);
        Ok(IndirectObject {
            id,
            object: Object::Stream(Stream { dict, data: bytes }),
            length_recovered,
        })
    }

    /// One level deeper than `depth`, or [`Error::LimitExceeded`] past
    /// [`Limits::max_nesting_depth`].
    fn enter(&self, depth: usize, offset: usize) -> Result<usize> {
        if depth >= self.max_nesting_depth {
            return Err(Error::limit(Limit::NestingDepth, Some(offset)));
        }
        Ok(depth + 1)
    }
}

/// After the `stream` keyword: skip spaces/tabs some writers emit, then CRLF, LF or a lone CR.
/// Sets `reached_end` when it had to look past the end of `data`.
fn skip_stream_eol(data: &[u8], mut pos: usize, reached_end: &mut bool) -> usize {
    while matches!(data.get(pos), Some(b' ' | b'\t')) {
        pos += 1;
    }
    let (first, second) = (data.get(pos), data.get(pos + 1));
    if first.is_none() || (first == Some(&b'\r') && second.is_none()) {
        *reached_end = true;
    }
    match (first, second) {
        (Some(b'\r'), Some(b'\n')) => pos + 2,
        (Some(b'\r' | b'\n'), _) => pos + 1,
        _ => pos,
    }
}

/// Whether `endstream` follows `end` after optional white-space. Sets `reached_end` when the
/// answer depends on bytes past the end of `data`.
fn ends_with_endstream(data: &[u8], end: usize, reached_end: &mut bool) -> bool {
    let mut pos = end;
    while data.get(pos).copied().is_some_and(is_whitespace) {
        pos += 1;
    }
    let rest = data.get(pos..).unwrap_or_default();
    if rest.len() < b"endstream".len() {
        *reached_end = true;
    }
    rest.starts_with(b"endstream")
}

/// The end of stream data that starts at `from`, found by scanning for `endstream`. Sets
/// `reached_end` when there is none before the end of `data`.
fn scan_for_endstream(data: &[u8], from: usize, reached_end: &mut bool) -> Result<usize> {
    let Some(idx) = find(data, b"endstream", from) else {
        *reached_end = true;
        return Err(Error::UnexpectedEof { offset: data.len() });
    };
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

    const MAX_DEPTH: usize = Limits::new().max_nesting_depth();

    #[test]
    fn parse_object_accepts_nesting_at_the_limit() {
        let input = [vec![b'['; MAX_DEPTH], vec![b']'; MAX_DEPTH]].concat();
        assert!(parse_object(&input).is_ok());
    }

    #[test]
    fn parse_object_rejects_nesting_past_the_limit() {
        let input = [vec![b'['; MAX_DEPTH + 1], vec![b']'; MAX_DEPTH + 1]].concat();
        assert!(matches!(
            parse_object(&input),
            Err(Error::LimitExceeded {
                limit: Limit::NestingDepth,
                ..
            })
        ));
    }

    #[test]
    fn parse_object_with_reports_the_offset_of_the_container_past_the_limit() {
        let limits = Limits::default().with_max_nesting_depth(2);
        assert_eq!(
            parse_object_with(b"[[ [1] ]]", &limits),
            Err(Error::LimitExceeded {
                limit: Limit::NestingDepth,
                at: Some(4)
            })
        );
    }

    #[test]
    fn indirect_stream_with_correct_length_is_not_recovered() {
        let input = b"4 0 obj << /Length 3 >> stream\nabc\nendstream endobj";
        let parsed = Parser::new(input, 0).indirect_object(&mut no_length).unwrap();
        assert!(!parsed.length_recovered);
    }

    #[test]
    fn parse_object_rejects_trailing_garbage() {
        assert!(parse_object(b"1 2").is_err());
    }

    #[test]
    fn indirect_stream_with_wrong_length_recovers_by_scanning() {
        let input = b"4 0 obj << /Length 999 >> stream\r\nabc\r\nendstream endobj";
        let parsed = Parser::new(input, 0).indirect_object(&mut no_length).unwrap();
        assert_eq!(
            (parsed.object, parsed.length_recovered),
            (
                Object::Stream(Stream::new([("Length", 999)].into_iter().collect(), b"abc".to_vec())),
                true
            )
        );
    }

    #[test]
    fn indirect_stream_uses_resolved_indirect_length() {
        let input = b"4 0 obj << /Length 5 0 R >> stream\nab\nendstream\n x endstream endobj";
        let mut resolve = |_| Some(15);
        let parsed = Parser::new(input, 0).indirect_object(&mut resolve).unwrap();
        assert_eq!(parsed.object.as_ref_stream_data(), Some(&b"ab\nendstream\n x"[..]));
    }

    impl Object {
        fn as_ref_stream_data(&self) -> Option<&[u8]> {
            match self {
                Self::Stream(s) => Some(&s.data),
                _ => None,
            }
        }
    }

    fn reached_end(input: &[u8]) -> bool {
        let mut parser = Parser::new(input, 0);
        let _ = parser.indirect_object(&mut no_length);
        parser.reached_end()
    }

    #[test]
    fn a_complete_object_followed_by_more_input_does_not_reach_the_end() {
        assert!(!reached_end(b"1 0 obj << /A 1 >> endobj\n2 0 obj"));
    }

    #[test]
    fn a_dictionary_at_the_end_reaches_it_looking_for_stream() {
        assert!(reached_end(b"1 0 obj << /A 1 >>"));
    }

    #[test]
    fn a_stream_whose_length_runs_past_the_end_reaches_it() {
        // The declared extent can't be checked, and the `endstream` scan finds none.
        assert!(reached_end(b"1 0 obj << /Length 100 >> stream\nabc"));
    }

    #[test]
    fn a_stream_with_a_valid_length_does_not_reach_the_end() {
        assert!(!reached_end(b"1 0 obj << /Length 3 >> stream\nabc\nendstream endobj "));
    }

    #[test]
    fn a_stream_without_endstream_after_its_length_reaches_the_end() {
        // A wrong `/Length` makes the parser scan for `endstream`; a short window might hold
        // only an `endstream` that sits inside the real data.
        assert!(reached_end(b"1 0 obj << /Length 1 >> stream\nabc"));
    }
}
