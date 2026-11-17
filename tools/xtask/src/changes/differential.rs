//! What each live differential reads, so the full lane runs it with `--sample` on a pull request
//! that changes it (`differential-<mode>` flags; the `differential` flag is any of them). A mode's
//! inputs are its code and tool runners ([`Mode::own_inputs`]), what every mode shares
//! ([`SHARED_INPUTS`]), its exceptions files, and the crate whose example writes Lamina PDF's side
//! with every crate that crate reaches (`cargo metadata`), so a change to the reader runs it.

use anyhow::Result;

use super::graph::{Graph, outermost};
use crate::live::Mode;
use crate::live::mode::SHARED_INPUTS;

/// Every path prefix `mode` reads, sorted, none inside another.
pub(crate) fn inputs(graph: &Graph, mode: Mode) -> Result<Vec<String>> {
    let (krate, _) = mode.example();
    let crates = graph.crate_dirs([krate])?;
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
    use std::path::Path;

    use super::super::graph::Graph;
    use super::touched;

    fn repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
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
        let graph = Graph::load(repo_root()).unwrap();
        assert_eq!(
            [
                "tools/xtask/src/live/text.rs",
                "tools/xtask/src/live/xmp/lamina-sample.json",
                "fixtures/differential/inspect-exceptions.toml",
                "fixtures/differential/xmp-exceptions.toml",
                ".github/actions/poppler/action.yml",
                "tools/differential/facts.py",
            ]
            .map(|path| modes(&graph, &[path])),
            [
                vec!["text"],
                vec!["xmp"],
                vec!["inspect"],
                vec!["xmp"],
                vec!["text", "inspect"],
                vec!["facts", "text"],
            ]
        );
    }

    /// laminapdf-model's examples may use its dev-dependencies, the façade among them, so the
    /// readers' crates reach all four.
    #[test]
    fn the_readers_crates_and_examples_should_run_every_mode() {
        let graph = Graph::load(repo_root()).unwrap();
        assert_eq!(
            [
                "crates/laminapdf-cos/src/lib.rs",
                "crates/laminapdf/examples/differential/inspect-facts.rs",
                "crates/laminapdf-model/examples/xmp-facts.rs",
            ]
            .map(|path| modes(&graph, &[path]).len()),
            [4, 4, 4]
        );
    }

    #[test]
    fn shared_inputs_should_run_every_mode() {
        let graph = Graph::load(repo_root()).unwrap();
        assert_eq!(
            [
                "tools/xtask/src/live.rs",
                "tools/xtask/src/runner.rs",
                ".github/actions/differential-tools/action.yml",
            ]
            .map(|path| modes(&graph, &[path]).len()),
            [4, 4, 4]
        );
    }

    #[test]
    fn docs_apps_and_other_xtask_code_should_run_no_mode() {
        let graph = Graph::load(repo_root()).unwrap();
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
