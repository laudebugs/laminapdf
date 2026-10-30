//! `docs/capabilities/manifest.toml`: one row per operation in the feature inventory
//! (`docs/research/pdf/06-feature-inventory.md` §3), and the check that the two agree.

use std::collections::BTreeSet;
use std::fmt;

use anyhow::{Context, Result, bail};

/// How far an operation works on a surface (full-editor §3), weakest first, so `max` is the
/// best proven status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Status {
    /// Detected and reported to the user, never silently dropped.
    Unsupported,
    /// Displayed or extracted faithfully.
    Read,
    /// Created or changed; the output passes qpdf `--check` and opens in pdf.js and `PDFium`.
    Write,
    /// Open → change → save → reopen preserves the feature and everything else.
    RoundTrip,
}

impl Status {
    pub(crate) const ALL: [Self; 4] = [Self::Unsupported, Self::Read, Self::Write, Self::RoundTrip];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Read => "read",
            Self::Write => "write",
            Self::RoundTrip => "round-trip",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == text)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A column of the matrix: the Rust core, `@lamina/pdf`, the web app, the desktop app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Surface {
    Core,
    Pdf,
    Web,
    Desktop,
}

impl Surface {
    pub(crate) const ALL: [Self; 4] = [Self::Core, Self::Pdf, Self::Web, Self::Desktop];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Pdf => "pdf",
            Self::Web => "web",
            Self::Desktop => "desktop",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == text)
    }
}

impl fmt::Display for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A feature group (06 §3's `### A. Text & typography`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Group {
    pub(crate) letter: char,
    pub(crate) name: String,
}

/// One operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    /// `<group>.<nn>`, stable: never renumbered or reused.
    pub(crate) id: String,
    /// The operation's name exactly as in 06 §3's first column.
    pub(crate) name: String,
    /// The editor round that targets it (`E6`).
    pub(crate) round: String,
    /// The status the round must reach on each targeted surface.
    pub(crate) target: Status,
    /// The columns the row targets; the others show `n/a`.
    pub(crate) surfaces: Vec<Surface>,
}

impl Row {
    pub(crate) fn group(&self) -> char {
        self.id.chars().next().unwrap_or('?')
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub(crate) groups: Vec<Group>,
    pub(crate) rows: Vec<Row>,
    /// IDs of removed operations, never to be used again.
    pub(crate) retired: BTreeSet<String>,
}

impl Manifest {
    pub(crate) fn row(&self, id: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.id == id)
    }

    /// Parses and validates the manifest: known groups, well-formed unique IDs, known statuses
    /// and surfaces, no retired ID in use.
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let table: toml::Table = toml::from_str(text).context("manifest.toml is not valid TOML")?;
        let version = table.get("version").and_then(toml::Value::as_integer);
        if version != Some(1) {
            bail!("manifest.toml: `version` must be 1");
        }
        let retired = table
            .get("retired")
            .and_then(toml::Value::as_array)
            .map(|ids| strings(ids, "retired"))
            .transpose()?
            .unwrap_or_default();
        let groups = tables(&table, "group")?
            .iter()
            .map(|g| {
                let id = string(g, "id", "group")?;
                let mut chars = id.chars();
                let (Some(letter), None) = (chars.next(), chars.next()) else {
                    bail!("group id `{id}` must be one capital letter");
                };
                if !letter.is_ascii_uppercase() {
                    bail!("group id `{id}` must be one capital letter");
                }
                Ok(Group {
                    letter,
                    name: string(g, "name", "group")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let rows = tables(&table, "op")?
            .iter()
            .map(|op| parse_row(op))
            .collect::<Result<Vec<_>>>()?;
        let manifest = Self { groups, rows, retired };
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<()> {
        let mut letters = BTreeSet::new();
        for group in &self.groups {
            if !letters.insert(group.letter) {
                bail!("group {} is listed twice", group.letter);
            }
        }
        let mut ids = BTreeSet::new();
        for row in &self.rows {
            if !is_id(&row.id) {
                bail!("op id `{}` is not `<group>.<nn>` (e.g. C.04)", row.id);
            }
            if !letters.contains(&row.group()) {
                bail!("op {} names an unknown group", row.id);
            }
            if !ids.insert(row.id.as_str()) {
                bail!("op {} is listed twice", row.id);
            }
            if self.retired.contains(&row.id) {
                bail!("op {} is retired: IDs are never reused, give it a new one", row.id);
            }
        }
        Ok(())
    }

    /// Problems with the manifest against 06 §3: every inventory operation must have exactly
    /// one row in its group, named exactly as in 06, and every row must be an inventory operation.
    pub(crate) fn inventory_problems(&self, inventory: &[(char, String)]) -> Vec<String> {
        let mut problems = Vec::new();
        for (group, name) in inventory {
            let rows: Vec<&str> = self
                .rows
                .iter()
                .filter(|r| r.group() == *group && r.name == *name)
                .map(|r| r.id.as_str())
                .collect();
            match rows.len() {
                1 => {}
                0 => problems.push(format!("06 §3 group {group} \"{name}\" has no manifest row")),
                _ => problems.push(format!(
                    "06 §3 group {group} \"{name}\" has {} manifest rows: {}",
                    rows.len(),
                    rows.join(", ")
                )),
            }
        }
        for row in &self.rows {
            if !inventory.iter().any(|(g, n)| *g == row.group() && *n == row.name) {
                problems.push(format!(
                    "manifest row {} \"{}\" is not an operation of 06 §3 group {} (renamed or removed? retire the ID)",
                    row.id,
                    row.name,
                    row.group()
                ));
            }
        }
        for group in &self.groups {
            if !inventory.iter().any(|(g, _)| *g == group.letter) {
                problems.push(format!("manifest group {} is not a group of 06 §3", group.letter));
            }
        }
        problems
    }
}

fn parse_row(op: &toml::Table) -> Result<Row> {
    let id = string(op, "id", "op")?;
    let target = match op.get("target") {
        None => Status::RoundTrip,
        Some(value) => {
            let text = value
                .as_str()
                .with_context(|| format!("op {id}: `target` must be a string"))?;
            Status::parse(text).with_context(|| format!("op {id}: unknown target status `{text}`"))?
        }
    };
    let surfaces = match op.get("surfaces") {
        None => Surface::ALL.to_vec(),
        Some(value) => {
            let names = value
                .as_array()
                .with_context(|| format!("op {id}: `surfaces` must be an array"))?;
            let mut surfaces = strings(names, "surfaces")?
                .iter()
                .map(|s| Surface::parse(s).with_context(|| format!("op {id}: unknown surface `{s}`")))
                .collect::<Result<Vec<_>>>()?;
            surfaces.sort();
            surfaces.dedup();
            if surfaces.is_empty() {
                bail!("op {id}: `surfaces` is empty");
            }
            surfaces
        }
    };
    for key in op.keys() {
        if !["id", "name", "round", "target", "surfaces"].contains(&key.as_str()) {
            bail!("op {id}: unknown key `{key}`");
        }
    }
    Ok(Row {
        name: string(op, "name", &id)?,
        round: string(op, "round", &id)?,
        id,
        target,
        surfaces,
    })
}

/// `<capital letter>.<two digits>`.
pub(crate) fn is_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    matches!(bytes, [g, b'.', a, b] if g.is_ascii_uppercase() && a.is_ascii_digit() && b.is_ascii_digit())
}

fn tables<'a>(table: &'a toml::Table, key: &str) -> Result<Vec<&'a toml::Table>> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .with_context(|| format!("manifest.toml has no [[{key}]] entries"))?
        .iter()
        .map(|v| {
            v.as_table()
                .with_context(|| format!("[[{key}]] entries must be tables"))
        })
        .collect()
}

fn string(table: &toml::Table, key: &str, what: &str) -> Result<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{what}: `{key}` must be a string"))
}

fn strings(values: &[toml::Value], what: &str) -> Result<BTreeSet<String>> {
    values
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .with_context(|| format!("`{what}` must hold strings"))
        })
        .collect()
}

/// The operations of 06 §3 as `(group letter, name)`, in order: the first cell of every table
/// row under each `### X. …` heading of section 3.
pub(crate) fn parse_inventory(text: &str) -> Result<Vec<(char, String)>> {
    let mut ops = Vec::new();
    let mut in_section = false;
    let mut group = None;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            in_section = heading.starts_with("3. ");
            group = None;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(heading) = line.strip_prefix("### ") {
            group = heading
                .split_once(". ")
                .and_then(|(letter, _)| letter.chars().next().filter(|_| letter.len() == 1));
            continue;
        }
        let (Some(letter), Some(row)) = (group, line.strip_prefix('|')) else {
            continue;
        };
        let first = split_cells(row).into_iter().next().unwrap_or_default();
        if first.is_empty() || first == "Operation" || first.starts_with("---") {
            continue;
        }
        ops.push((letter, first));
    }
    if ops.is_empty() {
        bail!("06-feature-inventory.md has no operations under `## 3.`");
    }
    Ok(ops)
}

/// The trimmed cells of a Markdown table row without its leading `|`, split on unescaped `|`.
pub(crate) fn split_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;
    for c in row.chars() {
        match c {
            '|' if !escaped => cells.push(std::mem::take(&mut cell).trim().to_owned()),
            _ => cell.push(c),
        }
        escaped = c == '\\' && !escaped;
    }
    let rest = cell.trim();
    if !rest.is_empty() {
        cells.push(rest.to_owned());
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
version = 1
retired = ["A.03"]

[[group]]
id = "A"
name = "Text"

[[op]]
id = "A.01"
name = "Add text"
round = "E6"

[[op]]
id = "A.02"
name = "Extract text"
round = "E3"
target = "read"
surfaces = ["pdf", "core"]
"#;

    #[test]
    fn parse_should_default_target_and_surfaces() {
        let manifest = Manifest::parse(MANIFEST).unwrap();
        let row = manifest.row("A.01").unwrap();
        assert_eq!(
            (row.target, row.surfaces.clone()),
            (Status::RoundTrip, Surface::ALL.to_vec())
        );
    }

    #[test]
    fn parse_should_sort_surfaces() {
        let manifest = Manifest::parse(MANIFEST).unwrap();
        assert_eq!(manifest.row("A.02").unwrap().surfaces, [Surface::Core, Surface::Pdf]);
    }

    #[test]
    fn parse_should_reject_a_duplicate_id() {
        let text = format!("{MANIFEST}\n[[op]]\nid = \"A.01\"\nname = \"x\"\nround = \"E1\"\n");
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn parse_should_reject_a_retired_id() {
        let text = format!("{MANIFEST}\n[[op]]\nid = \"A.03\"\nname = \"x\"\nround = \"E1\"\n");
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn parse_should_reject_an_unknown_group() {
        let text = format!("{MANIFEST}\n[[op]]\nid = \"Z.01\"\nname = \"x\"\nround = \"E1\"\n");
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn parse_should_reject_an_unknown_status() {
        let text = MANIFEST.replace("target = \"read\"", "target = \"edit\"");
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn is_id_should_accept_only_letter_dot_two_digits() {
        assert_eq!(
            ["C.04", "c.04", "C.4", "C.004", "CC.04"].map(is_id),
            [true, false, false, false, false]
        );
    }

    const INVENTORY: &str = "\
## 2. Other
| Operation | x |
| --- | --- |
| Not this | 1 |

## 3. Full operation inventory

### A. Text

| Operation | How | Diff. |
| --- | --- | --- |
| Add text | `BT … ET` \\| more | 2 |
| Extract text | interpreter | 2 |

## 4. Next
| Not this either | 1 |
";

    #[test]
    fn parse_inventory_should_read_only_section_3_first_cells() {
        assert_eq!(
            parse_inventory(INVENTORY).unwrap(),
            [('A', "Add text".to_owned()), ('A', "Extract text".to_owned())]
        );
    }

    #[test]
    fn inventory_problems_should_be_empty_when_rows_match() {
        let manifest = Manifest::parse(MANIFEST).unwrap();
        let inventory = parse_inventory(INVENTORY).unwrap();
        assert_eq!(manifest.inventory_problems(&inventory), Vec::<String>::new());
    }

    #[test]
    fn inventory_problems_should_report_a_missing_and_an_extra_row() {
        let manifest = Manifest::parse(&MANIFEST.replace("Extract text", "Extract all text")).unwrap();
        let inventory = parse_inventory(INVENTORY).unwrap();
        assert_eq!(manifest.inventory_problems(&inventory).len(), 2);
    }

    #[test]
    fn split_cells_should_keep_escaped_pipes() {
        assert_eq!(split_cells(" a \\| b | c |"), ["a \\| b", "c"]);
    }
}
