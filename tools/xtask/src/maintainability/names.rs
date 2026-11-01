//! Names with more than one home: a `pub struct` or `pub enum` name defined in two crates, and a
//! name the `lamina` façade makes public twice. Today's cases are listed in [`DUPLICATES`] with a
//! reason each; the check fails on a new one and on an entry that no longer matches the code, and
//! warns about an entry the merge base with `main` doesn't have, for a reviewer to look at.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Deserialize;

use super::source::{Source, Token};
use super::{Base, DUPLICATES, Problem, Tree, child_module_files, problem, warning};

/// The façade's root module.
const FACADE: &str = "crates/lamina/src/lib.rs";

/// The contents of [`DUPLICATES`].
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Allowed {
    /// Public type names defined in more than one crate.
    #[serde(default, rename = "type")]
    types: Vec<Entry>,
    /// Names the façade makes public more than once.
    #[serde(default)]
    facade: Vec<Entry>,
}

/// One accepted duplicate: its name, its homes (the crates that define a type, or the façade's
/// public paths of a name) and why it exists.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    #[serde(rename = "crates", alias = "paths")]
    homes: BTreeSet<String>,
    reason: String,
}

/// Where a name is defined or made public: file and 1-based line.
#[derive(Debug, Clone)]
struct Site {
    path: String,
    line: usize,
}

/// Name → home → where. A name with two or more homes is a duplicate.
type Homes = BTreeMap<String, BTreeMap<String, Site>>;

/// One kind of duplicate: how the check names it and what it tells the reader.
struct Kind {
    rule: &'static str,
    /// The message for a duplicate not listed (or listed with other homes): name, homes found.
    unlisted: fn(&str, &str) -> String,
    /// The message for an entry the code no longer needs.
    stale: fn(&str) -> String,
}

/// Compares the duplicates `found` with the `entries` that accept them.
fn compare(found: &Homes, entries: &[Entry], kind: &Kind) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (name, homes) in found.iter().filter(|(_, homes)| homes.len() > 1) {
        let keys: BTreeSet<String> = homes.keys().cloned().collect();
        let entry = entries.iter().find(|entry| entry.name == *name);
        if entry.is_some_and(|entry| entry.homes == keys) {
            continue;
        }
        let sites: Vec<String> = homes
            .iter()
            .map(|(home, site)| format!("{home} ({}:{})", site.path, site.line))
            .collect();
        let mut message = (kind.unlisted)(name, &sites.join(", "));
        if let Some(entry) = entry {
            let listed: Vec<&str> = entry.homes.iter().map(String::as_str).collect();
            let _ = write!(
                message,
                "; {DUPLICATES} lists {}: a duplicate may only lose homes",
                listed.join(", ")
            );
        }
        let Some(first) = homes.values().min_by_key(|site| (&site.path, site.line)) else {
            continue;
        };
        problems.push(problem(&first.path, first.line, kind.rule, message));
    }
    for entry in entries {
        if found.get(&entry.name).is_none_or(|homes| homes.len() <= 1) {
            problems.push(problem(DUPLICATES, 0, kind.rule, (kind.stale)(&entry.name)));
        }
        if entry.reason.trim().is_empty() {
            problems.push(problem(
                DUPLICATES,
                0,
                kind.rule,
                format!("`{}` needs a reason", entry.name),
            ));
        }
    }
    problems
}

const TYPES: Kind = Kind {
    rule: "duplicate-type",
    unlisted: |name, sites| {
        format!(
            "`{name}` is a public type in {sites}: give it one home in the lowest crate that needs it, or a name of its own"
        )
    },
    stale: |name| format!("`{name}` is no longer a public type in two crates: remove its entry"),
};

const FACADE_NAMES: Kind = Kind {
    rule: "facade-collision",
    unlisted: |name, sites| {
        format!(
            "the façade makes `{name}` public more than once: {sites}; each public item has one public path, so \
             give one of them another name"
        )
    },
    stale: |name| format!("the façade no longer makes `{name}` public twice: remove its entry"),
};

/// `pub struct` and `pub enum` names (outside test code) defined in two or more crates.
pub(super) fn duplicate_types(tree: &Tree) -> Vec<Problem> {
    let test_files = tree.out_of_line_test_files();
    let mut found = Homes::new();
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
                found
                    .entry((*name).to_owned())
                    .or_default()
                    .entry(krate.to_owned())
                    .or_insert(site);
            }
        }
    }
    compare(&found, &tree.allowed.types, &TYPES)
}

/// The crate a library or binary source belongs to: `crates/<crate>/src/…`.
fn crate_of(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("crates/")?;
    let (krate, inside) = rest.split_once('/')?;
    inside.starts_with("src/").then_some(krate)
}

/// Names the `lamina` façade makes public more than once across its public modules, by a
/// `pub use` or by defining a public item.
pub(super) fn facade_collisions(tree: &Tree) -> Vec<Problem> {
    let mut found = Homes::new();
    let mut modules = vec![(FACADE.to_owned(), "lamina".to_owned())];
    while let Some((path, module)) = modules.pop() {
        let Some(source) = tree.sources.get(&path) else {
            continue;
        };
        let items = public_items(source);
        for (name, offset) in items.names {
            let site = Site {
                path: path.clone(),
                line: source.line_of(offset),
            };
            found
                .entry(name.clone())
                .or_default()
                .insert(format!("{module}::{name}"), site);
        }
        for child in items.modules {
            let file = child_module_files(&path, &child)
                .into_iter()
                .find(|file| tree.sources.contains_key(file));
            if let Some(file) = file {
                modules.push((file, format!("{module}::{child}")));
            }
        }
    }
    compare(&found, &tree.allowed.facade, &FACADE_NAMES)
}

/// The allow-list against its version at the merge base with `main`: a new entry, or one with
/// more homes, is a warning. New duplicates are sometimes right (two crates' `Error`), so a
/// reviewer decides, but the warning makes sure one sees it.
pub(super) fn allow_list_shrinks(tree: &Tree) -> Vec<Problem> {
    const RULE: &str = "allow-list-shrinks";
    let Base::Records {
        allowed: Some(base), ..
    } = &tree.base
    else {
        return Vec::new();
    };
    let mut problems = Vec::new();
    for (current, before, kind) in [
        (&tree.allowed.types, &base.types, "type"),
        (&tree.allowed.facade, &base.facade, "facade"),
    ] {
        for entry in current {
            let message = match before.iter().find(|old| old.name == entry.name) {
                None => format!(
                    "a new [[{kind}]] entry for `{}`: a reviewer must agree to the new duplicate",
                    entry.name
                ),
                Some(old) if !entry.homes.is_subset(&old.homes) => format!(
                    "the [[{kind}]] entry for `{}` gains homes: a reviewer must agree to the new duplicate",
                    entry.name
                ),
                Some(_) => continue,
            };
            problems.push(warning(DUPLICATES, RULE, message));
        }
    }
    problems
}

/// What one façade module makes public at its top level.
#[derive(Debug, Default)]
struct PublicItems {
    /// Each name a `pub use` exports or a `pub` item defines, with the byte offset of its keyword.
    names: Vec<(String, usize)>,
    /// The `pub mod name;` modules. Inline `pub mod name { … }` modules and glob re-exports
    /// (`pub use a::*`) are not followed.
    modules: Vec<String>,
}

/// Item keywords whose next word is the item's name.
const NAMED_ITEMS: [&str; 7] = ["struct", "enum", "trait", "type", "union", "fn", "static"];

fn public_items(source: &Source) -> PublicItems {
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
                    .names
                    .extend(names.into_iter().map(|name| (name, keyword.offset())));
                index += end;
            }
            Some(rest) => items.names.extend(defined_name(rest)),
            None => {}
        }
    }
    items
}

/// The name a `pub` item defines, from the tokens after `pub`: `struct A`, `fn f`, `static mut S`,
/// `const C`, `const fn f`, `unsafe fn f`, `async fn f` (with the keyword's offset).
fn defined_name(rest: &[Token<'_>]) -> Option<(String, usize)> {
    let mut at = 0;
    loop {
        let keyword = *rest.get(at)?;
        let next = rest.get(at + 1).copied();
        let modifier = keyword.is("unsafe")
            || keyword.is("async")
            || (keyword.is("const") && next.is_some_and(|next| next.is("fn") || next.is("unsafe") || next.is("async")));
        if modifier {
            at += 1;
            continue;
        }
        let mut name_at = at + 1;
        if keyword.is("static") && next.is_some_and(|next| next.is("mut")) {
            name_at += 1;
        }
        let named = NAMED_ITEMS.iter().any(|item| keyword.is(item)) || keyword.is("const");
        return match rest.get(name_at) {
            Some(Token::Word(name, _)) if named => Some(((*name).to_owned(), keyword.offset())),
            _ => None,
        };
    }
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
        items.names.into_iter().map(|(name, _)| name).collect()
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
    fn public_items_should_yield_their_names() {
        let text = "pub struct A;\npub const fn b() {}\npub static mut C: u8 = 0;\npub const D: u8 = 0;\n\
                    pub unsafe fn e() {}\npub(crate) fn f() {}\nimpl A {\n    pub fn g() {}\n}\n";
        assert_eq!(names_of(text), ["A", "b", "C", "D", "e"]);
    }

    #[test]
    fn crate_private_and_nested_uses_should_not_count() {
        let names = names_of("pub(crate) use a::B;\nmod inner {\n    pub use a::C;\n}\n#[cfg(test)]\npub use a::D;\n");
        assert_eq!(names, Vec::<String>::new());
    }

    fn type_messages(files: &[(&str, &str)]) -> Vec<String> {
        duplicate_types(&Tree::from_files(files))
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
    fn a_facade_item_named_like_a_re_export_should_fail() {
        let messages = facade_messages(&[(FACADE, LIB), ("crates/lamina/src/extract.rs", "pub struct TextRun;\n")]);
        assert_eq!(messages.len(), 1);
    }

    fn shrink_messages(allowed: &str, base: &str) -> Vec<String> {
        let tree = Tree::from_files(&[(DUPLICATES, allowed)]).with_base("", base);
        allow_list_shrinks(&tree)
            .into_iter()
            .map(|p| p.finding.message)
            .collect()
    }

    const MATRIX_AB: &str = "[[type]]\nname = \"Matrix\"\ncrates = [\"a\", \"b\"]\nreason = \"x\"\n";

    #[test]
    fn a_new_allow_list_entry_should_warn_against_the_merge_base() {
        let messages = shrink_messages(MATRIX_AB, "");
        assert!(messages.len() == 1 && messages[0].contains("a new [[type]] entry for `Matrix`"));
    }

    #[test]
    fn an_allow_list_entry_with_more_homes_should_warn_against_the_merge_base() {
        let wider = MATRIX_AB.replace("[\"a\", \"b\"]", "[\"a\", \"b\", \"c\"]");
        assert_eq!(shrink_messages(&wider, MATRIX_AB).len(), 1);
    }

    #[test]
    fn an_unchanged_allow_list_should_not_warn() {
        assert_eq!(shrink_messages(MATRIX_AB, MATRIX_AB), Vec::<String>::new());
    }

    #[test]
    fn allow_list_warnings_should_never_fail_the_check() {
        let tree = Tree::from_files(&[(DUPLICATES, "")]).with_base("", "");
        let tree = Tree {
            allowed: toml::from_str(MATRIX_AB).unwrap(),
            ..tree
        };
        assert!(allow_list_shrinks(&tree).iter().all(|problem| problem.warning));
    }

    #[test]
    fn a_stale_facade_entry_should_fail() {
        let allowed = "[[facade]]\nname = \"TextRun\"\npaths = [\"lamina::TextRun\", \"lamina::extract::TextRun\"]\nreason = \"x\"\n";
        assert_eq!(facade_messages(&[(FACADE, LIB), (DUPLICATES, allowed)]).len(), 1);
    }
}
