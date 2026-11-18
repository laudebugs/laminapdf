//! A script's tokens, enough to recognise one helper call (ADR 0021 §2). Reading tokens is the
//! whole of what happens to script text: nothing evaluates it, generates code from it or builds
//! a string anything runs.

/// One token of a script.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    /// A name: `AFNumber_Format`, `true`, `new`, `Array`.
    Ident(String),
    /// A numeric literal, decimal only.
    Number(f64),
    /// A string literal, its escapes read.
    Str(String),
    /// One of `(`, `)`, `[`, `]`, `,`, `;`, `+`, `-`.
    Punct(char),
}

/// `script`'s tokens, or `None` when it holds anything a helper call can't: a comment, an
/// operator other than a sign, an unterminated string, a character outside the language.
pub(super) fn tokens(script: &str) -> Option<Vec<Token>> {
    let mut out = Vec::new();
    let mut chars = script.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c.is_ascii_alphabetic() || c == '_' || c == '$' {
            let mut name = String::new();
            while let Some(&c) = chars
                .peek()
                .filter(|c| c.is_ascii_alphanumeric() || **c == '_' || **c == '$')
            {
                name.push(c);
                chars.next();
            }
            out.push(Token::Ident(name));
        } else if c.is_ascii_digit() || c == '.' {
            out.push(Token::Number(number(&mut chars)?));
        } else if c == '"' || c == '\'' {
            chars.next();
            out.push(Token::Str(string(&mut chars, c)?));
        } else if "()[],;+-".contains(c) {
            out.push(Token::Punct(c));
            chars.next();
        } else {
            return None;
        }
    }
    Some(out)
}

/// A decimal literal: digits, an optional fraction and an optional exponent.
fn number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<f64> {
    let mut text = String::new();
    let digits = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>, text: &mut String| {
        let mut any = false;
        while let Some(&d) = chars.peek().filter(|d| d.is_ascii_digit()) {
            text.push(d);
            chars.next();
            any = true;
        }
        any
    };
    let whole = digits(chars, &mut text);
    let mut fraction = false;
    if chars.peek() == Some(&'.') {
        text.push('.');
        chars.next();
        fraction = digits(chars, &mut text);
    }
    if !whole && !fraction {
        return None;
    }
    if let Some(&e) = chars.peek().filter(|e| **e == 'e' || **e == 'E') {
        text.push(e);
        chars.next();
        if let Some(&sign) = chars.peek().filter(|s| **s == '+' || **s == '-') {
            text.push(sign);
            chars.next();
        }
        if !digits(chars, &mut text) {
            return None;
        }
    }
    // A name straight after a number (`2px`) is no literal.
    if chars
        .peek()
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
    {
        return None;
    }
    text.parse().ok().filter(|n: &f64| n.is_finite())
}

/// A string literal's text after its opening `quote`, up to the closing one.
fn string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, quote: char) -> Option<String> {
    let mut out = String::new();
    loop {
        match chars.next()? {
            c if c == quote => return Some(out),
            '\n' | '\r' => return None,
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'v' => out.push('\u{b}'),
                '0' => out.push('\0'),
                'x' => out.push(hex(chars, 2)?),
                'u' => out.push(hex(chars, 4)?),
                // A line continuation.
                '\n' => {}
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
}

/// The character `n` hex digits name.
fn hex(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, n: usize) -> Option<char> {
    let mut value = 0_u32;
    for _ in 0..n {
        value = value * 16 + chars.next()?.to_digit(16)?;
    }
    char::from_u32(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_helper_call_reads_as_a_name_its_arguments_and_punctuation() {
        assert_eq!(
            tokens(r#"AFNumber_Format(2, -1.5e1, "\x24", true);"#),
            Some(vec![
                Token::Ident("AFNumber_Format".into()),
                Token::Punct('('),
                Token::Number(2.0),
                Token::Punct(','),
                Token::Punct('-'),
                Token::Number(15.0),
                Token::Punct(','),
                Token::Str("$".into()),
                Token::Punct(','),
                Token::Ident("true".into()),
                Token::Punct(')'),
                Token::Punct(';'),
            ])
        );
    }

    #[test]
    fn operators_comments_and_open_strings_have_no_tokens() {
        assert_eq!(
            ["a = 1", "a.b", "// note", "\"open", "1px", "x * 2", "`t`"].map(tokens),
            [None, None, None, None, None, None, None]
        );
    }
}
