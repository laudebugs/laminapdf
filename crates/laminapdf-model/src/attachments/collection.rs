//! Portfolios ([`Document::collection`]): the catalog's `/Collection` (ISO 32000-2 §12.3.5),
//! which asks a viewer to present the document as a list of its embedded files, with the
//! columns its schema defines and each file's values for them (`/CI`).

use laminapdf_cos::{Dict, Object, Result};

use crate::document::Document;
use crate::id::AttachmentId;
use crate::values::{read_name, text_of};

/// How a portfolio asks to be shown first (`/View`, Table 153).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CollectionView {
    /// `/D`: a list of the files with the schema's columns (the default).
    #[default]
    Details,
    /// `/T`: tiles, one per file.
    Tile,
    /// `/H`: the files' list hidden, the initial document shown.
    Hidden,
    /// `/C`: a custom presentation (a navigator, §12.3.5.4), which Lamina PDF shows as a list.
    Custom,
}

/// What a schema field shows (`/Subtype`, Table 155).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectionFieldKind {
    /// `/S`: text from the file's collection item.
    Text,
    /// `/D`: a date from the file's collection item.
    Date,
    /// `/N`: a number from the file's collection item.
    Number,
    /// `/F`: the file's name.
    FileName,
    /// `/Desc`: the file's description.
    Description,
    /// `/ModDate`: the file's modification date.
    Modified,
    /// `/CreationDate`: the file's creation date.
    Created,
    /// `/Size`: the file's size.
    Size,
    /// `/CompressedSize`: the embedded stream's stored length.
    CompressedSize,
    /// A subtype the standard doesn't define, by name.
    Other(String),
}

/// One column of a portfolio's schema (Table 155).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CollectionField {
    /// Its key in the schema, which collection items use.
    pub key: String,
    /// `/N`: the column's heading.
    pub name: Option<String>,
    /// `/Subtype`: what it shows.
    pub kind: CollectionFieldKind,
    /// `/O`: its place among the columns, when given.
    pub order: Option<i64>,
    /// `/V`: whether it is shown (default true).
    pub visible: bool,
    /// `/E`: whether a viewer may let the user edit it (default false).
    pub editable: bool,
}

/// A value of a file's collection item (`/CI`, §12.3.5.3).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CollectionValue {
    /// A text string, decoded.
    Text(String),
    /// A number.
    Number(f64),
}

impl Eq for CollectionValue {}

/// A file of the portfolio: an entry of the `/EmbeddedFiles` tree, with its values for the
/// schema's columns.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CollectionFile {
    /// The attachment.
    pub attachment: AttachmentId,
    /// Its key in the tree, decoded.
    pub key: String,
    /// Its collection item's values, by schema key, in the item's order; a value with a
    /// prefix (a `CollectionSubitem`'s `/P`) is given without it.
    pub values: Vec<(String, CollectionValue)>,
}

/// A portfolio (§12.3.5): the catalog's `/Collection`, read.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Collection {
    /// `/Schema`: the columns, ordered by `/O` (fields without one last, by key).
    pub schema: Vec<CollectionField>,
    /// `/Sort`: the schema keys the files are sorted by, each with whether it is ascending.
    pub sort: Vec<(String, bool)>,
    /// `/View`.
    pub view: CollectionView,
    /// `/D`: the key of the file to show first; `None` for the portfolio's own pages.
    pub initial: Option<String>,
    /// The files, in the order of the `/EmbeddedFiles` tree.
    pub files: Vec<CollectionFile>,
}

impl Document {
    /// The portfolio the document is, when its catalog has a `/Collection`; `None` otherwise.
    ///
    /// # Errors
    /// Only errors that must propagate ([`laminapdf_cos::Error::must_propagate`]).
    ///
    /// Added in 0.6.0.
    pub fn collection(&self) -> Result<Option<Collection>> {
        let catalog = self.catalog()?;
        let Some(value) = catalog.get("Collection") else {
            return Ok(None);
        };
        let Some(collection) = self.read(value)? else {
            return Ok(None);
        };
        let Some(collection) = collection.as_dict() else {
            return Ok(None);
        };
        let mut out = Collection {
            schema: self.collection_schema(collection)?,
            sort: self.collection_sort(collection)?,
            view: match read_name(self, collection.get("View"))?.as_deref() {
                Some(b"T") => CollectionView::Tile,
                Some(b"H") => CollectionView::Hidden,
                Some(b"C") => CollectionView::Custom,
                _ => CollectionView::Details,
            },
            initial: match collection.get("D") {
                Some(d) => self.read(d)?.as_deref().and_then(text_of),
                None => None,
            },
            files: Vec::new(),
        };
        for found in self.found_attachments()? {
            let Some(key) = found.attachment.key else { continue };
            out.files.push(CollectionFile {
                attachment: found.attachment.id,
                key,
                values: self.collection_item(&found.spec)?,
            });
        }
        Ok(Some(out))
    }

    fn collection_schema(&self, collection: &Dict) -> Result<Vec<CollectionField>> {
        let Some(schema) = collection.get("Schema") else {
            return Ok(Vec::new());
        };
        let Some(schema) = self.read(schema)? else {
            return Ok(Vec::new());
        };
        let mut fields = Vec::new();
        for (key, field) in schema.as_dict().into_iter().flat_map(Dict::iter) {
            let Some(field) = self.read(field)? else { continue };
            let Some(field) = field.as_dict() else { continue };
            let Some(subtype) = read_name(self, field.get("Subtype"))? else {
                continue;
            };
            let flag = |k: &str, default: bool| match field.get(k) {
                Some(Object::Bool(b)) => *b,
                _ => default,
            };
            fields.push(CollectionField {
                key: String::from_utf8_lossy(key.as_bytes()).into_owned(),
                name: self.text_entry(field, "N")?,
                kind: field_kind(&subtype),
                order: field.get_int("O"),
                visible: flag("V", true),
                editable: flag("E", false),
            });
        }
        fields.sort_by(|a, b| match (a.order, b.order) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.key.cmp(&b.key)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.key.cmp(&b.key),
        });
        Ok(fields)
    }

    fn collection_sort(&self, collection: &Dict) -> Result<Vec<(String, bool)>> {
        let Some(sort) = collection.get("Sort") else {
            return Ok(Vec::new());
        };
        let Some(sort) = self.read(sort)? else {
            return Ok(Vec::new());
        };
        let Some(sort) = sort.as_dict() else {
            return Ok(Vec::new());
        };
        let keys: Vec<String> = match sort.get("S").map(|s| self.read(s)).transpose()?.flatten().as_deref() {
            Some(Object::Name(n)) => vec![String::from_utf8_lossy(n.as_bytes()).into_owned()],
            Some(Object::Array(items)) => items
                .iter()
                .filter_map(|o| match o {
                    Object::Name(n) => Some(String::from_utf8_lossy(n.as_bytes()).into_owned()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        let ascending: Vec<bool> = match sort.get("A").map(|a| self.read(a)).transpose()?.flatten().as_deref() {
            Some(Object::Bool(b)) => vec![*b],
            Some(Object::Array(items)) => items.iter().map(|o| !matches!(o, Object::Bool(false))).collect(),
            _ => Vec::new(),
        };
        // §12.3.5.2: one `/A` for several keys applies to the first; the rest are ascending.
        Ok(keys
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, ascending.get(i).copied().unwrap_or(true)))
            .collect())
    }

    /// The values of the file specification `spec`'s collection item (`/CI`).
    fn collection_item(&self, spec: &Dict) -> Result<Vec<(String, CollectionValue)>> {
        let Some(item) = spec.get("CI") else {
            return Ok(Vec::new());
        };
        let Some(item) = self.read(item)? else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for (key, value) in item.as_dict().into_iter().flat_map(Dict::iter) {
            if key.as_bytes() == b"Type" {
                continue;
            }
            let Some(value) = self.read(value)? else { continue };
            // A `CollectionSubitem` gives its value in `/D`, with a prefix in `/P`.
            let value = match value.as_dict() {
                Some(sub) => match sub.get("D") {
                    Some(d) => self.read(d)?,
                    None => None,
                },
                None => Some(value),
            };
            let value = match value.as_deref() {
                Some(Object::String(s)) => CollectionValue::Text(laminapdf_syntax::text::decode_text_string(&s.bytes)),
                Some(other) => match other.as_f64() {
                    Some(n) => CollectionValue::Number(n),
                    None => continue,
                },
                None => continue,
            };
            out.push((String::from_utf8_lossy(key.as_bytes()).into_owned(), value));
        }
        Ok(out)
    }
}

fn field_kind(subtype: &[u8]) -> CollectionFieldKind {
    match subtype {
        b"S" => CollectionFieldKind::Text,
        b"D" => CollectionFieldKind::Date,
        b"N" => CollectionFieldKind::Number,
        b"F" => CollectionFieldKind::FileName,
        b"Desc" => CollectionFieldKind::Description,
        b"ModDate" => CollectionFieldKind::Modified,
        b"CreationDate" => CollectionFieldKind::Created,
        b"Size" => CollectionFieldKind::Size,
        b"CompressedSize" => CollectionFieldKind::CompressedSize,
        other => CollectionFieldKind::Other(String::from_utf8_lossy(other).into_owned()),
    }
}
