//! `laminapdf render`: its PNGs against the render goldens (SSIM ≥ 0.99, as C2b compares a
//! backend with its own goldens), output templates over pages and inputs, clashes and refusals
//! before anything is written, standard output, and the report.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test helpers: a panic is a test failure"
)]

mod common;

use std::path::Path;

use common::schemas::Validator;
use common::{arg, folder, laminapdf, run};
use laminapdf_test_fixtures::fixtures_dir;
use laminapdf_test_fixtures::image::{Bitmap, read_png, ssim};

/// The lowest SSIM a render may score against a golden made by `laminapdf-render`'s own hayro
/// adapter (`fixtures/golden/render/hayro`). Those goldens come from aarch64, where hayro always
/// takes its NEON path and matches them (C2b's 0.99); elsewhere its SIMD path differs on
/// anti-aliased edges (fixtures/render-golden/README.md), so the bar is that of a second engine
/// against them (0.98, as PDFium against the native goldens).
const SSIM: f64 = if cfg!(target_arch = "aarch64") { 0.99 } else { 0.98 };

/// The lowest SSIM against the examples' goldens (`fixtures/render-golden`), on every machine.
/// `tools/render` makes those with `hayro::render`, which sizes and places the page itself; the
/// adapter plans its own tile, so glyph edges land a fraction of a pixel apart (the multilingual
/// example scores 0.986 on aarch64 too). Another pipeline gets a second engine's bar.
const EXAMPLE_SSIM: f64 = 0.98;

/// The PNG `bytes` as a bitmap.
fn decode(bytes: &[u8]) -> Bitmap {
    let dir = folder("render-decode");
    let path = dir.join("page.png");
    std::fs::write(&path, bytes).unwrap();
    read_png(&path)
}

fn assert_matches(rendered: &Path, golden: &Path, bar: f64) {
    let (actual, expected) = (read_png(rendered), read_png(golden));
    assert_eq!(
        (actual.width, actual.height),
        (expected.width, expected.height),
        "{}",
        golden.display()
    );
    let score = ssim(&actual, &expected);
    assert!(score >= bar, "{}: SSIM {score:.4}", golden.display());
}

#[test]
fn corpus_pages_at_96_dpi_should_match_the_hayro_goldens() {
    let out = folder("render-corpus");
    for (file, pages) in [("hello", 1), ("outline", 3), ("imaging-demo", 1)] {
        let template = out.join("{stem}-p{n}.png");
        let input = format!("fixtures/corpus/{file}.pdf");
        let run = laminapdf(&["render", &input, "--dpi", "96", "-o", arg(&template)]);
        assert_eq!(run.code, 0, "{file}: {}", run.stderr);
        for n in 1..=pages {
            let name = format!("{file}-p{n}.png");
            assert_matches(
                &out.join(&name),
                &fixtures_dir().join("golden/render/hayro").join(&name),
                SSIM,
            );
        }
    }
}

#[test]
fn every_example_at_72_dpi_should_match_its_render_golden() {
    let goldens = fixtures_dir().join("render-golden");
    let mut examples: Vec<String> = std::fs::read_dir(&goldens)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    examples.sort();
    assert!(examples.len() >= 10, "{examples:?}");
    let out = folder("render-examples");
    for example in &examples {
        let input = format!("fixtures/golden/examples/{example}.pdf");
        // `{n}` counts from 1; the goldens name pages from 0.
        let run = laminapdf(&["render", &input, "--scale", "1", "-o", arg(&out.join("{stem}-{n}.png"))]);
        assert_eq!(run.code, 0, "{example}: {}", run.stderr);
        let mut pages = 0;
        while goldens.join(example).join(format!("{pages}-hayro.png")).exists() {
            assert_matches(
                &out.join(format!("{example}-{}.png", pages + 1)),
                &goldens.join(example).join(format!("{pages}-hayro.png")),
                EXAMPLE_SSIM,
            );
            pages += 1;
        }
        assert!(pages > 0, "{example} has no hayro golden");
    }
}

#[test]
fn the_report_should_list_each_page_and_match_its_schema() {
    let out = folder("render-report");
    let run = laminapdf(&[
        "render",
        "fixtures/golden/links-and-outlines.pdf",
        "--pages",
        "z,1",
        "--dpi",
        "36",
        "-o",
        arg(&out.join("{stem}-{n:03}.png")),
        "--json",
    ]);
    let report = run.json();
    Validator::new().check("render.schema.json", &report);
    let pages: Vec<(u64, String)> = report["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["number"].as_u64().unwrap(), p["output"].as_str().unwrap().to_owned()))
        .collect();
    let path = |n: &str| arg(&out.join(format!("links-and-outlines-{n}.png"))).to_owned();
    assert_eq!(pages, [(4, path("004")), (1, path("001"))]);
    assert_eq!(
        (report["backend"].as_str(), report["scale"].as_f64()),
        (Some("hayro"), Some(0.5))
    );
}

#[test]
fn clashing_outputs_should_be_refused_before_anything_is_written() {
    let out = folder("render-clash");
    let refused = [
        // Four pages, one file.
        laminapdf(&[
            "render",
            "fixtures/golden/links-and-outlines.pdf",
            "-o",
            arg(&out.join("one.png")),
        ]),
        // Two inputs, the same files.
        laminapdf(&[
            "render",
            "fixtures/corpus/hello.pdf",
            "fixtures/corpus/outline.pdf",
            "-o",
            arg(&out.join("p{n}.png")),
        ]),
        // Standard output takes one page.
        laminapdf(&["render", "fixtures/golden/links-and-outlines.pdf", "-o", "-"]),
        // No such field.
        laminapdf(&[
            "render",
            "fixtures/corpus/hello.pdf",
            "-o",
            arg(&out.join("{page}.png")),
        ]),
    ];
    for run in &refused {
        assert_eq!(run.code, 2, "{}", run.stderr);
    }
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0, "nothing written");
}

#[test]
fn an_existing_file_should_stop_the_command_before_any_page_is_written() {
    let out = folder("render-existing");
    std::fs::write(out.join("doc-3.png"), b"keep").unwrap();
    let template = out.join("doc-{n}.png");
    let refused = laminapdf(&["render", "fixtures/golden/links-and-outlines.pdf", "-o", arg(&template)]);
    assert_eq!(refused.code, 4, "{}", refused.stderr);
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 1);
    assert_eq!(std::fs::read(out.join("doc-3.png")).unwrap(), b"keep");
    let forced = laminapdf(&[
        "render",
        "fixtures/golden/links-and-outlines.pdf",
        "-o",
        arg(&template),
        "--force",
    ]);
    assert_eq!((forced.code, std::fs::read_dir(&out).unwrap().count()), (0, 4));
}

#[test]
fn a_dry_run_should_write_nothing() {
    let out = folder("render-dry");
    let run = laminapdf(&[
        "render",
        "fixtures/corpus/hello.pdf",
        "-o",
        arg(&out.join("{n}.png")),
        "--dry-run",
        "--json",
    ]);
    assert_eq!((run.code, run.json()["dryRun"].as_bool()), (0, Some(true)));
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
}

#[test]
fn one_page_should_go_to_standard_output_with_the_report_on_standard_error() {
    let run = run(
        &[
            "render",
            "fixtures/corpus/hello.pdf",
            "--dpi",
            "96",
            "-o",
            "-",
            "--json",
        ],
        None,
        &[],
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
    let golden = read_png(&fixtures_dir().join("golden/render/hayro/hello-p1.png"));
    assert!(ssim(&decode(&run.stdout), &golden) >= SSIM);
    assert_eq!(run.stderr_json()[0]["schema"], "laminapdf.render/1");
}

#[test]
fn several_inputs_should_each_get_their_own_files() {
    let out = folder("render-batch");
    let run = laminapdf(&[
        "render",
        "fixtures/corpus/hello.pdf",
        "fixtures/corpus/outline.pdf",
        "--dpi",
        "24",
        "-o",
        arg(&out.join("{stem}-{n}.png")),
        "--json",
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let mut written: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    written.sort();
    assert_eq!(
        written,
        ["hello-1.png", "outline-1.png", "outline-2.png", "outline-3.png"]
    );
    let summary = run.stdout_text().lines().last().map(str::to_owned).unwrap();
    assert!(summary.contains(r#""ok":2"#), "{summary}");
}

#[test]
fn the_pdfium_backend_should_say_when_it_has_no_library() {
    let run = run(
        &[
            "render",
            "fixtures/corpus/hello.pdf",
            "--backend",
            "pdfium",
            "-o",
            "-",
            "--json",
        ],
        None,
        &[("LAMINA_PDF_PDFIUM_DIR", "/nonexistent/pdfium")],
    );
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr_json()[0]["error"]["code"], "render-backend-unavailable");
}

#[test]
fn the_default_resolution_should_give_whole_pixels_for_a_letter_page() {
    let run = laminapdf(&["render", "fixtures/corpus/hello.pdf", "-o", "-", "--dry-run", "--json"]);
    let page = &run.json()["pages"][0];
    // 612 × 792 pt at 150 dpi.
    assert_eq!(
        (page["width"].as_u64(), page["height"].as_u64()),
        (Some(1275), Some(1650))
    );
}

#[test]
fn flags_render_cant_honour_should_be_refused() {
    let out = folder("render-refused-flags");
    let template = out.join("{n}.png");
    let strict = laminapdf(&["render", "fixtures/corpus/hello.pdf", "-o", arg(&template), "--strict"]);
    let list = run(
        &["render", "--files-from", "-", "-o", "-"],
        Some(b"fixtures/corpus/hello.pdf\n"),
        &[],
    );
    let twice = laminapdf(&[
        "render",
        "fixtures/corpus/outline.pdf",
        "--pages",
        "1,1",
        "-o",
        arg(&template),
        "--json",
    ]);
    assert_eq!(
        (strict.code, list.code, twice.code),
        (2, 2, 2),
        "{} {} {}",
        strict.stderr,
        list.stderr,
        twice.stderr
    );
    assert_eq!(
        twice.stderr_json()[0]["error"]["message"],
        "the page range names a page twice"
    );
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
}
