//! Rust source as the maintainability checks read it: comments blanked, literals hidden, and the
//! lines of `#[cfg(test)]` items marked.
//!
//! The scanner is a small lexer, not a parser. It knows what can hide a brace or a keyword from a
//! line-based reading: line and nested block comments, string, byte and C-string literals, raw
//! strings with any number of `#`, and character literals (told apart from lifetimes). Everything
//! else is copied through, so the checks match tokens on [`Source::code`] without false hits in
//! comments or strings.

use crate::lines;

/// One Rust file, scanned.
#[derive(Debug)]
pub(crate) struct Source {
    /// The text with every comment blanked to spaces and the inside of every literal turned into
    /// `x`s (whitespace kept, quotes kept). It has the input's length and line breaks, so a byte
    /// offset or a line number means the same in both.
    pub(crate) code: String,
    /// For each line (0-based): whether it holds code, that is, anything but whitespace and comments.
    has_code: Vec<bool>,
    /// For each line (0-based): whether it lies inside a `#[cfg(test)]` item, its attribute included.
    test: Vec<bool>,
    /// The modules declared out of line by a `#[cfg(test)] mod name;`: test code in another file.
    pub(crate) test_modules: Vec<TestModule>,
    /// Where each line of [`Source::code`] starts ([`lines::starts`]).
    starts: Vec<usize>,
    /// Every `#[path = "…"]` attribute: its byte offset and value (masking hides values in `code`).
    pub(crate) paths: Vec<(usize, String)>,
}

/// A module declared out of line by `#[cfg(test)] mod name;`, with the file its `#[path]`
/// attribute names, if it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestModule {
    pub(crate) name: String,
    pub(crate) path: Option<String>,
}

/// One token of [`Source::code`]: an identifier (or number, or masked literal text) or one
/// punctuation byte, with its byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token<'a> {
    Word(&'a str, usize),
    Punct(u8, usize),
}

impl Token<'_> {
    /// The token's byte offset in the file.
    pub(crate) fn offset(self) -> usize {
        match self {
            Token::Word(_, offset) | Token::Punct(_, offset) => offset,
        }
    }

    /// Whether the token is the word `word`.
    pub(crate) fn is(self, word: &str) -> bool {
        matches!(self, Token::Word(text, _) if text == word)
    }

    /// Whether the token is the punctuation byte `byte`.
    pub(crate) fn is_punct(self, byte: u8) -> bool {
        matches!(self, Token::Punct(found, _) if found == byte)
    }
}

impl Source {
    /// Scans `text`.
    pub(crate) fn new(text: &str) -> Self {
        let code = mask(text);
        let starts = lines::starts(&code);
        let has_code = code.split('\n').map(|line| !line.trim().is_empty()).collect();
        let paths = attributes(&code)
            .iter()
            .filter(|a| !a.inner && a.content.split('=').next().is_some_and(|key| key.trim() == "path"))
            .filter_map(|a| {
                let raw = text.get(a.start..a.end)?;
                let value = raw.get(raw.find('"')? + 1..raw.rfind('"')?)?;
                Some((a.start, value.to_owned()))
            })
            .collect();
        let mut source = Self {
            test: vec![false; starts.len()],
            code,
            has_code,
            test_modules: Vec::new(),
            starts,
            paths,
        };
        source.mark_test_items(text);
        source
    }

    /// The number of lines that hold code outside `#[cfg(test)]` items.
    pub(crate) fn non_test_lines(&self) -> usize {
        self.has_code
            .iter()
            .zip(&self.test)
            .filter(|&(&code, &test)| code && !test)
            .count()
    }

    /// The 1-based line of byte `offset`.
    pub(crate) fn line_of(&self, offset: usize) -> usize {
        lines::line_of(&self.starts, offset)
    }

    /// Whether byte `offset` lies in a `#[cfg(test)]` item.
    pub(crate) fn is_test(&self, offset: usize) -> bool {
        self.test
            .get(self.line_of(offset).saturating_sub(1))
            .copied()
            .unwrap_or(false)
    }

    /// The tokens of [`Source::code`].
    pub(crate) fn tokens(&self) -> Vec<Token<'_>> {
        tokens(&self.code)
    }

    /// Every attribute (`#[…]` and `#![…]`) in the file.
    pub(crate) fn attributes(&self) -> Vec<Attribute<'_>> {
        attributes(&self.code)
    }

    /// Marks the lines of every `#[cfg(test)]` item; `text` is the unmasked source, for `#[path]`.
    fn mark_test_items(&mut self, text: &str) {
        let mut ranges = Vec::new();
        let mut modules = Vec::new();
        for attribute in attributes(&self.code) {
            if !is_test_cfg(attribute.content) {
                continue;
            }
            if attribute.inner {
                // `#![cfg(test)]` makes the enclosing module test code: an inline one, or the file.
                ranges.push(enclosing_block(&self.code, attribute.start));
                continue;
            }
            let end = item_end(&self.code, attribute.end);
            let item = self.code.get(attribute.end..end).unwrap_or_default();
            if let Some(name) = out_of_line_module(item) {
                modules.push(TestModule {
                    name: name.to_owned(),
                    path: path_attribute(&self.code, text, attribute.start, end),
                });
            }
            ranges.push((attribute.start, end));
        }
        self.test_modules = modules;
        for (start, end) in ranges {
            let first = self.line_of(start);
            let last = self.line_of(end.saturating_sub(1).max(start));
            for line in self.test.iter_mut().take(last).skip(first - 1) {
                *line = true;
            }
        }
    }
}

/// One attribute: `#[content]` or, when `inner`, `#![content]`, at bytes `start..end`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Attribute<'a> {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) inner: bool,
    pub(crate) content: &'a str,
}

/// The attributes in masked `code`; an attribute's brackets nest (`#[doc = [..]]` is rare, but
/// `#[expect(x, reason = "[a]")]` is masked to `"xxx"` anyway).
fn attributes(code: &str) -> Vec<Attribute<'_>> {
    let bytes = code.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(offset) = code.get(at..).and_then(|rest| rest.find('#')) {
        let start = at + offset;
        at = start + 1;
        let mut open = skip_whitespace(bytes, start + 1);
        let inner = bytes.get(open) == Some(&b'!');
        if inner {
            open = skip_whitespace(bytes, open + 1);
        }
        if bytes.get(open) != Some(&b'[') {
            continue;
        }
        let Some(close) = matching(bytes, open) else { continue };
        found.push(Attribute {
            start,
            end: close + 1,
            inner,
            content: code.get(open + 1..close).unwrap_or_default(),
        });
        at = close + 1;
    }
    found
}

/// Whether an attribute's content is `cfg(test)`, or `cfg(all(…))` with `test` among its
/// arguments: code compiled only for tests.
fn is_test_cfg(content: &str) -> bool {
    let compact: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    if compact == "cfg(test)" {
        return true;
    }
    let Some(arguments) = compact
        .strip_prefix("cfg(all(")
        .and_then(|rest| rest.strip_suffix("))"))
    else {
        return false;
    };
    let mut depth = 0usize;
    let mut start = 0;
    let mut found = false;
    for (index, byte) in arguments.bytes().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                found |= arguments.get(start..index) == Some("test");
                start = index + 1;
            }
            _ => {}
        }
    }
    found || arguments.get(start..) == Some("test")
}

/// The block that encloses byte `at`: from its `{` to just after its `}`, or the whole file
/// when `at` is at the top level.
fn enclosing_block(code: &str, at: usize) -> (usize, usize) {
    let bytes = code.as_bytes();
    let mut depth = 0usize;
    for index in (0..at.min(bytes.len())).rev() {
        match bytes.get(index) {
            Some(b'}') => depth += 1,
            Some(b'{') if depth == 0 => {
                let end = matching(bytes, index).map_or(bytes.len(), |close| close + 1);
                return (index, end);
            }
            Some(b'{') => depth -= 1,
            _ => {}
        }
    }
    (0, bytes.len())
}

/// The index of the bracket that closes the one at `open` (`(`, `[` or `{`, all counted together).
fn matching(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, &byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Words that start an item, after any visibility. An attribute on anything else (a field, a
/// variant, a match arm, a statement) covers it up to the next `,` outside brackets.
const ITEM_KEYWORDS: [&str; 15] = [
    "async",
    "const",
    "enum",
    "extern",
    "fn",
    "impl",
    "macro_rules",
    "mod",
    "static",
    "struct",
    "trait",
    "type",
    "union",
    "unsafe",
    "use",
];

/// Where the item that starts after an outer attribute ends. Its own further attributes are
/// skipped by the bracket count. It ends:
/// - after its first `;` outside brackets;
/// - after the `}` that closes its first braced body (an item's, a macro call's, a match arm's
///   or a block's), and a `;` straight after it, as in `static S: T = T { … };` (or a `,` for a
///   field, variant or arm);
/// - for a field, variant or match arm, after its first `,` outside brackets;
/// - before a bracket that closes the enclosing list or block.
///
/// Where it can't tell, it stops early, so the count errs towards more code, never less: a
/// comma in a field's generic arguments (`Map<A, B>`) ends the field, and a `let` whose value
/// has two braced blocks (`if a { b } else { c }`) ends after the first.
fn item_end(code: &str, from: usize) -> usize {
    let bytes = code.as_bytes();
    let is_item = starts_item(bytes, skip_attributes(bytes, from));
    let mut depth = 0usize;
    for (index, &byte) in bytes.iter().enumerate().skip(from) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => return index,
            b')' | b']' => depth -= 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let after = skip_whitespace(bytes, index + 1);
                    let separator = bytes.get(after) == Some(&b';') || (!is_item && bytes.get(after) == Some(&b','));
                    return if separator { after + 1 } else { index + 1 };
                }
            }
            b';' if depth == 0 => return index + 1,
            b',' if depth == 0 && !is_item => return index + 1,
            _ => {}
        }
    }
    bytes.len()
}

/// Whether the code at `at` starts an item: its first word, after `pub` and any `(…)`
/// restriction (`pub(in crate::a)`), is one of [`ITEM_KEYWORDS`].
fn starts_item(bytes: &[u8], at: usize) -> bool {
    let word_at = |at: usize| {
        let length = bytes
            .get(at..)
            .unwrap_or_default()
            .iter()
            .take_while(|&&b| is_ident(b))
            .count();
        (bytes.get(at..at + length).unwrap_or_default(), at + length)
    };
    let (mut word, after) = word_at(at);
    if word == b"pub" {
        let mut next = skip_whitespace(bytes, after);
        if bytes.get(next) == Some(&b'(') {
            next = matching(bytes, next).map_or(bytes.len(), |close| skip_whitespace(bytes, close + 1));
        }
        word = word_at(next).0;
    }
    ITEM_KEYWORDS.iter().any(|keyword| keyword.as_bytes() == word)
}

/// The offset of the first byte from `from` that is neither whitespace nor part of an outer attribute.
fn skip_attributes(bytes: &[u8], from: usize) -> usize {
    let mut at = skip_whitespace(bytes, from);
    while bytes.get(at) == Some(&b'#') {
        let open = skip_whitespace(bytes, at + 1);
        let Some(close) = (bytes.get(open) == Some(&b'['))
            .then(|| matching(bytes, open))
            .flatten()
        else {
            break;
        };
        at = skip_whitespace(bytes, close + 1);
    }
    at
}

/// The value of a `#[path = "…"]` attribute on the item whose `#[cfg(test)]` starts at `cfg`
/// and which ends at `to` (before or after the cfg), read from the unmasked `text` (masking hides
/// the value but keeps the offsets).
fn path_attribute(code: &str, text: &str, cfg: usize, to: usize) -> Option<String> {
    let attributes = attributes(code.get(..to)?);
    // The start of the run of outer attributes that holds the cfg.
    let mut from = cfg;
    while let Some(before) = attributes.iter().find(|attribute| {
        !attribute.inner
            && attribute.end <= from
            && code.get(attribute.end..from).is_some_and(|gap| gap.trim().is_empty())
    }) {
        from = before.start;
    }
    let attribute = attributes.iter().find(|attribute| {
        attribute.start >= from
            && attribute
                .content
                .split('=')
                .next()
                .is_some_and(|key| key.trim() == "path")
    })?;
    let raw = text.get(attribute.start..attribute.end)?;
    let open = raw.find('"')?;
    let close = raw.rfind('"')?;
    raw.get(open + 1..close).map(str::to_owned)
}

/// `name` when an item's text is a module declared out of line (`… mod name;`).
fn out_of_line_module(item: &str) -> Option<&str> {
    let tokens = tokens(item);
    match tokens.as_slice() {
        [.., module, Token::Word(name, _), semicolon] if module.is("mod") && semicolon.is_punct(b';') => Some(name),
        _ => None,
    }
}

fn skip_whitespace(bytes: &[u8], from: usize) -> usize {
    from + bytes
        .get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|byte| byte.is_ascii_whitespace())
        .count()
}

/// Whether `byte` can be part of an identifier (non-ASCII bytes count: Rust allows Unicode identifiers).
fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

fn tokens(code: &str) -> Vec<Token<'_>> {
    let bytes = code.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte.is_ascii_whitespace() {
            index += 1;
        } else if is_ident(byte) {
            let length = bytes
                .get(index..)
                .unwrap_or_default()
                .iter()
                .take_while(|&&b| is_ident(b))
                .count();
            found.push(Token::Word(code.get(index..index + length).unwrap_or_default(), index));
            index += length;
        } else {
            found.push(Token::Punct(byte, index));
            index += 1;
        }
    }
    found
}

/// What sits at a byte offset: a comment, or a literal whose inside runs `open..close`.
#[derive(Debug, PartialEq, Eq)]
enum Span {
    Comment { end: usize },
    Literal { open: usize, close: usize, end: usize },
}

/// `text` with comments blanked and literal contents masked; see [`Source::code`].
fn mask(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let after_ident = index > 0 && bytes.get(index - 1).copied().is_some_and(is_ident);
        match span_at(bytes, index, after_ident) {
            Some(Span::Comment { end }) => {
                let comment = bytes.get(index..end).unwrap_or_default();
                out.extend(comment.iter().map(|&b| if b == b'\n' { b'\n' } else { b' ' }));
                index = end;
            }
            Some(Span::Literal { open, close, end }) => {
                out.extend_from_slice(bytes.get(index..open).unwrap_or_default());
                let inside = bytes.get(open..close).unwrap_or_default();
                out.extend(inside.iter().map(|&b| if b.is_ascii_whitespace() { b } else { b'x' }));
                out.extend_from_slice(bytes.get(close..end).unwrap_or_default());
                index = end;
            }
            None => {
                out.push(byte);
                index += 1;
            }
        }
    }
    // Masking replaces whole characters with ASCII and copies the rest, so the bytes stay UTF-8.
    let code = String::from_utf8(out);
    debug_assert!(code.is_ok(), "masking must keep the text UTF-8");
    code.unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// The comment or literal that starts at `index`, if any. `after_ident` says whether the byte
/// before is part of an identifier, in which case `r`, `b` and `c` are not literal prefixes.
fn span_at(bytes: &[u8], index: usize, after_ident: bool) -> Option<Span> {
    let at = |offset: usize| bytes.get(index + offset).copied();
    match (at(0)?, at(1)) {
        (b'/', Some(b'/')) => Some(Span::Comment {
            end: bytes
                .iter()
                .skip(index)
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |n| index + n),
        }),
        (b'/', Some(b'*')) => Some(Span::Comment {
            end: block_comment_end(bytes, index),
        }),
        (b'"', _) => Some(quoted(bytes, index + 1)),
        (b'\'', _) => char_literal(bytes, index),
        (b'b', Some(b'\'')) if !after_ident => char_literal(bytes, index + 1),
        (b'b' | b'c', Some(b'"')) if !after_ident => Some(quoted(bytes, index + 2)),
        (b'b' | b'c', Some(b'r')) if !after_ident => raw_string(bytes, index + 1),
        (b'r', _) if !after_ident => raw_string(bytes, index),
        _ => None,
    }
}

/// The end of the block comment that opens at `index`; block comments nest.
fn block_comment_end(bytes: &[u8], index: usize) -> usize {
    let mut depth = 0usize;
    let mut at = index;
    while let Some(&byte) = bytes.get(at) {
        let next = bytes.get(at + 1).copied();
        if byte == b'/' && next == Some(b'*') {
            depth += 1;
            at += 2;
        } else if byte == b'*' && next == Some(b'/') {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return at;
            }
        } else {
            at += 1;
        }
    }
    bytes.len()
}

/// A string whose inside starts at `open` (after its `"`), with backslash escapes.
fn quoted(bytes: &[u8], open: usize) -> Span {
    let mut at = open;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'\\' => at += 2,
            b'"' => {
                return Span::Literal {
                    open,
                    close: at,
                    end: at + 1,
                };
            }
            _ => at += 1,
        }
    }
    Span::Literal {
        open,
        close: bytes.len(),
        end: bytes.len(),
    }
}

/// A raw string at `r` (index of the `r`): `r"…"`, `r#"…"#`, and so on. `None` for a raw
/// identifier (`r#type`) or a plain `r`.
fn raw_string(bytes: &[u8], r: usize) -> Option<Span> {
    let hashes = bytes.get(r + 1..)?.iter().take_while(|&&b| b == b'#').count();
    if bytes.get(r + 1 + hashes) != Some(&b'"') {
        return None;
    }
    let open = r + 2 + hashes;
    let closing = |at: usize| {
        bytes.get(at) == Some(&b'"')
            && bytes
                .get(at + 1..at + 1 + hashes)
                .is_some_and(|h| h.iter().all(|&b| b == b'#'))
    };
    let close = (open..bytes.len()).find(|&at| closing(at)).unwrap_or(bytes.len());
    Some(Span::Literal {
        open,
        close,
        end: (close + 1 + hashes).min(bytes.len()),
    })
}

/// A character literal at `quote` (the index of its `'`), or `None` for a lifetime or label.
fn char_literal(bytes: &[u8], quote: usize) -> Option<Span> {
    let open = quote + 1;
    let first = *bytes.get(open)?;
    let close = if first == b'\\' {
        // Skip the escaped byte (it may be `'`), then find the closing quote: `'\''`, `'\u{1F600}'`.
        open + 2 + bytes.get(open + 2..)?.iter().position(|&b| b == b'\'')?
    } else {
        let width = utf8_width(first);
        if bytes.get(open + width) != Some(&b'\'') {
            return None;
        }
        open + width
    };
    Some(Span::Literal {
        open,
        close,
        end: close + 1,
    })
}

/// The length of the UTF-8 sequence that starts with byte `lead`.
fn utf8_width(lead: u8) -> usize {
    match lead.leading_ones() {
        2 => 2,
        3 => 3,
        4 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(text: &str) -> usize {
        Source::new(text).non_test_lines()
    }

    #[test]
    fn blank_and_comment_lines_should_not_count() {
        let text = "// a\n\n/// doc\n//! inner doc\nfn f() {}\n   \n/* one\n two */\n/** doc */\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn a_line_with_code_and_a_comment_should_count() {
        assert_eq!(count("fn f() {} // trailing\n/* lead */ fn g() {}\n"), 2);
    }

    #[test]
    fn nested_block_comments_should_be_skipped_whole() {
        assert_eq!(count("/* outer /* inner */ still comment\n*/\nfn f() {}\n"), 1);
    }

    #[test]
    fn a_test_module_should_not_count() {
        let text = "fn f() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn code_after_a_test_module_should_count() {
        let text = "#[cfg(test)]\nmod tests {\n    fn t() {}\n}\nfn after() {}\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn braces_in_strings_should_not_end_a_test_module() {
        let text = "#[cfg(test)]\nmod tests {\n    const A: &str = \"}\";\n    const B: &str = r#\"}\"}\"#;\n    \
                    const C: char = '}';\n    const D: u8 = b'}';\n    const E: &[u8] = br\"}\";\n    // }\n    \
                    /* } */\n    fn t() {}\n}\nfn f() {}\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn an_escaped_quote_should_not_end_a_string() {
        let text =
            "#[cfg(test)]\nmod tests {\n    const A: &str = \"\\\"}\";\n    const B: char = '\\'';\n}\nfn f() {}\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn lifetimes_should_not_start_char_literals() {
        let text = "#[cfg(test)]\nmod tests {\n    fn t<'a>(x: &'a str) -> &'a str { x }\n}\nfn f() {}\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn a_multi_line_string_should_count_its_non_blank_lines() {
        assert_eq!(count("const S: &str = \"\nline\n\n// not a comment\n\";\n"), 4);
    }

    #[test]
    fn a_test_item_ending_in_a_semicolon_should_not_count() {
        assert_eq!(
            count("#[cfg(test)]\nuse std::fmt;\n#[cfg(test)]\nconst A: [u8; 2] = [1, 2];\nfn f() {}\n"),
            1
        );
    }

    #[test]
    fn a_test_only_variant_should_end_at_its_comma() {
        let text = "enum E {\n    #[cfg(test)]\n    Fake,\n    Real,\n}\n";
        assert_eq!(count(text), 3);
    }

    #[test]
    fn a_test_only_variant_with_fields_should_end_at_its_comma() {
        let text = "enum E {\n    Real,\n    #[cfg(test)]\n    Fake {\n        x: u8,\n    },\n}\n";
        assert_eq!(count(text), 3);
    }

    #[test]
    fn a_test_only_last_field_should_end_at_the_closing_brace() {
        let text = "struct S {\n    real: u8,\n    #[cfg(test)]\n    fake: u8\n}\n";
        assert_eq!(count(text), 3);
    }

    #[test]
    fn a_test_function_returning_a_generic_result_should_be_test_code() {
        let text = "#[cfg(test)]\npub(crate) fn f() -> Result<(), E> {\n    Ok(())\n}\nfn g() {}\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn a_test_struct_with_more_attributes_should_be_test_code() {
        let text = "#[cfg(test)]\n#[derive(Debug, Clone)]\nstruct Fake {\n    a: u8,\n    b: u8,\n}\nfn f() {}\n";
        assert_eq!(count(text), 1);
    }

    #[test]
    fn cfg_all_test_should_count_as_test_code() {
        assert_eq!(
            count("#[cfg(all(test, feature = \"x\"))]\nfn helper() {\n}\nfn f() {}\n"),
            1
        );
    }

    #[test]
    fn other_cfgs_should_count_as_code() {
        assert_eq!(count("#[cfg(not(test))]\nfn f() {\n}\n"), 3);
    }

    #[test]
    fn an_inner_cfg_test_should_make_the_file_test_code() {
        assert_eq!(count("//! Helpers.\n#![cfg(test)]\nfn helper() {}\n"), 0);
    }

    #[test]
    fn an_out_of_line_test_module_should_be_recorded_with_its_path() {
        let source = Source::new("#[cfg(test)]\n#[path = \"t.rs\"]\nmod tests;\nfn f() {}\n");
        let module = TestModule {
            name: "tests".to_owned(),
            path: Some("t.rs".to_owned()),
        };
        assert_eq!((source.non_test_lines(), source.test_modules), (1, vec![module]));
    }

    #[test]
    fn a_path_attribute_before_the_cfg_should_name_the_test_module_file() {
        let source = Source::new("#[path = \"t.rs\"]\n#[cfg(test)]\nmod tests;\nfn f() {}\n");
        let paths: Vec<_> = source.test_modules.iter().map(|module| module.path.clone()).collect();
        assert_eq!(paths, [Some("t.rs".to_owned())]);
    }

    #[test]
    fn a_test_macro_call_should_end_at_its_closing_brace() {
        assert_eq!(
            count("#[cfg(test)]\nproptest!{x}\nfn real(){ a(); }\nconst X: u8 = 1;"),
            2
        );
    }

    #[test]
    fn code_after_a_multi_line_test_macro_call_should_count() {
        assert_eq!(
            count("#[cfg(test)]\nproptest! {\n    x\n}\nfn real() {\n    a();\n}\nconst X: u8 = 1;\n"),
            4
        );
    }

    #[test]
    fn a_restricted_visibility_should_still_start_an_item() {
        assert_eq!(
            count("#[cfg(test)]\npub(in crate::a) fn f() {\n    a();\n}\nfn real() {}\n"),
            1
        );
    }

    #[test]
    fn a_test_match_arm_without_a_comma_should_end_at_its_block() {
        let text = "match x {\n    #[cfg(test)]\n    A => {\n        a()\n    }\n    B => b(),\n}\n";
        assert_eq!(count(text), 3);
    }

    #[test]
    fn a_test_statement_block_should_end_at_its_brace() {
        assert_eq!(
            count("fn f() {\n    #[cfg(test)]\n    {\n        a();\n    }\n    b();\n}\n"),
            3
        );
    }

    #[test]
    fn an_inner_cfg_test_in_an_inline_module_should_make_only_that_module_test_code() {
        assert_eq!(
            count("mod helpers {\n    #![cfg(test)]\n    fn h() {}\n}\nfn f() {}\n"),
            1
        );
    }

    #[test]
    fn cfg_all_with_test_anywhere_should_count_as_test_code() {
        let cfgs = [
            "cfg(all(feature = \"x\", test))",
            "cfg(all(test))",
            "cfg(all(not(test), unix))",
            "cfg(any(test, unix))",
        ]
        .map(is_test_cfg);
        assert_eq!(cfgs, [true, true, false, false]);
    }

    #[test]
    fn a_raw_identifier_should_stay_code() {
        assert_eq!(Source::new("let r#type = 1;").code, "let r#type = 1;");
    }

    #[test]
    fn masking_should_keep_length_and_line_breaks() {
        let text = "let s = \"a\nb\"; // é\nlet c = 'é';";
        let code = Source::new(text).code;
        assert_eq!((code.len(), code.matches('\n').count()), (text.len(), 2));
    }

    #[test]
    fn keywords_in_comments_and_strings_should_be_hidden() {
        let source = Source::new("// pub struct A;\nconst S: &str = \"pub struct B\";\npub struct C;\n");
        let words: Vec<_> = source.tokens().into_iter().filter(|token| token.is("struct")).collect();
        assert_eq!(words.len(), 1);
    }

    #[test]
    fn attributes_should_be_found_with_their_content() {
        let source =
            Source::new("#![forbid(unsafe_code)]\n# [ expect(clippy::too_many_lines, reason = \"]\") ]\nfn f() {}\n");
        let found: Vec<(bool, &str)> = source
            .attributes()
            .iter()
            .map(|a| (a.inner, a.content.trim()))
            .collect();
        assert_eq!(
            found,
            [
                (true, "forbid(unsafe_code)"),
                (false, "expect(clippy::too_many_lines, reason = \"x\")")
            ]
        );
    }
}
