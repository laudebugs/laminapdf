//! Runs one differential reader (lamina's example or a tool in `tools/differential/`) over a
//! list of inputs with crash and hang isolation (todo X3a review: A3–A6).
//!
//! The inputs go to the reader in chunks, each a separate process under a time limit. A chunk
//! that crashes, hangs or reports a different set of files than it was given is rerun one file
//! per process, so a bad file costs one entry (`"outcome": "timeout"` or `"crash"`) instead of
//! the run. A reader whose every single-file run in a chunk crashes is broken (missing, wrong
//! version, bad install): it stops there and the comparison reports it.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::{Value, json};

/// How a reader is started: `program args… <list.tsv> <out.json>`.
#[derive(Debug, Clone)]
pub(crate) struct Reader {
    /// The name in reports (`lamina`, `pdfjs`, …).
    pub(crate) name: &'static str,
    pub(crate) program: OsString,
    pub(crate) args: Vec<OsString>,
    pub(crate) dir: PathBuf,
    /// Whether the reader tells "repaired" from "opened" (used when it never produced a header).
    pub(crate) reports_repair: bool,
}

/// Limits for one reader's run.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) chunk: usize,
    pub(crate) chunk_timeout: Duration,
    pub(crate) file_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            chunk: 200,
            chunk_timeout: Duration::from_mins(20),
            file_timeout: Duration::from_secs(120),
        }
    }
}

/// What a reader produced: the report JSON `tools/differential/` writes, with a fact entry for
/// every input, plus whether the reader broke.
#[derive(Debug)]
pub(crate) struct Run {
    pub(crate) report: Value,
    /// Why the reader stopped before covering every input, if it did.
    pub(crate) broken: Option<String>,
}

/// How a child process ended.
#[derive(Debug)]
enum Exit {
    Status(ExitStatus),
    TimedOut,
}

/// Runs `command` (output appended to `log`) and kills it after `timeout`.
fn run_with_timeout(mut command: Command, log: &Path, timeout: Duration) -> Result<Exit> {
    let out = fs::OpenOptions::new().append(true).create(true).open(log)?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(out.try_clone()?)
        .stderr(out)
        .spawn()
        .context("starting a differential reader")?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Exit::Status(status));
        }
        if Instant::now() >= deadline {
            // Killing an already-exited child fails harmlessly; reap it either way.
            let _ = child.kill();
            let _ = child.wait();
            return Ok(Exit::TimedOut);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn marker(outcome: &str) -> Value {
    json!({"outcome": outcome, "page_count": null, "outline_titles": null})
}

/// Runs the reader once over `inputs`; the report if it exited cleanly and covered exactly
/// those inputs.
fn attempt(reader: &Reader, inputs: &[(String, PathBuf)], work: &Path, timeout: Duration) -> Result<Option<Value>> {
    let list = work.join(format!("{}-chunk.tsv", reader.name));
    let out = work.join(format!("{}-chunk.json", reader.name));
    let lines = inputs.iter().fold(String::new(), |mut s, (name, path)| {
        let _ = writeln!(s, "{name}\t{}", path.display());
        s
    });
    fs::write(&list, lines)?;
    let _ = fs::remove_file(&out);
    let mut command = Command::new(&reader.program);
    command.args(&reader.args).arg(&list).arg(&out).current_dir(&reader.dir);
    let log = work.join(format!("{}.log", reader.name));
    let Exit::Status(status) = run_with_timeout(command, &log, timeout)? else {
        return Ok(None);
    };
    if !status.success() {
        return Ok(None);
    }
    let Ok(text) = fs::read_to_string(&out) else {
        return Ok(None);
    };
    let Ok(report) = serde_json::from_str::<Value>(&text) else {
        return Ok(None);
    };
    let want: BTreeSet<&str> = inputs.iter().map(|(n, _)| n.as_str()).collect();
    let got: BTreeSet<&str> = report
        .get("facts")
        .and_then(Value::as_object)
        .map(|m| m.keys().map(String::as_str).collect())
        .unwrap_or_default();
    Ok((want == got).then_some(report))
}

/// Runs `reader` over every input (see the module docs); `work` holds its lists, outputs and
/// `<name>.log`.
pub(crate) fn run(reader: &Reader, inputs: &[(String, PathBuf)], work: &Path, limits: Limits) -> Result<Run> {
    let mut facts = serde_json::Map::new();
    let mut header: Option<(Value, Value, Value)> = None;
    let mut broken = None;
    let mut take = |report: Value, facts: &mut serde_json::Map<String, Value>| {
        if header.is_none() {
            header = Some((
                report.get("tool").cloned().unwrap_or(Value::Null),
                report.get("version").cloned().unwrap_or(Value::Null),
                report.get("reports_repair").cloned().unwrap_or(Value::Null),
            ));
        }
        if let Some(Value::Object(chunk)) = report.get("facts") {
            facts.extend(chunk.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    };
    for chunk in inputs.chunks(limits.chunk.max(1)) {
        if let Some(report) = attempt(reader, chunk, work, limits.chunk_timeout)? {
            take(report, &mut facts);
            continue;
        }
        let mut crashed = 0;
        for one in chunk.chunks(1) {
            let name = one.first().map(|(n, _)| n.clone()).unwrap_or_default();
            let started = Instant::now();
            match attempt(reader, one, work, limits.file_timeout)? {
                Some(report) => take(report, &mut facts),
                None if started.elapsed() >= limits.file_timeout => {
                    facts.insert(name, marker("timeout"));
                }
                None => {
                    crashed += 1;
                    facts.insert(name, marker("crash"));
                }
            }
        }
        if crashed == chunk.len() {
            broken = Some(format!(
                "every file of a {}-file chunk crashed {} on its own; see {}",
                chunk.len(),
                reader.name,
                work.join(format!("{}.log", reader.name)).display()
            ));
            break;
        }
    }
    let (tool, version, reports_repair) = header.unwrap_or((
        json!(reader.name),
        json!("unknown (no run succeeded)"),
        json!(reader.reports_repair),
    ));
    Ok(Run {
        report: json!({
            "tool": tool,
            "version": version,
            "reports_repair": reports_repair,
            "facts": Value::Object(facts),
        }),
        broken,
    })
}

/// The names in `report` that are not inputs, and the inputs missing from it.
pub(crate) fn coverage(report: &Value, inputs: &[(String, PathBuf)]) -> (Vec<String>, Vec<String>) {
    let names: BTreeSet<&str> = inputs.iter().map(|(n, _)| n.as_str()).collect();
    let facts: BTreeMap<&str, &Value> = report
        .get("facts")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.as_str(), v)).collect())
        .unwrap_or_default();
    let extra = facts
        .keys()
        .filter(|k| !names.contains(*k))
        .map(|k| (*k).to_owned())
        .collect();
    let missing = names
        .iter()
        .filter(|n| !facts.contains_key(*n))
        .map(|n| (*n).to_owned())
        .collect();
    (extra, missing)
}

// The tests drive a fake reader written as a POSIX shell script.
#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-runner-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A reader that is a shell script: `$1` is the list, `$2` the output.
    fn script_reader(dir: &Path, body: &str) -> Reader {
        let script = dir.join("reader.sh");
        fs::write(&script, body).unwrap();
        Reader {
            name: "fake",
            program: "sh".into(),
            args: vec![script.into_os_string()],
            dir: dir.to_path_buf(),
            reports_repair: true,
        }
    }

    fn inputs(names: &[&str]) -> Vec<(String, PathBuf)> {
        names.iter().map(|n| ((*n).to_owned(), PathBuf::from(*n))).collect()
    }

    /// Writes an `opened` fact for every list entry, but hangs on `hang.pdf` and exits 1 on
    /// `crash.pdf` (in a chunk or on its own).
    const FLAKY_READER: &str = r#"
if grep -q '^hang.pdf' "$1"; then sleep 30; fi
if grep -q '^crash.pdf' "$1"; then exit 1; fi
{
  printf '{"tool":"fake","version":"1","reports_repair":true,"facts":{'
  sep=''
  while IFS="$(printf '\t')" read -r name path; do
    printf '%s"%s":{"outcome":"opened","page_count":1,"outline_titles":[]}' "$sep" "$name"
    sep=','
  done < "$1"
  printf '}}'
} > "$2"
"#;

    /// Short enough to keep the tests quick, long enough that a loaded CI machine starts `sh`
    /// and writes a report well within them (500 ms flaked on a busy runner).
    fn quick() -> Limits {
        Limits {
            chunk: 10,
            chunk_timeout: Duration::from_secs(3),
            file_timeout: Duration::from_secs(3),
        }
    }

    #[test]
    fn run_should_isolate_a_hang_and_a_crash_to_their_files() {
        let dir = scratch("isolate");
        let reader = script_reader(&dir, FLAKY_READER);
        let run = run(&reader, &inputs(&["a.pdf", "crash.pdf", "hang.pdf"]), &dir, quick()).unwrap();
        let outcome = |n: &str| run.report["facts"][n]["outcome"].as_str().unwrap().to_owned();
        let got = (
            outcome("a.pdf"),
            outcome("crash.pdf"),
            outcome("hang.pdf"),
            run.broken.is_none(),
        );
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(got, ("opened".into(), "crash".into(), "timeout".into(), true));
    }

    #[test]
    fn run_should_mark_a_reader_broken_when_every_file_crashes() {
        let dir = scratch("broken");
        let reader = script_reader(&dir, "exit 3\n");
        let run = run(&reader, &inputs(&["a.pdf", "b.pdf"]), &dir, quick()).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(run.broken.is_some());
    }

    #[test]
    fn run_should_rerun_a_chunk_whose_report_drops_a_file() {
        let dir = scratch("drops");
        // Reports only the first line of whatever list it gets.
        let body = r#"name=$(head -n1 "$1" | cut -f1)
printf '{"tool":"fake","version":"1","reports_repair":true,"facts":{"%s":{"outcome":"opened","page_count":1,"outline_titles":[]}}}' "$name" > "$2"
"#;
        let reader = script_reader(&dir, body);
        let run = run(&reader, &inputs(&["a.pdf", "b.pdf"]), &dir, quick()).unwrap();
        let (extra, missing) = coverage(&run.report, &inputs(&["a.pdf", "b.pdf"]));
        fs::remove_dir_all(&dir).unwrap();
        assert!(extra.is_empty() && missing.is_empty());
    }

    #[test]
    fn coverage_should_list_extra_and_missing_names() {
        let report = json!({"facts": {"a.pdf": {}, "z.pdf": {}}});
        assert_eq!(
            coverage(&report, &inputs(&["a.pdf", "b.pdf"])),
            (vec!["z.pdf".to_owned()], vec!["b.pdf".to_owned()])
        );
    }
}
