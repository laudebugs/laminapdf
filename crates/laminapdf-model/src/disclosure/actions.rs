//! Actions and scripts (§12.6): what each trigger fires, followed through `/Next`, once each.

use laminapdf_cos::{Dict, Object, Result, tolerant};

use super::{Action, Owner, Scan, Trigger, len, name_of};
use crate::outline::ActionKind;

/// Actions listed at most, so a file with a huge `/Next` graph stays bounded.
const MAX_ACTIONS: usize = 10_000;

impl Scan<'_> {
    /// The catalog's `/OpenAction` and `/AA`, and the `/JavaScript` name tree.
    pub(super) fn document_actions(&mut self, catalog: &Dict) -> Result<()> {
        if let Some(open) = catalog.get("OpenAction") {
            // An array is a destination, not an action (§12.3.2).
            if self.read(open)?.is_some_and(|o| o.as_dict().is_some()) {
                self.actions(open, &Trigger::OpenAction, &Owner::Document)?;
            }
        }
        self.additional(catalog, &Owner::Document)?;
        if let Some(tree) = self.doc.name_tree("JavaScript")? {
            for (name, action) in tree.entries()? {
                let trigger = Trigger::NamedScript(String::from_utf8_lossy(&name).into_owned());
                self.actions(&action, &trigger, &Owner::Document)?;
            }
        }
        Ok(())
    }

    /// The actions of `dict`'s `/AA` dictionary, each under its key.
    pub(super) fn additional(&mut self, dict: &Dict, owner: &Owner) -> Result<()> {
        let Some(aa) = dict.get("AA") else {
            return Ok(());
        };
        let Some(aa) = self.read(aa)? else {
            return Ok(());
        };
        let Some(aa) = aa.as_dict() else {
            return Ok(());
        };
        for (key, action) in aa.iter() {
            let trigger = Trigger::Additional(String::from_utf8_lossy(key.as_bytes()).into_owned());
            self.actions(action, &trigger, owner)?;
        }
        Ok(())
    }

    /// The action at `start` and those its `/Next` reaches (a dictionary or an array of
    /// them), each listed once.
    pub(super) fn actions(&mut self, start: &Object, trigger: &Trigger, owner: &Owner) -> Result<()> {
        let mut stack = vec![start.clone()];
        while let Some(obj) = stack.pop() {
            if self.out.actions.len() >= MAX_ACTIONS {
                return Ok(());
            }
            let object = obj.as_ref().map(|r| r.num);
            if let Some(num) = object
                && !self.seen_actions.insert(num)
            {
                continue;
            }
            let Some(action) = self.read(&obj)? else { continue };
            if let Some(items) = action.as_array() {
                stack.extend(items.iter().rev().cloned());
                continue;
            }
            let Some(dict) = action.as_dict() else { continue };
            let Some(kind) = dict.get("S").and_then(name_of) else {
                continue;
            };
            let kind = ActionKind::from_name(kind.as_bytes());
            let script_bytes = match kind {
                ActionKind::JavaScript => self.script_bytes(dict)?,
                _ => None,
            };
            self.out.actions.push(Action {
                kind,
                trigger: trigger.clone(),
                owner: owner.clone(),
                object,
                script_bytes,
            });
            if let Some(next) = dict.get("Next") {
                stack.push(next.clone());
            }
        }
        Ok(())
    }

    /// The length of a JavaScript action's `/JS`: a string's bytes, or a stream decoded (its
    /// stored length when decoding fails or passes `max_stream_decoded`).
    fn script_bytes(&self, action: &Dict) -> Result<Option<u64>> {
        let Some(js) = action.get("JS") else {
            return Ok(None);
        };
        let Some(js) = self.read(js)? else {
            return Ok(None);
        };
        Ok(match &*js {
            Object::String(s) => Some(len(s.bytes.len())),
            Object::Stream(s) => Some(len(
                tolerant(self.doc.decode_stream(s))?.map_or(s.data.len(), |d| d.len())
            )),
            _ => None,
        })
    }
}
