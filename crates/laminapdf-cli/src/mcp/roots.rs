//! The folders `laminapdf mcp` may touch ([ADR 0031] §9), and the [`Io`] that keeps every tool
//! inside them.
//!
//! The roots are resolved once at start-up. A path a tool is given is read relative to the first
//! root when it is relative, resolved with every symbolic link followed (`canonicalize`), and
//! used only when the result lies inside a root. An output's folder is resolved the same way and
//! its file name must be a plain name. So `..`, an absolute path elsewhere and a link inside a root
//! that leads out are all `path-outside-roots`. The file is then read, or written, at the resolved
//! path, so what was checked is what is used.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use std::path::{Component, Path, PathBuf};

use laminapdf::ErrorCode;

use crate::exit::CliError;
use crate::io::{Io, ProcessIo, unreadable};

/// The server's root folders, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    roots: Vec<PathBuf>,
}

impl Roots {
    /// The folders `dirs`, each resolved; at least one.
    ///
    /// # Errors
    ///
    /// `invalid-command-line` when there are none, or one isn't a folder that exists.
    pub fn new(dirs: &[PathBuf]) -> Result<Self, CliError> {
        if dirs.is_empty() {
            return Err(CliError::new(
                ErrorCode::InvalidCommandLine,
                "laminapdf mcp needs at least one --root",
            ));
        }
        let roots = dirs
            .iter()
            .map(|dir| match std::fs::canonicalize(dir) {
                Ok(root) if root.is_dir() => Ok(root),
                _ => Err(
                    CliError::new(ErrorCode::InvalidCommandLine, "a --root isn't a folder that exists")
                        .with("argument", dir.to_string_lossy().as_ref()),
                ),
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { roots })
    }

    /// `path` against the first root when it is relative.
    fn join(&self, path: &str) -> PathBuf {
        let path = Path::new(path);
        match self.roots.first() {
            Some(first) if path.is_relative() => first.join(path),
            _ => path.to_owned(),
        }
    }

    fn contains(&self, path: &Path) -> bool {
        self.roots.iter().any(|root| path.starts_with(root))
    }

    /// The resolved input `path`, inside a root.
    ///
    /// # Errors
    ///
    /// `input-unreadable` when it doesn't exist; `path-outside-roots`.
    pub fn input(&self, path: &str) -> Result<PathBuf, CliError> {
        let resolved = std::fs::canonicalize(self.join(path)).map_err(|e| unreadable(path, &e))?;
        if self.contains(&resolved) {
            Ok(resolved)
        } else {
            Err(outside(path))
        }
    }

    /// The output `path`, its folder resolved and inside a root, its name a plain file name.
    ///
    /// # Errors
    ///
    /// `path-outside-roots`, also for a name such as `..`; `output-unwritable` style failures
    /// are left to the write.
    pub fn output(&self, path: &str) -> Result<PathBuf, CliError> {
        let joined = self.join(path);
        let name = match joined.components().next_back() {
            Some(Component::Normal(name)) => name.to_owned(),
            _ => return Err(outside(path)),
        };
        let folder = joined.parent().unwrap_or(Path::new("."));
        let folder = std::fs::canonicalize(folder).map_err(|_| outside(path))?;
        if !self.contains(&folder) {
            return Err(outside(path));
        }
        Ok(folder.join(name))
    }
}

/// `path-outside-roots` for `path`.
fn outside(path: &str) -> CliError {
    CliError::new(
        ErrorCode::PathOutsideRoots,
        "the path is outside the server's --root folders",
    )
    .with("path", path)
}

/// The commands' [`Io`] under `laminapdf mcp`: every path through [`Roots`], and no standard
/// input or output, which carry the protocol.
#[derive(Debug)]
pub struct RootedIo<'a> {
    roots: &'a Roots,
    inner: ProcessIo,
}

impl<'a> RootedIo<'a> {
    /// The I/O of one tool call.
    #[must_use]
    pub fn new(roots: &'a Roots) -> Self {
        Self {
            roots,
            inner: ProcessIo::default(),
        }
    }
}

impl Io for RootedIo<'_> {
    fn read(&mut self, input: &str) -> Result<Vec<u8>, CliError> {
        let path = self.roots.input(input)?;
        std::fs::read(&path).map_err(|e| unreadable(input, &e))
    }

    fn write(&mut self, output: &str, bytes: &[u8], force: bool) -> Result<(), CliError> {
        let path = self.roots.output(output)?;
        self.inner.write(&path.to_string_lossy(), bytes, force)
    }

    fn check(&self, output: &str, force: bool) -> Result<(), CliError> {
        let path = self.roots.output(output)?;
        self.inner.check(&path.to_string_lossy(), force)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A fresh tree: `<tmp>/<name>/root/in.pdf` and `<tmp>/<name>/outside/secret.pdf`.
    fn tree(name: &str) -> (PathBuf, Roots) {
        let base = std::env::temp_dir().join(format!("laminapdf-mcp-roots-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("root/sub")).unwrap();
        fs::create_dir_all(base.join("outside")).unwrap();
        fs::write(base.join("root/in.pdf"), b"in").unwrap();
        fs::write(base.join("outside/secret.pdf"), b"secret").unwrap();
        let roots = Roots::new(&[base.join("root")]).unwrap();
        (base, roots)
    }

    fn code(result: Result<PathBuf, CliError>) -> String {
        result.map_or_else(|e| e.diagnostic().code.clone(), |_| "ok".to_owned())
    }

    #[test]
    fn a_path_inside_a_root_should_resolve() {
        let (base, roots) = tree("inside");
        assert_eq!(
            roots.input("in.pdf").unwrap(),
            fs::canonicalize(base.join("root/in.pdf")).unwrap()
        );
        assert_eq!(code(roots.input("sub/../in.pdf")), "ok");
        assert_eq!(code(roots.output("sub/out.pdf")), "ok");
    }

    #[test]
    fn escapes_should_be_refused() {
        let (base, roots) = tree("escapes");
        let absolute = base.join("outside/secret.pdf");
        assert_eq!(
            [
                code(roots.input("../outside/secret.pdf")),
                code(roots.input(absolute.to_str().unwrap())),
                code(roots.output("../outside/new.pdf")),
                code(roots.output("..")),
                code(roots.output("sub/..")),
            ],
            ["path-outside-roots"; 5]
        );
        assert_eq!(code(roots.input("missing.pdf")), "input-unreadable");
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_leads_out_should_be_refused() {
        let (base, roots) = tree("links");
        std::os::unix::fs::symlink(base.join("outside/secret.pdf"), base.join("root/link.pdf")).unwrap();
        std::os::unix::fs::symlink(base.join("outside"), base.join("root/door")).unwrap();
        assert_eq!(
            [
                code(roots.input("link.pdf")),
                code(roots.input("door/secret.pdf")),
                code(roots.output("door/new.pdf"))
            ],
            ["path-outside-roots"; 3]
        );
    }

    #[test]
    fn the_io_should_never_touch_a_file_outside() {
        let (base, roots) = tree("io");
        let mut io = RootedIo::new(&roots);
        assert!(io.read("../outside/secret.pdf").is_err());
        assert!(io.write("../outside/secret.pdf", b"x", true).is_err());
        assert_eq!(fs::read(base.join("outside/secret.pdf")).unwrap(), b"secret");
        io.write("out.pdf", b"pdf", false).unwrap();
        assert_eq!(fs::read(base.join("root/out.pdf")).unwrap(), b"pdf");
    }

    #[test]
    fn a_root_that_isnt_a_folder_should_be_refused() {
        let (base, _) = tree("bad-root");
        assert!(Roots::new(&[base.join("root/in.pdf")]).is_err());
        assert!(Roots::new(&[]).is_err());
    }
}
