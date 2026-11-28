//! A field's actions (ISO 32000-2 §12.6) as typed data: what a push button does when it is
//! pressed, and what the other triggers of a field's `/AA` do, followed through `/Next`.
//!
//! The reader runs a few of them and names the rest ([ADR 0021] §1: no script runs). This
//! module only reads; it follows no address and runs nothing.
//!
//! [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object, Result};

use super::{FieldFlags, Form, TerminalField};
use crate::document::Document;
use crate::id::{AnnotId, FieldId};
use crate::outline::Outline;
use crate::values::text_of;

/// Actions read from one field at most, so a file with a huge `/Next` graph stays bounded.
const MAX_ACTIONS: usize = 64;
/// Field names or references one action may list at most.
const MAX_TARGETS: usize = 10_000;
/// The longest address read, in bytes: longer is refused as the address of no real server.
///
/// Added in 0.7.0.
pub const MAX_ADDRESS_LEN: usize = 8 * 1024;

/// What fires an action of a field.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ActionTrigger {
    /// The person presses and releases the button: the field's `/A`, and `/AA`'s `U`.
    Click,
    /// The pointer goes down on the field (`D`).
    MouseDown,
    /// The pointer enters the field (`E`).
    MouseEnter,
    /// The pointer leaves the field (`X`).
    MouseExit,
    /// The field gets the focus (`Fo`).
    Focus,
    /// The field loses the focus (`Bl`).
    Blur,
}

impl ActionTrigger {
    /// `click`, `mouseDown`, `mouseEnter`, `mouseExit`, `focus` or `blur`, as the bindings spell
    /// it.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::MouseDown => "mouseDown",
            Self::MouseEnter => "mouseEnter",
            Self::MouseExit => "mouseExit",
            Self::Focus => "focus",
            Self::Blur => "blur",
        }
    }

    /// The `/AA` keys read as triggers, with the trigger each is. The keys of the format,
    /// keystroke, validate and calculate scripts belong to a field's behaviour instead.
    const AA: [(&'static str, Self); 6] = [
        ("U", Self::Click),
        ("D", Self::MouseDown),
        ("E", Self::MouseEnter),
        ("X", Self::MouseExit),
        ("Fo", Self::Focus),
        ("Bl", Self::Blur),
    ];
}

/// A named action (§12.6.4.12): a page move the viewer knows.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NamedAction {
    /// `NextPage`.
    NextPage,
    /// `PrevPage`.
    PrevPage,
    /// `FirstPage`.
    FirstPage,
    /// `LastPage`.
    LastPage,
    /// Any other name, as stored (`Print`, `SaveAs`, …): the reader doesn't run it.
    Other(String),
}

/// What a Hide action (§12.6.4.10) names.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HideTarget {
    /// A terminal field of the form.
    Field(FieldId),
    /// An annotation that is not one of a field's widgets.
    Annotation(AnnotId),
}

/// The fields a reset-form or submit-form action covers (§12.7.6.2, Table 238: `/Fields` and
/// the Include/Exclude bit of `/Flags`). Names are fully qualified; a name covers the field of
/// that name and every field below it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FieldSelection {
    /// The names `/Fields` lists, references resolved to fully qualified names.
    pub names: Vec<String>,
    /// Whether the names are left out (Exclude) instead of the only ones taken.
    pub exclude: bool,
}

impl FieldSelection {
    /// A selection of `names` (fully qualified), left out when `exclude`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(names: Vec<String>, exclude: bool) -> Self {
        Self { names, exclude }
    }

    /// Whether the field named `name` is covered: every field when no name is listed and the
    /// selection includes; otherwise a field a listed name covers, or, for Exclude, one none does.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn covers(&self, name: &str) -> bool {
        if self.names.is_empty() {
            return true;
        }
        let listed = self
            .names
            .iter()
            .any(|n| name == n || name.strip_prefix(n.as_str()).is_some_and(|rest| rest.starts_with('.')));
        listed != self.exclude
    }

    /// The fields of `form` that hold a value and that the selection covers, in the form's
    /// order. Push buttons and signature fields hold no value and are never among them.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn fields<'f>(&self, form: &'f Form) -> Vec<&'f TerminalField> {
        form.fields
            .iter()
            .filter(|f| f.takes_value() && self.covers(&f.name))
            .collect()
    }
}

/// The flags of a submit-form action (§12.7.6.2, Table 238); bit 1 is `1 << 0`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SubmitFlags(pub u32);

impl SubmitFlags {
    /// `/Fields` lists the fields left out.
    ///
    /// Added in 0.7.0.
    pub const EXCLUDE: u32 = 1;
    /// Fields without a value are sent too.
    ///
    /// Added in 0.7.0.
    pub const INCLUDE_NO_VALUE_FIELDS: u32 = 1 << 1;
    /// HTML form encoding (`name=value&…`).
    ///
    /// Added in 0.7.0.
    pub const EXPORT_FORMAT: u32 = 1 << 2;
    /// The request is a GET.
    ///
    /// Added in 0.7.0.
    pub const GET_METHOD: u32 = 1 << 3;
    /// XFDF, not FDF.
    ///
    /// Added in 0.7.0.
    pub const XFDF: u32 = 1 << 5;
    /// The whole PDF is sent.
    ///
    /// Added in 0.7.0.
    pub const SUBMIT_PDF: u32 = 1 << 8;

    /// Whether `flag` (one of the constants) is set.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn has(self, flag: u32) -> bool {
        self.0 & flag != 0
    }
}

/// A `SubmitForm` action as stored: where, how and which fields. It sends nothing; the interchange
/// crate's `submit_request` turns it into the request a person confirms.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SubmitForm {
    /// The address (`/F`: a URL string, or a file specification of type URL), as stored.
    pub url: String,
    /// `/Flags`.
    pub flags: SubmitFlags,
    /// `/Fields` and the Exclude bit.
    pub selection: FieldSelection,
}

impl SubmitForm {
    /// A submit action sending to `url` with `flags` the fields `selection` covers.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(url: String, flags: SubmitFlags, selection: FieldSelection) -> Self {
        Self { url, flags, selection }
    }
}

/// An action of a field, typed (§12.6.4).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FormAction {
    /// `GoTo`: a place in this document; `None` when the destination names no page of it.
    GoTo {
        /// The page's index.
        page_index: Option<usize>,
    },
    /// `Named`.
    Named(NamedAction),
    /// `URI`: the address, as stored.
    Uri(String),
    /// `ResetForm`.
    ResetForm(FieldSelection),
    /// `Hide`: hides the targets (`hide`), or shows them again.
    Hide {
        /// What it names.
        targets: Vec<HideTarget>,
        /// `/H`: whether the targets are hidden (the default) or shown.
        hide: bool,
    },
    /// `SubmitForm`.
    SubmitForm(SubmitForm),
    /// `ImportData`: it names a file on the sender's computer; never run.
    ImportData,
    /// `JavaScript`: never run (ADR 0021).
    JavaScript,
    /// `Launch`: never run.
    Launch,
    /// `GoToR`: a place in another file; never run.
    GoToR,
    /// `GoToE`: a place in an embedded file; never run.
    GoToE,
    /// Any other `/S`, as stored; never run.
    Other(String),
}

impl FormAction {
    /// The action's `/S` name (`GoTo`, `URI`, `SubmitForm`, …).
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::GoTo { .. } => "GoTo",
            Self::Named(_) => "Named",
            Self::Uri(_) => "URI",
            Self::ResetForm(_) => "ResetForm",
            Self::Hide { .. } => "Hide",
            Self::SubmitForm(_) => "SubmitForm",
            Self::ImportData => "ImportData",
            Self::JavaScript => "JavaScript",
            Self::Launch => "Launch",
            Self::GoToR => "GoToR",
            Self::GoToE => "GoToE",
            Self::Other(name) => name,
        }
    }
}

/// One action of a field and what fires it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FieldAction {
    /// What fires it.
    pub trigger: ActionTrigger,
    /// The action. An action reached through `/Next` follows the one before it, with the same
    /// trigger.
    pub action: FormAction,
}

impl Document {
    /// What `field`'s widget and field dictionaries run when a person presses it or moves the
    /// pointer over it: `/A` and the `/AA` keys `U`, `D`, `E`, `X`, `Fo` and `Bl`, each
    /// followed through `/Next` once, in order. At most 64 actions are read, and no more than 10,000
    /// field names in one. Nothing runs, and an action that can't be read is left out.
    ///
    /// A field's keystroke, format, validate and calculate scripts are its
    /// [behaviour](Self::field_behaviour), not listed here.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::DataPending`], [`laminapdf_cos::Error::Source`] and
    /// [`laminapdf_cos::Error::Strict`]: the errors that must propagate.
    ///
    /// Added in 0.7.0.
    pub fn form_actions(&self, field: &TerminalField) -> Result<Vec<FieldAction>> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut owners: Vec<ObjRef> = field
            .widgets
            .iter()
            .filter_map(|w| w.id.as_lamina_id().obj_ref())
            .collect();
        owners.extend(field.id.as_lamina_id().obj_ref().filter(|r| !owners.contains(r)));
        let form = self.form()?;
        let outline = Outline::destinations_only(self);
        for owner in owners {
            let Some(dict) = self.read(&Object::Ref(owner))?.and_then(|o| o.as_dict().cloned()) else {
                continue;
            };
            if let Some(a) = dict.get("A") {
                self.action_chain(a, ActionTrigger::Click, &form, &outline, &mut seen, &mut out)?;
            }
            let Some(aa) = dict.get("AA") else { continue };
            let Some(aa) = self.read(aa)?.and_then(|o| o.as_dict().cloned()) else {
                continue;
            };
            for (key, trigger) in ActionTrigger::AA {
                if let Some(a) = aa.get(key) {
                    self.action_chain(a, trigger, &form, &outline, &mut seen, &mut out)?;
                }
            }
        }
        Ok(out)
    }

    /// The action at `start` and those its `/Next` reaches, each read once.
    fn action_chain(
        &self,
        start: &Object,
        trigger: ActionTrigger,
        form: &Form,
        outline: &Outline<'_>,
        seen: &mut BTreeSet<u32>,
        out: &mut Vec<FieldAction>,
    ) -> Result<()> {
        let mut stack = vec![start.clone()];
        while let Some(obj) = stack.pop() {
            if out.len() >= MAX_ACTIONS {
                return Ok(());
            }
            if let Some(r) = obj.as_ref()
                && !seen.insert(r.num)
            {
                continue;
            }
            let Some(action) = self.read(&obj)? else { continue };
            if let Some(items) = action.as_array() {
                stack.extend(items.iter().rev().cloned());
                continue;
            }
            let Some(dict) = action.as_dict() else { continue };
            let Some(kind) = dict.get_name("S") else { continue };
            out.push(FieldAction {
                trigger,
                action: self.typed_action(kind, dict, form, outline)?,
            });
            if let Some(next) = dict.get("Next") {
                stack.push(next.clone());
            }
        }
        Ok(())
    }

    fn typed_action(&self, kind: &[u8], dict: &Dict, form: &Form, outline: &Outline<'_>) -> Result<FormAction> {
        Ok(match kind {
            b"GoTo" => FormAction::GoTo {
                page_index: match dict.get("D") {
                    Some(d) => outline.destination(d)?.and_then(|d| d.page_index),
                    None => None,
                },
            },
            b"Named" => FormAction::Named(match dict.get_name("N") {
                Some(b"NextPage") => NamedAction::NextPage,
                Some(b"PrevPage") => NamedAction::PrevPage,
                Some(b"FirstPage") => NamedAction::FirstPage,
                Some(b"LastPage") => NamedAction::LastPage,
                other => NamedAction::Other(String::from_utf8_lossy(other.unwrap_or_default()).into_owned()),
            }),
            b"URI" => FormAction::Uri(match dict.get("URI") {
                Some(u) => self.address(u)?.unwrap_or_default(),
                None => String::new(),
            }),
            b"ResetForm" => FormAction::ResetForm(self.selection(dict, 1 << 0, form)?),
            b"Hide" => FormAction::Hide {
                targets: self.hide_targets(dict, form)?,
                hide: match dict.get("H") {
                    Some(h) => !matches!(self.read(h)?.as_deref(), Some(Object::Bool(false))),
                    None => true,
                },
            },
            b"SubmitForm" => {
                let flags = SubmitFlags(u32::try_from(dict.get_int("Flags").unwrap_or(0)).unwrap_or(0));
                FormAction::SubmitForm(SubmitForm {
                    url: match dict.get("F") {
                        Some(f) => self.file_address(f)?.unwrap_or_default(),
                        None => String::new(),
                    },
                    selection: self.selection(dict, SubmitFlags::EXCLUDE, form)?,
                    flags,
                })
            }
            b"ImportData" => FormAction::ImportData,
            b"JavaScript" => FormAction::JavaScript,
            b"Launch" => FormAction::Launch,
            b"GoToR" => FormAction::GoToR,
            b"GoToE" => FormAction::GoToE,
            other => FormAction::Other(String::from_utf8_lossy(other).into_owned()),
        })
    }

    /// An address string: bytes, up to [`MAX_ADDRESS_LEN`]; `None` for anything longer or not a
    /// string.
    fn address(&self, value: &Object) -> Result<Option<String>> {
        Ok(match self.read(value)?.as_deref() {
            Some(Object::String(s)) if s.bytes.len() <= MAX_ADDRESS_LEN => {
                Some(String::from_utf8_lossy(&s.bytes).trim().to_owned())
            }
            _ => None,
        })
    }

    /// A submit action's `/F`: a string, or a file specification dictionary of type URL (its
    /// `/F`) or any whose `/F` is a string.
    fn file_address(&self, value: &Object) -> Result<Option<String>> {
        let Some(resolved) = self.read(value)? else {
            return Ok(None);
        };
        match &*resolved {
            Object::String(_) => self.address(&resolved),
            Object::Dict(d) => match d.get("F") {
                Some(f) => self.address(f),
                None => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// `/Fields` and the Exclude bit (`exclude_bit`) of `/Flags`.
    fn selection(&self, dict: &Dict, exclude_bit: u32, form: &Form) -> Result<FieldSelection> {
        let flags = u32::try_from(dict.get_int("Flags").unwrap_or(0)).unwrap_or(0);
        let mut names = Vec::new();
        if let Some(fields) = dict.get("Fields")
            && let Some(fields) = self.read(fields)?
        {
            let items: Vec<Object> = match fields.as_array() {
                Some(items) => items.to_vec(),
                None => vec![(*fields).clone()],
            };
            for item in items.iter().take(MAX_TARGETS) {
                if let Some(name) = self.entry_name(item, form)? {
                    names.push(name);
                }
            }
        }
        Ok(FieldSelection {
            names,
            exclude: flags & exclude_bit != 0,
        })
    }

    /// The fully qualified name a `/Fields` entry stands for: a text string as it is, a reference
    /// to a field node by its `/T` chain.
    fn entry_name(&self, item: &Object, form: &Form) -> Result<Option<String>> {
        if let Some(r) = item.as_ref() {
            if let Some(f) = form.fields.iter().find(|f| {
                f.id.as_lamina_id().obj_ref() == Some(r)
                    || f.shared.iter().any(|s| s.as_lamina_id().obj_ref() == Some(r))
            }) {
                return Ok(Some(f.name.clone()));
            }
            return self.field_name(r);
        }
        Ok(self.read(item)?.and_then(|o| text_of(&o)))
    }

    /// A Hide action's `/T`: a field or annotation reference, a field name, or an array of them.
    fn hide_targets(&self, dict: &Dict, form: &Form) -> Result<Vec<HideTarget>> {
        let Some(t) = dict.get("T") else { return Ok(Vec::new()) };
        let Some(resolved) = self.read(t)? else {
            return Ok(Vec::new());
        };
        let items: Vec<Object> = match resolved.as_array() {
            Some(items) => items.to_vec(),
            None => vec![t.clone()],
        };
        let mut out = Vec::new();
        for item in items.iter().take(MAX_TARGETS) {
            if let Some(r) = item.as_ref() {
                let by_widget = form
                    .fields
                    .iter()
                    .find(|f| f.widgets.iter().any(|w| w.id.as_lamina_id().obj_ref() == Some(r)));
                let target = match by_widget {
                    Some(f) => Some(HideTarget::Field(f.id.clone())),
                    None => match self.entry_name(item, form)? {
                        Some(name) => form.field_named(&name).map(|f| HideTarget::Field(f.id.clone())),
                        None => None,
                    },
                };
                out.push(target.unwrap_or_else(|| HideTarget::Annotation(AnnotId::object(r))));
            } else if let Some(name) = self.read(item)?.and_then(|o| text_of(&o))
                && let Some(f) = form.field_named(&name)
            {
                out.push(HideTarget::Field(f.id.clone()));
            }
        }
        Ok(out)
    }
}

impl TerminalField {
    /// Whether a submit leaves the field out: `NoExport` (§12.7.4.1).
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn no_export(&self) -> bool {
        self.flags.has(FieldFlags::NO_EXPORT)
    }
}
