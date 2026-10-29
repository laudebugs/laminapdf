//! The files git tracks, which is what GitHub serves: the docs checks resolve links against this
//! set (case-sensitively, unlike macOS's default file system) and never see untracked drafts.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Tracked files and the directories that contain them, relative to the repository root.
#[derive(Debug, Default)]
pub(crate) struct Tracked {
    files: BTreeSet<PathBuf>,
    dirs: BTreeSet<PathBuf>,
}

impl Tracked {
    /// `git ls-files` at `root`.
    pub(crate) fn from_git(root: &Path) -> Result<Self> {
        let listing = git(root, &["ls-files", "-z"])?;
        Ok(Self::from_paths(listing.split('\0').filter(|path| !path.is_empty())))
    }

    /// The set made of these `/`-separated file paths.
    pub(crate) fn from_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> Self {
        let mut tracked = Self::default();
        for path in paths {
            let path = PathBuf::from(path);
            tracked.dirs.extend(
                path.ancestors()
                    .skip(1)
                    .filter(|dir| !dir.as_os_str().is_empty())
                    .map(Path::to_path_buf),
            );
            tracked.files.insert(path);
        }
        tracked
    }

    /// Every tracked file, sorted.
    pub(crate) fn files(&self) -> impl Iterator<Item = &Path> {
        self.files.iter().map(PathBuf::as_path)
    }

    pub(crate) fn is_file(&self, path: &Path) -> bool {
        self.files.contains(path)
    }

    /// A tracked file, a directory holding one, or the root itself (the empty path).
    pub(crate) fn exists(&self, path: &Path) -> bool {
        path.as_os_str().is_empty() || self.files.contains(path) || self.dirs.contains(path)
    }
}

/// Runs `git args…` in `root` and returns its standard output; fails with git's message.
pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("failed to run git; is it installed?")?;
    if !output.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).with_context(|| format!("`git {}` printed non-UTF-8 output", args.join(" ")))
}

/// A scratch directory for tests, removed on drop.
#[cfg(test)]
pub(crate) struct TempTree(pub(crate) PathBuf);

#[cfg(test)]
impl TempTree {
    /// Writes `files` (path, contents) under a fresh directory named after `name`.
    pub(crate) fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("lamina-xtask-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Self(root)
    }
}

#[cfg(test)]
impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Tracked;

    #[test]
    fn parent_directories_of_tracked_files_exist() {
        let tracked = Tracked::from_paths(["docs/todos/done/.gitkeep"]);
        assert!(tracked.exists(Path::new("docs/todos/done")) && tracked.exists(Path::new("docs")));
    }

    #[test]
    fn lookups_are_case_sensitive() {
        let tracked = Tracked::from_paths(["docs/README.md"]);
        assert!(!tracked.exists(Path::new("docs/readme.md")));
    }

    #[test]
    fn untracked_paths_do_not_exist() {
        let tracked = Tracked::from_paths(["a.md"]);
        assert!(!tracked.exists(Path::new("b.md")));
    }
}
