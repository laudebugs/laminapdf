//! Which objects refer to an object: an index of every reference in the document, built by
//! reading every object once (ISO 32000-2 §7.3.10). An inspector builds it on first use and keeps
//! it while the document is unchanged.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use laminapdf_cos::{ObjRef, Object, Result};

use crate::document::{Document, tolerant};

/// One place that holds a reference ([`ReferenceIndex::to`]).
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

/// Every reference in a document, by the object it refers to ([`Document::reference_index`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferenceIndex {
    /// Referrers by the referenced object's number, in object order.
    by_target: BTreeMap<u32, Vec<Referrer>>,
}

impl ReferenceIndex {
    /// The places that refer to object `num` (any generation), the trailer first, then by
    /// object number.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn to(&self, num: u32) -> &[Referrer] {
        self.by_target.get(&num).map_or(&[], Vec::as_slice)
    }

    /// The number of references in the document.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_target.values().map(Vec::len).sum()
    }

    /// Whether the document holds no reference at all.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_target.is_empty()
    }

    fn walk(&mut self, from: Option<ObjRef>, value: &Object, path: &mut String) {
        match value {
            Object::Ref(target) => self.by_target.entry(target.num).or_default().push(Referrer {
                from,
                path: path.clone(),
            }),
            Object::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    let len = path.len();
                    let _infallible = write!(path, "[{i}]");
                    self.walk(from, item, path);
                    path.truncate(len);
                }
            }
            Object::Dict(dict) => self.walk_dict(from, dict, path),
            Object::Stream(stream) => self.walk_dict(from, &stream.dict, path),
            _ => {}
        }
    }

    fn walk_dict(&mut self, from: Option<ObjRef>, dict: &laminapdf_cos::Dict, path: &mut String) {
        for (key, item) in dict.iter() {
            let len = path.len();
            path.push('/');
            path.push_str(&String::from_utf8_lossy(key.as_bytes()));
            self.walk(from, item, path);
            path.truncate(len);
        }
    }
}

impl Document {
    /// Every reference in the document, by what it refers to: the trailer's, then each
    /// object's (read once, as [`Document::objects`] reads them). An object that can't be read
    /// refers to nothing.
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.6.0.
    pub fn reference_index(&self) -> Result<ReferenceIndex> {
        let mut index = ReferenceIndex::default();
        let mut path = String::new();
        index.walk_dict(None, self.trailer(), &mut path);
        let numbers: Vec<u32> = self.object_numbers().collect();
        for num in numbers {
            let Some(object) = tolerant(self.object(num))? else {
                continue;
            };
            let generation = self.base_generation(num).unwrap_or(0);
            index.walk(Some(ObjRef::new(num, generation)), &object, &mut path);
        }
        Ok(index)
    }
}
