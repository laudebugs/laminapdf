//! Every screen in the Paper screen inventory has its design: part of `cargo xtask check docs`
//! (todo U6f, Paper design plan §5).
//!
//! `docs/design/paper/screens.md` lists the screens (`R05`, `O12`, …) in tables, one per area,
//! and ends with a canvases table saying which area document and which folder hold each range of
//! IDs (`R01–R09`, a whole letter such as `S`, or one ID such as `H12`, which wins over its
//! letter). For every screen the check wants:
//!
//! - a heading in its area document that names the ID (`## R05 Find in document`);
//! - a committed design file `<folder>/project/<ID>-*.dc.html`.
//!
//! The foundation screens F01–F05 are described in screens.md itself and drawn as the
//! foundations canvas's five named boards, so they need no heading. A screen listed under a
//! "Not yet drawn" heading in screens.md is reported, not failed. Each folder's `canvas.json`
//! must also name exactly the `.dc.html` files committed beside it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::tracked::git;

/// Where the inventory and its areas live, from the repository root.
const PAPER: &str = "docs/design/paper";

/// The foundation screens' boards in `foundations/project/`, which have no ID in their names.
const FOUNDATIONS: [(&str, &str); 5] = [
    ("F01", "Main"),
    ("F02", "Desktop"),
    ("F03", "Mobile-read"),
    ("F04", "Mobile-select"),
    ("F05", "Style"),
];

/// A screen ID: an upper-case letter and two digits.
fn is_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 3
        && bytes.first().is_some_and(u8::is_ascii_uppercase)
        && bytes.iter().skip(1).all(u8::is_ascii_digit)
}

/// The trimmed cells of a Markdown table row, or none for any other line.
fn cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

/// Which screens a canvases row covers.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Covers {
    /// One ID (`H12`): wins over a letter or range that also covers it.
    One(String),
    /// A range of one letter's IDs, inclusive (`R01–R09`).
    Range(char, u32, u32),
    /// Every ID of a letter (`S`).
    Letter(char),
}

impl Covers {
    fn parse(cell: &str) -> Option<Self> {
        if is_id(cell) {
            return Some(Self::One(cell.to_owned()));
        }
        if let Some((from, to)) = cell.split_once(['–', '-']) {
            let (from, to) = (from.trim(), to.trim());
            let letter = from.chars().next()?;
            if is_id(from) && is_id(to) && to.starts_with(letter) {
                return Some(Self::Range(
                    letter,
                    from.get(1..)?.parse().ok()?,
                    to.get(1..)?.parse().ok()?,
                ));
            }
            return None;
        }
        let mut chars = cell.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_uppercase() => Some(Self::Letter(c)),
            _ => None,
        }
    }

    /// How closely it covers `id`: 0 not at all, then letter, range, one ID.
    fn rank(&self, id: &str) -> u8 {
        let letter = id.chars().next();
        let number: Option<u32> = id.get(1..).and_then(|n| n.parse().ok());
        match self {
            Self::One(one) if one == id => 3,
            Self::Range(c, lo, hi) if letter == Some(*c) && number.is_some_and(|n| (*lo..=*hi).contains(&n)) => 2,
            Self::Letter(c) if letter == Some(*c) => 1,
            _ => 0,
        }
    }
}

/// One row of the canvases table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Canvas {
    covers: Covers,
    /// The area document, relative to the paper folder; none for "this page" (the foundations).
    document: Option<String>,
    /// The folder holding `project/canvas.json` and the boards.
    folder: String,
}

/// The target of the first Markdown link in `cell`, without its `#anchor`.
fn link_target(cell: &str) -> Option<&str> {
    let start = cell.find("](")? + 2;
    let rest = cell.get(start..)?;
    let target = rest.get(..rest.find(')')?)?;
    Some(target.split('#').next().unwrap_or(target))
}

/// What screens.md says: the screens, those not drawn yet, and the canvases.
#[derive(Debug, Default)]
struct Inventory {
    screens: BTreeSet<String>,
    not_drawn: BTreeSet<String>,
    canvases: Vec<Canvas>,
}

fn parse_inventory(markdown: &str) -> Inventory {
    let mut inventory = Inventory::default();
    let mut section = "";
    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.trim();
            continue;
        }
        let not_drawn = section.to_ascii_lowercase().contains("not yet drawn");
        if not_drawn {
            // Any ID that starts a list item or a table row.
            let item = line.trim_start().trim_start_matches(['-', '*', '|', ' ']);
            let first = item.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("");
            if is_id(first) {
                inventory.not_drawn.insert(first.to_owned());
            }
            continue;
        }
        let Some(row) = cells(line) else { continue };
        if section == "Canvases" {
            if let [screens, _area, document, _canvas, files] = row.as_slice()
                && let Some(covers) = Covers::parse(screens)
            {
                let folder = link_target(files)
                    .and_then(|t| t.split('/').next())
                    .unwrap_or("")
                    .to_owned();
                let document = link_target(document).map(str::to_owned);
                inventory.canvases.push(Canvas {
                    covers,
                    document,
                    folder,
                });
            }
        } else if let Some(id) = row.first().filter(|c| is_id(c)) {
            inventory.screens.insert((*id).to_owned());
        }
    }
    inventory
}

/// Whether `markdown` has a heading that names `id` as a word.
fn has_heading(markdown: &str, id: &str) -> bool {
    markdown
        .lines()
        .any(|line| line.starts_with('#') && line.split(|c: char| !c.is_ascii_alphanumeric()).any(|word| word == id))
}

/// The problems with the inventory, given each area document's text by path (relative to the
/// paper folder) and every tracked file under the paper folder (relative to it).
fn problems(
    screens_md: &str,
    documents: &BTreeMap<String, String>,
    files: &BTreeSet<String>,
) -> (Vec<String>, BTreeSet<String>) {
    let inventory = parse_inventory(screens_md);
    let mut found = Vec::new();
    for id in inventory.screens.iter().filter(|id| !inventory.not_drawn.contains(*id)) {
        let canvas = inventory
            .canvases
            .iter()
            .filter(|c| c.covers.rank(id) > 0)
            .max_by_key(|c| c.covers.rank(id));
        let Some(canvas) = canvas else {
            found.push(format!("{id}: no row of the canvases table covers it"));
            continue;
        };
        let project = format!("{}/project/", canvas.folder);
        match FOUNDATIONS.iter().find(|(f, _)| f == id) {
            Some((_, board)) if canvas.document.is_none() => {
                if !files.contains(&format!("{project}{board}.dc.html")) {
                    found.push(format!("{id}: no design file {project}{board}.dc.html"));
                }
            }
            _ => {
                match canvas.document.as_deref() {
                    None => found.push(format!("{id}: its canvases row names no area document")),
                    Some(doc) => match documents.get(doc) {
                        None => found.push(format!("{id}: its area document {doc} isn't committed")),
                        Some(text) if !has_heading(text, id) => {
                            found.push(format!("{id}: {doc} has no heading that names it"));
                        }
                        Some(_) => {}
                    },
                }
                let prefix = format!("{project}{id}-");
                if !files.iter().any(|f| f.starts_with(&prefix) && f.ends_with(".dc.html")) {
                    found.push(format!("{id}: no design file {prefix}*.dc.html"));
                }
            }
        }
    }
    (found, inventory.not_drawn)
}

/// The problems with one folder's `canvas.json` (`boards`, its parsed board names) against the
/// `.dc.html` files tracked in that folder.
fn board_problems(folder: &str, boards: &BTreeSet<String>, files: &BTreeSet<String>) -> Vec<String> {
    let project = format!("{folder}/project/");
    let drawn: BTreeSet<String> = files
        .iter()
        .filter_map(|f| f.strip_prefix(&project))
        .filter(|f| f.ends_with(".dc.html") && !f.contains('/'))
        .map(str::to_owned)
        .collect();
    let mut found = Vec::new();
    for board in boards.difference(&drawn) {
        found.push(format!("{project}canvas.json names {board}, which isn't committed"));
    }
    for file in drawn.difference(boards) {
        found.push(format!("{project}{file} is committed but not a board in canvas.json"));
    }
    found
}

/// Runs the check on the repository at `root`.
pub(crate) fn check(root: &Path) -> Result<()> {
    let paper = root.join(PAPER);
    let screens_md = fs::read_to_string(paper.join("screens.md")).context("reading the screen inventory")?;
    let prefix = format!("{PAPER}/");
    let files: BTreeSet<String> = git(root, &["ls-files", "-z", "--", PAPER])?
        .split('\0')
        .filter_map(|p| p.strip_prefix(&prefix))
        .map(str::to_owned)
        .collect();
    let mut documents = BTreeMap::new();
    for doc in files
        .iter()
        .filter(|f| !f.contains('/') && Path::new(f).extension().is_some_and(|e| e == "md"))
    {
        documents.insert(
            doc.clone(),
            fs::read_to_string(paper.join(doc)).with_context(|| format!("reading {doc}"))?,
        );
    }
    let (mut found, not_drawn) = problems(&screens_md, &documents, &files);
    let folders: BTreeSet<String> = parse_inventory(&screens_md)
        .canvases
        .into_iter()
        .map(|c| c.folder)
        .collect();
    for folder in &folders {
        let path = paper.join(folder).join("project/canvas.json");
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let boards = json
            .get("boards")
            .and_then(serde_json::Value::as_object)
            .map(|b| b.keys().cloned().collect())
            .unwrap_or_default();
        found.extend(board_problems(folder, &boards, &files));
    }
    if !not_drawn.is_empty() {
        let list: Vec<&str> = not_drawn.iter().map(String::as_str).collect();
        println!(
            "Not yet drawn (listed in screens.md, so not failed): {}",
            list.join(", ")
        );
    }
    if !found.is_empty() {
        let mut message = format!("{} screen(s) of {PAPER}/screens.md lack their design:", found.len());
        for problem in &found {
            let _ = write!(message, "\n  {problem}");
        }
        bail!(
            "{message}\nAdd the heading and the design file, or list the ID under a \"Not yet drawn\" heading in screens.md."
        );
    }
    println!(
        "{} screens, each with its heading and design file",
        parse_inventory(&screens_md).screens.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVENTORY: &str = "# Paper: screen inventory\n\n## F. Foundations\n\n| ID | Screen |\n| --- | --- |\n| F01 | Web reader |\n\n## R. Reader\n\n| ID | Screen |\n| --- | --- |\n| R01 | Home |\n| R12 | Shortcuts |\n\n## H. Account\n\n| ID | Screen |\n| --- | --- |\n| H02 | Plan |\n| H12 | Website home |\n\n## Canvases\n\n| Screens | Area | Document | Canvas | Files |\n| --- | --- | --- | --- | --- |\n| F | Foundations | this page | [canvas](https://x) | [`foundations/`](foundations/project/canvas.json) |\n| R01–R09 | Reader A | [reader-a.md](reader-a.md) | [canvas](https://x) | [`reader-a/`](reader-a/project/canvas.json) |\n| R10–R18 | Reader B | [reader-b.md](reader-b.md) | [canvas](https://x) | [`reader-b/`](reader-b/project/canvas.json) |\n| H | Account | [account.md](account.md) | [canvas](https://x) | [`account/`](account/project/canvas.json) |\n| H12 | Website home | [account.md](account.md#h12-website-home) | [canvas](https://x) | [`website-home/`](website-home/project/canvas.json) |\n";

    fn docs() -> BTreeMap<String, String> {
        [
            ("reader-a.md", "# Reader A\n\n## R01 Home / open\n"),
            ("reader-b.md", "# Reader B\n\n## R12 Keyboard shortcuts\n"),
            ("account.md", "# Account\n\n## H02 Plan\n\n## H12 Website home\n"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    }

    fn files() -> BTreeSet<String> {
        [
            "foundations/project/Main.dc.html",
            "reader-a/project/R01-home.dc.html",
            "reader-b/project/R12-shortcuts.dc.html",
            "account/project/H02-plan.dc.html",
            "website-home/project/H12-home.dc.html",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    #[test]
    fn problems_should_pass_when_every_screen_has_its_heading_and_file() {
        let (found, not_drawn) = problems(INVENTORY, &docs(), &files());
        assert_eq!(found, Vec::<String>::new());
        assert!(not_drawn.is_empty());
    }

    #[test]
    fn problems_should_report_a_missing_heading() {
        let mut docs = docs();
        docs.insert(
            "reader-b.md".to_owned(),
            "# Reader B\n\nR12 is mentioned only in prose.\n".to_owned(),
        );
        let (found, _) = problems(INVENTORY, &docs, &files());
        assert_eq!(found, ["R12: reader-b.md has no heading that names it"]);
    }

    #[test]
    fn problems_should_report_a_missing_design_file() {
        let mut files = files();
        files.remove("reader-a/project/R01-home.dc.html");
        files.remove("foundations/project/Main.dc.html");
        let (found, _) = problems(INVENTORY, &docs(), &files);
        assert_eq!(
            found,
            [
                "F01: no design file foundations/project/Main.dc.html",
                "R01: no design file reader-a/project/R01-*.dc.html"
            ]
        );
    }

    #[test]
    fn problems_should_send_an_id_to_its_own_row_before_its_letter() {
        let mut files = files();
        files.remove("website-home/project/H12-home.dc.html");
        files.insert("account/project/H12-home.dc.html".to_owned());
        let (found, _) = problems(INVENTORY, &docs(), &files);
        assert_eq!(found, ["H12: no design file website-home/project/H12-*.dc.html"]);
    }

    #[test]
    fn problems_should_report_not_yet_drawn_screens_instead_of_failing() {
        let inventory = format!("{INVENTORY}\n## Not yet drawn\n\n- R12: waits for the command registry\n");
        let mut files = files();
        files.remove("reader-b/project/R12-shortcuts.dc.html");
        let (found, not_drawn) = problems(&inventory, &docs(), &files);
        assert_eq!(found, Vec::<String>::new());
        assert_eq!(not_drawn.into_iter().collect::<Vec<_>>(), ["R12"]);
    }

    #[test]
    fn problems_should_report_a_screen_no_canvas_row_covers() {
        let inventory = INVENTORY.replace("| R12 | Shortcuts |", "| R12 | Shortcuts |\n| Z01 | Unknown |");
        let (found, _) = problems(&inventory, &docs(), &files());
        assert_eq!(found, ["Z01: no row of the canvases table covers it"]);
    }

    #[test]
    fn board_problems_should_compare_canvas_json_with_the_committed_files() {
        let boards: BTreeSet<String> = ["R01-home.dc.html", "R02-gone.dc.html"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut files = files();
        files.insert("reader-a/project/R01-extra.dc.html".to_owned());
        assert_eq!(
            board_problems("reader-a", &boards, &files),
            [
                "reader-a/project/canvas.json names R02-gone.dc.html, which isn't committed",
                "reader-a/project/R01-extra.dc.html is committed but not a board in canvas.json"
            ]
        );
    }

    #[test]
    fn covers_should_parse_letters_ranges_and_single_ids() {
        assert_eq!(Covers::parse("S"), Some(Covers::Letter('S')));
        assert_eq!(Covers::parse("O09–O17"), Some(Covers::Range('O', 9, 17)));
        assert_eq!(Covers::parse("H12"), Some(Covers::One("H12".to_owned())));
        assert_eq!(Covers::parse("Screens"), None);
    }
}
