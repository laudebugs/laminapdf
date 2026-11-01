//! Writes the render goldens (`cargo xtask render-golden --update` runs this): every page of
//! every `fixtures/corpus/` file, by each backend at 96 DPI on white, to
//! `fixtures/golden/render/<backend>/<stem>-p<N>.png`, except the files `skip.json` lists.
//! Old PNGs of each backend are removed first, so a page that went away doesn't linger.
//! Needs `LAMINA_PDF_PDFIUM_DIR` (`cargo xtask fetch-pdfium`).
#![allow(clippy::unwrap_used, clippy::panic, reason = "a tool: a panic is a failed run")]

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    for backend in common::backends() {
        let dir = common::golden_dir(backend.name());
        if dir.exists() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().is_some_and(|e| e == "png") {
                    std::fs::remove_file(path).unwrap();
                }
            }
        }
        std::fs::create_dir_all(&dir).unwrap();
        let skip = common::skipped(backend.name());
        let mut written = 0;
        for file in common::corpus() {
            if skip.contains_key(&file) {
                continue;
            }
            for (i, bitmap) in common::render_file(backend.as_ref(), &file).iter().enumerate() {
                common::write_png(&dir.join(common::golden_name(&file, i)), bitmap);
                written += 1;
            }
        }
        println!(
            "{} {}: {written} golden(s) in {}",
            backend.name(),
            backend.version(),
            dir.display()
        );
    }
}
