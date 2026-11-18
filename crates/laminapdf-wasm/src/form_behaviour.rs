//! Form behaviour across the boundary (ADR 0021): `fieldBehaviour(id)`, what a
//! field's helper calls do; `validateField(id, text)`, a value checked and shown as the field
//! would; `calculate()`, the form's calculations as one edit; and `formScripts()`, every script
//! with whether it is handled (the extract and text modules). Lamina PDF runs no script.

use laminapdf::reveal::{Owner, Trigger};
use laminapdf::{
    Behaviour, CalculateFunction, FieldBehaviour, FieldCheck, FieldId, FormScript, NegativeStyle, ScriptLocation,
    SeparatorStyle, SpecialFormat,
};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::dto::EditedJs;
use crate::error::{JsResult, invalid_argument, lamina_error};

/// One helper call, as data.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum FieldBehaviourJs {
    /// `AFNumber_Format` or `AFNumber_Keystroke`.
    Number {
        /// Digits after the point.
        decimals: u8,
        /// `commaDot` (1,234.56), `dot` (1234.56), `dotComma` (1.234,56), `comma` (1234,56) or
        /// `apostropheDot` (1'234.56).
        separator: &'static str,
        /// `minus`, `red`, `parentheses` or `redParentheses`.
        negative: &'static str,
        /// The currency symbol, empty for none.
        currency: String,
        /// Whether the currency comes first.
        prepend: bool,
    },
    /// `AFPercent_Format` or `AFPercent_Keystroke`.
    Percent {
        /// Digits after the point.
        decimals: u8,
        /// As a number's.
        separator: &'static str,
        /// Whether the `%` comes first.
        prepend: bool,
    },
    /// A date (`AFDate_FormatEx`, `AFDate_Format`) in a pattern such as `mm/dd/yyyy`.
    Date {
        /// The pattern.
        pattern: String,
    },
    /// A time (`AFTime_Format`) in a pattern such as `h:MM tt`.
    Time {
        /// The pattern.
        pattern: String,
    },
    /// `AFSpecial_Format`: `zip`, `zipPlus4`, `phone` or `ssn`.
    Special {
        /// The kind.
        kind: &'static str,
    },
    /// `AFRange_Validate`.
    Range {
        /// The least value accepted.
        #[tsify(optional)]
        min: Option<f64>,
        /// The greatest value accepted.
        #[tsify(optional)]
        max: Option<f64>,
    },
    /// A helper a later version of Lamina PDF handles, which this module doesn't name.
    Other,
    /// `AFSimple_Calculate`.
    Calculate {
        /// `sum`, `average`, `product`, `minimum` or `maximum`.
        function: &'static str,
        /// The fields it reads, by full name.
        fields: Vec<String>,
    },
}

const fn separator(s: SeparatorStyle) -> &'static str {
    match s {
        SeparatorStyle::CommaDot => "commaDot",
        SeparatorStyle::Dot => "dot",
        SeparatorStyle::DotComma => "dotComma",
        SeparatorStyle::Comma => "comma",
        _ => "apostropheDot",
    }
}

const fn negative(n: NegativeStyle) -> &'static str {
    match n {
        NegativeStyle::Minus => "minus",
        NegativeStyle::Red => "red",
        NegativeStyle::Parentheses => "parentheses",
        _ => "redParentheses",
    }
}

const fn special(s: SpecialFormat) -> &'static str {
    match s {
        SpecialFormat::Zip => "zip",
        SpecialFormat::ZipPlus4 => "zipPlus4",
        SpecialFormat::Phone => "phone",
        _ => "ssn",
    }
}

const fn function(f: CalculateFunction) -> &'static str {
    match f {
        CalculateFunction::Sum => "sum",
        CalculateFunction::Average => "average",
        CalculateFunction::Product => "product",
        CalculateFunction::Minimum => "minimum",
        _ => "maximum",
    }
}

impl From<&FieldBehaviour> for FieldBehaviourJs {
    fn from(b: &FieldBehaviour) -> Self {
        match b {
            FieldBehaviour::NumberFormat(n) => Self::Number {
                decimals: n.decimals,
                separator: separator(n.separator),
                negative: negative(n.negative),
                currency: n.currency.clone(),
                prepend: n.prepend,
            },
            FieldBehaviour::PercentFormat(p) => Self::Percent {
                decimals: p.decimals,
                separator: separator(p.separator),
                prepend: p.prepend,
            },
            FieldBehaviour::DateFormat(d) => Self::Date {
                pattern: d.pattern.clone(),
            },
            FieldBehaviour::TimeFormat(d) => Self::Time {
                pattern: d.pattern.clone(),
            },
            FieldBehaviour::SpecialFormat(s) => Self::Special { kind: special(*s) },
            FieldBehaviour::RangeValidate(r) => Self::Range { min: r.min, max: r.max },
            FieldBehaviour::SimpleCalculate(c) => Self::Calculate {
                function: function(c.function),
                fields: c.fields.clone(),
            },
            _ => Self::Other,
        }
    }
}

/// What a field's scripts do, as far as they are helper calls.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BehaviourJs {
    /// The keystroke check (`K`), when it is a helper call.
    #[tsify(optional)]
    pub keystroke: Option<FieldBehaviourJs>,
    /// The format (`F`), when it is a helper call.
    #[tsify(optional)]
    pub format: Option<FieldBehaviourJs>,
    /// The validation (`V`), when it is a helper call.
    #[tsify(optional)]
    pub validate: Option<FieldBehaviourJs>,
    /// The calculation (`C`), when it is a helper call.
    #[tsify(optional)]
    pub calculate: Option<FieldBehaviourJs>,
    /// The `/AA` keys (`K`, `F`, `V`, `C`) whose scripts aren't helper calls: the field is
    /// filled without them.
    pub not_handled: Vec<&'static str>,
}

impl From<Behaviour> for BehaviourJs {
    fn from(b: Behaviour) -> Self {
        Self {
            keystroke: b.keystroke.as_ref().map(Into::into),
            format: b.format.as_ref().map(Into::into),
            validate: b.validate.as_ref().map(Into::into),
            calculate: b.calculate.map(|c| (&FieldBehaviour::SimpleCalculate(c)).into()),
            not_handled: b.not_handled.iter().map(|t| t.key()).collect(),
        }
    }
}

/// A value checked as its field would commit it.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FieldCheckJs {
    /// What the field would store; left out when it is refused.
    #[tsify(optional)]
    pub value: Option<String>,
    /// Why it is refused: `format` (its number, date, time or special format refuses it) or
    /// `range` (outside its bounds); left out when it is accepted.
    #[tsify(optional)]
    pub refused: Option<&'static str>,
    /// What the field would show.
    pub shown: String,
    /// Whether a negative number's style draws it red.
    pub red: bool,
}

impl From<FieldCheck> for FieldCheckJs {
    fn from(c: FieldCheck) -> Self {
        let (value, refused) = match c.committed {
            Ok(v) => (Some(v), None),
            Err(why) => (None, Some(why.reason())),
        };
        Self {
            value,
            refused,
            red: c.shown.red,
            shown: c.shown.text,
        }
    }
}

/// One script of the document.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FormScriptJs {
    /// What it belongs to: `document`, `page`, `annotation`, `field` or `xfa`.
    pub owner: &'static str,
    /// The page, for a page's or an annotation's script.
    #[tsify(optional)]
    pub page: Option<u32>,
    /// The field's full name, or the XFA packet's name.
    #[tsify(optional)]
    pub name: Option<String>,
    /// What fires it: an `/AA` key (`K`, `F`, `O`, …), `open`, `activate` or `named`; left out
    /// for an XFA script.
    #[tsify(optional)]
    pub trigger: Option<String>,
    /// The object that holds it; left out when it is direct.
    #[tsify(optional)]
    pub object: Option<u32>,
    /// Its length in bytes.
    pub bytes: f64,
    /// `handled`, `not handled` or `xfa`.
    pub state: &'static str,
}

impl From<FormScript> for FormScriptJs {
    #[expect(clippy::cast_precision_loss, reason = "a script's size, far below 2^53")]
    fn from(s: FormScript) -> Self {
        let (owner, page, name, trigger) = match s.location {
            ScriptLocation::Action { trigger, owner } => {
                let trigger = Some(match trigger {
                    Trigger::OpenAction => "open".to_owned(),
                    Trigger::Additional(key) => key,
                    Trigger::Activate => "activate".to_owned(),
                    Trigger::NamedScript(name) => format!("named:{name}"),
                    _ => "other".to_owned(),
                });
                match owner {
                    Owner::Page(i) => ("page", u32::try_from(i).ok(), None, trigger),
                    Owner::Annotation { page, .. } => ("annotation", u32::try_from(page).ok(), None, trigger),
                    Owner::FormField(name) => ("field", None, Some(name), trigger),
                    // The document's, and any owner a later version adds.
                    _ => ("document", None, None, trigger),
                }
            }
            ScriptLocation::Xfa { packet } => ("xfa", None, Some(packet), None),
            _ => ("document", None, None, None),
        };
        Self {
            owner,
            page,
            name,
            trigger,
            object: s.object,
            bytes: s.bytes as f64,
            state: s.state.as_str(),
        }
    }
}

fn field_id(id: &str) -> JsResult<FieldId> {
    id.parse()
        .map_err(|_| invalid_argument("id", "a field ID such as f12.0".to_owned(), Vec::new()))
}

#[wasm_bindgen]
impl WasmDocument {
    /// What field `id`'s scripts do, as far as they are helper calls (the extract and text
    /// modules).
    ///
    /// # Errors
    /// `invalid-argument` for an ID that isn't a field's; `unknown-field`, and the reading errors.
    #[wasm_bindgen(js_name = fieldBehaviour, unchecked_return_type = "BehaviourJs")]
    pub fn field_behaviour(&self, id: &str) -> JsResult<JsValue> {
        let id = field_id(id)?;
        let b = self
            .shared()
            .borrow()
            .field_behaviour(&id)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&BehaviourJs::from(b)))
    }

    /// `text` checked as field `id` would commit it, and what it would show; nothing changes.
    ///
    /// # Errors
    /// As [`WasmDocument::field_behaviour`].
    #[wasm_bindgen(js_name = validateField, unchecked_return_type = "FieldCheckJs")]
    pub fn validate_field(&self, id: &str, text: &str) -> JsResult<JsValue> {
        let id = field_id(id)?;
        let check = self
            .shared()
            .borrow()
            .validate(&id, text)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&FieldCheckJs::from(check)))
    }

    /// Whether field `id`'s keystroke helper takes `text` while it is typed.
    ///
    /// # Errors
    /// As [`WasmDocument::field_behaviour`].
    #[wasm_bindgen(js_name = fieldAccepts)]
    pub fn field_accepts(&self, id: &str, text: &str) -> JsResult<bool> {
        let id = field_id(id)?;
        self.shared()
            .borrow()
            .field_accepts(&id, text)
            .map_err(|e| lamina_error(&e))
    }

    /// Runs the form's calculations once, in order, as one edit.
    ///
    /// # Errors
    /// `saved-not-rebased`, and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "EditedJs")]
    pub fn calculate(&self) -> JsResult<JsValue> {
        let edited = self.shared().borrow_mut().calculate().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }

    /// Every script of the document, with where it is, its size and whether it is handled;
    /// never its text.
    ///
    /// # Errors
    /// The reading errors.
    #[wasm_bindgen(js_name = formScripts, unchecked_return_type = "FormScriptJs[]")]
    pub fn form_scripts(&self) -> JsResult<JsValue> {
        let scripts = self.shared().borrow().form_scripts().map_err(|e| lamina_error(&e))?;
        let scripts: Vec<FormScriptJs> = scripts.into_iter().map(Into::into).collect();
        Ok(to_js(&scripts))
    }
}
