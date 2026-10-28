//! krilla-fit spike (Round B1): can krilla generate into lamina's object graph?
//!
//! `cargo run --release` runs experiments A, B and C, prints a measurements table and writes
//! the PDFs to `target/krilla-fit/` (not committed). See README.md.
//!
//! `cargo run --release -- photo-source <out.png>` writes the PNG that `scripts/make-jpeg.sh`
//! turns into `assets/photo.jpg`.

mod assets;
mod exp_a;
mod exp_b;
mod exp_c;
mod graph;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, ensure};
use lamina_cos::{Object, PdfString, XrefEol, incremental_update};

const RUNS: usize = 30;

/// Best-of-`RUNS` wall time and the last result.
fn best<T>(mut f: impl FnMut() -> Result<T>) -> Result<(Duration, T)> {
    let mut best = Duration::MAX;
    let mut last = None;
    for _ in 0..RUNS {
        let t = Instant::now();
        let v = f()?;
        best = best.min(t.elapsed());
        last = Some(v);
    }
    Ok((best, last.ok_or_else(|| anyhow!("no runs"))?))
}

fn ms(d: Duration) -> String {
    format!("{:.3} ms", d.as_secs_f64() * 1000.0)
}

fn kib(n: usize) -> String {
    format!("{:.1} KiB ({n} B)", n as f64 / 1024.0)
}

/// Open + walk every page and every object (what "open" costs for a caller that touches all).
fn open_and_walk(bytes: &[u8]) -> Result<usize> {
    let doc = lamina_cos::open(bytes)?;
    ensure!(!doc.was_repaired(), "lamina-cos had to repair the file");
    let pages = doc.pages()?;
    for p in &pages {
        doc.page_content(p)?;
    }
    for n in doc.object_numbers().collect::<Vec<_>>() {
        doc.object(n)?;
    }
    Ok(pages.len())
}

/// Total stored bytes of embedded font programs (streams with /Length1, /Length2 or /Subtype).
fn font_program_bytes(bytes: &[u8]) -> Result<usize> {
    let doc = lamina_cos::open(bytes)?;
    let mut total = 0;
    for n in doc.object_numbers().collect::<Vec<_>>() {
        let obj = doc.object(n)?;
        if let Object::Dict(d) = &*obj
            && d.get_name("Type") == Some(b"FontDescriptor")
        {
            for key in ["FontFile", "FontFile2", "FontFile3"] {
                if let Some(r) = d.get(key).and_then(Object::as_ref)
                    && let Object::Stream(s) = &*doc.resolve(r)?
                {
                    total += s.data.len();
                }
            }
        }
    }
    Ok(total)
}

/// Every page's /Parent is a /Type /Pages node (checks B1 doesn't point pages elsewhere).
fn parents_ok(bytes: &[u8]) -> Result<bool> {
    let doc = lamina_cos::open(bytes)?;
    for p in doc.pages()? {
        let parent = p.dict.get("Parent").cloned().unwrap_or(Object::Null);
        let parent = doc.deref(&parent)?;
        if parent.as_dict().and_then(|d| d.get_name("Type")) != Some(b"Pages") {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Sets /Info /Title with an incremental update.
fn retitle(bytes: &[u8], title: &str) -> Result<Vec<u8>> {
    let doc = lamina_cos::open(bytes)?;
    let info_ref = doc
        .trailer()
        .get("Info")
        .and_then(Object::as_ref)
        .context("no indirect /Info")?;
    let mut info = doc.resolve(info_ref)?.as_dict().cloned().context("/Info not a dict")?;
    info.insert("Title", Object::String(PdfString::literal(title)));
    Ok(incremental_update(
        bytes,
        &[(info_ref, Object::Dict(info))],
        XrefEol::default(),
    )?)
}

fn title_of(bytes: &[u8]) -> Result<String> {
    let doc = lamina_cos::open(bytes)?;
    let info = doc.trailer().get("Info").cloned().context("no /Info")?;
    let info = doc.deref(&info)?;
    match info.as_dict().and_then(|d| d.get("Title")) {
        Some(Object::String(s)) => Ok(String::from_utf8_lossy(&s.bytes).into_owned()),
        other => Err(anyhow!("title: {other:?}")),
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("photo-source") {
        let out = args.get(2).context("usage: photo-source <out.png>")?;
        return assets::write_photo_source(std::path::Path::new(out));
    }

    let out_dir = assets::spike_dir().join("target/krilla-fit");
    std::fs::create_dir_all(&out_dir)?;
    let save = |name: &str, bytes: &[u8]| -> Result<PathBuf> {
        let p = out_dir.join(name);
        std::fs::write(&p, bytes)?;
        Ok(p)
    };
    let assets = assets::load()?;
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut row = |k: &str, v: String| {
        println!("| {k} | {v} |");
        rows.push((k.to_owned(), v));
    };
    println!("| Measurement (best of {RUNS}, release) | Value |\n| --- | --- |");

    // ---- A: krilla as-is ----
    let (t, a) = best(|| exp_a::generate(&assets, false))?;
    row("A krilla generate (untagged)", ms(t));
    row("A krilla output size", kib(a.len()));
    row("A embedded font program (subset)", kib(font_program_bytes(&a)?));
    row(
        "A deterministic (two runs byte-identical)",
        (exp_a::generate(&assets, false)? == a).to_string(),
    );
    let (t, a_tagged) = best(|| exp_a::generate(&assets, true))?;
    row("A krilla generate (tagged)", ms(t));
    row("A krilla tagged output size", kib(a_tagged.len()));
    let (t, pages) = best(|| open_and_walk(&a))?;
    row(
        "A lamina-cos open + walk all pages/objects",
        format!("{} ({pages} pages)", ms(t)),
    );
    let (t, edited) = best(|| retitle(&a, "Edited by lamina-cos"))?;
    row(
        "A incremental /Info title update",
        format!("{}, +{} B", ms(t), edited.len() - a.len()),
    );
    row("A reopened title", title_of(&edited)?);
    row("A reopened page count", open_and_walk(&edited)?.to_string());
    save("a-krilla.pdf", &a)?;
    save("a-krilla-tagged.pdf", &a_tagged)?;
    save("a-krilla-edited.pdf", &edited)?;

    // ---- C: lamina-native builder ----
    let (t, c) = best(|| exp_c::generate(&assets, false))?;
    row("C lamina builder generate (classic xref)", ms(t));
    row("C output size (classic xref)", kib(c.bytes.len()));
    row(
        "C embedded font program (subset, Flate)",
        format!(
            "{} (raw subset {})",
            kib(font_program_bytes(&c.bytes)?),
            kib(c.font_subset_len)
        ),
    );
    let (t, cc) = best(|| exp_c::generate(&assets, true))?;
    row("C lamina builder generate (object + xref streams)", ms(t));
    row("C output size (object + xref streams)", kib(cc.bytes.len()));
    row(
        "C deterministic",
        (exp_c::generate(&assets, false)?.bytes == c.bytes).to_string(),
    );
    let (t, pages) = best(|| open_and_walk(&c.bytes))?;
    row("C lamina-cos open + walk", format!("{} ({pages} pages)", ms(t)));
    let c_edited = retitle(&c.bytes, "Edited by lamina-cos")?;
    row("C reopened title after incremental update", title_of(&c_edited)?);
    save("c-lamina.pdf", &c.bytes)?;
    save("c-lamina-compact.pdf", &cc.bytes)?;
    save("c-lamina-edited.pdf", &c_edited)?;

    // ---- B: graph integration ----
    let hello = std::fs::read(assets::spike_dir().join("../../fixtures/corpus/hello.pdf"))?;
    let base_pages = open_and_walk(&hello)?;
    let (t, (b1, report)) = best(|| graph::append_pages(&hello, &a_tagged))?;
    row(
        "B1 import krilla's 3 tagged pages into hello.pdf (incremental)",
        format!("{}, +{} B", ms(t), b1.len() - hello.len()),
    );
    row(
        "B1 objects copied / pages",
        format!("{} / {}", report.objects_copied, report.pages),
    );
    row(
        "B1 catalog keys imported / tagging dropped",
        format!("{:?} / {:?}", report.catalog_keys_imported, report.tagging_dropped),
    );
    row(
        "B1 page count after reopen; every page's /Parent is a /Pages node of the base",
        format!("{} (base {base_pages}); {}", open_and_walk(&b1)?, parents_ok(&b1)?),
    );
    let (t, (b1c, report_c)) = best(|| graph::append_pages(&c.bytes, &a_tagged))?;
    row(
        "B1 the same into the C document (has an outline and /Lang)",
        format!(
            "{}, +{} B, {} objects; conflicts {:?}; {} pages; parents ok: {}",
            ms(t),
            b1c.len() - c.bytes.len(),
            report_c.objects_copied,
            report_c.catalog_keys_conflicting,
            open_and_walk(&b1c)?,
            parents_ok(&b1c)?
        ),
    );
    let (b1t, report_t) = graph::append_pages(&a_tagged, &a_tagged)?;
    row(
        "B1 tagged pages into an already tagged base (A tagged into itself)",
        format!(
            "conflicts {:?}; tagging dropped {:?}; {} pages; parents ok: {}",
            report_t.catalog_keys_conflicting,
            report_t.tagging_dropped,
            open_and_walk(&b1t)?,
            parents_ok(&b1t)?
        ),
    );
    save("b1-krilla-into-tagged.pdf", &b1t)?;
    let (t, (b2, objs)) = best(|| exp_b::append_page_via_chunk(&a))?;
    row(
        "B2 pdf-writer chunk numbered in lamina's space → incremental",
        format!("{} ({objs} objects), +{} B", ms(t), b2.len() - a.len()),
    );
    row("B2 page count after reopen", open_and_walk(&b2)?.to_string());
    let (t, parsed) = best(|| {
        let doc = lamina_cos::open(&a)?;
        let mut chunk_bytes = 0;
        for n in doc.object_numbers().collect::<Vec<_>>() {
            chunk_bytes += doc.object(n)?.to_bytes().len();
        }
        Ok(chunk_bytes)
    })?;
    row(
        "B parse+reserialize all krilla objects (lamina Object round trip)",
        format!("{} for {}", ms(t), kib(parsed)),
    );
    save("b1-krilla-into-hello.pdf", &b1)?;
    save("b1-krilla-into-c.pdf", &b1c)?;
    save("b2-chunk-into-a.pdf", &b2)?;

    println!("\nPDFs written to {}", out_dir.display());
    Ok(())
}
