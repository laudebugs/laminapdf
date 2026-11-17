//! Which objects refer to an object (ISO 32000-2 §7.3.10): an index of who holds a reference
//! to whom, built by reading every object once, and the key paths inside a holder, found by
//! reading that one holder again when they are asked for. The index keeps one number per holder,
//! so even a large file's index stays small.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use laminapdf_cos::{Dict, ObjRef, Object, Result};

use crate::document::{Document, tolerant};

/// One place that holds a reference ([`Document::referrers`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct Referrer {
    /// The object that holds it; `None` for the trailer.
    pub from: Option<ObjRef>,
    /// Where in that object, as keys and indexes (`/Kids[2]`, `/Resources/Font/F1`); empty
    /// when the object's value is the reference itself.
    pub path: String,
}

/// The holder number standing for the trailer.
const TRAILER: u32 = u32::MAX;

/// Who holds a reference to each object ([`Document::reference_index`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferenceIndex {
    /// The holders of a reference to each object, by its number: object numbers ascending, the
    /// trailer as [`TRAILER`], each once.
    by_target: BTreeMap<u32, Vec<u32>>,
}

impl ReferenceIndex {
    /// The holders of a reference to object `num`: the trailer as `None` first, then objects
    /// by number.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn holders(&self, num: u32) -> Vec<Option<u32>> {
        let list = self.by_target.get(&num).map_or(&[][..], Vec::as_slice);
        let trailer = list.contains(&TRAILER).then_some(None);
        trailer
            .into_iter()
            .chain(list.iter().filter(|&&h| h != TRAILER).map(|&h| Some(h)))
            .collect()
    }

    /// Whether the document holds no reference at all.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_target.is_empty()
    }

    fn note(&mut self, holder: u32, value: &Object) {
        match value {
            Object::Ref(target) => {
                let list = self.by_target.entry(target.num).or_default();
                if list.last() != Some(&holder) {
                    list.push(holder);
                }
            }
            Object::Array(items) => items.iter().for_each(|item| self.note(holder, item)),
            Object::Dict(dict) => dict.iter().for_each(|(_, item)| self.note(holder, item)),
            Object::Stream(stream) => stream.dict.iter().for_each(|(_, item)| self.note(holder, item)),
            _ => {}
        }
    }
}

/// The key paths inside `value` at which it refers to object `target`.
fn paths_to(value: &Object, target: u32, path: &mut String, out: &mut Vec<String>) {
    match value {
        Object::Ref(r) if r.num == target => out.push(path.clone()),
        Object::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let len = path.len();
                let _infallible = write!(path, "[{i}]");
                paths_to(item, target, path, out);
                path.truncate(len);
            }
        }
        Object::Dict(dict) => dict_paths_to(dict, target, path, out),
        Object::Stream(stream) => dict_paths_to(&stream.dict, target, path, out),
        _ => {}
    }
}

fn dict_paths_to(dict: &Dict, target: u32, path: &mut String, out: &mut Vec<String>) {
    for (key, item) in dict.iter() {
        let len = path.len();
        path.push('/');
        path.push_str(&String::from_utf8_lossy(key.as_bytes()));
        paths_to(item, target, path, out);
        path.truncate(len);
    }
}

impl Document {
    /// Who holds a reference to each object: the trailer, then every object (read once, as
    /// [`Document::objects`] reads them). An object that can't be read holds nothing.
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.6.0.
    pub fn reference_index(&self) -> Result<ReferenceIndex> {
        let mut index = ReferenceIndex::default();
        index.note(TRAILER, &Object::Dict(self.trailer().clone()));
        for item in self.each_object() {
            if let (id, Some(object)) = item? {
                index.note(id.num, &object);
            }
        }
        Ok(index)
    }

    /// The places that refer to object `num`, by `index`'s holders: the trailer's keys first,
    /// then each object's, with the key path inside it. Only the holders are read again.
    ///
    /// # Errors
    /// Reading errors that aren't repairs.
    ///
    /// Added in 0.6.0.
    pub fn referrers(&self, index: &ReferenceIndex, num: u32) -> Result<Vec<Referrer>> {
        let mut out = Vec::new();
        for holder in index.holders(num) {
            let mut paths = Vec::new();
            let from = match holder {
                None => {
                    dict_paths_to(self.trailer(), num, &mut String::new(), &mut paths);
                    None
                }
                Some(h) => {
                    let Some(object) = tolerant(self.object(h))? else {
                        continue;
                    };
                    paths_to(&object, num, &mut String::new(), &mut paths);
                    self.object_id(h)
                }
            };
            out.extend(paths.into_iter().map(|path| Referrer { from, path }));
        }
        Ok(out)
    }
}
