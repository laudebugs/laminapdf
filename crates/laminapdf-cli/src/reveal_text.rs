//! `laminapdf reveal`'s text: the categories found, then one line for each. Names from the
//! document (Info keys, file, field and layer names) appear flattened to one line, as `info`
//! shows a title; values (Info values, comments, field values) never do.

use std::fmt::Write as _;

use laminapdf_cli::commands::{PageRef, RevealReport};

use crate::render::{count, one_line, size};

/// One line per category with findings, under the categories found.
pub(crate) fn reveal_text(report: &RevealReport) -> String {
    if report.found.is_empty() {
        return format!("{}: nothing beyond its pages\n", report.input);
    }
    let mut text = format!("{}: {}\n", report.input, report.found.join(", "));
    for category in &report.found {
        let parts = match category.as_str() {
            "history" => history(report),
            "metadata" => metadata(report),
            "attachments" => attachments(report),
            "active-content" => active_content(report),
            "annotations" => annotations(report),
            "forms" => forms(report),
            _ => hidden_content(report),
        };
        let _ = writeln!(text, "  {category:<15} {}", parts.join("; "));
    }
    text
}

fn n(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

fn pages(p: &[PageRef]) -> String {
    let numbers: Vec<String> = p.iter().map(|p| p.number.to_string()).collect();
    let noun = if numbers.len() == 1 { "page" } else { "pages" };
    format!("{noun} {}", numbers.join(", "))
}

fn history(r: &RevealReport) -> Vec<String> {
    let h = &r.history;
    let mut out = Vec::new();
    if h.revisions.len() > 1 {
        out.push(count(n(h.revisions.len()), "revision"));
    }
    for rev in h.revisions.iter().filter(|rev| rev.removed_words > 0) {
        let words = count(rev.removed_words, "word");
        out.push(format!("revision {} holds {words} the latest doesn't", rev.index));
    }
    if !h.unreferenced.is_empty() {
        out.push(format!(
            "{} nothing references",
            count(n(h.unreferenced.len()), "object")
        ));
    }
    out
}

fn metadata(r: &RevealReport) -> Vec<String> {
    let m = &r.metadata;
    let mut out = Vec::new();
    if !m.info.is_empty() {
        let keys: Vec<String> = m.info.iter().map(|e| one_line(&e.key)).collect();
        out.push(format!("Info: {}", keys.join(", ")));
    }
    if let Some(x) = &m.xmp {
        out.push(format!(
            "XMP ({}, {})",
            size(x.bytes),
            count(n(x.namespaces.len()), "namespace")
        ));
    }
    let others = [
        (m.metadata_streams, "metadata stream"),
        (m.piece_info, "PieceInfo dictionary"),
        (m.thumbnails, "thumbnail"),
    ];
    out.extend(
        others
            .into_iter()
            .filter(|(k, _)| *k > 0)
            .map(|(k, what)| count(k, what)),
    );
    out
}

fn attachments(r: &RevealReport) -> Vec<String> {
    r.attachments
        .iter()
        .map(|a| {
            let name = one_line(a.name.as_deref().unwrap_or("unnamed"));
            a.size
                .map_or_else(|| format!("\"{name}\""), |s| format!("\"{name}\" ({})", size(s)))
        })
        .collect()
}

fn active_content(r: &RevealReport) -> Vec<String> {
    let a = &r.active_content;
    let mut out = Vec::new();
    if a.scripts > 0 {
        out.push(format!("{} ({})", count(a.scripts, "script"), size(a.script_bytes)));
    }
    let mut kinds: Vec<&str> = a.actions.iter().map(|x| x.kind.as_str()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    if !kinds.is_empty() {
        out.push(format!("{}: {}", count(n(a.actions.len()), "action"), kinds.join(", ")));
    }
    if a.xfa {
        out.push("an XFA form".to_owned());
    }
    out
}

fn annotations(r: &RevealReport) -> Vec<String> {
    let a = &r.annotations;
    let mut out: Vec<String> = a
        .by_subtype
        .iter()
        .filter(|(s, _)| !matches!(s.as_str(), "Link" | "Widget" | "Popup"))
        .map(|(subtype, k)| format!("{k} {subtype}"))
        .collect();
    if !a.comments.is_empty() {
        out.push(count(n(a.comments.len()), "comment"));
    }
    out
}

fn forms(r: &RevealReport) -> Vec<String> {
    let filled: Vec<String> = r
        .forms
        .iter()
        .filter(|f| f.filled)
        .map(|f| one_line(f.name.as_deref().unwrap_or("unnamed")))
        .collect();
    vec![format!(
        "{} filled: {}",
        count(n(filled.len()), "field"),
        filled.join(", ")
    )]
}

fn hidden_content(r: &RevealReport) -> Vec<String> {
    let h = &r.hidden_content;
    let mut out: Vec<String> = h
        .text
        .iter()
        .map(|t| format!("{} {} characters on {}", t.chars, t.reason, pages(&t.pages)))
        .collect();
    out.extend(
        h.layers_off
            .iter()
            .map(|layer| format!("layer \"{}\" off", one_line(layer.name.as_deref().unwrap_or("unnamed")))),
    );
    out.extend(h.redactions.iter().map(|red| {
        format!(
            "{} on page {}",
            count(red.count, "unapplied redaction"),
            red.page.number
        )
    }));
    out
}
