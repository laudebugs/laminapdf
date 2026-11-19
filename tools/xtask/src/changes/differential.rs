//! What each live differential reads, so the full lane runs it with `--sample` on a pull request
//! that changes it (`differential-<mode>` flags; the `differential` flag is any of them). A mode's
//! inputs are its code and tool runners ([`Mode::own_inputs`]), what every mode shares
//! ([`SHARED_INPUTS`]), its exceptions files, and the crate whose example writes Lamina PDF's side
//! with every crate it reaches through normal and build dependencies (`cargo metadata`), so a
//! change to the reader runs it. Dev-dependencies are left out: a test can reach far (the
//! façade comes into laminapdf-model's through laminapdf-save), but the examples use only what
//! the library builds from, which a test below checks.

use anyhow::Result;

use super::graph::{Graph, outermost};
use crate::live::Mode;
use crate::live::mode::SHARED_INPUTS;

/// Every path prefix `mode` reads, sorted, none inside another.
pub(crate) fn inputs(graph: &Graph, mode: Mode) -> Result<Vec<String>> {
    let (krate, _) = mode.example();
    let crates = graph.runtime_crate_dirs([krate])?;
    let fixed = SHARED_INPUTS
        .iter()
        .chain(mode.own_inputs())
        .chain(mode.exceptions())
        .map(|&path| path.to_owned());
    Ok(outermost(fixed.chain(crates)))
}

/// Each mode `paths` reach, as `(name, touched)`, in [`Mode::ALL`]'s order.
pub(crate) fn touched(graph: &Graph, paths: &[&str]) -> Result<Vec<(&'static str, bool)>> {
    Mode::ALL
        .into_iter()
        .map(|mode| Ok((mode.name(), super::graph::touches(paths, &inputs(graph, mode)?))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::graph::Graph;
    use super::touched;
    use crate::live::Mode;
    use crate::workflows::{repository_file, root};

    fn graph() -> Graph {
        Graph::load(&root()).unwrap()
    }

    /// The modes `paths` reach, by name.
    fn modes(graph: &Graph, paths: &[&str]) -> Vec<&'static str> {
        touched(graph, paths)
            .unwrap()
            .into_iter()
            .filter_map(|(name, hit)| hit.then_some(name))
            .collect()
    }

    #[test]
    fn each_modes_own_code_and_exceptions_should_run_only_that_mode() {
        let graph = graph();
        assert_eq!(
            [
                "tools/xtask/src/live/text.rs",
                "tools/xtask/src/live/xmp/lamina-sample.json",
                "fixtures/differential/inspect-exceptions.toml",
                "fixtures/differential/xmp-exceptions.toml",
                ".github/actions/poppler/action.yml",
                "tools/differential/facts.py",
                "tools/xtask/src/live/forms.rs",
                "fixtures/differential/forms-exceptions.toml",
                "fixtures/forms/fields.pdf",
            ]
            .map(|path| modes(&graph, &[path])),
            [
                vec!["text"],
                vec!["xmp"],
                vec!["inspect"],
                vec!["xmp"],
                vec!["text", "inspect"],
                vec!["facts", "text", "forms"],
                vec!["forms"],
                vec!["forms"],
                vec!["forms"],
            ]
        );
    }

    #[test]
    fn the_readers_crates_should_run_the_modes_whose_examples_build_from_them() {
        let graph = graph();
        assert_eq!(
            [
                "crates/laminapdf-cos/src/lib.rs",
                "crates/laminapdf/examples/differential/inspect-facts.rs",
                "crates/laminapdf-model/examples/xmp-facts.rs",
            ]
            .map(|path| modes(&graph, &[path])),
            [
                vec!["facts", "text", "xmp", "inspect", "forms"],
                vec!["text", "inspect", "forms"],
                vec!["facts", "text", "xmp", "inspect", "forms"],
            ]
        );
    }

    /// The Lamina PDF crates an example's source names (`laminapdf_model::…` as
    /// `laminapdf-model`).
    fn crates_named(source: &str) -> Vec<String> {
        let mut found: Vec<String> = source
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
            .filter_map(|word| word.split_once("::").map(|(head, _)| head))
            .filter(|head| *head == "laminapdf" || head.starts_with("laminapdf_"))
            .map(|head| head.replace('_', "-"))
            .collect();
        found.sort();
        found.dedup();
        found
    }

    /// Each mode's example uses only crates its own crate builds from, so leaving out
    /// dev-dependencies loses none of its inputs.
    #[test]
    fn each_example_should_use_only_its_crates_normal_dependencies() {
        let graph = graph();
        let mut outside: Vec<String> = Vec::new();
        for mode in Mode::ALL {
            let (krate, example) = mode.example();
            let source = [
                format!("crates/{krate}/examples/{example}.rs"),
                format!("crates/{krate}/examples/differential/{example}.rs"),
            ]
            .iter()
            .find_map(|path| repository_file(path))
            .unwrap_or_else(|| panic!("no source for {krate}'s {example}"));
            let reach = graph.runtime_crate_dirs([krate]).unwrap();
            for used in crates_named(&source) {
                if !graph.runtime_crate_dirs([used.as_str()]).unwrap().is_subset(&reach) {
                    outside.push(format!("{example} uses {used}, outside {krate}'s normal dependencies"));
                }
            }
        }
        assert_eq!(outside, Vec::<String>::new());
    }

    #[test]
    fn crates_named_should_read_paths_and_uses() {
        assert_eq!(
            crates_named("use laminapdf_model::Document;\nlet x = laminapdf::open(); serde_json::json!();"),
            ["laminapdf", "laminapdf-model"]
        );
    }

    #[test]
    fn shared_inputs_should_run_every_mode() {
        let graph = graph();
        assert_eq!(
            [
                "tools/xtask/src/live.rs",
                "tools/xtask/src/runner.rs",
                ".github/actions/differential-tools/action.yml",
            ]
            .map(|path| modes(&graph, &[path]).len()),
            [5, 5, 5]
        );
    }

    #[test]
    fn docs_apps_and_other_xtask_code_should_run_no_mode() {
        let graph = graph();
        assert_eq!(
            modes(
                &graph,
                &[
                    "docs/plans/architecture.md",
                    "apps/web/src/app.tsx",
                    "packages/viewer/src/index.ts",
                    "tools/xtask/src/gallery.rs",
                    "fixtures/differential/text-exceptions.json",
                    ".github/workflows/nightly.yml",
                ]
            ),
            Vec::<&str>::new()
        );
    }
}
