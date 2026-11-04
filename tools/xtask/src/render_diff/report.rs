//! The run summary for people: the per-source table and the HTML page with the worst pages.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::rows::{CLASSES, Row, Tally, percent};

/// `` `file` p3 fail: 4.21 % differ, SSIM 0.8123 ``.
pub(super) fn describe(row: &Row) -> String {
    let page = row.page.map_or(String::new(), |p| format!(" p{p}"));
    let metric = match (row.diff_percent, row.ssim) {
        (Some(d), Some(s)) => format!("{d:.2} % differ, SSIM {s:.4}"),
        _ if row.class == "fail" => "page sizes differ".to_owned(),
        _ => row.detail.clone().unwrap_or_default(),
    };
    format!("`{}`{page} {}: {metric}", row.file, row.class)
}

pub(super) fn sources_table(tallies: &BTreeMap<String, Tally>) -> String {
    let mut md = String::from("| Source | Pages | pass+minor % | pass % |");
    for class in CLASSES {
        let _ = write!(md, " {class} |");
    }
    md.push_str("\n| --- | ---: | ---: | ---: |");
    md.push_str(&" ---: |".repeat(CLASSES.len()));
    md.push('\n');
    for (source, t) in tallies {
        let _ = write!(
            md,
            "| {source} | {} | {:.2} % | {:.2} % |",
            t.rows,
            percent(t.rate_bp()),
            percent(t.pass_bp())
        );
        for class in CLASSES {
            let _ = write!(md, " {} |", t.count(class));
        }
        md.push('\n');
    }
    md
}

pub(super) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The HTML summary: the per-source table, the sampling, and the worst pages with their
/// images (`diffs/<image>`).
pub(super) fn html(tallies: &BTreeMap<String, Tally>, worst: &[&Row], errors: &[&Row], context: &str) -> String {
    let mut h = String::from(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Render diff: hayro vs PDFium</title>\n<style>\n\
         :root { color-scheme: light dark; --bg: #fbfaf7; --fg: #1f1d1a; --muted: #6b665e; --rule: #ddd8cf; --fail: #b3261e; }\n\
         @media (prefers-color-scheme: dark) { :root { --bg: #161513; --fg: #ece8e1; --muted: #a29c92; --rule: #3a3732; --fail: #f2b8b5; } }\n\
         body { background: var(--bg); color: var(--fg); font: 15px/1.5 system-ui, sans-serif; margin: 0 auto; max-width: 1200px; padding: 16px; }\n\
         table { border-collapse: collapse; display: block; overflow-x: auto; }\n\
         th, td { border-bottom: 1px solid var(--rule); padding: 4px 10px; text-align: right; white-space: nowrap; }\n\
         th:first-child, td:first-child { text-align: left; }\n\
         .muted { color: var(--muted); } .fail { color: var(--fail); }\n\
         figure { border-top: 1px solid var(--rule); margin: 24px 0; padding-top: 12px; }\n\
         figure img { display: block; height: auto; max-width: 100%; }\n\
         code { overflow-wrap: anywhere; }\n</style>\n</head>\n<body>\n\
         <h1>Render diff: hayro vs PDFium</h1>\n",
    );
    let _ = writeln!(h, "<p class=\"muted\">{}</p>", escape(context));
    h.push_str("<table>\n<thead><tr><th>Source</th><th>Pages</th><th>pass+minor %</th><th>pass %</th>");
    for class in CLASSES {
        let _ = write!(h, "<th>{class}</th>");
    }
    h.push_str("</tr></thead>\n<tbody>\n");
    for (source, t) in tallies {
        let _ = write!(
            h,
            "<tr><td>{}</td><td>{}</td><td>{:.2} %</td><td>{:.2} %</td>",
            escape(source),
            t.rows,
            percent(t.rate_bp()),
            percent(t.pass_bp())
        );
        for class in CLASSES {
            let _ = write!(h, "<td>{}</td>", t.count(class));
        }
        h.push_str("</tr>\n");
    }
    h.push_str("</tbody>\n</table>\n");
    let _ = writeln!(
        h,
        "<h2>The {} worst pages</h2>\n<p class=\"muted\">Each image is PDFium | hayro | diff (differing pixels in red).</p>",
        worst.len()
    );
    for row in worst {
        let image = row.image.as_deref().unwrap_or_default();
        let _ = writeln!(
            h,
            "<figure><figcaption><span class=\"fail\">{}</span> <code>{}</code>{}: {}</figcaption>\
             <img loading=\"lazy\" src=\"diffs/{}\" alt=\"PDFium, hayro and diff of {} page {}\"></figure>",
            escape(&row.class),
            escape(&row.file),
            row.page.map_or(String::new(), |p| format!(" page {p}")),
            escape(describe(row).split(": ").last().unwrap_or_default()),
            escape(image),
            escape(&row.file),
            row.page.unwrap_or_default()
        );
    }
    if !errors.is_empty() {
        let _ = writeln!(h, "<h2>Errors and timeouts ({})</h2>\n<ul>", errors.len());
        for row in errors {
            let _ = writeln!(h, "<li>{}</li>", escape(&describe(row)));
        }
        h.push_str("</ul>\n");
    }
    h.push_str("</body>\n</html>\n");
    h
}
