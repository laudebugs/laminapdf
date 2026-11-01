//! Object parser: turns tokens into [`Object`]s, including indirect objects and streams.

use crate::diagnostic::Position;
use crate::error::{Error, Result};
use crate::lexer::{Lexer, Token, is_whitespace};
use crate::limits::{Limit, Limits};
use crate::object::{Dict, Name, ObjRef, Object, Stream};

/// Parses a single direct object from `input` with the default [`Limits`]. Trailing
/// white-space and comments are allowed; anything else after the object is an error.
///
/// # Errors
/// Returns an error on malformed syntax, truncation, or a hard limit exceeded
/// ([`Limits::max_nesting_depth`], [`Limits::max_string_length`]).
///
/// # Examples
/// ```
/// use laminapdf_syntax::{parse_object, Object};
/// let obj = parse_object(b"<< /Kids [3 0 R] /Count 1 >>")?;
/// assert_eq!(obj.as_dict().and_then(|d| d.get_int("Count")), Some(1));
/// # Ok::<(), laminapdf_syntax::Error>(())
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
/// use laminapdf_syntax::{Error, Limit, Limits, parse_object_with};
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
            at: Position::offset(parser.lex.pos()),
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

/// What [`Parser::indirect_head`] read.
#[derive(Debug, Clone, PartialEq)]
pub enum IndirectHead {
    /// An object that is not a stream: complete.
    Object(IndirectObject),
    /// A stream whose extent is still to be found.
    Stream(StreamHead),
}

/// A stream's header and dictionary, before its data is read ([`Parser::indirect_head`]).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct StreamHead {
    /// The object's number and generation.
    pub id: ObjRef,
    /// The stream dictionary.
    pub dict: Dict,
    /// Offset of the first data byte (after `stream` and its EOL).
    pub data_start: usize,
    /// `/Length` as declared (resolved when indirect); `None` when missing or unresolvable.
    pub declared_length: Option<i64>,
}

impl StreamHead {
    /// Where the data ends if `/Length` is right (`None` when it is missing, negative or
    /// overflows).
    #[must_use]
    pub fn declared_end(&self) -> Option<usize> {
        self.declared_length
            .and_then(|len| usize::try_from(len).ok())
            .and_then(|len| self.data_start.checked_add(len))
    }

    /// The finished object: this stream with `data`; `length_recovered` when the extent was
    /// found by scanning for `endstream` rather than from `/Length`.
    #[must_use]
    pub fn into_object(self, data: Vec<u8>, length_recovered: bool) -> IndirectObject {
        IndirectObject {
            id: self.id,
            object: Object::Stream(Stream { dict: self.dict, data }),
            length_recovered,
        }
    }
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
    /// `laminapdf-cos` relies on this to read sources without an in-memory slice in windows.
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
            Token::ArrayOpen => {
                let depth = self.enter(depth, start)?;
                let mut items = Vec::new();
                loop {
                    match self.lex.next_token()? {
                        Token::ArrayClose => return Ok(Object::Array(items)),
                        Token::Eof => {
                            return Err(Error::UnexpectedEof {
                                at: Position::offset(self.lex.pos()),
                            });
                        }
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
                        Token::Eof => {
                            return Err(Error::UnexpectedEof {
                                at: Position::offset(self.lex.pos()),
                            });
                        }
                        _ => {
                            return Err(Error::Syntax {
                                at: Position::offset(start),
                                message: "dictionary key must be a name",
                            });
                        }
                    }
                }
            }
            Token::Eof => Err(Error::UnexpectedEof {
                at: Position::offset(start),
            }),
            scalar => scalar.into_scalar().ok_or(Error::Syntax {
                at: Position::offset(start),
                message: "unexpected token",
            }),
        }
    }

    /// After an integer, looks ahead for `gen R`; rewinds if it is not a reference. An
    /// integer that can't be an object number is not followed by a look ahead at all, so its
    /// result never depends on the bytes after it ([`Parser::reached_end`]).
    fn maybe_reference(&mut self, num: i64) -> Object {
        let save = self.lex.pos();
        if let Ok(num_u32) = u32::try_from(num)
            && let Ok(Token::Integer(g)) = self.lex.next_token()
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
            at: Position::offset(offset),
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
        let head = match self.indirect_head(resolve_length)? {
            IndirectHead::Object(object) => return Ok(object),
            IndirectHead::Stream(head) => head,
        };
        let data = self.lex.data();
        let end_flag = &mut self.reached_end;
        let declared_end = head.declared_end().filter(|&end| endstream_at(data, end, end_flag));
        let length_recovered = declared_end.is_none();
        let end = if let Some(end) = declared_end {
            end
        } else {
            let Some(endstream) = find(data, b"endstream", head.data_start) else {
                *end_flag = true;
                return Err(Error::UnexpectedEof {
                    at: Position::offset(data.len()),
                });
            };
            stream_data_end(data, head.data_start, endstream)
        };
        let bytes = data.get(head.data_start..end).unwrap_or_default().to_vec();
        // Position after `endstream` (there is one at or after `end`), then consume an
        // optional `endobj`.
        let after = find(data, b"endstream", end).map_or(data.len(), |p| p + b"endstream".len());
        self.lex.set_pos(after);
        Ok(head.into_object(bytes, length_recovered))
    }

    /// The first part of [`Parser::indirect_object`]: the header and the value, and for a
    /// stream, its dictionary, where its data starts and its declared `/Length` (resolved
    /// with `resolve_length` when indirect), without looking for its end.
    ///
    /// For callers that find a stream's extent themselves (`laminapdf-cos` reading a source in
    /// windows checks a `/Length` with a small read at the declared end instead of reading
    /// up to it). [`endstream_at`] and [`stream_data_end`] are the checks
    /// [`Parser::indirect_object`] makes, so both give the same result.
    ///
    /// # Errors
    /// A malformed header or object.
    pub fn indirect_head(&mut self, resolve_length: &mut dyn FnMut(ObjRef) -> Option<i64>) -> Result<IndirectHead> {
        let id = self.object_header()?;
        let obj = self.object()?;
        let after_obj = self.lex.pos();
        let plain = |object| {
            IndirectHead::Object(IndirectObject {
                id,
                object,
                length_recovered: false,
            })
        };
        let Object::Dict(dict) = obj else {
            return Ok(plain(obj));
        };
        if !matches!(self.lex.next_token(), Ok(Token::Keyword(b"stream"))) {
            self.lex.set_pos(after_obj);
            return Ok(plain(Object::Dict(dict)));
        }
        let data_start = skip_stream_eol(self.lex.data(), self.lex.pos(), &mut self.reached_end);
        let declared_length = match dict.get("Length") {
            Some(Object::Integer(n)) => Some(*n),
            Some(Object::Ref(r)) => resolve_length(*r),
            _ => None,
        };
        Ok(IndirectHead::Stream(StreamHead {
            id,
            dict,
            data_start,
            declared_length,
        }))
    }

    /// The lexer this parser reads tokens from, for callers that mix tokens and objects (an
    /// xref table followed by its trailer). Reads through it count for
    /// [`Parser::reached_end`].
    pub const fn lexer(&mut self) -> &mut Lexer<'a> {
        &mut self.lex
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

/// Whether `endstream` follows `end` after optional white-space: the check that a stream's
/// declared `/Length` is right. Sets `reached_end` when the answer depends on bytes past the
/// end of `data`.
pub fn endstream_at(data: &[u8], end: usize, reached_end: &mut bool) -> bool {
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

/// The end of stream data that starts at `from` when its `endstream` keyword is at `idx`: the
/// EOL before the keyword (CRLF, LF or CR) is not data. Reads only `data[idx - 2..idx]`.
pub fn stream_data_end(data: &[u8], from: usize, idx: usize) -> usize {
    let mut end = idx;
    if end > from && data.get(end - 1) == Some(&b'\n') {
        end -= 1;
    }
    if end > from && data.get(end - 1) == Some(&b'\r') {
        end -= 1;
    }
    end
}

/// Finds the first occurrence of `needle` in `hay` at or after byte offset `from`, returning
/// its offset. Used to recover stream extents (`endstream`) and, in `laminapdf-cos`, to locate
/// file-structure keywords. `None` when `from` is past the end or `needle` is absent.
///
/// ```
/// assert_eq!(laminapdf_syntax::find(b"stream\nabc\nendstream", b"endstream", 0), Some(11));
/// assert_eq!(laminapdf_syntax::find(b"abc", b"b", 2), None);
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
                at: Some(Position::FileOffset(4))
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

    fn object_reached_end(input: &[u8]) -> bool {
        let mut parser = Parser::new(input, 0);
        let _ = parser.object();
        parser.reached_end()
    }

    #[test]
    fn a_negative_integer_at_the_end_does_not_look_ahead_for_a_reference() {
        // `-5` can't be an object number, so nothing after it can change the result.
        assert!(!object_reached_end(b"-5 "));
    }

    #[test]
    fn an_integer_pair_at_the_end_reaches_it_as_it_may_be_a_reference() {
        // `5 0` might continue as `5 0 R`: the result depends on what follows.
        assert!(object_reached_end(b"5 0"));
    }

    #[test]
    fn indirect_head_stops_at_the_stream_data() {
        let mut parser = Parser::new(b"1 0 obj << /Length 3 >> stream\nabc\nendstream", 0);
        let Ok(IndirectHead::Stream(head)) = parser.indirect_head(&mut no_length) else {
            panic!("not a stream head");
        };
        assert_eq!((head.data_start, head.declared_end()), (31, Some(34)));
    }

    #[test]
    fn stream_data_end_drops_the_eol_before_endstream() {
        assert_eq!(stream_data_end(b"abc\r\nendstream", 0, 5), 3);
    }
}
