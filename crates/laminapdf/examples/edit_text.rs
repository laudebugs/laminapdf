//! Change the text a page already shows, in the font it is set in.
//!
//! Lists the first page's runs of text, then rewrites the first one that says "world" to say
//! "Lamina PDF" instead. The new text uses the font the run was set in, so every character must
//! be one that font can draw; a character it can't is refused with `missing-glyph`, and a run
//! that can't be edited at all (a Type 3 font, vertical text) says why. The rest of the line
//! moves by the difference in width; the lines below stay where they were.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_text -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{ContentTextRun, Document, EditText, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: edit_text in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?)?;
    std::fs::write(out.join("edit_text.pdf"), edited)?;
    Ok(())
}

/// The document with "world" rewritten on its first page.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let runs = doc.text_runs(0)?;
    list(&runs);
    let run = runs
        .iter()
        .find(|r| r.read_only.is_none() && r.text.contains("world"))
        .ok_or("no editable run says \"world\"")?;
    let page = doc.page_info(0)?.id;
    let text = run.text.replace("world", "Lamina PDF");
    doc.apply(vec![EditText::new(page, run.run, text).into()])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// Prints each run: its line, text, font and, when it can't be edited, why.
fn list(runs: &[ContentTextRun]) {
    for run in runs {
        let read_only = match run.read_only {
            Some(reason) => format!(" (read-only: {})", reason.as_str()),
            None => String::new(),
        };
        println!(
            "line {}: {:?} in {} {}{read_only}",
            run.line, run.text, run.font_name, run.font_size
        );
    }
}
