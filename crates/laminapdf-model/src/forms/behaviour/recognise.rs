//! The closed list of helper calls (ADR 0021 §2): a script is one call to a listed `AF*`
//! function with literal arguments, optionally ending in `;`, or it is not handled.

use super::tokens::{Token, tokens};
use super::{
    CalculateFunction, DatePattern, FieldBehaviour, FieldTrigger, NegativeStyle, NumberFormat, PercentFormat,
    RangeValidate, SeparatorStyle, SimpleCalculate, SpecialFormat,
};

/// The most decimals a number or percent format may ask for; a call asking for more is not
/// handled.
const MAX_DECIMALS: f64 = 20.0;

/// The longest date or time pattern a call may give, in characters.
const MAX_PATTERN: usize = 128;

/// `AFDate_Format`'s patterns, by index (pdf.js's `DateFormats`).
const DATE_FORMATS: [&str; 14] = [
    "m/d",
    "m/d/yy",
    "mm/dd/yy",
    "mm/yy",
    "d-mmm",
    "d-mmm-yy",
    "dd-mmm-yy",
    "yy-mm-dd",
    "mmm-yy",
    "mmmm-yy",
    "mmm d, yyyy",
    "mmmm d, yyyy",
    "m/d/yy h:MM tt",
    "m/d/yy HH:MM",
];

/// `AFTime_Format`'s patterns, by index.
const TIME_FORMATS: [&str; 4] = ["HH:MM", "h:MM tt", "HH:MM:ss", "h:MM:ss tt"];

/// A literal argument.
#[derive(Debug, Clone, PartialEq)]
enum Arg {
    Number(f64),
    Str(String),
    Bool(bool),
    /// An array literal or `new Array (…)` of strings.
    Strings(Vec<String>),
}

/// The behaviour `script` is a helper call for under `trigger`, or `None` when it is not one:
/// a call the list doesn't name, the wrong trigger for it, or arguments that aren't literals of
/// the types it takes.
pub(crate) fn recognise(script: &str, trigger: FieldTrigger) -> Option<FieldBehaviour> {
    let (name, args) = call(&tokens(script)?)?;
    let behaviour = match (name.as_str(), args.as_slice()) {
        (
            "AFNumber_Format" | "AFNumber_Keystroke",
            [
                Arg::Number(decimals),
                Arg::Number(separator),
                Arg::Number(negative),
                Arg::Number(_),
                Arg::Str(currency),
                Arg::Bool(prepend),
            ],
        ) => FieldBehaviour::NumberFormat(NumberFormat {
            decimals: decimals_of(*decimals)?,
            separator: SeparatorStyle::of(*separator)?,
            negative: NegativeStyle::of(*negative)?,
            currency: currency.clone(),
            prepend: *prepend,
        }),
        ("AFPercent_Format" | "AFPercent_Keystroke", [Arg::Number(decimals), Arg::Number(separator), rest @ ..]) => {
            let prepend = match rest {
                [] => false,
                [Arg::Bool(b)] if name == "AFPercent_Format" => *b,
                _ => return None,
            };
            FieldBehaviour::PercentFormat(PercentFormat {
                decimals: decimals_of(*decimals)?,
                separator: SeparatorStyle::of(*separator)?,
                prepend,
            })
        }
        ("AFDate_FormatEx" | "AFDate_KeystrokeEx", [Arg::Str(pattern)]) => {
            FieldBehaviour::DateFormat(pattern_of(pattern)?)
        }
        ("AFDate_Format" | "AFDate_Keystroke", [Arg::Number(i)]) => {
            FieldBehaviour::DateFormat(pattern_of(index(&DATE_FORMATS, *i)?)?)
        }
        ("AFTime_FormatEx" | "AFTime_KeystrokeEx", [Arg::Str(pattern)]) => {
            FieldBehaviour::TimeFormat(pattern_of(pattern)?)
        }
        ("AFTime_Format" | "AFTime_Keystroke", [Arg::Number(i)]) => {
            FieldBehaviour::TimeFormat(pattern_of(index(&TIME_FORMATS, *i)?)?)
        }
        ("AFSpecial_Format" | "AFSpecial_Keystroke", [Arg::Number(i)]) => FieldBehaviour::SpecialFormat(
            *[
                SpecialFormat::Zip,
                SpecialFormat::ZipPlus4,
                SpecialFormat::Phone,
                SpecialFormat::Ssn,
            ]
            .get(whole(*i)?)?,
        ),
        ("AFRange_Validate", [Arg::Bool(above), Arg::Number(min), Arg::Bool(below), Arg::Number(max)]) => {
            FieldBehaviour::RangeValidate(RangeValidate {
                min: above.then_some(*min),
                max: below.then_some(*max),
            })
        }
        ("AFSimple_Calculate", [Arg::Str(function), fields]) => FieldBehaviour::SimpleCalculate(SimpleCalculate {
            function: CalculateFunction::of(function)?,
            fields: match fields {
                Arg::Strings(names) => names.clone(),
                // A list in one string, as `AFMakeArrayFromList` splits it.
                Arg::Str(list) => list
                    .split(',')
                    .map(|n| n.strip_prefix(' ').unwrap_or(n).to_owned())
                    .collect(),
                _ => return None,
            },
        }),
        _ => return None,
    };
    (expected_trigger(&name)? == trigger).then_some(behaviour)
}

/// The trigger a helper belongs to: `K` for keystrokes, `F` formats, `V` validates, `C`
/// calculates.
fn expected_trigger(name: &str) -> Option<FieldTrigger> {
    Some(if name.ends_with("_Keystroke") || name.ends_with("_KeystrokeEx") {
        FieldTrigger::Keystroke
    } else if name.ends_with("_Format") || name.ends_with("_FormatEx") {
        FieldTrigger::Format
    } else if name == "AFRange_Validate" {
        FieldTrigger::Validate
    } else if name == "AFSimple_Calculate" {
        FieldTrigger::Calculate
    } else {
        return None;
    })
}

/// A call `name(args…)`, then an optional `;`, and nothing else.
fn call(tokens: &[Token]) -> Option<(String, Vec<Arg>)> {
    let [Token::Ident(name), Token::Punct('('), rest @ ..] = tokens else {
        return None;
    };
    let mut rest = rest;
    let mut args = Vec::new();
    if let [Token::Punct(')'), after @ ..] = rest {
        rest = after;
    } else {
        loop {
            let (arg, after) = arg(rest)?;
            args.push(arg);
            match after {
                [Token::Punct(','), after @ ..] => rest = after,
                [Token::Punct(')'), after @ ..] => {
                    rest = after;
                    break;
                }
                _ => return None,
            }
        }
    }
    matches!(rest, [] | [Token::Punct(';')]).then(|| (name.clone(), args))
}

/// One literal argument and the tokens after it.
fn arg(tokens: &[Token]) -> Option<(Arg, &[Token])> {
    Some(match tokens {
        [Token::Number(n), rest @ ..] | [Token::Punct('+'), Token::Number(n), rest @ ..] => (Arg::Number(*n), rest),
        [Token::Punct('-'), Token::Number(n), rest @ ..] => (Arg::Number(-n), rest),
        [Token::Str(s), rest @ ..] => (Arg::Str(s.clone()), rest),
        [Token::Ident(b), rest @ ..] if b == "true" || b == "false" => (Arg::Bool(b == "true"), rest),
        [Token::Punct('['), rest @ ..] => {
            let (names, rest) = strings(rest, ']')?;
            (Arg::Strings(names), rest)
        }
        [Token::Ident(new), Token::Ident(array), Token::Punct('('), rest @ ..] if new == "new" && array == "Array" => {
            let (names, rest) = strings(rest, ')')?;
            (Arg::Strings(names), rest)
        }
        _ => return None,
    })
}

/// String literals separated by commas, up to `close`.
fn strings(tokens: &[Token], close: char) -> Option<(Vec<String>, &[Token])> {
    let mut out = Vec::new();
    let mut rest = tokens;
    if let [Token::Punct(c), after @ ..] = rest
        && *c == close
    {
        return Some((out, after));
    }
    loop {
        let [Token::Str(s), after @ ..] = rest else {
            return None;
        };
        out.push(s.clone());
        match after {
            [Token::Punct(','), after @ ..] => rest = after,
            [Token::Punct(c), after @ ..] if *c == close => return Some((out, after)),
            _ => return None,
        }
    }
}

/// `n` as a whole number from 0, when it is one.
fn whole(n: f64) -> Option<usize> {
    (n >= 0.0 && n.fract() == 0.0 && n < 1e9).then(|| {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "checked above")]
        let i = n as usize;
        i
    })
}

/// The number of decimals `n` asks for.
fn decimals_of(n: f64) -> Option<u8> {
    if n > MAX_DECIMALS {
        return None;
    }
    u8::try_from(whole(n)?).ok()
}

/// The pattern at index `i` of `table`.
fn index<'a>(table: &[&'a str], i: f64) -> Option<&'a str> {
    table.get(whole(i)?).copied()
}

/// A date or time pattern a call gives, when it is short enough to read.
fn pattern_of(pattern: &str) -> Option<DatePattern> {
    (!pattern.is_empty() && pattern.chars().count() <= MAX_PATTERN).then(|| DatePattern {
        pattern: pattern.to_owned(),
    })
}

impl SeparatorStyle {
    /// `sepStyle`: pdf.js clamps it to 0–4.
    fn of(n: f64) -> Option<Self> {
        if !n.is_finite() {
            return None;
        }
        Some(match n.floor() {
            n if n <= 0.0 => Self::CommaDot,
            1.0 => Self::Dot,
            2.0 => Self::DotComma,
            3.0 => Self::Comma,
            _ => Self::ApostropheDot,
        })
    }
}

impl NegativeStyle {
    /// `negStyle`, 0 to 3.
    fn of(n: f64) -> Option<Self> {
        [Self::Minus, Self::Red, Self::Parentheses, Self::RedParentheses]
            .get(whole(n)?)
            .copied()
    }
}

impl CalculateFunction {
    /// `cFunction`.
    fn of(name: &str) -> Option<Self> {
        Some(match name {
            "SUM" => Self::Sum,
            "AVG" => Self::Average,
            "PRD" => Self::Product,
            "MIN" => Self::Minimum,
            "MAX" => Self::Maximum,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(script: &str) -> Option<FieldBehaviour> {
        recognise(script, FieldTrigger::Format)
    }

    #[test]
    fn acrobats_number_format_is_a_number_format() {
        assert_eq!(
            format(r#"AFNumber_Format(2, 0, 0, 0, "$", true);"#),
            Some(FieldBehaviour::NumberFormat(NumberFormat {
                decimals: 2,
                separator: SeparatorStyle::CommaDot,
                negative: NegativeStyle::Minus,
                currency: "$".into(),
                prepend: true,
            }))
        );
    }

    #[test]
    fn a_calculation_takes_new_array_an_array_literal_or_a_list_in_a_string() {
        let fields = |script: &str| match recognise(script, FieldTrigger::Calculate) {
            Some(FieldBehaviour::SimpleCalculate(c)) => Some(c.fields),
            _ => None,
        };
        let both = Some(vec!["a".to_owned(), "b".to_owned()]);
        assert_eq!(
            [
                r#"AFSimple_Calculate("SUM", new Array ("a", "b"));"#,
                r#"AFSimple_Calculate("SUM", ["a", "b"])"#,
                r#"AFSimple_Calculate("SUM", "a, b");"#,
            ]
            .map(fields),
            [both.clone(), both.clone(), both]
        );
    }

    #[test]
    fn every_shape_that_is_not_one_literal_helper_call_is_not_handled() {
        let not_handled = [
            "event.value = 1;",
            r#"AFNumber_Format(2, 0, 0, 0, "", true); AFNumber_Format(2, 0, 0, 0, "", true);"#,
            r#"AFNumber_Format(n, 0, 0, 0, "", true);"#,
            r#"AFNumber_Format(1 + 1, 0, 0, 0, "", true);"#,
            r#"AFNumber_Format(2, 0, 0, 0, "" + "", true);"#,
            "this.getField(\"a\").value;",
            "AFMagic_Format(1);",
            r#"AFNumber_Format(2, 0, 0, 0, "", true)() "#,
            r#"AFNumber_Format(2, 0, 0, 0, "");"#,
            r#"AFNumber_Format(2, 0, 7, 0, "", true);"#,
            r#"AFNumber_Format(99, 0, 0, 0, "", true);"#,
            "AFDate_Format(14);",
            "AFSpecial_Format(4);",
            "AFNumber_Format;",
            "",
        ];
        assert!(not_handled.iter().all(|s| format(s).is_none()));
    }

    #[test]
    fn a_helper_under_the_wrong_trigger_is_not_handled() {
        assert_eq!(
            [
                recognise("AFRange_Validate(true, 0, true, 10);", FieldTrigger::Format),
                recognise(r#"AFNumber_Format(2, 0, 0, 0, "", true);"#, FieldTrigger::Keystroke),
            ],
            [None, None]
        );
    }

    #[test]
    fn the_index_forms_name_pdf_js_patterns() {
        assert_eq!(
            [format("AFDate_Format(11);"), format("AFTime_Format(1)")],
            [
                Some(FieldBehaviour::DateFormat(DatePattern {
                    pattern: "mmmm d, yyyy".into()
                })),
                Some(FieldBehaviour::TimeFormat(DatePattern {
                    pattern: "h:MM tt".into()
                })),
            ]
        );
    }
}
