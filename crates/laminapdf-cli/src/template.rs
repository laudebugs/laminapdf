//! Output templates ([ADR 0031], its batch amendment): one `-o` names every file a command
//! writes, with fields filled in per input and per page: `out/{stem}-{n:03}.png`.
//!
//! The fields are `{stem}` (the input's file name without its extension), `{name}` (the file
//! name), `{index}` (the input's position, from 0) and `{n}` (the page number, from 1). A width
//! after a colon pads a number with zeros (`{n:03}` is `007`). `{{` and `}}` are braces.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use std::fmt::Write as _;

use laminapdf::ErrorCode;

use crate::exit::CliError;
use crate::io::STDIO;

/// The fields a template can name.
const FIELDS: [&str; 4] = ["stem", "name", "index", "n"];

/// The widest zero padding a field takes.
const MAX_WIDTH: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Stem,
    Name,
    Index,
    Page,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Text(String),
    Field { field: Field, width: usize },
}

/// A parsed `-o` template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    parts: Vec<Part>,
}

/// What a template is filled in with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill<'a> {
    /// The input as given (a path, or `-`).
    pub input: &'a str,
    /// The input's position among the command's inputs, from 0.
    pub index: usize,
    /// The page number, from 1.
    pub page: u32,
}

/// `invalid-command-line` for a template, with the template in `data`.
fn invalid(template: &str, message: &str) -> CliError {
    CliError::new(ErrorCode::InvalidCommandLine, message).with("argument", template)
}

impl Template {
    /// Parses `text`.
    ///
    /// # Errors
    ///
    /// `invalid-command-line` for an unknown field (with `didYouMean`), a bad width, or a lone
    /// `{` or `}`.
    pub fn parse(text: &str) -> Result<Self, CliError> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    literal.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    literal.push('}');
                }
                '}' => {
                    return Err(invalid(
                        text,
                        "a `}` in the output template has no `{`: write `}}` for a brace",
                    ));
                }
                '{' => {
                    let mut inner = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(c) => inner.push(c),
                            None => return Err(invalid(text, "a `{` in the output template is never closed")),
                        }
                    }
                    if !literal.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut literal)));
                    }
                    parts.push(field(text, &inner)?);
                }
                c => literal.push(c),
            }
        }
        if !literal.is_empty() {
            parts.push(Part::Text(literal));
        }
        Ok(Self { parts })
    }

    /// Whether the template is `-`: standard output.
    #[must_use]
    pub fn is_stdout(&self) -> bool {
        matches!(self.parts.as_slice(), [Part::Text(t)] if t == STDIO)
    }

    /// Whether the template names the page number, so each page gets a file of its own.
    #[must_use]
    pub fn has_page(&self) -> bool {
        self.parts
            .iter()
            .any(|p| matches!(p, Part::Field { field: Field::Page, .. }))
    }

    /// Whether the template names the input (`{stem}`, `{name}` or `{index}`), so each input
    /// gets files of its own.
    #[must_use]
    pub fn has_input(&self) -> bool {
        self.parts.iter().any(|p| {
            matches!(
                p,
                Part::Field {
                    field: Field::Stem | Field::Name | Field::Index,
                    ..
                }
            )
        })
    }

    /// The path for `fill`.
    #[must_use]
    pub fn expand(&self, fill: Fill<'_>) -> String {
        let name = file_name(fill.input);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Field { field, width } => {
                    let _ = match field {
                        Field::Stem => write!(out, "{stem}"),
                        Field::Name => write!(out, "{name}"),
                        Field::Index => write!(out, "{:0width$}", fill.index, width = *width),
                        Field::Page => write!(out, "{:0width$}", fill.page, width = *width),
                    };
                }
            }
        }
        out
    }
}

/// The last component of `input`; `stdin` for standard input.
fn file_name(input: &str) -> &str {
    if input == STDIO {
        return "stdin";
    }
    input.rsplit(['/', '\\']).next().unwrap_or(input)
}

/// One `{…}` field.
fn field(template: &str, inner: &str) -> Result<Part, CliError> {
    let (name, width) = match inner.split_once(':') {
        Some((name, width)) => {
            let parsed = width
                .parse::<usize>()
                .ok()
                .filter(|w| width.starts_with('0') && *w <= MAX_WIDTH);
            let Some(width) = parsed else {
                return Err(invalid(
                    template,
                    "a width in the output template is a zero and up to 9 digits, as in {n:03}",
                ));
            };
            (name, width)
        }
        None => (inner, 0),
    };
    let field = match name {
        "stem" => Field::Stem,
        "name" => Field::Name,
        "index" => Field::Index,
        "n" => Field::Page,
        _ => {
            let known: Vec<String> = FIELDS.iter().map(|&f| f.to_owned()).collect();
            return Err(
                invalid(template, "the output template names a field that doesn't exist")
                    .with("field", name)
                    .with_did_you_mean(laminapdf::did_you_mean(name, &known)),
            );
        }
    };
    if width > 0 && matches!(field, Field::Stem | Field::Name) {
        return Err(invalid(template, "only {index} and {n} take a width"));
    }
    Ok(Part::Field { field, width })
}

#[cfg(test)]
mod tests {
    use super::{Fill, Template};

    fn expand(template: &str, input: &str, index: usize, page: u32) -> String {
        Template::parse(template)
            .map(|t| t.expand(Fill { input, index, page }))
            .unwrap_or_else(|e| e.diagnostic().message.clone())
    }

    #[test]
    fn fields_should_be_filled_in() {
        assert_eq!(
            expand("out/{stem}-{n:03}.png", "docs/report.pdf", 0, 7),
            "out/report-007.png"
        );
        assert_eq!(
            expand("{name}.{index:02}.{n}.png", "a/b.c.pdf", 3, 12),
            "b.c.pdf.03.12.png"
        );
        assert_eq!(expand("{{literal}}-{n}.png", "x.pdf", 0, 1), "{literal}-1.png");
        assert_eq!(expand("{stem}.png", "-", 0, 1), "stdin.png");
        assert_eq!(expand("{stem}.png", "noext", 0, 1), "noext.png");
    }

    #[test]
    fn a_bad_template_should_be_refused_with_a_suggestion() {
        for bad in ["{page}.png", "{n:3}.png", "{n:0x}.png", "{stem:02}", "{n", "n}"] {
            assert!(Template::parse(bad).is_err(), "{bad}");
        }
        let error = Template::parse("{stm}.png").unwrap_err();
        assert_eq!(error.diagnostic().did_you_mean, vec!["stem".to_owned()]);
    }

    #[test]
    fn it_should_say_what_it_names() {
        let parse = |t: &str| Template::parse(t).ok();
        assert!(parse("-").is_some_and(|t| t.is_stdout()));
        assert!(parse("{n}.png").is_some_and(|t| t.has_page() && !t.has_input()));
        assert!(parse("{index}.png").is_some_and(|t| t.has_input() && !t.has_page()));
    }
}
