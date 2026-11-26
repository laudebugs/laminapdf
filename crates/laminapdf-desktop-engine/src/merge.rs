//! A new document of a document's pages and other files' pages, for Merge documents into a new file
//! (E2 plan §7). The engine can't write files, so the whole new document waits as a save's output,
//! read in parts by the main process, which writes it where the person chose.

use laminapdf::{Document, FieldClash, MergeInput, MergeOptions, OutlineMerge, SaveMode, SaveOptions};
use laminapdf_desktop_protocol::{FailureKind, MergeFields, MergeOutline};

use crate::edit::Refused;
use crate::worker::kind_of;

/// A file to merge in: its bytes, its password and the name of its bookmark.
pub(crate) struct File {
    pub(crate) bytes: Vec<u8>,
    pub(crate) password: Option<String>,
    pub(crate) title: String,
}

/// The new document, written whole: `doc`'s pages as its edits leave them under `title`, then each
/// of `files`.
///
/// # Errors
/// [`FailureKind::SaveRefused`] with the error's code (`wrong-password` for a file that needs
/// another password, `no-pages-left`, the reading errors).
pub(crate) fn build(
    doc: &Document,
    title: String,
    files: Vec<File>,
    outline: MergeOutline,
    fields: MergeFields,
) -> Result<Vec<u8>, Refused> {
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    let ids: Vec<_> = doc
        .page_infos()
        .map_err(|e| refused(&e))?
        .into_iter()
        .map(|page| page.id)
        .collect();
    let mut copy = doc.extract_pages(&ids).map_err(|e| refused(&e))?;
    let own = copy.save_copy(&options).map_err(|e| refused(&e))?;
    let mut inputs = vec![MergeInput::new(own.bytes).with_title(title)];
    inputs.extend(files.into_iter().map(|file| {
        let input = MergeInput::new(file.bytes).with_title(file.title);
        match file.password {
            Some(password) => input.with_password(password),
            None => input,
        }
    }));
    let merge = MergeOptions::default()
        .with_outline(match outline {
            MergeOutline::PerFile => OutlineMerge::PerFile,
            MergeOutline::Keep => OutlineMerge::Keep,
            MergeOutline::None => OutlineMerge::None,
        })
        .with_fields(match fields {
            MergeFields::Rename => FieldClash::Rename,
            MergeFields::Link => FieldClash::Link,
        });
    let mut merged = Document::merge(inputs, merge).map_err(|e| refused(&e))?;
    Ok(merged.save_copy(&options).map_err(|e| refused(&e))?.bytes)
}

fn refused(error: &laminapdf::Error) -> Refused {
    let kind = if kind_of(error.code()) == FailureKind::WrongPassword {
        FailureKind::WrongPassword
    } else {
        FailureKind::SaveRefused
    };
    (kind, error.code().to_owned())
}
