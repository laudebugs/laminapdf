//! The command-line transcripts the docs site shows (todo Q4a): each `fixtures/cli/transcripts/
//! <name>.console` is a terminal session, the commands and what they print, that this test writes
//! and compares, so no command or output on the docs' pages is typed by hand.
//!
//! `transcripts.json` gives each transcript's commands and the files it needs, under the names the
//! page uses, and any setup commands that run first and aren't shown. Each runs in a folder of its own with those files and the built `laminapdf` first on
//! `PATH`, and with no `LAMINA_PDF_*` variable from the caller's environment. Its commands run in
//! one `sh`, so `echo $?` and pipes work as they do in a terminal, with standard error merged into
//! standard output. The version is written `{{version}}` (the docs fill it in), and timings
//! (`"elapsedMs":…`) are masked in the comparison. `UPDATE_GOLDEN=1` rewrites the transcripts
//! from the run. Unix only: it needs `sh`, and the output is the same on every system.
#![cfg(unix)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Separates the commands' outputs in the shell's: a record separator on a line of its own.
const MARK: &str = "\u{1e}";

fn root() -> PathBuf {
    laminapdf_test_fixtures::fixtures_dir().parent().unwrap().to_owned()
}

fn transcripts_dir() -> PathBuf {
    laminapdf_test_fixtures::fixtures_dir().join("cli/transcripts")
}

/// What `transcripts.json` gives one transcript.
struct Entry {
    /// Commands run first and not shown, such as the build that makes the PDF a later step of a
    /// tutorial reads (optional).
    setup: Vec<String>,
    /// The commands, as the reader types them.
    commands: Vec<String>,
    /// The name in its folder → the path from the repository root.
    files: BTreeMap<String, String>,
}

fn manifest() -> BTreeMap<String, Entry> {
    let text = fs::read_to_string(transcripts_dir().join("transcripts.json")).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();
    json.as_object()
        .unwrap()
        .iter()
        .map(|(name, entry)| {
            let files = entry["files"].as_object().map_or_else(BTreeMap::new, |files| {
                files
                    .iter()
                    .map(|(to, from)| {
                        let from = from
                            .as_str()
                            .unwrap_or_else(|| panic!("{name}: files.{to} isn't a path"));
                        (to.clone(), from.to_owned())
                    })
                    .collect()
            });
            let list = |key: &str| -> Vec<String> {
                entry[key].as_array().map_or_else(Vec::new, |commands| {
                    commands
                        .iter()
                        .map(|command| {
                            let command = command.as_str();
                            command
                                .unwrap_or_else(|| panic!("{name}: a {key} command isn't a string"))
                                .to_owned()
                        })
                        .collect()
                })
            };
            let entry = Entry {
                setup: list("setup"),
                commands: list("commands"),
                files,
            };
            assert!(!entry.commands.is_empty(), "{name}: no commands");
            (name.clone(), entry)
        })
        .collect()
}

/// Runs `commands` in `dir` and writes the session as a terminal shows it.
fn run(dir: &Path, setup: &[String], commands: &[String]) -> String {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_laminapdf"));
    let path = format!(
        "{}:{}",
        binary.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    // Each command starts with the exit status of the one before (`echo $?`), after the mark.
    let mut script = String::from("exec 2>&1\n");
    // The setup must work: a failure stops the session before the first mark, and the assertion
    // below names it.
    for command in setup {
        script.push_str(command);
        script.push_str(" >/dev/null || exit 1\n");
    }
    for command in commands {
        script.push_str("s=$?; printf '");
        script.push_str(MARK);
        script.push_str("\\n'; (exit $s); ");
        script.push_str(command);
        script.push('\n');
    }
    let mut shell = Command::new("sh");
    shell
        .arg("-c")
        .arg(&script)
        .current_dir(dir)
        .env("PATH", path)
        .env("LC_ALL", "C");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("LAMINA_PDF_") {
            shell.env_remove(key);
        }
    }
    let output = shell.output().unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    let separator = format!("{MARK}\n");
    let outputs: Vec<&str> = text.split(&separator).skip(1).collect();
    assert_eq!(
        outputs.len(),
        commands.len(),
        "the shell stopped before every command ran ({}):\n{text}",
        output.status
    );
    let mut session = String::new();
    for (command, printed) in commands.iter().zip(outputs) {
        session.push_str("$ ");
        session.push_str(command);
        session.push('\n');
        session.push_str(printed);
    }
    session.replace(env!("CARGO_PKG_VERSION"), "{{version}}")
}

/// `text` with every timing replaced, for the comparison only.
fn masked(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("\"elapsedMs\":") {
        let (head, tail) = rest.split_at(at + "\"elapsedMs\":".len());
        out.push_str(head);
        out.push('#');
        rest = tail.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    out.push_str(rest);
    out
}

#[test]
fn every_transcript_is_what_the_commands_print() {
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("transcripts");
    let mut stale = Vec::new();
    for (name, entry) in manifest() {
        let file = transcripts_dir().join(format!("{name}.console"));
        let expected = fs::read_to_string(&file).unwrap_or_default();
        let dir = scratch.join(&name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for (to, from) in &entry.files {
            fs::copy(root().join(from), dir.join(to)).unwrap_or_else(|e| panic!("{name}: {from}: {e}"));
        }
        let actual = run(&dir, &entry.setup, &entry.commands);
        if masked(&actual) == masked(&expected) {
            continue;
        }
        if update {
            fs::write(&file, &actual).unwrap();
        } else {
            stale.push(format!("{name}.console:\n--- expected\n{expected}--- actual\n{actual}"));
        }
    }
    assert!(
        stale.is_empty(),
        "stale transcripts (UPDATE_GOLDEN=1 rewrites them):\n{}",
        stale.join("\n")
    );
}

#[test]
fn every_transcript_file_is_in_the_manifest() {
    let manifest = manifest();
    let files: Vec<String> = fs::read_dir(transcripts_dir())
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().into_string().unwrap();
            name.strip_suffix(".console").map(str::to_owned)
        })
        .collect();
    let missing: Vec<&String> = files.iter().filter(|name| !manifest.contains_key(*name)).collect();
    assert!(
        missing.is_empty(),
        "transcripts without a transcripts.json entry: {missing:?}"
    );
}

#[test]
fn timings_are_masked_and_nothing_else() {
    assert_eq!(
        masked(r#"{"elapsedMs":71,"failed":1}"#),
        masked(r#"{"elapsedMs":9,"failed":1}"#)
    );
    assert_ne!(masked(r#"{"failed":1}"#), masked(r#"{"failed":2}"#));
}
