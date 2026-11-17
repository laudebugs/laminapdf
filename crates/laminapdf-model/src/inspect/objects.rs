//! Every object of a document, for an inspector: its number, kind, `/Type` and where it is
//! stored (ISO 32000-2 §7.3, §7.5). Reading an object stream's members decodes the object
//! stream, within the document's `Limits`; an object that can't be read is listed as `null`
//! (ADR 0003), never fails the list.

use laminapdf_cos::{ObjRef, Object, ObjectLocation, Result};

use crate::document::{Document, tolerant};

/// What an object is (ISO 32000-2 §7.3).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ObjectKind {
    /// `null`, or an object that can't be read.
    Null,
    /// `true` or `false`.
    Boolean,
    /// An integer.
    Integer,
    /// A real number.
    Real,
    /// A string.
    String,
    /// A name.
    Name,
    /// An array.
    Array,
    /// A dictionary.
    Dictionary,
    /// A stream.
    Stream,
    /// A reference (an indirect object whose value is another reference).
    Reference,
}

impl ObjectKind {
    /// Every kind, in the order of ISO 32000-2 §7.3: the one list the bindings' names come from.
    ///
    /// Added in 0.6.0.
    pub const ALL: [Self; 10] = [
        Self::Null,
        Self::Boolean,
        Self::Integer,
        Self::Real,
        Self::String,
        Self::Name,
        Self::Array,
        Self::Dictionary,
        Self::Stream,
        Self::Reference,
    ];

    /// The kind of `object`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn of(object: &Object) -> Self {
        match object {
            Object::Null => Self::Null,
            Object::Bool(_) => Self::Boolean,
            Object::Integer(_) => Self::Integer,
            Object::Real(_) => Self::Real,
            Object::String(_) => Self::String,
            Object::Name(_) => Self::Name,
            Object::Array(_) => Self::Array,
            Object::Dict(_) => Self::Dictionary,
            Object::Stream(_) => Self::Stream,
            Object::Ref(_) => Self::Reference,
        }
    }

    /// Its name in lower camel case (`dictionary`, `stream`), as the bindings spell it.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Real => "real",
            Self::String => "string",
            Self::Name => "name",
            Self::Array => "array",
            Self::Dictionary => "dictionary",
            Self::Stream => "stream",
            Self::Reference => "reference",
        }
    }
}

/// One object of a document ([`Document::objects`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ObjectEntry {
    /// Its number and generation.
    pub id: ObjRef,
    /// What it is.
    pub kind: ObjectKind,
    /// A dictionary's or stream's `/Type`, lossily decoded.
    pub type_name: Option<String>,
    /// A dictionary's or stream's `/Subtype` (`/S` for a structure element), lossily decoded.
    pub subtype: Option<String>,
    /// Where it is stored.
    pub location: ObjectLocation,
    /// A stream's data length as stored, in bytes (encoded).
    pub stream_length: Option<u64>,
}

impl Document {
    /// Every object that exists now, by ascending number ([`laminapdf_cos::Document::object_numbers`]),
    /// each read once. Lazy: an object is read when the iterator reaches it.
    ///
    /// # Errors
    /// Each item: reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.6.0.
    pub fn objects(&self) -> impl Iterator<Item = Result<ObjectEntry>> + '_ {
        self.each_object()
            .map(|item| item.map(|(id, object)| self.entry_of(id, object.as_deref())))
    }

    /// Object `num`'s entry; `None` when the document has no such object.
    ///
    /// # Errors
    /// Reading errors that aren't repairs.
    ///
    /// Added in 0.6.0.
    pub fn object_entry(&self, num: u32) -> Result<Option<ObjectEntry>> {
        let Some(id) = self.object_id(num) else {
            return Ok(None);
        };
        let object = tolerant(self.object(num))?;
        Ok(Some(self.entry_of(id, object.as_deref())))
    }

    /// Object `num`'s entry from its value `object`, already read (`None`: it can't be read);
    /// `None` when the document has no such object. A caller that needs the value too reads it
    /// once.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn entry_of_number(&self, num: u32, object: Option<&Object>) -> Option<ObjectEntry> {
        Some(self.entry_of(self.object_id(num)?, object))
    }

    /// The entry of object `id` whose value is `object` (`None`: it can't be read).
    fn entry_of(&self, id: ObjRef, object: Option<&Object>) -> ObjectEntry {
        let object = object.unwrap_or(&Object::Null);
        let dict = match object {
            Object::Dict(d) => Some(d),
            Object::Stream(s) => Some(&s.dict),
            _ => None,
        };
        let name_of = |key: &str| {
            dict.and_then(|d| d.get_name(key))
                .map(|n| String::from_utf8_lossy(n).into_owned())
        };
        ObjectEntry {
            id,
            kind: ObjectKind::of(object),
            type_name: name_of("Type"),
            subtype: name_of("Subtype").or_else(|| name_of("S")),
            location: self.object_location(id.num).unwrap_or(ObjectLocation::Created),
            stream_length: match object {
                Object::Stream(s) => Some(u64::try_from(s.data.len()).unwrap_or(u64::MAX)),
                _ => None,
            },
        }
    }
}
