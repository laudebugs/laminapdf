//! Zip codes, phone numbers and social security numbers: `AFSpecial_Format` and
//! `AFSpecial_Keystroke`, through `util.printx` and the keystroke masks (`9` a digit, `A` a
//! letter, `O` either, `X` anything; other characters stand for themselves).

use super::SpecialFormat;

impl SpecialFormat {
    /// The masks a value may be typed to: the phone's seven digits or ten with the area code.
    const fn masks(self) -> &'static [&'static str] {
        match self {
            Self::Zip => &["99999"],
            Self::ZipPlus4 => &["99999-9999"],
            Self::Phone => &["999-9999", "(999) 999-9999"],
            Self::Ssn => &["999-99-9999"],
        }
    }

    /// What `AFSpecial_Format` shows for `text`: its digits written into the format's pattern.
    pub(crate) fn show(self, text: &str) -> String {
        let pattern = match self {
            Self::Zip => "99999",
            Self::ZipPlus4 => "99999-9999",
            Self::Phone if printx("9999999999", text).len() >= 10 => "(999) 999-9999",
            Self::Phone => "999-9999",
            Self::Ssn => "999-99-9999",
        };
        printx(pattern, text)
    }

    /// Whether `text` fits the format while typed or once committed, as `AFSpecial_Keystroke`
    /// checks it: as typed, then without its `-`, `(`, `)` and spaces.
    pub(crate) fn accepts(self, text: &str, commit: bool) -> bool {
        if text.is_empty() {
            return true;
        }
        let bare: String = text
            .chars()
            .filter(|c| !matches!(c, '-' | '(' | ')') && !c.is_whitespace())
            .collect();
        self.masks().iter().any(|mask| {
            fits(mask, text, commit) || {
                let bare_mask: String = mask
                    .chars()
                    .filter(|c| !matches!(c, '-' | '(' | ')') && !c.is_whitespace())
                    .collect();
                fits(&bare_mask, &bare, commit)
            }
        })
    }
}

/// Whether `value` fits `mask`: whole when committing, as far as it goes while typing.
fn fits(mask: &str, value: &str, commit: bool) -> bool {
    let (m, v) = (mask.chars().count(), value.chars().count());
    if v > m || (commit && v < m) {
        return false;
    }
    mask.chars().zip(value.chars()).all(|(k, c)| match k {
        '9' => c.is_ascii_digit(),
        'A' => c.is_ascii_alphabetic(),
        'O' => c.is_ascii_alphanumeric(),
        'X' => true,
        other => other == c,
    })
}

/// `source` written into `pattern` as `util.printx` writes it: `9` takes the next digit, `A`
/// the next letter, `X` the next letter or digit, `?` the next character, `*` the rest, `>`, `<`
/// and `=` switch to upper, lower and unchanged case, and `\` writes the next character.
pub(crate) fn printx(pattern: &str, source: &str) -> String {
    let src: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut out = String::new();
    let mut case: fn(char) -> String = |c| c.to_string();
    let mut escaped = false;
    for command in pattern.chars() {
        if escaped {
            out.push(command);
            escaped = false;
            continue;
        }
        if i >= src.len() {
            break;
        }
        let mut next = |accept: fn(&char) -> bool| {
            while let Some(&c) = src.get(i) {
                i += 1;
                if accept(&c) {
                    return Some(c);
                }
            }
            None
        };
        match command {
            '?' => {
                if let Some(&c) = src.get(i) {
                    out.push_str(&case(c));
                    i += 1;
                }
            }
            'X' => {
                if let Some(c) = next(char::is_ascii_alphanumeric) {
                    out.push_str(&case(c));
                }
            }
            'A' => {
                if let Some(c) = next(char::is_ascii_alphabetic) {
                    out.push_str(&case(c));
                }
            }
            '9' => {
                if let Some(c) = next(char::is_ascii_digit) {
                    out.push(c);
                }
            }
            '*' => {
                while let Some(&c) = src.get(i) {
                    out.push_str(&case(c));
                    i += 1;
                }
            }
            '\\' => escaped = true,
            '>' => case = |c| c.to_uppercase().collect(),
            '<' => case = |c| c.to_lowercase().collect(),
            '=' => case = |c| c.to_string(),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_special_format_writes_its_digits_into_its_pattern() {
        assert_eq!(
            [
                SpecialFormat::Zip.show("12345"),
                SpecialFormat::ZipPlus4.show("123456789"),
                SpecialFormat::Phone.show("5551234567"),
                SpecialFormat::Phone.show("1234567"),
                SpecialFormat::Ssn.show("123 45 6789"),
            ],
            ["12345", "12345-6789", "(555) 123-4567", "123-4567", "123-45-6789"]
        );
    }

    #[test]
    fn a_phone_number_is_typed_with_or_without_its_marks() {
        let phone = SpecialFormat::Phone;
        assert_eq!(
            [
                phone.accepts("(555) 12", false),
                phone.accepts("555123", false),
                phone.accepts("5551234567", true),
                phone.accepts("555-12a", false),
                phone.accepts("55512", true),
            ],
            [true, true, true, false, false]
        );
    }

    #[test]
    fn printx_switches_case_and_escapes() {
        assert_eq!(printx(">AAA<A\\9 9", "abcd5"), "ABCd9 5");
    }
}
