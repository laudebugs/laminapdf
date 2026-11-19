//! The fields a `FieldMDP` lock or a signature field's `/Lock` covers (ISO 32000-2 §12.8.2.4,
//! Table 258; §12.7.5.5), by fully qualified name, and how authoring a field meets them
//! ([ADR 0007] §11–§13).
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use laminapdf_cos::{Dict, Document, Object};
use laminapdf_syntax::text::decode_text_string;

use crate::error::Result;

/// The fields a `FieldMDP` lock covers (§12.8.2.4, Table 258), by fully qualified name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Lock {
    /// `/Action /All`, or an action this reader doesn't know (the strictest reading).
    All,
    /// `/Action /Include`: the fields named in `/Fields`.
    Include(Vec<String>),
    /// `/Action /Exclude`: every field but those named in `/Fields`.
    Exclude(Vec<String>),
}

impl Lock {
    /// The lock that `params` (a `/Lock` dictionary or `FieldMDP` transform parameters)
    /// describes; `/Fields` and its names may be indirect. Read tolerantly (ADR 0003): a list
    /// that is missing (Table 258 requires it for `/Include` and `/Exclude`), isn't an array or
    /// can't be read whole makes the lock [`Lock::All`], the strictest reading.
    ///
    /// # Errors
    /// Unavailable input and hard limits reading `/Fields`.
    pub(crate) fn from_params(doc: &Document, params: &Dict) -> Result<Self> {
        let action = params.get_name("Action");
        if !matches!(action, Some(b"Include" | b"Exclude")) {
            return Ok(Self::All);
        }
        let mut names = Vec::new();
        let Some(fields) = params.get("Fields") else {
            return Ok(Self::All);
        };
        let Some(fields) = laminapdf_cos::tolerant(doc.deref(fields))? else {
            return Ok(Self::All);
        };
        let Some(items) = fields.as_array() else {
            return Ok(Self::All);
        };
        for item in items {
            match laminapdf_cos::tolerant(doc.deref(item))?.as_deref() {
                Some(Object::String(name)) => names.push(decode_text_string(&name.bytes)),
                _ => return Ok(Self::All),
            }
        }
        Ok(if action == Some(b"Include") {
            Self::Include(names)
        } else {
            Self::Exclude(names)
        })
    }

    /// Whether the lock covers the field named `name`: a listed name covers that field and
    /// the fields below it. A field whose name couldn't be read (`None`) is covered by every
    /// lock, since it may be one the lock names.
    ///
    /// Table 258 says only "field names", and ISO 32000-2 §12.7.4.2 has both fully qualified
    /// and partial ones, so each list is read the stricter way. An `/Include` name also counts
    /// as the trailing part of a fully qualified one (`street` covers `address.street`), while
    /// an `/Exclude` name frees only the field with that fully qualified name and those below.
    pub(crate) fn covers(&self, name: Option<&str>) -> bool {
        let Some(name) = name else { return true };
        let qualified = |names: &[String]| names.iter().any(|n| at_or_below(name, n));
        let partial = |names: &[String]| {
            let dotted = format!(".{name}.");
            names.iter().any(|n| dotted.contains(&format!(".{n}.")))
        };
        match self {
            Self::All => true,
            Self::Include(names) => partial(names),
            Self::Exclude(names) => !qualified(names),
        }
    }

    /// Whether the lock covers the field named `name` or any field below it: what authoring
    /// the field (which renames, deletes or redefines the fields below it too) is held to. An
    /// `/Include` lock also covers a field above one it names; an `/Exclude` lock covers every
    /// field it doesn't name, a field created after signing included (E9 plan §5).
    pub(crate) fn covers_subtree(&self, name: Option<&str>) -> bool {
        let below = |names: &[String]| name.is_some_and(|name| names.iter().any(|n| at_or_below(n, name) && n != name));
        self.covers(name) || matches!(self, Self::Include(names) if below(names))
    }
}

/// Whether the field named `name` is the one named `of` or below it (both fully qualified).
pub(crate) fn at_or_below(name: &str, of: &str) -> bool {
    name.strip_prefix(of)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

/// Whether the fields named `a` and `b` (both fully qualified, read from the form) are one, or
/// one is below the other; a name that couldn't be read (`None`) may be either.
pub(crate) fn related(a: Option<&str>, b: Option<&str>) -> bool {
    let (Some(a), Some(b)) = (a, b) else { return true };
    at_or_below(a, b) || at_or_below(b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock(action: &str, fields: &[&str]) -> Lock {
        let params: Dict = [
            ("Action", Object::name(action)),
            (
                "Fields",
                Object::Array(
                    fields
                        .iter()
                        .map(|f| Object::String(laminapdf_cos::PdfString::literal(f.as_bytes().to_vec())))
                        .collect(),
                ),
            ),
        ]
        .into_iter()
        .collect();
        Lock::from_params(&Document::empty(None, &laminapdf_cos::OpenOptions::default()), &params).unwrap()
    }

    fn from(params: &[(&str, Object)]) -> Lock {
        let params: Dict = params.iter().cloned().collect();
        Lock::from_params(&Document::empty(None, &laminapdf_cos::OpenOptions::default()), &params).unwrap()
    }

    #[test]
    fn a_list_that_is_not_an_array_locks_every_field() {
        let single = Object::String(laminapdf_cos::PdfString::literal(b"name".to_vec()));
        assert_eq!(
            from(&[("Action", Object::name("Include")), ("Fields", single)]),
            Lock::All
        );
    }

    #[test]
    fn a_missing_list_locks_every_field() {
        assert_eq!(
            (
                from(&[("Action", Object::name("Include"))]),
                from(&[("Action", Object::name("Exclude"))])
            ),
            (Lock::All, Lock::All)
        );
    }

    #[test]
    fn an_included_partial_name_covers_the_fields_it_ends() {
        let include = lock("Include", &["street"]);
        assert_eq!(
            [
                include.covers(Some("address.street")),
                include.covers(Some("address.street.line")),
                include.covers(Some("address.streets")),
            ],
            [true, true, false]
        );
    }

    #[test]
    fn an_excluded_partial_name_frees_no_other_field() {
        assert!(lock("Exclude", &["street"]).covers(Some("address.street")));
    }

    #[test]
    fn an_included_name_covers_that_field_and_the_fields_below_it() {
        let include = lock("Include", &["address"]);
        assert_eq!(
            [
                include.covers(Some("address")),
                include.covers(Some("address.street")),
                include.covers(Some("addresses")),
                include.covers(None),
            ],
            [true, true, false, true]
        );
    }

    #[test]
    fn an_excluding_lock_covers_every_field_it_does_not_name() {
        let exclude = lock("Exclude", &["notes"]);
        assert_eq!(
            [
                exclude.covers(Some("notes")),
                exclude.covers(Some("total")),
                exclude.covers(None)
            ],
            [false, true, true]
        );
    }

    #[test]
    fn authoring_a_field_above_an_included_one_is_covered() {
        // Renaming or deleting `address` renames or deletes `address.street` too.
        let include = lock("Include", &["address.street"]);
        assert_eq!(
            [
                include.covers_subtree(Some("address")),
                include.covers_subtree(Some("address.street.line")),
                include.covers_subtree(Some("addressee")),
                include.covers_subtree(Some("other")),
            ],
            [true, true, false, false]
        );
    }

    #[test]
    fn an_excluding_lock_covers_a_new_field_it_does_not_name() {
        let exclude = lock("Exclude", &["notes"]);
        assert_eq!(
            [
                exclude.covers_subtree(Some("created")),
                exclude.covers_subtree(Some("notes.extra")),
                exclude.covers_subtree(None),
            ],
            [true, false, true]
        );
    }

    #[test]
    fn fields_are_related_when_one_is_the_other_or_below_it() {
        assert_eq!(
            [
                related(Some("Sig1"), Some("Sig1")),
                related(Some("Sig1.x"), Some("Sig1")),
                related(Some("form"), Some("form.Sig1")),
                related(Some("Sig10"), Some("Sig1")),
                related(None, Some("Sig1")),
            ],
            [true, true, true, false, true]
        );
    }

    #[test]
    fn a_lock_with_an_unknown_action_covers_every_field() {
        assert!(lock("Some", &["x"]).covers(Some("y")));
    }
}
