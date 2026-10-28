//! Object streams (§7.5.7): `N` pairs of `num offset`, then the objects from `/First`.

use lamina_syntax::{Dict, Lexer, Object, Parser, Token};

use crate::error::{Error, Result};

fn invalid(message: &'static str) -> Error {
    Error::InvalidStructure {
        kind: "object stream",
        message,
    }
}

/// Parses decoded object-stream bytes into `(object number, object)` pairs, in order.
/// Members that fail to parse become `null` so one bad member does not hide the others.
pub(crate) fn parse(dict: &Dict, decoded: &[u8]) -> Result<Vec<(u32, Object)>> {
    let n = dict
        .get_int("N")
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| invalid("bad /N"))?;
    let first = dict
        .get_int("First")
        .and_then(|f| usize::try_from(f).ok())
        .ok_or_else(|| invalid("bad /First"))?;
    let mut header = Lexer::new(decoded, 0);
    let mut pairs = Vec::new();
    // `n` comes from the file; the header itself bounds the loop.
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
                .and_then(|pos| Parser::new(decoded, pos).object().ok())
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
        let members = parse(&dict, b"1 0 2 2 7 (x)").unwrap();
        assert_eq!(
            members,
            vec![
                (1, Object::Integer(7)),
                (2, Object::String(lamina_syntax::PdfString::literal("x")))
            ]
        );
    }

    #[test]
    fn parse_rejects_missing_first() {
        let dict: Dict = [("N", 1)].into_iter().collect();
        assert!(parse(&dict, b"1 0 7").is_err());
    }
}
