//! Writes `laminapdf-cli`'s golden into a directory and prints its path:
//! `cli-rotate-hello.pdf`, `laminapdf pages rotate fixtures/corpus/hello.pdf +90` (an
//! incremental update of the input). `cargo xtask golden --update` runs it on `fixtures/golden/`;
//! `tests/pages.rs` compares the binary's output with it.
//!
//! ```sh
//! cargo run -p laminapdf-cli --example golden -- <out-dir>
//! ```

use std::error::Error;
use std::path::Path;

use laminapdf_cli::commands::{RotateArgs, Settings, rotate};
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
    Ok(())
}
