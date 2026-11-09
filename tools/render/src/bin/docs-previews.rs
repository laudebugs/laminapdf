//! `docs-previews <pdf-dir> <out-dir>` (todo B7d): renders every page of every PDF in `<pdf-dir>`
//! with hayro at 144 dpi (2× the render goldens, sharp on a 2× screen) to
//! `<out-dir>/<stem>-<page>.png`, for the docs site's Result tabs. Prints one JSON line per page,
//! `{"example":…,"page":…,"width":…,"height":…}`, which `cargo xtask docs-previews` reads into
//! the manifest. `<page>` is 0-based.

#[path = "../hayro_render.rs"]
mod hayro_render;
#[path = "../image.rs"]
#[expect(
    dead_code,
    reason = "shared with render-golden, which also reads PPM and PNG goldens and crops them"
)]
mod image;

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

/// Twice the render goldens' 72 dpi: one picture pixel per device pixel on a 2× screen.
const DPI: f32 = 144.0;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("docs-previews: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [pdfs, out] = args.as_slice() else {
        bail!("usage: docs-previews <pdf-dir> <out-dir>");
    };
    let (pdfs, out) = (PathBuf::from(pdfs), PathBuf::from(out));
    fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
    let mut files: Vec<PathBuf> = fs::read_dir(&pdfs)
        .with_context(|| format!("reading {}", pdfs.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    for path in files {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .with_context(|| format!("{} has no UTF-8 name", path.display()))?
            .to_owned();
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let doc = hayro_render::Doc::load(bytes).with_context(|| stem.clone())?;
        for page in 0..doc.page_count() {
            let img = doc.render(page, DPI).with_context(|| format!("{stem} page {page}"))?;
            image::write_png(&out.join(format!("{stem}-{page}.png")), &img)?;
            println!(
                "{}",
                serde_json::json!({ "example": stem, "page": page, "width": img.width, "height": img.height })
            );
        }
    }
    Ok(())
}
