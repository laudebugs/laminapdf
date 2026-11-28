//! A form's actions across the boundary (the extract and text modules, as the form view is):
//! `WasmDocument.formActions(id)` reads what a field does when it is pressed as typed data, and
//! `submitRequest(id, index)` builds the request one of its `SubmitForm` actions would make, which
//! nothing here sends.

use js_sys::{Reflect, Uint8Array};
use laminapdf::{
    ActionTrigger, FieldAction, FieldId, FieldKind, FormAction, HideTarget, NamedAction, SubmitForm, SubmitMethod,
    SubmitRequest, SubmitTarget,
};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, invalid_argument, lamina_error};
use crate::form_behaviour::field_id;

/// What a Hide action names.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum HideTargetJs {
    /// A form field, by ID.
    Field(String),
    /// An annotation that isn't a field's widget, by ID.
    Annotation(String),
}

/// One action of a field, typed.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum FormActionJs {
    /// `GoTo`: a page of this document; `pageIndex` is left out when the destination names none.
    GoTo {
        /// The page's index, 0-based.
        #[serde(skip_serializing_if = "Option::is_none")]
        page_index: Option<usize>,
    },
    /// `Named`: `nextPage`, `prevPage`, `firstPage` or `lastPage`, or `other` with its `name`.
    Named {
        /// Which.
        #[tsify(type = "\"nextPage\" | \"prevPage\" | \"firstPage\" | \"lastPage\" | \"other\"")]
        action: &'static str,
        /// The name a named action not listed has, as stored.
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// `URI`.
    Uri {
        /// The address as stored: untrusted text, not yet checked.
        url: String,
    },
    /// `ResetForm`.
    ResetForm {
        /// The IDs of the fields it resets, with Include and Exclude resolved: every field that
        /// holds a value when the action names none.
        fields: Vec<String>,
    },
    /// `Hide`.
    Hide {
        /// What it names.
        targets: Vec<HideTargetJs>,
        /// Whether the targets are hidden (otherwise shown again).
        hide: bool,
    },
    /// `SubmitForm`: ask `submitRequest` for what it would send.
    SubmitForm {
        /// The address as stored.
        url: String,
    },
    /// `ImportData`: never run.
    ImportData,
    /// `JavaScript`: never run.
    JavaScript,
    /// `Launch`: never run.
    Launch,
    /// `GoToR`: never run.
    GoToR,
    /// `GoToE`: never run.
    GoToE,
    /// Any other action, with its `/S` name; never run.
    Other {
        /// The `/S` name.
        name: String,
    },
}

/// An action and what fires it.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct FieldActionJs {
    /// `click`, `mouseDown`, `mouseEnter`, `mouseExit`, `focus` or `blur` (`other` for a trigger this
    /// version doesn't know).
    #[tsify(type = "\"click\" | \"mouseDown\" | \"mouseEnter\" | \"mouseExit\" | \"focus\" | \"blur\" | \"other\"")]
    pub trigger: &'static str,
    /// The action.
    pub action: FormActionJs,
}

/// A field a send carries, and the value as a person reads it.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct SubmittedFieldJs {
    /// The fully qualified name.
    pub name: String,
    /// The value that leaves the device.
    pub value: String,
}

/// What a `SubmitForm` action would send.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmitRequestJs {
    /// The address, with the query of a GET.
    pub url: String,
    /// The scheme and host the dialog names; the address for a mail.
    pub host: String,
    /// `web` or `mail` (a mail sends no data).
    #[tsify(type = "\"web\" | \"mail\"")]
    pub target: &'static str,
    /// `GET` or `POST`.
    #[tsify(type = "\"GET\" | \"POST\"")]
    pub method: &'static str,
    /// `html`, `fdf` or `xfdf`.
    #[tsify(type = "\"html\" | \"fdf\" | \"xfdf\"")]
    pub format: &'static str,
    /// The body's media type.
    pub content_type: &'static str,
    /// The fields in the body, in the form's order.
    pub fields: Vec<SubmittedFieldJs>,
    /// Names of required fields that are empty: a send is blocked until they are filled.
    pub empty_required: Vec<String>,
}

fn action(a: FieldAction, form_fields: &impl Fn(&laminapdf::FieldSelection) -> Vec<String>) -> FieldActionJs {
    let (trigger, action) = (a.trigger, a.action);
    FieldActionJs {
        trigger: match trigger {
            ActionTrigger::MouseDown => "mouseDown",
            ActionTrigger::MouseEnter => "mouseEnter",
            ActionTrigger::MouseExit => "mouseExit",
            ActionTrigger::Focus => "focus",
            ActionTrigger::Blur => "blur",
            ActionTrigger::Click => "click",
            _ => "other",
        },
        action: match action {
            FormAction::GoTo { page_index } => FormActionJs::GoTo { page_index },
            FormAction::Named(n) => {
                let (action, name) = match n {
                    NamedAction::NextPage => ("nextPage", None),
                    NamedAction::PrevPage => ("prevPage", None),
                    NamedAction::FirstPage => ("firstPage", None),
                    NamedAction::LastPage => ("lastPage", None),
                    NamedAction::Other(name) => ("other", Some(name)),
                    _ => ("other", None),
                };
                FormActionJs::Named { action, name }
            }
            FormAction::Uri(url) => FormActionJs::Uri { url },
            FormAction::ResetForm(selection) => FormActionJs::ResetForm {
                fields: form_fields(&selection),
            },
            FormAction::Hide { targets, hide } => FormActionJs::Hide {
                targets: targets
                    .into_iter()
                    .filter_map(|t| match t {
                        HideTarget::Field(id) => Some(HideTargetJs::Field(id.to_string())),
                        HideTarget::Annotation(id) => Some(HideTargetJs::Annotation(id.to_string())),
                        _ => None,
                    })
                    .collect(),
                hide,
            },
            FormAction::SubmitForm(s) => FormActionJs::SubmitForm { url: s.url },
            FormAction::ImportData => FormActionJs::ImportData,
            FormAction::JavaScript => FormActionJs::JavaScript,
            FormAction::Launch => FormActionJs::Launch,
            FormAction::GoToR => FormActionJs::GoToR,
            FormAction::GoToE => FormActionJs::GoToE,
            FormAction::Other(name) => FormActionJs::Other { name },
            other => FormActionJs::Other {
                name: other.name().to_owned(),
            },
        },
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// What field `id` does when it is pressed or the pointer moves over it, as typed data
    /// (the extract and text modules). Nothing runs.
    ///
    /// # Errors
    /// `invalid-argument` for an ID that isn't a field's; `unknown-field`, and the reading errors.
    #[wasm_bindgen(js_name = formActions, unchecked_return_type = "FieldActionJs[]")]
    pub fn form_actions(&self, id: &str) -> JsResult<JsValue> {
        let id = field_id(id)?;
        let doc = self.shared();
        let doc = doc.borrow();
        let actions = doc.form_actions(&id).map_err(|e| lamina_error(&e))?;
        let form = doc.form().map_err(|e| lamina_error(&e))?;
        let resolve = |selection: &laminapdf::FieldSelection| -> Vec<String> {
            form.fields
                .iter()
                .filter(|f| {
                    selection.covers(&f.name) && !matches!(f.kind, FieldKind::PushButton | FieldKind::Signature)
                })
                .map(|f| f.id.clone())
                .collect()
        };
        let out: Vec<FieldActionJs> = actions.into_iter().map(|a| action(a, &resolve)).collect();
        Ok(to_js(&out))
    }

    /// The request action number `index` of field `id` (as `formActions` lists them) would make:
    /// `{ url, host, target, method, format, contentType, body, fields, emptyRequired }`. Nothing
    /// is sent and nothing changes (the extract and text modules).
    ///
    /// # Errors
    /// `invalid-argument` for an ID that isn't a field's, or an index that isn't a `SubmitForm`
    /// action; `unknown-field`; `submit-pdf-refused` and `submit-url-refused`, and the reading
    /// errors.
    #[wasm_bindgen(js_name = submitRequest, unchecked_return_type = "SubmitRequestJs & { body: Uint8Array<ArrayBuffer> }")]
    pub fn submit_request(&self, id: &str, index: u32) -> JsResult<JsValue> {
        let id: FieldId = field_id(id)?;
        let doc = self.shared();
        let doc = doc.borrow();
        let submit: SubmitForm = doc
            .form_actions(&id)
            .map_err(|e| lamina_error(&e))?
            .into_iter()
            .nth(index as usize)
            .and_then(|a| match a.action {
                FormAction::SubmitForm(s) => Some(s),
                _ => None,
            })
            .ok_or_else(|| invalid_argument("index", "the index of a SubmitForm action".to_owned(), Vec::new()))?;
        let request = doc.submit_request(&submit).map_err(|e| lamina_error(&e))?;
        request_js(request)
    }
}

fn request_js(request: SubmitRequest) -> JsResult<JsValue> {
    let body = request.body.clone();
    let out = to_js(&SubmitRequestJs {
        url: request.url,
        host: request.host,
        target: match request.target {
            SubmitTarget::Mail => "mail",
            _ => "web",
        },
        method: match request.method {
            SubmitMethod::Get => "GET",
            _ => "POST",
        },
        format: request.format.as_str(),
        content_type: request.format.content_type(),
        fields: request
            .fields
            .into_iter()
            .map(|f| SubmittedFieldJs {
                name: f.name,
                value: f.value,
            })
            .collect(),
        empty_required: request.empty_required,
    });
    Reflect::set(&out, &"body".into(), &Uint8Array::from(body.as_slice()))?;
    Ok(out)
}
