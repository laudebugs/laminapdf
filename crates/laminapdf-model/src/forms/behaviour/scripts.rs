//! Reading scripts: a field's behaviour from its `/AA`, and every script of a document with
//! whether it is handled ([ADR 0021] §3's inventory).
//!
//! [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object, Result, tolerant};
use laminapdf_syntax::text::decode_text_string;

use super::{Behaviour, FieldBehaviour, FieldTrigger, recognise};
use crate::disclosure::{Owner, Trigger};
use crate::document::Document;
use crate::forms::TerminalField;
use crate::outline::ActionKind;

/// Scripts listed at most, so a file with a huge `/Next` graph stays bounded.
const MAX_SCRIPTS: usize = 10_000;

/// Where a script is.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScriptLocation {
    /// A JavaScript action: what fires it and what it belongs to.
    Action {
        /// What fires it.
        trigger: Trigger,
        /// What it belongs to.
        owner: Owner,
    },
    /// A `<script>` element of an XFA packet (§12.7.9).
    Xfa {
        /// The packet's name (`template`, `datasets`, …), or `xdp` for an `/XFA` stream.
        packet: String,
    },
}

/// Whether Lamina PDF does a script's work ([ADR 0021] §3).
///
/// [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScriptState {
    /// A helper call, done natively.
    Handled,
    /// Anything else: never run.
    NotHandled,
    /// An XFA script: never handled.
    Xfa,
}

impl ScriptState {
    /// `handled`, `not handled` or `xfa`, as the bindings and reports spell it.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Handled => "handled",
            Self::NotHandled => "not handled",
            Self::Xfa => "xfa",
        }
    }
}

/// One script of a document.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FormScript {
    /// Where it is.
    pub location: ScriptLocation,
    /// The object that holds it: the action, or the XFA packet's stream; `None` when direct.
    pub object: Option<u32>,
    /// Its length in bytes.
    pub bytes: u64,
    /// Whether it is handled.
    pub state: ScriptState,
}

impl Document {
    /// What `field`'s keystroke, format, validate and calculate scripts do, as far as each is
    /// one of Acrobat's helper calls ([ADR 0021] §2). A script that isn't one, or is longer
    /// than [`Limits::max_script_length`](laminapdf_cos::Limits::max_script_length), is listed
    /// in [`Behaviour::not_handled`]. `/AA` is read from the field, then from each field above
    /// it, as pdf.js reads it.
    ///
    /// [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::DataPending`], [`laminapdf_cos::Error::Source`] and
    /// [`laminapdf_cos::Error::Strict`]: the errors that must propagate.
    ///
    /// Added in 0.6.0.
    pub fn field_behaviour(&self, field: &TerminalField) -> Result<Behaviour> {
        let mut behaviour = Behaviour::default();
        let Some(start) = field.id.as_lamina_id().obj_ref() else {
            return Ok(behaviour);
        };
        for trigger in FieldTrigger::ALL {
            let Some(action) = self.inherited_action(start, trigger.key())? else {
                continue;
            };
            match self.single_script(&action)?.and_then(|s| recognise(&s, trigger)) {
                Some(FieldBehaviour::SimpleCalculate(c)) if trigger == FieldTrigger::Calculate => {
                    behaviour.calculate = Some(c);
                }
                Some(b) if trigger == FieldTrigger::Keystroke => behaviour.keystroke = Some(b),
                Some(b) if trigger == FieldTrigger::Format => behaviour.format = Some(b),
                Some(b) if trigger == FieldTrigger::Validate => behaviour.validate = Some(b),
                _ if self.is_script(&action)? => behaviour.not_handled.push(trigger),
                _ => {}
            }
        }
        Ok(behaviour)
    }

    /// The action under `key` in the `/AA` of the field at `start` or the nearest field above
    /// it that has one.
    fn inherited_action(&self, start: ObjRef, key: &str) -> Result<Option<Object>> {
        let mut current = Object::Ref(start);
        for _ in 0..=self.limits().max_tree_depth() {
            let Some(dict) = self.read(&current)?.and_then(|o| o.as_dict().cloned()) else {
                return Ok(None);
            };
            if let Some(aa) = dict.get("AA")
                && let Some(action) = self
                    .read(aa)?
                    .and_then(|o| o.as_dict().and_then(|d| d.get(key)).cloned())
            {
                return Ok(Some(action));
            }
            let Some(parent) = dict.get("Parent") else {
                return Ok(None);
            };
            current = parent.clone();
        }
        Ok(None)
    }

    /// Whether `action` is a JavaScript action.
    fn is_script(&self, action: &Object) -> Result<bool> {
        Ok(self
            .read(action)?
            .is_some_and(|a| a.as_dict().and_then(|d| d.get_name("S")) == Some(b"JavaScript")))
    }

    /// The text of `action`'s script when it is one JavaScript action with nothing after it in
    /// `/Next`, and no longer than `max_script_length`.
    fn single_script(&self, action: &Object) -> Result<Option<String>> {
        let Some(action) = self.read(action)? else {
            return Ok(None);
        };
        let Some(dict) = action.as_dict() else {
            return Ok(None);
        };
        if dict.get_name("S") != Some(b"JavaScript") {
            return Ok(None);
        }
        let follows = match dict.get("Next") {
            Some(next) => !matches!(self.read(next)?.as_deref(), None | Some(Object::Null)),
            None => false,
        };
        if follows {
            return Ok(None);
        }
        self.script_text(dict)
    }

    /// `/JS` as text: a string, or a stream decoded; `None` past `max_script_length`.
    fn script_text(&self, action: &Dict) -> Result<Option<String>> {
        let max = self.limits().max_script_length();
        let Some(js) = action.get("JS") else {
            return Ok(None);
        };
        let Some(js) = self.read(js)? else {
            return Ok(None);
        };
        let bytes = match &*js {
            Object::String(s) => s.bytes.clone(),
            Object::Stream(s) if s.data.len() <= max => match tolerant(self.decode_stream(s))? {
                Some(d) => d,
                None => return Ok(None),
            },
            _ => return Ok(None),
        };
        Ok((bytes.len() <= max).then(|| decode_text_string(&bytes)))
    }

    /// Every script in the document, each with where it is, its size and whether it is
    /// handled ([ADR 0021] §3): the document's, the pages', the annotations' and the fields'
    /// JavaScript actions through `/Next`, and the `<script>` elements of an XFA form.
    ///
    /// A field's script is handled when it is the only action of its `K`, `F`, `V` or `C`
    /// trigger and a helper call for that trigger; every other script is not.
    ///
    /// [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::DataPending`], [`laminapdf_cos::Error::Source`] and
    /// [`laminapdf_cos::Error::Strict`]: the errors that must propagate.
    ///
    /// Added in 0.6.0.
    pub fn form_scripts(&self) -> Result<Vec<FormScript>> {
        let fields = self.field_nodes()?;
        let objects: BTreeSet<u32> = fields.iter().map(|(r, _)| r.num).collect();
        let mut out = Vec::new();
        for action in self.disclosure()?.actions {
            let field_owned = match &action.owner {
                Owner::FormField(_) => true,
                Owner::Annotation { object: Some(n), .. } => objects.contains(n),
                _ => false,
            };
            if action.kind == ActionKind::JavaScript && !field_owned {
                out.push(FormScript {
                    location: ScriptLocation::Action {
                        trigger: action.trigger,
                        owner: action.owner,
                    },
                    object: action.object,
                    bytes: action.script_bytes.unwrap_or(0),
                    state: ScriptState::NotHandled,
                });
            }
        }
        let mut seen = BTreeSet::new();
        for (node, name) in &fields {
            let Some(dict) = self.read(&Object::Ref(*node))?.and_then(|o| o.as_dict().cloned()) else {
                continue;
            };
            let Some(aa) = dict.get("AA") else { continue };
            let Some(aa) = self.read(aa)?.and_then(|o| o.as_dict().cloned()) else {
                continue;
            };
            for (key, first) in aa.iter() {
                let key = String::from_utf8_lossy(key.as_bytes()).into_owned();
                let trigger = FieldTrigger::ALL.into_iter().find(|t| t.key() == key);
                let handled = match trigger {
                    Some(t) => self.single_script(first)?.and_then(|s| recognise(&s, t)).is_some(),
                    None => false,
                };
                self.chain(first, &mut seen, &mut out, |object, bytes| FormScript {
                    location: ScriptLocation::Action {
                        trigger: Trigger::Additional(key.clone()),
                        owner: Owner::FormField(name.clone()),
                    },
                    object,
                    bytes,
                    state: if handled {
                        ScriptState::Handled
                    } else {
                        ScriptState::NotHandled
                    },
                })?;
            }
        }
        self.xfa_scripts(&mut out)?;
        out.truncate(MAX_SCRIPTS);
        Ok(out)
    }

    /// Lists each JavaScript action of the chain at `start` once, made by `entry`.
    fn chain(
        &self,
        start: &Object,
        seen: &mut BTreeSet<u32>,
        out: &mut Vec<FormScript>,
        entry: impl Fn(Option<u32>, u64) -> FormScript,
    ) -> Result<()> {
        let mut stack = vec![start.clone()];
        while let Some(obj) = stack.pop() {
            if out.len() >= MAX_SCRIPTS {
                break;
            }
            let object = obj.as_ref().map(|r| r.num);
            if object.is_some_and(|n| !seen.insert(n)) {
                continue;
            }
            let Some(action) = self.read(&obj)? else { continue };
            if let Some(items) = action.as_array() {
                stack.extend(items.iter().rev().cloned());
                continue;
            }
            let Some(dict) = action.as_dict() else { continue };
            if dict.get_name("S") == Some(b"JavaScript") {
                out.push(entry(object, self.script_len(dict)?));
            }
            if let Some(next) = dict.get("Next") {
                stack.push(next.clone());
            }
        }
        Ok(())
    }

    /// The length of `/JS` in bytes: a string's, or a stream's decoded (stored when it can't be).
    fn script_len(&self, action: &Dict) -> Result<u64> {
        let Some(js) = action.get("JS") else { return Ok(0) };
        let Some(js) = self.read(js)? else { return Ok(0) };
        let len = match &*js {
            Object::String(s) => s.bytes.len(),
            Object::Stream(s) => tolerant(self.decode_stream(s))?.map_or(s.data.len(), |d| d.len()),
            _ => 0,
        };
        Ok(u64::try_from(len).unwrap_or(u64::MAX))
    }

    /// Every node of the field tree, field or not, with its full name; each object once.
    fn field_nodes(&self) -> Result<Vec<(ObjRef, String)>> {
        let mut out = Vec::new();
        let Some(acro) = self.acro_form_dict()? else {
            return Ok(out);
        };
        let Some(fields) = acro.get("Fields") else {
            return Ok(out);
        };
        let Some(fields) = self.read(fields)? else {
            return Ok(out);
        };
        let max_depth = self.limits().max_tree_depth();
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(Object, String, usize)> = fields
            .as_array()
            .unwrap_or_default()
            .iter()
            .rev()
            .map(|f| (f.clone(), String::new(), 0))
            .collect();
        while let Some((obj, parent, depth)) = stack.pop() {
            let Some(r) = obj.as_ref() else { continue };
            if depth > max_depth || !seen.insert(r.num) {
                continue;
            }
            let Some(dict) = self.read(&obj)?.and_then(|o| o.as_dict().cloned()) else {
                continue;
            };
            let partial = match dict.get("T") {
                Some(t) => self.read(t)?.and_then(|o| text_of(&o)),
                None => None,
            };
            let name = match (partial, parent.is_empty()) {
                (Some(t), true) => t,
                (Some(t), false) => format!("{parent}.{t}"),
                (None, _) => parent.clone(),
            };
            if let Some(kids) = dict.get("Kids")
                && let Some(kids) = self.read(kids)?
            {
                for kid in kids.as_array().unwrap_or_default().iter().rev() {
                    stack.push((kid.clone(), name.clone(), depth + 1));
                }
            }
            out.push((r, name));
        }
        Ok(out)
    }

    /// `/AcroForm`, resolved.
    fn acro_form_dict(&self) -> Result<Option<Dict>> {
        match self.catalog()?.get("AcroForm") {
            Some(v) => Ok(self.read(v)?.and_then(|o| o.as_dict().cloned())),
            None => Ok(None),
        }
    }

    /// The `<script>` elements of the form's XFA packets.
    fn xfa_scripts(&self, out: &mut Vec<FormScript>) -> Result<()> {
        let Some(acro) = self.acro_form_dict()? else {
            return Ok(());
        };
        let Some(xfa) = acro.get("XFA") else { return Ok(()) };
        let packets: Vec<(String, Object)> = match self.read(xfa)?.as_deref() {
            Some(Object::Array(items)) => items
                .chunks(2)
                .filter_map(|pair| match pair {
                    [name, stream] => Some((text_of(name).unwrap_or_default(), stream.clone())),
                    _ => None,
                })
                .collect(),
            Some(_) => vec![("xdp".to_owned(), xfa.clone())],
            None => Vec::new(),
        };
        for (packet, stream) in packets {
            let object = stream.as_ref().map(|r| r.num);
            let Some(Object::Stream(s)) = self.read(&stream)?.as_deref().cloned() else {
                continue;
            };
            let Some(data) = tolerant(self.decode_stream(&s))? else {
                continue;
            };
            for bytes in script_elements(&data) {
                if out.len() >= MAX_SCRIPTS {
                    return Ok(());
                }
                out.push(FormScript {
                    location: ScriptLocation::Xfa { packet: packet.clone() },
                    object,
                    bytes,
                    state: ScriptState::Xfa,
                });
            }
        }
        Ok(())
    }
}

/// The length of each `<script …>…</script>` element's content in an XFA packet.
fn script_elements(xml: &[u8]) -> Vec<u64> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = find(rest, b"<script") {
        let after = rest.get(start + 7..).unwrap_or_default();
        // `<scripts>` or `<scriptX` is another element.
        if !after
            .first()
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b'>' || *b == b'/')
        {
            rest = after;
            continue;
        }
        let Some(open_end) = find(after, b">") else { break };
        let self_closing = open_end > 0 && after.get(open_end - 1) == Some(&b'/');
        let body = after.get(open_end + 1..).unwrap_or_default();
        if self_closing {
            out.push(0);
            rest = body;
            continue;
        }
        let close = find(body, b"</script>").unwrap_or(body.len());
        out.push(u64::try_from(close).unwrap_or(u64::MAX));
        rest = body.get(close..).unwrap_or_default();
    }
    out
}

/// A string object's text.
fn text_of(obj: &Object) -> Option<String> {
    match obj {
        Object::String(s) => Some(decode_text_string(&s.bytes)),
        _ => None,
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfa_script_elements_are_found_and_measured() {
        let xml = br#"<template><scripts/><script contentType="application/x-javascript">app.alert(1)</script><script/></template>"#;
        assert_eq!(script_elements(xml), [12, 0]);
    }
}
