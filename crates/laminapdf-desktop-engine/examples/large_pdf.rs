//! Writes a large, image-heavy PDF with the `laminapdf` builder: the desktop engine's 1 GB
//! fixture, never committed. `cargo xtask fixtures large-pdf` runs it.
//!
//! ```sh
//! cargo run --release -p laminapdf-desktop-engine --example large_pdf -- --pages 20000 --out target/fixtures/1gb.pdf
//! ```
//!
//! Each US Letter page shows its own 168 × 112 image of seeded noise (56 KB that Flate can't
//! shrink), so 20,000 pages make a file of about 1.1 GB that a reader has to seek through. The
//! same arguments always write the same file. Building holds the whole document in memory: 20,000
//! pages (1,082 MiB) took 41 s and peaked at 1.5 GB on an M1 Pro.

use std::path::PathBuf;
use std::process::ExitCode;

use laminapdf::{BuildOptions, Builder, Image, PageSize, Pt, RectPt, SaveOptions};
use laminapdf_test_fixtures::Rng;

/// The image's size in pixels: 168 × 112 × 3 bytes is 56,448 bytes a page.
const WIDTH: u32 = 168;
const HEIGHT: u32 = 112;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("large_pdf: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let (pages, out) = args()?;
    let mut rng = Rng::new(0x1a31_7a00);
    let mut builder = Builder::new(BuildOptions::default());
    let mut rgba = vec![255_u8; (WIDTH * HEIGHT * 4) as usize];
    for index in 0..pages {
        for [r, g, b, _] in rgba.as_chunks_mut::<4>().0 {
            let [x, y, z, ..] = rng.next_u64().to_le_bytes();
            (*r, *g, *b) = (x, y, z);
        }
        let image = Image::rgba(WIDTH, HEIGHT, &rgba).map_err(|e| e.to_string())?;
        let mut page = builder.page(PageSize::LETTER);
        // The image fills the page's top half, keeping its aspect.
        let at = RectPt::from_xywh(Pt::new(36.0), Pt::new(396.0), Pt::new(540.0), Pt::new(360.0));
        page.surface().image(&image, at);
        page.finish().map_err(|e| format!("page {index}: {e}"))?;
        if index % 1000 == 999 {
            eprintln!("large_pdf: {} pages", index + 1);
        }
    }
    let mut doc = builder.finish().map_err(|e| e.to_string())?;
    let saved = doc.save(&SaveOptions::default()).map_err(|e| e.to_string())?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&out, &saved.bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!(
        "large_pdf: {pages} pages, {} MiB → {}",
        saved.bytes.len() >> 20,
        out.display()
    );
    Ok(())
}

/// `--pages N` (default 20,000) and `--out PATH` (default `target/fixtures/1gb.pdf`).
fn args() -> Result<(u32, PathBuf), String> {
    let mut pages = 20_000;
    let mut out = PathBuf::from("target/fixtures/1gb.pdf");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
        match arg.as_str() {
            "--pages" => pages = value.parse().map_err(|_| format!("--pages {value}: not a number"))?,
            "--out" => out = PathBuf::from(value),
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    Ok((pages, out))
}
