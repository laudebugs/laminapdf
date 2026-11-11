//! Writes `laminapdf-cli`'s goldens into a directory and prints their paths:
//! `cli-rotate-hello.pdf`, `laminapdf pages rotate fixtures/corpus/hello.pdf +90` (an
//! incremental update of the input), and `cli-build-report.pdf`, `laminapdf build
//! fixtures/markdown/report.md` with the Noto fonts under PDF/UA-1, and `cli-build-invoice.pdf`,
//! `laminapdf build fixtures/documents/invoice.json --data fixtures/documents/invoice-data.json`
//! with the template's two fonts. `cargo xtask golden --update`
//! runs it on `fixtures/golden/`; `tests/pages.rs` and `tests/build.rs` compare the binary's
//! output with them.
//!
//! ```sh
//! cargo run -p laminapdf-cli --example golden -- <out-dir>
//! ```

use std::error::Error;
use std::path::Path;

use laminapdf_cli::commands::{BuildArgs, RotateArgs, Settings, build, rotate};
use laminapdf_cli::io::ProcessIo;

fn main() -> Result<(), Box<dyn Error>> {
    let out = std::env::args().nth(1).ok_or("usage: golden <out-dir>")?;
    let input = laminapdf_test_fixtures::fixtures_dir().join("corpus/hello.pdf");
    let output = Path::new(&out).join("cli-rotate-hello.pdf");
    let args = RotateArgs {
        input: input.to_string_lossy().into_owned(),
        rotation: "+90".to_owned(),
        output: Some(output.to_string_lossy().into_owned()),
        in_place: false,
    };
    let settings = Settings {
        force: true,
        ..Settings::default()
    };
    rotate(&args, &settings, &mut ProcessIo::default())?;
    println!("{}", output.display());
    let built = Path::new(&out).join("cli-build-report.pdf");
    build(
        &report_args(&built.to_string_lossy()),
        &settings,
        &mut ProcessIo::default(),
    )?;
    println!("{}", built.display());
    let invoice = Path::new(&out).join("cli-build-invoice.pdf");
    build(
        &invoice_args(&invoice.to_string_lossy()),
        &settings,
        &mut ProcessIo::default(),
    )?;
    println!("{}", invoice.display());
    Ok(())
}

/// `laminapdf build fixtures/documents/invoice.json --data …/invoice-data.json -o <output>` with
/// the fonts the template names, as `tests/build_document.rs` runs it.
fn invoice_args(output: &str) -> BuildArgs {
    let fixtures = laminapdf_test_fixtures::fixtures_dir();
    let path = |p: &str| fixtures.join(p).to_string_lossy().into_owned();
    BuildArgs {
        input: path("documents/invoice.json"),
        output: output.to_owned(),
        data: Some(path("documents/invoice-data.json")),
        fonts: vec![
            format!("regular={}", path("fonts/NotoSans-Regular.ttf")),
            format!("bold={}#1", path("fonts/NotoSans-RegularBold.ttc")),
        ],
        ..BuildArgs::default()
    }
}

/// `laminapdf build fixtures/markdown/report.md -o <output>` with the fixture fonts, as
/// `tests/build.rs` runs it.
fn report_args(output: &str) -> BuildArgs {
    let fixtures = laminapdf_test_fixtures::fixtures_dir();
    let path = |p: &str| fixtures.join(p).to_string_lossy().into_owned();
    BuildArgs {
        input: path("markdown/report.md"),
        output: output.to_owned(),
        fonts: vec![
            format!("regular={}", path("fonts/NotoSans-Regular.ttf")),
            format!("bold={}#1", path("fonts/NotoSans-RegularBold.ttc")),
            format!("fallback={}", path("fonts/NotoSansArabic-Regular.ttf")),
            format!("fallback={}", path("fonts/NotoSansDevanagari-Regular.ttf")),
        ],
        profiles: vec!["pdf-ua-1".to_owned()],
        ..BuildArgs::default()
    }
}
