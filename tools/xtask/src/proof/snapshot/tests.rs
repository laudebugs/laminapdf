use serde_json::{Value, json};

use super::*;

/// A proof record that ran on `ran`, with the jobs' results and `lex`'s fuzzed seconds.
fn record(commit: &str, ran: &str, full: bool, jobs: &[(&str, &str)], fuzz_seconds: u64) -> Value {
    let jobs: serde_json::Map<String, Value> = jobs
        .iter()
        .map(|(job, result)| ((*job).to_owned(), json!(result)))
        .collect();
    json!({
        "kind": "proof", "version": 1, "commit": commit, "date": format!("{ran}T04:00:00Z"), "ran": ran,
        "runner": "github-hosted/ubuntu-24.04-x64", "quick": !full, "lamina": "0.2.0-alpha.4", "full": full,
        "jobs": jobs,
        "corpus": {"complete": true, "inputs": 3, "sources": {
            "pdfjs": {"files": 3, "opened": 1, "repaired": 1, "refused": 1, "refused_alone": 1,
                      "errors": {"panic": 0, "crash": 1, "timeout": 0}}}},
        "differential": {"inputs": 3, "sample": null, "corpus": "all", "source_filter": [], "failures": 0,
            "lamina_errors": 0, "missing": 0, "broken_readers": 0,
            "tools": {"pdfjs": {"version": "6.4.299", "files": 3, "timeouts": 0, "crashes": 0,
                "facts": {"page_count": {"agree": 2, "disagree": 1, "excused": 1}}}}},
        "text": {"sample": null, "source_filter": [], "tools": {"pdfjs": "6.4.299"},
            "sources": {"pdfjs": {"files": 3, "pages": 9, "compared": 8, "agree": 7}}},
        "render": null,
        "fuzz": {
            "lex": {"seconds": 600, "fuzzed_seconds": fuzz_seconds, "executions": 10, "corpus_files": 1, "crashed": false},
            "open": {"seconds": 600, "fuzzed_seconds": null, "executions": 5, "corpus_files": 1, "crashed": true}},
        "verapdf": {"version": "1.30.3", "validations": [
            {"file": "a.pdf", "profile": "ua1", "compliant": true, "rules_passed": 1, "rules_failed": 0}]},
        "roundtrip": {"cases": 4, "saved": 3, "refused": 1, "tools": {"qpdf": {"version": "12", "passed": true}}},
        "runtimes": {"passed": ["node"], "failed": [], "missing": []},
    })
}

const PASSED: &[(&str, &str)] = &[("differential", "success"), ("fuzz", "success")];
const FAILED: &[(&str, &str)] = &[("differential", "success"), ("render-diff", "failure")];

fn history(lines: &[Value]) -> String {
    lines
        .iter()
        .map(|line| serde_json::to_string(line).unwrap() + "\n")
        .collect()
}

/// The snapshot of `lines` written on `today`, which must have a full record.
fn take(lines: &[Value], today: &str) -> Snapshot {
    snapshot(&history(lines), today).unwrap().unwrap()
}

#[test]
fn snapshot_should_take_the_newest_full_proof_record() {
    let snapshot = take(
        &[
            record("a", "2026-11-01", true, PASSED, 600),
            json!({"kind": "bench", "benches": {}}),
            record("b", "2026-11-02", true, FAILED, 600),
            record("c", "2026-11-03", false, PASSED, 30),
        ],
        "2026-11-04",
    );
    assert_eq!(
        (snapshot.night.commit.as_str(), snapshot.night.ran.as_str()),
        ("b", "2026-11-02")
    );
    assert!(!snapshot.night.passed);
    assert_eq!(snapshot.date, "2026-11-04");
}

#[test]
fn snapshot_should_count_passed_nights_over_the_last_thirty_full_ones() {
    let mut lines: Vec<Value> = (1..=31)
        .map(|day| {
            let jobs = if day % 2 == 0 { FAILED } else { PASSED };
            record(&format!("c{day}"), &format!("2026-10-{day:02}"), true, jobs, 600)
        })
        .collect();
    lines.push(record("short", "2026-11-01", false, PASSED, 30));
    let snapshot = take(&lines, "2026-11-01");
    // Days 2 to 31: 15 odd days passed and 15 even days failed; day 1 falls outside the 30.
    assert_eq!(
        snapshot.nights,
        Nights {
            recorded: 30,
            passed: 15,
            since: "2026-10-02".to_owned()
        }
    );
}

#[test]
fn snapshot_should_sum_libfuzzers_own_hours_over_every_full_record() {
    // Each full run: `lex` fuzzed 3,600 s; `open` crashed and printed no time, so it adds nothing.
    let mut lines: Vec<Value> = (1..=3)
        .map(|day| record(&format!("c{day}"), &format!("2026-11-0{day}"), true, PASSED, 3600))
        .collect();
    lines.push(record("short", "2026-11-04", false, PASSED, 36_000));
    assert_eq!(take(&lines, "2026-11-04").fuzz_hours, 3);
}

#[test]
fn a_failure_a_rerun_fixed_should_still_fail_the_night() {
    // The run fails, its failed jobs are re-run the same day and pass: the night didn't pass.
    let lines = [
        record("a", "2026-11-01", true, PASSED, 600),
        record("b", "2026-11-02", true, FAILED, 600),
        record("b", "2026-11-02", true, PASSED, 600),
    ];
    let snapshot = take(&lines, "2026-11-02");
    assert_eq!(
        snapshot.nights,
        Nights {
            recorded: 2,
            passed: 1,
            since: "2026-11-01".to_owned()
        }
    );
    assert!(!snapshot.night.passed);
    assert_eq!(snapshot.night.jobs["render-diff"], "failure");
    assert_eq!(snapshot.night.jobs["fuzz"], "success");
}

#[test]
fn several_runs_on_one_day_should_count_as_one_night() {
    let lines = [
        record("a", "2026-11-01", true, PASSED, 600),
        record("b", "2026-11-01", true, PASSED, 600),
        record("c", "2026-11-01", true, PASSED, 600),
    ];
    let snapshot = take(&lines, "2026-11-01");
    assert_eq!(
        snapshot.nights,
        Nights {
            recorded: 1,
            passed: 1,
            since: "2026-11-01".to_owned()
        }
    );
    // The figures are the day's newest run's.
    assert_eq!(snapshot.night.commit, "c");
}

#[test]
fn snapshot_should_be_none_without_a_full_record() {
    let lines = history(&[record("a", "2026-11-01", false, PASSED, 30), json!({"kind": "bench"})]);
    assert_eq!(snapshot(&lines, "2026-11-01").unwrap(), None);
}

#[test]
fn snapshot_should_leave_out_proof_records_of_another_version() {
    let mut newer = record("b", "2026-11-02", true, PASSED, 600);
    newer["version"] = json!(2);
    newer["corpus"] = json!("a shape version 1 can't read");
    let snapshot = take(&[record("a", "2026-11-01", true, PASSED, 600), newer], "2026-11-02");
    assert_eq!(snapshot.night.commit, "a");
}

#[test]
fn snapshot_should_fail_on_a_proof_record_it_cannot_read() {
    let mut broken = record("a", "2026-11-01", true, PASSED, 600);
    broken["jobs"] = json!("not an object");
    assert!(snapshot(&history(&[broken]), "2026-11-01").is_err());
}

#[test]
fn a_night_without_jobs_should_not_pass() {
    assert!(
        !take(&[record("a", "2026-11-01", true, &[], 600)], "2026-11-01")
            .night
            .passed
    );
}

#[test]
fn night_should_keep_null_sections_null_and_cut_the_rest_to_what_the_page_shows() {
    let snapshot = take(&[record("a", "2026-11-01", true, PASSED, 600)], "2026-11-01");
    let night = serde_json::to_value(&snapshot.night).unwrap();
    assert_eq!(night["render"], Value::Null);
    assert_eq!(
        night["corpus"]["sources"]["pdfjs"],
        json!({"files": 3, "opened": 1, "repaired": 1, "refused": 1, "refusedAlone": 1, "errors": 1})
    );
    assert_eq!(
        night["differential"]["tools"]["pdfjs"]["facts"]["pageCount"],
        json!({"agree": 2, "disagree": 1, "excused": 1})
    );
    // Both targets report executions, so they sum; one of the two crashed.
    assert_eq!(
        night["fuzz"],
        json!({"targets": 2, "seconds": 600, "executions": 15, "crashed": 1})
    );
    assert_eq!(
        night["verapdf"]["validations"][0],
        json!({"file": "a.pdf", "profile": "ua1", "compliant": true})
    );
    assert_eq!(night["roundtrip"]["tools"], json!({"qpdf": true}));
}

#[test]
fn snapshot_should_read_back_as_written() {
    let snapshot = take(&[record("a", "2026-11-01", true, PASSED, 600)], "2026-11-01");
    let text = serde_json::to_string_pretty(&snapshot).unwrap();
    assert_eq!(serde_json::from_str::<Snapshot>(&text).unwrap(), snapshot);
    assert!(text.contains("\"fuzzHours\""), "camelCase keys: {text}");
}

#[test]
fn days_between_should_count_calendar_days() {
    assert_eq!(days_between("2026-10-30", "2026-11-02"), Some(3));
    assert_eq!(days_between("2024-02-28", "2024-03-01"), Some(2));
    assert_eq!(days_between("2026-11-02", "2026-11-01"), None);
    assert_eq!(days_between("not a date", "2026-11-01"), None);
}

/// A folder with a committed snapshot, which is no git repository.
fn committed(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("xtask-proof-snapshot-{name}-{}", std::process::id()));
    let output = root.join(OUTPUT);
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(&output, "committed\n").unwrap();
    (root, output)
}

#[test]
fn run_should_keep_the_committed_file_offline() {
    let (root, output) = committed("offline");
    run(&root, true).unwrap();
    assert_eq!(fs::read_to_string(&output).unwrap(), "committed\n");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn run_should_keep_the_committed_file_without_the_history_branch() {
    // Not a git repository, so `git show origin/bench-data:history.jsonl` fails.
    let (root, output) = committed("no-branch");
    run(&root, false).unwrap();
    assert_eq!(fs::read_to_string(&output).unwrap(), "committed\n");
    let _ = fs::remove_dir_all(&root);
}
