//! `cargo xtask check docs-prose`: the rules of docs/guidelines/docs-writing.md that a tool can
//! check, over every page a reader of the docs sees: the docs site's pages, the error-code pages
//! it renders (`docs/errors/`) and the package READMEs that `TypeDoc` renders.
//!
//! `cargo xtask check docs` runs it with `--all-strict`, so every report is an error. Run on its
//! own while drafting, a report on a file changed since the merge base with `main` (the working
//! tree and untracked drafts included) is an error, and one on a file nobody has touched is a
//! warning, counted per rule. The ratchet is shared with the examples checks ([`crate::reports`]).

mod extract;
mod rules;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::reports::{Options, Ratchet};
use crate::tracked::Tracked;
use rules::{Config, Scope};
pub(crate) use rules::{is_adr_number, is_todo_id};

/// `cargo xtask check docs-prose [--all-strict] [--warnings] [--json]`.
pub(crate) fn run(root: &Path, options: Options) -> Result<()> {
    let config = load_config(root)?;
    let untracked = Ratchet::untracked(root)?;
    let files = files(root, &untracked)?;
    let mut ratchet = Ratchet::new(root, "docs-prose", options, &untracked);
    for (path, scope) in &files {
        let source = match std::fs::read_to_string(root.join(path)) {
            Ok(source) => source,
            // Deleted in the working tree but not yet from the index: nothing to check.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| format!("reading {path}")),
        };
        ratchet.add(path, rules::check(&extract::parse(&source), *scope, &config));
    }
    ratchet.finish(files.len(), "docs/guidelines/docs-writing.md")
}

/// The words that look like todo IDs but aren't (`id_exceptions` in the config), which
/// `check maintainability`'s todo-ID check skips too.
pub(crate) fn id_exceptions(root: &Path) -> Result<Vec<String>> {
    Ok(load_config(root)?.id_exceptions)
}

fn load_config(root: &Path) -> Result<Config> {
    let path = root.join("tools/xtask/docs-prose.toml");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// The files the check reads, with the rules that apply to each, sorted by path. Untracked files
/// count too, so a new page is checked while it is drafted.
fn files(root: &Path, untracked: &[String]) -> Result<BTreeMap<String, Scope>> {
    let tracked = Tracked::from_git(root)?;
    let paths = tracked
        .files()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .chain(untracked.iter().cloned());
    Ok(paths
        .filter_map(|path| scope(&path).map(|scope| (path, scope)))
        .collect())
}

/// Which rules apply to a path, or `None` when the check doesn't read it.
fn scope(path: &str) -> Option<Scope> {
    if let Some(page) = path.strip_prefix("apps/docs/src/content/docs/") {
        if !crate::util::has_extension(page, &["md", "mdx"]) {
            return None;
        }
        let guide = page.starts_with("guides/") || page.starts_with("recipes/");
        return Some(Scope::SitePage { guide });
    }
    let error_page =
        path.starts_with("docs/errors/") && crate::util::has_extension(path, &["md"]) && !path.ends_with("/README.md");
    let package_readme =
        path.starts_with("packages/") && path.ends_with("/README.md") && path.matches('/').count() == 2;
    (error_page || package_readme).then_some(Scope::Rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_pages_should_be_in_scope_and_guides_marked() {
        assert_eq!(
            scope("apps/docs/src/content/docs/install.mdx"),
            Some(Scope::SitePage { guide: false })
        );
        assert_eq!(
            scope("apps/docs/src/content/docs/guides/limits.mdx"),
            Some(Scope::SitePage { guide: true })
        );
        assert_eq!(
            scope("apps/docs/src/content/docs/recipes/invoice.mdx"),
            Some(Scope::SitePage { guide: true })
        );
    }

    #[test]
    fn rendered_pages_should_be_in_scope() {
        assert_eq!(scope("docs/errors/disposed.md"), Some(Scope::Rendered));
        assert_eq!(scope("packages/sdk/README.md"), Some(Scope::Rendered));
    }

    #[test]
    fn other_files_should_be_out_of_scope() {
        assert_eq!(scope("docs/errors/README.md"), None);
        assert_eq!(scope("packages/sdk/examples/README.md"), None);
        assert_eq!(scope("docs/plans/quality.md"), None);
        assert_eq!(scope("apps/docs/src/content/docs/data.json"), None);
    }

    #[test]
    fn the_repository_config_should_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(load_config(&root).is_ok_and(|config| !config.banned_words.is_empty()));
    }
}
