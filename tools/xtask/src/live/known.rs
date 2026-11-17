//! The `[[known]]` entries the field-by-field differentials share (the XMP one's
//! `fixtures/differential/xmp-exceptions.toml`, the inspection one's `inspect-exceptions.toml`):
//! a `file` glob, the aspects it excuses (the XMP fields, the inspection facts; none listed
//! means every one), an `issue` and a `reason`.

use anyhow::{Context, Result, bail};

use super::PENDING_ISSUE;
use super::exceptions::glob;

/// A `[[known]]` entry.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Known {
    file: String,
    /// The aspects it excuses; empty: every one.
    aspects: Vec<String>,
    issue: String,
}

impl Known {
    /// Whether it excuses a mismatch of `aspect` in `file`.
    pub(super) fn excuses(&self, file: &str, aspect: &str) -> bool {
        glob(&self.file, file) && (self.aspects.is_empty() || self.aspects.iter().any(|a| a == aspect))
    }

    /// How a report names it: `file [aspects] (issue)`.
    pub(super) fn label(&self) -> String {
        format!("{} [{}] ({})", self.file, self.aspects.join(", "), self.issue)
    }
}

/// Warns, in the workflow's annotations, of each entry in `file` with no tracking issue yet.
pub(super) fn warn_pending(known: &[Known], file: &str) {
    for k in known.iter().filter(|k| k.issue == PENDING_ISSUE) {
        println!("::warning::{file}: [[known]] {} has no tracking issue yet", k.label());
    }
}

/// The `[[known]]` entries of an exceptions file's text, whose aspects are listed under
/// `list_key` (`fields`, `facts`) and must be among `allowed`; anything else is an error.
pub(super) fn parse_known(text: &str, list_key: &str, allowed: &[&str]) -> Result<Vec<Known>> {
    let table: toml::Table = toml::from_str(text)?;
    if let Some(other) = table.keys().find(|k| *k != "known") {
        bail!("unknown table {other:?}: only [[known]] entries");
    }
    let Some(entries) = table.get("known") else {
        return Ok(Vec::new());
    };
    let entries = entries.as_array().context("`known` must be [[known]] entries")?;
    entries.iter().map(|entry| known(entry, list_key, allowed)).collect()
}

/// One `[[known]]` entry.
fn known(entry: &toml::Value, list_key: &str, allowed: &[&str]) -> Result<Known> {
    let entry = entry.as_table().context("a [[known]] entry must be a table")?;
    if let Some(other) = entry
        .keys()
        .find(|k| !["file", list_key, "issue", "reason"].contains(&k.as_str()))
    {
        bail!("[[known]]: unknown key {other:?}");
    }
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(toml::Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
            .with_context(|| format!("[[known]] without `{key}`: {entry:?}"))
    };
    let aspects: Vec<String> = match entry.get(list_key) {
        None => Vec::new(),
        Some(v) => v
            .as_array()
            .with_context(|| format!("`{list_key}` must be an array"))?
            .iter()
            .map(|f| {
                f.as_str()
                    .map(str::to_owned)
                    .with_context(|| format!("`{list_key}` holds strings"))
            })
            .collect::<Result<_>>()?,
    };
    if let Some(bad) = aspects.iter().find(|a| !allowed.contains(&a.as_str())) {
        bail!(
            "[[known]]: unknown {list_key} entry {bad:?} (one of {})",
            allowed.join(", ")
        );
    }
    text("reason")?;
    Ok(Known {
        file: text("file")?,
        aspects,
        issue: text("issue")?,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_known;

    #[test]
    fn an_entry_excuses_its_aspects_in_the_files_its_glob_matches() {
        let known = parse_known(
            "[[known]]\nfile = \"a*\"\nfacts = [\"fonts\"]\nissue = \"#1\"\nreason = \"r\"\n",
            "facts",
            &["objects", "fonts"],
        )
        .unwrap();
        assert!(known[0].excuses("abc.pdf", "fonts"));
        assert!(!known[0].excuses("abc.pdf", "objects"));
        assert!(!known[0].excuses("b.pdf", "fonts"));
    }

    #[test]
    fn an_unknown_aspect_or_key_is_refused() {
        assert!(
            parse_known(
                "[[known]]\nfile = \"a\"\nfacts = [\"font\"]\nissue = \"x\"\nreason = \"r\"\n",
                "facts",
                &["fonts"]
            )
            .is_err()
        );
        assert!(
            parse_known(
                "[[known]]\nfile = \"a\"\nfields = []\nissue = \"x\"\nreason = \"r\"\n",
                "facts",
                &["fonts"]
            )
            .is_err()
        );
    }
}
