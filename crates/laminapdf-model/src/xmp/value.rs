//! A value any XMP property can be set to (ISO 16684-1 §8.2): the simple types, a language
//! alternative, and ordered or unordered arrays of simple values. Structures are written only
//! through a whole packet.

use crate::date::PdfDate;

/// A real number for [`XmpValue::Real`]: finite, so it always has a decimal form, and compared
/// by its bits, so values can be `Eq`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
pub struct XmpReal(f64);

impl XmpReal {
    /// `value`, or `None` when it is infinite or NaN, which XMP can't write.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    /// The number.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for XmpReal {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for XmpReal {}

/// One entry of a language alternative: its `xml:lang` and its text.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LangText {
    /// The language, an RFC 3066 tag such as `en-GB`, or `x-default`.
    pub lang: String,
    /// The text in that language.
    pub text: String,
}

impl LangText {
    /// `text` in `lang`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(lang: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            lang: lang.into(),
            text: text.into(),
        }
    }
}

/// A property's new value.
///
/// ```
/// use laminapdf_model::xmp::{LangText, XmpValue};
///
/// let title = XmpValue::LangAlt(vec![LangText::new("x-default", "Minutes"), LangText::new("fr", "Procès-verbal")]);
/// assert_eq!(title.value_type(), "Lang Alt");
/// assert_eq!(XmpValue::Seq(vec![XmpValue::Integer(3)]).value_type(), "Seq Integer");
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmpValue {
    /// Text.
    Text(String),
    /// A URI, written as the property's `rdf:resource`.
    Uri(String),
    /// A whole number.
    Integer(i64),
    /// A real number.
    Real(XmpReal),
    /// `True` or `False`.
    Boolean(bool),
    /// A date, written in ISO 8601.
    Date(PdfDate),
    /// The same text in several languages (`rdf:Alt`); the `x-default` entry is written first.
    LangAlt(Vec<LangText>),
    /// An ordered array (`rdf:Seq`) of simple values of one type.
    Seq(Vec<Self>),
    /// An unordered array (`rdf:Bag`) of simple values of one type.
    Bag(Vec<Self>),
}

/// Why an [`XmpValue`] can't be written.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ValueError {
    /// An array or language alternative with no entries: remove the property instead.
    #[error("the array is empty")]
    Empty,
    /// An array holding an array or a language alternative.
    #[error("an array holds only simple values")]
    Nested,
    /// An array whose items aren't all of one type.
    #[error("an array's items differ in type")]
    Mixed,
    /// A language alternative with an empty or repeated language.
    #[error("a language is empty or repeated")]
    Language,
}

impl XmpValue {
    /// Whether the value is simple: text, a URI, a number, a boolean or a date.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_simple(&self) -> bool {
        !matches!(self, Self::LangAlt(_) | Self::Seq(_) | Self::Bag(_))
    }

    /// The value's type as a PDF/A extension schema declares it (ISO 19005-2 §6.6.2.3.3):
    /// `Text`, `URI`, `Integer`, `Real`, `Boolean`, `Date`, `Lang Alt`, or `Seq` or `Bag` and
    /// the items' type.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn value_type(&self) -> String {
        match self {
            Self::Text(_) => "Text".to_owned(),
            Self::Uri(_) => "URI".to_owned(),
            Self::Integer(_) => "Integer".to_owned(),
            Self::Real(_) => "Real".to_owned(),
            Self::Boolean(_) => "Boolean".to_owned(),
            Self::Date(_) => "Date".to_owned(),
            Self::LangAlt(_) => "Lang Alt".to_owned(),
            Self::Seq(items) | Self::Bag(items) => {
                let array = if matches!(self, Self::Seq(_)) { "Seq" } else { "Bag" };
                format!(
                    "{array} {}",
                    items.first().map_or_else(|| "Text".to_owned(), Self::value_type)
                )
            }
        }
    }

    /// Checks that the value can be written: arrays not empty, of simple values of one type;
    /// language alternatives not empty, each language given once.
    ///
    /// # Errors
    /// The [`ValueError`] that says what is wrong.
    ///
    /// Added in 0.2.0.
    pub fn check(&self) -> Result<(), ValueError> {
        match self {
            Self::LangAlt(entries) => {
                if entries.is_empty() {
                    return Err(ValueError::Empty);
                }
                for (i, e) in entries.iter().enumerate() {
                    let repeated = entries.iter().take(i).any(|o| o.lang.eq_ignore_ascii_case(&e.lang));
                    if e.lang.trim().is_empty() || repeated {
                        return Err(ValueError::Language);
                    }
                }
                Ok(())
            }
            Self::Seq(items) | Self::Bag(items) => {
                let first = items.first().ok_or(ValueError::Empty)?;
                if !items.iter().all(Self::is_simple) {
                    return Err(ValueError::Nested);
                }
                if items
                    .iter()
                    .any(|i| std::mem::discriminant(i) != std::mem::discriminant(first))
                {
                    return Err(ValueError::Mixed);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// A simple value's text as the packet writes it: `True`/`False`, dates in ISO 8601,
    /// numbers in decimal. `None` for an array or a language alternative.
    pub(super) fn simple_text(&self) -> Option<String> {
        Some(match self {
            Self::Text(s) | Self::Uri(s) => s.clone(),
            Self::Integer(n) => n.to_string(),
            Self::Real(r) => r.get().to_string(),
            Self::Boolean(true) => "True".to_owned(),
            Self::Boolean(false) => "False".to_owned(),
            Self::Date(d) => d.to_iso8601(),
            Self::LangAlt(_) | Self::Seq(_) | Self::Bag(_) => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reals_must_be_finite() {
        assert!(XmpReal::new(f64::NAN).is_none() && XmpReal::new(f64::INFINITY).is_none());
        assert_eq!(XmpReal::new(1.5).map(XmpReal::get), Some(1.5));
    }

    #[test]
    fn arrays_hold_simple_values_of_one_type() {
        let text = || XmpValue::Text("a".into());
        assert_eq!(XmpValue::Seq(vec![]).check(), Err(ValueError::Empty));
        assert_eq!(
            XmpValue::Bag(vec![XmpValue::Seq(vec![text()])]).check(),
            Err(ValueError::Nested)
        );
        assert_eq!(
            XmpValue::Seq(vec![text(), XmpValue::Integer(1)]).check(),
            Err(ValueError::Mixed)
        );
        assert_eq!(XmpValue::Bag(vec![text(), text()]).check(), Ok(()));
    }

    #[test]
    fn languages_are_given_once() {
        let alt = |langs: &[&str]| XmpValue::LangAlt(langs.iter().map(|l| LangText::new(*l, "t")).collect());
        assert_eq!(alt(&[]).check(), Err(ValueError::Empty));
        assert_eq!(alt(&["en", "EN"]).check(), Err(ValueError::Language));
        assert_eq!(alt(&[" "]).check(), Err(ValueError::Language));
        assert_eq!(alt(&["x-default", "en"]).check(), Ok(()));
    }

    #[test]
    fn simple_values_are_written_as_xmp_spells_them() {
        assert_eq!(XmpValue::Boolean(true).simple_text().as_deref(), Some("True"));
        assert_eq!(XmpValue::Integer(-3).simple_text().as_deref(), Some("-3"));
        assert_eq!(
            XmpValue::Real(XmpReal::new(0.25).unwrap()).simple_text().as_deref(),
            Some("0.25")
        );
        assert_eq!(XmpValue::Seq(vec![]).simple_text(), None);
    }
}
