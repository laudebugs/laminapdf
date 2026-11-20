//! The live differential's modes, one per `differential --live` flag, and what each reads. The
//! nightly runs every mode in full; the full lane runs a mode with `--sample` on a pull request
//! that changes its inputs (`cargo xtask changes`' `differential-<name>` flags). The list here is
//! the one the workflow tests hold both lanes to, so a new mode is covered once it is added.

use super::{EXCEPTIONS, forms, inspect, xmp};

/// One kind of live differential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Outcome, page count and outline titles against pdf.js, PDFium, qpdf and pypdf.
    Facts,
    /// `--text`: page text against pdf.js and pdftotext.
    Text,
    /// `--xmp`: XMP values against `exiftool`.
    Xmp,
    /// `--inspect`: objects and fonts against qpdf and pdffonts.
    Inspect,
    /// `--forms`: form fields' names, kinds and values against pypdf and pdf.js.
    Forms,
}

/// What every mode reads: the shared parts of the differential's code and the xtask modules they
/// call (the process runner, the corpus, the workspace and output helpers, `step` in `main.rs`,
/// Python's path in `harness.rs`), the corpus manifest, the pinned tool versions (`doctor.rs`),
/// the build's manifests and toolchain, and the lane and setup that run it.
pub(crate) const SHARED_INPUTS: [&str; 23] = [
    "tools/xtask/src/live.rs",
    "tools/xtask/src/live/mode.rs",
    "tools/xtask/src/live/compare.rs",
    "tools/xtask/src/live/exceptions.rs",
    "tools/xtask/src/live/known.rs",
    "tools/xtask/src/runner.rs",
    "tools/xtask/src/corpus.rs",
    "tools/xtask/src/doctor.rs",
    "tools/xtask/src/util.rs",
    "tools/xtask/src/workspace.rs",
    "tools/xtask/src/harness.rs",
    "tools/xtask/src/main.rs",
    "fixtures/corpus-manifest.toml",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/",
    ".github/workflows/full.yml",
    ".github/actions/apt-fail-fast/",
    ".github/actions/apt-install/",
    ".github/actions/differential-tools/",
    ".github/actions/host-rust/",
    ".github/actions/node-pnpm/",
];

impl Mode {
    pub(crate) const ALL: [Self; 5] = [Self::Facts, Self::Text, Self::Xmp, Self::Inspect, Self::Forms];

    /// Its name, as `cargo xtask changes` prints its flag: `differential-<name>`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Facts => "facts",
            Self::Text => "text",
            Self::Xmp => "xmp",
            Self::Inspect => "inspect",
            Self::Forms => "forms",
        }
    }

    /// The flag that picks it after `--live`; the facts need none.
    pub(crate) fn flag(self) -> Option<&'static str> {
        match self {
            Self::Facts => None,
            Self::Text => Some("--text"),
            Self::Xmp => Some("--xmp"),
            Self::Inspect => Some("--inspect"),
            Self::Forms => Some("--forms"),
        }
    }

    /// The exceptions files it reads, which must exist.
    pub(crate) fn exceptions(self) -> &'static [&'static str] {
        match self {
            Self::Facts | Self::Text => &EXCEPTIONS,
            Self::Xmp => &[xmp::EXCEPTIONS_FILE],
            Self::Inspect => &[inspect::EXCEPTIONS_FILE],
            Self::Forms => &[forms::EXCEPTIONS_FILE],
        }
    }

    /// The crate whose example writes Lamina PDF's side, and the example's name.
    pub(crate) fn example(self) -> (&'static str, &'static str) {
        match self {
            Self::Facts => ("laminapdf-model", "differential-facts"),
            Self::Text => ("laminapdf", "text-facts"),
            Self::Xmp => ("laminapdf-model", "xmp-facts"),
            Self::Inspect => ("laminapdf", "inspect-facts"),
            Self::Forms => ("laminapdf", "form-facts"),
        }
    }

    /// The code, tool runners and committed PDFs only this mode reads, beyond [`SHARED_INPUTS`]:
    /// the facts' mutants come from `fixtures/corpus`, and the XMP, inspection and form
    /// differentials read committed PDFs beside the external corpus.
    pub(crate) fn own_inputs(self) -> &'static [&'static str] {
        match self {
            Self::Facts => &[
                "tools/differential/",
                "tools/corpus-facts/",
                ".github/actions/qpdf/",
                "fixtures/corpus/",
            ],
            Self::Text => &[
                "tools/xtask/src/live/text.rs",
                "tools/differential/",
                ".github/actions/poppler/",
            ],
            Self::Xmp => &[
                "tools/xtask/src/live/xmp.rs",
                "tools/xtask/src/live/xmp/",
                "fixtures/corpus/",
                "fixtures/golden/",
            ],
            Self::Inspect => &[
                "tools/xtask/src/live/inspect.rs",
                // The committed tools' facts `--inspect --update` writes.
                "fixtures/differential/inspect.json",
                ".github/actions/qpdf/",
                ".github/actions/poppler/",
                "fixtures/corpus/",
                "fixtures/golden/",
                "fixtures/signed/",
                "fixtures/attachments/",
                "fixtures/revisions/",
            ],
            Self::Forms => &[
                "tools/xtask/src/live/forms.rs",
                "tools/differential/",
                "fixtures/forms/",
            ],
        }
    }

    /// The `differential --live` arguments that run it, as the workflow tests read the lanes.
    #[cfg(test)]
    pub(crate) fn args(self) -> String {
        self.flag()
            .map_or_else(|| "--live".to_owned(), |flag| format!("--live {flag}"))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Mode, SHARED_INPUTS};
    use crate::workflows::{repository_file, root};

    fn read(path: &str) -> String {
        repository_file(path).unwrap_or_else(|| panic!("can't read {path}"))
    }

    /// The live sources: live.rs and every `.rs` file under live/, as paths from the root.
    fn live_sources() -> Vec<String> {
        let dir = root().join("tools/xtask/src/live");
        let mut found: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| Path::new(name).extension().is_some_and(|e| e == "rs"))
            .map(|name| format!("tools/xtask/src/live/{name}"))
            .collect();
        found.push("tools/xtask/src/live.rs".to_owned());
        found.sort();
        found
    }

    /// Every input of `mode`: shared, its own and its exceptions files.
    fn all_inputs(mode: Mode) -> Vec<&'static str> {
        SHARED_INPUTS
            .into_iter()
            .chain(mode.own_inputs().iter().copied())
            .chain(mode.exceptions().iter().copied())
            .collect()
    }

    /// Whether `path` (a file, or a folder named without its `/`) is in `inputs`.
    fn covered(inputs: &[&str], path: &str) -> bool {
        inputs
            .iter()
            .any(|input| path.starts_with(input) || input.strip_suffix('/') == Some(path))
    }

    /// `a/b/../c` as `a/c`.
    fn normalise(path: &str) -> String {
        let mut parts: Vec<&str> = Vec::new();
        for part in path.split('/') {
            match part {
                ".." => {
                    parts.pop();
                }
                "." | "" => {}
                _ => parts.push(part),
            }
        }
        parts.join("/")
    }

    /// The repository paths `text` (the source at `file`) reads: the xtask module of each
    /// `crate::<name>` (`main.rs` for an item of the crate root, such as `step`), each
    /// `include_str!`, and each path literal under fixtures/, tools/ or .github/ (format strings
    /// and messages, which hold `{` or a space, left out).
    fn named_paths(file: &str, text: &str) -> Vec<String> {
        let ident = |rest: &str| -> String {
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        };
        let mut modules: Vec<String> = Vec::new();
        for rest in text.split("crate::").skip(1) {
            match rest.strip_prefix('{') {
                Some(list) => modules.extend(
                    list.split('}')
                        .next()
                        .unwrap_or_default()
                        .split(',')
                        .map(|m| ident(m.trim())),
                ),
                None => modules.push(ident(rest)),
            }
        }
        let mut found: Vec<String> = modules
            .into_iter()
            .filter(|m| !m.is_empty())
            .map(|m| {
                let module = format!("tools/xtask/src/{m}.rs");
                if root().join(&module).is_file() {
                    module
                } else {
                    "tools/xtask/src/main.rs".to_owned()
                }
            })
            .collect();
        let dir = Path::new(file).parent().and_then(Path::to_str).unwrap_or_default();
        for rest in text.split("include_str!(\"").skip(1) {
            let relative = rest.split('"').next().unwrap_or_default();
            found.push(normalise(&format!("{dir}/{relative}")));
        }
        for literal in text.split('"').skip(1).step_by(2) {
            let rooted = ["fixtures/", "tools/", ".github/"]
                .iter()
                .any(|p| literal.starts_with(p));
            if rooted && !literal.contains(' ') && !literal.contains('{') {
                found.push(literal.to_owned());
            }
        }
        found.sort();
        found.dedup();
        found
    }

    /// Every path a live source reads is an input of a mode that reads that source, so a change
    /// to it runs the sampled differential. What no source names (the setup's actions, the
    /// examples' crates) is listed by hand or comes from `cargo metadata`.
    #[test]
    fn every_path_a_live_source_names_should_be_an_input_of_a_mode_that_reads_it() {
        let mut missing: Vec<String> = Vec::new();
        for file in live_sources().into_iter().filter(|f| !f.ends_with("/mode.rs")) {
            let readers: Vec<Mode> = Mode::ALL
                .into_iter()
                .filter(|&m| covered(&all_inputs(m), &file))
                .collect();
            if readers.is_empty() {
                missing.push(format!("{file} is no mode's input"));
                continue;
            }
            let inputs: Vec<&str> = readers.iter().flat_map(|&m| all_inputs(m)).collect();
            missing.extend(
                named_paths(&file, &read(&file))
                    .into_iter()
                    .filter(|path| !covered(&inputs, path))
                    .map(|path| format!("{file} reads {path}, which no mode reading it lists")),
            );
        }
        assert_eq!(missing, Vec::<String>::new());
    }

    #[test]
    fn named_paths_should_read_modules_includes_and_literals() {
        let text = "use crate::{corpus, step};\nlet x = crate::doctor::qpdf();\n\
                    include_str!(\"../../corpus-facts/facts.py\");\nlet p = \"fixtures/signed\";\n\
                    let m = \"tools/differential and more\";\n";
        assert_eq!(
            named_paths("tools/xtask/src/live.rs", text),
            [
                "fixtures/signed",
                "tools/corpus-facts/facts.py",
                "tools/xtask/src/corpus.rs",
                "tools/xtask/src/doctor.rs",
                "tools/xtask/src/main.rs",
            ]
        );
    }

    /// The example each mode names is the one its code builds and runs.
    #[test]
    fn each_mode_should_name_the_example_its_code_runs() {
        let wrong: Vec<&str> = Mode::ALL
            .into_iter()
            .filter(|mode| {
                let (krate, example) = mode.example();
                let code = match mode {
                    Mode::Facts => read("tools/xtask/src/live.rs"),
                    _ => read(&format!("tools/xtask/src/live/{}.rs", mode.name())),
                };
                let manifest = read(&format!("crates/{krate}/Cargo.toml"));
                let in_crate = manifest.contains(&format!("name = \"{example}\""))
                    || root().join(format!("crates/{krate}/examples/{example}.rs")).is_file();
                !code.contains(&format!("\"{example}\"")) || !in_crate
            })
            .map(Mode::name)
            .collect();
        assert_eq!(wrong, Vec::<&str>::new());
    }

    #[test]
    fn args_should_put_the_flag_after_live() {
        assert_eq!(
            Mode::ALL.map(Mode::args),
            [
                "--live",
                "--live --text",
                "--live --xmp",
                "--live --inspect",
                "--live --forms"
            ]
        );
    }
}
