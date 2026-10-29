//! Every tracked path must check out on Windows, macOS and Linux. Part of `cargo xtask check
//! docs`, which the fast lane runs on every PR, docs-only ones included (the full lane's Windows
//! job is skipped for those).
//!
//! Rejected: characters Windows forbids in names (`<>:"|?*`, `\` and control characters), a
//! name ending in a dot or a space, a reserved device name (`CON`, `PRN`, `AUX`, `NUL`,
//! `COM1`–`COM9`, `LPT1`–`LPT9`, with or without an extension), and two paths that differ only
//! in case (they collide on case-insensitive file systems).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};

use crate::tracked::git;

const RESERVED: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

/// Checks every file `git ls-files` lists at `root`.
pub(crate) fn check(root: &Path) -> Result<()> {
    let listing = git(root, &["ls-files", "-z"])?;
    let found = problems(listing.split('\0').filter(|p| !p.is_empty()));
    if !found.is_empty() {
        bail!(
            "{} tracked path(s) can't be checked out everywhere:\n  {}",
            found.len(),
            found.join("\n  ")
        );
    }
    Ok(())
}

/// The problems with `paths` (`/`-separated, as git lists them), one message per problem.
pub(crate) fn problems<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut found = Vec::new();
    // Lower-cased file or directory path → the first spelling seen.
    let mut spellings: BTreeMap<String, &str> = BTreeMap::new();
    for path in paths {
        for name in path.split('/') {
            if let Some(why) = bad_name(name) {
                found.push(format!("{path}: {name:?} {why}"));
            }
        }
        let prefixes = path.match_indices('/').map(|(i, _)| &path[..i]).chain([path]);
        for prefix in prefixes {
            let spelling = spellings.entry(prefix.to_lowercase()).or_insert(prefix);
            if *spelling != prefix {
                found.push(format!("{prefix} and {spelling} differ only in case"));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Why Windows can't create a file or directory called `name`, if it can't.
fn bad_name(name: &str) -> Option<&'static str> {
    if name.chars().any(|c| c.is_control() || "<>:\"|?*\\".contains(c)) {
        return Some("contains a character Windows forbids (<>:\"|?*\\ or a control character)");
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Some("ends with a dot or a space");
    }
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    let upper = stem.to_ascii_uppercase();
    let numbered = ["COM", "LPT"].iter().any(|prefix| {
        upper
            .strip_prefix(prefix)
            .is_some_and(|n| n.len() == 1 && n.bytes().all(|b| (b'1'..=b'9').contains(&b)))
    });
    if RESERVED.contains(&upper.as_str()) || numbered {
        return Some("is a reserved device name on Windows");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::problems;

    #[test]
    fn ordinary_paths_are_fine() {
        let paths = [
            "docs/README.md",
            "crates/lamina-cos/src/lib.rs",
            ".github/workflows/ci.yml",
        ];
        assert_eq!(problems(paths), Vec::<String>::new());
    }

    #[test]
    fn forbidden_characters_are_rejected() {
        for path in [
            "docs/a:b.md",
            "docs/what?.md",
            "docs/a|b.md",
            "docs/\"q\".md",
            "docs/a\u{7}.md",
        ] {
            assert_eq!(problems([path]).len(), 1, "{path}");
        }
    }

    #[test]
    fn trailing_dots_and_spaces_are_rejected() {
        assert_eq!(problems(["docs/notes./a.md", "docs/b.md "]).len(), 2);
    }

    #[test]
    fn reserved_device_names_are_rejected_with_any_extension() {
        for path in [
            "docs/con.md",
            "docs/NUL",
            "docs/aux.tar.gz",
            "docs/com1.txt",
            "docs/lpt9/x.md",
        ] {
            assert_eq!(problems([path]).len(), 1, "{path}");
        }
    }

    #[test]
    fn names_that_only_start_like_device_names_are_fine() {
        assert_eq!(
            problems(["docs/console.md", "docs/com10.md", "docs/null.md", "docs/com0.md"]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn files_differing_only_in_case_collide() {
        assert_eq!(problems(["docs/README.md", "docs/readme.md"]).len(), 1);
    }

    #[test]
    fn directories_differing_only_in_case_collide() {
        assert_eq!(problems(["Docs/a.md", "docs/b.md"]).len(), 1);
    }
}
