//! Object streams (§7.5.7): `N` pairs of `num offset`, then the objects from `/First`.

use laminapdf_syntax::{Dict, Lexer, Limits, Object, Parser, Token};

use crate::error::{Error, Result};

fn invalid(message: &'static str) -> Error {
    Error::InvalidStructure {
        kind: "object stream",
        message,
    }
}

/// One object of an object stream, as [`parse`] read it.
#[derive(Debug)]
pub(crate) struct Member {
    pub(crate) num: u32,
    pub(crate) object: laminapdf_syntax::Result<Object>,
    /// Whether a dictionary in it had something other than a name where a key belongs, which
    /// was skipped (`dictionary-key-skipped`).
    pub(crate) skipped_keys: bool,
}

/// Parses decoded object-stream bytes into its members, in order.
/// A member that fails to parse (malformed, or over a hard limit such as
/// [`Limits::max_string_length`]) keeps its error, so one bad member does not hide the others
/// and resolving it fails exactly as it would for an object stored in the file (ADR 0003 §1).
/// A member offset past the end of the data is such an error too.
///
/// # Errors
/// [`Error::InvalidStructure`] for a missing or negative `/N` or `/First`, a `/First` past the
/// end of the data, or a header without any pair when `/N` is positive.
pub(crate) fn parse(dict: &Dict, decoded: &[u8], limits: &Limits) -> Result<Vec<Member>> {
    let n = dict
        .get_int("N")
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| invalid("bad /N"))?;
    let first = dict
        .get_int("First")
        .and_then(|f| usize::try_from(f).ok())
        .ok_or_else(|| invalid("bad /First"))?;
    // The `num offset` pairs live strictly before `/First`; lexing past it would read the
    // first members as extra pairs when `/N` is overstated.
    let header_bytes = decoded
        .get(..first)
        .ok_or_else(|| invalid("/First is past the end of the data"))?;
    let mut header = Lexer::new(header_bytes, 0).with_limits(limits);
    let mut pairs = Vec::new();
    // At most `/N` pairs, and no more than the header holds (`n` comes from the file). A header
    // with fewer pairs than `/N` keeps the ones it has (tolerant reading).
    while pairs.len() < n {
        let (Ok(Token::Integer(num)), Ok(Token::Integer(off))) = (header.next_token(), header.next_token()) else {
            break;
        };
        let (Ok(num), Ok(off)) = (u32::try_from(num), usize::try_from(off)) else {
            break;
        };
        pairs.push((num, off));
    }
    if pairs.is_empty() && n > 0 {
        return Err(invalid("empty header"));
    }
    Ok(pairs
        .into_iter()
        .map(|(num, off)| {
            let mut parser = Parser::new(decoded, first.saturating_add(off)).with_limits(limits);
            let object = parser.object();
            Member {
                num,
                object,
                skipped_keys: parser.skipped_keys(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_members_relative_to_first() {
        let dict: Dict = [("N", 2), ("First", 8)].into_iter().collect();
        let members = parse(&dict, b"1 0 2 2 7 (x)", &Limits::default()).unwrap();
        let members: Vec<_> = members.into_iter().map(|m| (m.num, m.object.unwrap())).collect();
        assert_eq!(
            members,
            vec![
                (1, Object::Integer(7)),
                (2, Object::String(laminapdf_syntax::PdfString::literal("x")))
            ]
        );
    }

    #[test]
    fn parse_should_not_read_members_as_pairs_when_n_is_overstated() {
        // The header holds 2 pairs; /N claims 3, so lexing past /First would read `7 8`.
        let dict: Dict = [("N", 3), ("First", 8)].into_iter().collect();
        let members = parse(&dict, b"1 0 2 2 7 8", &Limits::default()).unwrap();
        let members: Vec<_> = members.into_iter().map(|m| (m.num, m.object.unwrap())).collect();
        assert_eq!(members, vec![(1, Object::Integer(7)), (2, Object::Integer(8))]);
    }

    #[test]
    fn parse_should_keep_the_limit_error_of_a_member_over_a_hard_limit() {
        let dict: Dict = [("N", 2), ("First", 8)].into_iter().collect();
        let limits = Limits::default().with_max_string_length(2);
        let members: Vec<_> = parse(&dict, b"1 0 2 6 (abc) 7", &limits)
            .unwrap()
            .into_iter()
            .map(|m| (m.num, m.object))
            .collect();
        assert_eq!(
            members,
            vec![
                (
                    1,
                    Err(laminapdf_syntax::Error::LimitExceeded {
                        limit: laminapdf_syntax::Limit::StringLength,
                        at: Some(laminapdf_syntax::Position::FileOffset(8))
                    })
                ),
                (2, Ok(Object::Integer(7)))
            ]
        );
    }

    #[test]
    fn parse_should_report_a_member_with_a_skipped_dictionary_key() {
        let dict: Dict = [("N", 2), ("First", 8)].into_iter().collect();
        let members = parse(&dict, b"1 0 2 13 << x /A 1 >> 7", &Limits::default()).unwrap();
        let skipped: Vec<_> = members.iter().map(|m| (m.num, m.skipped_keys)).collect();
        assert_eq!(skipped, vec![(1, true), (2, false)]);
    }

    #[test]
    fn parse_rejects_first_past_the_end_of_the_data() {
        let dict: Dict = [("N", 1), ("First", 99)].into_iter().collect();
        assert!(parse(&dict, b"1 0 7", &Limits::default()).is_err());
    }

    #[test]
    fn parse_rejects_missing_first() {
        let dict: Dict = [("N", 1)].into_iter().collect();
        assert!(parse(&dict, b"1 0 7", &Limits::default()).is_err());
    }
}
