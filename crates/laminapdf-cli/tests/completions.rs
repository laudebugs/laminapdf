//! Shell completions and man pages come from the clap definitions.
#![allow(clippy::unwrap_used, reason = "test helpers: a panic is a test failure")]

mod common;

use std::process::{Command, Stdio};

use common::{arg, folder, laminapdf};

#[test]
fn the_bash_script_should_parse() {
    let run = laminapdf(&["completions", "bash"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let script = folder("completions").join("laminapdf.bash");
    std::fs::write(&script, &run.stdout).unwrap();
    let parsed = Command::new("bash")
        .args(["-n", arg(&script)])
        .stdout(Stdio::null())
        .status();
    // A machine without bash can't check the syntax; the script was still written.
    if let Ok(status) = parsed {
        assert!(status.success());
    }
    for shell in ["zsh", "fish", "powershell", "elvish"] {
        let run = laminapdf(&["completions", shell]);
        assert!(run.code == 0 && run.stdout_text().contains("laminapdf"), "{shell}");
    }
}

#[test]
fn there_should_be_a_man_page_per_command() {
    let out = folder("man-pages");
    assert_eq!(laminapdf(&["man-pages", arg(&out)]).code, 0);
    let mut pages: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    pages.sort();
    for page in [
        "laminapdf.1",
        "laminapdf-info.1",
        "laminapdf-extract.1",
        "laminapdf-pages-rotate.1",
        "laminapdf-schema.1",
    ] {
        assert!(pages.iter().any(|p| p == page), "{page} in {pages:?}");
    }
    assert!(
        !pages.iter().any(|p| p.contains("man-pages")),
        "the hidden command has none"
    );
}
