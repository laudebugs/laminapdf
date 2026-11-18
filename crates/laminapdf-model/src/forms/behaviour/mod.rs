//! Form behaviour without JavaScript ([ADR 0021] §2): a field's `/AA` keystroke, format,
//! validate and calculate scripts read as tokens and matched against Acrobat's helper calls,
//! whose work native code then does. Nothing here evaluates a script.
//!
//! [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md

mod date;
mod number;
mod recognise;
mod scripts;
mod special;
mod tokens;

pub use scripts::{FormScript, ScriptLocation, ScriptState};

pub(crate) use recognise::recognise;

use super::{FieldValue, Form};

/// The trigger of a field's `/AA` a behaviour belongs to (§12.6.3, Table 199).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldTrigger {
    /// `K`: a keystroke, or the value committed.
    Keystroke,
    /// `F`: what the field shows for its value.
    Format,
    /// `V`: whether a committed value is accepted.
    Validate,
    /// `C`: the value, from other fields'.
    Calculate,
}

impl FieldTrigger {
    /// The four, in `/AA`'s key order.
    pub(crate) const ALL: [Self; 4] = [Self::Keystroke, Self::Format, Self::Validate, Self::Calculate];

    /// The `/AA` key.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Keystroke => "K",
            Self::Format => "F",
            Self::Validate => "V",
            Self::Calculate => "C",
        }
    }
}

/// One recognised helper call, as data ([ADR 0021] §2's list).
///
/// [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FieldBehaviour {
    /// `AFNumber_Format` or `AFNumber_Keystroke`.
    NumberFormat(NumberFormat),
    /// `AFPercent_Format` or `AFPercent_Keystroke`.
    PercentFormat(PercentFormat),
    /// `AFDate_FormatEx`, `AFDate_Format` or their keystroke forms.
    DateFormat(DatePattern),
    /// `AFTime_Format`, `AFTime_FormatEx` or their keystroke forms.
    TimeFormat(DatePattern),
    /// `AFSpecial_Format` or `AFSpecial_Keystroke`.
    SpecialFormat(SpecialFormat),
    /// `AFRange_Validate`.
    RangeValidate(RangeValidate),
    /// `AFSimple_Calculate`.
    SimpleCalculate(SimpleCalculate),
}

/// A number with a fixed count of decimals, grouped, perhaps with a currency.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NumberFormat {
    /// Digits after the point.
    pub decimals: u8,
    /// How thousands are grouped and the point written.
    pub separator: SeparatorStyle,
    /// How a negative number shows.
    pub negative: NegativeStyle,
    /// The currency symbol, empty for none.
    pub currency: String,
    /// Whether the currency comes first.
    pub prepend: bool,
}

/// `sepStyle`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SeparatorStyle {
    /// 0: `1,234.56`.
    CommaDot,
    /// 1: `1234.56`.
    Dot,
    /// 2: `1.234,56`.
    DotComma,
    /// 3: `1234,56`.
    Comma,
    /// 4: `1'234.56`.
    ApostropheDot,
}

/// `negStyle`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NegativeStyle {
    /// 0: a minus sign.
    Minus,
    /// 1: red, without a sign.
    Red,
    /// 2: in parentheses.
    Parentheses,
    /// 3: red, in parentheses.
    RedParentheses,
}

/// A percentage: the number times 100, with a `%`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PercentFormat {
    /// Digits after the point.
    pub decimals: u8,
    /// How thousands are grouped and the point written.
    pub separator: SeparatorStyle,
    /// Whether the `%` comes first.
    pub prepend: bool,
}

/// A date or time pattern, as `util.printd` writes it (`mmmm d, yyyy`, `h:MM tt`).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DatePattern {
    /// The pattern.
    pub pattern: String,
}

/// `AFSpecial_Format`'s kinds.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpecialFormat {
    /// 0: `12345`.
    Zip,
    /// 1: `12345-6789`.
    ZipPlus4,
    /// 2: `(555) 123-4567`, or `123-4567` without an area code.
    Phone,
    /// 3: `123-45-6789`.
    Ssn,
}

/// Bounds a number must keep, each inclusive and optional.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct RangeValidate {
    /// The least value accepted.
    pub min: Option<f64>,
    /// The greatest value accepted.
    pub max: Option<f64>,
}

/// A value computed from other fields'.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SimpleCalculate {
    /// What it computes.
    pub function: CalculateFunction,
    /// The fields it reads, by full name; a name of a field with kids reads every terminal
    /// field below it.
    pub fields: Vec<String>,
}

/// `AFSimple_Calculate`'s functions.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CalculateFunction {
    /// `SUM`.
    Sum,
    /// `AVG`.
    Average,
    /// `PRD`.
    Product,
    /// `MIN`.
    Minimum,
    /// `MAX`.
    Maximum,
}

/// What a field's scripts do, as far as they are helper calls.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Behaviour {
    /// `K`, when it is a helper call.
    pub keystroke: Option<FieldBehaviour>,
    /// `F`, when it is a helper call.
    pub format: Option<FieldBehaviour>,
    /// `V`, when it is a helper call.
    pub validate: Option<FieldBehaviour>,
    /// `C`, when it is a helper call.
    pub calculate: Option<SimpleCalculate>,
    /// The triggers whose scripts aren't helper calls: the field is filled without them.
    pub not_handled: Vec<FieldTrigger>,
}

/// What a field shows for its value.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShownValue {
    /// The text.
    pub text: String,
    /// Whether a negative number's style draws it red.
    pub red: bool,
}

/// Why a committed value is refused.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum InvalidValue {
    /// It isn't a number (`AFNumber_Keystroke`, `AFPercent_Keystroke`).
    NotANumber,
    /// It isn't a date or time in `pattern` (`AFDate_KeystrokeEx`, `AFTime_Keystroke`).
    NotADate {
        /// The pattern.
        pattern: String,
    },
    /// It doesn't fit the format's mask (`AFSpecial_Keystroke`).
    NotSpecial(SpecialFormat),
    /// It is outside the bounds (`AFRange_Validate`).
    OutOfRange(RangeValidate),
}

impl ShownValue {
    /// `text` as it is, in the field's own colour.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn plain(text: String) -> Self {
        Self { text, red: false }
    }
}

impl FieldBehaviour {
    /// What a format shows for `value`; `None` for a behaviour that isn't a format.
    ///
    /// A number or percent shows nothing for text that holds no number, as pdf.js does; a date
    /// it can't read shows as typed.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn show(&self, value: &str) -> Option<ShownValue> {
        let plain = ShownValue::plain;
        Some(match self {
            Self::NumberFormat(f) => match f.show(value) {
                Some((text, red)) => ShownValue { text, red },
                None => plain(String::new()),
            },
            Self::PercentFormat(f) => plain(f.show(value).unwrap_or_default()),
            Self::DateFormat(f) | Self::TimeFormat(f) => plain(if value.is_empty() {
                String::new()
            } else {
                date::parse(&f.pattern, value).map_or_else(|| value.to_owned(), |d| date::printd(&f.pattern, d))
            }),
            Self::SpecialFormat(f) => plain(f.show(value)),
            Self::RangeValidate(_) | Self::SimpleCalculate(_) => return None,
        })
    }

    /// Whether a keystroke behaviour takes `text` as the field's text while it is typed; every
    /// text, for a behaviour that isn't a keystroke check.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn accepts(&self, text: &str) -> bool {
        match self {
            Self::NumberFormat(NumberFormat { separator, .. })
            | Self::PercentFormat(PercentFormat { separator, .. }) => {
                number::keystroke_accepts(*separator, text, false)
            }
            Self::SpecialFormat(f) => f.accepts(text, false),
            _ => true,
        }
    }

    /// The value committing `text` stores, or why it is refused: the keystroke check at
    /// commit, then a range's bounds.
    ///
    /// # Errors
    /// [`InvalidValue`], what the check refuses.
    ///
    /// Added in 0.6.0.
    pub fn commit(&self, text: &str) -> Result<String, InvalidValue> {
        match self {
            Self::NumberFormat(NumberFormat { separator, .. })
            | Self::PercentFormat(PercentFormat { separator, .. }) => {
                if number::keystroke_accepts(*separator, text, true) {
                    Ok(number::committed(*separator, text))
                } else {
                    Err(InvalidValue::NotANumber)
                }
            }
            Self::DateFormat(f) | Self::TimeFormat(f) => {
                if text.is_empty() || date::parse(&f.pattern, text).is_some() {
                    Ok(text.to_owned())
                } else {
                    Err(InvalidValue::NotADate {
                        pattern: f.pattern.clone(),
                    })
                }
            }
            Self::SpecialFormat(f) => {
                if f.accepts(text, true) {
                    Ok(text.to_owned())
                } else {
                    Err(InvalidValue::NotSpecial(*f))
                }
            }
            Self::RangeValidate(r) => {
                let inside = number::make_number(text)
                    .is_none_or(|v| r.min.is_none_or(|min| v >= min) && r.max.is_none_or(|max| v <= max));
                if text.is_empty() || inside {
                    Ok(text.to_owned())
                } else {
                    Err(InvalidValue::OutOfRange(*r))
                }
            }
            Self::SimpleCalculate(_) => Ok(text.to_owned()),
        }
    }
}

impl Behaviour {
    /// What the field shows for `value`: its format's text, else the value.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn show(&self, value: &str) -> ShownValue {
        self.format
            .as_ref()
            .and_then(|f| f.show(value))
            .unwrap_or_else(|| ShownValue {
                text: value.to_owned(),
                red: false,
            })
    }

    /// The value committing `text` stores: the keystroke check, then validation.
    ///
    /// # Errors
    /// [`InvalidValue`], what a check refuses.
    ///
    /// Added in 0.6.0.
    pub fn commit(&self, text: &str) -> Result<String, InvalidValue> {
        let mut value = text.to_owned();
        for behaviour in [&self.keystroke, &self.validate].into_iter().flatten() {
            value = behaviour.commit(&value)?;
        }
        Ok(value)
    }
}

impl SimpleCalculate {
    /// The value it computes from `form`'s values, as `AFSimple_Calculate` does: each named
    /// field's terminal fields, a value that isn't a number counting 0, rounded to six places
    /// and written as JavaScript writes a number. A name the form doesn't have counts nothing.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn compute(&self, form: &Form) -> String {
        let mut values = Vec::new();
        for name in &self.fields {
            let prefix = format!("{name}.");
            for field in form
                .fields
                .iter()
                .filter(|f| f.name == *name || f.name.starts_with(&prefix))
            {
                let text = match &field.value {
                    Some(FieldValue::Text(t)) => t.clone(),
                    Some(FieldValue::Button(Some(s))) => s.clone(),
                    Some(FieldValue::Choice(c)) => c.first().cloned().unwrap_or_default(),
                    _ => String::new(),
                };
                values.push(number::make_number(&text).unwrap_or(0.0));
            }
        }
        if values.is_empty() {
            return "0".to_owned();
        }
        let sum = || {
            // Neumaier's compensated sum, close to `Math.sumPrecise`.
            let (mut total, mut carry) = (0.0_f64, 0.0_f64);
            for v in &values {
                let t = total + v;
                carry += if total.abs() >= v.abs() {
                    (total - t) + v
                } else {
                    (v - t) + total
                };
                total = t;
            }
            total + carry
        };
        let result = match self.function {
            CalculateFunction::Sum => sum(),
            #[expect(clippy::cast_precision_loss, reason = "a count of fields")]
            CalculateFunction::Average => sum() / values.len() as f64,
            CalculateFunction::Product => values.iter().product(),
            CalculateFunction::Minimum => values.iter().copied().fold(f64::INFINITY, f64::min),
            CalculateFunction::Maximum => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        };
        // `Math.round(1e6 * x) / 1e6`: halves round up, towards positive infinity.
        let scaled = 1e6 * result;
        let rounded = if scaled - scaled.floor() >= 0.5 {
            scaled.floor() + 1.0
        } else {
            scaled.floor()
        };
        number::js_string(rounded / 1e6)
    }
}
