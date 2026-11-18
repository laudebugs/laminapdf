//! The interactive form across the boundary: `WasmDocument.formFields()`, every field with its
//! value and widgets in page space, and `tabOrder(index)`, the order Tab visits a page's widgets
//! in (the extract and text modules, so the base module stays in its size budget). The op that
//! changes a value, `setFieldValue`, crosses through `apply` in every module.

use laminapdf::{FieldInfo, FieldValue, FormFields, PageTabOrder, Tabs, WidgetInfo};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// The form's fields, and what its field tree and the pages' widgets disagree on.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FormFieldsJs {
    /// The fields, in the order the form lists them.
    pub fields: Vec<FormFieldJs>,
    /// The IDs of widgets a page shows that no field reaches.
    pub orphan_widgets: Vec<String>,
    /// The IDs of fields none of whose widgets a page shows.
    pub widgetless: Vec<String>,
    /// Whether the form asks viewers to draw every widget again.
    pub need_appearances: bool,
    /// Whether the form has an XFA part.
    pub has_xfa: bool,
}

/// A form field.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FormFieldJs {
    /// Its ID (`f12.0`), as `setFieldValue` takes it.
    pub id: String,
    /// The IDs of other objects with the same name, which share its value.
    pub shared: Vec<String>,
    /// Its fully qualified name (document text).
    pub name: String,
    /// The name the interface shows (document text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    /// The kind of field.
    #[tsify(
        type = "\"text\" | \"checkBox\" | \"radioGroup\" | \"pushButton\" | \"comboBox\" | \"listBox\" | \"signature\""
    )]
    pub kind: &'static str,
    /// `/Ff`, the flags.
    pub flags: u32,
    /// Whether the interface may change it.
    pub read_only: bool,
    /// Whether it needs a value before the form is submitted.
    pub required: bool,
    /// Its value; left out when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(type = "FieldValue", optional)]
    pub value: Option<FieldValueJs>,
    /// The value resetting the form gives it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(type = "FieldValue", optional)]
    pub default_value: Option<FieldValueJs>,
    /// The longest text it takes, in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_len: Option<u32>,
    /// A list or combo box's options, or a button group's export values.
    pub options: Vec<ChoiceOptionJs>,
    /// The states its buttons turn on to.
    pub on_states: Vec<String>,
    /// Whether a signature field holds a signature.
    pub signed: bool,
    /// Its widgets.
    pub widgets: Vec<FormWidgetJs>,
}

/// One option of a list or combo box.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct ChoiceOptionJs {
    /// The value exported.
    pub export: String,
    /// The text shown.
    pub display: String,
}

/// A field's value, in the shape `setFieldValue` takes it.
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub(crate) enum FieldValueJs {
    Text(String),
    Button(Option<String>),
    Choice(Vec<String>),
}

/// One widget of a field.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FormWidgetJs {
    /// Its ID (`a12.0`).
    pub id: String,
    /// The 0-based index of the page that shows it; left out when no page does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Its rectangle in that page's space: `[x0, y0, x1, y1]`, top-left and bottom-right.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f64; 4]>,
    /// `/F`, its annotation flags.
    pub flags: u32,
    /// For a check box or radio button, the state it turns on to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_state: Option<String>,
    /// The state it shows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// The order Tab visits a page's widgets in.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TabOrderJs {
    /// The order the page asks for; left out when it asks for none this reader knows.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(type = "\"row\" | \"column\" | \"structure\" | \"annotations\" | \"widgets\"")]
    pub tabs: Option<&'static str>,
    /// The page's widgets in that order.
    pub widgets: Vec<TabStopJs>,
}

/// A widget in the tab order.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct TabStopJs {
    /// The widget's ID.
    pub widget: String,
    /// The ID of its field.
    pub field: String,
}

fn value(v: Option<FieldValue>) -> Option<FieldValueJs> {
    match v? {
        FieldValue::Text(t) => Some(FieldValueJs::Text(t)),
        FieldValue::Button(b) => Some(FieldValueJs::Button(b)),
        FieldValue::Choice(c) => Some(FieldValueJs::Choice(c)),
        _ => None,
    }
}

fn widget(w: WidgetInfo) -> FormWidgetJs {
    FormWidgetJs {
        id: w.id,
        page: w.page,
        rect: w.rect.map(|r| [r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get()]),
        flags: w.flags,
        on_state: w.on_state,
        state: w.state,
    }
}

fn field(f: FieldInfo) -> FormFieldJs {
    FormFieldJs {
        id: f.id,
        shared: f.shared,
        name: f.name,
        tooltip: f.tooltip,
        kind: f.kind.as_str(),
        flags: f.flags,
        read_only: f.read_only,
        required: f.required,
        value: value(f.value),
        default_value: value(f.default_value),
        max_len: f.max_len,
        options: f
            .options
            .into_iter()
            .map(|o| ChoiceOptionJs {
                export: o.export,
                display: o.display,
            })
            .collect(),
        on_states: f.on_states,
        signed: f.signed,
        widgets: f.widgets.into_iter().map(widget).collect(),
    }
}

impl From<FormFields> for FormFieldsJs {
    fn from(form: FormFields) -> Self {
        Self {
            fields: form.fields.into_iter().map(field).collect(),
            orphan_widgets: form.orphan_widgets,
            widgetless: form.widgetless,
            need_appearances: form.need_appearances,
            has_xfa: form.has_xfa,
        }
    }
}

impl From<PageTabOrder> for TabOrderJs {
    fn from(order: PageTabOrder) -> Self {
        Self {
            tabs: order.tabs.map(|t| match t {
                Tabs::Row => "row",
                Tabs::Column => "column",
                Tabs::Structure => "structure",
                Tabs::Annotations => "annotations",
                _ => "widgets",
            }),
            widgets: order
                .widgets
                .into_iter()
                .map(|s| TabStopJs {
                    widget: s.widget,
                    field: s.field,
                })
                .collect(),
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The form's fields with their values and widgets (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = formFields, unchecked_return_type = "FormFieldsJs")]
    pub fn form_fields(&self) -> JsResult<JsValue> {
        let form = self.shared().borrow().form().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&FormFieldsJs::from(form)))
    }

    /// The order Tab visits page `index`'s widgets in (the extract and text modules).
    ///
    /// # Errors
    /// `page-index-out-of-range`, `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = tabOrder, unchecked_return_type = "TabOrderJs")]
    pub fn tab_order(&self, index: u32) -> JsResult<JsValue> {
        let order = self.shared().borrow().tab_order(index).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&TabOrderJs::from(order)))
    }
}
