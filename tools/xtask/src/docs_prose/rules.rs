//! The rules of docs/guidelines/docs-writing.md that a tool can check. Each rule reads a parsed
//! [`Page`] and returns [`Finding`]s; which rules apply depends on the file's [`Scope`].

use std::collections::BTreeSet;

use serde::Deserialize;

use super::extract::{Block, CODE, Kind, Page};
use crate::reports::{Finding, finding};

/// `tools/xtask/docs-prose.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) banned_words: Vec<String>,
    #[serde(default)]
    pub(crate) glossary: Vec<String>,
    #[serde(default)]
    pub(crate) term: Vec<Term>,
    #[serde(default)]
    pub(crate) qualified: Vec<Qualified>,
    #[serde(default)]
    pub(crate) proper_nouns: BTreeSet<String>,
    #[serde(default)]
    pub(crate) internal_words: Vec<String>,
    #[serde(default)]
    pub(crate) internal_phrases: Vec<String>,
    #[serde(default)]
    pub(crate) repository_paths: Vec<String>,
    #[serde(default)]
    pub(crate) abbreviations: BTreeSet<String>,
    #[serde(default)]
    pub(crate) id_exceptions: Vec<String>,
}

/// A spelling to replace ("One term per thing").
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Term {
    pub(crate) spelling: String,
    #[serde(rename = "use")]
    pub(crate) replacement: String,
}

/// A word that needs a qualifier before it ("Worker" → "Cloudflare Worker" or "Web Worker").
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Qualified {
    pub(crate) word: String,
    pub(crate) after: Vec<String>,
    #[serde(rename = "use")]
    pub(crate) replacement: String,
}

/// Which rules apply to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// A docs-site page; `guide` for guides and recipes, which open with a short paragraph.
    SitePage { guide: bool },
    /// A page the site renders from elsewhere: an error-code page or a package README.
    Rendered,
}

pub(crate) const MAX_SENTENCE_WORDS: usize = 40;
pub(crate) const MAX_PARAGRAPH_WORDS: usize = 100;
pub(crate) const MAX_OPENING_WORDS: usize = 50;
pub(crate) const MAX_DESCRIPTION_CHARS: usize = 160;

/// Every rule that applies to `scope`, in reading order.
pub(crate) fn check(page: &Page, scope: Scope, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Scope::SitePage { guide } = scope {
        findings.extend(frontmatter(page));
        if guide {
            findings.extend(opening_paragraph(page));
        }
        findings.extend(glossary(page, config));
        findings.extend(snippet_intro(page));
    }
    for block in reader_blocks(page) {
        findings.extend(lengths(block, config));
        findings.extend(internal_ids(block, config));
        findings.extend(banned_words(block, config));
        findings.extend(one_term(block, config));
        findings.extend(sentence_case(block, config));
    }
    findings.sort_by_key(|finding| finding.line);
    findings
}

/// The blocks the reader rules check: everything before a page's last section, "For contributors".
/// That section holds decision and todo links on purpose, on every page: the docs site leaves it
/// out of error pages (`apps/docs/plugins/error-pages.mjs`), and on a README it is marked as
/// contributors' notes. `end` is at most the length, so the fallback never applies.
fn reader_blocks(page: &Page) -> &[Block] {
    let end = page
        .blocks
        .iter()
        .rposition(|block| block.kind == Kind::Heading(2) && block.text() == "For contributors")
        .unwrap_or(page.blocks.len());
    page.blocks.get(..end).unwrap_or(&page.blocks)
}

/// Words of prose: whitespace-separated, with surrounding punctuation and a possessive `'s`
/// trimmed (so "CI's" is `CI` and "Adobe's" is `Adobe`).
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace().map(bare_word).filter(|word| !word.is_empty())
}

fn bare_word(raw: &str) -> &str {
    let word = raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '⟨' && c != '⟩');
    word.strip_suffix("'s")
        .or_else(|| word.strip_suffix("’s"))
        .unwrap_or(word)
}

fn frontmatter(page: &Page) -> Vec<Finding> {
    let Some(frontmatter) = &page.frontmatter else {
        return vec![finding(
            1,
            "frontmatter",
            "the page has no frontmatter with a title and a description",
        )];
    };
    let mut findings = Vec::new();
    if frontmatter.title.as_deref().is_none_or(str::is_empty) {
        findings.push(finding(1, "frontmatter", "the frontmatter has no `title`"));
    }
    match frontmatter.description.as_deref() {
        None | Some("") => findings.push(finding(1, "frontmatter", "the frontmatter has no `description`")),
        Some(description) if description.chars().count() > MAX_DESCRIPTION_CHARS => findings.push(finding(
            1,
            "frontmatter",
            format!(
                "the description has {} characters; the limit is {MAX_DESCRIPTION_CHARS}",
                description.chars().count()
            ),
        )),
        Some(_) => {}
    }
    findings
}

/// Guides and recipes open with one paragraph of at most 50 words before anything else.
fn opening_paragraph(page: &Page) -> Vec<Finding> {
    let first = page.blocks.iter().find(|block| block.kind != Kind::Wrapper);
    match first {
        Some(block) if block.kind == Kind::Paragraph => {
            let count = words(&block.text()).count();
            if count > MAX_OPENING_WORDS {
                vec![finding(
                    block.line,
                    "opening-paragraph",
                    format!(
                        "the opening paragraph has {count} words; cut it to {MAX_OPENING_WORDS} words or fewer that say what you'll make and when you'd need it"
                    ),
                )]
            } else {
                Vec::new()
            }
        }
        Some(block) => vec![finding(
            block.line,
            "opening-paragraph",
            "open with a paragraph that says what you'll make and when you'd need it, before any list, code or heading",
        )],
        None => Vec::new(),
    }
}

/// A sentence's word count and its first few words (to name it in a report).
#[derive(Debug, PartialEq, Eq)]
struct Sentence {
    words: usize,
    start: String,
}

/// Splits prose into sentences.
fn sentences(text: &str, config: &Config) -> Vec<Sentence> {
    const QUOTED_WORDS: usize = 6;
    let mut sentences = Vec::new();
    let mut count = 0;
    let mut start: Vec<&str> = Vec::new();
    for raw in text.split_whitespace() {
        if raw.chars().any(char::is_alphanumeric) || raw.contains(CODE) {
            count += 1;
        }
        if start.len() < QUOTED_WORDS {
            start.push(raw);
        }
        let end = raw.trim_end_matches(['"', '\'', ')', '”', '’', '*', '_']);
        let ends_sentence = end.ends_with(['.', '?', '!']) && {
            let stem = end.trim_end_matches(['.', '?', '!']).to_lowercase();
            !config.abbreviations.contains(stem.as_str())
        };
        if ends_sentence && count > 0 {
            sentences.push(Sentence {
                words: count,
                start: start.join(" "),
            });
            count = 0;
            start.clear();
        }
    }
    if count > 0 {
        sentences.push(Sentence {
            words: count,
            start: start.join(" "),
        });
    }
    sentences
}

fn lengths(block: &Block, config: &Config) -> Vec<Finding> {
    if !matches!(block.kind, Kind::Paragraph | Kind::ListItem) {
        return Vec::new();
    }
    let text = block.text();
    let mut findings: Vec<Finding> = sentences(&text, config)
        .into_iter()
        .filter(|sentence| sentence.words > MAX_SENTENCE_WORDS)
        .map(|sentence| {
            finding(
                block.line,
                "sentence-length",
                format!(
                    "the sentence starting \"{}…\" has {} words; split it into sentences of {MAX_SENTENCE_WORDS} or fewer",
                    sentence.start, sentence.words
                ),
            )
        })
        .collect();
    let count = words(&text).count();
    if count > MAX_PARAGRAPH_WORDS {
        findings.push(finding(
            block.line,
            "paragraph-length",
            format!("the paragraph has {count} words; split it into paragraphs of {MAX_PARAGRAPH_WORDS} or fewer"),
        ));
    }
    findings
}

/// An exclamation mark in the writer's own words: one inside double quotes ("Hello, world!") is
/// quoted text, such as what an example draws, and is allowed.
/// Quotes pair up left to right (a straight `"` with the next one, `“` with the next `”`), and a
/// `!` inside a pair is quoted. A `"` with no partner opens nothing, so it can't hide later marks;
/// a stray `"` that a real quote follows still pairs with it (a limit the tests show).
fn has_unquoted_exclamation(text: &str) -> bool {
    let mut quoted_ranges = Vec::new();
    let straight: Vec<usize> = text.match_indices('"').map(|(index, _)| index).collect();
    quoted_ranges.extend(straight.as_chunks::<2>().0.iter().map(|[open, close]| (*open, *close)));
    let mut rest = text;
    let mut base = 0;
    while let Some(open) = rest.find('“') {
        let Some(close) = rest.get(open..).and_then(|after| after.find('”')) else {
            break;
        };
        quoted_ranges.push((base + open, base + open + close));
        base += open + close;
        rest = text.get(base..).unwrap_or_default();
    }
    text.match_indices('!').any(|(index, _)| {
        !quoted_ranges
            .iter()
            .any(|(open, close)| *open < index && index < *close)
    })
}

/// A todo ID: one capital letter, one or two digits and a lowercase suffix (`B8b`, `Q1a`). Bare
/// IDs without a suffix (`X5`, `P1`) are left out on purpose: they read like paper sizes (`A4`),
/// quarters (`Q3`) and revisions (`R5`). `id_exceptions` in the config lists words that look
/// like IDs but aren't.
pub(crate) fn is_todo_id(word: &str) -> bool {
    let mut chars = word.chars();
    let letter = chars.next().is_some_and(|c| c.is_ascii_uppercase());
    let rest: Vec<char> = chars.collect();
    let digits = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    letter && (1..=2).contains(&digits) && rest.len() == digits + 1 && rest.last().is_some_and(char::is_ascii_lowercase)
}

/// Whether `word`, followed by `next`, names an ADR by number: `ADR 0004` (`ADRs 0004`) or
/// `ADR-0004`.
pub(crate) fn is_adr_number(word: &str, next: Option<&str>) -> bool {
    let is_number = |text: &str| text.len() == 4 && text.chars().all(|c| c.is_ascii_digit());
    let adr_then_number = matches!(word, "ADR" | "ADRs") && next.is_some_and(is_number);
    adr_then_number || word.strip_prefix("ADR-").is_some_and(is_number)
}

fn internal_ids(block: &Block, config: &Config) -> Vec<Finding> {
    let text = block.text();
    let words: Vec<&str> = words(&text).collect();
    let mut found = BTreeSet::new();
    for (index, word) in words.iter().enumerate() {
        if is_todo_id(word) && !config.id_exceptions.iter().any(|exception| exception == word) {
            found.insert(format!("the todo ID `{word}`"));
        }
        let next = words.get(index + 1);
        if is_adr_number(word, next.copied()) {
            found.insert("an ADR number".to_owned());
        }
        let round_id = next.is_some_and(|next| {
            let mut chars = next.chars();
            chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.next().is_some_and(|c| c.is_ascii_digit())
        });
        if word.eq_ignore_ascii_case("round") && round_id {
            found.insert("a round (\"Round …\")".to_owned());
        }
        if config.internal_words.iter().any(|internal| internal == word) {
            found.insert(format!("`{word}`"));
        }
    }
    let lower = text.to_lowercase();
    for phrase in &config.internal_phrases {
        if contains_phrase(&lower, &phrase.to_lowercase()) {
            found.insert(format!("\"{phrase}\""));
        }
    }
    for path in &config.repository_paths {
        let mentions_path = text.match_indices(path.as_str()).any(|(index, _)| {
            text.get(..index)
                .and_then(|before| before.chars().next_back())
                .is_none_or(|c| c.is_whitespace() || c == '(')
        });
        if mentions_path {
            found.insert(format!("the repository path `{path}…`"));
        }
    }
    found
        .into_iter()
        .map(|what| {
            finding(
                block.line,
                "internal-id",
                format!("remove {what} from reader text; contributors look these up in `docs/`"),
            )
        })
        .collect()
}

/// True when `phrase` occurs in `text` as whole words (both already lowercase).
fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(index, _)| {
        let before = text.get(..index).and_then(|head| head.chars().next_back());
        let after = text.get(index + phrase.len()..).and_then(|tail| tail.chars().next());
        before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric())
    })
}

fn banned_words(block: &Block, config: &Config) -> Vec<Finding> {
    if matches!(block.kind, Kind::Snippet | Kind::Wrapper | Kind::Other) {
        return Vec::new();
    }
    let text = block.text();
    let lower = text.to_lowercase();
    let mut findings: Vec<Finding> = config
        .banned_words
        .iter()
        .filter(|banned| contains_phrase(&lower, &banned.to_lowercase()))
        .map(|banned| {
            finding(
                block.line,
                "banned-word",
                format!("remove \"{banned}\": the docs-writing guideline bans it"),
            )
        })
        .collect();
    if has_unquoted_exclamation(&text) {
        findings.push(finding(
            block.line,
            "banned-word",
            "replace the exclamation mark with a full stop",
        ));
    }
    findings
}

fn one_term(block: &Block, config: &Config) -> Vec<Finding> {
    let text = block.text();
    let words: Vec<&str> = words(&text).collect();
    let mut findings = Vec::new();
    for term in &config.term {
        if words.iter().any(|word| *word == term.spelling) {
            findings.push(finding(
                block.line,
                "one-term",
                format!("write \"{}\", not \"{}\"", term.replacement, term.spelling),
            ));
        }
    }
    for qualified in &config.qualified {
        let bare = words.iter().enumerate().any(|(index, word)| {
            word.eq_ignore_ascii_case(&qualified.word)
                && index
                    .checked_sub(1)
                    .and_then(|previous| words.get(previous))
                    .is_none_or(|previous| !qualified.after.iter().any(|after| previous.eq_ignore_ascii_case(after)))
        });
        if bare {
            findings.push(finding(
                block.line,
                "one-term",
                format!(
                    "write \"{}\" in full, not \"{}\" alone",
                    qualified.replacement, qualified.word
                ),
            ));
        }
    }
    findings
}

fn sentence_case(block: &Block, config: &Config) -> Vec<Finding> {
    if !matches!(block.kind, Kind::Heading(_)) {
        return Vec::new();
    }
    let text = block.text();
    let mut after_colon = false;
    let mut capitalised = Vec::new();
    // Hyphenated words are judged part by part, so "Right-To-Left" is caught.
    let parts = text.split_whitespace().flat_map(|raw| {
        let ends_with_colon = raw.ends_with(':');
        let pieces: Vec<&str> = raw.split('-').collect();
        let last = pieces.len().saturating_sub(1);
        pieces
            .into_iter()
            .enumerate()
            .map(move |(index, piece)| (piece, ends_with_colon && index == last))
    });
    for (index, (raw, ends_with_colon)) in parts.enumerate() {
        let word = bare_word(raw);
        let allowed = index == 0
            || after_colon
            || raw.contains(CODE)
            || !word.chars().next().is_some_and(char::is_uppercase)
            || word.chars().filter(|c| c.is_uppercase()).count() >= 2
            || word.chars().any(|c| c.is_ascii_digit())
            || config.proper_nouns.contains(word);
        if !allowed {
            capitalised.push(word.to_owned());
        }
        after_colon = ends_with_colon;
    }
    if capitalised.is_empty() {
        Vec::new()
    } else {
        vec![finding(
            block.line,
            "sentence-case",
            format!(
                "write the heading in sentence case: make {} lowercase, or add {} to `proper_nouns` in tools/xtask/docs-prose.toml",
                capitalised.join(", "),
                if capitalised.len() == 1 { "it" } else { "them" }
            ),
        )]
    }
}

/// The first use of each glossary term links to the glossary, or the page allows it.
fn glossary(page: &Page, config: &Config) -> Vec<Finding> {
    let allowed: BTreeSet<String> = page
        .frontmatter
        .as_ref()
        .map(|frontmatter| frontmatter.allow.iter().map(|term| term.to_lowercase()).collect())
        .unwrap_or_default();
    let mut findings = Vec::new();
    for term in &config.glossary {
        let lower = term.to_lowercase();
        if allowed.contains(&lower) {
            continue;
        }
        let first_use = page
            .blocks
            .iter()
            .filter(|block| matches!(block.kind, Kind::Paragraph | Kind::ListItem))
            .find_map(|block| {
                block
                    .segments
                    .iter()
                    .find(|segment| contains_phrase(&segment.text.to_lowercase(), &lower))
                    .map(|segment| (block.line, segment))
            });
        if let Some((line, segment)) = first_use {
            let linked = segment
                .link
                .as_deref()
                .is_some_and(|link| link.contains("/reference/glossary"));
            if !linked {
                findings.push(finding(
                    line,
                    "glossary",
                    format!(
                        "explain \"{term}\" where it first appears, link it to /reference/glossary/, or list it under `prose.allow` in the frontmatter"
                    ),
                ));
            }
        }
    }
    findings
}

/// A sentence that says what the code does comes before every `<Snippet>`. Snippets in a run of
/// tabs share one sentence: walking back, an earlier snippet is skipped only when a tab wrapper
/// separates it from this one; two snippets with nothing between them need two sentences.
fn snippet_intro(page: &Page) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, block) in page.blocks.iter().enumerate() {
        if block.kind != Kind::Snippet {
            continue;
        }
        let mut crossed_wrapper = false;
        let before = page
            .blocks
            .iter()
            .take(index)
            .rev()
            .find(|previous| match previous.kind {
                Kind::Wrapper => {
                    crossed_wrapper = true;
                    false
                }
                Kind::Snippet if crossed_wrapper => {
                    crossed_wrapper = false;
                    false
                }
                _ => true,
            });
        if !before.is_some_and(|previous| matches!(previous.kind, Kind::Paragraph | Kind::ListItem)) {
            findings.push(finding(
                block.line,
                "snippet-intro",
                "put a sentence that says what the code does before this snippet",
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::super::extract::parse;
    use super::*;

    fn config() -> Config {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs-prose.toml")).unwrap();
        toml::from_str(&text).unwrap()
    }

    const FRONT: &str = "---\ntitle: T\ndescription: D\n---\n\n";

    fn rules_for(body: &str, guide: bool) -> Vec<&'static str> {
        let page = parse(&format!("{FRONT}{body}"));
        check(&page, Scope::SitePage { guide }, &config())
            .into_iter()
            .map(|f| f.rule)
            .collect()
    }

    #[test]
    fn a_clean_page_should_have_no_findings() {
        assert_eq!(
            rules_for("You make an invoice here. It takes five minutes.\n", true),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn a_page_without_frontmatter_should_be_reported() {
        let page = parse("Text.\n");
        let findings = check(&page, Scope::SitePage { guide: false }, &config());
        assert_eq!(findings.first().map(|f| f.rule), Some("frontmatter"));
    }

    #[test]
    fn a_long_description_should_be_reported() {
        let page = parse(&format!(
            "---\ntitle: T\ndescription: {}\n---\n\nText.\n",
            "x".repeat(161)
        ));
        assert_eq!(frontmatter(&page).len(), 1);
    }

    #[test]
    fn a_sentence_over_forty_words_should_be_reported_and_forty_should_pass() {
        let forty = "word ".repeat(40).trim_end().to_owned() + ".";
        assert_eq!(rules_for(&format!("{forty}\n"), false), Vec::<&str>::new());
        let forty_one = format!("one {forty}\n");
        assert_eq!(rules_for(&forty_one, false), vec!["sentence-length"]);
    }

    #[test]
    fn abbreviations_should_not_end_a_sentence() {
        let counts: Vec<usize> = sentences("Use one font, e.g. Source Sans 3, for text. Done.", &config())
            .into_iter()
            .map(|sentence| sentence.words)
            .collect();
        assert_eq!(counts, vec![9, 1]);
    }

    #[test]
    fn a_paragraph_over_a_hundred_words_should_be_reported() {
        let body = "Short sentence here. ".repeat(34);
        assert_eq!(rules_for(&format!("{body}\n"), false), vec!["paragraph-length"]);
    }

    #[test]
    fn a_guide_should_open_with_a_paragraph() {
        assert_eq!(
            rules_for("## Heading first\n\nText.\n", true),
            vec!["opening-paragraph"]
        );
    }

    #[test]
    fn todo_ids_adr_numbers_and_internal_words_should_be_reported() {
        let rules = rules_for("Q1a added this; see ADR 0026. The golden file and CI agree.\n", false);
        assert_eq!(rules, vec!["internal-id"; 4]);
    }

    #[test]
    fn rounds_and_other_adr_spellings_should_be_reported() {
        assert_eq!(rules_for("It came in Round B1.\n", false), vec!["internal-id"]);
        assert_eq!(rules_for("See ADR-0026 and ADRs 0027.\n", false), vec!["internal-id"]);
        assert_eq!(rules_for("A round of review.\n", false), Vec::<&str>::new());
    }

    #[test]
    fn possessives_should_not_hide_internal_words() {
        assert_eq!(rules_for("The CI's checks pass.\n", false), vec!["internal-id"]);
    }

    #[test]
    fn possessive_proper_nouns_should_be_allowed_in_headings() {
        assert_eq!(rules_for("## Use Adobe's fonts\n\nText.\n", false), Vec::<&str>::new());
    }

    #[test]
    fn hyphenated_title_case_should_be_reported() {
        assert_eq!(
            rules_for("## Text that runs Right-To-Left\n\nText.\n", false),
            vec!["sentence-case"]
        );
    }

    #[test]
    fn two_snippets_in_a_row_should_need_two_sentences() {
        let body = "Intro:\n\n<Snippet file=\"a.ts\" />\n\n<Snippet file=\"b.ts\" />\n";
        assert_eq!(rules_for(body, false), vec!["snippet-intro"]);
    }

    #[test]
    fn a_long_sentence_should_be_named_by_its_first_words() {
        let page = parse(&format!("{FRONT}{}\n", "word ".repeat(41).trim_end()));
        let message = check(&page, Scope::SitePage { guide: false }, &config())
            .into_iter()
            .map(|finding| finding.message)
            .next();
        assert!(message.is_some_and(|message| message.starts_with("the sentence starting \"word word word")));
    }

    #[test]
    fn the_contributor_section_should_not_be_checked() {
        let body = "## Why\n\nRepair keeps the file usable.\n\n## For contributors\n\nSee ADR 0003 and Q1a.\n";
        assert_eq!(rules_for(body, false), Vec::<&str>::new());
    }

    #[test]
    fn a_quoted_exclamation_should_be_allowed() {
        assert_eq!(
            rules_for("It says \"Hello, world!\" on one page.\n", false),
            Vec::<&str>::new()
        );
        assert_eq!(rules_for("It works!\n", false), vec!["banned-word"]);
    }

    #[test]
    fn quotes_should_pair_left_to_right() {
        // The limit of pairing: a stray `"` followed by a real quote reads as one quoted span.
        assert_eq!(rules_for("A 3\" margin. It works! \"x\"\n", false), Vec::<&str>::new());
        assert_eq!(
            rules_for("\"Hi!\" and \"there\". It works!\n", false),
            vec!["banned-word"]
        );
    }

    #[test]
    fn a_stray_quote_should_not_hide_a_later_exclamation() {
        assert_eq!(rules_for("A 3\" margin. It works!\n", false), vec!["banned-word"]);
    }

    #[test]
    fn paper_sizes_and_revisions_should_not_be_todo_ids() {
        assert_eq!(
            rules_for("Print on A4 paper with R5 encryption in Q3.\n", false),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn repository_paths_in_prose_should_be_reported_but_not_in_code() {
        assert_eq!(
            rules_for("Look in crates/laminapdf for it.\n", false),
            vec!["internal-id"]
        );
        assert_eq!(
            rules_for("Look in `crates/laminapdf` for it.\n", false),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn banned_words_and_exclamation_marks_should_be_reported() {
        assert_eq!(
            rules_for("It is just text!\n", false),
            vec!["banned-word", "banned-word"]
        );
    }

    #[test]
    fn banned_words_should_match_whole_words_only() {
        assert_eq!(rules_for("Adjust the margins.\n", false), Vec::<&str>::new());
    }

    #[test]
    fn a_bare_worker_should_be_reported_but_not_a_qualified_one() {
        assert_eq!(rules_for("Run it in a worker.\n", false), vec!["one-term"]);
        assert_eq!(
            rules_for("Run it in a Web Worker or a Cloudflare Worker.\n", false),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn a_replaced_spelling_should_be_reported() {
        assert_eq!(rules_for("Use Standard14 fonts.\n", false), vec!["one-term"]);
    }

    #[test]
    fn title_case_headings_should_be_reported() {
        assert_eq!(rules_for("## Open And Save\n\nText.\n", false), vec!["sentence-case"]);
        assert_eq!(
            rules_for("## Open a PDF in TypeScript\n\nText.\n", false),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn an_unlinked_glossary_term_should_be_reported_unless_allowed() {
        assert_eq!(rules_for("Edits go into the overlay.\n", false), vec!["glossary"]);
        assert_eq!(
            rules_for("Edits go into the [overlay](/reference/glossary/#overlay).\n", false),
            Vec::<&str>::new()
        );
        let page = parse(
            "---\ntitle: T\ndescription: D\nprose:\n  allow:\n    - overlay\n---\n\nEdits go into the overlay.\n",
        );
        assert_eq!(
            check(&page, Scope::SitePage { guide: false }, &config()),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn a_snippet_should_follow_a_sentence() {
        assert_eq!(
            rules_for("## Code\n\n<Snippet file=\"a.ts\" />\n", false),
            vec!["snippet-intro"]
        );
        assert_eq!(
            rules_for("This builds a page:\n\n<Snippet file=\"a.ts\" />\n", false),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn snippets_in_tabs_should_share_one_sentence() {
        let body = "This builds a page:\n\n<Tabs>\n<TabItem label=\"TS\">\n\n<Snippet file=\"a.ts\" />\n\n</TabItem>\n<TabItem label=\"Rust\">\n\n<Snippet file=\"a.rs\" />\n\n</TabItem>\n</Tabs>\n";
        assert_eq!(rules_for(body, false), Vec::<&str>::new());
    }

    #[test]
    fn rendered_pages_should_skip_site_only_rules() {
        let page = parse("# `aes-padding-invalid`\n\nThe overlay is fine here.\n");
        assert_eq!(check(&page, Scope::Rendered, &config()), Vec::<Finding>::new());
    }
}
