//! Numbers as Acrobat's helpers read and write them: `AFMakeNumber`, `util.printf`'s `%,S.Nf`,
//! `AFNumber_Format`, `AFPercent_Format` and their keystroke checks. pdf.js's implementation
//! (`scripting_api/aform.js` and `util.js`, Apache-2.0) is the reading followed; where it
//! differs, the comment says why.

use super::{NegativeStyle, NumberFormat, PercentFormat, SeparatorStyle};

/// The number a field's text holds, as `AFMakeNumber` reads it: trimmed, its first comma
/// taken for a decimal point, then the longest decimal prefix (`12abc` is 12). `None` for no
/// number, or one that isn't finite.
#[must_use]
pub(crate) fn make_number(text: &str) -> Option<f64> {
    let text = text.trim().replacen(',', ".", 1);
    let bytes = text.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    let start_digits = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let mut digits = end > start_digits;
    if bytes.get(end) == Some(&b'.') {
        let point = end;
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        digits |= end > point + 1;
        if !digits {
            return None;
        }
    }
    if !digits {
        return None;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut e = end + 1;
        if matches!(bytes.get(e), Some(b'+' | b'-')) {
            e += 1;
        }
        let exponent_digits = e;
        while bytes.get(e).is_some_and(u8::is_ascii_digit) {
            e += 1;
        }
        if e > exponent_digits {
            end = e;
        }
    }
    text.get(..end)?.parse().ok().filter(|n: &f64| n.is_finite())
}

/// `n` as JavaScript's `String(n)` writes it: the shortest digits that read back as `n`, with
/// an exponent from 1e21 up and below 1e-6.
#[must_use]
pub(crate) fn js_string(n: f64) -> String {
    if n == 0.0 {
        return "0".to_owned();
    }
    let magnitude = n.abs();
    if (1e-6..1e21).contains(&magnitude) {
        return format!("{n}");
    }
    // Rust writes `1e21` and `1.5e-7`; JavaScript writes `1e+21` and `1.5e-7`.
    let text = format!("{n:e}");
    match text.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => format!("{mantissa}e+{exponent}"),
        _ => text,
    }
}

/// `abs` (in `[0, 1)`) to `decimals` places as JavaScript's `toFixed` gives it, `"0.xx"` or
/// `"1.00"`: the exact binary value rounded, a tie going up.
fn to_fixed(abs: f64, decimals: u8) -> String {
    // 1,100 places hold every digit of a double below 1, so this is the exact value.
    let exact = format!("{abs:.1100}");
    let (whole, fraction) = exact.split_once('.').unwrap_or((&exact, ""));
    let keep = usize::from(decimals);
    let mut digits: Vec<u8> = format!("{whole}{}", fraction.get(..keep).unwrap_or(fraction)).into_bytes();
    if fraction.as_bytes().get(keep).is_some_and(|d| *d >= b'5') {
        for d in digits.iter_mut().rev() {
            if *d == b'9' {
                *d = b'0';
            } else {
                *d += 1;
                break;
            }
        }
        if digits.iter().all(|d| *d == b'0') {
            digits.insert(0, b'1');
        }
    }
    let split = digits.len() - keep;
    let text = String::from_utf8_lossy(&digits).into_owned();
    match (text.get(..split), text.get(split..)) {
        (Some(w), Some(f)) if keep > 0 => format!("{w}.{f}"),
        (Some(w), _) => w.to_owned(),
        _ => text,
    }
}

/// `value` written as `util.printf("%,S.Nf", value)` writes it: `decimals` places, thousands
/// grouped and the point written by `separator`, a minus sign when it is negative.
///
/// pdf.js takes the sign from the whole part, so `-0.5` prints as `0.50`; this takes it from the
/// value, as C's `printf` and Acrobat do.
#[must_use]
pub(crate) fn printf(value: f64, decimals: u8, separator: SeparatorStyle) -> String {
    let (thousands, point) = separator.marks();
    let mut whole = value.abs().trunc();
    let fixed = to_fixed(value.abs() - whole, decimals);
    let fraction = if let Some((carry, digits)) = fixed.split_once('.') {
        if carry == "1" {
            whole += 1.0;
        }
        format!("{point}{digits}")
    } else {
        if fixed == "1" {
            whole += 1.0;
        }
        String::new()
    };
    let digits = format!("{whole:.0}");
    let grouped = if thousands.is_empty() || digits.len() <= 3 {
        digits
    } else {
        let mut out = String::new();
        let first = digits.len() % 3;
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (i + 3 - first) % 3 == 0 {
                out.push_str(thousands);
            }
            out.push(c);
        }
        out
    };
    let sign = if value < 0.0 { "-" } else { "" };
    format!("{sign}{grouped}{fraction}")
}

impl SeparatorStyle {
    /// The thousands mark and the decimal point.
    const fn marks(self) -> (&'static str, &'static str) {
        match self {
            Self::CommaDot => (",", "."),
            Self::Dot => ("", "."),
            Self::DotComma => (".", ","),
            Self::Comma => ("", ","),
            Self::ApostropheDot => ("'", "."),
        }
    }

    /// Whether the decimal point is a comma.
    const fn comma_point(self) -> bool {
        matches!(self, Self::DotComma | Self::Comma)
    }
}

impl NumberFormat {
    /// What `AFNumber_Format` shows for `text`, and whether it is drawn red; `None` when the
    /// text holds no number (pdf.js shows nothing then).
    pub(crate) fn show(&self, text: &str) -> Option<(String, bool)> {
        let value = make_number(text)?;
        let negative = value < 0.0;
        let parentheses = negative
            && matches!(
                self.negative,
                NegativeStyle::Parentheses | NegativeStyle::RedParentheses
            );
        let red = negative && matches!(self.negative, NegativeStyle::Red | NegativeStyle::RedParentheses);
        // The minus sign shows only in the minus style; prepended currency puts it first.
        let shown = if negative && (self.negative != NegativeStyle::Minus || self.prepend) {
            -value
        } else {
            value
        };
        let mut out = String::new();
        if negative && self.prepend && self.negative == NegativeStyle::Minus {
            out.push('-');
        }
        if parentheses {
            out.push('(');
        }
        if self.prepend {
            out.push_str(&self.currency);
        }
        out.push_str(&printf(shown, self.decimals, self.separator));
        if !self.prepend {
            out.push_str(&self.currency);
        }
        if parentheses {
            out.push(')');
        }
        Some((out, red))
    }
}

impl PercentFormat {
    /// What `AFPercent_Format` shows for `text`: the number times 100 and `%`. `None` when the
    /// text holds no number (pdf.js shows `%` alone; an empty field stays empty here).
    pub(crate) fn show(self, text: &str) -> Option<String> {
        let value = make_number(text)?;
        let digits = printf(value * 100.0, self.decimals, self.separator);
        Some(if self.prepend {
            format!("%{digits}")
        } else {
            format!("{digits}%")
        })
    }
}

/// Whether `text` is a number while being typed (`commit` false) or once committed, as
/// `AFNumber_Keystroke` checks it: digits, an optional sign and one decimal point (a comma for
/// the comma styles), no thousands marks.
pub(crate) fn keystroke_accepts(separator: SeparatorStyle, text: &str, commit: bool) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return true;
    }
    let point = if separator.comma_point() { ',' } else { '.' };
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (whole, fraction) = match body.split_once(point) {
        Some((w, f)) => (w, Some(f)),
        None => (body, None),
    };
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !fraction.is_none_or(digits) {
        return false;
    }
    !commit || !whole.is_empty() || fraction.is_some_and(|f| !f.is_empty())
}

/// The value a committed number takes: for the comma styles, the number written with a point,
/// as `AFNumber_Keystroke` stores `parseFloat` of it; otherwise the text as typed.
pub(crate) fn committed(separator: SeparatorStyle, text: &str) -> String {
    if separator.comma_point()
        && let Some(n) = make_number(text)
    {
        return js_string(n);
    }
    text.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(
        decimals: u8,
        separator: SeparatorStyle,
        negative: NegativeStyle,
        currency: &str,
        prepend: bool,
    ) -> NumberFormat {
        NumberFormat {
            decimals,
            separator,
            negative,
            currency: currency.into(),
            prepend,
        }
    }

    #[test]
    fn make_number_reads_a_prefix_and_a_comma_point() {
        assert_eq!(
            ["12abc", " 1,5 ", "-.5", "1e3", "e3", "", ".", "Infinity", "+7"].map(make_number),
            [
                Some(12.0),
                Some(1.5),
                Some(-0.5),
                Some(1000.0),
                None,
                None,
                None,
                None,
                Some(7.0)
            ]
        );
    }

    #[test]
    fn printf_groups_rounds_half_up_and_carries() {
        assert_eq!(
            [
                printf(1234.5, 2, SeparatorStyle::CommaDot),
                printf(1_234_567.891, 2, SeparatorStyle::ApostropheDot),
                printf(0.125, 2, SeparatorStyle::Dot),
                printf(9.999, 2, SeparatorStyle::DotComma),
                printf(-2.5, 0, SeparatorStyle::Comma),
                printf(-0.5, 2, SeparatorStyle::CommaDot),
            ],
            ["1,234.50", "1'234'567.89", "0.13", "10,00", "-3", "-0.50"]
        );
    }

    #[test]
    fn the_four_negative_styles_match_pdf_js() {
        let show = |negative, prepend| number(2, SeparatorStyle::CommaDot, negative, "$", prepend).show("-1234.5");
        assert_eq!(
            [
                show(NegativeStyle::Minus, true),
                show(NegativeStyle::Minus, false),
                show(NegativeStyle::Red, true),
                show(NegativeStyle::Parentheses, false),
                show(NegativeStyle::RedParentheses, true),
            ],
            [
                Some(("-$1,234.50".into(), false)),
                Some(("-1,234.50$".into(), false)),
                Some(("$1,234.50".into(), true)),
                Some(("(1,234.50$)".into(), false)),
                Some(("($1,234.50)".into(), true)),
            ]
        );
    }

    #[test]
    fn a_percent_multiplies_by_100() {
        let percent = |prepend| PercentFormat {
            decimals: 1,
            separator: SeparatorStyle::CommaDot,
            prepend,
        };
        assert_eq!(
            [
                percent(false).show("0.1234"),
                percent(true).show("12.5"),
                percent(false).show("x")
            ],
            [Some("12.3%".into()), Some("%1,250.0".into()), None]
        );
    }

    #[test]
    fn the_keystroke_check_takes_partial_numbers_while_typing_and_whole_ones_at_commit() {
        let dot = SeparatorStyle::CommaDot;
        assert_eq!(
            [
                keystroke_accepts(dot, "-", false),
                keystroke_accepts(dot, "-", true),
                keystroke_accepts(dot, "1.", true),
                keystroke_accepts(dot, "1,000", false),
                keystroke_accepts(SeparatorStyle::Comma, "1,5", true),
                keystroke_accepts(dot, "12a", false),
            ],
            [true, false, true, false, true, false]
        );
    }

    #[test]
    fn a_comma_style_commits_the_number_with_a_point() {
        assert_eq!(
            [
                committed(SeparatorStyle::DotComma, "1234,5"),
                committed(SeparatorStyle::CommaDot, "1234.5")
            ],
            ["1234.5", "1234.5"]
        );
    }

    #[test]
    fn js_string_writes_exponents_as_javascript_does() {
        assert_eq!(
            [
                js_string(4938.0),
                js_string(1e21),
                js_string(1.5e-7),
                js_string(0.1 + 0.2)
            ],
            ["4938", "1e+21", "1.5e-7", "0.30000000000000004"]
        );
    }
}
