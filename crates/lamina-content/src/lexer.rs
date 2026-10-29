//! A tolerant content-stream tokenizer (ISO 32000-2 §7.8.2): operands followed by an
//! operator, over the tokens of `lamina-syntax`'s [`Lexer`].

use lamina_cos::{Error, Result};
use std::collections::VecDeque;

use lamina_syntax::{Dict, Lexer, Limits, Name, Object, Parser, Position, Token, is_whitespace};

use crate::operator::Operator;

/// The most operands kept for one operator. Real operators take at most 33 (`scn` with 32
/// DeviceN components and a pattern name, §8.6.6.5 and Annex C); when more pile up, the
/// oldest are dropped, so a run of numbers without an operator can't grow memory (ADR 0023).
pub const MAX_OPERANDS: usize = 64;

/// How many bytes after a candidate `EI` must be printable ASCII or white-space for the
/// inline-image heuristic to accept it (pdf.js looks at 10; ADR 0023 §3).
const EI_LOOKAHEAD: usize = 10;

/// One operator and the operands read before it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Op<'a> {
    /// The operator.
    pub operator: Operator<'a>,
    /// Every operand read since the previous operator (at most [`MAX_OPERANDS`], the newest),
    /// unchecked: [`Signature::check`](crate::Signature::check) picks the ones it uses.
    pub operands: Vec<Object>,
    /// Byte offset of the operator keyword in the content.
    pub offset: usize,
    /// For [`Operator::BeginInlineImage`]: the image, `BI` through `EI`.
    pub inline_image: Option<InlineImage<'a>>,
}

/// An inline image (`BI <dict> ID <data> EI`, §8.9.7).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct InlineImage<'a> {
    /// The image dictionary as written (abbreviated keys such as `/W`, `/BPC`, `/F` are kept).
    pub dict: Dict,
    /// The image data, still encoded by its `/F` filters, borrowed from the content.
    pub data: &'a [u8],
    /// Byte offset of the data in the content.
    pub data_offset: usize,
}

/// A problem the tokenizer skipped over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IssueKind {
    /// Bytes that are not a token (an unbalanced `)`, `>`, `]` or `>>`, a bad hex digit, an
    /// unterminated string, array or dictionary) or operands with no operator at the end;
    /// they were skipped with the operands read so far (`content-syntax`).
    Syntax,
    /// No `EI` ends an inline image; the rest of the content was skipped
    /// (`inline-image-unterminated`).
    InlineImageUnterminated,
}

/// A skipped problem and where it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Issue {
    /// What was wrong.
    pub kind: IssueKind,
    /// Byte offset in the content.
    pub offset: usize,
}

/// One item of a [`ContentIter`].
#[derive(Debug, Clone, PartialEq)]
pub enum Lexed<'a> {
    /// An operator with its operands.
    Op(Op<'a>),
    /// Something the tokenizer skipped; reading goes on after it.
    Issue(Issue),
}

/// Reads decoded content-stream bytes as [`Op`]s: operands followed by an operator
/// (§7.8.2), with inline images read whole (§8.9.7).
///
/// Tolerant: bytes that are not tokens are skipped with an [`Issue`] and reading goes on.
/// Only a hard limit of the syntax layer ([`Limits::max_string_length`],
/// [`Limits::max_nesting_depth`] inside an array or dictionary operand) ends the iteration
/// with an error. Linear in the input: every step consumes at least one byte.
///
/// ```
/// use lamina_content::{ContentIter, Lexed, Operator};
///
/// let ops: Vec<_> = ContentIter::new(b"BT /F1 12 Tf (Hi) Tj ET")
///     .filter_map(|item| match item {
///         Ok(Lexed::Op(op)) => Some(op.operator),
///         _ => None,
///     })
///     .collect();
/// assert_eq!(ops, [Operator::BeginText, Operator::Font, Operator::ShowText, Operator::EndText]);
/// ```
#[derive(Debug, Clone)]
pub struct ContentIter<'a> {
    lexer: Lexer<'a>,
    limits: Limits,
    /// Operands since the previous operator: at most [`MAX_OPERANDS`], the newest.
    operands: VecDeque<Object>,
    /// Offset of the oldest operand in `operands`.
    first_operand: Option<usize>,
    /// Items read ahead (issues inside an inline-image dictionary, then the image).
    queue: VecDeque<Lexed<'a>>,
    done: bool,
}

/// What reading one token gave.
enum Step<'a> {
    Operand(Object),
    Keyword(&'a [u8]),
    Item(Lexed<'a>),
    End,
}

impl<'a> ContentIter<'a> {
    /// Reads `content` with the default [`Limits`].
    #[must_use]
    pub fn new(content: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(content, 0),
            limits: Limits::default(),
            operands: VecDeque::new(),
            first_operand: None,
            queue: VecDeque::new(),
            done: false,
        }
    }

    /// Applies `limits` to strings, names and the nesting of array and dictionary operands.
    #[must_use]
    pub fn with_limits(mut self, limits: &Limits) -> Self {
        self.lexer = self.lexer.with_limits(limits);
        self.limits = *limits;
        self
    }

    /// The byte offset reading has reached.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.lexer.pos()
    }

    fn data(&self) -> &'a [u8] {
        self.lexer.data()
    }

    fn issue(kind: IssueKind, offset: usize) -> Lexed<'a> {
        Lexed::Issue(Issue { kind, offset })
    }

    /// A skipped token: drops the operands read so far and resumes at `resume` (at least one
    /// byte past `start`).
    fn skip(&mut self, start: usize, at: usize, resume: usize) -> Step<'a> {
        self.clear_operands();
        self.lexer.set_pos(resume.max(start + 1));
        Step::Item(Self::issue(IssueKind::Syntax, at))
    }

    /// Reads one token: an operand, an operator keyword, an issue or the end.
    fn step(&mut self) -> Result<Step<'a>> {
        self.lexer.skip_whitespace();
        let start = self.lexer.pos();
        let token = match self.lexer.next_token() {
            Ok(token) => token,
            Err(e @ lamina_syntax::Error::LimitExceeded { .. }) => return Err(Error::from(e)),
            Err(lamina_syntax::Error::Syntax { at, .. }) => {
                // The lexer runs over this content, so its positions are offsets in it.
                let offset = match at {
                    Position::FileOffset(offset) => usize::try_from(offset).unwrap_or(start),
                    _ => start,
                };
                // A bad hex digit: the rest of that string up to its `>` goes too, so nothing
                // inside it is read as an operator.
                let data = self.data();
                let resume = if data.get(start) == Some(&b'<') && offset > start {
                    find_byte(data, offset, b'>').map_or(data.len(), |close| close + 1)
                } else {
                    offset + 1
                };
                return Ok(self.skip(start, offset, resume));
            }
            // An unterminated string runs to the end of the content.
            Err(_) => {
                self.clear_operands();
                self.lexer.set_pos(self.data().len());
                return Ok(Step::Item(Self::issue(IssueKind::Syntax, start)));
            }
        };
        Ok(match token {
            Token::Eof => Step::End,
            Token::ArrayOpen | Token::DictOpen => {
                let mut parser = Parser::new(self.data(), start).with_limits(&self.limits);
                match parser.object() {
                    Ok(object) => {
                        self.lexer.set_pos(parser.pos());
                        Step::Operand(object)
                    }
                    Err(e @ lamina_syntax::Error::LimitExceeded { .. }) => return Err(Error::from(e)),
                    Err(_) => self.skip(start, start, parser.pos()),
                }
            }
            Token::ArrayClose | Token::DictClose => self.skip(start, start, start + 1),
            Token::Keyword(keyword @ (b"true" | b"false" | b"null")) => {
                Step::Operand(Token::Keyword(keyword).into_scalar().unwrap_or_default())
            }
            Token::Keyword(keyword) => Step::Keyword(keyword),
            scalar => Step::Operand(scalar.into_scalar().unwrap_or_default()),
        })
    }

    /// Keeps `operand`, which starts at `offset`; past [`MAX_OPERANDS`] the oldest goes.
    fn push(&mut self, operand: Object, offset: usize) {
        if self.operands.len() >= MAX_OPERANDS {
            self.operands.pop_front();
        }
        self.operands.push_back(operand);
        self.first_operand.get_or_insert(offset);
    }

    fn clear_operands(&mut self) {
        self.operands.clear();
        self.first_operand = None;
    }

    fn take_operands(&mut self) -> Vec<Object> {
        self.first_operand = None;
        std::mem::take(&mut self.operands).into()
    }

    /// Reads `<dict pairs> ID <data> EI` after the `BI` at `bi`.
    fn inline_image(&mut self, bi: usize, operands: Vec<Object>) -> Result<Lexed<'a>> {
        let mut dict = Dict::new();
        let mut key: Option<Name> = None;
        loop {
            match self.step()? {
                Step::Keyword(b"ID") => break,
                Step::End => {
                    self.done = true;
                    self.queue
                        .push_back(Self::issue(IssueKind::InlineImageUnterminated, bi));
                    return Ok(self
                        .queue
                        .pop_front()
                        .unwrap_or(Self::issue(IssueKind::InlineImageUnterminated, bi)));
                }
                Step::Operand(Object::Name(name)) if key.is_none() => key = Some(name),
                Step::Operand(value) => {
                    if let Some(key) = key.take() {
                        dict.insert(key, value);
                    }
                }
                // Skipped tokens inside the dictionary are reported before the image.
                Step::Item(item) => self.queue.push_back(item),
                // Keywords other than `ID` (and their place as a value) are ignored.
                Step::Keyword(_) => key = None,
            }
        }
        let data = self.data();
        // One white-space byte separates `ID` from the data (§8.9.7); CR LF counts as one.
        let mut data_start = self.lexer.pos();
        if data.get(data_start..data_start + 2) == Some(b"\r\n") {
            data_start += 2;
        } else if data.get(data_start).copied().is_some_and(is_whitespace) {
            data_start += 1;
        }
        let Some((data_end, resume)) = image_end(data, data_start, &dict) else {
            self.done = true;
            self.lexer.set_pos(data.len());
            // After the dictionary's own issues, which come first in offset order.
            self.queue
                .push_back(Self::issue(IssueKind::InlineImageUnterminated, bi));
            return Ok(self
                .queue
                .pop_front()
                .unwrap_or(Self::issue(IssueKind::InlineImageUnterminated, bi)));
        };
        self.lexer.set_pos(resume);
        self.queue.push_back(Lexed::Op(Op {
            operator: Operator::BeginInlineImage,
            operands,
            offset: bi,
            inline_image: Some(InlineImage {
                dict,
                data: data.get(data_start..data_end).unwrap_or_default(),
                data_offset: data_start,
            }),
        }));
        Ok(self.queue.pop_front().unwrap_or(Lexed::Issue(Issue {
            kind: IssueKind::Syntax,
            offset: bi,
        })))
    }
}

impl<'a> Iterator for ContentIter<'a> {
    type Item = Result<Lexed<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.queue.pop_front() {
            return Some(Ok(item));
        }
        if self.done {
            return None;
        }
        loop {
            self.lexer.skip_whitespace();
            let start = self.lexer.pos();
            let step = match self.step() {
                Ok(step) => step,
                Err(e) => {
                    self.done = true;
                    return Some(Err(e));
                }
            };
            match step {
                Step::Operand(operand) => self.push(operand, start),
                Step::Item(item) => return Some(Ok(item)),
                Step::End => {
                    self.done = true;
                    let first = self.first_operand?;
                    self.clear_operands();
                    return Some(Ok(Self::issue(IssueKind::Syntax, first)));
                }
                Step::Keyword(keyword) => {
                    let offset = self.lexer.pos() - keyword.len();
                    let operator = Operator::from_keyword(keyword);
                    let operands = self.take_operands();
                    if operator == Operator::BeginInlineImage {
                        let image = self.inline_image(offset, operands);
                        if image.is_err() {
                            // A hard limit ends the iteration: issues saved from the image's
                            // dictionary are dropped with it.
                            self.done = true;
                            self.queue.clear();
                        }
                        return Some(image);
                    }
                    return Some(Ok(Lexed::Op(Op {
                        operator,
                        operands,
                        offset,
                        inline_image: None,
                    })));
                }
            }
        }
    }
}

/// Where an inline image's data ends and where reading resumes after its `EI`, in order of
/// preference (ADR 0023): the declared `/L` (`/Length`, PDF 2.0), the length computed from an
/// unfiltered image's size, then the first `EI` that looks like the end.
fn image_end(data: &[u8], start: usize, dict: &Dict) -> Option<(usize, usize)> {
    let declared = ["L", "Length"]
        .iter()
        .find_map(|key| dict.get(key).and_then(Object::as_int))
        .and_then(|len| usize::try_from(len).ok());
    for len in [declared, computed_length(dict)].into_iter().flatten() {
        if let Some(end) = start.checked_add(len)
            && let Some(resume) = ei_after(data, end)
        {
            return Some((end, resume));
        }
    }
    heuristic_end(data, start)
}

/// After optional white-space at `at`, an `EI` keyword: where reading resumes after it.
fn ei_after(data: &[u8], at: usize) -> Option<usize> {
    let rest = data.get(at..)?;
    let skip = rest.iter().take_while(|&&b| is_whitespace(b)).count();
    let ei = at + skip;
    is_ei(data, ei).then_some(ei + 2)
}

/// `EI` at `at`, followed by white-space, a delimiter or the end.
fn is_ei(data: &[u8], at: usize) -> bool {
    data.get(at..at + 2) == Some(b"EI") && data.get(at + 2).is_none_or(|&b| !lamina_syntax::is_regular(b))
}

/// The pdf.js-style heuristic (ADR 0023 §3): the first `EI` preceded by white-space whose
/// next [`EI_LOOKAHEAD`] bytes are printable ASCII or white-space and whose next token can
/// follow an inline image (the end, an operand, or an Annex A operator). The data ends before
/// the end-of-line marker in front of `EI` (CR LF, or one white-space byte).
fn heuristic_end(data: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut at = start;
    while let Some(found) = data.get(at..)?.windows(2).position(|w| w == b"EI") {
        let ei = at + found;
        let preceded = ei > start && data.get(ei - 1).copied().is_some_and(is_whitespace);
        if preceded && is_ei(data, ei) && plausible_after(data.get(ei + 2..).unwrap_or_default()) {
            let crlf = ei >= start + 2 && data.get(ei - 2..ei) == Some(b"\r\n");
            return Some((if crlf { ei - 2 } else { ei - 1 }, ei + 2));
        }
        at = ei + 1;
    }
    None
}

/// Whether `after` (the bytes after a candidate `EI`) looks like content.
fn plausible_after(after: &[u8]) -> bool {
    let printable = after
        .iter()
        .take(EI_LOOKAHEAD)
        .all(|&b| is_whitespace(b) || (0x20..0x7f).contains(&b));
    if !printable {
        return false;
    }
    let skip = after.iter().take_while(|&&b| is_whitespace(b)).count();
    let rest = after.get(skip..).unwrap_or_default();
    match rest.first() {
        None => true,
        Some(&b) if !lamina_syntax::is_regular(b) || b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.') => true,
        Some(_) => {
            let run = rest.iter().take_while(|&&b| lamina_syntax::is_regular(b)).count();
            let keyword = rest.get(..run).unwrap_or_default();
            matches!(keyword, b"true" | b"false" | b"null")
                || !matches!(Operator::from_keyword(keyword), Operator::Unknown(_))
        }
    }
}

/// The first `byte` at or after `from`.
fn find_byte(data: &[u8], from: usize, byte: u8) -> Option<usize> {
    data.get(from..)?.iter().position(|&b| b == byte).map(|i| from + i)
}

/// The data length of an unfiltered inline image from `/W`, `/H`, `/BPC` and `/CS` (or
/// `/IM`), when all are known (§8.9.7 Table 90, abbreviations in Table 91).
fn computed_length(dict: &Dict) -> Option<usize> {
    let get = |short: &str, long: &str| dict.get(short).or_else(|| dict.get(long));
    let filtered = match get("F", "Filter") {
        None | Some(Object::Null) => false,
        Some(Object::Array(filters)) => !filters.is_empty(),
        Some(_) => true,
    };
    if filtered {
        return None;
    }
    let dimension = |short, long| {
        get(short, long)
            .and_then(Object::as_int)
            .and_then(|v| usize::try_from(v).ok())
    };
    let width = dimension("W", "Width")?;
    let height = dimension("H", "Height")?;
    let mask = matches!(get("IM", "ImageMask"), Some(Object::Bool(true)));
    let (components, bits) = if mask {
        (1, 1)
    } else {
        let components = match get("CS", "ColorSpace")? {
            Object::Name(name) => match name.as_bytes() {
                b"G" | b"DeviceGray" | b"CalGray" | b"I" | b"Indexed" => 1,
                b"RGB" | b"DeviceRGB" | b"CalRGB" => 3,
                b"CMYK" | b"DeviceCMYK" => 4,
                _ => return None,
            },
            Object::Array(items) => match items.first() {
                Some(Object::Name(n)) if matches!(n.as_bytes(), b"I" | b"Indexed") => 1,
                _ => return None,
            },
            _ => return None,
        };
        (components, dimension("BPC", "BitsPerComponent")?)
    };
    let row_bits = width.checked_mul(components)?.checked_mul(bits)?;
    row_bits.div_ceil(8).checked_mul(height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(content: &[u8]) -> Vec<Lexed<'_>> {
        ContentIter::new(content)
            .map(|item| {
                item.unwrap_or(Lexed::Issue(Issue {
                    kind: IssueKind::Syntax,
                    offset: usize::MAX,
                }))
            })
            .collect()
    }

    fn first_image(content: &[u8]) -> Option<InlineImage<'_>> {
        items(content).into_iter().find_map(|item| match item {
            Lexed::Op(op) => op.inline_image,
            Lexed::Issue(_) => None,
        })
    }

    #[test]
    fn inline_image_uses_the_declared_length_even_with_ei_in_the_data() {
        let image = first_image(b"BI /W 1 /H 1 /L 5 /F /AHx ID 1 EI \n EI Q");
        assert_eq!(image.map(|i| i.data), Some(&b"1 EI "[..]));
    }

    #[test]
    fn inline_image_length_is_computed_for_unfiltered_data() {
        // 2 × 1 RGB, 8 bits: six bytes, one of them spelling `EI` with its neighbours.
        let image = first_image(b"BI /W 2 /H 1 /BPC 8 /CS /RGB ID a EI b\nEI Q");
        assert_eq!(image.map(|i| i.data), Some(&b"a EI b"[..]));
    }

    #[test]
    fn inline_image_heuristic_skips_an_ei_followed_by_binary() {
        let image = first_image(b"BI /F /DCT ID \xff EI \x01\x02 EI Q");
        assert_eq!(image.map(|i| i.data), Some(&b"\xff EI \x01\x02"[..]));
    }

    fn kinds(content: &[u8]) -> Vec<Option<IssueKind>> {
        items(content)
            .into_iter()
            .map(|item| match item {
                Lexed::Issue(issue) => Some(issue.kind),
                Lexed::Op(_) => None,
            })
            .collect()
    }

    #[test]
    fn dictionary_issues_come_before_an_unterminated_image() {
        let expected = [Some(IssueKind::Syntax), Some(IssueKind::InlineImageUnterminated)];
        // No `EI` after the data, and input ending inside the dictionary.
        assert_eq!(kinds(b"BI /W 1 ) /H 1 ID \x01\x02"), expected);
        assert_eq!(kinds(b"BI /W 1 ) /H"), expected);
    }

    #[test]
    fn a_hard_limit_in_an_image_dictionary_ends_the_iteration() {
        let limits = Limits::default().with_max_nesting_depth(2);
        let results: Vec<_> = ContentIter::new(b"BI /W 1 ) /D [[[[1]]]] ID \x01 EI q Q")
            .with_limits(&limits)
            .collect();
        assert!(matches!(results.last(), Some(Err(_))), "{results:?}");
        assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
    }

    #[test]
    fn unterminated_inline_image_is_an_issue() {
        assert_eq!(
            items(b"q BI /F /DCT ID \x01\x02"),
            [
                Lexed::Op(Op {
                    operator: Operator::Save,
                    operands: vec![],
                    offset: 0,
                    inline_image: None
                }),
                Lexed::Issue(Issue {
                    kind: IssueKind::InlineImageUnterminated,
                    offset: 2
                })
            ]
        );
    }

    #[test]
    fn a_stray_close_paren_is_skipped_with_its_operands() {
        assert_eq!(
            items(b"1 ) 2 w"),
            [
                Lexed::Issue(Issue {
                    kind: IssueKind::Syntax,
                    offset: 2
                }),
                Lexed::Op(Op {
                    operator: Operator::LineWidth,
                    operands: vec![Object::Integer(2)],
                    offset: 6,
                    inline_image: None
                })
            ]
        );
    }

    #[test]
    fn operands_without_an_operator_at_the_end_are_an_issue() {
        assert!(matches!(
            items(b"1 2").as_slice(),
            [Lexed::Issue(Issue {
                kind: IssueKind::Syntax,
                ..
            })]
        ));
    }

    #[test]
    fn at_most_max_operands_are_kept() {
        let mut content = b"0 ".repeat(MAX_OPERANDS + 10);
        content.extend_from_slice(b"n");
        let kept = items(&content).into_iter().find_map(|item| match item {
            Lexed::Op(op) => Some(op.operands.len()),
            Lexed::Issue(_) => None,
        });
        assert_eq!(kept, Some(MAX_OPERANDS));
    }

    #[test]
    fn array_and_dictionary_operands_are_parsed_whole() {
        let ops = items(b"[(a) -20 (b)] TJ /P <</MCID 3>> BDC");
        let operands: Vec<_> = ops
            .into_iter()
            .filter_map(|item| match item {
                Lexed::Op(op) => Some(op.operands.len()),
                Lexed::Issue(_) => None,
            })
            .collect();
        assert_eq!(operands, [1, 2]);
    }

    // Review round 1 (A1–A7).

    #[test]
    fn heuristic_accepts_an_ei_followed_by_another_inline_image_s_binary() {
        let image = first_image(b"BI /F /DCT ID \xff\xd8 EI BI /F /DCT ID \x00\x01 EI Q");
        assert_eq!(image.map(|i| i.data), Some(&b"\xff\xd8"[..]));
    }

    #[test]
    fn heuristic_rejects_an_ei_followed_by_an_unknown_keyword() {
        let image = first_image(b"BI /F /DCT ID \xff EI xyz EI Q");
        assert_eq!(image.map(|i| i.data), Some(&b"\xff EI xyz"[..]));
    }

    #[test]
    fn a_hard_limit_inside_an_inline_image_dictionary_ends_the_iteration() {
        let limits = Limits::default().with_max_string_length(2);
        let after_error = ContentIter::new(b"BI /W (abcd) ID x EI 0 0 m")
            .with_limits(&limits)
            .skip_while(Result::is_ok)
            .count();
        assert_eq!(after_error, 1);
    }

    #[test]
    fn a_bad_hex_digit_skips_the_rest_of_the_hex_string() {
        let ops: Vec<_> = items(b"<4G Q> 1 w")
            .into_iter()
            .map(|item| match item {
                Lexed::Op(op) => op.operator.keyword().to_vec(),
                Lexed::Issue(_) => b"issue".to_vec(),
            })
            .collect();
        assert_eq!(ops, [b"issue".to_vec(), b"w".to_vec()]);
    }

    #[test]
    fn a_syntax_issue_inside_an_inline_image_dictionary_is_reported() {
        let issues = items(b"BI /W 1 ) /H 1 /F /DCT ID \xff EI")
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    Lexed::Issue(Issue {
                        kind: IssueKind::Syntax,
                        ..
                    })
                )
            })
            .count();
        assert_eq!(issues, 1);
    }

    #[test]
    fn crlf_after_id_is_one_separator() {
        let image = first_image(b"BI /W 2 /H 1 /BPC 8 /CS /G ID\r\nab EI");
        assert_eq!(image.map(|i| i.data), Some(&b"ab"[..]));
    }

    #[test]
    fn heuristic_end_drops_a_crlf_before_ei() {
        let image = first_image(b"BI /F /DCT ID \xff\r\nEI");
        assert_eq!(image.map(|i| i.data), Some(&b"\xff"[..]));
    }

    #[test]
    fn dangling_operands_are_reported_at_the_first_one() {
        assert_eq!(
            items(b"q 1 2"),
            [
                Lexed::Op(Op {
                    operator: Operator::Save,
                    operands: vec![],
                    offset: 0,
                    inline_image: None
                }),
                Lexed::Issue(Issue {
                    kind: IssueKind::Syntax,
                    offset: 2
                })
            ]
        );
    }
}
