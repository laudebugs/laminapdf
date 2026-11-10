//! `cargo xtask progress [--offline]`: writes `apps/site/src/data/progress.json`, the snapshot the
//! website's home page reads for its roadmap and dashboard (website plan §4).
//!
//! The snapshot comes from four sources:
//! - the todo files on `main`, through [`todos`]'s own parser and checks, so its counts and "Can
//!   start now" list are the board's;
//! - the roadmap's Milestones table on `main`, each milestone's state worked out from the todos its
//!   Rounds cell names ([`milestone_state`]);
//! - `git log` on `main`: the merged pull requests, squash merges (subjects ending in `(#<n>)`) and
//!   merge commits alike;
//! - the open pull requests (`gh api`). A pull request claims a todo when it adds that todo's file
//!   under `ongoing/` or `done/`, which only its branch holds, so `main` alone can't say what is in
//!   progress. With `--offline`, or when gh fails, the last snapshot's list stays and the file
//!   says so (`inProgress.source`).
//!
//! It reads the todo files and the roadmap from `main` (`origin/main` when there is one), not from
//! the checkout, so a branch's own todo moves never reach the snapshot. The file names no person
//! and holds no pull request number or link into the repository, which is private.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::changes::{at_revision, default_base};
use crate::todos::{self, SECTIONS, State, Todo, TodoFile, id_parts, order_key};
use crate::tracked::git;
use crate::util::gh;

/// The snapshot, relative to the repository root.
pub(crate) const OUTPUT: &str = "apps/site/src/data/progress.json";
/// The roadmap whose Milestones table the snapshot reads.
const ROADMAP: &str = "docs/plans/roadmap.md";
/// The file format's version; a change the site's reader can't handle bumps it.
const VERSION: u32 = 1;
/// How many merged pull requests the snapshot lists.
const RECENT: usize = 10;

/// `apps/site/src/data/progress.json`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    version: u32,
    /// The day the snapshot was written, `YYYY-MM-DD` (UTC).
    date: String,
    totals: Counts,
    /// One entry per section of the board, in the board's order.
    rounds: Vec<Round>,
    milestones: Vec<Milestone>,
    in_progress: InProgress,
    /// The board's "Can start now", in board order, without the todos already in progress.
    next_up: Vec<TodoRef>,
    merged: Merged,
}

/// Todo counts. A todo is in progress when it is in `ongoing/` or an open pull request claims it;
/// `ready` counts the other ready todos.
#[derive(Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Counts {
    todos: usize,
    done: usize,
    in_progress: usize,
    ready: usize,
}

/// One section of the board: its ID letter, the board's heading and its counts.
#[derive(Debug, Serialize)]
struct Round {
    letter: char,
    heading: &'static str,
    #[serde(flatten)]
    counts: Counts,
}

/// One row of the roadmap's Milestones table.
#[derive(Debug, Serialize)]
struct Milestone {
    /// `M0`.
    id: String,
    /// `Foundations`.
    name: String,
    /// The Rounds cell as written (`B9, C1–C7, U1`).
    rounds: String,
    /// The "What ships" cell as written, Markdown code spans included.
    ships: String,
    state: MilestoneState,
}

/// Done when every todo of a milestone's rounds is done; under way when any is done or in
/// progress; not started otherwise.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum MilestoneState {
    Done,
    UnderWay,
    NotStarted,
}

/// The todos in progress, and where the list of claims came from.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InProgress {
    source: ClaimSource,
    /// The day the open pull requests were read; `None` when they never were.
    checked: Option<String>,
    todos: Vec<TodoRef>,
}

/// Where the claims of the in-progress list came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ClaimSource {
    /// The open pull requests, read on `checked`.
    PullRequests,
    /// The last snapshot's list, because the pull requests couldn't be read this time.
    PreviousSnapshot,
    /// Neither: only the todos in `ongoing/`.
    None,
}

/// A todo by ID and title, as the board shows it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TodoRef {
    id: String,
    title: String,
}

/// The pull requests merged into `main`: how many, and the latest squash-merge titles without
/// their numbers (a title that still names a pull request is left out).
#[derive(Debug, Serialize)]
struct Merged {
    count: usize,
    recent: Vec<String>,
}

/// What [`snapshot`] reads; [`run`] gathers it, the tests give fixtures.
pub(crate) struct Inputs<'a> {
    pub(crate) files: &'a [TodoFile],
    pub(crate) roadmap: &'a str,
    /// `git log --format=%s` on `main`, newest first.
    pub(crate) log: &'a str,
    /// The `docs/todos/` paths the open pull requests add; `None` when they weren't read.
    pub(crate) claims: Option<&'a [String]>,
    /// The last snapshot's in-progress list, if there is a last snapshot.
    pub(crate) previous: Option<InProgress>,
    /// Today, `YYYY-MM-DD`.
    pub(crate) date: &'a str,
}

/// Whether `args` ask for `--offline`, for a command whose only flag it is (`cargo xtask progress`
/// and `cargo xtask proof snapshot`, named by `command`); fails on anything else.
pub(crate) fn offline(args: &[String], command: &str) -> Result<bool> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--offline" => Ok(true),
        _ => bail!("usage: cargo xtask {command} [--offline]"),
    }
}

/// Writes the snapshot to [`OUTPUT`]. `offline` leaves out the open pull requests.
pub(crate) fn run(root: &Path, offline: bool) -> Result<()> {
    let output = root.join(OUTPUT);
    let previous = fs::read_to_string(&output)
        .ok()
        .and_then(|text| serde_json::from_str::<Previous>(&text).ok())
        .map(|previous| previous.in_progress);
    let claims = if offline {
        None
    } else {
        open_pull_request_paths(root)
            .inspect_err(|error| {
                // The website's deploy runs this: say so where the run shows it, since the job passes.
                if std::env::var_os("GITHUB_ACTIONS").is_some() {
                    println!("::warning title=progress::couldn't read the open pull requests, so in progress is the last snapshot's list");
                }
                eprintln!("warning: couldn't read the open pull requests: {error:#}");
            })
            .ok()
    };
    if claims.is_none() {
        match &previous {
            Some(previous) if previous.source != ClaimSource::None => {
                eprintln!(
                    "warning: in progress: kept the last snapshot's list (inProgress.source says so in the file)"
                );
            }
            _ => eprintln!("warning: in progress: no pull requests read and no earlier list, so only ongoing/ shows"),
        }
    }
    let base = default_base(root);
    println!("progress: reading the board, the roadmap and the merges from {base}");
    let files = todo_files_at(root, base)?;
    let roadmap = at_revision(root, base, ROADMAP).with_context(|| format!("reading {ROADMAP} on {base}"))?;
    let log = git(root, &["log", "--first-parent", "--format=%s", base])?;
    let snapshot = snapshot(&Inputs {
        files: &files,
        roadmap: &roadmap,
        log: &log,
        claims: claims.as_deref(),
        previous,
        date: &today(),
    })?;
    let mut json = serde_json::to_string_pretty(&snapshot).context("serialising the snapshot")?;
    json.push('\n');
    if let Some(folder) = output.parent() {
        fs::create_dir_all(folder).with_context(|| format!("creating {}", folder.display()))?;
    }
    fs::write(&output, json).with_context(|| format!("writing {OUTPUT}"))?;
    println!(
        "progress: wrote {OUTPUT}: {} of {} todos done, {} in progress, {} merged pull requests",
        snapshot.totals.done, snapshot.totals.todos, snapshot.totals.in_progress, snapshot.merged.count
    );
    Ok(())
}

/// The part of the last snapshot that [`run`] keeps when it can't read the pull requests.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Previous {
    in_progress: InProgress,
}

/// The snapshot of `inputs`; fails on a todo graph the board would reject or a milestone table it
/// can't read.
pub(crate) fn snapshot(inputs: &Inputs<'_>) -> Result<Snapshot> {
    let todos = match todos::checked(inputs.files) {
        Ok(todos) => todos,
        Err(problems) => bail!(
            "the todo files have problems; run `cargo xtask todos`:\n  {}",
            problems.join("\n  ")
        ),
    };
    let in_progress = in_progress(&todos, inputs);
    let ids: BTreeSet<&str> = in_progress.todos.iter().map(|t| t.id.as_str()).collect();
    let counts = |todos: &[&Todo]| Counts {
        todos: todos.len(),
        done: todos.iter().filter(|t| t.state == State::Done).count(),
        in_progress: todos.iter().filter(|t| ids.contains(t.id.as_str())).count(),
        ready: todos
            .iter()
            .filter(|t| t.state == State::Ready && !ids.contains(t.id.as_str()))
            .count(),
    };
    let all: Vec<&Todo> = todos.iter().collect();
    let by_id: BTreeMap<&str, &Todo> = todos.iter().map(|t| (t.id.as_str(), t)).collect();
    let rounds = SECTIONS
        .iter()
        .filter_map(|&(letter, heading)| {
            let section: Vec<&Todo> = todos.iter().filter(|t| t.id.starts_with(letter)).collect();
            (!section.is_empty()).then(|| Round {
                letter,
                heading,
                counts: counts(&section),
            })
        })
        .collect();
    Ok(Snapshot {
        version: VERSION,
        date: inputs.date.to_owned(),
        totals: counts(&all),
        rounds,
        milestones: milestones(inputs.roadmap, &todos, &ids)?,
        next_up: todos::startable(&todos, &by_id)
            .into_iter()
            .filter(|t| !ids.contains(t.id.as_str()))
            .map(todo_ref)
            .collect(),
        merged: merged(inputs.log),
        in_progress,
    })
}

fn todo_ref(todo: &Todo) -> TodoRef {
    TodoRef {
        id: todo.id.clone(),
        title: todo.title.clone(),
    }
}

/// The todos in `ongoing/`, plus the not-yet-done todos the open pull requests claim (or, without
/// them, the ones the last snapshot listed), in board order.
fn in_progress(todos: &[Todo], inputs: &Inputs<'_>) -> InProgress {
    let (source, checked, claimed): (_, _, BTreeSet<String>) = match (inputs.claims, &inputs.previous) {
        (Some(paths), _) => (
            ClaimSource::PullRequests,
            Some(inputs.date.to_owned()),
            claimed_ids(paths),
        ),
        (None, Some(previous)) if previous.source != ClaimSource::None => (
            ClaimSource::PreviousSnapshot,
            previous.checked.clone(),
            previous.todos.iter().map(|t| t.id.clone()).collect(),
        ),
        (None, _) => (ClaimSource::None, None, BTreeSet::new()),
    };
    let mut listed: Vec<&Todo> = todos
        .iter()
        .filter(|t| t.state == State::Ongoing || (t.state == State::Ready && claimed.contains(&t.id)))
        .collect();
    listed.sort_by_key(|t| order_key(&t.id));
    InProgress {
        source,
        checked,
        todos: listed.into_iter().map(todo_ref).collect(),
    }
}

/// The IDs of the todos whose files `paths` puts under `docs/todos/ongoing/` or `docs/todos/done/`.
fn claimed_ids(paths: &[String]) -> BTreeSet<String> {
    paths
        .iter()
        .filter_map(|path| {
            let file = path
                .strip_prefix("docs/todos/ongoing/")
                .or_else(|| path.strip_prefix("docs/todos/done/"))?;
            let (id, _) = file.split_once('-')?;
            id_parts(id).map(|_| id.to_owned())
        })
        .collect()
}

/// The roadmap's Milestones table, each row with its state.
fn milestones(roadmap: &str, todos: &[Todo], in_progress: &BTreeSet<&str>) -> Result<Vec<Milestone>> {
    milestone_rows(roadmap)?
        .into_iter()
        .map(|[milestone, rounds, ships]| {
            let (id, name) = milestone
                .split_once(' ')
                .with_context(|| format!("{ROADMAP}: milestone {milestone:?} is not `M<n> <name>`"))?;
            let mut matched: Vec<&Todo> = Vec::new();
            for token in rounds.split(',').map(str::trim) {
                let found = matching(token, todos).with_context(|| format!("{ROADMAP}: milestone {id}"))?;
                matched.extend(found);
            }
            Ok(Milestone {
                id: id.to_owned(),
                name: name.to_owned(),
                rounds,
                ships,
                state: milestone_state(&matched, in_progress),
            })
        })
        .collect()
}

/// The cells of each row of the table under `## Milestones`, after its header.
fn milestone_rows(roadmap: &str) -> Result<Vec<[String; 3]>> {
    let mut lines = roadmap
        .lines()
        .skip_while(|line| line.trim() != "## Milestones")
        .skip(1)
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| line.starts_with('|'));
    let header = lines.next().unwrap_or_default();
    if cells(header).as_deref() != Some(&["Milestone", "Rounds", "What ships"].map(str::to_owned)[..]) {
        bail!("{ROADMAP}: no `| Milestone | Rounds | What ships |` table under `## Milestones`");
    }
    lines
        .skip(1)
        .map(|line| {
            cells(line)
                .and_then(|cells| <[String; 3]>::try_from(cells).ok())
                .with_context(|| format!("{ROADMAP}: a Milestones row doesn't have three cells: {line:?}"))
        })
        .collect()
}

/// The trimmed cells of a Markdown table row.
fn cells(line: &str) -> Option<Vec<String>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(|cell| cell.trim().to_owned()).collect())
}

/// The todos one token of a Rounds cell names: a letter (`A`, every todo of that section), a
/// range of rounds (`C1–C7`), a round (`U1`, every todo `U1…`) or a todo (`U3a`). A token that
/// names no todo fails, so a renamed round can't drop out of its milestone unnoticed.
fn matching<'a>(token: &str, todos: &'a [Todo]) -> Result<Vec<&'a Todo>> {
    let rounds = |letter: char, from: u64, to: u64| {
        move |todo: &&Todo| id_parts(&todo.id).is_some_and(|(l, n, _)| l == letter && (from..=to).contains(&n))
    };
    let found: Vec<&Todo> = match round_range(token) {
        Some((letter, from, to)) => todos.iter().filter(rounds(letter, from, to)).collect(),
        None => match (token.chars().next(), id_parts(token)) {
            (Some(letter), None) if token.len() == 1 && letter.is_ascii_uppercase() => {
                todos.iter().filter(|t| t.id.starts_with(letter)).collect()
            }
            (_, Some((letter, round, ""))) => todos.iter().filter(rounds(letter, round, round)).collect(),
            (_, Some(_)) => todos.iter().filter(|t| t.id == token).collect(),
            _ => bail!("{token:?} is not a letter, a round, a range of rounds or a todo ID"),
        },
    };
    if found.is_empty() {
        bail!("{token:?} matches no todo");
    }
    Ok(found)
}

/// `B1–B7` (an en dash or a hyphen) as `('B', 1, 7)`.
fn round_range(token: &str) -> Option<(char, u64, u64)> {
    let (from, to) = token.split_once(['–', '-'])?;
    let (letter, from, "") = id_parts(from.trim())? else {
        return None;
    };
    let (to_letter, to, "") = id_parts(to.trim())? else {
        return None;
    };
    (letter == to_letter && from <= to).then_some((letter, from, to))
}

/// See [`MilestoneState`].
fn milestone_state(todos: &[&Todo], in_progress: &BTreeSet<&str>) -> MilestoneState {
    if todos.iter().all(|t| t.state == State::Done) {
        MilestoneState::Done
    } else if todos
        .iter()
        .any(|t| t.state == State::Done || in_progress.contains(t.id.as_str()))
    {
        MilestoneState::UnderWay
    } else {
        MilestoneState::NotStarted
    }
}

/// The pull requests merged into `main` per `log` (newest first): squash merges, whose subjects end
/// in `(#<n>)`, and merge commits (`Merge pull request #<n> …`), which name a branch and so give no
/// title.
fn merged(log: &str) -> Merged {
    let mut count = 0;
    let mut recent = Vec::new();
    for subject in log.lines() {
        if let Some(title) = without_pr_number(subject) {
            count += 1;
            if recent.len() < RECENT && !names_a_pull_request(&title) {
                recent.push(title);
            }
        } else if subject.starts_with("Merge pull request #") {
            count += 1;
        }
    }
    Merged { count, recent }
}

/// A squash merge's subject without its ` (#<n>)`, and without any other ` (#<n>)` it quotes (a
/// revert's), or `None` for any other commit.
fn without_pr_number(subject: &str) -> Option<String> {
    let (title, number) = subject.trim_end().rsplit_once(" (#")?;
    let number = number.strip_suffix(')')?;
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut out = String::with_capacity(title.len());
    let mut rest = title;
    while let Some((before, after)) = rest.split_once(" (#") {
        out.push_str(before);
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        match after.get(digits..).and_then(|tail| tail.strip_prefix(')')) {
            // ` (#123)`: dropped.
            Some(tail) if digits > 0 => rest = tail,
            // Anything else stays as written.
            _ => {
                out.push_str(" (#");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Some(out)
}

/// Whether `title` still names a pull request by number (`#<digit>`), which the file never holds.
fn names_a_pull_request(title: &str) -> bool {
    title
        .as_bytes()
        .windows(2)
        .any(|pair| pair.first() == Some(&b'#') && pair.get(1).is_some_and(u8::is_ascii_digit))
}

/// The todo files on `rev`, as [`todos::load`] reads them from a checkout: the snapshot describes
/// `main`, whichever branch runs the command.
fn todo_files_at(root: &Path, rev: &str) -> Result<Vec<TodoFile>> {
    let mut files = Vec::new();
    for state in State::ALL {
        let folder = format!("docs/todos/{}/", state.name());
        let names = git(root, &["ls-tree", "--name-only", &format!("{rev}:{folder}")])?;
        for file in names
            .lines()
            .filter(|name| Path::new(name).extension().is_some_and(|ext| ext == "md"))
        {
            let text = at_revision(root, rev, &format!("{folder}{file}"))?;
            files.push(TodoFile {
                state,
                file: file.to_owned(),
                text,
            });
        }
    }
    Ok(files)
}

/// The `docs/todos/` paths each open pull request adds or changes, read with `gh api` (REST).
fn open_pull_request_paths(root: &Path) -> Result<Vec<String>> {
    let numbers = gh(
        root,
        &[
            "api",
            "--paginate",
            "repos/{owner}/{repo}/pulls?state=open&per_page=100",
            "--jq",
            ".[].number",
        ],
    )?;
    let mut paths = Vec::new();
    for number in numbers.lines().map(str::trim).filter(|n| !n.is_empty()) {
        let files = gh(
            root,
            &[
                "api",
                "--paginate",
                &format!("repos/{{owner}}/{{repo}}/pulls/{number}/files?per_page=100"),
                "--jq",
                r#".[] | select(.status != "removed") | .filename"#,
            ],
        )?;
        paths.extend(
            files
                .lines()
                .filter(|path| path.starts_with("docs/todos/"))
                .map(str::to_owned),
        );
    }
    Ok(paths)
}

/// Today in UTC, `YYYY-MM-DD`.
pub(crate) fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / 86_400);
    let (year, month, day) = civil_date(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Days from 1970-01-01 to a `YYYY-MM-DD` date (Howard Hinnant's `days_from_civil`, the inverse of
/// [`civil_date`]), or `None` when it isn't one or comes before 1970.
pub(crate) fn days_since_epoch(date: &str) -> Option<u64> {
    let mut parts = date.splitn(3, '-').map(str::parse::<u64>);
    let (year, month, day) = (parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 1970 {
        return None;
    }
    let year = if month <= 2 { year - 1 } else { year };
    let era = year / 400;
    let year_of_era = year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    (era * 146_097 + day_of_era).checked_sub(719_468)
}

/// The proleptic Gregorian date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{
        ClaimSource, InProgress, Inputs, MilestoneState, Previous, Snapshot, TodoRef, civil_date, claimed_ids,
        days_since_epoch, merged, snapshot, without_pr_number,
    };
    use crate::todos::tests::todo;
    use crate::todos::{State, TodoFile, generate};

    const ROADMAP: &str = "# Roadmap\n\n## Milestones\n\n\
        | Milestone | Rounds | What ships |\n| --- | --- | --- |\n\
        | M0 Foundations | A | The monorepo |\n\
        | M1 Library | B1–B2 | `laminapdf` |\n\
        | M2 Reader | C1–C7, U3a | The reader |\n\
        | M3 Cross-cutting | X | CI |\n\n## Later\n";

    const LOG: &str = "feat(site): the website (#246)\nfix: a typo (#245)\nMerge pull request #7 from someone/branch\nInitial import\n";

    /// A1 and B1, B2a done; B2b ready after B2a; C1 ready, C2 after C1; C8 done (outside C1–C7);
    /// U3a ready, U3b done; X1 ongoing.
    fn board() -> Vec<TodoFile> {
        vec![
            todo(State::Done, "A1", "None"),
            todo(State::Done, "B1", "None"),
            todo(State::Done, "B2a", "None"),
            todo(State::Ready, "B2b", "[B2a](../done/B2a-x.md)"),
            todo(State::Ready, "C1", "None"),
            todo(State::Ready, "C2", "[C1](./C1-x.md)"),
            todo(State::Done, "C8", "None"),
            todo(State::Ready, "U3a", "None"),
            todo(State::Done, "U3b", "None"),
            todo(State::Ongoing, "X1", "[A1](../done/A1-x.md)"),
        ]
    }

    fn take(files: &[TodoFile], claims: Option<&[String]>, previous: Option<InProgress>) -> Snapshot {
        snapshot(&Inputs {
            files,
            roadmap: ROADMAP,
            log: LOG,
            claims,
            previous,
            date: "2026-11-05",
        })
        .unwrap()
    }

    fn ids(todos: &[TodoRef]) -> Vec<&str> {
        todos.iter().map(|t| t.id.as_str()).collect()
    }

    fn states(snapshot: &Snapshot) -> Vec<(&str, MilestoneState)> {
        snapshot.milestones.iter().map(|m| (m.id.as_str(), m.state)).collect()
    }

    #[test]
    fn counts_and_next_up_equal_the_boards() {
        let files = board();
        let board = generate(&files).unwrap().board;
        let snapshot = take(&files, None, None);
        let start_now: Vec<&str> = board
            .split("## Can start now")
            .nth(1)
            .unwrap()
            .split("## Ongoing")
            .next()
            .unwrap()
            .lines()
            .filter_map(|line| line.strip_prefix("- [")?.split(']').next())
            .collect();
        assert_eq!(ids(&snapshot.next_up), start_now);
        assert_eq!(start_now, ["B2b", "C1", "U3a"]);
        // The board's table rows, `| ID | [title](…) | state | … |`, counted per state and section.
        for round in &snapshot.rounds {
            let rows: Vec<&str> = board
                .lines()
                .filter(|line| line.starts_with(&format!("| {}", round.letter)))
                .collect();
            let count = |state: &str| {
                rows.iter()
                    .filter(|row| row.contains(&format!(" | {state} | ")))
                    .count()
            };
            assert_eq!(
                (round.counts.done, round.counts.in_progress, round.counts.ready),
                (count("done"), count("ongoing"), count("ready")),
                "section {}",
                round.letter
            );
        }
        assert_eq!((snapshot.totals.todos, snapshot.totals.done), (10, 5));
    }

    #[test]
    fn milestone_states_follow_their_rounds_ranges_and_todos() {
        let files = board();
        assert_eq!(
            states(&take(&files, None, None)),
            [
                ("M0", MilestoneState::Done),
                ("M1", MilestoneState::UnderWay),
                ("M2", MilestoneState::NotStarted),
                ("M3", MilestoneState::UnderWay),
            ]
        );
        // A claim by an open pull request puts M2 under way; C8 (done) lies outside C1–C7.
        let claims = ["docs/todos/ongoing/C1-x.md".to_owned()];
        assert_eq!(
            states(&take(&files, Some(&claims), None))[2],
            ("M2", MilestoneState::UnderWay)
        );
    }

    #[test]
    fn a_rounds_token_that_matches_no_todo_fails() {
        for token in ["C9–C12", "Z", "U3c", "W1"] {
            let roadmap = ROADMAP.replace("C1–C7, U3a", token);
            let error = snapshot(&Inputs {
                files: &board(),
                roadmap: &roadmap,
                log: LOG,
                claims: None,
                previous: None,
                date: "2026-11-05",
            })
            .unwrap_err();
            assert!(format!("{error:#}").contains("matches no todo"), "{token}: {error:#}");
        }
    }

    #[test]
    fn open_pull_requests_claim_the_todos_they_move() {
        let claims = [
            "docs/todos/ongoing/C1-x.md".to_owned(),
            "docs/todos/done/U3a-x.md".to_owned(),
            "docs/todos/ready/C2-x.md".to_owned(),
            "docs/todos/README.md".to_owned(),
        ];
        assert_eq!(claimed_ids(&claims).into_iter().collect::<Vec<_>>(), ["C1", "U3a"]);
        let snapshot = take(&board(), Some(&claims), None);
        assert_eq!(snapshot.in_progress.source, ClaimSource::PullRequests);
        assert_eq!(snapshot.in_progress.checked.as_deref(), Some("2026-11-05"));
        assert_eq!(ids(&snapshot.in_progress.todos), ["C1", "U3a", "X1"]);
        // Claimed todos leave "next up" for "in progress".
        assert_eq!(ids(&snapshot.next_up), ["B2b"]);
        let c = snapshot.rounds.iter().find(|r| r.letter == 'C').unwrap();
        assert_eq!((c.counts.done, c.counts.in_progress, c.counts.ready), (1, 1, 1));
    }

    #[test]
    fn offline_keeps_the_last_snapshots_claims_that_are_not_done() {
        let previous = InProgress {
            source: ClaimSource::PullRequests,
            checked: Some("2026-11-01".to_owned()),
            todos: ["C1", "U3b"]
                .map(|id| TodoRef {
                    id: id.to_owned(),
                    title: String::new(),
                })
                .to_vec(),
        };
        let snapshot = take(&board(), None, Some(previous));
        assert_eq!(snapshot.in_progress.source, ClaimSource::PreviousSnapshot);
        assert_eq!(snapshot.in_progress.checked.as_deref(), Some("2026-11-01"));
        assert_eq!(ids(&snapshot.in_progress.todos), ["C1", "X1"]);
        let none = take(&board(), None, None);
        assert_eq!(
            (none.in_progress.source, none.in_progress.checked),
            (ClaimSource::None, None)
        );
        // A last snapshot that read no pull requests passes nothing on.
        let empty = InProgress {
            source: ClaimSource::None,
            checked: None,
            todos: Vec::new(),
        };
        assert_eq!(take(&board(), None, Some(empty)).in_progress.source, ClaimSource::None);
        assert_eq!(ids(&none.in_progress.todos), ["X1"]);
    }

    #[test]
    fn the_file_reads_back_and_holds_no_pull_request_number() {
        let json = serde_json::to_string_pretty(&take(&board(), None, None)).unwrap();
        let previous: Previous = serde_json::from_str(&json).unwrap();
        assert_eq!(ids(&previous.in_progress.todos), ["X1"]);
        assert!(json.contains("\"recent\": [\n      \"feat(site): the website\",\n      \"fix: a typo\"\n    ]"));
        assert!(json.contains("\"count\": 3"));
        assert!(!json.contains('#'), "{json}");
        // The keys in the order the site's type lists them (serde_json's Value would sort them).
        let at = |key: &str| json.find(&format!("\n  \"{key}\": ")).unwrap();
        let keys = [
            "version",
            "date",
            "totals",
            "rounds",
            "milestones",
            "inProgress",
            "nextUp",
            "merged",
        ];
        assert!(keys.windows(2).all(|pair| at(pair[0]) < at(pair[1])), "{json}");
    }

    #[test]
    fn merge_subjects_lose_their_pull_request_numbers() {
        assert_eq!(without_pr_number("feat(x): y (#12)").as_deref(), Some("feat(x): y"));
        assert_eq!(
            without_pr_number("Revert \"feat: x (#212)\" (#250)").as_deref(),
            Some("Revert \"feat: x\"")
        );
        assert_eq!(
            without_pr_number("fix: keep (#tag) (#9)").as_deref(),
            Some("fix: keep (#tag)")
        );
        assert_eq!(without_pr_number("Initial import"), None);
        assert_eq!(without_pr_number("fix: a (#12) b"), None);
        assert_eq!(without_pr_number("fix: a (#)"), None);
    }

    #[test]
    fn merged_counts_merge_commits_and_leaves_out_titles_that_name_a_pull_request() {
        let log = "fix: handle #198 review notes (#251)\nMerge pull request #204 from someone/branch\nfeat: x (#200)\n";
        let merged = merged(log);
        assert_eq!(merged.count, 3);
        assert_eq!(merged.recent, ["feat: x"]);
    }

    #[test]
    fn days_since_epoch_should_invert_civil_date() {
        for days in [0, 59, 60, 365, 11_016, 20_767, 20_768] {
            let (y, m, d) = civil_date(days);
            assert_eq!(days_since_epoch(&format!("{y:04}-{m:02}-{d:02}")), Some(days));
        }
        assert_eq!(days_since_epoch("1969-12-31"), None);
        assert_eq!(days_since_epoch("2026-13-01"), None);
        assert_eq!(days_since_epoch("not a date"), None);
    }

    #[test]
    fn civil_dates_count_from_the_epoch() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(11_017), (2000, 3, 1));
        assert_eq!(civil_date(19_782), (2024, 2, 29));
        assert_eq!(civil_date(20_762), (2026, 11, 5));
    }
}
