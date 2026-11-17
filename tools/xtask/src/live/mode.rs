//! The live differential's modes, one per `differential --live` flag, and what each reads. The
//! nightly runs every mode in full; the full lane runs a mode with `--sample` on a pull request
//! that changes its inputs (`cargo xtask changes`' `differential-<name>` flags). The list here is
//! the one the workflow tests hold both lanes to, so a new mode is covered once it is added.

use super::{EXCEPTIONS, inspect, xmp};

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
}

/// What every mode reads: the shared parts of the differential's code, the process runner, the
/// corpus and its manifest, the pinned tool versions (`doctor.rs`), the build's manifests and
/// toolchain, and the lane and setup that run it.
pub(crate) const SHARED_INPUTS: [&str; 16] = [
    "tools/xtask/src/live.rs",
    "tools/xtask/src/live/mode.rs",
    "tools/xtask/src/live/compare.rs",
    "tools/xtask/src/live/exceptions.rs",
    "tools/xtask/src/live/known.rs",
    "tools/xtask/src/runner.rs",
    "tools/xtask/src/corpus.rs",
    "tools/xtask/src/doctor.rs",
    "fixtures/corpus-manifest.toml",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/",
    ".github/workflows/full.yml",
    ".github/actions/differential-tools/",
    ".github/actions/host-rust/",
];

impl Mode {
    pub(crate) const ALL: [Self; 4] = [Self::Facts, Self::Text, Self::Xmp, Self::Inspect];

    /// Its name, as `cargo xtask changes` prints its flag: `differential-<name>`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Facts => "facts",
            Self::Text => "text",
            Self::Xmp => "xmp",
            Self::Inspect => "inspect",
        }
    }

    /// The flag that picks it after `--live`; the facts need none.
    pub(crate) fn flag(self) -> Option<&'static str> {
        match self {
            Self::Facts => None,
            Self::Text => Some("--text"),
            Self::Xmp => Some("--xmp"),
            Self::Inspect => Some("--inspect"),
        }
    }

    /// The exceptions files it reads, which must exist.
    pub(crate) fn exceptions(self) -> &'static [&'static str] {
        match self {
            Self::Facts | Self::Text => &EXCEPTIONS,
            Self::Xmp => &[xmp::EXCEPTIONS_FILE],
            Self::Inspect => &[inspect::EXCEPTIONS_FILE],
        }
    }

    /// The crate whose example writes Lamina PDF's side, and the example's name.
    pub(crate) fn example(self) -> (&'static str, &'static str) {
        match self {
            Self::Facts => ("laminapdf-model", "differential-facts"),
            Self::Text => ("laminapdf", "text-facts"),
            Self::Xmp => ("laminapdf-model", "xmp-facts"),
            Self::Inspect => ("laminapdf", "inspect-facts"),
        }
    }

    /// The code and tool runners only this mode reads, beyond [`SHARED_INPUTS`].
    pub(crate) fn own_inputs(self) -> &'static [&'static str] {
        match self {
            Self::Facts => &["tools/differential/", ".github/actions/qpdf/"],
            Self::Text => &[
                "tools/xtask/src/live/text.rs",
                "tools/differential/",
                ".github/actions/poppler/",
            ],
            Self::Xmp => &["tools/xtask/src/live/xmp.rs", "tools/xtask/src/live/xmp/"],
            Self::Inspect => &[
                "tools/xtask/src/live/inspect.rs",
                ".github/actions/qpdf/",
                ".github/actions/poppler/",
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
    use super::{Mode, SHARED_INPUTS};

    fn repo_file(path: &str) -> String {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        std::fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("can't read {path}: {e}"))
    }

    /// The example each mode names is the one its code builds and runs.
    #[test]
    fn each_mode_should_name_the_example_its_code_runs() {
        let wrong: Vec<&str> = Mode::ALL
            .into_iter()
            .filter(|mode| {
                let (krate, example) = mode.example();
                let code = match mode {
                    Mode::Facts => repo_file("tools/xtask/src/live.rs"),
                    _ => repo_file(&format!("tools/xtask/src/live/{}.rs", mode.name())),
                };
                let manifest = repo_file(&format!("crates/{krate}/Cargo.toml"));
                let in_crate = manifest.contains(&format!("name = \"{example}\""))
                    || std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join(format!("../../crates/{krate}/examples/{example}.rs"))
                        .is_file();
                !code.contains(&format!("\"{example}\"")) || !in_crate
            })
            .map(Mode::name)
            .collect();
        assert_eq!(wrong, Vec::<&str>::new());
    }

    /// Every file of the differential's code is some mode's input, so a change to it runs the
    /// sampled differentials that read it.
    #[test]
    fn every_live_source_should_be_an_input() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/live");
        let inputs: Vec<&str> = SHARED_INPUTS
            .into_iter()
            .chain(Mode::ALL.into_iter().flat_map(|m| m.own_inputs().iter().copied()))
            .collect();
        let missing: Vec<String> = std::fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let dir = if entry.path().is_dir() { "/" } else { "" };
                format!("tools/xtask/src/live/{name}{dir}")
            })
            .filter(|path| !inputs.iter().any(|input| path.starts_with(input)))
            .collect();
        assert_eq!(missing, Vec::<String>::new());
    }

    #[test]
    fn args_should_put_the_flag_after_live() {
        assert_eq!(
            Mode::ALL.map(Mode::args),
            ["--live", "--live --text", "--live --xmp", "--live --inspect"]
        );
    }
}
