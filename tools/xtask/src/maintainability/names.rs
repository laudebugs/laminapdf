//! Names with more than one home: a `pub struct` or `pub enum` name defined in two crates, and a
//! name the `lamina` façade re-exports twice. Today's cases are listed in [`DUPLICATES`] with a
//! reason each; the check fails on a new one and on an entry that no longer matches the code.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde::Deserialize;

use super::source::Token;
use super::{DUPLICATES, Problem, Tree, child_module_files, problem};

/// The façade's root module.
const FACADE: &str = "crates/lamina/src/lib.rs";

/// The contents of [`DUPLICATES`].
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Allowed {
    /// Public type names defined in more than one crate.
    #[serde(default, rename = "type")]
    types: Vec<AllowedType>,
    /// Names the façade re-exports more than once.
    #[serde(default)]
    facade: Vec<AllowedFacade>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AllowedType {
    name: String,
    crates: BTreeSet<String>,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AllowedFacade {
    name: String,
    paths: BTreeSet<String>,
    reason: String,
}

/// Where a name is defined or re-exported: file and 1-based line.
#[derive(Debug, Clone)]
struct Site {
    path: String,
    line: usize,
}

/// `pub struct` and `pub enum` names (outside test code) defined in two or more crates.
pub(super) fn duplicate_types(tree: &Tree) -> Result<Vec<Problem>> {
    const RULE: &str = "duplicate-type";
    let allowed: Allowed = tree.parse_config(DUPLICATES)?;
    let test_files = tree.out_of_line_test_files();
    // name → crate → first definition
    let mut defined: BTreeMap<&str, BTreeMap<&str, Site>> = BTreeMap::new();
    for (path, source) in &tree.sources {
        let Some(krate) = crate_of(path) else { continue };
        if test_files.contains(path) {
            continue;
        }
        let tokens = source.tokens();
        for window in tokens.windows(3) {
            let [visibility, keyword, Token::Word(name, _)] = window else {
                continue;
            };
            if visibility.is("pub") && (keyword.is("struct") || keyword.is("enum")) && !source.is_test(keyword.offset())
            {
                let site = Site {
                    path: path.clone(),
                    line: source.line_of(keyword.offset()),
                };
                defined.entry(name).or_default().entry(krate).or_insert(site);
            }
        }
    }
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for (name, crates) in defined.iter().filter(|(_, crates)| crates.len() > 1) {
        seen.insert(*name);
        let found: BTreeSet<String> = crates.keys().map(|krate| (*krate).to_owned()).collect();
        let entry = allowed.types.iter().find(|entry| entry.name == *name);
        if entry.is_some_and(|entry| entry.crates == found) {
            continue;
        }
        let sites: Vec<String> = crates
            .iter()
            .map(|(krate, site)| format!("{krate} ({}:{})", site.path, site.line))
            .collect();
        let message = match entry {
            None => format!(
                "`{name}` is a public type in {}: give it one home in the lowest crate that needs it, or a \
                 name of its own",
                sites.join(", ")
            ),
            Some(entry) => format!(
                "`{name}` is a public type in {}, but {DUPLICATES} lists {}: a duplicate may only lose crates; \
                 update the entry",
                sites.join(", "),
                entry.crates.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        };
        let first = crates.values().next().cloned().unwrap_or(Site {
            path: DUPLICATES.to_owned(),
            line: 0,
        });
        problems.push(problem(&first.path, first.line, RULE, message));
    }
    for entry in &allowed.types {
        if !seen.contains(entry.name.as_str()) {
            problems.push(problem(
                DUPLICATES,
                0,
                RULE,
                format!(
                    "`{}` is no longer a public type in two crates: remove its entry",
                    entry.name
                ),
            ));
        }
        if entry.reason.trim().is_empty() {
            problems.push(problem(DUPLICATES, 0, RULE, format!("`{}` needs a reason", entry.name)));
        }
    }
    Ok(problems)
}

/// The crate a library or binary source belongs to: `crates/<crate>/src/…`.
fn crate_of(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("crates/")?;
    let (krate, inside) = rest.split_once('/')?;
    inside.starts_with("src/").then_some(krate)
}

/// Names the `lamina` façade re-exports more than once, across its public modules.
pub(super) fn facade_collisions(tree: &Tree) -> Result<Vec<Problem>> {
    const RULE: &str = "facade-collision";
    let allowed: Allowed = tree.parse_config(DUPLICATES)?;
    let mut exports: BTreeMap<String, Vec<(String, Site)>> = BTreeMap::new();
    let mut modules = vec![(FACADE.to_owned(), "lamina".to_owned())];
    while let Some((path, module)) = modules.pop() {
        let Some(source) = tree.sources.get(&path) else {
            continue;
        };
        let found = public_items(source);
        for (name, offset) in found.reexports {
            let site = Site {
                path: path.clone(),
                line: source.line_of(offset),
            };
            exports
                .entry(name.clone())
                .or_default()
                .push((format!("{module}::{name}"), site));
        }
        for child in found.modules {
            let file = child_module_files(&path, &child)
                .into_iter()
                .find(|file| tree.sources.contains_key(file));
            if let Some(file) = file {
                modules.push((file, format!("{module}::{child}")));
            }
        }
    }
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for (name, sites) in exports.iter().filter(|(_, sites)| sites.len() > 1) {
        seen.insert(name.as_str());
        let paths: BTreeSet<String> = sites.iter().map(|(path, _)| path.clone()).collect();
        if allowed
            .facade
            .iter()
            .any(|entry| entry.name == *name && entry.paths == paths)
        {
            continue;
        }
        let listed: Vec<String> = sites
            .iter()
            .map(|(path, site)| format!("{path} ({}:{})", site.path, site.line))
            .collect();
        let message = format!(
            "the façade re-exports `{name}` more than once: {}; each public item has one public path, so give \
             one of them another name",
            listed.join(", ")
        );
        let Some((_, first)) = sites.iter().min_by_key(|(_, site)| (&site.path, site.line)) else {
            continue;
        };
        problems.push(problem(&first.path, first.line, RULE, message));
    }
    for entry in &allowed.facade {
        if !seen.contains(entry.name.as_str()) {
            problems.push(problem(
                DUPLICATES,
                0,
                RULE,
                format!(
                    "the façade no longer re-exports `{}` twice: remove its entry",
                    entry.name
                ),
            ));
        }
        if entry.reason.trim().is_empty() {
            problems.push(problem(DUPLICATES, 0, RULE, format!("`{}` needs a reason", entry.name)));
        }
    }
    Ok(problems)
}

/// What one façade module makes public at its top level.
#[derive(Debug, Default)]
struct PublicItems {
    /// Each name a `pub use` exports, with the byte offset of its `use`.
    reexports: Vec<(String, usize)>,
    /// The `pub mod name;` modules (inline `pub mod name { … }` are not followed).
    modules: Vec<String>,
}

fn public_items(source: &super::source::Source) -> PublicItems {
    let tokens = source.tokens();
    let mut items = PublicItems::default();
    let mut depth = 0usize;
    let mut index = 0;
    while let Some(&token) = tokens.get(index) {
        index += 1;
        if token.is_punct(b'{') {
            depth += 1;
        } else if token.is_punct(b'}') {
            depth = depth.saturating_sub(1);
        }
        if depth > 0 || !token.is("pub") || source.is_test(token.offset()) {
            continue;
        }
        match tokens.get(index..) {
            Some([keyword, Token::Word(name, _), semicolon, ..]) if keyword.is("mod") && semicolon.is_punct(b';') => {
                items.modules.push((*name).to_owned());
            }
            Some([keyword, rest @ ..]) if keyword.is("use") => {
                let end = rest.iter().position(|token| token.is_punct(b';')).unwrap_or(rest.len());
                let tree = rest.get(..end).unwrap_or_default();
                let mut names = Vec::new();
                use_tree(tree, &mut 0, None, &mut names);
                items
                    .reexports
                    .extend(names.into_iter().map(|name| (name, keyword.offset())));
                index += end;
            }
            _ => {}
        }
    }
    items
}

/// Collects the names a use tree (`a::b::{C, D as E, f::{self, G}}`, `*` contributes none)
/// brings in, reading from `tokens[*at]`; `parent` is the last path segment before a brace.
fn use_tree(tokens: &[Token<'_>], at: &mut usize, parent: Option<&str>, names: &mut Vec<String>) {
    let mut last = parent.map(str::to_owned);
    while let Some(&token) = tokens.get(*at) {
        *at += 1;
        match token {
            Token::Punct(b':', _) => {}
            Token::Punct(b'{', _) => {
                loop {
                    use_tree(tokens, at, last.as_deref(), names);
                    match tokens.get(*at) {
                        Some(token) if token.is_punct(b',') => *at += 1,
                        Some(token) if token.is_punct(b'}') => {
                            *at += 1;
                            return;
                        }
                        _ => return,
                    }
                    // A trailing comma: `{A, B,}`.
                    if tokens.get(*at).is_some_and(|token| token.is_punct(b'}')) {
                        *at += 1;
                        return;
                    }
                }
            }
            Token::Word(word, _) => {
                let next = tokens.get(*at).copied();
                if next.is_some_and(|token| token.is_punct(b':')) {
                    last = Some(word.to_owned());
                    continue;
                }
                if next.is_some_and(|token| token.is("as")) {
                    *at += 1;
                    if let Some(Token::Word(alias, _)) = tokens.get(*at) {
                        *at += 1;
                        if *alias != "_" {
                            names.push((*alias).to_owned());
                        }
                    }
                    return;
                }
                let name = if word == "self" {
                    last.take()
                } else {
                    Some(word.to_owned())
                };
                names.extend(name);
                return;
            }
            // `*` (a glob brings in no name we can see), or the end of a list.
            Token::Punct(..) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::source::Source;
    use super::*;

    fn names_of(statement: &str) -> Vec<String> {
        let items = public_items(&Source::new(statement));
        items.reexports.into_iter().map(|(name, _)| name).collect()
    }

    #[test]
    fn a_use_tree_should_yield_its_names_and_aliases() {
        let names = names_of("pub use a::b::{C, D as E, f::{self, G}, h::*, I as _,};");
        assert_eq!(names, ["C", "E", "f", "G"]);
    }

    #[test]
    fn a_single_path_and_an_alias_should_yield_one_name_each() {
        assert_eq!(
            names_of("pub use lamina_cos::Error as CosError;\npub use job::CancelToken;"),
            ["CosError", "CancelToken"]
        );
    }

    #[test]
    fn crate_private_and_nested_uses_should_not_count() {
        let names = names_of("pub(crate) use a::B;\nmod inner {\n    pub use a::C;\n}\n#[cfg(test)]\npub use a::D;\n");
        assert_eq!(names, Vec::<String>::new());
    }

    fn type_messages(files: &[(&str, &str)]) -> Vec<String> {
        duplicate_types(&Tree::from_files(files))
            .unwrap()
            .into_iter()
            .map(|p| format!("{}:{}: {}", p.path, p.finding.line, p.finding.message))
            .collect()
    }

    #[test]
    fn a_new_duplicate_type_should_fail() {
        let messages = type_messages(&[
            ("crates/a/src/lib.rs", "pub struct Matrix;\n"),
            ("crates/b/src/geometry.rs", "//! Geometry.\npub enum Matrix {}\n"),
        ]);
        assert!(
            messages.len() == 1 && messages[0].contains("a (crates/a/src/lib.rs:1), b (crates/b/src/geometry.rs:2)")
        );
    }

    #[test]
    fn an_allowed_duplicate_type_should_pass() {
        let allowed =
            "[[type]]\nname = \"Matrix\"\ncrates = [\"a\", \"b\"]\nreason = \"two orders; one todo merges them\"\n";
        let messages = type_messages(&[
            ("crates/a/src/lib.rs", "pub struct Matrix;\n"),
            ("crates/b/src/lib.rs", "pub struct Matrix;\n"),
            (DUPLICATES, allowed),
        ]);
        assert_eq!(messages, Vec::<String>::new());
    }

    #[test]
    fn a_duplicate_in_a_third_crate_should_fail() {
        let allowed = "[[type]]\nname = \"Matrix\"\ncrates = [\"a\", \"b\"]\nreason = \"x\"\n";
        let files = [
            ("crates/a/src/lib.rs", "pub struct Matrix;\n"),
            ("crates/b/src/lib.rs", "pub struct Matrix;\n"),
            ("crates/c/src/lib.rs", "pub struct Matrix;\n"),
            (DUPLICATES, allowed),
        ];
        assert_eq!(type_messages(&files).len(), 1);
    }

    #[test]
    fn a_stale_allowed_duplicate_should_fail() {
        let allowed = "[[type]]\nname = \"Matrix\"\ncrates = [\"a\", \"b\"]\nreason = \"x\"\n";
        let messages = type_messages(&[("crates/a/src/lib.rs", "pub struct Matrix;\n"), (DUPLICATES, allowed)]);
        assert!(messages.len() == 1 && messages[0].contains("remove its entry"));
    }

    #[test]
    fn private_test_and_non_crate_types_should_not_count() {
        let messages = type_messages(&[
            (
                "crates/a/src/lib.rs",
                "pub(crate) struct A;\n#[cfg(test)]\nmod tests {\n    pub struct B;\n}\n",
            ),
            (
                "crates/b/src/lib.rs",
                "pub struct A;\npub struct B;\n// pub struct C;\n",
            ),
            ("crates/b/tests/it.rs", "pub struct C;\n"),
            ("tools/xtask/src/main.rs", "pub struct C;\n"),
            ("crates/c/src/lib.rs", "const S: &str = \"pub struct C\";\n"),
        ]);
        assert_eq!(messages, Vec::<String>::new());
    }

    fn facade_messages(files: &[(&str, &str)]) -> Vec<String> {
        facade_collisions(&Tree::from_files(files))
            .unwrap()
            .into_iter()
            .map(|p| p.finding.message)
            .collect()
    }

    const LIB: &str = "mod job;\npub mod extract;\npub use job::CancelToken;\npub use lamina_build::TextRun;\n";

    #[test]
    fn a_name_re_exported_in_two_public_modules_should_fail() {
        let messages = facade_messages(&[
            (FACADE, LIB),
            (
                "crates/lamina/src/extract.rs",
                "pub use lamina_text::extract::{Glyph, TextRun};\n",
            ),
        ]);
        assert!(
            messages.len() == 1
                && messages[0].contains("lamina::TextRun")
                && messages[0].contains("lamina::extract::TextRun")
        );
    }

    #[test]
    fn a_re_export_inside_a_private_module_should_not_count() {
        let messages = facade_messages(&[
            (FACADE, LIB),
            ("crates/lamina/src/job.rs", "pub use lamina_syntax::CancelToken;\n"),
        ]);
        assert_eq!(messages, Vec::<String>::new());
    }

    #[test]
    fn an_allowed_facade_collision_should_pass() {
        let allowed = "[[facade]]\nname = \"TextRun\"\npaths = [\"lamina::TextRun\", \"lamina::extract::TextRun\"]\nreason = \"x\"\n";
        let messages = facade_messages(&[
            (FACADE, LIB),
            (
                "crates/lamina/src/extract.rs",
                "pub use lamina_text::extract::TextRun;\n",
            ),
            (DUPLICATES, allowed),
        ]);
        assert_eq!(messages, Vec::<String>::new());
    }

    #[test]
    fn a_stale_facade_entry_should_fail() {
        let allowed = "[[facade]]\nname = \"TextRun\"\npaths = [\"lamina::TextRun\", \"lamina::extract::TextRun\"]\nreason = \"x\"\n";
        assert_eq!(facade_messages(&[(FACADE, LIB), (DUPLICATES, allowed)]).len(), 1);
    }
}
