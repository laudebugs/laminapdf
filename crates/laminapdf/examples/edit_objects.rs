//! Move, resize, delete and reorder what a page already draws.
//!
//! Lists the first page's objects, then changes four of them in one edit: shrinks the first
//! shape about its centre, deletes the second, moves the last image across the page and sends
//! the third shape, a line across the page, under everything else. Each edit names its object
//! as the list does, and only the objects it names are written again: the rest of the page
//! keeps its bytes. An object that would draw differently in its new place in the stacking order
//! (another clip or transparency there) is refused with `content-not-editable`.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_objects -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    ContentEdit, ContentObjectKind, Document, EditContent, Matrix, OpenOptions, PageObject, SaveOptions, StackPosition,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: edit_objects in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?)?;
    std::fs::write(out.join("edit_objects.pdf"), edited)?;
    Ok(())
}

/// The document with four objects of its first page changed.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let objects = doc.content_objects(0)?;
    for object in &objects {
        println!(
            "{:?} {:?}{}",
            object.kind,
            object.bounds,
            match object.read_only {
                Some(reason) => format!(" (read-only: {})", reason.as_str()),
                None => String::new(),
            }
        );
    }
    let shapes: Vec<&PageObject> = objects.iter().filter(|o| o.kind == ContentObjectKind::Path).collect();
    let photo = objects
        .iter()
        .rev()
        .find(|o| o.kind == ContentObjectKind::Image)
        .ok_or("no image")?;
    let (square, second, line) = match shapes.as_slice() {
        [square, second, line, ..] => (square, second, line),
        _ => return Err("the page needs three shapes".into()),
    };
    let square_box = square.bounds.ok_or("the square draws nothing")?;
    // Page space: points from the top-left, y down. Half size, about the square's centre.
    let (cx, cy) = (
        (square_box.x0.get() + square_box.x1.get()) / 2.0,
        (square_box.y0.get() + square_box.y1.get()) / 2.0,
    );
    let shrink = Matrix::translate(-cx, -cy)
        .then(&Matrix::scale(0.5, 0.5))
        .then(&Matrix::translate(cx, cy));
    let page = doc.page_info(0)?.id;
    let edits = vec![
        ContentEdit::Transform {
            object: square.object,
            matrix: shrink,
        },
        ContentEdit::Delete { object: second.object },
        ContentEdit::Transform {
            object: photo.object,
            matrix: Matrix::translate(300.0, 0.0),
        },
        ContentEdit::Reorder {
            object: line.object,
            to: StackPosition::Back,
        },
    ];
    doc.apply(vec![EditContent::new(page, edits).into()])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
