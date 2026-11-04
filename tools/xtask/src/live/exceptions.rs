//! The exceptions files (`fixtures/differential/exceptions.toml`, `full-exceptions.toml`):
//! `[[exception]]`, `[[class]]` and `[[known]]` entries, and which disagreement each excuses.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::{FACTS, Tool, text};

/// How an exception relates lamina's value to a tool's (`relation` in a `[[class]]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Relation {
    /// lamina skipped what it could not read: a lower page count, or outline titles that are
    /// a shorter subsequence of the tool's.
    LaminaPdfSkips,
    /// Page text that differs from the tool's only by lamina's U+FFFD for glyphs without a
    /// Unicode value, where the tool writes nothing or a C0 control character.
    Unmapped,
}

impl Relation {
    pub(super) fn holds(self, ours: &Value, theirs: &Value) -> bool {
        match self {
            Self::LaminaPdfSkips => match (ours, theirs) {
                (Value::Number(a), Value::Number(b)) => a.as_u64().zip(b.as_u64()).is_some_and(|(a, b)| a < b),
                (Value::Array(a), Value::Array(b)) => {
                    let mut rest = b.iter();
                    a.len() < b.len() && a.iter().all(|x| rest.any(|y| y == x))
                }
                _ => false,
            },
            Self::Unmapped => match (ours, theirs) {
                (Value::String(a), Value::String(b)) => {
                    let ours: String = a.chars().filter(|c| *c != '\u{FFFD}').collect();
                    let theirs: String = b
                        .chars()
                        .filter(|c| *c != '\u{FFFD}' && !c.is_ascii_control())
                        .collect();
                    ours.len() < a.len() && ours == theirs
                }
                _ => false,
            },
        }
    }
}

/// A `[[class]]` of deliberate differences: every disagreement on `facts` with `tools` (all
/// when empty) in a file matching `file` (a glob, `*` for any run of characters), where lamina's
/// value is `lamina` and the tool's `theirs` (either may be omitted), `relation` holds,
/// lamina's other facts (including its `error` code) match `lamina_facts`, and lamina raised
/// the warning `lamina_warning` (text only: the text facts list the file's warning codes).
#[derive(Debug)]
pub(super) struct Class {
    pub(super) file: String,
    pub(super) facts: Vec<String>,
    pub(super) tools: Vec<String>,
    pub(super) lamina: Option<Value>,
    pub(super) lamina_facts: BTreeMap<String, Value>,
    pub(super) theirs: Option<Value>,
    pub(super) relation: Option<Relation>,
    pub(super) lamina_warning: Option<String>,
    pub(super) rule: String,
}

impl Class {
    /// A `[[class]]` entry (its rule required).
    pub(super) fn read(entry: &toml::Value) -> Result<Self> {
        let e = Entry::read(entry, "class")?;
        e.require_rule()?;
        let relation = match entry.get("relation").and_then(toml::Value::as_str) {
            None => None,
            Some("lamina-skips") => Some(Relation::LaminaPdfSkips),
            Some("unmapped") => Some(Relation::Unmapped),
            Some(other) => bail!("[[class]] relation {other:?}: \"lamina-skips\" or \"unmapped\""),
        };
        let file = match e.files.as_slice() {
            [] => "*".to_owned(),
            [one] => one.clone(),
            _ => bail!("a [[class]] takes one `file` glob: {entry:?}"),
        };
        Ok(Self {
            file,
            facts: e.facts,
            tools: e.tools,
            lamina: entry.get("lamina").map(toml_to_json),
            lamina_facts: entry
                .get("lamina_facts")
                .and_then(toml::Value::as_table)
                .map(|t| t.iter().map(|(k, v)| (k.clone(), toml_to_json(v))).collect())
                .unwrap_or_default(),
            theirs: entry.get("theirs").map(toml_to_json),
            relation,
            lamina_warning: entry
                .get("lamina_warning")
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
            rule: e.rule,
        })
    }

    pub(super) fn matches(&self, file: &str, fact: &str, tool: &str, lamina: &Value, theirs: &Value) -> bool {
        let ours = lamina.get(fact).unwrap_or(&Value::Null);
        self.could_apply(file, fact, tool, lamina)
            && self.theirs.as_ref().is_none_or(|v| v == theirs)
            && self.relation.is_none_or(|r| r.holds(ours, theirs))
    }

    /// Whether everything on lamina's side holds (the file, the fact, the tool, lamina's
    /// values and warning), so the class could excuse a disagreement here: a class that could
    /// apply in a run and excused nothing is stale.
    pub(super) fn could_apply(&self, file: &str, fact: &str, tool: &str, lamina: &Value) -> bool {
        let ours = lamina.get(fact).unwrap_or(&Value::Null);
        glob(&self.file, file)
            && self.lamina_facts.iter().all(|(k, v)| lamina.get(k) == Some(v))
            && self.facts.iter().any(|f| f == fact)
            && (self.tools.is_empty() || self.tools.iter().any(|t| t == tool))
            && self.lamina.as_ref().is_none_or(|v| v == ours)
            && self.lamina_warning.as_ref().is_none_or(|code| {
                lamina
                    .get("warnings")
                    .and_then(Value::as_array)
                    .is_some_and(|w| w.iter().any(|w| w == code.as_str()))
            })
    }
}

/// A `[[known]]` disagreement: not deliberate, tracked by an issue until lamina is fixed (like
/// an ignored flaky test, ADR 0018 §4).
#[derive(Debug)]
pub(super) struct Known {
    pub(super) file: String,
    pub(super) facts: Vec<String>,
    pub(super) tools: Vec<String>,
    /// The tracking issue, or `None` while it is [`PENDING_ISSUE`] (reported as a warning).
    pub(super) issue: Option<String>,
}

/// The `issue` of a `[[known]]` entry whose tracking issue doesn't exist yet. Accepted, but every
/// run warns about it and the report lists it, so it can't pass for tracking.
pub(super) const PENDING_ISSUE: &str = "PENDING";

/// Which entry excused a disagreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Excuse {
    /// An `[[exception]]` for that file (both lanes).
    File,
    /// The `[[class]]` at this index.
    Class(usize),
    /// The `[[known]]` entry at this index.
    Known(usize),
}

/// `fixtures/differential/exceptions.toml` and `full-exceptions.toml`. `[[exception]]` entries
/// (shared with the PR lanes' test) name a `file` or `files`; without `tools` they are about
/// pypdf, the PR lanes' only tool.
/// `[[class]]` and `[[known]]` entries are read by the nightly lane only. Every `[[exception]]`
/// and `[[class]]` names the ADR 0003 rule behind lamina's answer; a `[[known]]` names its issue.
/// Page-text entries (`facts = ["text"]`) are split off into `text` when the file is read, so
/// the facts lane never sees them.
#[derive(Debug, Default)]
pub(crate) struct Exceptions {
    pub(super) files: BTreeSet<(String, String, String)>,
    pub(super) classes: Vec<Class>,
    pub(super) known: Vec<Known>,
    pub(super) text: text::TextEntries,
}

pub(super) fn toml_to_json(value: &toml::Value) -> Value {
    match value {
        toml::Value::String(s) => json!(s),
        toml::Value::Integer(n) => json!(n),
        toml::Value::Float(f) => json!(f),
        toml::Value::Boolean(b) => json!(b),
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_to_json).collect()),
        _ => Value::Null,
    }
}

/// `*` matches any run of characters (including `/`); everything else matches itself.
pub(super) fn glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(at) => rest = rest.get(at + part.len()..).unwrap_or_default(),
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

impl Exceptions {
    pub(super) fn read(path: &Path) -> Result<Self> {
        Self::parse(&fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?)
    }

    /// Adds `other`'s entries (class and known indexes continue after this file's).
    pub(super) fn merge(&mut self, other: Self) {
        self.files.extend(other.files);
        self.classes.extend(other.classes);
        self.known.extend(other.known);
        self.text.classes.extend(other.text.classes);
        self.text.known.extend(other.text.known);
    }

    /// `[[known]]` files whose tracking issue is still [`PENDING_ISSUE`].
    pub(super) fn pending(&self) -> Vec<&str> {
        self.known
            .iter()
            .filter(|k| k.issue.is_none())
            .map(|k| k.file.as_str())
            .collect()
    }

    pub(super) fn parse(text: &str) -> Result<Self> {
        let table: toml::Table = text.parse().context("parsing exceptions.toml")?;
        let entries = |key: &str| -> Vec<toml::Value> {
            table
                .get(key)
                .and_then(toml::Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let mut out = Self::default();
        for entry in entries("exception") {
            let e = Entry::read(&entry, "exception")?;
            e.require_rule()?;
            if e.files.is_empty() {
                bail!("every [[exception]] names its `file` or `files`: {entry:?}");
            }
            if e.facts.iter().any(|f| f == text::TEXT) {
                bail!(
                    "page text takes a [[class]] or [[known]] entry, not an [[exception]] (the PR lane reads those): {entry:?}"
                );
            }
            let tools = if e.tools.is_empty() {
                vec![Tool::Pypdf.name().to_owned()]
            } else {
                e.tools.clone()
            };
            for file in &e.files {
                for fact in &e.facts {
                    for tool in &tools {
                        out.files.insert((file.clone(), fact.clone(), tool.clone()));
                    }
                }
            }
        }
        for entry in entries("class") {
            let class = Class::read(&entry)?;
            if text::is_text(&class.facts) {
                out.text.classes.push(class);
            } else {
                out.classes.push(class);
            }
        }
        for entry in entries("known") {
            let e = Entry::read(&entry, "known")?;
            let issue = entry.get("issue").and_then(toml::Value::as_str).unwrap_or_default();
            let issue = match issue {
                PENDING_ISSUE => None,
                url if url.starts_with("https://") => Some(url.to_owned()),
                _ => bail!("a [[known]] `issue` is a URL or \"{PENDING_ISSUE}\": {entry:?}"),
            };
            if e.reason.is_empty() {
                bail!("every [[known]] gives a `reason`: {entry:?}");
            }
            if text::is_text(&e.facts) {
                out.text
                    .known
                    .extend(text::TextKnown::read(&entry, &e, issue.as_deref())?);
                continue;
            }
            if e.files.is_empty() || e.files.iter().any(|f| f.contains('*')) {
                bail!("a [[known]] names its `file` or `files` (no globs): {entry:?}");
            }
            // One entry per file, so each is reported (and goes stale) on its own.
            for file in e.files {
                out.known.push(Known {
                    file,
                    facts: e.facts.clone(),
                    tools: e.tools.clone(),
                    issue: issue.clone(),
                });
            }
        }
        Ok(out)
    }

    /// A short label for the entry behind `excuse`, for the report.
    pub(super) fn describe(&self, excuse: Excuse) -> String {
        match excuse {
            Excuse::File => "exception".to_owned(),
            Excuse::Class(i) => self
                .classes
                .get(i)
                .map_or_else(String::new, |c| format!("class {}: {}", i + 1, c.rule)),
            Excuse::Known(i) => self.known.get(i).map_or_else(String::new, |k| match &k.issue {
                Some(url) => format!("known: {url}"),
                None => "known: tracking issue pending".to_owned(),
            }),
        }
    }

    /// The entry that excuses lamina's `fact` (from all of lamina's facts about `file`) against
    /// `tool`'s value `theirs`, if any.
    pub(super) fn excuse(&self, file: &str, fact: &str, tool: &str, lamina: &Value, theirs: &Value) -> Option<Excuse> {
        if self
            .files
            .contains(&(file.to_owned(), fact.to_owned(), tool.to_owned()))
        {
            return Some(Excuse::File);
        }
        if let Some(i) = self
            .classes
            .iter()
            .position(|c| c.matches(file, fact, tool, lamina, theirs))
        {
            return Some(Excuse::Class(i));
        }
        self.known
            .iter()
            .position(|k| {
                k.file == file
                    && k.facts.iter().any(|f| f == fact)
                    && (k.tools.is_empty() || k.tools.iter().any(|t| t == tool))
            })
            .map(Excuse::Known)
    }
}

/// The fields every kind of entry shares.
pub(super) struct Entry {
    /// `file` or `files` (exactly one of them, or neither).
    pub(super) files: Vec<String>,
    pub(super) facts: Vec<String>,
    /// Validated tool names (`pdfjs`, `pdfium`, `qpdf`, `pypdf`; `pdfjs` and `pdftotext` for
    /// page text).
    pub(super) tools: Vec<String>,
    pub(super) rule: String,
    pub(super) reason: String,
}

impl Entry {
    pub(super) fn read(entry: &toml::Value, kind: &str) -> Result<Self> {
        let text = |key: &str| {
            entry
                .get(key)
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let strings = |key: &str| -> Result<Vec<String>> {
            let Some(value) = entry.get(key) else {
                return Ok(Vec::new());
            };
            let list = value
                .as_array()
                .with_context(|| format!("[[{kind}]] `{key}` must be an array: {entry:?}"))?;
            let out: Vec<String> = list.iter().filter_map(toml::Value::as_str).map(str::to_owned).collect();
            if out.is_empty() || out.len() != list.len() {
                bail!("[[{kind}]] `{key}` must be a non-empty list of strings: {entry:?}");
            }
            Ok(out)
        };
        let facts = strings("facts")?;
        let is_text = facts == [text::TEXT];
        if facts.is_empty() || !(is_text || facts.iter().all(|f| FACTS.contains(&f.as_str()))) {
            bail!(
                "[[{kind}]] needs `facts` from {FACTS:?}, or [\"{}\"] alone: {entry:?}",
                text::TEXT
            );
        }
        let tools = strings("tools")?;
        for tool in &tools {
            if is_text {
                if !text::TOOLS.contains(&tool.as_str()) {
                    bail!("[[{kind}]] text tools are {:?}: {entry:?}", text::TOOLS);
                }
            } else {
                Tool::parse(tool).with_context(|| format!("[[{kind}]] {entry:?}"))?;
            }
        }
        let files = match (entry.get("file"), entry.get("files")) {
            (None, None) => Vec::new(),
            (Some(file), None) => match file.as_str() {
                Some(f) if !f.is_empty() => vec![f.to_owned()],
                _ => bail!("[[{kind}]] `file` must be a non-empty string: {entry:?}"),
            },
            (None, Some(_)) => strings("files")?,
            (Some(_), Some(_)) => bail!("[[{kind}]] has both `file` and `files`: {entry:?}"),
        };
        Ok(Self {
            files,
            facts,
            tools,
            rule: text("rule"),
            reason: text("reason"),
        })
    }

    pub(super) fn require_rule(&self) -> Result<()> {
        if !self.rule.starts_with("ADR 0003") || self.reason.is_empty() {
            bail!(
                "every exception and class names its ADR 0003 `rule` and a `reason` ({:?})",
                self.files
            );
        }
        Ok(())
    }
}
