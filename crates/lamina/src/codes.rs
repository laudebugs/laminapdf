//! Hints and `docsUrl`s from `fixtures/codes.json` (generated into `$OUT_DIR` by `build.rs`).

include!(concat!(env!("OUT_DIR"), "/codes.rs"));

/// The hint of `code` with `{limit}` filled in (ADR 0026 §5: the only parameters are lamina's
/// own vocabulary). A parameter with no value leaves the hint out rather than show a brace.
pub(crate) fn hint(code: &str, limit: Option<&'static str>) -> Option<String> {
    let template = lookup(HINTS, code)?;
    let filled = match limit {
        Some(limit) => template.replace("{limit}", limit),
        None => template.to_owned(),
    };
    (!filled.contains('{')).then_some(filled)
}

/// `base + code + suffix` (ADR 0026 §6).
pub(crate) fn docs_url(code: &str) -> String {
    format!("{DOCS_BASE}{code}{DOCS_SUFFIX}")
}

/// The summary of `code`: a fixed sentence, used as a warning's `message`.
pub(crate) fn summary(code: &str) -> Option<&'static str> {
    lookup(SUMMARIES, code)
}

fn lookup(table: &'static [(&'static str, &'static str)], code: &str) -> Option<&'static str> {
    table
        .binary_search_by(|(c, _)| (*c).cmp(code))
        .ok()
        .and_then(|i| table.get(i))
        .map(|(_, text)| *text)
}
