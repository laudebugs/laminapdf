//! Object streams (§7.5.7): `N` pairs of `num offset`, then the objects from `/First`.

use lamina_syntax::{Dict, Lexer, Limits, Object, Parser, Token};

use crate::error::{Error, Result};

fn invalid(message: &'static str) -> Error {
    Error::InvalidStructure {
        kind: "object stream",
        message,
    }
}

/// Parses decoded object-stream bytes into `(object number, object)` pairs, in order.
/// Members that fail to parse become `null` so one bad member does not hide the others.
///
/// # Errors
/// [`Error::InvalidStructure`] for a missing or negative `/N` or `/First`, a `/First` past the
/// end of the data, or a header without any pair when `/N` is positive.
pub(crate) fn parse(dict: &Dict, decoded: &[u8], limits: &Limits) -> Result<Vec<(u32, Object)>> {
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
            let obj = first
                .checked_add(off)
                .and_then(|pos| Parser::new(decoded, pos).with_limits(limits).object().ok())
                .unwrap_or(Object::Null);
            (num, obj)
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
        assert_eq!(
            members,
            vec![
                (1, Object::Integer(7)),
                (2, Object::String(lamina_syntax::PdfString::literal("x")))
            ]
        );
    }

    #[test]
    fn parse_should_not_read_members_as_pairs_when_n_is_overstated() {
        // The header holds 2 pairs; /N claims 3, so lexing past /First would read `7 8`.
        let dict: Dict = [("N", 3), ("First", 8)].into_iter().collect();
        let members = parse(&dict, b"1 0 2 2 7 8", &Limits::default()).unwrap();
        assert_eq!(members, vec![(1, Object::Integer(7)), (2, Object::Integer(8))]);
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
