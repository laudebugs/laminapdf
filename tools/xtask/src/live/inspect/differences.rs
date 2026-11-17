//! What differs between lamina's objects or fonts and the tools', one object or font at a time,
//! and the `[[class]]` entries of `inspect-exceptions.toml` that excuse a difference by its
//! values rather than by its file.
//!
//! A difference is an object or font only lamina lists, one only the tool lists, or one both
//! list whose fields differ (objects by number; fonts by object, a direct font by family and
//! name). A `[[class]]` names the `relation` (`lamina-only`, `tool-only` or `differ`), for
//! `differ` the `fields` that may differ, and what each side's record must hold: `lamina` and
//! `theirs` (inline tables, a value or an array of the values allowed), `lamina_null` and
//! `theirs_null` (fields that are null in that side's record), and `lamina_file` (an inline
//! table of lamina's facts about the whole file, an array fact holding the value). lamina's font
//! records also have `appearance`: lamina reaches the font only through annotations'
//! appearance streams.

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

use crate::live::exceptions::glob;

/// How lamina's record relates to the tool's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Relation {
    LaminaOnly,
    ToolOnly,
    Differ,
}

impl Relation {
    fn parse(text: &str) -> Result<Self> {
        Ok(match text {
            "lamina-only" => Self::LaminaOnly,
            "tool-only" => Self::ToolOnly,
            "differ" => Self::Differ,
            other => bail!("[[class]] relation {other:?}: \"lamina-only\", \"tool-only\" or \"differ\""),
        })
    }
}

/// One object or font the two read differently.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Difference {
    pub(crate) relation: Relation,
    /// The fields that differ (`differ` only).
    pub(crate) fields: Vec<String>,
    pub(crate) lamina: Option<Value>,
    pub(crate) theirs: Option<Value>,
    /// How the report names it (`object 4: kind`, `lamina: Helvetica (5 0)`).
    pub(crate) label: String,
}

impl Difference {
    fn new(lamina: Option<&Value>, theirs: Option<&Value>, label: String) -> Self {
        let fields = match (lamina, theirs) {
            (Some(Value::Object(a)), Some(Value::Object(b))) => {
                let mut keys: Vec<String> = a.keys().chain(b.keys()).cloned().collect();
                keys.sort();
                keys.dedup();
                keys.retain(|k| a.get(k) != b.get(k));
                keys
            }
            _ => Vec::new(),
        };
        let relation = match (lamina, theirs) {
            (Some(_), None) => Relation::LaminaOnly,
            (None, Some(_)) => Relation::ToolOnly,
            _ => Relation::Differ,
        };
        Self {
            relation,
            fields,
            lamina: lamina.cloned(),
            theirs: theirs.cloned(),
            label,
        }
    }
}

/// What differs between two objects maps (by number).
pub(crate) fn objects(ours: &Value, theirs: &Value) -> Vec<Difference> {
    let empty = Map::new();
    let (ours, theirs) = (ours.as_object().unwrap_or(&empty), theirs.as_object().unwrap_or(&empty));
    let mut out = Vec::new();
    for num in ours.keys().chain(theirs.keys().filter(|k| !ours.contains_key(*k))) {
        let (a, b) = (ours.get(num), theirs.get(num));
        if a == b {
            continue;
        }
        let mut d = Difference::new(a, b, String::new());
        d.label = match d.relation {
            Relation::LaminaOnly => format!("object {num}: only lamina lists it"),
            Relation::ToolOnly => format!("object {num}: only qpdf lists it"),
            Relation::Differ => format!("object {num}: {}", d.fields.join(", ")),
        };
        out.push(d);
    }
    out
}

/// How a font is paired with the other side's: its object, else its family and name.
fn font_key(font: &Value) -> (Option<&Value>, Option<&Value>, Option<&Value>) {
    match font.get("object").filter(|o| !o.is_null()) {
        Some(object) => (Some(object), None, None),
        None => (None, font.get("family"), font.get("name")),
    }
}

/// What differs between two font lists; `appearance` lists the objects of the fonts lamina
/// reaches only through annotations' appearance streams.
pub(crate) fn fonts(ours: &Value, theirs: &Value, appearance: &[Value]) -> Vec<Difference> {
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    let ours: Vec<Value> = list(ours)
        .into_iter()
        .map(|mut f| {
            let reached = f.get("object").is_some_and(|o| appearance.contains(o));
            if let Some(f) = f.as_object_mut() {
                f.insert("appearance".to_owned(), Value::Bool(reached));
            }
            f
        })
        .collect();
    let theirs = list(theirs);
    let without_appearance = |f: &Value| {
        let mut f = f.clone();
        if let Some(f) = f.as_object_mut() {
            f.remove("appearance");
        }
        f
    };
    let label = |f: &Value| {
        format!(
            "{} ({})",
            f.get("name").and_then(Value::as_str).unwrap_or("[none]"),
            f.get("object").and_then(Value::as_str).unwrap_or("direct")
        )
    };
    let mut out = Vec::new();
    let mut paired = vec![false; theirs.len()];
    for a in &ours {
        let found = theirs
            .iter()
            .enumerate()
            .find(|(i, b)| !paired.get(*i).copied().unwrap_or(true) && font_key(b) == font_key(a));
        match found {
            Some((i, b)) => {
                if let Some(slot) = paired.get_mut(i) {
                    *slot = true;
                }
                if &without_appearance(a) != b {
                    let mut d = Difference::new(Some(a), Some(b), String::new());
                    d.fields.retain(|f| f != "appearance");
                    d.label = format!("{}: {}", label(a), d.fields.join(", "));
                    out.push(d);
                }
            }
            None => out.push(Difference::new(Some(a), None, format!("lamina: {}", label(a)))),
        }
    }
    for (b, _) in theirs.iter().zip(&paired).filter(|(_, p)| !**p) {
        out.push(Difference::new(None, Some(b), format!("pdffonts: {}", label(b))));
    }
    out
}

/// A `[[class]]` of deliberate differences.
#[derive(Debug)]
pub(crate) struct Class {
    file: String,
    fact: String,
    relation: Relation,
    fields: Vec<String>,
    lamina: toml::Table,
    theirs: toml::Table,
    lamina_null: Vec<String>,
    theirs_null: Vec<String>,
    lamina_file: toml::Table,
    pub(crate) rule: String,
}

/// The strings of the array at `key` (none when absent).
fn strings(entry: &toml::Table, key: &str) -> Result<Vec<String>> {
    let Some(value) = entry.get(key) else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .with_context(|| format!("[[class]] `{key}` must be an array"))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .with_context(|| format!("[[class]] `{key}` holds strings"))
        })
        .collect()
}

impl Class {
    /// One `[[class]]` entry; `facts` are the facts it may name.
    pub(crate) fn read(entry: &toml::Value, facts: &[&str]) -> Result<Self> {
        let entry = entry.as_table().context("a [[class]] entry must be a table")?;
        let keys = [
            "file",
            "facts",
            "relation",
            "fields",
            "lamina",
            "theirs",
            "lamina_null",
            "theirs_null",
            "lamina_file",
            "rule",
            "reason",
        ];
        if let Some(other) = entry.keys().find(|k| !keys.contains(&k.as_str())) {
            bail!("[[class]]: unknown key {other:?}");
        }
        let text = |key: &str| {
            entry
                .get(key)
                .and_then(toml::Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
                .with_context(|| format!("[[class]] without `{key}`: {entry:?}"))
        };
        let [fact] = strings(entry, "facts")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("a [[class]] names one of `facts`: {entry:?}"))?;
        if !facts.contains(&fact.as_str()) {
            bail!("[[class]]: unknown fact {fact:?} (one of {})", facts.join(", "));
        }
        let relation = Relation::parse(&text("relation")?)?;
        let fields = strings(entry, "fields")?;
        if (relation == Relation::Differ) == fields.is_empty() {
            bail!("a [[class]] lists the `fields` that may differ exactly when its relation is \"differ\": {entry:?}");
        }
        let table = |key: &str| -> Result<toml::Table> {
            match entry.get(key) {
                None => Ok(toml::Table::new()),
                Some(v) => v
                    .as_table()
                    .cloned()
                    .with_context(|| format!("[[class]] `{key}` must be an inline table")),
            }
        };
        let lamina = table("lamina")?;
        let theirs = table("theirs")?;
        let lamina_null = strings(entry, "lamina_null")?;
        let theirs_null = strings(entry, "theirs_null")?;
        let no_lamina = lamina.is_empty() && lamina_null.is_empty();
        let no_theirs = theirs.is_empty() && theirs_null.is_empty();
        if relation == Relation::LaminaOnly && !no_theirs || relation == Relation::ToolOnly && !no_lamina {
            bail!("[[class]]: a side the relation leaves out has no values to match: {entry:?}");
        }
        text("reason")?;
        Ok(Self {
            file: entry
                .get("file")
                .and_then(toml::Value::as_str)
                .unwrap_or("*")
                .to_owned(),
            fact,
            relation,
            fields,
            lamina,
            theirs,
            lamina_null,
            theirs_null,
            lamina_file: table("lamina_file")?,
            rule: text("rule")?,
        })
    }

    /// Whether it excuses `difference` of `fact` in `file`, whose lamina facts are `facts`.
    pub(crate) fn excuses(&self, file: &str, fact: &str, difference: &Difference, facts: &Value) -> bool {
        glob(&self.file, file)
            && self.fact == fact
            && self.relation == difference.relation
            && difference.fields.iter().all(|f| self.fields.contains(f))
            && holds(&self.lamina, difference.lamina.as_ref())
            && holds(&self.theirs, difference.theirs.as_ref())
            && nulls(&self.lamina_null, difference.lamina.as_ref())
            && nulls(&self.theirs_null, difference.theirs.as_ref())
            && self.lamina_file.iter().all(|(key, want)| match facts.get(key) {
                Some(Value::Array(items)) => items.iter().any(|item| same(want, Some(item))),
                have => same(want, have),
            })
    }
}

/// Whether every field of `keys` is null (or absent) in `record`, which must be there when
/// `keys` names any.
fn nulls(keys: &[String], record: Option<&Value>) -> bool {
    match record {
        Some(record) => keys.iter().all(|k| record.get(k).is_none_or(Value::is_null)),
        None => keys.is_empty(),
    }
}

/// Whether `record` has every value `wanted` names (an array: one of them).
fn holds(wanted: &toml::Table, record: Option<&Value>) -> bool {
    wanted.iter().all(|(key, want)| {
        let have = record.and_then(|r| r.get(key));
        match want {
            toml::Value::Array(options) => options.iter().any(|o| same(o, have)),
            one => same(one, have),
        }
    })
}

/// Whether a TOML value equals a JSON one.
fn same(want: &toml::Value, have: Option<&Value>) -> bool {
    match (want, have) {
        (toml::Value::String(a), Some(Value::String(b))) => a == b,
        (toml::Value::Boolean(a), Some(Value::Bool(b))) => a == b,
        (toml::Value::Integer(a), Some(Value::Number(b))) => b.as_i64() == Some(*a),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn class(text: &str) -> Class {
        let value: toml::Value = toml::from_str(text).unwrap();
        Class::read(&value, &["objects", "fonts"]).unwrap()
    }

    #[test]
    fn a_font_both_list_should_differ_by_its_fields() {
        let ours = json!([{"object": "4 0", "family": "TrueType", "encoding": "Builtin"}]);
        let theirs = json!([{"object": "4 0", "family": "TrueType", "encoding": "WinAnsi"}]);
        let d = fonts(&ours, &theirs, &[]);
        assert_eq!(
            d.iter().map(|d| (d.relation, d.fields.clone())).collect::<Vec<_>>(),
            [(Relation::Differ, vec!["encoding".to_owned()])]
        );
    }

    #[test]
    fn a_class_should_excuse_only_the_values_it_names() {
        let c = class(
            "facts = [\"fonts\"]\nrelation = \"differ\"\nfields = [\"encoding\"]\n\
             lamina = { family = \"TrueType\", encoding = \"Builtin\" }\ntheirs = { encoding = [\"WinAnsi\", \"Standard\"] }\n\
             rule = \"r\"\nreason = \"r\"\n",
        );
        let font = |encoding: &str| json!([{"object": "4 0", "family": "TrueType", "encoding": encoding}]);
        let excused = |theirs: &str| {
            fonts(&font("Builtin"), &font(theirs), &[])
                .iter()
                .all(|d| c.excuses("a.pdf", "fonts", d, &json!({})))
        };
        assert_eq!((excused("WinAnsi"), excused("MacRoman")), (true, false));
    }

    #[test]
    fn appearance_should_mark_the_fonts_lamina_reaches_through_appearances() {
        let c = class(
            "facts = [\"fonts\"]\nrelation = \"lamina-only\"\nlamina = { appearance = true }\nrule = \"r\"\nreason = \"r\"\n",
        );
        let ours = json!([{"object": "5 0", "family": "Type1"}, {"object": "6 0", "family": "Type1"}]);
        let d = fonts(&ours, &json!([]), &[json!("5 0")]);
        let excused: Vec<bool> = d.iter().map(|d| c.excuses("a.pdf", "fonts", d, &json!({}))).collect();
        assert_eq!(excused, [true, false]);
    }

    #[test]
    fn null_should_match_only_null_fields() {
        let c = class(
            "facts = [\"objects\"]\nrelation = \"tool-only\"\ntheirs = { kind = \"other\" }\ntheirs_null = [\"at\"]\n\
             rule = \"r\"\nreason = \"r\"\n",
        );
        let theirs = json!({"7": {"kind": "other", "at": null}, "8": {"kind": "other", "at": "offset:9"}});
        let excused: Vec<bool> = objects(&json!({}), &theirs)
            .iter()
            .map(|d| c.excuses("a.pdf", "objects", d, &json!({})))
            .collect();
        assert_eq!(excused, [true, false]);
    }

    #[test]
    fn lamina_file_should_match_a_fact_of_the_whole_file() {
        let c = class(
            "facts = [\"fonts\"]\nrelation = \"tool-only\"\nlamina_file = { annotations_without_appearance = \"FreeText\" }\n\
             rule = \"r\"\nreason = \"r\"\n",
        );
        let d = fonts(&json!([]), &json!([{"object": null, "family": "Type1"}]), &[]);
        let excused = |facts: Value| d.iter().all(|d| c.excuses("a.pdf", "fonts", d, &facts));
        assert_eq!(
            (
                excused(json!({"annotations_without_appearance": ["Text", "FreeText"]})),
                excused(json!({"annotations_without_appearance": ["Text"]}))
            ),
            (true, false)
        );
    }

    #[test]
    fn a_class_should_refuse_fields_without_differ() {
        let value: toml::Value = toml::from_str(
            "facts = [\"fonts\"]\nrelation = \"lamina-only\"\nfields = [\"name\"]\nrule = \"r\"\nreason = \"r\"\n",
        )
        .unwrap();
        assert!(Class::read(&value, &["fonts"]).is_err());
    }
}
