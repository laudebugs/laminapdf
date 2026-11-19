//! Form data (E8 plan §2.1, decision 2): each field's value by its fully qualified name, as XFDF's
//! `<fields>`, FDF's `/Fields` and Lamina PDF's JSON carry it. The files nest names by their
//! parts (`address.city` is `city` inside `address`); this module turns the flat list into that
//! tree and back.

/// A form data file's format.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormDataFormat {
    /// XFDF (ISO 19444-1): XML.
    Xfdf,
    /// FDF (ISO 32000-2 §12.7.8): PDF syntax.
    Fdf,
    /// Lamina PDF's JSON, the object `fill_form` takes.
    Json,
}

impl FormDataFormat {
    /// The format's name as people know it: `XFDF`, `FDF` or `JSON`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Xfdf => "XFDF",
            Self::Fdf => "FDF",
            Self::Json => "JSON",
        }
    }
}

impl std::fmt::Display for FormDataFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A form field's value in a form data file, by the field's fully qualified name.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldData {
    /// The fully qualified name: each ancestor's partial name, then the field's, joined by `.`
    /// (ISO 32000-2 §12.7.4.2).
    pub name: String,
    /// The value.
    pub value: DataValue,
}

impl FieldData {
    /// `name`'s value `value`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(name: impl Into<String>, value: DataValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// A value as a form data file holds it, before it is matched to a field's kind.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataValue {
    /// Text: a text field's value, or a button's on-state or export value as XFDF and JSON
    /// write it.
    Text(String),
    /// A button's state by name, as FDF writes it (`/Yes`, `/Off`).
    State(String),
    /// A check box with one on-state, as JSON writes it: `true` on, `false` off.
    Checked(bool),
    /// A list box's selected export values, several when it allows them.
    Choices(Vec<String>),
    /// No value: the field is empty.
    Empty,
}

/// Fields grouped by the first part of their names, in first-seen order: the tree XFDF and FDF
/// write. A node holds the field whose name ends there, if one does, and the nodes below it.
#[derive(Debug, Default)]
pub(crate) struct NameTree<'f> {
    pub(crate) children: Vec<(&'f str, NameTree<'f>)>,
    pub(crate) value: Option<&'f DataValue>,
}

impl<'f> NameTree<'f> {
    /// The tree of `fields`' names.
    pub(crate) fn of(fields: &'f [FieldData]) -> Self {
        let mut root = Self::default();
        for field in fields {
            let mut node = &mut root;
            for part in field.name.split('.') {
                let at = node.children.iter().position(|(p, _)| *p == part).unwrap_or_else(|| {
                    node.children.push((part, Self::default()));
                    node.children.len() - 1
                });
                let Some((_, child)) = node.children.get_mut(at) else {
                    break;
                };
                node = child;
            }
            node.value = Some(&field.value);
        }
        root
    }
}

/// `fields` with each name once: a later value for a name replaces an earlier one where the
/// earlier stood, as fields with one name share a value (ISO 32000-2 §12.7.4.2). XFDF and FDF
/// nest names, so a file can't write a name twice; reading one back gives the same fields.
pub(crate) fn one_per_name(fields: Vec<FieldData>) -> Vec<FieldData> {
    let mut out: Vec<FieldData> = Vec::with_capacity(fields.len());
    for field in fields {
        match out.iter_mut().find(|f| f.name == field.name) {
            Some(earlier) => earlier.value = field.value,
            None => out.push(field),
        }
    }
    out
}

/// `name` below `parent`, joined as a fully qualified name.
pub(crate) fn qualified(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}.{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_per_name_should_keep_the_first_place_and_the_last_value() {
        let fields = one_per_name(vec![
            FieldData::new("a", DataValue::Text("1".into())),
            FieldData::new("b", DataValue::Empty),
            FieldData::new("a", DataValue::Text("2".into())),
        ]);
        assert_eq!(
            fields,
            [
                FieldData::new("a", DataValue::Text("2".into())),
                FieldData::new("b", DataValue::Empty)
            ]
        );
    }

    #[test]
    fn name_tree_should_nest_names_by_their_parts_in_first_seen_order() {
        let fields = [
            FieldData::new("address.city", DataValue::Text("Lagos".into())),
            FieldData::new("name", DataValue::Text("Ada".into())),
            FieldData::new("address.street", DataValue::Empty),
        ];
        let tree = NameTree::of(&fields);
        let shape: Vec<(&str, Vec<&str>)> = tree
            .children
            .iter()
            .map(|(p, n)| (*p, n.children.iter().map(|(c, _)| *c).collect()))
            .collect();
        assert_eq!(shape, [("address", vec!["city", "street"]), ("name", vec![])]);
    }
}
