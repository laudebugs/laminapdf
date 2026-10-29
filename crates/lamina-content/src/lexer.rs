//! A tolerant content-stream tokenizer (ISO 32000-2 §7.8.2): operands followed by an
//! operator, over the tokens of `lamina-syntax`'s [`Lexer`].

use lamina_cos::{Error, Result};
use lamina_syntax::{Dict, Lexer, Limits, Name, Object, Parser, PdfString, StringFormat, Token, is_whitespace};

use crate::operator::Operator;

/// The most operands kept for one operator. Real operators take at most 33 (`scn` with 32
/// DeviceN components and a pattern name, §8.6.6.5 and Annex C); when more pile up, the
/// oldest are dropped, so a run of numbers without an operator can't grow memory (ADR 0023).
pub const MAX_OPERANDS: usize = 64;

/// How many bytes after a candidate `EI` the inline-image heuristic looks at (ADR 0023).
const EI_LOOKAHEAD: usize = 64;

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
    operands: Vec<Object>,
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
            operands: Vec::new(),
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
        self.operands.clear();
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
            Err(lamina_syntax::Error::Syntax { offset, .. }) => return Ok(self.skip(start, offset, offset + 1)),
            // An unterminated string runs to the end of the content.
            Err(_) => {
                self.operands.clear();
                self.lexer.set_pos(self.data().len());
                return Ok(Step::Item(Self::issue(IssueKind::Syntax, start)));
            }
        };
        Ok(match token {
            Token::Eof => Step::End,
            Token::Integer(i) => Step::Operand(Object::Integer(i)),
            Token::Real(r) => Step::Operand(Object::Real(r)),
            Token::Name(n) => Step::Operand(Object::Name(Name(n))),
            Token::LiteralString(bytes) => Step::Operand(Object::String(PdfString {
                bytes,
                format: StringFormat::Literal,
            })),
            Token::HexString(bytes) => Step::Operand(Object::String(PdfString {
                bytes,
                format: StringFormat::Hex,
            })),
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
            Token::Keyword(b"true") => Step::Operand(Object::Bool(true)),
            Token::Keyword(b"false") => Step::Operand(Object::Bool(false)),
            Token::Keyword(b"null") => Step::Operand(Object::Null),
            Token::Keyword(keyword) => Step::Keyword(keyword),
        })
    }

    fn push(&mut self, operand: Object) {
        if self.operands.len() >= MAX_OPERANDS {
            self.operands.remove(0);
        }
        self.operands.push(operand);
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
                    return Ok(Self::issue(IssueKind::InlineImageUnterminated, bi));
                }
                Step::Operand(Object::Name(name)) if key.is_none() => key = Some(name),
                Step::Operand(value) => {
                    if let Some(key) = key.take() {
                        dict.insert(key, value);
                    }
                }
                // Keywords other than `ID` and skipped tokens inside the dictionary are ignored.
                Step::Keyword(_) | Step::Item(_) => {}
            }
        }
        let data = self.data();
        // One white-space byte separates `ID` from the data (§8.9.7).
        let mut data_start = self.lexer.pos();
        if data.get(data_start).copied().is_some_and(is_whitespace) {
            data_start += 1;
        }
        let Some((data_end, resume)) = image_end(data, data_start, &dict) else {
            self.done = true;
            self.lexer.set_pos(data.len());
            return Ok(Self::issue(IssueKind::InlineImageUnterminated, bi));
        };
        self.lexer.set_pos(resume);
        Ok(Lexed::Op(Op {
            operator: Operator::BeginInlineImage,
            operands,
            offset: bi,
            inline_image: Some(InlineImage {
                dict,
                data: data.get(data_start..data_end).unwrap_or_default(),
                data_offset: data_start,
            }),
        }))
    }
}

impl<'a> Iterator for ContentIter<'a> {
    type Item = Result<Lexed<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let start = self.lexer.pos();
            let step = match self.step() {
                Ok(step) => step,
                Err(e) => {
                    self.done = true;
                    return Some(Err(e));
                }
            };
            match step {
                Step::Operand(operand) => self.push(operand),
                Step::Item(item) => return Some(Ok(item)),
                Step::End => {
                    self.done = true;
                    if self.operands.is_empty() {
                        return None;
                    }
                    self.operands.clear();
                    return Some(Ok(Self::issue(IssueKind::Syntax, start)));
                }
                Step::Keyword(keyword) => {
                    let offset = self.lexer.pos() - keyword.len();
                    let operator = Operator::from_keyword(keyword);
                    let operands = std::mem::take(&mut self.operands);
                    if operator == Operator::BeginInlineImage {
                        return Some(self.inline_image(offset, operands));
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

/// The pdf.js-style heuristic: the first `EI` preceded by white-space whose next
/// [`EI_LOOKAHEAD`] bytes are all printable ASCII or white-space (content, not image data).
fn heuristic_end(data: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut at = start;
    while let Some(found) = data.get(at..)?.windows(2).position(|w| w == b"EI") {
        let ei = at + found;
        let preceded = ei > start && data.get(ei - 1).copied().is_some_and(is_whitespace);
        if preceded && is_ei(data, ei) {
            let after = data.get(ei + 2..).unwrap_or_default();
            let plausible = after
                .iter()
                .take(EI_LOOKAHEAD)
                .all(|&b| is_whitespace(b) || (0x20..0x7f).contains(&b));
            if plausible {
                return Some((ei - 1, ei + 2));
            }
        }
        at = ei + 1;
    }
    None
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
}
