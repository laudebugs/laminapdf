//! Whether optional content is off as the document opens (ISO 32000-2 §8.11): an optional
//! content group against the default configuration's `/BaseState`, `/ON` and `/OFF`
//! (§8.11.4.3), and a membership dictionary by its `/OCGs` and `/P` policy (§8.11.2.2). A
//! visibility expression (`/VE`) and the usage application dictionaries aren't evaluated: the
//! membership's `/P` decides, as a reader without them does. Anything that can't be read counts
//! as on, so a caller never drops what a viewer shows.

use laminapdf_cos::{Dict, ObjRef, Object, Result};

use crate::document::Document;

impl Document {
    /// Whether the content that the `/OC` value `oc` marks (an optional content group or a
    /// membership dictionary, by reference) is hidden in the default configuration, as the
    /// document opens. `false` for a document without optional content and for anything that
    /// can't be read.
    ///
    /// # Errors
    /// Only what must propagate (strict mode, pending data, a source error).
    ///
    /// Added in 0.6.0.
    pub fn optional_content_off(&self, oc: &Object) -> Result<bool> {
        let Some(config) = self.default_config()? else {
            return Ok(false);
        };
        let Some(target) = oc.as_ref() else {
            return Ok(false);
        };
        let Some(dict) = self.read(oc)?.and_then(|o| o.as_dict().cloned()) else {
            return Ok(false);
        };
        if dict.get_name("Type") != Some(b"OCMD".as_slice()) {
            return Ok(!config.on(target));
        }
        let groups: Vec<ObjRef> = match dict.get("OCGs") {
            Some(Object::Ref(r)) => match self.read(&Object::Ref(*r))?.as_deref() {
                Some(Object::Array(items)) => items.iter().filter_map(Object::as_ref).collect(),
                Some(Object::Dict(_)) => vec![*r],
                _ => Vec::new(),
            },
            Some(Object::Array(items)) => items.iter().filter_map(Object::as_ref).collect(),
            _ => Vec::new(),
        };
        if groups.is_empty() {
            return Ok(false);
        }
        let on = groups.iter().filter(|g| config.on(**g)).count();
        let visible = match dict.get_name("P") {
            Some(b"AllOn") => on == groups.len(),
            Some(b"AnyOff") => on < groups.len(),
            Some(b"AllOff") => on == 0,
            _ => on > 0,
        };
        Ok(!visible)
    }

    /// `/OCProperties /D`, read through references; `None` when the document has none.
    fn default_config(&self) -> Result<Option<Config>> {
        let catalog = self.catalog()?;
        let Some(properties) = self.dict_of(catalog.get("OCProperties"))? else {
            return Ok(None);
        };
        let Some(d) = self.dict_of(properties.get("D"))? else {
            return Ok(None);
        };
        let refs = |key: &str| -> Result<Vec<ObjRef>> {
            Ok(match d.get(key) {
                Some(value) => self
                    .read(value)?
                    .as_deref()
                    .and_then(Object::as_array)
                    .map(|items| items.iter().filter_map(Object::as_ref).collect())
                    .unwrap_or_default(),
                None => Vec::new(),
            })
        };
        Ok(Some(Config {
            base_off: d.get_name("BaseState") == Some(b"OFF".as_slice()),
            on: refs("ON")?,
            off: refs("OFF")?,
        }))
    }

    fn dict_of(&self, value: Option<&Object>) -> Result<Option<Dict>> {
        Ok(match value {
            Some(value) => self.read(value)?.and_then(|o| o.as_dict().cloned()),
            None => None,
        })
    }
}

/// The default configuration's states.
struct Config {
    base_off: bool,
    on: Vec<ObjRef>,
    off: Vec<ObjRef>,
}

impl Config {
    fn on(&self, group: ObjRef) -> bool {
        if self.base_off {
            self.on.contains(&group)
        } else {
            !self.off.contains(&group)
        }
    }
}
