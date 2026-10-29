//! `summary.md`: totals, per-corpus class counts, threshold sensitivity,
//! timing, peak RSS and the most frequent errors and hayro warnings.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::compare;
use crate::driver::{Proc, Row};

const CLASSES: [&str; 8] = [
    "pass",
    "minor",
    "fail",
    "hayro-error",
    "pdfium-error",
    "both-error",
    "timeout",
    "rendered",
];

fn pct(n: usize, d: usize) -> f64 {
    if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 }
}

fn quantile(v: &mut [f64], q: f64) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let i = ((v.len() - 1) as f64 * q).round() as usize;
    v.get(i).copied()
}

fn fmt_opt(v: Option<f64>, unit: &str) -> String {
    v.map(|v| format!("{v:.1}{unit}")).unwrap_or_else(|| "–".into())
}

/// Normalises an error message so similar ones group together.
fn bucket(e: &str) -> String {
    let e: String = e.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect();
    e.chars().take(90).collect()
}

fn top(counts: BTreeMap<String, usize>, n: usize) -> Vec<(String, usize)> {
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

pub fn render(rows: &[Row], procs: &[Proc], header: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# hayro-parity summary\n\n{header}\n");
    let mut corpora: Vec<&str> = rows.iter().map(|r| r.corpus.as_str()).collect();
    corpora.sort_unstable();
    corpora.dedup();
    let groups: Vec<(&str, Vec<&Row>)> = corpora
        .iter()
        .map(|c| (*c, rows.iter().filter(|r| r.corpus == *c).collect()))
        .chain(std::iter::once(("total", rows.iter().collect())))
        .collect();

    let _ = writeln!(s, "## Classes (default thresholds)\n");
    let _ = writeln!(
        s,
        "| corpus | files | pages | {} |\n| --- | ---: | ---: |{}",
        CLASSES.join(" | "),
        " ---: |".repeat(CLASSES.len())
    );
    for (name, g) in &groups {
        let mut files: Vec<&str> = g.iter().map(|r| r.file.as_str()).collect();
        files.dedup();
        let cells: Vec<String> = CLASSES
            .iter()
            .map(|c| {
                let n = g.iter().filter(|r| r.class == *c).count();
                format!("{n} ({:.1} %)", pct(n, g.len()))
            })
            .collect();
        let _ = writeln!(s, "| {name} | {} | {} | {} |", files.len(), g.len(), cells.join(" | "));
    }

    let _ = writeln!(
        s,
        "\n## Threshold sensitivity\n\nPass / pass+minor as a share of **all** pages, and of the pages both engines rendered at the same size (`compared`).\n\n| corpus | thresholds | pass | pass+minor | pass (of compared) | pass+minor (of compared) |\n| --- | --- | ---: | ---: | ---: | ---: |"
    );
    for (name, g) in &groups {
        let compared: Vec<(f64, f64)> = g
            .iter()
            .filter(|r| matches!(r.class.as_str(), "pass" | "minor" | "fail"))
            .filter_map(|r| Some((r.ssim?, r.diff_pct?)))
            .collect();
        for t in compare::ALL {
            let pass = compared.iter().filter(|(a, b)| t.classify(*a, *b) == "pass").count();
            let ok = compared.iter().filter(|(a, b)| t.classify(*a, *b) != "fail").count();
            let _ = writeln!(
                s,
                "| {name} | {} (SSIM ≥ {}, ≤ {} %; minor SSIM ≥ {}, ≤ {} %) | {:.1} % | {:.1} % | {:.1} % | {:.1} % |",
                t.name,
                t.pass_ssim,
                t.pass_diff_pct,
                t.minor_ssim,
                t.minor_diff_pct,
                pct(pass, g.len()),
                pct(ok, g.len()),
                pct(pass, compared.len()),
                pct(ok, compared.len()),
            );
        }
    }

    let _ = writeln!(
        s,
        "\n## Speed and memory\n\nPer-page render time over pages the engine rendered without error. Peak RSS is per child process (one document, both engines loaded in a two-engine run).\n\n| corpus | hayro median | hayro p95 | hayro total | PDFium median | PDFium p95 | PDFium total | pages hayro faster | peak RSS median | peak RSS p95 | peak RSS max |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    for (name, g) in &groups {
        let mut h: Vec<f64> = g
            .iter()
            .filter(|r| r.hayro_err.is_empty())
            .filter_map(|r| r.hayro_ms)
            .collect();
        let mut p: Vec<f64> = g
            .iter()
            .filter(|r| r.pdfium_err.is_empty())
            .filter_map(|r| r.pdfium_ms)
            .collect();
        let both: Vec<(f64, f64)> = g
            .iter()
            .filter(|r| r.hayro_err.is_empty() && r.pdfium_err.is_empty())
            .filter_map(|r| Some((r.hayro_ms?, r.pdfium_ms?)))
            .collect();
        let faster = both.iter().filter(|(a, b)| a < b).count();
        let (ht, pt): (f64, f64) = (h.iter().sum(), p.iter().sum());
        let mut rss: Vec<f64> = procs
            .iter()
            .filter(|p| *name == "total" || p.corpus == *name)
            .map(|p| p.max_rss as f64 / (1024.0 * 1024.0))
            .collect();
        let _ = writeln!(
            s,
            "| {name} | {} | {} | {:.1} s | {} | {} | {:.1} s | {} | {} | {} | {} |",
            fmt_opt(quantile(&mut h, 0.5), " ms"),
            fmt_opt(quantile(&mut h, 0.95), " ms"),
            ht / 1000.0,
            fmt_opt(quantile(&mut p, 0.5), " ms"),
            fmt_opt(quantile(&mut p, 0.95), " ms"),
            pt / 1000.0,
            if both.is_empty() {
                "–".into()
            } else {
                format!("{:.1} %", pct(faster, both.len()))
            },
            fmt_opt(quantile(&mut rss, 0.5), " MiB"),
            fmt_opt(quantile(&mut rss, 0.95), " MiB"),
            fmt_opt(quantile(&mut rss, 1.0), " MiB"),
        );
    }

    let enc: Vec<&Row> = rows.iter().filter(|r| r.encrypted).collect();
    let _ = writeln!(
        s,
        "\n## Encrypted documents\n\n{} pages from files containing `/Encrypt`:",
        enc.len()
    );
    for c in CLASSES {
        let n = enc.iter().filter(|r| r.class == c).count();
        if n > 0 {
            let _ = writeln!(s, "- {c}: {n}");
        }
    }

    for (title, pick) in [
        ("hayro errors", (|r: &Row| r.hayro_err.as_str()) as fn(&Row) -> &str),
        ("PDFium errors", |r: &Row| r.pdfium_err.as_str()),
    ] {
        let mut counts = BTreeMap::new();
        for r in rows {
            let e = pick(r);
            if !e.is_empty() {
                *counts.entry(bucket(e)).or_insert(0) += 1;
            }
        }
        let _ = writeln!(s, "\n## Top {title} (pages)\n");
        for (e, n) in top(counts, 15) {
            let _ = writeln!(s, "- {n} × `{e}`");
        }
    }

    let _ = writeln!(s, "\n## hayro warnings by class (pages raising each warning)\n");
    let _ = writeln!(s, "| warning | pass | minor | fail |\n| --- | ---: | ---: | ---: |");
    let mut warn: BTreeMap<String, [usize; 3]> = BTreeMap::new();
    for r in rows {
        let Some(i) = ["pass", "minor", "fail"].iter().position(|c| *c == r.class) else {
            continue;
        };
        for w in r.warnings.split(';').filter(|w| !w.trim().is_empty()) {
            warn.entry(w.trim().to_owned()).or_default()[i] += 1;
        }
    }
    let mut warn: Vec<_> = warn.into_iter().collect();
    warn.sort_by(|a, b| b.1[2].cmp(&a.1[2]).then(a.0.cmp(&b.0)));
    for (w, [p, m, f]) in warn.iter().take(20) {
        let _ = writeln!(s, "| `{w}` | {p} | {m} | {f} |");
    }
    s
}
