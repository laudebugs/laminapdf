//! Pages' annotations and actions, the interactive form's fields (§12.7) and the layers the
//! default configuration turns off (§8.11.4.3).

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, Object, Result};

use super::{Annotation, EmbeddedFileHolder, FormField, Owner, Scan, Trigger, name_of, text_of};
use crate::document::Document;
use crate::page::Page;

/// A field on the walk: the object, its parent's full name, and the `/FT` and `/V` it
/// inherits, at a depth.
type Inherit = (Object, String, Option<String>, Option<Object>, usize);

/// `/F` bits that hide an annotation: Hidden (2) and `NoView` (6), §12.5.3.
const HIDDEN_FLAGS: i64 = (1 << 1) | (1 << 5);

impl Scan<'_> {
    /// Each page's `/AA` and annotations: their subtype, author and comment, their actions,
    /// and the files `FileAttachment` annotations hold.
    pub(super) fn pages(&mut self) -> Result<()> {
        let pages = self.doc.pages()?;
        for (index, page) in pages.iter().enumerate() {
            let dict = &page.cos().dict;
            self.additional(dict, &Owner::Page(index))?;
            let Some(annots) = dict.get("Annots") else { continue };
            let Some(annots) = self.read(annots)? else { continue };
            for entry in annots.as_array().unwrap_or_default() {
                self.annotation(index, entry)?;
            }
        }
        Ok(())
    }

    fn annotation(&mut self, page: usize, entry: &Object) -> Result<()> {
        let object = entry.as_ref().map(|r| r.num);
        let Some(annot) = self.read(entry)? else {
            return Ok(());
        };
        let Some(dict) = annot.as_dict() else {
            return Ok(());
        };
        let subtype = subtype(self.doc, dict)?;
        let owner = Owner::Annotation { page, object };
        if let Some(a) = dict.get("A") {
            self.actions(a, &Trigger::Activate, &owner)?;
        }
        self.additional(dict, &owner)?;
        if subtype == "FileAttachment"
            && let Some(fs) = dict.get("FS")
        {
            self.hold(fs, EmbeddedFileHolder::Annotation)?;
        }
        let author = self.text(dict, "T")?;
        let contents = self.text(dict, "Contents")?;
        self.out.annotations.push(Annotation {
            subtype,
            page,
            object,
            author,
            contents,
        });
        Ok(())
    }

    fn text(&self, dict: &Dict, key: &str) -> Result<Option<String>> {
        match dict.get(key) {
            Some(v) => Ok(self.read(v)?.as_deref().and_then(text_of)),
            None => Ok(None),
        }
    }

    /// The terminal fields of `/AcroForm /Fields`, with their full names, values and
    /// actions, and whether the form has an XFA part.
    pub(super) fn fields(&mut self, catalog: &Dict) -> Result<()> {
        let Some(form) = catalog.get("AcroForm") else {
            return Ok(());
        };
        let Some(form) = self.read(form)? else {
            return Ok(());
        };
        let Some(form) = form.as_dict() else {
            return Ok(());
        };
        self.out.xfa = form.get("XFA").is_some();
        let Some(fields) = form.get("Fields") else {
            return Ok(());
        };
        let Some(fields) = self.read(fields)? else {
            return Ok(());
        };
        let max_depth = self.doc.limits().max_tree_depth();
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        // (field, parent's full name, inherited /FT, depth), in document order.
        let mut stack: Vec<Inherit> = fields
            .as_array()
            .unwrap_or_default()
            .iter()
            .rev()
            .map(|f| (f.clone(), String::new(), None, None, 0))
            .collect();
        while let Some((obj, parent, inherited, inherited_value, depth)) = stack.pop() {
            if depth > max_depth || obj.as_ref().is_some_and(|r| !seen.insert(r.num)) {
                continue;
            }
            let Some(field) = self.read(&obj)? else { continue };
            let Some(dict) = field.as_dict() else { continue };
            let partial = self.text(dict, "T")?;
            let name = match (&partial, parent.is_empty()) {
                (Some(t), true) => t.clone(),
                (Some(t), false) => format!("{parent}.{t}"),
                (None, _) => parent.clone(),
            };
            let kind = dict.get("FT").and_then(name_of).or(inherited);
            // `/V` is inheritable too (§12.7.4.1): a value set on a parent field holds for its kids.
            let own_value = dict.get("V").cloned().or(inherited_value);
            let kids = self.kids(dict)?;
            let child_fields: Vec<&Object> = kids
                .iter()
                .filter(|(_, k)| k.get("T").is_some())
                .map(|(o, _)| o)
                .collect();
            if !child_fields.is_empty() {
                for kid in child_fields.into_iter().rev() {
                    stack.push((kid.clone(), name.clone(), kind.clone(), own_value.clone(), depth + 1));
                }
                continue;
            }
            // A merged field and widget's actions are listed with the annotation.
            if dict.get_name("Subtype") != Some(b"Widget") {
                self.additional(dict, &Owner::FormField(name.clone()))?;
            }
            let widgets: Vec<&Dict> = if kids.is_empty() {
                vec![dict]
            } else {
                kids.iter().map(|(_, k)| k).collect()
            };
            let hidden = widgets
                .iter()
                .all(|w| w.get_int("F").is_some_and(|f| f & HIDDEN_FLAGS != 0));
            let value = match &own_value {
                Some(v) => self.read(v)?.as_deref().and_then(value_of),
                None => None,
            };
            self.out.fields.push(FormField {
                name,
                kind,
                value,
                hidden,
                object: obj.as_ref().map(|r| r.num),
            });
        }
        Ok(())
    }

    /// `/Kids`, each with its dictionary.
    fn kids(&self, dict: &Dict) -> Result<Vec<(Object, Dict)>> {
        let Some(kids) = dict.get("Kids") else {
            return Ok(Vec::new());
        };
        let Some(kids) = self.read(kids)? else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for kid in kids.as_array().unwrap_or_default() {
            if let Some(d) = self.read(kid)?.as_deref().and_then(Object::as_dict) {
                out.push((kid.clone(), d.clone()));
            }
        }
        Ok(out)
    }

    /// The names of the optional content groups `/OCProperties /D /OFF` lists.
    pub(super) fn layers(&mut self, catalog: &Dict) -> Result<()> {
        let mut current = catalog.get("OCProperties").cloned();
        for key in ["D", "OFF"] {
            let Some(obj) = current.take() else {
                return Ok(());
            };
            current = self
                .read(&obj)?
                .as_deref()
                .and_then(Object::as_dict)
                .and_then(|d| d.get(key))
                .cloned();
        }
        let Some(off) = current else {
            return Ok(());
        };
        let Some(off) = self.read(&off)? else {
            return Ok(());
        };
        for group in off.as_array().unwrap_or_default() {
            if let Some(g) = self.read(group)?.as_deref().and_then(Object::as_dict) {
                let name = self.text(g, "Name")?.unwrap_or_default();
                self.out.layers_off.push(name);
            }
        }
        Ok(())
    }
}

/// An annotation's `/Subtype`, resolved when it is a reference; empty when it isn't a name.
pub(super) fn subtype(doc: &Document, annot: &Dict) -> Result<String> {
    match annot.get("Subtype") {
        Some(v) => Ok(doc.read(v)?.as_deref().and_then(name_of).unwrap_or_default()),
        None => Ok(String::new()),
    }
}

impl Document {
    /// How many `Redact` annotations (§12.5.6.23) `page` has: marks a redaction tool left
    /// without applying them. `/Annots` entries and `/Subtype` may be references.
    ///
    /// # Errors
    /// The errors that must propagate (`data-pending`, the source's, strict mode).
    pub fn redact_annotations(&self, page: &Page<'_>) -> Result<u32> {
        let Some(annots) = page.cos().dict.get("Annots") else {
            return Ok(0);
        };
        let Some(annots) = self.read(annots)? else {
            return Ok(0);
        };
        let mut n = 0u32;
        for entry in annots.as_array().unwrap_or_default() {
            if let Some(annot) = self.read(entry)?
                && let Some(dict) = annot.as_dict()
                && subtype(self, dict)? == "Redact"
            {
                n = n.saturating_add(1);
            }
        }
        Ok(n)
    }
}

/// A field value as text: a text string decoded, a name as written, an array's items joined.
fn value_of(value: &Object) -> Option<String> {
    match value {
        Object::Array(items) => {
            let parts: Vec<String> = items.iter().filter_map(|i| text_of(i).or_else(|| name_of(i))).collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        other => text_of(other).or_else(|| name_of(other)),
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::{Object, PdfString};

    use super::value_of;

    #[test]
    fn values_read_as_text() {
        assert_eq!(value_of(&Object::name("Yes")).as_deref(), Some("Yes"));
        let s = Object::String(PdfString::literal(b"filled".to_vec()));
        assert_eq!(value_of(&s).as_deref(), Some("filled"));
        let a = Object::Array(vec![
            Object::String(PdfString::literal(b"a".to_vec())),
            Object::name("b"),
        ]);
        assert_eq!(value_of(&a).as_deref(), Some("a, b"));
        assert_eq!(value_of(&Object::Integer(3)), None);
    }
}
