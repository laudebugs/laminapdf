//! Merge PDFs into one, with a bookmark for each file.
//!
//! Each file's own bookmarks go under a bookmark named after it, its form fields keep working
//! (renamed when two files use one name), and its pages keep their labels.
//!
//! ```sh
//! cargo run -p laminapdf --example merge_documents -- a.pdf b.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};

use laminapdf::{Document, MergeInput, MergeOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let [first, second, rest @ ..] = args.as_slice() else {
        return Err("usage: merge_documents a.pdf b.pdf [out-dir]".into());
    };
    let out = rest.first().cloned().unwrap_or_else(|| "target/examples-out".into());
    std::fs::create_dir_all(&out)?;
    let files = [
        (name(first), std::fs::read(first)?),
        (name(second), std::fs::read(second)?),
    ];
    std::fs::write(out.join("merge_documents.pdf"), merge(files.to_vec())?)?;
    Ok(())
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One document of `files` (each a name and its bytes), in order, with a bookmark per file.
pub(crate) fn merge(files: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, Box<dyn Error>> {
    let inputs = files
        .into_iter()
        .map(|(name, bytes)| MergeInput::new(bytes).with_title(name))
        .collect();
    let mut merged = Document::merge(inputs, MergeOptions::default())?;
    Ok(merged.save(&SaveOptions::new())?.bytes)
}
